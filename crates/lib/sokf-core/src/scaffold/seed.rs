//! This module works out the state of each lifecycle dimension from the
//! `--state` options given to `sokf new`, and derives the seed scaffold for
//! those states.

use super::{Refusal, SokfScaffold};
use crate::{
    Error, Result,
    grammar::{Scaffold, SokfSchema, States},
};

/// `states` returns one state for each lifecycle dimension: the state
/// given for it, or else its first state. It refuses a key that the
/// lifecycle doesn't declare, a state that the dimension doesn't have, and
/// a dimension that is given twice.
// sokf-implements: CLI-6_AC-3
// sokf-implements: CLI-6_AC-19
// sokf-implements: CLI-6_AC-38
pub fn states(schema: &SokfSchema, given: &[(String, String)]) -> Result<States> {
    let mut out = States::default();
    for (key, state) in given {
        let Some(dim) = schema.core.core.lifecycle.iter().find(|d| &d.key == key) else {
            return Err(Error::New(format!(
                "`--state {key}={state}`: the lifecycle declares no `{key}`"
            )));
        };
        if !dim.states.contains(state) {
            return Err(Error::New(format!(
                "`--state {key}={state}`: `{state}` is not a state of `{key}`. The states are {}",
                dim.states.join(", ")
            )));
        }
        if out.0.contains_key(key) {
            return Err(Error::New(format!("`--state {key}` is given twice")));
        }
        out.0.insert(key.clone(), vec![state.clone()]);
    }
    for dim in &schema.core.core.lifecycle {
        if let Some(first) = dim.states.first() {
            out.0
                .entry(dim.key.clone())
                .or_insert_with(|| vec![first.clone()]);
        }
    }
    Ok(out)
}

/// `seed` returns the scaffold derived from the schema for the given
/// states. A schema has no separate template, so `sokf new` builds the
/// new document from the schema's rules.
// sokf-implements: CLI-6_AC-1
// sokf-implements: CLI-6_AC-16
pub fn seed(schema: &SokfSchema, states: &States) -> std::result::Result<Scaffold, Vec<Refusal>> {
    let document =
        tramlines_md::derive::derive_one(&schema.core, states, &SokfScaffold::new(schema))?;
    Ok(Scaffold {
        document,
        lifecycle: None,
        line: schema.core.core.line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::fixtures::schema;

    fn pairs(given: &[(&str, &str)]) -> Vec<(String, String)> {
        given
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // sokf-tests: CLI-6_AC-3
    // sokf-tests: CLI-6_AC-19
    // sokf-tests: CLI-6_AC-38
    #[test]
    fn states_default_to_the_first_and_refuse_the_undeclared() {
        let c = schema();
        let s = states(&c, &[]).unwrap();
        assert_eq!(s.0["state"], vec!["open"]);
        assert_eq!(s.0["kind"], vec!["bug"]);
        let s = states(&c, &pairs(&[("kind", "feature")])).unwrap();
        assert_eq!(s.0["state"], vec!["open"]);
        assert_eq!(s.0["kind"], vec!["feature"]);
        for (given, needle) in [
            (pairs(&[("phase", "a")]), "declares no `phase`"),
            (pairs(&[("state", "done")]), "not a state of `state`"),
            (
                pairs(&[("state", "open"), ("state", "closed")]),
                "given twice",
            ),
        ] {
            let e = states(&c, &given).unwrap_err().to_string();
            assert!(e.contains(needle), "{e}");
        }
    }

    // sokf-tests: CLI-6_AC-1
    #[test]
    fn the_seed_is_the_derived_template_at_the_states_asked_for() {
        let c = schema();
        let pick = |given: &[(&str, &str)]| {
            seed(&c, &states(&c, &pairs(given)).unwrap())
                .unwrap()
                .document
        };
        let open_bug = pick(&[]);
        assert!(
            open_bug.contains("state: open") && open_bug.contains("kind: bug"),
            "{open_bug}"
        );
        let closed = pick(&[("state", "closed"), ("kind", "feature")]);
        assert!(
            closed.contains("state: closed") && closed.contains("kind: feature"),
            "{closed}"
        );
    }
}
