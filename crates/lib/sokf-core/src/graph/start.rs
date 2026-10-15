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

/// `key_entry` returns the key that a reference to a key alone identifies,
/// with the key's slug, when the reference identifies exactly one document
/// (CLI-7_AC-74).
pub fn key_entry(model: &Model, keys: &KeyIndex, arg: &str) -> Option<(KeyEntry, String)> {
    if !is_key_reference(arg) {
        return None;
    }
    let [path] = paths_of_reference(model, Some(keys), arg).try_into().ok()?;
    let written = arg.split_once('#')?.1;
    [written.to_string(), crate::markdown::slug(written)]
        .into_iter()
        .find_map(|slug| {
            let entry = keys.document.get(&(path.clone(), slug.clone()))?.first()?;
            Some((entry.clone(), slug))
        })
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
    let stores: Vec<usize> = match store {
        None => (0..model.knowledges.len()).collect(),
        Some(s) => model.stores_named(s),
    };
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
            let (k, id) = match model.id_at(p) {
                Some(found) => found,
                None => {
                    // The run does not hold this document, so the id comes
                    // from the store's row.
                    let k = model.knowledge_of(p)?;
                    let id = model.fallback.as_ref()?.fields(p)?.id?;
                    (k, id)
                }
            };
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
        // For a reference to a key alone, the start is the key's node when
        // the graph has one.
        // sokf-implements: CLI-7_AC-32
        ([one], None) => Ok(Start::Node(
            keys.and_then(|k| key_entry(model, k, arg))
                .map(|(_, slug)| NodeKey::Key(format!("{one}#{slug}")))
                .filter(|k| graph.node(k).is_some())
                .unwrap_or_else(|| NodeKey::Path(one.clone())),
        )),
        ([], Some(p)) => Ok(path_start(graph, model, p)),
        ([], None) => Err(Error::Start(format!(
            "`{arg}` is neither an id nor a path in the repository"
        ))),
    }
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
    // sokf-tests: CLI-7_AC-31
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
