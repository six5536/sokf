//! This module holds the fixtures that the unit tests of the scaffold share:
//! a schema with two lifecycle dimensions, and a builder of states.

use std::collections::BTreeMap;

use crate::grammar::{LifecycleDimension, Schema, SokfSchema, States};

/// `schema` returns a schema whose lifecycle has the dimensions `state`
/// (`open`, `closed`) and `kind` (`bug`, `feature`). The `state` dimension
/// places documents.
pub(super) fn schema() -> SokfSchema {
    let mut c = Schema::demand_nothing();
    c.core.lifecycle = vec![
        LifecycleDimension {
            key: "state".into(),
            states: vec!["open".into(), "closed".into()],
            guidance: None,
            line: 1,
            extensions: BTreeMap::default(),
        },
        LifecycleDimension {
            key: "kind".into(),
            states: vec!["bug".into(), "feature".into()],
            guidance: None,
            line: 2,
            extensions: BTreeMap::default(),
        },
    ];
    SokfSchema {
        paths: vec!["state".into()],
        ..SokfSchema::of_schema_document(c)
    }
}

/// `states` returns the states that `pairs` give, one state for each key.
pub(super) fn states(pairs: &[(&str, &str)]) -> States {
    States(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
            .collect::<BTreeMap<_, _>>(),
    )
}
