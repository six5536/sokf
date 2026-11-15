//! This module tests the generated index (`SOKF_FORMAT_96` to `SOKF_FORMAT_110`)
//! end to end, through the binary: the error lines, the `created` line and
//! its JSON operation, and how `validate --fix` handles `PATHS` when a
//! document moves.

use common::Fixture;

use crate::common;

// sokf-tests: CLI-1_AC-10
// sokf-tests: CLI-4_AC-7
// sokf-tests: CLI-4_AC-27
#[test]
fn a_missing_and_a_stale_index_are_errors_at_the_index_path() {
    let f = Fixture::new("indexes");
    let (stdout, stderr, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert!(
        stdout.contains(
            "/knowledge/none/index.md: error: the directory has no `index.md` (SOKF_FORMAT_96)\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("/knowledge/sub/index.md: error: the index is stale: it differs from the generated content (SOKF_FORMAT_97)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("/knowledge/schemas/glob.md:7: error: the `additional-files` glob `/knowledge/sub/index.md` matches the index `/knowledge/sub/index.md` (SOKF_SCHEMA_5)\n"),
        "{stdout}"
    );
    // The CRLF index is current after its line endings are normalised. The
    // misplaced issue is the only other error.
    assert!(!stdout.contains("/knowledge/crlf/index.md"), "{stdout}");
    assert!(stdout.ends_with("\n4 errors, 0 warnings\n"), "{stdout}");
    assert_eq!(stderr, "");
}

// sokf-tests: CLI-4_AC-5
// sokf-tests: CLI-4_AC-6
// sokf-tests: CLI-4_AC-26
// sokf-tests: SOKF_VALIDATION_33
// sokf-tests: SOKF_VALIDATION_55
#[test]
fn paths_with_a_move_write_the_indexes_it_touches() {
    let f = Fixture::new("indexes");
    // `PATHS` holds only the document. `validate --fix` still writes the index
    // of the directory the document leaves, the index of the directory it
    // enters, and the index above that directory, which the move creates, so
    // that the parent lists it (CLI-4_AC-5, SOKF_VALIDATION_55). It writes no
    // other index.
    let (stdout, _, code) = f.run(&[
        "validate",
        "--fix",
        "/knowledge/issues/open/issue-001-move.md",
    ]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        stdout,
        "moved   /knowledge/issues/open/issue-001-move.md -> /knowledge/issues/closed/issue-001-move.md\n\
         created /knowledge/issues/closed/index.md\n\
         rewrote /knowledge/issues/index.md\n\
         rewrote /knowledge/issues/open/index.md\n\
         \n0 errors, 0 warnings\n"
    );
    assert!(!f.0.join("knowledge/none/index.md").exists());
    // A second run without `PATHS` makes the remaining fixes and reports
    // them as JSON: it writes the missing index and the stale one.
    let (stdout, _, code) = f.run(&["validate", "--fix", "--json"]);
    assert_eq!(code, 1, "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        doc["fixes"],
        serde_json::json!([
            {"op": "created", "path": "/knowledge/none/index.md"},
            {"op": "rewrote", "path": "/knowledge/sub/index.md"}
        ])
    );
    assert_eq!(doc["errors"].as_array().unwrap().len(), 1, "{stdout}");
    let (stdout, _, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(
        stdout.starts_with("/knowledge/schemas/glob.md:7: error"),
        "{stdout}"
    );
}
