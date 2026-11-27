//! This module works out the state of each lifecycle dimension from the
//! `--state` options given to `sokf new` and `sokf guide`, and derives the
//! skeleton of a new document at those states.

use std::collections::BTreeMap;

use super::{Refusal, SokfScaffold, Words};
use crate::{
    Error, Result,
    grammar::{SokfSchema, States},
};

/// `states` returns one state for each lifecycle dimension: the state
/// given for it, or else its first state. It refuses what `given_states`
/// refuses.
// sokf-implements: CLI-6_AC-3
pub fn states(schema: &SokfSchema, given: &[(String, String)]) -> Result<States> {
    let mut out = given_states(schema, given).map_err(Error::New)?;
    for dim in &schema.core.core.lifecycle {
        if let Some(first) = dim.states.first() {
            out.0
                .entry(dim.key.clone())
                .or_insert_with(|| vec![first.clone()]);
        }
    }
    Ok(out)
}

/// `given_states` returns the state of each lifecycle dimension that
/// `given` names, the values of `--state` of `sokf new` and `sokf guide`.
/// It refuses a key that the lifecycle doesn't declare, a state that the
/// dimension doesn't have, and a dimension that is given twice, with a
/// message that names the option.
// sokf-implements: CLI-6_AC-19
// sokf-implements: CLI-6_AC-38
// sokf-implements: CLI-17_AC-12
// sokf-implements: CLI-17_AC-13
pub fn given_states(
    schema: &SokfSchema,
    given: &[(String, String)],
) -> std::result::Result<States, String> {
    let mut out = States::default();
    for (key, state) in given {
        let Some(dim) = schema.core.core.lifecycle.iter().find(|d| &d.key == key) else {
            return Err(format!(
                "`--state {key}={state}`: the lifecycle declares no `{key}`"
            ));
        };
        if !dim.states.contains(state) {
            return Err(format!(
                "`--state {key}={state}`: `{state}` is not a state of `{key}`. The states are {}",
                dim.states.join(", ")
            ));
        }
        if out.0.contains_key(key) {
            return Err(format!("`--state {key}` is given twice"));
        }
        out.0.insert(key.clone(), vec![state.clone()]);
    }
    Ok(out)
}

/// `Refused` says why no skeleton derives for a new document: the schema
/// derives no scaffold at the states, whatever the document's values, or the
/// scaffold with the document's values, such as a title that a paragraph
/// bound to it can't hold, refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The schema derives no scaffold at the states.
    Schema(Vec<Refusal>),
    /// The scaffold with the document's values refuses.
    Values(Vec<Refusal>),
}

/// `skeleton` returns the skeleton of a new document of `schema` at
/// `states`, whose frontmatter holds `values`, such as its id and its title,
/// with `words`, sokf's texts in the language of the schema: what the schema
/// prescribes, the values, each lifecycle key with its state, and each
/// required key left empty, and no example, no guidance, no marker, and no
/// optional part. A schema has no separate template, so `sokf new` builds
/// the new document from the schema's rules. The scaffold of the schema
/// comes first, so that a refusal there names a fault of the schema and not
/// of the values.
// sokf-implements: CLI-6_AC-1
// sokf-implements: CLI-6_AC-2
// sokf-implements: CLI-6_AC-7
// sokf-implements: CLI-6_AC-16
// sokf-implements: CLI-6_AC-35
// sokf-implements: CLI-6_AC-126
pub fn skeleton(
    schema: &SokfSchema,
    states: &States,
    words: &Words<'_>,
    values: &BTreeMap<String, String>,
) -> std::result::Result<String, Refused> {
    let host = SokfScaffold::with_words(schema, words);
    tramlines_md::derive::derive_one(&schema.core, states, &host).map_err(Refused::Schema)?;
    let host = SokfScaffold::for_skeleton(schema, words);
    tramlines_md::derive::skeleton(&schema.core, states, &host, values).map_err(Refused::Values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Texts, scaffold::fixtures::schema};

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
    // sokf-tests: CLI-6_AC-126
    #[test]
    fn the_skeleton_is_derived_at_the_states_asked_for_with_the_values() {
        let c = schema();
        let pick = |given: &[(&str, &str)]| {
            let texts = Texts::english();
            let values = BTreeMap::from([
                ("id".to_string(), "issue-004-retry".to_string()),
                ("title".to_string(), "Retry".to_string()),
            ]);
            skeleton(
                &c,
                &states(&c, &pairs(given)).unwrap(),
                &Words::new(&texts, "en"),
                &values,
            )
            .unwrap()
        };
        let open_bug = pick(&[]);
        assert!(
            open_bug.contains("state: open")
                && open_bug.contains("kind: bug")
                && open_bug.contains("id: issue-004-retry")
                && open_bug.contains("title: Retry"),
            "{open_bug}"
        );
        assert!(!open_bug.contains("sokf:"), "{open_bug}");
        let closed = pick(&[("state", "closed"), ("kind", "feature")]);
        assert!(
            closed.contains("state: closed") && closed.contains("kind: feature"),
            "{closed}"
        );
    }
}
