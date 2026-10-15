//! This module builds the graph: `build`, `build_from`, and
//! `Graph::add_edges` add the nodes of the model and its declared edges,
//! with their inverses (CLI-7).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde_json::{Map, Value};

use super::{Edge, EdgeKind, Graph, Hydrator, Node, NodeKey, NodeKind, model::ancestors};
use crate::{
    Document, FileKind, Model, RepoPath, Segments, Yaml,
    markdown::{Body, IncludeTarget},
    model::keys::Label,
    yaml::Spanned,
};

/// A `Target` is what the `to` of a `links` entry resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A document, of this store or another.
    Document {
        /// The path of the document.
        path: RepoPath,
        /// The id of the document.
        id: String,
        /// The index of the document's knowledge.
        knowledge: usize,
    },
    /// A repository file, by path.
    File(RepoPath),
}

/// `resolve_to` resolves `to`, written in a document of `knowledge`. Text
/// that starts with `/` is a path, and any other text is an id. A store can
/// prefix an id. A path that identifies a document in any store
/// resolves to that document (SOKF_FORMAT_54). The lookup searches both the
/// documents that the run holds and the rows of the store, so the target
/// does not need to be in the run (CACHE-1_AC-22).
// sokf-implements: SOKF_FORMAT_54
// sokf-implements: CONF-2_AC-3
// sokf-implements: FORMAT-2_AC-1
pub fn resolve_to(model: &Model, knowledge: usize, to: &str) -> Option<Target> {
    if !to.starts_with('/') {
        let (store, id) = crate::model::split_store(to);
        let (k, path) = path_in(model, knowledge, store, id)?;
        return Some(Target::Document {
            path,
            id: id.to_string(),
            knowledge: k,
        });
    }
    let path = RepoPath::parse(to)?;
    if let Some((k, id)) = model.id_of(&path) {
        return Some(Target::Document {
            path,
            id,
            knowledge: k,
        });
    }
    model.exists(&path).then_some(Target::File(path))
}

/// `path_in` returns the knowledge and the path of the document that `id`
/// identifies in `store`. A document of the knowledge `citing` cites the
/// id. When no store is written, `path_in` looks in `citing` itself.
fn path_in(
    model: &Model,
    citing: usize,
    store: Option<&str>,
    id: &str,
) -> Option<(usize, RepoPath)> {
    let k = match store {
        None => citing,
        Some(s) => model.store(citing, s)?.knowledge,
    };
    model.path_of_id(k, id).map(|p| (k, p))
}

/// `inverse_rel` returns the inverse of the relation `rel`. It covers each
/// core relation and the relations of `resource`, `sources`, and `includes`
/// edges, and it returns `relates-to` for a custom value.
// sokf-implements: SOKF_FORMAT_59
// sokf-implements: SOKF_FORMAT_55
// sokf-implements: SOKF_FORMAT_57
// sokf-implements: SOKF_FORMAT_115
// sokf-implements: SOKF_FORMAT_116
pub fn inverse_rel(rel: &str) -> &'static str {
    match rel {
        "relates-to" => "relates-to",
        "part-of" => "has-part",
        "has-part" => "part-of",
        "depends-on" => "depended-on-by",
        "depended-on-by" => "depends-on",
        "references" => "referenced-by",
        "referenced-by" => "references",
        "supersedes" => "superseded-by",
        "superseded-by" => "supersedes",
        "implements" => "implemented-by",
        "implemented-by" => "implements",
        "tests" => "tested-by",
        "tested-by" => "tests",
        "contradicts" => "contradicts",
        "describes" => "described-by",
        "described-by" => "describes",
        "derives-from" => "derived-by",
        "derived-by" => "derives-from",
        "includes" => "included-by",
        "included-by" => "includes",
        _ => "relates-to",
    }
}

/// `frontmatter_json` returns the fields of a document node: the
/// frontmatter as written, without `links` and the keys that the tool sets,
/// then `status`, and then `broken` and `reason` when the document is
/// broken.
// sokf-implements: CLI-7_AC-41
// sokf-implements: CLI-7_AC-42
pub fn frontmatter_json(doc: &Document) -> Map<String, Value> {
    let mut fields = Map::new();
    if let Some(entries) = doc.frontmatter.as_ref().and_then(Spanned::as_map) {
        for (key, value) in entries {
            if matches!(
                key.value.as_str(),
                "links" | "kind" | "path" | "broken" | "reason"
            ) {
                continue;
            }
            fields.insert(key.value.clone(), yaml_json(value));
        }
    }
    // sokf-implements: CLI-7_AC-53
    if !fields.contains_key("status") {
        fields.insert("status".into(), Value::from("stable"));
    }
    // sokf-implements: CLI-7_AC-54
    let reason = if doc.frontmatter.is_none() && doc.frontmatter_found {
        Some("frontmatter does not parse")
    } else if doc.id().is_none() {
        Some("no id")
    } else {
        None
    };
    if let Some(reason) = reason {
        fields.insert("broken".into(), Value::Bool(true));
        fields.insert("reason".into(), Value::from(reason));
    }
    fields
}

fn yaml_json(value: &Spanned) -> Value {
    match &value.value {
        Yaml::Null => Value::Null,
        Yaml::Bool(b) => Value::Bool(*b),
        Yaml::Int(i) => Value::from(*i),
        Yaml::Float(f) => serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number),
        Yaml::Str(s) => Value::from(s.as_str()),
        Yaml::Seq(items) => Value::Array(items.iter().map(yaml_json).collect()),
        Yaml::Map(entries) => Value::Object(
            entries
                .iter()
                .map(|(k, v)| (k.value.clone(), yaml_json(v)))
                .collect(),
        ),
    }
}

/// A `RawEdge` is one edge as its source declares it, before the nodes at
/// its two ends exist in the graph. The cache keeps a document's edges in
/// this form. A `RawEdge` stores the target as a path when the target
/// resolves, and as the text written in the source when it doesn't
/// (CACHE-1_AC-22).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawEdge {
    /// The path of the source.
    pub from: RepoPath,
    /// The kind of the edge.
    pub kind: EdgeKind,
    /// The target as the source wrote it: a `to`, a label's id, a
    /// resource, or an include's target.
    pub written: String,
    /// The node key that the target resolved to.
    pub to: NodeKey,
    /// The relation, such as `implements`.
    pub rel: String,
    /// The region of an include, or the slug of the fragment that a code
    /// marker cites, if any.
    pub region: Option<String>,
    /// The line of a code marker in its file.
    pub line: Option<usize>,
    /// The `note` of a `links` entry.
    pub note: Option<String>,
}

/// `edges_of` returns the edges that the file at `path` declares. For a
/// document or a schema, these are its links, body links, resources,
/// sources, and includes, and for an additional file, its body links and
/// includes. It returns no edge for a path that the run doesn't hold.
// sokf-implements: SOKF_FORMAT_57
// sokf-implements: SOKF_FORMAT_115
// sokf-implements: SOKF_FORMAT_116
// sokf-implements: SOKF_FORMAT_117
// sokf-implements: SOKF_FORMAT_118
// sokf-implements: CACHE-1_AC-22
pub fn edges_of(model: &Model, path: &RepoPath) -> Vec<RawEdge> {
    let mut b = Builder::new(model);
    if let Some((k, doc)) = model.loaded_document(path) {
        b.document_edges(k, doc);
        b.include_edges(&doc.path, Some(k), &doc.segments);
        return b.raw;
    }
    if let Some(crate::Located::Additional(file)) = model.by_path(path) {
        b.body_links(&file.path, None, &file.body, &BTreeSet::new());
        b.include_edges(&file.path, file.schema.map(|s| s.knowledge), &file.segments);
    }
    b.raw
}

/// `build` builds the graph of `model` from the declared edges of every
/// document, schema, and additional file that the run holds.
pub fn build(model: &Model) -> Graph {
    build_from(model, declared_edges(model), None)
}

/// `declared_edges` returns the declared edges of every document, schema,
/// and additional file that the run holds, in that order.
pub(super) fn declared_edges(model: &Model) -> Vec<RawEdge> {
    model
        .documents_and_schemas()
        .map(|(_, d)| &d.path)
        .chain(model.additional.iter().map(|a| &a.path))
        .flat_map(|p| edges_of(model, p))
        .collect()
}

/// `build_from` builds the graph from the files that the walk lists and
/// from `edges`. When `build` calls it, `edges` holds every declared edge.
/// A caller can instead pass no edges. That caller then calls
/// `Graph::add_edges` later to add the edges near the start nodes. With a `hydrator`, the
/// graph loads the fields of a document node that the run doesn't hold
/// when a query first prints the node or reads its fields (CACHE-1_AC-22).
// sokf-implements: CLI-7_AC-25
// sokf-implements: CLI-7_AC-83
// sokf-implements: CACHE-1_AC-22
// sokf-implements: CLI-7_AC-57
pub fn build_from(model: &Model, edges: Vec<RawEdge>, hydrator: Option<Hydrator>) -> Graph {
    let mut b = Builder::new(model);
    let mut built: HashSet<NodeKey> = HashSet::new();
    // sokf-implements: CLI-7_AC-51
    // sokf-implements: CLI-7_AC-52
    for knowledge in &model.knowledges {
        let mut fields = Map::new();
        if let Some(m) = &knowledge.manifest {
            for (key, value) in [
                ("name", &m.name),
                ("scope", &m.scope),
                ("description", &m.description),
            ] {
                if let Some(value) = value {
                    fields.insert(key.into(), Value::from(value.as_str()));
                }
            }
        }
        b.insert(knowledge.root.clone(), NodeKind::Root, fields);
    }
    // Every document and schema that the walk lists becomes a node, and
    // those that the run holds carry their fields.
    // sokf-implements: CLI-7_AC-61
    // sokf-implements: CLI-7_AC-88
    for file in &model.tree.files {
        if !matches!(file.kind, FileKind::Document | FileKind::Schema) {
            continue;
        }
        let Some(k) = file.root else { continue };
        let fields = match model.loaded_document(&file.path) {
            Some((_, doc)) => {
                built.insert(NodeKey::Path(file.path.clone()));
                frontmatter_json(doc)
            }
            None => Map::new(),
        };
        b.insert(file.path.clone(), NodeKind::Document, fields);
        b.directories(&file.path, &model.knowledges[k].root);
    }
    // sokf-implements: CLI-7_AC-55
    for (path, schema) in &model.dispatch {
        let mut fields = Map::new();
        if let Some(r) = schema {
            fields.insert("schema".into(), Value::from(model.schema(*r).id()));
        }
        b.insert(path.clone(), NodeKind::Additional, fields);
        if let Some(k) = model.knowledge_of(path) {
            b.directories(path, &model.knowledges[k].root);
        }
    }
    // Add the asset nodes before any edge, so that an edge whose target is an
    // asset's path finds the asset's node instead of creating a `file` node
    // (CLI-7_AC-25).
    for knowledge in &model.knowledges {
        for asset in &knowledge.assets {
            let mut fields = Map::new();
            let sidecar = asset.sidecar_path();
            let id = match asset.sidecar {
                Some(i) => knowledge.documents[i].id().map(str::to_string),
                None if asset.has_sidecar => model
                    .fallback
                    .as_ref()
                    .and_then(|f| f.fields(&sidecar))
                    .and_then(|f| f.id),
                None => None,
            };
            if let Some(id) = id {
                fields.insert("sidecar".into(), Value::from(id));
            }
            b.insert(asset.path.clone(), NodeKind::Asset, fields);
            b.directories(&asset.path, &knowledge.root);
        }
    }
    // `finish` adds a node for the target of each edge.
    b.raw.extend(edges);
    let graph = b.finish();
    match hydrator {
        Some(h) => graph.with_hydrator(h, built),
        None => graph,
    }
}

struct Builder<'a> {
    model: &'a Model,
    nodes: BTreeMap<NodeKey, Node>,
    /// The edges declared so far, which don't have their nodes yet.
    raw: Vec<RawEdge>,
}

impl<'a> Builder<'a> {
    fn new(model: &'a Model) -> Self {
        Builder {
            model,
            nodes: BTreeMap::new(),
            raw: Vec::new(),
        }
    }

    fn insert(&mut self, path: RepoPath, kind: NodeKind, fields: Map<String, Value>) {
        let key = NodeKey::Path(path);
        self.nodes.insert(key.clone(), Node { key, kind, fields });
    }

    /// `directories` adds a node for each directory between `root` and
    /// `path`, excluding both.
    fn directories(&mut self, path: &RepoPath, root: &RepoPath) {
        for d in ancestors(path).take_while(|d| d != root && d.starts_with(root)) {
            let key = NodeKey::Path(d);
            self.nodes.entry(key.clone()).or_insert_with(|| Node {
                key,
                kind: NodeKind::Directory,
                fields: Map::new(),
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn edge(
        &mut self,
        from: &RepoPath,
        kind: EdgeKind,
        written: &str,
        to: NodeKey,
        rel: &str,
        region: Option<String>,
        note: Option<String>,
    ) {
        self.raw.push(RawEdge {
            from: from.clone(),
            kind,
            written: written.to_string(),
            to,
            rel: rel.to_string(),
            region,
            line: None,
            note,
        });
    }

    /// `path_target` returns the node key of a path target: the path when
    /// the file exists, and a missing key otherwise.
    fn path_target(&self, written: &str) -> NodeKey {
        match RepoPath::parse(written) {
            Some(p) if self.model.exists(&p) => NodeKey::Path(p),
            _ => NodeKey::Missing(written.to_string()),
        }
    }

    fn document_edges(&mut self, k: usize, doc: &Document) {
        let mut reached = BTreeSet::new();
        for entry in &doc.core.links {
            let to = match resolve_to(self.model, k, &entry.to) {
                Some(Target::Document { path: p, .. } | Target::File(p)) => NodeKey::Path(p),
                None => NodeKey::Missing(entry.to.clone()),
            };
            reached.insert(to.clone());
            self.edge(
                &doc.path,
                EdgeKind::Links,
                &entry.to,
                to,
                &entry.rel,
                None,
                entry.note.clone(),
            );
        }
        self.body_links(&doc.path, Some(k), &doc.body, &reached);
        let resources = doc.core.resource.iter().flat_map(|r| r.entries());
        let paths = resources
            .map(|r| (EdgeKind::Resource, r.as_str(), "describes"))
            .chain(
                doc.core
                    .sources
                    .iter()
                    .map(|e| (EdgeKind::Sources, e.resource.as_str(), "derives-from")),
            );
        for (kind, written, rel) in paths {
            if RepoPath::parse(written).is_some() {
                let to = self.path_target(written);
                self.edge(&doc.path, kind, written, to, rel, None, None);
            }
        }
    }

    /// `body_links` adds a `relates-to` edge for each body link
    /// `sokf:<id>`, once per target, when no `links` entry reaches that
    /// target.
    fn body_links(
        &mut self,
        from: &RepoPath,
        knowledge: Option<usize>,
        body: &Body,
        reached: &BTreeSet<NodeKey>,
    ) {
        let mut seen = reached.clone();
        for link in &body.links {
            let Some(Label {
                store,
                id: Some(id),
                ..
            }) = Label::of(&link.target)
            else {
                continue;
            };
            let written = match store {
                Some(s) => format!("{s}:{id}"),
                None => id.to_string(),
            };
            let to = match knowledge.and_then(|k| path_in(self.model, k, store, id)) {
                Some((_, path)) => NodeKey::Path(path),
                None => NodeKey::Missing(written.clone()),
            };
            if seen.insert(to.clone()) {
                self.edge(
                    from,
                    EdgeKind::Links,
                    &written,
                    to,
                    "relates-to",
                    None,
                    None,
                );
            }
        }
    }

    fn include_edges(&mut self, from: &RepoPath, knowledge: Option<usize>, segments: &Segments) {
        for block in &segments.regions {
            let (to, region) = match &IncludeTarget::parse(&block.target) {
                // The id of a sidecar resolves to its asset, or to the
                // sidecar itself when the fragment is empty. A fragment
                // with a name identifies the region that the include takes
                // (FORMAT-2_AC-5).
                IncludeTarget::Document {
                    store,
                    id,
                    fragment,
                } => {
                    let to = match knowledge
                        .and_then(|k| path_in(self.model, k, store.as_deref(), id))
                    {
                        Some((_, path)) => {
                            match (fragment.as_deref(), self.model.asset_of(&path)) {
                                (Some(""), _) | (_, None) => NodeKey::Path(path),
                                (_, Some((_, asset))) => NodeKey::Path(asset.path.clone()),
                            }
                        }
                        None => NodeKey::Missing(block.target.clone()),
                    };
                    (to, fragment.clone().filter(|f| !f.is_empty()))
                }
                IncludeTarget::File { path, region } => (self.path_target(path), region.clone()),
            };
            self.edge(
                from,
                EdgeKind::Includes,
                &block.target,
                to,
                "includes",
                region,
                None,
            );
        }
    }

    /// `finish` returns the graph of the nodes and the declared edges, with
    /// their inverses, sorted and without duplicates.
    fn finish(mut self) -> Graph {
        let raw = std::mem::take(&mut self.raw);
        let edges = link(&mut self.nodes, raw);
        Graph::new(self.nodes, edges)
    }
}

impl Graph {
    /// `add_edges` adds the declared edges `raw`, with their nodes and
    /// inverses, to a graph that was built with no declared edge. `raw`
    /// holds the edges near the start nodes of the graph, read from the store
    /// (CACHE-1_AC-38).
    pub fn add_edges(&mut self, raw: Vec<RawEdge>) {
        let edges = link(&mut self.nodes, raw);
        self.extend_edges(edges);
    }
}

/// `target` returns the key of the node that an edge ends at. A path that
/// isn't a node yet becomes a `file` node, and an unresolved target becomes
/// a `missing` node.
// sokf-implements: CLI-7_AC-18
fn target(nodes: &mut BTreeMap<NodeKey, Node>, key: NodeKey) -> NodeKey {
    let kind = match &key {
        NodeKey::Path(_) => NodeKind::File,
        NodeKey::Key(_) => NodeKind::Key,
        NodeKey::Missing(_) => NodeKind::Missing,
    };
    nodes.entry(key.clone()).or_insert_with(|| Node {
        key: key.clone(),
        kind,
        fields: Map::new(),
    });
    key
}

/// `link` gives each declared edge its nodes, and adds the inverse of each
/// edge unless that inverse is declared too.
// sokf-implements: CLI-7_AC-5
fn link(nodes: &mut BTreeMap<NodeKey, Node>, raw: Vec<RawEdge>) -> Vec<Edge> {
    let mut edges: Vec<Edge> = Vec::with_capacity(raw.len());
    for e in raw {
        let to = target(nodes, e.to);
        // A marker's file is outside every knowledge root, so it is a node
        // only once an edge reaches it.
        let from = target(nodes, NodeKey::Path(e.from));
        edges.push(Edge {
            from,
            kind: e.kind,
            to,
            rel: e.rel,
            region: e.region,
            line: e.line,
            synthesised: false,
            note: e.note,
        });
    }
    type Ends<'e> = (
        &'e NodeKey,
        EdgeKind,
        &'e NodeKey,
        &'e str,
        Option<&'e str>,
        Option<usize>,
    );
    fn ends(e: &Edge) -> Ends<'_> {
        (&e.from, e.kind, &e.to, &e.rel, e.region.as_deref(), e.line)
    }
    let declared: BTreeSet<Ends> = edges.iter().map(ends).collect();
    let inverses: Vec<Edge> = edges
        .iter()
        .map(|e| Edge {
            from: e.to.clone(),
            kind: e.kind,
            to: e.from.clone(),
            rel: inverse_rel(&e.rel).to_string(),
            region: e.region.clone(),
            line: e.line,
            synthesised: true,
            note: None,
        })
        .filter(|inv| !declared.contains(&ends(inv)))
        .collect();
    edges.extend(inverses);
    edges
}

#[cfg(test)]
mod tests;
