//! These are the end-to-end tests of `sokf validate`. They check that the JSON
//! output includes every warning, and that the command refuses a path outside
//! the root and a query that does not evaluate. They also check that, when the
//! cache store is not a database, the command rebuilds the store without
//! printing a message.

use std::fs;

use common::Fixture;

use crate::common;

/// `WARNED` is a document with one error and one warning, and the warning
/// comes from `rel: likes`, which is outside the core set.
const WARNED: &str = "---\ntype: Issue\nid: issue-001-retry\ntitle: Retry fails twice\nstate: open\nkind: bug\nlinks:\n  - rel: implements\n    to: decision-single-binary\n  - rel: likes\n    to: decision-single-binary\n---\n\n# Retry fails twice\n\nThe retry of [the one binary][sokf:decision-single-binary] runs twice.\n\n## Reproduction\n\n1. Run the daemon.\n\n<!-- sokf:links -->\n\n[sokf:decision-single-binary]: /knowledge/decisions/decision-single-binary.md\n";

// sokf-tests: CLI-4_AC-13
#[test]
fn json_includes_every_warning_without_the_warnings_flag() {
    let f = Fixture::new("clean");
    f.write("knowledge/issues/open/issue-001-retry.md", WARNED);
    let (stdout, _, code) = f.run(&["validate", "--json"]);
    assert_eq!(code, 1, "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let warnings = doc["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{stdout}");
    assert_eq!(
        warnings[0]["path"],
        "/knowledge/issues/open/issue-001-retry.md"
    );
    assert_eq!(warnings[0]["line"], 10);
    assert!(
        warnings[0]["message"]
            .as_str()
            .unwrap()
            .contains("`rel: likes` is outside the core set"),
        "{stdout}"
    );
}

// sokf-tests: CLI-4_AC-15
#[test]
fn a_path_outside_the_root_exits_2() {
    let f = Fixture::new("clean");
    let (stdout, stderr, code) = f.run_here(&["validate", ".."]);
    assert_eq!(code, 2, "{stderr}");
    assert_eq!(stdout, "");
    assert!(stderr.starts_with("error: ..: "), "{stderr}");
}

// sokf-tests: CLI-4_AC-17
#[test]
fn a_query_that_does_not_evaluate_exits_2() {
    let f = Fixture::new("clean");
    let (stdout, stderr, code) = f.run(&["validate", "--json", "--query", "abs('x')"]);
    assert_eq!(code, 2, "{stderr}");
    assert_eq!(stdout, "");
    assert!(stderr.starts_with("error: query: "), "{stderr}");
}

// sokf-tests: CLI-4_AC-19
#[test]
fn a_store_that_is_not_a_database_is_rebuilt_with_no_message() {
    let f = Fixture::new("clean");
    let first = f.run(&["validate"]);
    assert_eq!(first, ("0 errors, 0 warnings\n".into(), String::new(), 0));
    // The test replaces the store with bytes that are not a database.
    let dir = f.0.join(".sokf/cache");
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("sokf.sqlite") {
            fs::remove_file(&path).unwrap();
        }
    }
    fs::write(
        dir.join("sokf.sqlite"),
        "not sqlite at all, and long enough to matter",
    )
    .unwrap();
    let second = f.run(&["validate"]);
    assert_eq!(second, first);
    let third = f.run(&["validate"]);
    assert_eq!(third, first);
}
