//! This module resolves the reference of a marker (sokf-format-spec §9.5).
//! A marker belongs to no store, so a reference that names no store
//! resolves in every store. A bare key resolves only among the keys unique
//! in the repository.

use std::collections::BTreeSet;

use crate::{
    Model,
    markdown::{is_slug, slug},
    model::{
        Query,
        keys::{self, KeyIndex, Label, Resolved, Unresolved},
    },
};

/// `Reference` is the reference of a marker, classified by its spelling
/// (SOKF_FORMAT_146).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference<'a> {
    /// A bare key, such as `CLI-1_AC-2`.
    Key(&'a str),
    /// A label, in the form that follows `sokf:` in a link.
    Label(Label<'a>),
}

impl<'a> Reference<'a> {
    /// `parse` classifies a reference by its form. A target with a store or
    /// a fragment is a label, and so is a lowercase kebab-case word. Any
    /// other bare word is a key.
    // sokf-implements: SOKF_FORMAT_146
    pub fn parse(target: &'a str) -> Self {
        if target.contains(['#', ':']) {
            Reference::Label(Label::parse(target))
        } else if is_slug(target) {
            Reference::Label(Label {
                store: None,
                id: Some(target),
                fragment: None,
            })
        } else {
            Reference::Key(target)
        }
    }
}

/// `Unresolvable` says why a reference resolves to nothing: a message and
/// the criterion that the reference breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolvable {
    /// The message says what the reference points to. It follows the
    /// marker in the finding.
    pub message: String,
    /// `criterion` is the criterion that the reference breaks.
    pub criterion: &'static str,
}

impl Unresolvable {
    fn new(message: impl Into<String>, criterion: &'static str) -> Self {
        Unresolvable {
            message: message.into(),
            criterion,
        }
    }
}

/// `resolve` resolves the reference of a marker. It records each lookup
/// against the watched marker file, so that sokf checks the marker file
/// again when anything that a lookup has read changes.
// sokf-implements: SOKF_FORMAT_145
// sokf-implements: SOKF_FORMAT_147
// sokf-implements: SOKF_FORMAT_148
// sokf-implements: SOKF_FORMAT_149
// sokf-implements: SOKF_FORMAT_150
pub fn resolve(model: &Model, keys: &KeyIndex, target: &str) -> Result<Resolved, Unresolvable> {
    match Reference::parse(target) {
        Reference::Key(key) => bare_key(model, keys, key),
        Reference::Label(label) => {
            let stores = match label.store {
                Some(store) => vec![store_named(model, store)?],
                None => (0..model.knowledges.len()).collect(),
            };
            match label.id {
                Some(id) if !id.is_empty() => by_id(model, keys, &stores, label),
                _ => by_fragment(model, keys, &stores, label),
            }
        }
    }
}

/// `bare_key` resolves a bare key to the only key in the repository with
/// the same slug.
fn bare_key(model: &Model, keys: &KeyIndex, key: &str) -> Result<Resolved, Unresolvable> {
    let s = slug(key);
    model.note(Query::KeyRepository(s.clone()));
    match keys.repository.get(&s).map(Vec::as_slice).unwrap_or(&[]) {
        [one] => Ok(Resolved {
            knowledge: one.knowledge,
            path: one.path.clone(),
            slug: Some(s),
        }),
        [] => Err(Unresolvable::new(
            format!(
                "refers to no key with the slug `{s}` that a schema declares unique in the repository"
            ),
            "SOKF_FORMAT_147",
        )),
        _ => Err(Unresolvable::new(
            format!("refers to the key `{s}`, which more than one document captures"),
            "SOKF_FORMAT_145",
        )),
    }
}

/// `store_named` finds the store that the reference of a marker names. The
/// reference writes the store either as `scope/name` or as a `name` used by
/// only one store across all scopes.
fn store_named(model: &Model, store: &str) -> Result<usize, Unresolvable> {
    let hits: BTreeSet<usize> = (0..model.knowledges.len())
        .filter_map(|k| model.store(k, store).map(|h| h.knowledge))
        .collect();
    let mut hits = hits.into_iter();
    match (hits.next(), hits.next()) {
        (Some(k), None) => Ok(k),
        (None, _) => Err(Unresolvable::new(
            format!(
                "refers to the store `{store}`, which is not a knowledge store in this repository"
            ),
            "SOKF_FORMAT_145",
        )),
        (Some(_), Some(_)) => Err(Unresolvable::new(
            format!(
                "refers to the store `{store}`, which exists in more than one scope. Write it as `SCOPE/{store}`, where `SCOPE` is the scope of the store"
            ),
            "SOKF_FORMAT_145",
        )),
    }
}

/// `by_id` resolves an id, with its fragment if it has one, in the one
/// store of `stores` that holds the id.
fn by_id(
    model: &Model,
    keys: &KeyIndex,
    stores: &[usize],
    label: Label<'_>,
) -> Result<Resolved, Unresolvable> {
    let id = label.id.unwrap_or_default();
    let holding: Vec<usize> = stores
        .iter()
        .copied()
        .filter(|&k| !model.ids(k, id).is_empty())
        .collect();
    let k = match holding.as_slice() {
        [k] => *k,
        [] => {
            return Err(Unresolvable::new(
                format!("refers to the id `{id}`, which no document carries"),
                "SOKF_FORMAT_145",
            ));
        }
        many => {
            let names: Vec<String> = many.iter().map(|&k| store_name(model, k)).collect();
            return Err(Unresolvable::new(
                format!(
                    "refers to the id `{id}`, which more than one store holds ({}). Put the store's name and a colon before the id, as in `STORE:{id}`, where `STORE` is the name of the store",
                    names.join(" and ")
                ),
                "SOKF_FORMAT_149",
            ));
        }
    };
    in_store(model, keys, k, label.without_store())
}

/// `by_fragment` resolves a bare fragment to the one key that it names
/// across `stores`.
fn by_fragment(
    model: &Model,
    keys: &KeyIndex,
    stores: &[usize],
    label: Label<'_>,
) -> Result<Resolved, Unresolvable> {
    let mut found: Vec<Resolved> = Vec::new();
    let mut first_error = None;
    for &k in stores {
        match in_store(model, keys, k, label.without_store()) {
            Ok(r) => {
                if !found.contains(&r) {
                    found.push(r);
                }
            }
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(first_error.unwrap_or_else(|| {
            Unresolvable::new("refers to no document and no key", "SOKF_FORMAT_145")
        })),
        _ => Err(Unresolvable::new(
            format!(
                "refers to the key `{key}`, which more than one store captures. Put the store's name and a colon before the `#`, as in `STORE:#{key}`, where `STORE` is the name of the store",
                key = label.fragment.unwrap_or_default()
            ),
            "SOKF_FORMAT_145",
        )),
    }
}

/// `in_store` resolves a label in store `k`. When the fragment has the
/// form of a key, `in_store` resolves the label again with the fragment's
/// slug (SOKF_FORMAT_150).
fn in_store(
    model: &Model,
    keys: &KeyIndex,
    k: usize,
    label: Label<'_>,
) -> Result<Resolved, Unresolvable> {
    keys::resolve_slugged(model, keys, k, label)
        .0
        .map_err(|u| Unresolvable::new(reason(&u), "SOKF_FORMAT_145"))
}

/// `reason` gives the message for a marker whose label resolves to
/// nothing. The message says what the label points to.
fn reason(u: &Unresolved) -> String {
    match u {
        Unresolved::NoStore(store) => {
            format!("refers to the store `{store}`, which is not a knowledge store in this repository")
        }
        Unresolved::NoDocument => "resolves to no document".to_string(),
        Unresolved::DuplicateId => {
            "refers to an id that more than one document of its store carries".to_string()
        }
        Unresolved::NoFragment | Unresolved::KeyForm(_) => {
            "refers to a fragment that is neither a captured key nor a heading".to_string()
        }
        Unresolved::BareNotFound => {
            "refers to no key that a schema declares unique in the store or in the repository"
                .to_string()
        }
        Unresolved::BareAmbiguous => "refers to a key that more than one document captures".to_string(),
        Unresolved::BareDocumentScope => {
            "refers to a key that is unique only within its own document, so a marker must identify the document too"
                .to_string()
        }
    }
}

/// `store_name` gives the name that a finding uses for store `k`: the name
/// in the store's identity, or, when it has no identity, its root path.
fn store_name(model: &Model, k: usize) -> String {
    let knowledge = &model.knowledges[k];
    knowledge
        .identity()
        .map(|i| format!("`{}`", i.name))
        .unwrap_or_else(|| format!("the store at `{}`", knowledge.root))
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: SOKF_FORMAT_146
    #[test]
    fn a_reference_is_read_by_its_spelling() {
        let label = |store, id, fragment| {
            Reference::Label(Label {
                store,
                id,
                fragment,
            })
        };
        assert_eq!(Reference::parse("CLI-1_AC-2"), Reference::Key("CLI-1_AC-2"));
        assert_eq!(
            Reference::parse("SOKF_FORMAT_96"),
            Reference::Key("SOKF_FORMAT_96")
        );
        assert_eq!(
            Reference::parse("design-cache"),
            label(None, Some("design-cache"), None)
        );
        assert_eq!(
            Reference::parse("contract-cli#CLI-1_AC-2"),
            label(None, Some("contract-cli"), Some("CLI-1_AC-2"))
        );
        assert_eq!(
            Reference::parse("sokf-spec:#sokf_format_96"),
            label(Some("sokf-spec"), None, Some("sokf_format_96"))
        );
        assert_eq!(
            Reference::parse("acme/common:glossary"),
            label(Some("acme/common"), Some("glossary"), None)
        );
        // A lowercase key with an underscore isn't a slug, so it's a key, not an id.
        assert_eq!(Reference::parse("cli_p-5"), Reference::Key("cli_p-5"));
    }
}
