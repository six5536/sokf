//! This module reads the words that a user gives the graph commands:
//! `resolve_start` reads the `START` of a graph command (CLI-7), and
//! `resolve_id` reads the id that `locate` takes (CLI-8).

use std::path::Path;

use super::{Graph, NodeKey, NodeKind};
use crate::{
    Error, Model, PathArg, RepoPath, Result,
    model::{KeyEntry, KeyIndex, split_store},
};

/// `Start` is what a `START` resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// A knowledge root, or a directory in one, whose layout the command
    /// prints.
    Container(RepoPath),
    /// A document, an additional file, or a repository file, for which the
    /// command prints one row.
    Node(NodeKey),
    /// A directory outside every knowledge store, for which the command
    /// prints the roots and the files under it.
    Outside(RepoPath),
}

impl Start {
    /// `key` returns the key of the start's graph node.
    pub fn key(&self) -> NodeKey {
        match self {
            Start::Container(p) | Start::Outside(p) => NodeKey::Path(p.clone()),
            Start::Node(k) => k.clone(),
        }
    }

    /// `path` returns the start as a path, for the `start` field of the
    /// JSON output, or `None` for a node that has no path.
    pub fn path(&self) -> Option<&RepoPath> {
        match self {
            Start::Container(p) | Start::Outside(p) => Some(p),
            Start::Node(k) => k.path(),
        }
    }
}

/// `is_key_reference` reports whether `arg` is a reference to a key alone,
/// `#key` or `<store>:#key`, which the caller can resolve only with the
/// index of captured keys.
pub fn is_key_reference(arg: &str) -> bool {
    let arg = arg.strip_prefix("sokf:").unwrap_or(arg);
    arg.split_once('#')
        .is_some_and(|(head, _)| split_store(head).1.is_empty())
}

/// `key_entry` returns the key that a reference identifies, with the key's
/// slug. A reference to a key alone identifies the key when exactly one
/// document captures it (CLI-7_AC-74), at repository or knowledge scope or,
/// failing those, at document scope (CLI-7_AC-98). A reference
/// `<id>#<key>` identifies the key when the document with that id captures
/// it (CLI-7_AC-97).
pub fn key_entry(model: &Model, keys: &KeyIndex, arg: &str) -> Option<(KeyEntry, String)> {
    let written = arg.split_once('#')?.1;
    if written.is_empty() {
        return None;
    }
    let path = match paths_of_reference(model, Some(keys), arg).as_slice() {
        [one] => one.clone(),
        [] => match document_scoped(model, keys, arg).as_slice() {
            [(one, _)] => one.clone(),
            _ => return None,
        },
        _ => return None,
    };
    [written.to_string(), crate::markdown::slug(written)]
        .into_iter()
        .find_map(|slug| {
            let entry = keys.document.get(&(path.clone(), slug.clone()))?.first()?;
            Some((entry.clone(), slug))
        })
}

/// `document_scoped` returns each document that captures the key of `arg`,
/// with the key's slug, in the order of the documents' paths, when `arg` is
/// a reference to a key alone that matches no key of repository or knowledge
/// scope (CLI-7_AC-98 to CLI-7_AC-100). The documents lie in the stores
/// that the reference gives, or in every store when it gives none.
fn document_scoped(model: &Model, keys: &KeyIndex, arg: &str) -> Vec<(RepoPath, String)> {
    if !is_key_reference(arg) || !paths_of_reference(model, Some(keys), arg).is_empty() {
        return Vec::new();
    }
    let reference = arg.strip_prefix("sokf:").unwrap_or(arg);
    let Some((head, written)) = reference.split_once('#') else {
        return Vec::new();
    };
    let stores = stores_of(model, split_store(head).0);
    let slugs = [written.to_string(), crate::markdown::slug(written)];
    let mut found: Vec<(RepoPath, String)> = keys
        .document
        .keys()
        .filter(|(p, s)| {
            slugs.contains(s) && model.knowledge_of(p).is_some_and(|k| stores.contains(&k))
        })
        .cloned()
        .collect();
    // The key as written and its slug can both match one document, so each
    // document keeps one slug.
    found.sort();
    found.dedup_by(|a, b| a.0 == b.0);
    found
}

/// `stores_of` returns the stores that a reference searches: each store of
/// the name it gives, or every store when it gives none.
fn stores_of(model: &Model, store: Option<&str>) -> Vec<usize> {
    match store {
        None => (0..model.knowledges.len()).collect(),
        Some(s) => model.stores_named(s),
    }
}

/// `note` returns the note that `graph` prints for `arg`, when `arg` is a
/// reference to a key alone that one document captures at document scope
/// only. The note gives the reference that names the key in a link
/// (CLI-7_AC-98).
pub(super) fn note(model: &Model, keys: Option<&KeyIndex>, arg: &str) -> Option<String> {
    let [(path, slug)] = document_scoped(model, keys?, arg).try_into().ok()?;
    let reference = key_reference(model, &path, &slug)?;
    let id = reference.split_once('#')?.0;
    Some(format!(
        "`{arg}` is unique only within `{id}`, so link it as `{reference}`"
    ))
}

/// `key_reference` returns the reference `<id>#<slug>` to the key `slug` of
/// the document at `path`. The id has its store before it when another
/// store has a document with the same id. `key_reference` returns `None`
/// when another document of the same store has the id, because no reference
/// then names the document.
fn key_reference(model: &Model, path: &RepoPath, slug: &str) -> Option<String> {
    let (k, id) = id_of(model, path)?;
    let mut elsewhere = false;
    for o in 0..model.knowledges.len() {
        if model.ids(o, &id).into_iter().any(|p| &p != path) {
            if o == k {
                return None;
            }
            elsewhere = true;
        }
    }
    Some(if elsewhere {
        format!("{}:{id}#{slug}", model.store_name(k)?)
    } else {
        format!("{id}#{slug}")
    })
}

/// `id_of` returns the store and the id of the document at `path`.
fn id_of(model: &Model, path: &RepoPath) -> Option<(usize, String)> {
    match model.id_at(path) {
        Some(found) => Some(found),
        None => {
            // The run does not hold this document, so the id comes from the
            // store's row.
            let k = model.knowledge_of(path)?;
            let id = model.fallback.as_ref()?.fields(path)?.id?;
            Some((k, id))
        }
    }
}

/// `paths_of_reference` returns the paths of every document and schema that
/// a reference identifies, when it is written outside every store, such as
/// on the command line. The reference
/// `[sokf:][<store>:]<id>[#<fragment>]` identifies the document with that
/// id, whatever its fragment. The reference `[sokf:][<store>:]#<key>`
/// identifies the document that captures the key, when only one document
/// captures it (CLI-7_AC-29 to CLI-7_AC-32). A reference with no store
/// searches every root.
fn paths_of_reference(model: &Model, keys: Option<&KeyIndex>, arg: &str) -> Vec<RepoPath> {
    // sokf-implements: CLI-7_AC-60
    let reference = arg.strip_prefix("sokf:").unwrap_or(arg);
    let (head, fragment) = match reference.split_once('#') {
        Some((h, f)) => (h, Some(f)),
        None => (reference, None),
    };
    let (store, id) = split_store(head);
    let stores = stores_of(model, store);
    let mut paths: Vec<RepoPath> =
        if id.is_empty() {
            let (Some(keys), Some(key)) = (keys, fragment.filter(|f| !f.is_empty())) else {
                return Vec::new();
            };
            // The index holds each key under its slug, so the lookup tries
            // the key as written and then its slug.
            [key.to_string(), crate::markdown::slug(key)]
                .into_iter()
                .flat_map(|key| {
                    keys.repository
                        .get(&key)
                        .into_iter()
                        .flatten()
                        .chain(stores.iter().flat_map(|&k| {
                            keys.knowledge.get(&(k, key.clone())).into_iter().flatten()
                        }))
                        .filter(|e| stores.contains(&e.knowledge))
                        .map(|e| e.path.clone())
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            // The lookup searches the run's documents and the store's rows
            // (CACHE-1_AC-21).
            stores.iter().flat_map(|&k| model.ids(k, id)).collect()
        };
    paths.sort();
    paths.dedup();
    paths
}

/// `qualified` returns the qualified form of `arg` for each document in
/// `paths`, as advice for an ambiguous reference. Each form starts with the
/// store's name, or with its whole identity when another root shares that
/// name. The document's id follows, or `#key` for a reference to a key
/// alone. `qualified` returns an empty string when it cannot qualify every
/// document.
fn qualified(model: &Model, arg: &str, paths: &[RepoPath]) -> String {
    let key = is_key_reference(arg)
        .then(|| arg.split_once('#').map(|(_, k)| k))
        .flatten();
    let forms: Vec<String> = paths
        .iter()
        .filter_map(|p| {
            let (k, id) = id_of(model, p)?;
            let store = model.store_name(k)?;
            Some(match key {
                Some(key) => format!("`{store}:#{key}`"),
                None => format!("`{store}:{id}`"),
            })
        })
        .collect();
    if forms.len() == paths.len() {
        format!(". Write {}", forms.join(" or "))
    } else {
        String::new()
    }
}

fn join(paths: &[RepoPath]) -> String {
    paths
        .iter()
        .map(RepoPath::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

/// `resolve_id` returns the one document that a reference identifies across
/// every root, or refuses a reference that identifies no document or more
/// than one (CLI-8_AC-9).
// sokf-implements: CLI-8_AC-9
// sokf-implements: CLI-8_AC-10
// sokf-implements: CLI-8_AC-11
// sokf-implements: CLI-8_AC-6
// sokf-implements: FORMAT-2_AC-1
pub fn resolve_id(model: &Model, keys: Option<&KeyIndex>, id: &str) -> Result<RepoPath> {
    match paths_of_reference(model, keys, id).as_slice() {
        [] => Err(Error::Locate(format!("`{id}` identifies no document"))),
        [one] => Ok(one.clone()),
        many => Err(Error::Locate(format!(
            "`{id}` identifies more than one document: {}{}",
            join(many),
            qualified(model, id, many)
        ))),
    }
}

/// `resolve_start` reads `arg` both as an `id` and as a path, and refuses it
/// unless exactly one reading resolves (CLI-7_AC-19, CLI-7_AC-20). When the
/// start is a directory or a repository file, the function adds it to the
/// graph as a node.
// sokf-implements: CLI-7_AC-29
// sokf-implements: CLI-7_AC-30
// sokf-implements: CLI-7_AC-31
// sokf-implements: CLI-7_AC-32
// sokf-implements: CLI-7_AC-33
// sokf-implements: CLI-1_AC-7
// sokf-implements: CLI-1_AC-8
// sokf-implements: CLI-7_AC-4
// sokf-implements: CLI-7_AC-19
// sokf-implements: CLI-7_AC-20
pub fn resolve_start(
    graph: &mut Graph,
    model: &Model,
    keys: Option<&KeyIndex>,
    cwd: &Path,
    arg: &str,
) -> Result<Start> {
    // The function reads a word shaped as a path only as a path, even when
    // a document has that word as its id.
    // sokf-implements: CLI-7_AC-28
    let path_shaped = arg.starts_with('.') || arg.ends_with('/') || arg.ends_with(".md");
    let ids = if path_shaped {
        Vec::new()
    } else {
        paths_of_reference(model, keys, arg)
    };
    let path = PathArg(arg).resolve(&model.tree.root, cwd);
    match (ids.as_slice(), path) {
        ([_, ..], Some(p)) => Err(Error::Start(format!(
            "`{arg}` is both an id ({}) and a path ({p})",
            join(&ids)
        ))),
        ([_, _, ..], None) => Err(Error::Start(format!(
            "`{arg}` identifies more than one document: {}{}",
            join(&ids),
            qualified(model, arg, &ids)
        ))),
        // For a reference to a key alone, and for `<id>#<key>` when the
        // document captures the key, the start is the key's node when the
        // graph has one.
        // sokf-implements: CLI-7_AC-32
        // sokf-implements: CLI-7_AC-97
        ([one], None) => Ok(key_start(graph, model, keys, arg, one)),
        ([], Some(p)) => Ok(path_start(graph, model, p)),
        ([], None) => {
            // A key that no document captures at repository or knowledge
            // scope can still be a key of document scope.
            let scoped = keys.map_or_else(Vec::new, |k| document_scoped(model, k, arg));
            match scoped.as_slice() {
                // sokf-implements: CLI-7_AC-98
                [(one, _)] => Ok(key_start(graph, model, keys, arg, one)),
                // sokf-implements: CLI-7_AC-100
                [] => Err(Error::Start(format!(
                    "`{arg}` is neither an id nor a path in the repository"
                ))),
                // sokf-implements: CLI-7_AC-99
                many => {
                    let forms: Vec<String> = many
                        .iter()
                        .map(|(p, slug)| match key_reference(model, p, slug) {
                            Some(r) => format!("`{r}`"),
                            None => p.to_string(),
                        })
                        .collect();
                    Err(Error::Start(format!(
                        "`{arg}` is a key of document scope that more than one document captures. Write {}",
                        capped(&forms)
                    )))
                }
            }
        }
    }
}

/// `LISTED` is the number of references that the error for a key of
/// document scope lists before it counts the rest (CLI-7_AC-99).
const LISTED: usize = 10;

/// `capped` joins the first `LISTED` of `forms` with "or", in their order,
/// and follows them with "and N more" when `forms` holds more.
// sokf-implements: CLI-7_AC-99
fn capped(forms: &[String]) -> String {
    let listed = forms[..forms.len().min(LISTED)].join(" or ");
    match forms.len().saturating_sub(LISTED) {
        0 => listed,
        more => format!("{listed}, and {more} more"),
    }
}

/// `key_start` returns the start of `arg` in the document at `path`: the
/// node of the key that `arg` names, when the graph has one, and the
/// document otherwise.
fn key_start(
    graph: &Graph,
    model: &Model,
    keys: Option<&KeyIndex>,
    arg: &str,
    path: &RepoPath,
) -> Start {
    Start::Node(
        keys.and_then(|k| key_entry(model, k, arg))
            .map(|(_, slug)| NodeKey::Key(format!("{path}#{slug}")))
            .filter(|k| graph.node(k).is_some())
            .unwrap_or_else(|| NodeKey::Path(path.clone())),
    )
}

/// `path_start` reads a path as a start. It tries each reading in order: a
/// root, a directory, a node that the graph has, and then a repository
/// file.
fn path_start(graph: &mut Graph, model: &Model, path: RepoPath) -> Start {
    if model.tree.roots.contains(&path) {
        return Start::Container(path);
    }
    if path.to_fs(&model.tree.root).is_dir() {
        graph.ensure(path.clone(), NodeKind::Directory);
        return if model.knowledge_of(&path).is_some() {
            Start::Container(path)
        } else {
            Start::Outside(path)
        };
    }
    let key = NodeKey::Path(path.clone());
    if graph.node(&key).is_none() {
        graph.ensure(path, NodeKind::File);
    }
    Start::Node(key)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        graph::build,
        test_support::{fresh_dir, model_at, write},
    };

    fn temp(name: &str) -> PathBuf {
        let dir = fresh_dir(&format!("start-{name}"));
        for (rel, text) in [
            (
                "knowledge/manifest.sokf.yaml",
                "sokf: \"0.6\"\nname: k\nscope: test\n",
            ),
            (
                "knowledge/core/a.md",
                "---\ntype: Note\nid: a\nresource: /src/a.rs\n---\n\n# A\n",
            ),
            (
                "knowledge/core/b.md",
                "---\ntype: Note\nid: dup\n---\n\n# B\n",
            ),
            ("knowledge/c.md", "---\ntype: Note\nid: dup\n---\n\n# C\n"),
            (
                "knowledge/core.md",
                "---\ntype: Note\nid: core\n---\n\n# Core\n",
            ),
            (
                "other/manifest.sokf.yaml",
                "sokf: \"0.6\"\nname: o\nscope: test\n",
            ),
            ("other/n.md", "---\ntype: Note\nid: a\n---\n\n# N\n"),
            ("src/a.rs", ""),
            ("src/plain.rs", ""),
            ("docs/x.txt", ""),
        ] {
            write(&dir, rel, text);
        }
        std::fs::create_dir_all(dir.join("knowledge/empty")).unwrap();
        dir
    }

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    // sokf-tests: CLI-7_AC-4
    // sokf-tests: CLI-7_AC-19
    // sokf-tests: CLI-7_AC-20
    // sokf-tests: CLI-7_AC-29
    // sokf-tests: CLI-1_AC-7
    // sokf-tests: CLI-1_AC-8
    #[test]
    fn each_reading_and_each_refusal() {
        let dir = temp("reading");
        let m = model_at(&dir, &["/knowledge"]);
        let mut g = build(&m);
        let mut start = |arg: &str, cwd: &Path| resolve_start(&mut g, &m, None, cwd, arg);
        // An id that identifies exactly one document starts at that document.
        assert_eq!(
            start("a", &dir).unwrap(),
            Start::Node(NodeKey::Path(p("/knowledge/core/a.md")))
        );
        // A root and a directory in a root each start as a container, in
        // both spellings.
        assert_eq!(
            start("knowledge", &dir).unwrap(),
            Start::Container(p("/knowledge"))
        );
        assert_eq!(
            start("/knowledge", &dir).unwrap(),
            Start::Container(p("/knowledge"))
        );
        assert_eq!(
            start("core/", &dir.join("knowledge")).unwrap(),
            Start::Container(p("/knowledge/core"))
        );
        assert_eq!(
            start("./core", &dir.join("knowledge")).unwrap(),
            Start::Container(p("/knowledge/core"))
        );
        assert_eq!(
            start("empty", &dir.join("knowledge")).unwrap(),
            Start::Container(p("/knowledge/empty"))
        );
        // A document given by its path, a file that an edge points to, and
        // a plain file each start at their own node.
        assert_eq!(
            start("core.md", &dir.join("knowledge")).unwrap(),
            Start::Node(NodeKey::Path(p("/knowledge/core.md")))
        );
        assert_eq!(
            start("src/a.rs", &dir).unwrap(),
            Start::Node(NodeKey::Path(p("/src/a.rs")))
        );
        assert_eq!(
            start("src/plain.rs", &dir).unwrap(),
            Start::Node(NodeKey::Path(p("/src/plain.rs")))
        );
        // A directory outside every knowledge store, the repository root
        // among them, starts outside the stores.
        assert_eq!(start("src", &dir).unwrap(), Start::Outside(p("/src")));
        assert_eq!(start(".", &dir).unwrap(), Start::Outside(p("/")));
        // `resolve_start` refuses a duplicate id, an id that is also a path,
        // and a word that resolves to nothing.
        for (arg, needle) in [
            (
                "dup",
                "identifies more than one document: /knowledge/c.md, /knowledge/core/b.md",
            ),
            (
                "core",
                "is both an id (/knowledge/core.md) and a path (/knowledge/core)",
            ),
            ("nothing", "neither an id nor a path"),
            ("/knowledge/gone.md", "neither an id nor a path"),
        ] {
            let err = start(arg, &dir.join("knowledge")).unwrap_err();
            assert!(matches!(err, Error::Start(_)), "{arg}: {err}");
            assert!(err.to_string().contains(needle), "{arg}: {err}");
        }
        // The graph holds the nodes that the starts resolved to, each with
        // its kind.
        assert_eq!(
            g.kind(&NodeKey::Path(p("/knowledge/empty"))),
            Some(NodeKind::Directory)
        );
        assert_eq!(g.kind(&NodeKey::Path(p("/src"))), Some(NodeKind::Directory));
        assert_eq!(
            g.kind(&NodeKey::Path(p("/src/plain.rs"))),
            Some(NodeKind::File)
        );
        assert_eq!(g.kind(&NodeKey::Path(p("/src/a.rs"))), Some(NodeKind::File));
        assert_eq!(Start::Outside(p("/src")).key(), NodeKey::Path(p("/src")));
        assert_eq!(Start::Node(NodeKey::Missing("x".into())).path(), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-8_AC-6
    // sokf-tests: CLI-8_AC-9
    // sokf-tests: CLI-8_AC-11
    // sokf-tests: CLI-7_AC-30
    #[test]
    fn references_resolve_across_roots_or_refuse() {
        let dir = temp("ids");
        let m = model_at(&dir, &["/knowledge", "/other"]);
        assert_eq!(
            resolve_id(&m, None, "core").unwrap(),
            p("/knowledge/core.md")
        );
        let err = resolve_id(&m, None, "a").unwrap_err();
        assert!(matches!(err, Error::Locate(_)));
        assert_eq!(
            err.to_string(),
            "locate: `a` identifies more than one document: /knowledge/core/a.md, /other/n.md. Write `k:a` or `o:a`"
        );
        assert_eq!(
            resolve_id(&m, None, "zz").unwrap_err().to_string(),
            "locate: `zz` identifies no document"
        );
        // Each form of a reference resolves: a store given by its whole
        // identity or by its name, the `sokf:` scheme, and a fragment.
        for form in ["o:a", "test/o:a", "sokf:o:a", "o:a#n"] {
            assert_eq!(
                resolve_id(&m, None, form).unwrap(),
                p("/other/n.md"),
                "{form}"
            );
        }
        assert!(resolve_id(&m, None, "nope:a").is_err());
        assert!(is_key_reference("#k") && is_key_reference("sokf:o:#k"));
        assert!(!is_key_reference("o:a#k") && !is_key_reference("a"));
        // `resolve_start` also resolves an id across every root.
        let mut g = build(&m);
        let err = resolve_start(&mut g, &m, None, &dir, "a").unwrap_err();
        assert!(err.to_string().contains("/other/n.md"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-7_AC-31
    // sokf-tests: CLI-7_AC-97
    // sokf-tests: CLI-7_AC-98
    // sokf-tests: CLI-7_AC-99
    // sokf-tests: CLI-7_AC-100
    #[test]
    fn a_key_of_document_scope_starts_when_one_document_captures_it() {
        use crate::grammar::KeyScope;
        let dir = temp("document-scope");
        let m = model_at(&dir, &["/knowledge", "/other"]);
        let mut keys = KeyIndex::default();
        for (key, path, scope) in [
            ("X-1", "/knowledge/core/a.md", KeyScope::Document),
            ("Y-1", "/knowledge/core.md", KeyScope::Document),
            ("Y-1", "/knowledge/core/a.md", KeyScope::Document),
            (
                "R-1",
                "/knowledge/core/a.md",
                KeyScope::Host("knowledge".into()),
            ),
            ("R-1", "/knowledge/core.md", KeyScope::Document),
        ] {
            keys.add(KeyEntry {
                key: key.into(),
                path: p(path),
                line: 3,
                heading_slug: None,
                scope,
                knowledge: m.knowledge_of(&p(path)).unwrap(),
                rule_id: None,
            });
        }
        let mut g = build(&m);
        for (path, slug) in [
            ("/knowledge/core/a.md", "x-1"),
            ("/knowledge/core/a.md", "y-1"),
            ("/knowledge/core.md", "y-1"),
        ] {
            g.add_key(&p(path), slug, &slug.to_uppercase(), 3, &[]);
        }
        let key = |s: &str| Start::Node(NodeKey::Key(s.into()));
        let mut start = |arg: &str| resolve_start(&mut g, &m, Some(&keys), &dir, arg);
        // One document captures `X-1`, so `#x-1` starts at the key, in each
        // spelling, and the note names the document. The id `a` is also an
        // id of the store `o`, so the reference gives the store.
        for arg in ["#x-1", "#X-1", "k:#x-1", "sokf:#x-1"] {
            assert_eq!(
                start(arg).unwrap(),
                key("/knowledge/core/a.md#x-1"),
                "{arg}"
            );
        }
        assert_eq!(
            note(&m, Some(&keys), "#x-1").as_deref(),
            Some("`#x-1` is unique only within `k:a`, so link it as `k:a#x-1`")
        );
        // `<id>#<key>` starts at the key that the document captures, and at
        // the document when the fragment names no key of it.
        assert_eq!(start("core#y-1").unwrap(), key("/knowledge/core.md#y-1"));
        assert_eq!(
            start("core#nothing").unwrap(),
            Start::Node(NodeKey::Path(p("/knowledge/core.md")))
        );
        // A key of knowledge scope keeps its start, with no note.
        assert_eq!(
            start("#r-1").unwrap(),
            Start::Node(NodeKey::Path(p("/knowledge/core/a.md")))
        );
        assert_eq!(note(&m, Some(&keys), "#r-1"), None);
        assert_eq!(note(&m, Some(&keys), "core#y-1"), None);
        // Two documents capture `Y-1`, so the error lists the reference of
        // each, in the order of their paths. No document of the store `o`
        // captures it, and no document captures `#none`.
        for (arg, message) in [
            (
                "#y-1",
                "graph: `#y-1` is a key of document scope that more than one document captures. Write `core#y-1` or `k:a#y-1`",
            ),
            (
                "o:#y-1",
                "graph: `o:#y-1` is neither an id nor a path in the repository",
            ),
            (
                "#none",
                "graph: `#none` is neither an id nor a path in the repository",
            ),
        ] {
            let err = start(arg).unwrap_err();
            assert!(matches!(err, Error::Start(_)), "{arg}: {err}");
            assert_eq!(err.to_string(), message);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-7_AC-99
    #[test]
    fn the_error_lists_ten_references_and_counts_the_rest() {
        let forms = |n: usize| (1..=n).map(|i| format!("`n{i}#y-1`")).collect::<Vec<_>>();
        let ten = (1..=10)
            .map(|i| format!("`n{i}#y-1`"))
            .collect::<Vec<_>>()
            .join(" or ");
        assert_eq!(capped(&forms(2)), "`n1#y-1` or `n2#y-1`");
        assert_eq!(capped(&forms(10)), ten);
        assert_eq!(capped(&forms(11)), format!("{ten}, and 1 more"));
        assert_eq!(capped(&forms(103)), format!("{ten}, and 93 more"));
    }

    // sokf-tests: CLI-7_AC-28
    #[test]
    fn a_word_shaped_as_a_path_is_read_only_as_a_path() {
        let dir = temp("path-shaped");
        // Each id breaks the slug rules and has the shape of a path.
        for (rel, id) in [
            ("knowledge/core.md.md", "core.md"),
            ("knowledge/e.md", "core/"),
            ("knowledge/f.md", ".hidden"),
        ] {
            std::fs::write(
                dir.join(rel),
                format!("---\ntype: Note\nid: \"{id}\"\n---\n\n# X\n"),
            )
            .unwrap();
        }
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        let m = model_at(&dir, &["/knowledge"]);
        let mut g = build(&m);
        let mut start = |arg: &str, cwd: &Path| resolve_start(&mut g, &m, None, cwd, arg);
        let knowledge = dir.join("knowledge");
        assert_eq!(
            start("core.md", &knowledge).unwrap(),
            Start::Node(NodeKey::Path(p("/knowledge/core.md")))
        );
        assert_eq!(
            start("core/", &knowledge).unwrap(),
            Start::Container(p("/knowledge/core"))
        );
        assert_eq!(
            start(".hidden", &dir).unwrap(),
            Start::Outside(p("/.hidden"))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
