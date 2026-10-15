//! The lifecycle check applies where the schema declares a `root`. Where the
//! schema ties a document's lifecycle states to the path, a document must sit
//! in the directory that its states call for (SOKF_SCHEMA_26). Elsewhere, a
//! document must lie under the root, at any depth (SOKF_SCHEMA_51).

use crate::{
    Finding, RepoPath,
    grammar::{SokfSchema, States},
};

/// `derived_dir` returns the directory derived from a document's lifecycle
/// states. The directory is the schema's `root`, then, in order, one level
/// for each lifecycle dimension that the schema ties to the path. Each level
/// is named after the document's state in that dimension. It returns `None` when the schema has no root, or
/// when the document's state in such a dimension is missing, holds more than
/// one value, or isn't one of the dimension's states.
// sokf-implements: SOKF_SCHEMA_26
pub fn derived_dir(schema: &SokfSchema, states: &States) -> Option<RepoPath> {
    let mut dir = RepoPath::parse(schema.root.as_deref()?)?;
    for dim in schema
        .core
        .core
        .lifecycle
        .iter()
        .filter(|d| schema.is_path(&d.key))
    {
        let value = match states.0.get(&dim.key).map(Vec::as_slice) {
            Some([one]) => one,
            _ => return None,
        };
        if !dim.states.contains(value) {
            return None;
        }
        dir = dir.join(value)?;
    }
    Some(dir)
}

/// `placed` reports whether a document at `path` sits where its schema puts
/// it: in the directory derived from its lifecycle states, or, for a schema
/// with no path-enforced dimension, anywhere under the root. `derived` is the
/// directory that `derived_dir` returns.
pub fn placed(path: &RepoPath, schema: &SokfSchema, derived: &RepoPath) -> bool {
    if schema.paths.is_empty() {
        path.starts_with(derived)
    } else {
        path.parent().as_ref() == Some(derived)
    }
}

/// `check_directory` reports an error when the document doesn't sit where
/// its schema puts it. It returns `None` when the document sits there, or
/// when no directory can be derived.
// sokf-implements: SOKF_SCHEMA_26
// sokf-implements: SOKF_SCHEMA_51
// sokf-implements: EXT-7_AC-1
// sokf-implements: EXT-7_AC-4
pub fn check_directory(path: &RepoPath, schema: &SokfSchema, states: &States) -> Option<Finding> {
    let derived = derived_dir(schema, states)?;
    if placed(path, schema, &derived) {
        return None;
    }
    if schema.paths.is_empty() {
        return Some(Finding::error(
            path.clone(),
            None,
            "SOKF_SCHEMA_51",
            format!("the document lies outside `{derived}`, the root of its schema"),
        ));
    }
    let actual = path.parent()?;
    Some(Finding::error(
        path.clone(),
        None,
        "SOKF_SCHEMA_26",
        format!(
            "the document sits in `{actual}`, but its lifecycle states place it in `{derived}`"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use super::*;
    use crate::grammar::{Dispatch, LifecycleDimension, Schema};

    fn schema(root: Option<&str>, dims: &[(&str, &[&str], bool)]) -> SokfSchema {
        let mut core = Schema::demand_nothing();
        core.core.frontmatter_open = false;
        core.core.line = 1;
        core.core.lifecycle = dims
            .iter()
            .map(|(key, states, _)| LifecycleDimension {
                key: key.to_string(),
                states: states.iter().map(|s| s.to_string()).collect(),
                guidance: None,
                line: 1,
                extensions: Default::default(),
            })
            .collect();
        SokfSchema {
            dispatch: Dispatch::Type("T".into()),
            root: root.map(str::to_string),
            paths: dims
                .iter()
                .filter(|(_, _, path)| *path)
                .map(|(key, _, _)| key.to_string())
                .collect(),
            links_open: false,
            ..SokfSchema::of_schema_document(core)
        }
    }

    fn states(pairs: &[(&str, &str)]) -> States {
        States(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn derives_the_directory_from_path_enforced_dimensions() {
        let c = schema(
            Some("/k/issues"),
            &[
                ("state", &["open", "closed"], true),
                ("kind", &["bug", "feature"], false),
                ("area", &["cli", "core"], true),
            ],
        );
        assert_eq!(
            derived_dir(
                &c,
                &states(&[("state", "open"), ("kind", "bug"), ("area", "cli")])
            )
            .unwrap()
            .as_str(),
            "/k/issues/open/cli"
        );
        assert!(derived_dir(&c, &states(&[("state", "open")])).is_none());
        assert!(derived_dir(&c, &states(&[("state", "nope"), ("area", "cli")])).is_none());
        assert!(derived_dir(&schema(None, &[]), &States::default()).is_none());
        let p = |s: &str| RepoPath::parse(s).unwrap();
        let st = states(&[("state", "open"), ("area", "cli")]);
        assert!(check_directory(&p("/k/issues/open/cli/i.md"), &c, &st).is_none());
        let f = check_directory(&p("/k/issues/closed/cli/i.md"), &c, &st).unwrap();
        assert!(
            f.message.contains("place it in `/k/issues/open/cli`"),
            "{}",
            f.message
        );
    }

    // sokf-tests: SOKF_SCHEMA_51
    #[test]
    fn a_document_of_a_root_with_no_path_dimension_lies_anywhere_under_it() {
        let c = schema(Some("/k/designs"), &[]);
        let p = |s: &str| RepoPath::parse(s).unwrap();
        let none = States::default();
        assert!(check_directory(&p("/k/designs/d.md"), &c, &none).is_none());
        assert!(check_directory(&p("/k/designs/core/d.md"), &c, &none).is_none());
        let f = check_directory(&p("/k/d.md"), &c, &none).unwrap();
        assert_eq!(f.criterion, "SOKF_SCHEMA_51");
    }

    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn derived_dir_is_root_then_each_path_state_in_order(
            flags in prop::collection::vec(any::<bool>(), 1..4),
            picks in prop::collection::vec(0usize..2, 1..4),
        ) {
            let names = ["d0", "d1", "d2"];
            let dims: Vec<(&str, &[&str], bool)> = flags.iter().enumerate().map(|(i, &p)| (names[i], &["a", "b"][..], p)).collect();
            let c = schema(Some("/k"), &dims);
            let st: Vec<(&str, &str)> = dims.iter().enumerate().map(|(i, (k, _, _))| (*k, ["a", "b"][picks[i % picks.len()]])).collect();
            let dir = derived_dir(&c, &states(&st)).unwrap();
            let mut want = String::from("/k");
            for (k, _, p) in &dims {
                if *p {
                    let v = st.iter().find(|(kk, _)| kk == k).unwrap().1;
                    want.push('/');
                    want.push_str(v);
                }
            }
            prop_assert_eq!(dir.as_str(), want.as_str());
        }
    }
}
