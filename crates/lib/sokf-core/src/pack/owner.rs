//! This module decides which pack owns each destination and each schema
//! type, given the order of the packs. The highest pack that supplies a
//! destination owns it, and the highest pack that supplies a schema of a
//! type to a store owns the type in that store, because each store holds
//! its own schema of each type. Every other pack's version is shadowed: sync keeps
//! its base copy and doesn't write it. A destination whose owner changes
//! passes to its new owner by a merge (the decision "The rules of owning a
//! file" of design-pack).

use std::collections::BTreeMap;

use super::lock::Lock;
use crate::RepoPath;

/// A `Supply` is what one pack supplies: each destination, with the type
/// of the schema that lands there, when the file is a schema.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Supply {
    /// The store that receives the pack's schemas, when it has any.
    pub store: Option<RepoPath>,
    /// Each destination, with the schema's type.
    pub files: Vec<(RepoPath, Option<String>)>,
}

/// A `Shadow` says that another pack owns a destination of a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shadow {
    /// The pack that owns it.
    pub owner: String,
    /// Whether the owner owns the schema type of the file, at another
    /// destination, and not the destination itself.
    pub by_type: bool,
}

/// A `Takeover` says that a pack owns a destination that another pack
/// owned until this sync, so the file holds that pack's version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Takeover {
    /// The pack that owned the destination.
    pub from: String,
    /// The path of the file in that pack.
    pub rel: String,
    /// The hash of that pack's base.
    pub base: String,
    /// Whether that pack's version was executable.
    pub executable: bool,
}

/// An `Ownership` is what one pack owns of what it supplies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ownership {
    /// Each destination of the pack that another pack owns.
    pub shadowed: BTreeMap<RepoPath, Shadow>,
    /// Each destination that the pack takes over from another pack.
    pub takeover: BTreeMap<RepoPath, Takeover>,
}

/// `owners` returns the ownership of each pack of `supplies`, given
/// `order`, the packs from the lowest to the highest, and `old`, the lock
/// before the sync, which says which pack's version each file holds.
// sokf-implements: CLI-13_AC-73
// sokf-implements: CLI-13_AC-74
pub fn owners(
    order: &[String],
    supplies: &BTreeMap<String, Supply>,
    old: &Lock,
) -> BTreeMap<String, Ownership> {
    let mut dest_owner: BTreeMap<&RepoPath, &str> = BTreeMap::new();
    // A type is owned in one store, so two packs of different stores both
    // own it.
    let mut type_owner: BTreeMap<(Option<&RepoPath>, &str), &str> = BTreeMap::new();
    for name in order {
        let Some(supply) = supplies.get(name) else {
            continue;
        };
        for (dest, ty) in &supply.files {
            dest_owner.insert(dest, name);
            if let Some(t) = ty {
                type_owner.insert((supply.store.as_ref(), t), name);
            }
        }
    }
    let mut out = BTreeMap::new();
    for (name, supply) in supplies {
        let mut own = Ownership::default();
        for (dest, ty) in &supply.files {
            let owner = dest_owner.get(dest).copied().unwrap_or(name);
            let by_type = ty
                .as_deref()
                .and_then(|t| type_owner.get(&(supply.store.as_ref(), t)).copied())
                .filter(|o| *o != name);
            if owner != name {
                own.shadowed.insert(
                    dest.clone(),
                    Shadow {
                        owner: owner.to_string(),
                        by_type: false,
                    },
                );
            } else if let Some(o) = by_type {
                own.shadowed.insert(
                    dest.clone(),
                    Shadow {
                        owner: o.to_string(),
                        by_type: true,
                    },
                );
            } else if let Some((from, rel)) = old.owner_of(dest.as_str())
                && from != name
            {
                let f = &old.packs[from].files[rel];
                own.takeover.insert(
                    dest.clone(),
                    Takeover {
                        from: from.to_string(),
                        rel: rel.to_string(),
                        base: f.base.clone(),
                        executable: f.executable,
                    },
                );
            }
        }
        out.insert(name.clone(), own);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{LockedFile, LockedPack};

    fn path(p: &str) -> RepoPath {
        RepoPath::parse(p).unwrap()
    }

    fn supply(files: &[(&str, Option<&str>)]) -> Supply {
        Supply {
            store: Some(path("/k")),
            files: files
                .iter()
                .map(|(d, t)| (path(d), t.map(str::to_string)))
                .collect(),
        }
    }

    fn lock(owner: &str, dest: &str) -> Lock {
        let mut lock = Lock::default();
        let mut pack = LockedPack::default();
        pack.files.insert(
            "schemas/issue.md".into(),
            LockedFile {
                path: dest.into(),
                base: "fnv1a64:1".into(),
                executable: false,
                shadowed_by: None,
                changed: false,
            },
        );
        lock.packs.insert(owner.into(), pack);
        lock
    }

    // sokf-tests: CLI-13_AC-73
    // sokf-tests: CLI-13_AC-74
    #[test]
    fn the_highest_pack_owns_a_destination_and_a_type() {
        let order = ["base".to_string(), "rust".to_string()];
        let supplies = BTreeMap::from([
            (
                "base".to_string(),
                supply(&[
                    ("/k/schemas/issue.md", Some("Issue")),
                    ("/rustfmt.toml", None),
                    ("/deny.toml", None),
                ]),
            ),
            (
                "rust".to_string(),
                supply(&[
                    ("/k/schemas/issue-strict.md", Some("Issue")),
                    ("/rustfmt.toml", None),
                ]),
            ),
        ]);
        let old = lock("base", "/rustfmt.toml");
        let own = owners(&order, &supplies, &old);
        let base = &own["base"];
        assert_eq!(
            base.shadowed[&path("/rustfmt.toml")],
            Shadow {
                owner: "rust".into(),
                by_type: false
            }
        );
        assert_eq!(
            base.shadowed[&path("/k/schemas/issue.md")],
            Shadow {
                owner: "rust".into(),
                by_type: true
            }
        );
        assert!(!base.shadowed.contains_key(&path("/deny.toml")));
        let rust = &own["rust"];
        assert!(rust.shadowed.is_empty());
        // `rust` takes over the file that `base` wrote before.
        assert_eq!(rust.takeover[&path("/rustfmt.toml")].from, "base");
        assert!(
            !rust
                .takeover
                .contains_key(&path("/k/schemas/issue-strict.md"))
        );
    }

    // sokf-tests: CLI-13_AC-74
    #[test]
    fn a_type_is_owned_in_each_store_apart() {
        let order = ["base".to_string(), "rust".to_string()];
        let other = Supply {
            store: Some(path("/other")),
            files: vec![(path("/other/schemas/issue.md"), Some("Issue".into()))],
        };
        let supplies = BTreeMap::from([
            (
                "base".to_string(),
                supply(&[("/k/schemas/issue.md", Some("Issue"))]),
            ),
            ("rust".to_string(), other),
        ]);
        let own = owners(&order, &supplies, &Lock::default());
        assert!(own["base"].shadowed.is_empty());
        assert!(own["rust"].shadowed.is_empty());
    }
}
