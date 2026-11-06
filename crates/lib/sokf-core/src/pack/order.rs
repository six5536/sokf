//! This module orders the packs of a repository from the lowest to the
//! highest: a pack comes after every pack that it depends on, and the list
//! of `[[pack]]` entries orders the rest. A higher pack owns each file and
//! each schema type that a lower pack also supplies (the decision "The order
//! of the packs" of design-pack).

use std::collections::{BTreeMap, BTreeSet};

use crate::{Error, Result};

/// `precedence` returns the packs of `list`, which holds their names in the
/// order of the `[[pack]]` entries, from the lowest to the highest.
/// `depends` maps the name of a pack to the names of the packs that it
/// depends on. The order is a stable topological sort: it takes, at each
/// step, the first pack of the list whose dependencies it has taken, so a
/// pack comes after every pack that it depends on, directly or through
/// others, and the list orders the rest as far as the dependencies allow. It
/// refuses dependencies that form a cycle, and names its packs. A dependency
/// on a pack outside `list` doesn't count here.
// sokf-implements: CLI-13_AC-70
// sokf-implements: CLI-13_AC-71
pub fn precedence(list: &[String], depends: &BTreeMap<String, Vec<String>>) -> Result<Vec<String>> {
    let names: BTreeSet<&str> = list.iter().map(String::as_str).collect();
    let mut placed: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::new();
    while out.len() < list.len() {
        let next = list.iter().find(|n| {
            !placed.contains(n.as_str())
                && depends.get(n.as_str()).is_none_or(|deps| {
                    deps.iter()
                        .all(|d| !names.contains(d.as_str()) || placed.contains(d.as_str()))
                })
        });
        let Some(next) = next else {
            let cycle: Vec<&str> = list
                .iter()
                .map(String::as_str)
                .filter(|n| !placed.contains(n))
                .collect();
            return Err(Error::Pack(format!(
                "the dependencies of the packs {} form a cycle",
                cycle.join(", ")
            )));
        };
        placed.insert(next);
        out.push(next.clone());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(list: &[&str], deps: &[(&str, &[&str])]) -> Result<Vec<String>> {
        let list: Vec<String> = list.iter().map(|s| s.to_string()).collect();
        let deps = deps
            .iter()
            .map(|(n, d)| (n.to_string(), d.iter().map(|s| s.to_string()).collect()))
            .collect();
        precedence(&list, &deps)
    }

    // sokf-tests: CLI-13_AC-70
    // sokf-tests: CLI-13_AC-71
    #[test]
    fn a_pack_comes_after_its_dependencies_and_the_list_orders_the_rest() {
        // Without dependencies, the list is the order: the later entry is
        // higher.
        assert_eq!(order(&["a", "b"], &[]).unwrap(), ["a", "b"]);
        // A pack comes after the pack that it depends on, wherever its
        // entry lies.
        assert_eq!(
            order(&["rust", "base"], &[("rust", &["base"])]).unwrap(),
            ["base", "rust"]
        );
        // Through another pack too.
        assert_eq!(
            order(&["c", "b", "a"], &[("c", &["b"]), ("b", &["a"])]).unwrap(),
            ["a", "b", "c"]
        );
        // The list orders the packs that the dependencies leave free, as far
        // as the dependencies allow.
        assert_eq!(
            order(&["x", "base", "rust"], &[("rust", &["base"])]).unwrap(),
            ["x", "base", "rust"]
        );
        // A dependency outside the list doesn't count here.
        assert_eq!(order(&["a"], &[("a", &["gone"])]).unwrap(), ["a"]);
        let e = order(&["a", "b", "c"], &[("a", &["b"]), ("b", &["a"])])
            .unwrap_err()
            .to_string();
        assert!(e.contains("the packs a, b form a cycle"), "{e}");
    }
}
