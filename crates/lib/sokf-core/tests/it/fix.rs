//! These tests run `--fix` on a fixture tree, snapshot the report and the
//! tree that the fix leaves, and check that a second run fixes nothing.

use std::fmt::Write as _;

use common::{TempTree, json};
use sokf_core::CacheMode;

use crate::common;

// sokf-tests: SOKF_VALIDATION_14
// sokf-tests: SOKF_VALIDATION_15
// sokf-tests: SOKF_VALIDATION_17
// sokf-tests: SOKF_VALIDATION_19
// sokf-tests: SOKF_VALIDATION_25
// sokf-tests: SOKF_VALIDATION_26
// sokf-tests: SOKF_VALIDATION_27
// sokf-tests: SOKF_VALIDATION_33
// sokf-tests: SOKF_VALIDATION_37
// sokf-tests: SOKF_VALIDATION_38
// sokf-tests: SOKF_VALIDATION_39
#[test]
fn fix_rewrites_moves_and_then_runs_clean() {
    let tree = TempTree::fixture("fixable");
    let before = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!("fixable_before", json(&before));
    let fixed = tree.validate(true, CacheMode::Off);
    insta::assert_snapshot!("fixable_report", json(&fixed));
    let mut dump = String::new();
    for (rel, text) in tree.files() {
        if [
            "README.md",
            "knowledge/decisions/decision-single-binary.md",
            "knowledge/issues/closed/issue-002-done.md",
            "knowledge/overview.md",
        ]
        .contains(&rel.as_str())
        {
            let _ = write!(dump, "==== {rel}\n{text}");
        }
    }
    insta::assert_snapshot!("fixable_tree", dump);
    assert!(
        !tree
            .dir()
            .join("knowledge/issues/open/issue-002-done.md")
            .exists()
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
    assert_eq!(again.errors, fixed.errors);
}

// sokf-tests: CLI-4_AC-5
#[test]
fn fix_within_paths_repairs_only_those_files_plus_moved_definitions() {
    let tree = TempTree::fixture("fixable");
    let report = tree.validate_paths(&["knowledge/issues"], true);
    let moved: Vec<String> = report
        .fixes
        .iter()
        .map(|f| serde_json::to_string(f).unwrap())
        .collect();
    assert_eq!(moved.len(), 6, "{moved:?}");
    // The fix makes six changes:
    // - The move
    // - The new index of the directory that the file enters
    // - The rewrite of the moved file
    // - The rewrite of the index of `issues/`, which gains the new subdirectory
    // - The rewrite of the index of the directory that the file leaves
    // - The rewrite of the definition block that held the old path, although
    //   that block is out of scope
    assert!(moved[0].contains("moved"));
    assert!(
        moved[1].contains(r#""op":"created","path":"/knowledge/issues/closed/index.md""#),
        "{moved:?}"
    );
    assert!(
        moved
            .iter()
            .any(|m| m.contains("knowledge/issues/open/index.md"))
    );
    assert!(moved.iter().any(|m| m.contains("knowledge/overview.md")));
    assert!(
        tree.read("knowledge/issues/closed/index.md")
            .contains("[sokf:issue-002-done]: /knowledge/issues/closed/issue-002-done.md")
    );
    assert!(
        !tree
            .read("knowledge/issues/open/index.md")
            .contains("issue-002-done")
    );
    assert!(
        tree.read("knowledge/overview.md").contains("stale"),
        "the fix leaves the out-of-scope include as it was"
    );
    assert!(
        tree.read("knowledge/overview.md")
            .contains("/knowledge/issues/closed/issue-002-done.md")
    );
    assert!(
        tree.read("README.md").contains("fn old()"),
        "the out-of-scope file is unchanged"
    );
}

// sokf-tests: SOKF_VALIDATION_20
// sokf-tests: SOKF_VALIDATION_21
#[test]
fn a_refused_move_is_a_finding_and_nothing_moves() {
    let tree = TempTree::fixture("fixable");
    tree.write("knowledge/issues/closed/issue-002-done.md", "occupied\n");
    let report = tree.validate(true, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.message.contains("cannot move the document")),
        "{}",
        json(&report)
    );
    assert!(
        tree.dir()
            .join("knowledge/issues/open/issue-002-done.md")
            .exists()
    );
    assert_eq!(
        tree.read("knowledge/issues/closed/issue-002-done.md"),
        "occupied\n"
    );
}

// sokf-tests: SOKF_FORMAT_96
// sokf-tests: SOKF_FORMAT_98
// sokf-tests: SOKF_FORMAT_99
// sokf-tests: SOKF_FORMAT_100
// sokf-tests: SOKF_FORMAT_101
// sokf-tests: SOKF_FORMAT_103
// sokf-tests: SOKF_FORMAT_108
// sokf-tests: SOKF_FORMAT_109
// sokf-tests: SOKF_FORMAT_110
// sokf-tests: SOKF_SCHEMA_5
// sokf-tests: SOKF_VALIDATION_33
// sokf-tests: SOKF_VALIDATION_35
// sokf-tests: SOKF_VALIDATION_36
// sokf-tests: SOKF_VALIDATION_37
// sokf-tests: CLI-12_AC-1
// sokf-tests: CLI-12_AC-2
#[test]
fn fix_writes_every_missing_and_stale_index_after_the_moves() {
    let tree = TempTree::fixture("indexes");
    let fixed = tree.validate(true, CacheMode::Off);
    insta::assert_snapshot!("indexes_report", json(&fixed));
    let mut dump = String::new();
    for (rel, text) in tree.files() {
        if rel.ends_with("index.md") {
            let _ = write!(dump, "==== {rel}\n{text}");
        }
    }
    insta::assert_snapshot!("indexes_tree", dump.replace("\r\n", "<CRLF>\n"));
    // The index with CRLF line endings keeps them, and an index that the
    // fix creates has LF line endings.
    assert!(tree.read("knowledge/crlf/index.md").contains("\r\n"));
    assert!(!tree.read("knowledge/none/index.md").contains('\r'));
    // The one error left is the finding about the glob, which is on the
    // schema, not on an index.
    assert_eq!(fixed.errors.len(), 1, "{}", json(&fixed));
    assert!(fixed.errors[0].message.contains("matches the index"));
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
    assert_eq!(again.errors, fixed.errors);
}

// sokf-tests: CLI-12_AC-2
#[test]
fn fix_keeps_each_files_line_ending() {
    let tree = TempTree::fixture("fixable");
    for (rel, text) in tree.files() {
        if rel.ends_with("overview.md") || rel == "README.md" {
            tree.write(&rel, &text.replace('\n', "\r\n"));
        }
    }
    tree.validate(true, CacheMode::Off);
    assert!(tree.read("README.md").contains("\r\n"));
    assert!(!tree.read("README.md").replace("\r\n", "").contains('\n'));
    assert!(tree.read("knowledge/overview.md").contains("\r\n"));
    assert!(
        !tree
            .read("knowledge/decisions/decision-single-binary.md")
            .contains('\r')
    );
}

// sokf-tests: FORMAT-1_AC-5
// sokf-tests: CLI-4_AC-23
// sokf-tests: FORMAT-2_AC-2
// sokf-tests: CLI-4_AC-24
#[test]
fn fix_moves_the_pair_and_rewrites_a_sidecars_resource() {
    let tree = TempTree::fixture("assets");
    let fixed = tree.validate(true, CacheMode::Off);
    insta::assert_snapshot!("assets_report", json(&fixed));
    let mut dump = String::new();
    for (rel, text) in tree.files() {
        if [
            "knowledge/media/draft/index.md",
            "knowledge/media/draft/stale.png.md",
            "knowledge/media/index.md",
            "knowledge/media/published/index.md",
            "knowledge/media/published/sunset.png.md",
            "knowledge/notes/album.md",
        ]
        .contains(&rel.as_str())
        {
            let _ = write!(dump, "==== {rel}\n{text}");
        }
    }
    insta::assert_snapshot!("assets_tree", dump);
    // The asset moves with its sidecar, and the fix neither reads nor
    // changes the asset.
    assert!(!tree.dir().join("knowledge/media/draft/sunset.png").exists());
    assert_eq!(
        tree.read("knowledge/media/published/sunset.png"),
        "PNG sunset\n"
    );
    // The `resource` line is the only frontmatter change.
    let stale = tree.read("knowledge/media/draft/stale.png.md");
    assert!(stale.starts_with(
        "---\ntype: Image\nid: stale\ntitle: Stale\nstate: draft\nresource: /knowledge/media/draft/stale.png\n---\n"
    ));
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
    assert_eq!(again.errors, fixed.errors);
}

// sokf-tests: CLI-4_AC-23
#[test]
fn a_pair_move_is_refused_when_either_target_exists() {
    let tree = TempTree::fixture("assets");
    tree.write("knowledge/media/published/sunset.png", "occupied\n");
    let report = tree.validate(true, CacheMode::Off);
    assert!(
        report.errors.iter().any(|f| f.message.contains(
            "cannot move the sidecar and its asset to /knowledge/media/published/sunset.png"
        )),
        "{}",
        json(&report)
    );
    assert!(tree.dir().join("knowledge/media/draft/sunset.png").exists());
    assert!(
        tree.dir()
            .join("knowledge/media/draft/sunset.png.md")
            .exists()
    );
    assert!(
        !tree
            .dir()
            .join("knowledge/media/published/sunset.png.md")
            .exists()
    );
}

// sokf-tests: CLI-4_AC-24
// sokf-tests: CLI-4_AC-23
// sokf-tests: CLI-4_AC-5
#[test]
fn paths_naming_the_sidecar_move_its_asset_too() {
    let tree = TempTree::fixture("assets");
    let report = tree.validate_paths(&["knowledge/media/draft/sunset.png.md"], true);
    let moved: Vec<String> = report
        .fixes
        .iter()
        .filter_map(|f| serde_json::to_string(f).ok())
        .filter(|f| f.contains("moved"))
        .collect();
    assert_eq!(moved.len(), 2, "{moved:?}");
    assert!(
        moved[0].contains("/knowledge/media/draft/sunset.png\""),
        "{moved:?}"
    );
    assert!(
        moved[1].contains("/knowledge/media/draft/sunset.png.md\""),
        "{moved:?}"
    );
    assert!(
        tree.dir()
            .join("knowledge/media/published/sunset.png")
            .exists()
    );
    // The stale `resource` line stays, because its file is not in `PATHS`.
    assert!(
        tree.read("knowledge/media/draft/stale.png.md")
            .contains("old-name.png")
    );
}

// A note lies outside the root of its schema, which has no path-enforced
// dimension. The check reports it, the fix moves it into the root, and a
// second run fixes nothing.
// sokf-tests: SOKF_SCHEMA_51
// sokf-tests: SOKF_VALIDATION_54
// sokf-tests: EXT-7_AC-4
#[test]
fn fix_moves_a_document_outside_its_root_into_the_root() {
    let tree = TempTree::fixture("new");
    let schema = tree.read("knowledge/schemas/note.md");
    tree.write(
        "knowledge/schemas/note.md",
        &schema.replacen(
            "line-limit: 40\n",
            "line-limit: 40\nroot: /knowledge/notes\n",
            1,
        ),
    );
    let note = tree.read("knowledge/notes/note-existing.md");
    tree.write(
        "knowledge/stray.md",
        &note.replace("note-existing", "stray"),
    );
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.criterion == "SOKF_SCHEMA_51" && f.path.as_str() == "/knowledge/stray.md"),
        "{}",
        json(&report)
    );
    tree.validate(true, CacheMode::Off);
    assert!(!tree.dir().join("knowledge/stray.md").exists());
    assert!(tree.dir().join("knowledge/notes/stray.md").exists());
    let again = tree.validate(true, CacheMode::Off);
    assert!(
        !again.errors.iter().any(|f| f.criterion == "SOKF_SCHEMA_51"),
        "{}",
        json(&again)
    );
}

// A move into a directory that doesn't exist yet adds that directory to the
// index of its parent, and one fix writes that index too, so a second run
// finds nothing to fix. The move can create one directory, or two levels of
// them when a second dimension also sets the path.
// sokf-tests: SOKF_VALIDATION_33
// sokf-tests: SOKF_VALIDATION_37
// sokf-tests: SOKF_VALIDATION_55
#[test]
fn one_fix_writes_the_index_above_a_directory_that_a_move_creates() {
    for (kind_path, landed) in [
        (false, "knowledge/issues/deferred/issue-001-retry.md"),
        (true, "knowledge/issues/deferred/bug/issue-001-retry.md"),
    ] {
        let tree = TempTree::fixture("clean");
        let schema = tree.read("knowledge/schemas/issue.md").replace(
            "states: [open, closed], path: true",
            "states: [open, closed, deferred], path: true",
        );
        let schema = if kind_path {
            schema.replace(
                "{ key: kind, states: [bug, feature] }",
                "{ key: kind, states: [bug, feature], path: true }",
            )
        } else {
            schema
        };
        tree.write("knowledge/schemas/issue.md", &schema);
        if kind_path {
            // Settle the issue in its directory of `kind` before the move
            // under test.
            tree.validate(true, CacheMode::On);
        }
        let issue = if kind_path {
            "knowledge/issues/open/bug/issue-001-retry.md"
        } else {
            "knowledge/issues/open/issue-001-retry.md"
        };
        tree.write(
            issue,
            &tree
                .read(issue)
                .replace("state: open\n", "state: deferred\n"),
        );
        let fixed = tree.validate(true, CacheMode::On);
        assert!(tree.dir().join(landed).exists(), "{}", json(&fixed));
        assert!(fixed.errors.is_empty(), "{}", json(&fixed));
        assert!(
            tree.read("knowledge/issues/index.md").contains("deferred/"),
            "{}",
            tree.read("knowledge/issues/index.md")
        );
        let again = tree.validate(true, CacheMode::On);
        assert!(again.fixes.is_empty(), "{}", json(&again));
    }
}

// sokf-tests: SOKF_VALIDATION_56
// sokf-tests: CLI-4_AC-35
#[test]
fn fix_leaves_a_hand_written_index_and_reports_it_once() {
    let page = "---\ntitle: Home\n---\n\n# Home\n\nWelcome to the knowledge of the project.\n";
    let tree = TempTree::clean_with(&[("knowledge/index.md", page)]);
    let fixed = tree.validate(true, CacheMode::Off);
    assert!(
        fixed.fixes.iter().all(|f| !serde_json::to_string(f)
            .unwrap()
            .contains("/knowledge/index.md")),
        "{}",
        json(&fixed)
    );
    assert_eq!(tree.read("knowledge/index.md"), page);
    let on_index: Vec<String> = fixed
        .errors
        .iter()
        .filter(|f| f.path.as_str() == "/knowledge/index.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        on_index,
        vec!["the file is not a generated index, so `--fix` doesn't overwrite it: move its content into a document, then run `--fix` (SOKF_FORMAT_97)".to_string()]
    );
}

/// `FIX_BROKEN_SCHEMA_KEPT` lists the files of the `fix-broken-schema`
/// fixture whose schema breaks the grammar: a document and an additional
/// file, each with anchors, references to respell, and a missing definition
/// block.
const FIX_BROKEN_SCHEMA_KEPT: [&str; 2] = ["README.md", "knowledge/a.md"];

/// `FIX_BROKEN_SCHEMA_THING` is the document of the `fix-broken-schema`
/// fixture that no schema governs, as the fix leaves it. Its anchor is
/// longer than the code span after it, so a code span read at its columns
/// without the anchor would lie before the text that it holds.
const FIX_BROKEN_SCHEMA_THING: &str = "---\ntype: Thing\nid: b\ntitle: B\ndescription: A thing that no schema governs.\n---\n\n# B\n\nThe thing links [the note][sokf:a] and [the note again][sokf:a].\n\n- <a id=\"t-1-an-anchor-longer-than-the-code-after-it\"></a>T-1 `[the note](/knowledge/a.md)` in code, then [the note][sokf:a].\n\n<!-- sokf:links -->\n\n[sokf:a]: /knowledge/a.md\n";

// sokf-tests: CLI-4_AC-106
// sokf-tests: CLI-4_AC-107
// sokf-tests: SOKF_VALIDATION_41
#[test]
fn a_fix_under_a_broken_schema_changes_none_of_its_files() {
    for cache in [CacheMode::Off, CacheMode::On] {
        let tree = TempTree::fixture("fix-broken-schema");
        let before = tree.files();
        let fixed = tree.validate(true, cache);
        insta::assert_snapshot!("fix_broken_schema_report", json(&fixed));
        // The fix changes no file whose schema breaks the grammar, and the
        // report names each one as a file that sokf doesn't check.
        for rel in FIX_BROKEN_SCHEMA_KEPT {
            let held = &before.iter().find(|(r, _)| r == rel).unwrap().1;
            assert_eq!(&tree.read(rel), held, "{rel}");
            assert!(
                fixed
                    .errors
                    .iter()
                    .any(|f| f.criterion == "SOKF_VALIDATION_41"
                        && f.path.as_str() == format!("/{rel}")),
                "{rel}: {}",
                json(&fixed)
            );
        }
        // The fix keeps the anchor of the document that no schema governs,
        // and still respells its references, but none inside a code span.
        assert_eq!(tree.read("knowledge/b.md"), FIX_BROKEN_SCHEMA_THING);
        let again = tree.validate(true, cache);
        assert!(again.fixes.is_empty(), "{}", json(&again));
        assert_eq!(again.errors, fixed.errors);
    }
}
