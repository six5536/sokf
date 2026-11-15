//! This module runs the JMESPath compliance suite (`tests/jmespath/*.json`)
//! against the evaluator. For every `result` case, the evaluator must return
//! the case's expected result. For every `error` case, it must fail with the
//! case's class of error. A `bench` case is a
//! timing fixture with no assertion, so the module skips it, as the
//! `jmespath` crate's own harness does.

// sokf-tests: CLI-7_AC-43
// sokf-tests: CLI_P-14

use serde_json::Value;

use super::eval::{Failure, equal, eval};

const SUITES: &[(&str, &str)] = &[
    ("basic", include_str!("../../tests/jmespath/basic.json")),
    (
        "benchmarks",
        include_str!("../../tests/jmespath/benchmarks.json"),
    ),
    ("boolean", include_str!("../../tests/jmespath/boolean.json")),
    ("current", include_str!("../../tests/jmespath/current.json")),
    ("escape", include_str!("../../tests/jmespath/escape.json")),
    ("filters", include_str!("../../tests/jmespath/filters.json")),
    (
        "functions",
        include_str!("../../tests/jmespath/functions.json"),
    ),
    (
        "identifiers",
        include_str!("../../tests/jmespath/identifiers.json"),
    ),
    ("indices", include_str!("../../tests/jmespath/indices.json")),
    ("literal", include_str!("../../tests/jmespath/literal.json")),
    (
        "multiselect",
        include_str!("../../tests/jmespath/multiselect.json"),
    ),
    ("pipe", include_str!("../../tests/jmespath/pipe.json")),
    ("slice", include_str!("../../tests/jmespath/slice.json")),
    ("syntax", include_str!("../../tests/jmespath/syntax.json")),
    ("unicode", include_str!("../../tests/jmespath/unicode.json")),
    (
        "wildcard",
        include_str!("../../tests/jmespath/wildcard.json"),
    ),
];

/// `run_case` runs one case and returns `None` when it passes, or the
/// reason it failed.
fn run_case(given: &Value, case: &Value) -> Option<String> {
    let expression = case["expression"].as_str()?;
    let error = case.get("error").and_then(Value::as_str);
    let compiled = match (jmespath::compile(expression), error) {
        (Ok(c), _) => c,
        (Err(_), Some("syntax")) => return None,
        (Err(e), _) => return Some(format!("did not parse: {e}")),
    };
    if let Some(error) = error {
        let outcome = eval(compiled.as_ast(), given);
        let ok = matches!(
            (error, &outcome),
            ("syntax", Err(_))
                | ("invalid-arity", Err(Failure::Arity { .. }))
                | ("invalid-type", Err(Failure::Type { .. }))
                | ("invalid-value", Err(Failure::Slice))
                | ("unknown-function", Err(Failure::UnknownFunction(_)))
        );
        return (!ok).then(|| format!("expected {error}, got {outcome:?}"));
    }
    let outcome = eval(compiled.as_ast(), given);
    let expected = &case["result"];
    match outcome {
        Ok(got) if equal(&got, expected) => None,
        Ok(got) => Some(format!("expected {expected}, got {got}")),
        Err(e) => Some(format!("expected {expected}, got error {e}")),
    }
}

#[test]
fn the_evaluator_passes_the_compliance_suite() {
    let mut failures = Vec::new();
    let mut count = 0;
    for (name, text) in SUITES {
        let suites: Vec<Value> = serde_json::from_str(text).unwrap();
        for suite in &suites {
            for case in suite["cases"].as_array().unwrap() {
                if case.get("bench").is_some() {
                    continue;
                }
                count += 1;
                if let Some(why) = run_case(&suite["given"], case) {
                    failures.push(format!("{name}: {}: {why}", case["expression"]));
                }
            }
        }
    }
    assert!(
        count > 800,
        "only {count} cases ran. The suite has more than 800"
    );
    assert!(
        failures.is_empty(),
        "{} of {count} cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
