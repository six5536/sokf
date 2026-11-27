//! These tests check that a warning on a schema leaves the schema in force:
//! the validator still checks each document of its type against it.

use common::{TempTree, json};
use sokf_core::{CacheMode, Report, Severity};

use crate::common;

/// `SCHEMA` is the path of the Decision schema of the `clean` fixture.
const SCHEMA: &str = "knowledge/schemas/decision.md";

/// `DOCUMENT` is the Decision document of the `clean` fixture.
const DOCUMENT: &str = "knowledge/decisions/decision-single-binary.md";

/// `Warned` is one warning that sokf or the engine can report on a Schema
/// document, with the edits of the fixture that draw it: the key of the
/// warning, each edit of the schema as the text that it replaces and the
/// text that takes its place, and the text of `/.sokf/config.toml`.
type Warned = (
    &'static str,
    &'static [(&'static str, &'static str)],
    Option<&'static str>,
);

/// `WARNED` lists each warning of the class.
const WARNED: [Warned; 5] = [
    // A pattern of `current-links` that names emphasis in a form that never
    // matches, which sokf reports while it compiles the schema.
    (
        "SOKF_SCHEMA_110",
        &[(
            "line-limit: 120\n",
            "line-limit: 120\ncurrent-links: ['^__Planned__: ']\n",
        )],
        None,
    ),
    // A banned pattern of the same kind, which the engine reports when it
    // checks the schema.
    (
        "TLMD_264",
        &[(
            "prohibited-patterns:\n",
            "prohibited-patterns:\n  - pattern: '__Draft__'\n    message: 'A finished document is no draft.'\n",
        )],
        None,
    ),
    // An optional section that the scaffold leaves out.
    (
        "TLMD_124",
        &[(
            "prohibited-patterns:\n",
            "  - heading: Notes\n    level: 2\n    optional: true\n    guidance: Notes on the decision.\n    blocks:\n      - kind: prose\nprohibited-patterns:\n",
        )],
        None,
    ),
    // A `cited-by` entry of markers while the marker check is off.
    (
        "EXT-9_AC-8",
        &[(
            "    const: Decision\n",
            "    const: Decision\ncited-by:\n  - from-marker: implements\nmarkers: [implements]\n",
        )],
        Some("[markers]\ncheck = \"off\"\n"),
    ),
    // A `sokf:optional` region left around the schema's fence.
    (
        "TLMD_11",
        &[
            ("````yaml\n", "<!-- sokf:optional -->\n\n````yaml\n"),
            (
                "contains no TODO.'\n````\n",
                "contains no TODO.'\n````\n\n<!-- /sokf:optional -->\n",
            ),
        ],
        None,
    ),
];

// Each warning that a Schema document can draw leaves the schema in force,
// so the validator still checks the documents of its type: a banned word in
// the Decision document draws its error, and no finding says that the
// schema breaks the grammar. With the cache on, a second run that checks
// only the edited document reads the schema's findings from the cache and
// still checks the document.
// sokf-tests: EXT-1_AC-5
#[test]
fn a_schema_that_draws_only_a_warning_still_checks_its_documents() {
    for (key, edits, config) in WARNED {
        for cache in [CacheMode::Off, CacheMode::On] {
            let tree = TempTree::fixture("clean");
            let mut schema = tree.read(SCHEMA);
            for (from, to) in edits {
                assert!(schema.contains(from), "{key}: {from}");
                schema = schema.replacen(from, to, 1);
            }
            tree.write(SCHEMA, &schema);
            if let Some(config) = config {
                tree.write(".sokf/config.toml", config);
            }
            let document = tree.read(DOCUMENT);
            tree.write(DOCUMENT, &format!("{document}\nTODO\n"));
            let report = tree.validate(false, cache);
            assert_checked(&report, key, DOCUMENT, cache);
            if cache == CacheMode::On {
                tree.write(DOCUMENT, &format!("{document}\nTODO again\n"));
                let again = tree.validate(false, cache);
                assert_checked(&again, key, DOCUMENT, cache);
            }
        }
    }
}

// An additional file of a schema that draws only a warning is checked
// against the schema too: a banned word in the readme draws its error.
// sokf-tests: EXT-1_AC-5
#[test]
fn a_schema_that_draws_only_a_warning_still_checks_its_additional_files() {
    let readme_schema = "knowledge/schemas/readme.md";
    for cache in [CacheMode::Off, CacheMode::On] {
        let tree = TempTree::fixture("clean");
        let schema = tree.read(readme_schema).replacen(
            "line-limit: 0\n",
            "line-limit: 0\ncurrent-links: ['^__Planned__: ']\nprohibited-patterns:\n  - pattern: '\\bTODO\\b'\n    message: 'A finished readme contains no TODO.'\n",
            1,
        );
        tree.write(readme_schema, &schema);
        let readme = tree.read("README.md");
        tree.write(
            "README.md",
            &readme.replacen("validator.\n", "validator. TODO\n", 1),
        );
        let report = tree.validate(false, cache);
        assert_checked_at(
            &report,
            "SOKF_SCHEMA_110",
            readme_schema,
            "README.md",
            cache,
        );
    }
}

/// `assert_checked` asserts that the report of a run on the fixture with
/// the Decision schema warned by `key` holds exactly that warning on the
/// schema, the error of the banned word in `document`, and no finding of a
/// schema that breaks the grammar.
fn assert_checked(report: &Report, key: &str, document: &str, cache: CacheMode) {
    assert_checked_at(report, key, SCHEMA, document, cache);
}

/// `assert_checked_at` asserts what `assert_checked` does, for the schema
/// at `schema`.
fn assert_checked_at(report: &Report, key: &str, schema: &str, document: &str, cache: CacheMode) {
    let on_schema: Vec<(Severity, &str)> = report
        .errors
        .iter()
        .chain(&report.warnings)
        .filter(|f| f.path.as_str() == format!("/{schema}"))
        .map(|f| (f.severity, f.criterion.as_str()))
        .collect();
    assert_eq!(
        on_schema,
        [(Severity::Warning, key)],
        "{key} {cache:?}: {}",
        json(report)
    );
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.criterion == "TL_75" && f.path.as_str() == format!("/{document}")),
        "{key} {cache:?}: {}",
        json(report)
    );
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.criterion != "SOKF_VALIDATION_41"),
        "{key} {cache:?}: {}",
        json(report)
    );
}

// A fix under a schema that draws only a warning repairs the documents of
// its type: a link to a document by its path becomes a link by its id, and
// a second run fixes nothing.
// sokf-tests: EXT-1_AC-5
#[test]
fn a_fix_under_a_schema_that_draws_only_a_warning_repairs_its_documents() {
    for cache in [CacheMode::Off, CacheMode::On] {
        let tree = TempTree::fixture("clean");
        let schema = tree.read(SCHEMA).replacen(
            "line-limit: 120\n",
            "line-limit: 120\ncurrent-links: ['^__Planned__: ']\n",
            1,
        );
        tree.write(SCHEMA, &schema);
        let document = tree.read(DOCUMENT).replacen(
            "is a subcommand.",
            "is a subcommand, as [the overview](/knowledge/overview.md) says.",
            1,
        );
        tree.write(DOCUMENT, &document);
        let fixed = tree.validate(true, cache);
        assert!(fixed.errors.is_empty(), "{cache:?}: {}", json(&fixed));
        assert!(
            tree.read(DOCUMENT)
                .contains("as [the overview][sokf:overview] says."),
            "{cache:?}: {}",
            tree.read(DOCUMENT)
        );
        let again = tree.validate(true, cache);
        assert!(again.fixes.is_empty(), "{cache:?}: {}", json(&again));
    }
}
