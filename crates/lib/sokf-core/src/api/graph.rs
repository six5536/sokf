//! This module runs the `sokf graph` command (CLI-7).

use std::{collections::BTreeSet, path::PathBuf, rc::Rc};

use crate::{
    Cache, CacheMode, Model, RepoPath, RepoRoot, Result, config,
    graph::{self, EdgeKind, Graph, GraphQuery, GraphResult, Hydrator, NodeKey, RawEdge},
    model::{KeyIndex, ParseMode, parse_source},
    source,
};

/// `GraphOptions` holds the options of `sokf graph`.
#[derive(Debug, Clone)]
pub struct GraphOptions {
    /// `query` is the query that `graph` runs on the graph.
    pub query: GraphQuery,
    /// `cwd` is the current directory, from which each `START` path resolves.
    pub cwd: PathBuf,
    /// `cache` is the cache mode. It is `Off` when the command has `--no-cache`.
    pub cache: CacheMode,
}

/// `graph` builds the graph and runs the query on it. With the cache on, it
/// first checks the documents that changed since the last check, as `validate`
/// does, so that the store is current before `graph` reads its edges and keys.
/// The graph has a node for each document in the repository, and `graph` reads
/// from the store only the edges near each `START`. `graph` reads a document's
/// fields from its file only when the query first visits the document's node.
/// With `--no-cache`, `graph` reads every document and every edge. The cache is
/// the only file that `graph` writes.
// sokf-implements: CLI-9_AC-3
// sokf-implements: CACHE-1_AC-27
// sokf-implements: CACHE-1_AC-22
// sokf-implements: CACHE-1_AC-38
// sokf-implements: CACHE-1_AC-44
// sokf-implements: CACHE-1_AC-45
// sokf-implements: CLI-7_AC-16
// sokf-implements: CLI-7_AC-17
// sokf-implements: CLI-7_AC-63
pub fn graph(root: &RepoRoot, opts: &GraphOptions) -> Result<(Graph, GraphResult)> {
    let config = config::load_config(root)?;
    let cache = Cache::open(root, opts.cache);
    // A `START` that is a key needs the keys that the schemas define. An id
    // doesn't, so `graph` loads keys only when one of the `START` arguments is
    // a key.
    let keyed = opts.query.start.iter().any(|s| graph::is_key_reference(s));
    // With no `START` and no layout edges, `graph` needs no edge, so it reads
    // none (CLI-7_AC-1). Otherwise, with `--no-cache`, it builds every edge,
    // and resolves every marker with every key.
    let every_edge = !cache.is_on() && (!opts.query.start.is_empty() || opts.query.layout_edges);
    let (mut model, keys, all_keys): (Model, Option<KeyIndex>, Option<KeyIndex>) = if keyed {
        let slugs = super::locate::key_slugs(&opts.query.start);
        let (model, keys) = super::load_keys_for(root, &config, &cache, &slugs)?;
        (model, Some(keys), None)
    } else if every_edge {
        let (model, keys) = super::load_with_keys(root, &config, &cache)?;
        (model, None, Some(keys))
    } else {
        (super::load_for_lookup(root, &config, &cache)?, None, None)
    };
    model.marker_check = config.markers;
    let markers = match keys.as_ref().or(all_keys.as_ref()) {
        Some(every) if every_edge => super::markers::marker_edges(&model, every, root),
        _ => Vec::new(),
    };
    let mut graph = if cache.is_on() {
        graph::build_from(&model, Vec::new(), Some(hydrator(root)))
    } else if every_edge {
        let mut graph = graph::build(&model);
        graph.add_edges(markers.clone());
        graph
    } else {
        graph::build_from(&model, Vec::new(), None)
    };
    graph.set_archived(model.archived_documents());
    if let Some(keys) = &keys {
        for arg in &opts.query.start {
            if let Some((entry, slug)) = graph::key_entry(&model, keys, arg) {
                let citing = citing(&model, keys, &cache, &entry.path, &slug);
                graph.add_key(&entry.path, &slug, &entry.key, entry.line, &citing);
                graph.add_edges(key_markers(&cache, &markers, &entry.path, &slug));
            }
        }
    }
    if cache.is_on() {
        let starts = graph::starts(&mut graph, &model, keys.as_ref(), &opts.cwd, &opts.query)?;
        let edges = graph::edges_near(&graph, &starts, &opts.query, &cache);
        graph.add_edges(edges);
    }
    let result = graph::run(&mut graph, &model, keys.as_ref(), &opts.cwd, &opts.query)?;
    Ok((graph, result))
}

/// `hydrator` returns the `Hydrator` that reads a document node's fields from
/// its file when the graph first visits the node (CACHE-1_AC-22).
fn hydrator(root: &RepoRoot) -> Hydrator {
    let repo = root.clone();
    Hydrator(Rc::new(move |path: &RepoPath| {
        let src = source::load(&repo, path).ok()?;
        let parsed = parse_source(&src, ParseMode::Document);
        Some(graph::frontmatter_json(&crate::Document::new(src, parsed)))
    }))
}

/// `key_markers` returns an edge to the key `slug` of the document at `path`
/// from the file of each marker that cites the key, with the marker's line.
/// With the cache on, it takes the markers from the citations that the check
/// stores. With `--no-cache`, it takes them from `markers`, the edges of every
/// marker that the run read.
// sokf-implements: CLI-7_AC-94
fn key_markers(cache: &Cache, markers: &[RawEdge], path: &RepoPath, slug: &str) -> Vec<RawEdge> {
    let key = NodeKey::Key(format!("{path}#{slug}"));
    let edge = |from: RepoPath, rel: String, line: Option<usize>| RawEdge {
        from,
        kind: EdgeKind::Marker,
        written: key.as_str().to_string(),
        to: key.clone(),
        rel,
        region: None,
        line,
        note: None,
    };
    if cache.is_on() {
        cache
            .stored_citations_of(path, &BTreeSet::new())
            .into_iter()
            .filter(|c| c.slug.as_deref() == Some(slug))
            .filter_map(|c| Some(edge(c.source, c.rel?, Some(c.line))))
            .collect()
    } else {
        markers
            .iter()
            .filter(|m| m.to.path() == Some(path) && m.region.as_deref() == Some(slug))
            .map(|m| edge(m.from.clone(), m.rel.clone(), m.line))
            .collect()
    }
}

/// `citing` returns the paths of the documents that cite the key `slug` of the
/// document at `path`. With the cache on, it takes them from the citations that
/// the check stores. With `--no-cache`, it resolves each link in the body of
/// every document that the run loads. `citing` leaves the document at `path`
/// out of the result (CLI-7_AC-74).
// sokf-implements: CLI-7_AC-74
fn citing(
    model: &Model,
    keys: &KeyIndex,
    cache: &Cache,
    path: &RepoPath,
    slug: &str,
) -> Vec<RepoPath> {
    let mut out: BTreeSet<RepoPath> = BTreeSet::new();
    if cache.is_on() {
        for c in cache.stored_citations_of(path, &BTreeSet::new()) {
            if c.rel.is_none() && c.slug.as_deref() == Some(slug) {
                out.insert(c.source);
            }
        }
    } else {
        for (k, doc) in model.documents() {
            if doc.type_name().is_none() {
                continue;
            }
            for link in &doc.body.links {
                let Some(label) = crate::model::keys::Label::of(&link.target) else {
                    continue;
                };
                if let Ok(r) = crate::model::keys::resolve(model, keys, k, label)
                    && &r.path == path
                    && r.slug.as_deref() == Some(slug)
                {
                    out.insert(doc.path.clone());
                }
            }
        }
    }
    out.remove(path);
    out.into_iter().collect()
}
