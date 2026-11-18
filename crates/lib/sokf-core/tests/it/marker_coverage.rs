//! These tests cover markers as citations (contract-schema-extension
//! EXT-9): a `cited-by` entry with a `from-marker` key counts only the
//! markers of its own relationship, a schema's `markers` refuses the markers
//! of any other relationship, and the cache checks a cited document again
//! when a marker that cites it changes.

use std::fmt::Write as _;

use common::{
    TempTree, age,
    incremental::{edit, run},
};
use sokf_core::{CacheMode, Finding, Report};

use crate::common;

/// `demand_markers` makes the requirement schema of the fixture demand an
/// `implements` marker for each criterion and a `tests` marker for the
/// document. A missing `implements` marker is a warning, and the schema
/// admits only those two relationships. `demand_markers` also makes the
/// issue schema admit no marker.
fn demand_markers(tree: &TempTree) {
    let rel = "knowledge/schemas/requirement.md";
    let text = tree.read(rel)
        .replace(
            "each: { format: requirement-ears, unique: repository,",
            "each: { format: requirement-ears, unique: repository, key-of: criteria,",
        )
        .replace(
            "    const: Requirement\n",
            "    const: Requirement\ncited-by:\n  - from-marker: implements\n    keys: criteria\n    severity: warn\n  - from-marker: tests\nmarkers: [implements, tests]\n",
        );
    tree.write(rel, &text);
    let rel = "knowledge/schemas/issue.md";
    let text = tree
        .read(rel)
        .replace("line-limit: 200\n", "line-limit: 200\nmarkers: []\n");
    tree.write(rel, &text);
}

/// `markers` builds a marker file with one marker on each line. Each marker
/// is `// sokf-`, then `REL`, then `: TARGET`, where `REL` is the relation and
/// `TARGET` is the key it cites. No line of this test file is then itself a
/// marker.
fn markers(lines: &[(&str, &str)]) -> String {
    lines.iter().fold(String::new(), |mut out, (rel, target)| {
        let _ = writeln!(out, "// sokf-{rel}: {target}");
        out
    })
}

/// `source` returns a marker file of four markers.
fn source() -> String {
    markers(&[
        ("implements", "CLI-1_AC-1"),
        ("tests", "CLI-1_AC-2"),
        ("references", "req-cli"),
        ("implements", "issue-001-retry"),
    ])
}

fn shown(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|f| match f.line {
            Some(line) => format!("{}:{line}: {}", f.path, f.message),
            None => format!("{}: {}", f.path, f.message),
        })
        .collect()
}

fn validate(tree: &TempTree) -> Report {
    tree.validate_all(false, CacheMode::Off)
}

// sokf-tests: EXT-9_AC-1
// sokf-tests: EXT-9_AC-3
// sokf-tests: EXT-9_AC-6
// sokf-tests: EXT-9_AC-7
// sokf-tests: SOKF_SCHEMA_49
// sokf-tests: CONF-7_AC-1
#[test]
fn a_marker_cites_for_its_relationship_alone_and_a_refused_one_fails() {
    let tree = TempTree::fixture("clean");
    demand_markers(&tree);
    tree.write("src/a.rs", &source());
    let report = validate(&tree);
    let errors = shown(&report.errors);
    let warnings = shown(&report.warnings);
    // CLI-1_AC-2 has a `tests` marker, which doesn't count as an
    // `implements` marker. The document has its `tests` marker.
    let uncited: Vec<&String> = warnings
        .iter()
        .filter(|w| w.contains("cites key"))
        .collect();
    assert_eq!(uncited.len(), 1, "{warnings:#?}");
    assert!(
        uncited[0].starts_with("/knowledge/req-cli.md:19:")
            && uncited[0].contains("no `sokf-implements:` marker cites key `CLI-1_AC-2`")
            && uncited[0].contains(
                "(schema schema-requirement, cited-by marker `implements` of `criteria`)"
            ),
        "{uncited:?}"
    );
    assert!(
        warnings.iter().all(|w| !w.contains("cites this document")),
        "{warnings:#?}"
    );
    // The requirement schema refuses `references`, and the issue schema
    // refuses every marker: each refused marker fails at its own line.
    let refused: Vec<&String> = errors
        .iter()
        .filter(|e| e.starts_with("/src/a.rs"))
        .collect();
    assert_eq!(refused.len(), 2, "{errors:#?}");
    assert!(
        refused[0].starts_with("/src/a.rs:3:")
            && refused[0].contains("`sokf-references:` cites `req-cli`, whose schema admits only `sokf-implements:`, `sokf-tests:`")
            && refused[0].contains("(schema schema-requirement, markers)"),
        "{refused:?}"
    );
    assert!(
        refused[1].starts_with("/src/a.rs:4:")
            && refused[1].contains("whose schema admits no marker")
            && refused[1].contains("(schema schema-issue, markers)"),
        "{refused:?}"
    );
    // Without markers, nothing cites either key or the document.
    tree.write("src/a.rs", "fn main() {}\n");
    let warnings = shown(&validate(&tree).warnings);
    assert_eq!(
        warnings.iter().filter(|w| w.contains("cites key")).count(),
        2,
        "{warnings:#?}"
    );
    let errors = shown(&validate(&tree).errors);
    assert!(
        errors
            .iter()
            .any(|e| e.starts_with("/knowledge/req-cli.md: ")
                && e.contains("no `sokf-tests:` marker cites this document")),
        "{errors:#?}"
    );
}

// sokf-tests: EXT-9_AC-8
// sokf-tests: CONF-7_AC-2
#[test]
fn with_markers_off_no_marker_entry_is_judged_and_the_schema_warns() {
    let tree = TempTree::fixture("clean");
    demand_markers(&tree);
    tree.write("src/a.rs", &source());
    tree.write(".sokf/config.toml", "[markers]\ncheck = \"warn\"\n");
    let report = validate(&tree);
    // Validation reports a refused marker as a warning, and a missing
    // citation keeps the severity of its `cited-by` entry.
    assert!(
        shown(&report.errors)
            .iter()
            .all(|e| !e.starts_with("/src/")),
        "{:?}",
        report.errors
    );
    assert_eq!(
        shown(&report.warnings)
            .iter()
            .filter(|w| w.contains("whose schema admits"))
            .count(),
        2
    );
    // When no marker cites the document, its entry is still an error.
    tree.write("src/a.rs", "fn main() {}\n");
    let errors = shown(&validate(&tree).errors);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("no `sokf-tests:` marker cites this document")),
        "{errors:#?}"
    );
    tree.write(".sokf/config.toml", "[markers]\ncheck = \"off\"\n");
    tree.write("src/a.rs", "fn main() {}\n");
    let report = validate(&tree);
    assert!(report.errors.is_empty(), "{:?}", shown(&report.errors));
    let warnings = shown(&report.warnings);
    assert!(
        warnings.iter().all(|w| !w.contains("marker cites")),
        "{warnings:#?}"
    );
    let off: Vec<&String> = warnings
        .iter()
        .filter(|w| w.contains("`markers.check` is `off`"))
        .collect();
    assert_eq!(off.len(), 1, "{warnings:#?}");
    assert!(
        off[0].starts_with("/knowledge/schemas/requirement.md:"),
        "{off:?}"
    );
}

// sokf-tests: EXT-9_AC-6
// sokf-tests: CACHE-1_AC-14
#[test]
fn a_changed_marker_checks_the_document_it_cites_again() {
    let tree = TempTree::fixture("clean");
    demand_markers(&tree);
    tree.write(
        "src/a.rs",
        &markers(&[("implements", "CLI-1_AC-1"), ("tests", "req-cli")]),
    );
    age(tree.dir(), 10);
    let (_, full, findings, _) = run(&tree);
    assert!(full);
    let uncited = |findings: &[String]| -> Vec<String> {
        findings
            .iter()
            .filter(|f| f.contains("marker cites"))
            .cloned()
            .collect()
    };
    assert_eq!(uncited(&findings).len(), 1, "{findings:#?}");
    assert!(
        uncited(&findings)[0].contains("`CLI-1_AC-2`"),
        "{findings:#?}"
    );
    // When the marker keeps its target but cites another key, the cache
    // checks the requirement again.
    edit(
        &tree,
        "src/a.rs",
        &markers(&[("implements", "CLI-1_AC-2"), ("tests", "req-cli")]),
    );
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/req-cli.md"), "{dirty:?}");
    assert_eq!(uncited(&findings).len(), 1, "{findings:#?}");
    assert!(
        uncited(&findings)[0].contains("`CLI-1_AC-1`"),
        "{findings:#?}"
    );
    // When the marker keeps its target and key but changes its relationship,
    // the cache checks the requirement again.
    edit(
        &tree,
        "src/a.rs",
        &markers(&[("tests", "CLI-1_AC-2"), ("tests", "req-cli")]),
    );
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/req-cli.md"), "{dirty:?}");
    assert_eq!(uncited(&findings).len(), 2, "{findings:#?}");
    // When a refused marker moves down a line, its finding moves with it.
    edit(
        &tree,
        "src/a.rs",
        &markers(&[("references", "req-cli"), ("tests", "req-cli")]),
    );
    let (_, _, findings, _) = run(&tree);
    assert!(
        findings.iter().any(|f| f.starts_with("/src/a.rs:Some(1):")),
        "{findings:#?}"
    );
    edit(
        &tree,
        "src/a.rs",
        &format!(
            "\n{}",
            markers(&[("references", "req-cli"), ("tests", "req-cli")])
        ),
    );
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/req-cli.md"), "{dirty:?}");
    let refused: Vec<&String> = findings
        .iter()
        .filter(|f| f.starts_with("/src/a.rs"))
        .collect();
    assert_eq!(refused.len(), 1, "{findings:#?}");
    assert!(refused[0].starts_with("/src/a.rs:Some(2):"), "{refused:?}");
    // When nothing changes, no check runs again, and the store still
    // reports the refused marker in its file, using the output it saved for
    // the requirement.
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.starts_with("/src/a.rs:Some(2):")),
        "{findings:#?}"
    );
    // When the markers go and the file stays, their citations go with them.
    edit(&tree, "src/a.rs", "fn main() {}\n");
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/req-cli.md"), "{dirty:?}");
    assert_eq!(uncited(&findings).len(), 3, "{findings:#?}");
    assert!(
        findings.iter().all(|f| !f.starts_with("/src/a.rs")),
        "{findings:#?}"
    );
    // When the markers come back and then the file goes, every citation
    // goes with the file, and so does the refused marker's finding.
    edit(
        &tree,
        "src/a.rs",
        &markers(&[("references", "req-cli"), ("tests", "req-cli")]),
    );
    let (_, _, findings, _) = run(&tree);
    assert_eq!(uncited(&findings).len(), 2, "{findings:#?}");
    std::fs::remove_file(tree.dir().join("src/a.rs")).unwrap();
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/req-cli.md"), "{dirty:?}");
    assert_eq!(uncited(&findings).len(), 3, "{findings:#?}");
    assert!(
        findings.iter().all(|f| !f.starts_with("/src/a.rs")),
        "{findings:#?}"
    );
}

// sokf-tests: EXT-9_AC-6
// sokf-tests: CACHE-1_AC-23
#[test]
fn a_hinted_marker_change_checks_the_document_it_cites_again() {
    let tree = TempTree::fixture("clean");
    demand_markers(&tree);
    tree.write(
        "src/a.rs",
        &markers(&[("implements", "CLI-1_AC-1"), ("tests", "req-cli")]),
    );
    age(tree.dir(), 10);
    run(&tree);
    // When the document's `tests` marker becomes an `implements` marker,
    // no marker satisfies the document's `cited-by` entry, and because that
    // entry's severity is error, validation reports an error.
    edit(
        &tree,
        "src/a.rs",
        &markers(&[("implements", "CLI-1_AC-1"), ("implements", "req-cli")]),
    );
    let (dirty, errors, _) = common::incremental::run_hinted(&tree, &["/src/a.rs"]);
    assert!(dirty.contains("/knowledge/req-cli.md"), "{dirty:?}");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("no `sokf-tests:` marker cites this document")),
        "{errors:#?}"
    );
}
