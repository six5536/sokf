//! These tests cover markers (sokf-format-spec §9.5): sokf searches the
//! repository files outside the knowledge roots for markers, resolves each
//! marker, and reports each fault at its line. When a document that a marker
//! file cites changes, the cache checks the marker file again without reading
//! it.

use common::{
    TempTree, age,
    incremental::{edit, run, run_hinted, set},
};
use sokf_core::{CacheMode, Finding};

use crate::common;

/// Markers that resolve, one in each form of a reference.
const GOOD: &str = include_str!("../fixtures/marker-sources/good.rs");

/// Faulty markers and ignore directives, one on each line.
const BAD: &str = include_str!("../fixtures/marker-sources/bad.rs");

/// `on_src` formats each finding on a file under `/src/` as
/// `path:line: message (KEY)`.
fn on_src(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .filter(|f| f.path.as_str().starts_with("/src/"))
        .map(|f| {
            format!(
                "{}:{}: {}",
                f.path,
                f.line.unwrap_or(0),
                sokf_core::report_message(f)
            )
        })
        .collect()
}

// sokf-tests: SOKF_FORMAT_142
// sokf-tests: SOKF_FORMAT_143
// sokf-tests: SOKF_FORMAT_144
// sokf-tests: SOKF_FORMAT_145
// sokf-tests: SOKF_FORMAT_146
// sokf-tests: SOKF_FORMAT_147
// sokf-tests: SOKF_FORMAT_148
// sokf-tests: SOKF_FORMAT_150
// sokf-tests: SOKF_VALIDATION_50
#[test]
fn markers_in_every_form_of_a_reference_resolve() {
    let tree = TempTree::clean_with(&[("src/good.rs", GOOD)]);
    let report = tree.validate_all(false, CacheMode::Off);
    assert!(on_src(&report.errors).is_empty(), "{:?}", report.errors);
    assert!(on_src(&report.warnings).is_empty(), "{:?}", report.warnings);
}

// sokf-tests: SOKF_FORMAT_144
// sokf-tests: SOKF_FORMAT_145
// sokf-tests: SOKF_FORMAT_147
// sokf-tests: SOKF_FORMAT_151
// sokf-tests: SOKF_FORMAT_153
// sokf-tests: SOKF_VALIDATION_51
// sokf-tests: CONF-7_AC-1
// sokf-tests: CLI-4_AC-41
#[test]
fn each_fault_of_a_marker_is_reported_at_its_line() {
    let tree = TempTree::clean_with(&[("src/bad.rs", BAD)]);
    let report = tree.validate_all(false, CacheMode::Off);
    let errors = on_src(&report.errors);
    insta::assert_snapshot!(
        errors
            .iter()
            .chain(&on_src(&report.warnings))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    let expect = [
        (1, "that a schema declares unique in the repository"),
        (2, "refers to the id `req-none`, which no document carries"),
        (
            3,
            "refers to a fragment that is neither a captured key nor a heading",
        ),
        (
            4,
            "has no reference after it, or has a word after its reference",
        ),
        (
            5,
            "has no reference after it, or has a word after its reference",
        ),
        (10, "`sokf:ignore-end` appears outside an ignore region"),
        (
            11,
            "refers to the store `nostore`, which is not a knowledge store",
        ),
        (12, "has a rel that isn't kebab case"),
    ];
    assert_eq!(errors.len(), expect.len(), "{errors:#?}");
    for ((line, text), got) in expect.iter().zip(&errors) {
        assert!(
            got.starts_with(&format!("/src/bad.rs:{line}: ")) && got.contains(text),
            "line {line}: {got}"
        );
    }
    let warnings = on_src(&report.warnings);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("/src/bad.rs:6: ") && warnings[0].contains("`mitigates`"),
        "{warnings:?}"
    );
}

// sokf-tests: CONF-7_AC-2
// sokf-tests: CONF-7_AC-3
#[test]
fn the_markers_check_warns_or_reads_nothing() {
    let tree = TempTree::clean_with(&[
        ("src/bad.rs", BAD),
        (".sokf/config.toml", "[markers]\ncheck = \"warn\"\n"),
    ]);
    let report = tree.validate_all(false, CacheMode::Off);
    assert!(on_src(&report.errors).is_empty(), "{:?}", report.errors);
    assert_eq!(on_src(&report.warnings).len(), 9);
    tree.write(".sokf/config.toml", "[markers]\ncheck = \"off\"\n");
    let report = tree.validate_all(false, CacheMode::Off);
    assert_eq!(on_src(&report.errors), [] as [std::string::String; 0]);
    assert_eq!(on_src(&report.warnings), [] as [std::string::String; 0]);
}

// sokf-tests: SOKF_FORMAT_149
#[test]
fn an_id_that_two_stores_hold_names_its_store() {
    let tree = TempTree::fixture("two-roots");
    tree.write(
        "src/a.rs",
        // This string is a sample marker file, not a marker of this repository.
        // sokf:ignore-next-line
        "// sokf-references: shared\n// sokf-references: second:shared\n// sokf-references: only\n",
    );
    let report = tree.validate_all(false, CacheMode::Off);
    let errors = on_src(&report.errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("/src/a.rs:1: ")
            && errors[0].contains("which more than one store holds (`first` and `second`)"),
        "{errors:?}"
    );
}

// sokf-tests: CACHE-1_AC-32
// sokf-tests: CACHE-1_AC-33
#[test]
fn a_marker_file_is_read_when_it_changes_and_checked_from_the_store_otherwise() {
    let tree = TempTree::clean_with(&[("src/good.rs", GOOD), ("src/plain.rs", "fn plain() {}\n")]);
    age(tree.dir(), 10);
    let (_, full, findings, _) = run(&tree);
    assert!(full);
    assert!(
        findings.iter().all(|f| !f.starts_with("/src/")),
        "{findings:?}"
    );
    // When nothing changes, the run reads no file for markers.
    let (dirty, _, _, read) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(read, 1, "the manifest alone");
    // When one marker file changes, the run reads and checks that file alone.
    edit(
        &tree,
        "src/good.rs",
        &format!("{GOOD}// sokf-tests: CLI-9_AC-9\n"),
    );
    let (dirty, _, findings, read) = run(&tree);
    assert_eq!(dirty, set(&["/src/good.rs"]));
    // The run reads the manifest, the marker file, and the document whose
    // headings the file's references cite, but no other marker file.
    assert_eq!(read, 3, "the manifest, the marker file, and req-cli");
    assert_eq!(
        findings
            .iter()
            .filter(|f| f.starts_with("/src/good.rs"))
            .count(),
        1,
        "{findings:?}"
    );
    // When a file with no marker changes, the run reads it but checks nothing.
    edit(&tree, "src/plain.rs", "fn plain() { let _ = 1; }\n");
    let (dirty, _, _, read) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(read, 2, "the manifest and the plain file");
    // When the cited key is removed, the run checks the marker file again
    // from its stored rows without reading it. The test makes the file
    // unreadable, so the following finding can come only from the stored rows.
    make_unreadable(&tree, "src/good.rs");
    let rel = "knowledge/req-cli.md";
    let text = tree.read(rel);
    let gone = text.replace("CLI-1_AC-2 [unwanted]", "CLI-1_AC-3 [unwanted]");
    assert_ne!(gone, text);
    edit(&tree, rel, &gone);
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/src/good.rs"), "{dirty:?}");
    assert!(
        findings
            .iter()
            .any(|f| f.starts_with("/src/good.rs:Some(3): `sokf-implements: CLI-1_AC-2`")),
        "{findings:?}"
    );
    make_readable(&tree, "src/good.rs");
    // When the markers are removed, the stored output is removed with them.
    edit(&tree, "src/good.rs", "fn main() {}\n");
    let (dirty, _, findings, _) = run(&tree);
    assert_eq!(dirty, set(&["/src/good.rs"]));
    assert!(
        findings.iter().all(|f| !f.starts_with("/src/")),
        "{findings:?}"
    );
}

// sokf-tests: CACHE-1_AC-23
// sokf-tests: CACHE-1_AC-32
#[test]
fn a_hinted_marker_file_is_read_alone() {
    let tree = TempTree::clean_with(&[("src/good.rs", GOOD), ("src/other.rs", GOOD)]);
    age(tree.dir(), 10);
    run(&tree);
    edit(
        &tree,
        "src/other.rs",
        &format!("{GOOD}// sokf-tests: CLI-9_AC-9\n"),
    );
    edit(
        &tree,
        "src/good.rs",
        &format!("{GOOD}// sokf-tests: CLI-9_AC-9\n"),
    );
    let (dirty, findings, read) = run_hinted(&tree, &["/src/good.rs"]);
    assert_eq!(dirty, set(&["/src/good.rs"]));
    // The run reads the hinted marker file, the manifest, and the document
    // that its references cite, but not the other marker file.
    assert_eq!(read, 3, "the hinted file, the manifest, and req-cli");
    assert!(
        findings.iter().all(|f| !f.starts_with("/src/other.rs")),
        "{findings:?}"
    );
}

#[cfg(unix)]
fn make_unreadable(tree: &TempTree, rel: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = tree.dir().join(rel);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
}

#[cfg(unix)]
fn make_readable(tree: &TempTree, rel: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = tree.dir().join(rel);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[cfg(not(unix))]
fn make_unreadable(_: &TempTree, _: &str) {}

#[cfg(not(unix))]
fn make_readable(_: &TempTree, _: &str) {}

// sokf-tests: CACHE-1_AC-33
#[test]
fn a_marker_file_follows_the_headings_and_keys_it_cites() {
    let tree = TempTree::clean_with(&[(
        "src/a.rs",
        // sokf:ignore-next-line
        "// sokf-references: req-cli#acceptance-criteria\n// sokf-implements: CLI-1_AC-9\n",
    )]);
    age(tree.dir(), 10);
    let (_, _, findings, _) = run(&tree);
    let on_a: Vec<&String> = findings
        .iter()
        .filter(|f| f.starts_with("/src/a.rs"))
        .collect();
    assert_eq!(on_a.len(), 1, "{findings:?}");
    assert!(on_a[0].starts_with("/src/a.rs:Some(2)"), "{on_a:?}");
    // When the cited heading is renamed, the run checks the marker file again.
    let rel = "knowledge/req-cli.md";
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text.replacen("## Acceptance Criteria", "## Criteria", 1),
    );
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/src/a.rs"), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.starts_with("/src/a.rs:Some(1)")),
        "{findings:?}"
    );
    // When the missing key is added, the marker resolves.
    let text = tree.read(rel);
    let more = text.replacen(
        "- <a id=\"cli-1_ac-2\"></a>",
        "- <a id=\"cli-1_ac-9\"></a>CLI-1_AC-9 [unwanted]: IF a flag is unknown, THEN the system SHALL exit with a failure.\n- <a id=\"cli-1_ac-2\"></a>",
        1,
    );
    assert_ne!(more, text);
    edit(
        &tree,
        rel,
        &more.replacen("## Criteria", "## Acceptance Criteria", 1),
    );
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/src/a.rs"), "{dirty:?}");
    assert!(
        findings.iter().all(|f| !f.starts_with("/src/a.rs")),
        "{findings:?}"
    );
}

#[test]
fn an_additional_file_with_markers_has_one_output_for_both_checks() {
    let tree = TempTree::fixture("clean");
    let readme = tree.read("README.md");
    tree.write(
        "README.md",
        &format!("<!-- sokf-references: no-such-doc -->\n{readme}"),
    );
    age(tree.dir(), 10);
    let (_, _, findings, _) = run(&tree);
    assert!(
        findings
            .iter()
            .any(|f| f.starts_with("/README.md:Some(1)") && f.contains("no-such-doc")),
        "{findings:?}"
    );
    // When the readme's schema changes, the run checks the readme again, and
    // the readme still has one output, which holds the finding about its
    // marker.
    let schema = "knowledge/schemas/readme.md";
    let text = tree.read(schema);
    edit(&tree, schema, &format!("{text}\n"));
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/README.md"), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.contains("no-such-doc")),
        "{findings:?}"
    );
}

// sokf-tests: SOKF_VALIDATION_53
#[test]
fn the_fix_rewrites_no_marker() {
    let tree = TempTree::clean_with(&[("src/bad.rs", BAD)]);
    let report = tree.validate_all(true, CacheMode::Off);
    assert_eq!(tree.read("src/bad.rs"), BAD);
    assert!(
        report
            .fixes
            .iter()
            .all(|f| !format!("{f:?}").contains("/src/")),
        "{:?}",
        report.fixes
    );
    assert_ne!(on_src(&report.errors), [] as [std::string::String; 0]);
}

// sokf-tests: CACHE-1_AC-32
#[cfg(unix)]
#[test]
fn a_file_that_does_not_read_is_read_again_the_next_run() {
    let tree = TempTree::clean_with(&[]);
    age(tree.dir(), 10);
    run(&tree);
    tree.write("src/late.rs", "// sokf-implements: CLI-9_AC-9\n");
    common::age_file(&tree.dir().join("src/late.rs"), 10);
    make_unreadable(&tree, "src/late.rs");
    let (_, _, findings, _) = run(&tree);
    assert!(findings.iter().all(|f| !f.starts_with("/src/late.rs")));
    // When the file becomes readable again and its size and modification
    // time are unchanged, the run reads it.
    make_readable(&tree, "src/late.rs");
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/src/late.rs"), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.starts_with("/src/late.rs")),
        "{findings:?}"
    );
}

// A line inside a fenced code block of a Markdown file outside the roots is
// an example, whether it shows a marker or an ignore directive, at any
// depth of the file's blocks, while a fence of any other file hides
// nothing.
// sokf-tests: SOKF_FORMAT_250
#[test]
fn a_fenced_marker_of_a_markdown_file_is_an_example() {
    let guide = "# Guide\n\n```rust\n// sokf-implements: NO-SUCH_AC-1\n// sokf:ignore-begin\n```\n\n~~~\n# sokf-tests: also-none\n~~~\n\n- An item.\n\n  ```\n  // sokf-implements: NESTED-NONE_AC-1\n  ```\n\n// sokf-implements: OUTSIDE-NONE_AC-1\n";
    let source = "/*\n```\n*/\n// sokf-implements: RS-NONE_AC-1\n// ```\n";
    let tree = TempTree::clean_with(&[("docs/guide.md", guide), ("src/fenced.rs", source)]);
    let report = tree.validate_all(false, CacheMode::Off);
    let lines = |path: &str| -> Vec<usize> {
        report
            .errors
            .iter()
            .chain(&report.warnings)
            .filter(|f| f.path.as_str() == path)
            .map(|f| f.line.unwrap_or(0))
            .collect()
    };
    assert_eq!(lines("/docs/guide.md"), [18], "{:?}", report.errors);
    assert_eq!(lines("/src/fenced.rs"), [4], "{:?}", report.errors);
}

// A fence hides its lines in a quote, in a file that a rule opens, and with CRLF line endings, while an indented block of code
// hides nothing.
// sokf-tests: SOKF_FORMAT_250
#[test]
fn a_fence_hides_its_lines_in_a_quote_after_a_rule_and_with_crlf() {
    let notes = "---\n\n# Notes\n\n> ```\n> // sokf-implements: QUOTE-NONE_AC-1\n> ```\n\nText.\n\n    // sokf-implements: INDENTED-NONE_AC-1\n";
    let crlf = "# Crlf\r\n\r\n```\r\n// sokf-implements: CRLF-NONE_AC-1\r\n```\r\n\r\n// sokf-implements: AFTER-NONE_AC-1\r\n";
    let tree = TempTree::clean_with(&[("docs/notes.md", notes), ("docs/crlf.md", crlf)]);
    let report = tree.validate_all(false, CacheMode::Off);
    let lines = |path: &str| -> Vec<usize> {
        report
            .errors
            .iter()
            .filter(|f| f.path.as_str() == path)
            .map(|f| f.line.unwrap_or(0))
            .collect()
    };
    assert_eq!(lines("/docs/notes.md"), [11], "{:?}", report.errors);
    assert_eq!(lines("/docs/crlf.md"), [7], "{:?}", report.errors);
}

// A fence hides its lines in a footnote's definition, in a list item of a
// quote, in a quote of a nested list item, after a quote's mark, a space,
// and a tab, and in a file with a carriage return alone, while a marker
// outside the fences stays an error.
// sokf-tests: SOKF_FORMAT_250
#[test]
fn a_fence_hides_its_lines_at_any_depth_and_after_a_carriage_return_alone() {
    let deep = "# Deep\n\nA note.[^1]\n\n[^1]: The note.\n\n    ```\n    // sokf-implements: FOOT-NONE_AC-1\n    ```\n\n> - ```\n>   // sokf-implements: QLIST-NONE_AC-1\n>   ```\n\n- An item.\n  - A nested item.\n\n    > ```\n    > // sokf-implements: LISTQ-NONE_AC-1\n    > ```\n\n> \t```\n> \t// sokf-implements: TAB-NONE_AC-1\n> \t```\n\n// sokf-implements: OUTSIDE-NONE_AC-1\n";
    let cr = "# Cr\r\r```\n// sokf-implements: CR-NONE_AC-1\n```\n\n// sokf-implements: AFTER-NONE_AC-1\n";
    let tree = TempTree::clean_with(&[("docs/deep.md", deep), ("docs/cr.md", cr)]);
    let report = tree.validate_all(false, CacheMode::Off);
    let lines = |path: &str| -> Vec<usize> {
        report
            .errors
            .iter()
            .filter(|f| f.path.as_str() == path)
            .map(|f| f.line.unwrap_or(0))
            .collect()
    };
    assert_eq!(lines("/docs/deep.md"), [26], "{:?}", report.errors);
    assert_eq!(lines("/docs/cr.md"), [5], "{:?}", report.errors);
}

// A fence whose list item takes a tab after its mark hides its lines, an
// indented block whose first line looks like a fence hides nothing, a fence
// inside an include block of a file outside the roots and after a
// definition block hides its lines, and so does a fence that carriage
// returns alone end.
// sokf-tests: SOKF_FORMAT_250
#[test]
fn only_a_fence_that_markdown_reads_hides_its_lines() {
    let tab = "# Tab\n\n-\t```\n\t// sokf-implements: TAB-NONE_AC-1\n\t```\n";
    let indented = "# Indented\n\n    ```\n    // sokf-implements: IND-NONE_AC-1\n";
    let include = "# Include\n\n<!-- sokf:include knowledge/x.md -->\n\n```\n// sokf-implements: INC-NONE_AC-1\n```\n\n<!-- /sokf:include -->\n\n<!-- sokf:links -->\n\n```\n// sokf-implements: LINKS-NONE_AC-1\n```\n";
    let cr = "# Cr\n\n```\r// sokf-implements: CR-NONE_AC-1\r```\r\r// sokf-implements: CRAFTER-NONE_AC-1\r";
    let tree = TempTree::clean_with(&[
        ("docs/tab.md", tab),
        ("docs/indented.md", indented),
        ("docs/include.md", include),
        ("docs/cr.md", cr),
    ]);
    let report = tree.validate_all(false, CacheMode::Off);
    let lines = |path: &str| -> Vec<usize> {
        report
            .errors
            .iter()
            .filter(|f| f.path.as_str() == path && f.criterion == "SOKF_FORMAT_147")
            .map(|f| f.line.unwrap_or(0))
            .collect()
    };
    let tab_lines = lines("/docs/tab.md");
    assert!(tab_lines.is_empty(), "{tab_lines:?}");
    assert_eq!(lines("/docs/indented.md"), [4], "{:?}", report.errors);
    let include_lines = lines("/docs/include.md");
    assert!(include_lines.is_empty(), "{include_lines:?}");
    // The scan reads the lines that carriage returns alone join as one, and
    // passes them by, because their first word lies in the fence.
    let cr_lines: Vec<usize> = report
        .errors
        .iter()
        .filter(|f| f.path.as_str() == "/docs/cr.md")
        .map(|f| f.line.unwrap_or(0))
        .collect();
    assert!(cr_lines.is_empty(), "{cr_lines:?}");
}
