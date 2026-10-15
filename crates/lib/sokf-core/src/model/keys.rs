//! This module indexes the captured keys by scope, and resolves `sokf:`
//! labels and their fragments as the format specification defines.

use std::collections::HashMap;

use super::{Model, Query};
use crate::{
    RepoPath,
    grammar::KeyScope,
    markdown::{Body, link_headings, slug},
};

/// A `KeyEntry` is one captured key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEntry {
    /// The key.
    pub key: String,
    /// The document that captures it.
    pub path: RepoPath,
    /// The line that captures the key.
    pub line: usize,
    /// The slug of the heading, when a heading captures the key.
    pub heading_slug: Option<String>,
    /// The scope that the schema declares for the key.
    pub scope: KeyScope,
    /// The knowledge of the document that captures the key.
    pub knowledge: usize,
    /// The `rule-id` of the key. It is the nearest `rule-id` around the rule
    /// that captures the key. `for-each-key-of` and `cited-by` refer
    /// to this `rule-id`.
    pub rule_id: Option<String>,
}

/// A `KeyIndex` holds the keys by the scope in which each key must be
/// unique (the grammar's `unique`). It stores each key under its slug. The
/// slug is the id of the key's anchor, and a fragment refers to a key by
/// that slug (SOKF_FORMAT_72).
#[derive(Debug, Clone, Default)]
pub struct KeyIndex {
    /// Every key of every document, by path and by slug.
    pub document: HashMap<(RepoPath, String), Vec<KeyEntry>>,
    /// The keys of knowledge scope, by knowledge and by slug.
    pub knowledge: HashMap<(usize, String), Vec<KeyEntry>>,
    /// The keys of repository scope, by slug.
    pub repository: HashMap<String, Vec<KeyEntry>>,
}

impl KeyIndex {
    /// `add` records a captured key under its document and its scope.
    pub fn add(&mut self, entry: KeyEntry) {
        let s = slug(&entry.key);
        match &entry.scope {
            KeyScope::Host(scope) if scope == "knowledge" => self
                .knowledge
                .entry((entry.knowledge, s.clone()))
                .or_default()
                .push(entry.clone()),
            KeyScope::Host(scope) if scope == "repository" => self
                .repository
                .entry(s.clone())
                .or_default()
                .push(entry.clone()),
            _ => {}
        }
        self.document
            .entry((entry.path.clone(), s))
            .or_default()
            .push(entry);
    }

    /// `of_document` returns the keys that `path` captures.
    pub fn of_document<'a>(&'a self, path: &'a RepoPath) -> impl Iterator<Item = &'a KeyEntry> {
        self.document
            .iter()
            .filter(move |((p, _), _)| p == path)
            .flat_map(|(_, entries)| entries.iter())
    }
}

impl Extend<KeyEntry> for KeyIndex {
    fn extend<I: IntoIterator<Item = KeyEntry>>(&mut self, entries: I) {
        for entry in entries {
            self.add(entry);
        }
    }
}

/// A `Label` holds the three parts of a `sokf:` label: the store
/// (optional), the id (absent when the label is a bare fragment), and the
/// fragment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Label<'a> {
    /// The store, written as `name` or `scope/name`.
    pub store: Option<&'a str>,
    /// The document id.
    pub id: Option<&'a str>,
    /// The key or heading slug.
    pub fragment: Option<&'a str>,
}

impl<'a> Label<'a> {
    /// `with_fragment` returns the label with `fragment` in place of its own.
    pub fn with_fragment(self, fragment: Option<&'a str>) -> Self {
        Label { fragment, ..self }
    }

    /// `without_store` returns the label with no store, so that it resolves
    /// in the store that the caller chooses.
    pub fn without_store(self) -> Self {
        Label {
            store: None,
            ..self
        }
    }
}

/// A `Resolved` is the target of a `sokf:` label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The knowledge of the target.
    pub knowledge: usize,
    /// The target document.
    pub path: RepoPath,
    /// The fragment that the definition block appends to the path as
    /// `#slug`. It is the slug of the heading or key that the label refers to.
    pub slug: Option<String>,
}

/// An `Unresolved` states the reason that a `sokf:` label is a broken edge
/// (SOKF_FORMAT_62).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// The store matches no knowledge root, current or former. The variant
    /// holds the store as written in the label.
    NoStore(String),
    /// The id matches no document in the knowledge.
    NoDocument,
    /// The id matches more than one document.
    DuplicateId,
    /// The fragment matches no key and no heading.
    NoFragment,
    /// A bare fragment matches no key at repository or knowledge scope.
    BareNotFound,
    /// A bare fragment matches keys in more than one document.
    BareAmbiguous,
    /// A bare fragment matches only a key of document scope.
    BareDocumentScope,
    /// The fragment is the text of a key or a heading, not its slug. The
    /// variant holds the slug.
    KeyForm(String),
}

impl Unresolved {
    /// `message` returns the error message for `label`.
    pub fn message(&self, label: &str) -> String {
        match self {
            Unresolved::NoStore(store) => format!(
                "`[{label}]` refers to the store `{store}`, which is not a knowledge root of this repository"
            ),
            Unresolved::NoDocument => format!("`[{label}]` resolves to no document"),
            Unresolved::DuplicateId => {
                format!("more than one document has the id in `[{label}]`")
            }
            Unresolved::NoFragment => {
                format!(
                    "`[{label}]` refers to a fragment that is neither a captured key nor a heading"
                )
            }
            Unresolved::BareNotFound => format!(
                "`[{label}]` refers to a key that no document of its store captures at repository or knowledge scope"
            ),
            Unresolved::BareAmbiguous => {
                format!("`[{label}]` refers to a key that more than one document captures")
            }
            Unresolved::BareDocumentScope => {
                format!("`[{label}]` refers to a key of document scope only")
            }
            Unresolved::KeyForm(slug) => format!(
                "`[{label}]` gives the fragment as text, not as a slug. Use `#{slug}`, which `--fix` writes for you"
            ),
        }
    }
}

/// `resolve_in` resolves a fragment against the headings and keys of one
/// document, by slug (SOKF_FORMAT_71). The headings include each heading
/// inside a quote or a list item, and each heading has the slug that the
/// host gives it, so no two headings share one (SOKF_FORMAT_155). It returns the fragment itself, which the
/// definition block appends. When the fragment matches only by its text,
/// `resolve_in` returns `Unresolved::KeyForm` with the slug.
// sokf-implements: SOKF_FORMAT_71
// sokf-implements: SOKF_FORMAT_72
pub fn resolve_in(
    keys: &KeyIndex,
    path: &RepoPath,
    body: &Body,
    fragment: &str,
) -> Result<String, Unresolved> {
    let headings = link_headings(body);
    let found = |f: &str| {
        keys.document.contains_key(&(path.clone(), f.to_string()))
            || headings.iter().any(|h| h.slug == f)
    };
    if found(fragment) {
        return Ok(fragment.to_string());
    }
    let s = slug(fragment);
    if s != fragment && found(&s) {
        Err(Unresolved::KeyForm(s))
    } else {
        Err(Unresolved::NoFragment)
    }
}

/// `resolve` resolves a `sokf:` label from a document of `knowledge`: first
/// the store, then the id in that store, then the fragment. A bare fragment
/// with no store refers to a key in the store of the document that holds
/// the label. If no key there matches, but a key of repository scope in
/// another store does, the label resolves to that key, because that key is
/// unique. The canonical form of the label then includes that other store
/// (SOKF_FORMAT_65).
// sokf-implements: SOKF_FORMAT_61
// sokf-implements: SOKF_FORMAT_63
// sokf-implements: SOKF_FORMAT_64
// sokf-implements: SOKF_FORMAT_66
// sokf-implements: SOKF_FORMAT_70
// sokf-implements: FORMAT-2_AC-1
// sokf-implements: FORMAT-2_AC-3
pub fn resolve(
    model: &Model,
    keys: &KeyIndex,
    knowledge: usize,
    label: Label<'_>,
) -> Result<Resolved, Unresolved> {
    let target = match label.store {
        None => knowledge,
        Some(store) => {
            model
                .store(knowledge, store)
                .ok_or_else(|| Unresolved::NoStore(store.to_string()))?
                .knowledge
        }
    };
    if model.knowledges.get(target).is_none() {
        return Err(Unresolved::NoDocument);
    }
    match (label.id.filter(|i| !i.is_empty()), label.fragment) {
        (None, None) => Err(Unresolved::NoDocument),
        // `sokf:#`, with no id and an empty fragment, refers to nothing (SOKF_FORMAT_66).
        (None, Some("")) => Err(Unresolved::NoDocument),
        (Some(id), fragment) => {
            let doc = match model.ids(target, id).len() {
                0 => return Err(Unresolved::NoDocument),
                1 => model.document(target, id).ok_or(Unresolved::NoDocument)?,
                _ => return Err(Unresolved::DuplicateId),
            };
            // An empty fragment refers to the whole document (FORMAT-2_AC-3).
            let Some(fragment) = fragment.filter(|f| !f.is_empty()) else {
                return Ok(Resolved {
                    knowledge: target,
                    path: doc.path.clone(),
                    slug: None,
                });
            };
            resolve_in(keys, &doc.path, &doc.body, fragment).map(|slug| Resolved {
                knowledge: target,
                path: doc.path.clone(),
                slug: Some(slug),
            })
        }
        (None, Some(fragment)) => {
            let elsewhere = label.store.is_none();
            match bare(model, keys, target, fragment, elsewhere) {
                Err(Unresolved::BareNotFound) => {
                    let s = slug(fragment);
                    if s != fragment && bare(model, keys, target, &s, elsewhere).is_ok() {
                        Err(Unresolved::KeyForm(s))
                    } else {
                        Err(Unresolved::BareNotFound)
                    }
                }
                other => other,
            }
        }
    }
}

/// `resolve_slugged` resolves a label as `resolve` does. When the label
/// gives its fragment as text, not as a slug, `resolve_slugged` resolves
/// the label again with the fragment's slug, and returns that slug too.
pub fn resolve_slugged(
    model: &Model,
    keys: &KeyIndex,
    knowledge: usize,
    label: Label<'_>,
) -> (Result<Resolved, Unresolved>, Option<String>) {
    match resolve(model, keys, knowledge, label) {
        Err(Unresolved::KeyForm(s)) => {
            let resolved = resolve(model, keys, knowledge, label.with_fragment(Some(&s)));
            (resolved, Some(s))
        }
        other => (other, None),
    }
}

/// `bare` resolves a bare fragment, by its slug, to a key of `knowledge`
/// whose scope is the knowledge or the repository. When no such key exists
/// and `elsewhere` is set, it
/// resolves the fragment to a key of repository scope in another store.
fn bare(
    model: &Model,
    keys: &KeyIndex,
    knowledge: usize,
    fragment: &str,
    elsewhere: bool,
) -> Result<Resolved, Unresolved> {
    let found = |one: &KeyEntry| Resolved {
        knowledge: one.knowledge,
        path: one.path.clone(),
        slug: Some(fragment.to_string()),
    };
    model.note(Query::KeyRepository(fragment.to_string()));
    model.note(Query::KeyKnowledge {
        knowledge,
        slug: fragment.to_string(),
    });
    let repo = keys
        .repository
        .get(fragment)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let own: Vec<&KeyEntry> = repo.iter().filter(|e| e.knowledge == knowledge).collect();
    match own.as_slice() {
        [one] => return Ok(found(one)),
        [] => {}
        _ => return Err(Unresolved::BareAmbiguous),
    }
    let local = keys
        .knowledge
        .get(&(knowledge, fragment.to_string()))
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    match local {
        [one] => Ok(found(one)),
        [] => {
            if elsewhere {
                match repo {
                    [one] => return Ok(found(one)),
                    [] => {}
                    _ => return Err(Unresolved::BareAmbiguous),
                }
            }
            model.note(Query::KeyDocument {
                knowledge,
                slug: fragment.to_string(),
            });
            let document_only = keys
                .document
                .keys()
                .any(|(p, k)| k == fragment && model.knowledge_of(p) == Some(knowledge));
            if document_only {
                Err(Unresolved::BareDocumentScope)
            } else {
                Err(Unresolved::BareNotFound)
            }
        }
        _ => Err(Unresolved::BareAmbiguous),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn entry(key: &str, path: &str, scope: KeyScope, slug: Option<&str>) -> KeyEntry {
        KeyEntry {
            key: key.into(),
            path: p(path),
            line: 1,
            heading_slug: slug.map(str::to_string),
            scope,
            knowledge: 0,
            rule_id: None,
        }
    }

    #[test]
    fn indexes_by_scope() {
        let mut index = KeyIndex::default();
        index.add(entry("D", "/a.md", KeyScope::Document, None));
        index.add(entry(
            "K",
            "/a.md",
            KeyScope::Host("knowledge".into()),
            Some("k-heading"),
        ));
        index.add(entry(
            "R",
            "/b.md",
            KeyScope::Host("repository".into()),
            None,
        ));
        assert_eq!(index.document.len(), 3);
        assert_eq!(index.knowledge.len(), 1);
        assert_eq!(index.repository.len(), 1);
        assert_eq!(index.of_document(&p("/a.md")).count(), 2);
        assert_eq!(index.of_document(&p("/c.md")).count(), 0);
        for u in [
            Unresolved::NoStore("s".into()),
            Unresolved::NoDocument,
            Unresolved::DuplicateId,
            Unresolved::NoFragment,
            Unresolved::BareNotFound,
            Unresolved::BareAmbiguous,
            Unresolved::BareDocumentScope,
            Unresolved::KeyForm("k".into()),
        ] {
            assert!(u.message("sokf:x").contains("[sokf:x]"));
        }
    }
}

#[cfg(test)]
mod fragment_tests {
    use super::*;
    use crate::{Config, FsLoader, RepoRoot, collect};

    // sokf-tests: FORMAT-2_AC-3
    #[test]
    fn an_empty_fragment_names_the_whole_document_and_a_bare_one_nothing() {
        let dir = crate::test_support::fresh_dir("keys-empty");
        std::fs::create_dir_all(dir.join("knowledge")).unwrap();
        std::fs::write(
            dir.join("knowledge/a.md"),
            "---\ntype: Note\nid: a\n---\n\n# A\n\n## Part\n",
        )
        .unwrap();
        let root = RepoRoot::new(dir.clone());
        let tree = collect(
            &root,
            &Config {
                roots: vec![RepoPath::parse("/knowledge").unwrap()],
                exclude: vec![],
                declined: Default::default(),
                markers: Default::default(),
            },
        )
        .unwrap();
        let model = crate::model::build(tree, &mut FsLoader::new(&root)).unwrap();
        let keys = KeyIndex::default();
        let label = |id, fragment| Label {
            store: None,
            id,
            fragment,
        };
        let path = RepoPath::parse("/knowledge/a.md").unwrap();
        assert_eq!(
            resolve(&model, &keys, 0, label(Some("a"), Some(""))),
            Ok(Resolved {
                knowledge: 0,
                path: path.clone(),
                slug: None
            })
        );
        assert_eq!(
            resolve(&model, &keys, 0, label(Some("a"), Some("part"))),
            Ok(Resolved {
                knowledge: 0,
                path,
                slug: Some("part".into())
            })
        );
        assert_eq!(
            resolve(&model, &keys, 0, label(None, Some(""))),
            Err(Unresolved::NoDocument)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
