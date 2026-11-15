//! This module resolves the `extends` of each store's manifest, the chain of
//! stores whose schemas a store inherits, and the schema that governs a type
//! in a store through that chain (`SOKF_SCHEMA_96`, `SOKF_SCHEMA_97`). It
//! also rebases the paths of an inherited schema onto the inheriting store
//! (`SOKF_SCHEMA_98`).

use super::{Identity, Knowledge, Model, Query, SchemaRef};
use crate::RepoPath;

/// A `Broken` says why the `extends` of a store's manifest gives the store
/// no chain (`SOKF_FORMAT_206`, `SOKF_FORMAT_207`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Broken {
    /// The `extends` names no knowledge store of the repository. The
    /// variant holds the value as written.
    NoStore(String),
    /// The `extends` names an identity that more than one store holds. The
    /// variant holds the value as written.
    Shared(String),
    /// The chain of `extends` from the store comes back to the store.
    Cycle,
}

/// `Chains` holds, for each store, its chain of `extends` and why it has
/// none, resolved once when the model is built.
#[derive(Debug, Clone, Default)]
pub struct Chains {
    /// The chain of each store: the store itself, the store that it
    /// extends, and so on. A chain stops at a store that extends nothing, at
    /// a broken store, which inherits nothing, and before a store that it
    /// already holds, so a broken store's chain holds only itself
    /// (FORMAT-5_AC-10).
    pub chains: Vec<Vec<usize>>,
    /// Why each store's `extends` is broken, or `None` (`broken`).
    pub broken: Vec<Option<Broken>>,
}

impl Chains {
    /// `of` resolves the chain of every store of `knowledges`.
    pub fn of(knowledges: &[Knowledge]) -> Self {
        let broken: Vec<Option<Broken>> = (0..knowledges.len())
            .map(|k| broken(knowledges, k))
            .collect();
        let chains = (0..knowledges.len())
            .map(|k| {
                let mut chain = vec![k];
                let mut at = k;
                while broken[at].is_none() {
                    match parent_of(knowledges, at) {
                        Ok(Some(p)) if !chain.contains(&p) => {
                            chain.push(p);
                            at = p;
                        }
                        _ => break,
                    }
                }
                chain
            })
            .collect();
        Chains { chains, broken }
    }
}

/// `parent_of` returns the store that the manifest of store `k` names in
/// `extends`, or `None` when the manifest names none. It looks the
/// identity up among the current identities first, then the former ones,
/// and resolves to nothing when two stores share it.
pub(crate) fn parent_of(knowledges: &[Knowledge], k: usize) -> Result<Option<usize>, Broken> {
    let Some(written) = knowledges[k]
        .manifest
        .as_ref()
        .and_then(|m| m.extends.as_deref())
    else {
        return Ok(None);
    };
    let wanted = Identity::parse(written).ok_or_else(|| Broken::NoStore(written.to_string()))?;
    let (hits, _) = super::stores::matching_in(knowledges, |i| *i == wanted);
    match hits.as_slice() {
        [one] => Ok(Some(*one)),
        [] => Err(Broken::NoStore(written.to_string())),
        _ => Err(Broken::Shared(written.to_string())),
    }
}

/// `broken` returns why store `k` has no chain: its `extends` names no
/// store, or the chain from it comes back to it. It returns `None` for a
/// store whose `extends` resolves, or that has none.
pub(crate) fn broken(knowledges: &[Knowledge], k: usize) -> Option<Broken> {
    let mut seen = vec![k];
    let mut at = k;
    loop {
        match parent_of(knowledges, at) {
            Err(b) if at == k => return Some(b),
            Ok(Some(p)) if p == k => return Some(Broken::Cycle),
            Ok(Some(p)) if !seen.contains(&p) => {
                seen.push(p);
                at = p;
            }
            _ => return None,
        }
    }
}

/// `claimants_in` returns the schemas that govern `type_name` in the store
/// whose chain is `chain`: the store's own schemas of the type, or else
/// those of the nearest store of its chain that holds one
/// (`SOKF_SCHEMA_96`, `SOKF_SCHEMA_97`). A schema of additional files
/// dispatches on no type, so no store inherits it (`SOKF_SCHEMA_99`).
/// `visit` sees each store that the lookup reads.
// sokf-implements: SOKF_SCHEMA_99
// sokf-implements: FORMAT-5_AC-7
pub(crate) fn claimants_in(
    knowledges: &[Knowledge],
    chain: &[usize],
    type_name: &str,
    visit: &mut dyn FnMut(usize),
) -> Vec<SchemaRef> {
    for &s in chain {
        visit(s);
        let own = knowledges[s].schemas_for(type_name);
        if !own.is_empty() {
            return own
                .iter()
                .map(|&index| SchemaRef {
                    knowledge: s,
                    index,
                })
                .collect();
        }
    }
    Vec::new()
}

/// `inherited_in` returns each schema that store `k` inherits: for each
/// type that no store nearer in the chain declares, every schema of the
/// type in the nearest store that declares it.
pub(crate) fn inherited_in(knowledges: &[Knowledge], chain: &[usize]) -> Vec<SchemaRef> {
    let k = chain[0];
    let mut declared: std::collections::BTreeSet<&str> = knowledges[k]
        .schemas_by_type
        .keys()
        .map(String::as_str)
        .collect();
    let mut found = Vec::new();
    for &s in &chain[1..] {
        let mut types: Vec<(&String, &Vec<usize>)> = knowledges[s].schemas_by_type.iter().collect();
        types.sort();
        let mut here = Vec::new();
        for (t, indexes) in types {
            if declared.contains(t.as_str()) {
                continue;
            }
            here.push(t.as_str());
            found.extend(indexes.iter().map(|&index| SchemaRef {
                knowledge: s,
                index,
            }));
        }
        declared.extend(here);
    }
    found
}

/// `rebase` returns `path`, a path or a glob of a schema in the store at
/// `from`, at the same place under the store at `to`, when it lies under
/// `from`, and unchanged otherwise (`SOKF_SCHEMA_98`).
// sokf-implements: SOKF_SCHEMA_98
// sokf-implements: FORMAT-5_AC-6
pub fn rebase(path: &str, from: &RepoPath, to: &RepoPath) -> String {
    let from = from.as_str().trim_end_matches('/');
    let to = to.as_str().trim_end_matches('/');
    match path.strip_prefix(from) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{to}{rest}"),
        _ => path.to_string(),
    }
}

impl Model {
    /// `own_roots` returns the number of own stores, those of
    /// `knowledge.roots`, which come before the registered ones
    /// (CLI-6_AC-116).
    pub fn own_roots(&self) -> usize {
        self.knowledges.len() - self.tree.registered_count.min(self.knowledges.len())
    }

    /// `broken` returns why store `k` has no chain of `extends`, or `None`
    /// when it has one.
    pub fn broken(&self, k: usize) -> Option<&Broken> {
        self.chains.broken.get(k)?.as_ref()
    }

    /// `in_broken_store` returns whether `path` lies in a store whose
    /// `extends` is broken, whose files no check and no fix touches
    /// (FORMAT-5_AC-10).
    pub fn in_broken_store(&self, path: &RepoPath) -> bool {
        self.knowledge_of(path)
            .is_some_and(|k| self.broken(k).is_some())
    }

    /// `inherited` returns each schema that store `k` inherits.
    pub fn inherited(&self, k: usize) -> Vec<SchemaRef> {
        inherited_in(&self.knowledges, &self.chains.chains[k])
    }

    /// `claimants` returns the schemas that govern `type_name` in store `k`,
    /// through the chain of `extends`. It records the schema query of each
    /// store that it reads, so that a schema that a store of the chain
    /// adds, removes, or changes makes the reader dirty (CACHE-1_AC-15).
    pub fn claimants(&self, k: usize, type_name: &str) -> Vec<SchemaRef> {
        let Some(chain) = self.chains.chains.get(k) else {
            return Vec::new();
        };
        claimants_in(&self.knowledges, chain, type_name, &mut |s| {
            self.note(Query::Schema {
                knowledge: s,
                key: type_name.to_string(),
            });
        })
    }

    /// `declares` returns whether store `k` declares `type_name`, in a
    /// schema of its own or in one that it inherits (CLI-6_AC-117).
    pub fn declares(&self, k: usize, type_name: &str) -> bool {
        !self.claimants(k, type_name).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    #[test]
    fn rebases_a_path_under_the_schema_store_only() {
        let (from, to) = (p("/knowledge"), p("/packs/login"));
        assert_eq!(
            rebase("/knowledge/contracts", &from, &to),
            "/packs/login/contracts"
        );
        assert_eq!(rebase("/knowledge", &from, &to), "/packs/login");
        assert_eq!(
            rebase("/knowledge/json/**/*.schema.json", &from, &to),
            "/packs/login/json/**/*.schema.json"
        );
        assert_eq!(rebase("/knowledgebase/x", &from, &to), "/knowledgebase/x");
        assert_eq!(rebase("/elsewhere", &from, &to), "/elsewhere");
    }
}
