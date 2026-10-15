//! This module defines the identities of stores and the qualified ids that
//! refer to them (SOKF_FORMAT_10, SOKF_FORMAT_64). The identity of a store is
//! `<scope>/<name>`. To refer to another store, a document puts that store's
//! identity in front of the id it would use inside that store. Within one
//! scope, the name of the store is enough.

use std::fmt;

use super::{Document, Knowledge, Located, Model};
use crate::RepoPath;
use crate::markdown::is_slug;

/// `Identity` is the identity of a store, `<scope>/<name>` (SOKF_FORMAT_10).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identity {
    /// `scope` identifies the owner of the store.
    pub scope: String,
    /// `name` is the name of the store within its scope.
    pub name: String,
}

impl Identity {
    /// `parse` reads `<scope>/<name>`, and returns `None` unless both parts
    /// are slugs.
    pub fn parse(text: &str) -> Option<Self> {
        let (scope, name) = text.split_once('/')?;
        Identity::new(scope, name)
    }

    /// `new` returns the identity `scope/name`, or `None` unless both parts
    /// are slugs.
    fn new(scope: &str, name: &str) -> Option<Self> {
        (is_slug(scope) && is_slug(name)).then(|| Identity {
            scope: scope.to_string(),
            name: name.to_string(),
        })
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.scope, self.name)
    }
}

impl Knowledge {
    /// `identity` returns the identity of the store, when its manifest
    /// declares a `scope` and a `name` that are both slugs.
    pub fn identity(&self) -> Option<Identity> {
        let m = self.manifest.as_ref()?;
        Identity::new(m.scope.as_deref()?, m.name.as_deref()?)
    }

    /// `former` returns the former identities of the store, which its
    /// manifest lists in `renamed-from`. It skips an entry that is not a
    /// valid identity.
    pub fn former(&self) -> Vec<Identity> {
        self.manifest
            .iter()
            .flat_map(|m| &m.renamed_from)
            .filter_map(|(old, _)| Identity::parse(old))
            .collect()
    }
}

/// `StoreHit` is the store that a qualified id refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreHit {
    /// `knowledge` is the index of the knowledge store.
    pub knowledge: usize,
    /// `former` is true when the id uses a former identity of the store
    /// (`renamed-from`).
    pub former: bool,
}

/// `IdentityRow` holds the current and former identities of one store as
/// text, in the form that the cache keeps (CACHE-1_AC-16).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdentityRow {
    /// `knowledge` is the index of the knowledge store.
    pub knowledge: usize,
    /// `identity` is the identity, `scope/name`, or an empty string when the
    /// manifest declares none.
    pub identity: String,
    /// `former` holds the former identities, one `scope/name` on each line.
    pub former: String,
}

impl Model {
    /// `identity_rows` returns the current and former identities of every
    /// store, in the form that the cache compares (CACHE-1_AC-16).
    pub fn identity_rows(&self) -> Vec<IdentityRow> {
        self.knowledges
            .iter()
            .enumerate()
            .map(|(k, kn)| IdentityRow {
                knowledge: k,
                identity: kn.identity().map(|i| i.to_string()).unwrap_or_default(),
                former: kn
                    .former()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            })
            .collect()
    }

    /// `store` finds the store that `written` refers to from a document of
    /// store `citing` (SOKF_FORMAT_64). It looks up a `name` among the
    /// stores in the scope of the citing store, and a `scope/name` among
    /// every store. It matches the current identities first, then the
    /// former ones. When two stores share an identity, `store` returns
    /// `None`, because validation reports that clash on their manifests.
    ///
    /// `store` records a single dependency on the identities of all stores
    /// (CACHE-1_AC-16), not one for each store it reads. A finer dependency
    /// would save little, for two reasons. Renaming a store changes every
    /// qualified id that refers to the store, and few documents need `store`.
    // sokf-implements: SOKF_FORMAT_64
    // sokf-implements: CACHE-1_AC-16
    // sokf-implements: CONF-2_AC-3
    pub fn store(&self, citing: usize, written: &str) -> Option<StoreHit> {
        self.note(super::Query::Identities);
        let wanted = match written.split_once('/') {
            Some(_) => Identity::parse(written)?,
            None => Identity {
                scope: self.knowledges.get(citing)?.identity()?.scope,
                name: written.to_string(),
            },
        };
        let (hits, former) = self.matching(|i| *i == wanted);
        match hits.as_slice() {
            [knowledge] => Some(StoreHit {
                knowledge: *knowledge,
                former,
            }),
            _ => None,
        }
    }

    /// `matching` returns the stores whose current identity `wanted`
    /// accepts. When there are none, it returns the stores with a former
    /// identity that `wanted` accepts, and says so with `true`.
    fn matching(&self, wanted: impl Fn(&Identity) -> bool) -> (Vec<usize>, bool) {
        let n = self.knowledges.len();
        let current: Vec<usize> = (0..n)
            .filter(|&k| self.knowledges[k].identity().is_some_and(|i| wanted(&i)))
            .collect();
        if !current.is_empty() {
            return (current, false);
        }
        let former = (0..n)
            .filter(|&k| self.knowledges[k].former().iter().any(&wanted))
            .collect();
        (former, true)
    }

    /// `stores_named` returns the stores that `written` names on the command
    /// line, where no citing store gives a scope. It matches a `scope/name`
    /// against the whole identity and a `name` against the name of every
    /// store. It matches the current identities first, then the former ones.
    // sokf-implements: CLI-7_AC-64
    pub fn stores_named(&self, written: &str) -> Vec<usize> {
        let matches = |identity: &Identity| match written.split_once('/') {
            Some(_) => identity.to_string() == written,
            None => identity.name == written,
        };
        self.matching(matches).0
    }

    /// `store_name` returns the shortest name of store `k` on the command
    /// line: its `name`, or its whole identity when another store shares
    /// that name. It returns `None` when the store has no identity.
    pub fn store_name(&self, k: usize) -> Option<String> {
        let identity = self.knowledges.get(k)?.identity()?;
        let shared = self
            .knowledges
            .iter()
            .filter(|o| o.identity().is_some_and(|i| i.name == identity.name))
            .count()
            > 1;
        Some(if shared {
            identity.to_string()
        } else {
            identity.name
        })
    }

    /// `document_in` finds the document that `id` refers to in `store`.
    /// `store` is a store prefix as a document of store `citing` wrote it. It
    /// returns the document with the index of the knowledge store that
    /// holds it. When `store` is `None`, it looks in `citing`.
    pub fn document_in(
        &self,
        citing: usize,
        store: Option<&str>,
        id: &str,
    ) -> Option<(usize, &Document)> {
        let k = match store {
            None => citing,
            Some(s) => self.store(citing, s)?.knowledge,
        };
        self.document(k, id).map(|d| (k, d))
    }

    /// `id_at` returns the id of the document or schema at `path`, in any
    /// store, with the index of its knowledge store.
    pub fn id_at(&self, path: &RepoPath) -> Option<(usize, String)> {
        match self.by_path(path)? {
            Located::Document(k, d) => Some((k, d.id()?.to_string())),
            Located::Schema(k, s) => Some((k, s.id())),
            _ => None,
        }
    }

    /// `path_label` returns the canonical `sokf:` label that a document in
    /// store `citing` should use for a link to `path` with `fragment`
    /// (SOKF_FORMAT_65). The label depends on the file at `path`:
    ///
    /// - For a document, the label is its id with the fragment.
    /// - For a sidecar, the label is its id with the fragment, and with an
    ///   empty fragment when the link has none, because the path refers to
    ///   the sidecar file and not to the asset.
    /// - For an asset, the label is the id of its sidecar, which refers to
    ///   the asset.
    ///
    /// `path_label` returns `None` for any other file, and for an asset
    /// path with a fragment.
    pub fn path_label(
        &self,
        citing: usize,
        path: &RepoPath,
        fragment: Option<&str>,
    ) -> Option<String> {
        if let Some((k, id)) = self.id_at(path) {
            let fragment = match self.asset_of(path) {
                Some(_) => Some(fragment.unwrap_or("")),
                None => fragment,
            };
            return self.canonical_label(citing, k, Some(&id), fragment);
        }
        let (k, asset) = self.asset_at(path)?;
        if fragment.is_some() {
            return None;
        }
        self.note(super::Query::Path(asset.sidecar_path()));
        let id = self.knowledges[k].documents[asset.sidecar?].id()?;
        self.canonical_label(citing, k, Some(id), None)
    }

    /// `prefix` returns the store prefix that a document in store `citing`
    /// should put before an id in store `target`. It is the shortest
    /// unambiguous prefix (SOKF_FORMAT_65). The prefix is empty within one store,
    /// `name:` within one scope, and `scope/name:` otherwise. `prefix`
    /// returns `None` when the two stores differ and either has no
    /// identity.
    pub fn prefix(&self, citing: usize, target: usize) -> Option<String> {
        if citing == target {
            return Some(String::new());
        }
        self.note(super::Query::Identities);
        let from = self.knowledges.get(citing)?.identity()?;
        let to = self.knowledges.get(target)?.identity()?;
        Some(if from.scope == to.scope {
            format!("{}:", to.name)
        } else {
            format!("{to}:")
        })
    }

    /// `canonical_label` returns the canonical `sokf:` label that a document
    /// in store `citing` should use for `id` and `fragment` in store `target`
    /// (SOKF_FORMAT_65).
    pub fn canonical_label(
        &self,
        citing: usize,
        target: usize,
        id: Option<&str>,
        fragment: Option<&str>,
    ) -> Option<String> {
        self.canonical_include(citing, target, id.unwrap_or(""), fragment)
            .map(|target| format!("sokf:{target}"))
    }

    /// `canonical_include` returns the canonical target that an include
    /// marker in store `citing` should use for `id` and `fragment` in store
    /// `target` (SOKF_FORMAT_65).
    pub fn canonical_include(
        &self,
        citing: usize,
        target: usize,
        id: &str,
        fragment: Option<&str>,
    ) -> Option<String> {
        let prefix = self.prefix(citing, target)?;
        Some(format!(
            "{prefix}{id}{}",
            fragment.map(|f| format!("#{f}")).unwrap_or_default()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::split_store;

    #[test]
    fn an_identity_is_two_slugs() {
        assert_eq!(
            Identity::parse("ncoderz/sokf-spec").map(|i| i.to_string()),
            Some("ncoderz/sokf-spec".into())
        );
        assert_eq!(Identity::parse("sokf-spec"), None);
        assert_eq!(Identity::parse("Ncoderz/x"), None);
        assert_eq!(Identity::parse("a/b/c"), None);
    }

    #[test]
    fn a_qualified_id_splits_at_its_store() {
        assert_eq!(split_store("glossary"), (None, "glossary"));
        assert_eq!(
            split_store("sokf-spec:glossary"),
            (Some("sokf-spec"), "glossary")
        );
        assert_eq!(split_store("acme/common:"), (Some("acme/common"), ""));
    }
}
