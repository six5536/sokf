//! This module reads the arguments that a user gives the graph commands,
//! `guide`, and `locate`: `resolve_start` reads the `START` of a graph command
//! (CLI-7), `resolve_document` reads the document that `guide` takes (CLI-17)
//! in the same way, and `resolve_id` reads the id that `locate` takes (CLI-8).

use std::path::Path;

use super::{Graph, NodeKey, NodeKind};
use crate::{
    Error, FileKind, Model, PathArg, RepoPath, Result,
    grammar::KeyScope,
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
    let (path, slug) = match paths_of_reference(model, Some(keys), arg).as_slice() {
        [one] => (one.clone(), None),
        [] => match document_scoped(model, keys, arg).as_slice() {
            [(one, slug)] => (one.clone(), Some(slug.clone())),
            _ => return None,
        },
        _ => return None,
    };
    // A key alone that names a key of repository or knowledge scope names
    // it by the first id that such a key has, as `paths_of_reference` found
    // the document, so a key of document scope under an earlier id of the
    // same document is not the one it names.
    let wide = slug.is_none() && is_key_reference(arg);
    slug.into_iter()
        .chain(std::iter::once(written.to_string()))
        .chain(crate::markdown::key_ids(written))
        .find_map(|slug| {
            let entry = keys
                .document
                .get(&(path.clone(), slug.clone()))?
                .iter()
                .find(|e| !wide || !matches!(e.scope, KeyScope::Document))?;
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
    // The lookup tries the key as written and then each id that it can
    // name, and keeps the documents of the first that some document has.
    std::iter::once(written.to_string())
        .chain(crate::markdown::key_ids(written))
        .map(|slug| {
            let mut found: Vec<(RepoPath, String)> = keys
                .document
                .keys()
                .filter(|(p, s)| {
                    *s == slug && model.knowledge_of(p).is_some_and(|k| stores.contains(&k))
                })
                .cloned()
                .collect();
            found.sort();
            found
        })
        .find(|found| !found.is_empty())
        .unwrap_or_default()
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
    if let Some(found) = model.id_at(path) {
        Some(found)
    } else {
        // The run does not hold this document, so the id comes from the
        // store's row.
        let k = model.knowledge_of(path)?;
        let id = model.fallback.as_ref()?.fields(path)?.id?;
        Some((k, id))
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
            // The index holds each key under its id, so the lookup tries the
            // key as written and then each id that it can name, and stops at
            // the first that some key has.
            std::iter::once(key.to_string())
                .chain(crate::markdown::key_ids(key))
                .map(|key| {
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
                .find(|paths| !paths.is_empty())
                .unwrap_or_default()
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

/// `Reading` is what an argument names when the tool reads it both as an id
/// and as a path, and exactly one reading resolves.
enum Reading {
    /// The document or schema that the argument identifies as an id, or as a
    /// reference to a key that the document captures.
    Id(RepoPath),
    /// The file or directory that the argument names as a path.
    Path(RepoPath),
}

/// `read` reads `arg` both as an `id` and as a path, and refuses it unless
/// exactly one reading resolves (CLI-7_AC-19, CLI-7_AC-20). The message of
/// a refusal says why, for the command to name.
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
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "a START that ends in a lowercase `.md` is path-shaped, as CLI-7_AC-28 says, so the comparison is case-sensitive on purpose"
)]
fn read(
    model: &Model,
    keys: Option<&KeyIndex>,
    cwd: &Path,
    arg: &str,
) -> std::result::Result<Reading, String> {
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
        ([_, ..], Some(p)) => Err(format!(
            "`{arg}` is both an id ({}) and a path ({p})",
            join(&ids)
        )),
        ([_, _, ..], None) => Err(format!(
            "`{arg}` identifies more than one document: {}{}",
            join(&ids),
            qualified(model, arg, &ids)
        )),
        ([one], None) => Ok(Reading::Id(one.clone())),
        ([], Some(p)) => Ok(Reading::Path(p)),
        ([], None) => {
            // A key that no document captures at repository or knowledge
            // scope can still be a key of document scope.
            let scoped = keys.map_or_else(Vec::new, |k| document_scoped(model, k, arg));
            match scoped.as_slice() {
                // sokf-implements: CLI-7_AC-98
                [(one, _)] => Ok(Reading::Id(one.clone())),
                // sokf-implements: CLI-7_AC-100
                [] => Err(format!(
                    "`{arg}` is neither an id nor a path in the repository"
                )),
                // sokf-implements: CLI-7_AC-99
                many => {
                    let forms: Vec<String> = many
                        .iter()
                        .map(|(p, slug)| match key_reference(model, p, slug) {
                            Some(r) => format!("`{r}`"),
                            None => p.to_string(),
                        })
                        .collect();
                    Err(format!(
                        "`{arg}` is a key of document scope that more than one document captures. Write {}",
                        capped(&forms)
                    ))
                }
            }
        }
    }
}

/// `resolve_start` reads `arg` as `read` does. When the start is a directory
/// or a repository file, the function adds it to the graph as a node.
pub fn resolve_start(
    graph: &mut Graph,
    model: &Model,
    keys: Option<&KeyIndex>,
    cwd: &Path,
    arg: &str,
) -> Result<Start> {
    match read(model, keys, cwd, arg).map_err(Error::Start)? {
        // For a reference to a key alone, and for `<id>#<key>` when the
        // document captures the key, the start is the key's node when the
        // graph has one.
        // sokf-implements: CLI-7_AC-32
        // sokf-implements: CLI-7_AC-97
        Reading::Id(one) => Ok(key_start(graph, model, keys, arg, &one)),
        Reading::Path(p) => Ok(path_start(graph, model, p)),
    }
}

/// `resolve_document` reads `arg` as `resolve_start` reads a `START`, and
/// returns the document or schema that it names. It refuses an argument
/// that names no document of a knowledge store, such as a directory, an
/// asset, or an additional file.
// sokf-implements: CLI-17_AC-1
// sokf-implements: CLI-17_AC-2
// sokf-implements: CLI-17_AC-3
pub fn resolve_document(
    model: &Model,
    keys: Option<&KeyIndex>,
    cwd: &Path,
    arg: &str,
) -> Result<RepoPath> {
    match read(model, keys, cwd, arg).map_err(Error::Guide)? {
        Reading::Id(path) => Ok(path),
        Reading::Path(path) => match model.tree.file(&path).map(|f| f.kind) {
            Some(FileKind::Document | FileKind::Schema) => Ok(path),
            _ => Err(Error::Guide(format!(
                "`{arg}` names {path}, which is no document of a knowledge store"
            ))),
        },
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
mod tests;
