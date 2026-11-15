//! These tests check six rules of the core specifications that sokf once
//! missed: a manifest's `renamed-from` as a scalar, a file in `schemas/`
//! that is not a Schema document, an anchor in a document with no schema, an
//! include whose source the same fix repairs, a sidecar's `resource` that the
//! fix could not reach, and a `sokf:only` marker inside inline code.

use common::TempTree;
use sokf_core::CacheMode;

use crate::common;

/// `errors` returns each error of a run as `path:line: message (criterion)`.
fn errors(tree: &TempTree) -> Vec<String> {
    tree.validate(false, CacheMode::Off)
        .errors
        .iter()
        .map(|f| {
            format!(
                "{}:{}: {} ({})",
                f.path,
                f.line.unwrap_or(0),
                f.message,
                f.criterion
            )
        })
        .collect()
}

// sokf-tests: SOKF_FORMAT_11
#[test]
fn a_renamed_from_that_is_not_a_list_is_reported() {
    let tree = TempTree::fixture("clean");
    let manifest = tree.read("knowledge/manifest.sokf.yaml");
    tree.write(
        "knowledge/manifest.sokf.yaml",
        &format!("{manifest}renamed-from: example-org/old\n"),
    );
    assert_eq!(
        errors(&tree),
        vec![
            "/knowledge/manifest.sokf.yaml:5: `renamed-from` must be a list of `<scope>/<name>` entries (SOKF_FORMAT_11)"
        ]
    );
}

// sokf-tests: SOKF_FORMAT_3
#[test]
fn schemas_holds_only_schema_documents_and_the_index() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/sub/note.md",
        "---\ntype: Decision\nid: a-note\n---\n\n# A note\n",
    );
    tree.write("knowledge/schemas/notes.txt", "notes\n");
    let found: Vec<String> = errors(&tree)
        .into_iter()
        .filter(|e| e.ends_with("(SOKF_FORMAT_3)"))
        .collect();
    let says = |path: &str| {
        format!(
            "{path}:0: `schemas/` holds only Schema documents, each directly in it, and the index (SOKF_FORMAT_3)"
        )
    };
    assert_eq!(
        found,
        vec![
            says("/knowledge/schemas/notes.txt"),
            says("/knowledge/schemas/sub/note.md"),
        ]
    );
}

// sokf-tests: SOKF_FORMAT_83
#[test]
fn an_anchor_in_a_document_with_no_schema_is_reported() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/note.md",
        "---\ntype: Note\nid: note\n---\n\n# A note\n\n<a id=\"x-1\"></a>X-1 text\n",
    );
    let found: Vec<String> = errors(&tree)
        .into_iter()
        .filter(|e| e.starts_with("/knowledge/note.md:"))
        .collect();
    assert_eq!(
        found,
        vec![
            "/knowledge/note.md:2: type `Note` has no schema (SOKF_SCHEMA_13)",
            "/knowledge/note.md:8: the anchor `#x-1` names no key: no schema governs the document, so it has none (SOKF_FORMAT_83)",
        ]
    );
}

// sokf-tests: SOKF_VALIDATION_15
// sokf-tests: SOKF_VALIDATION_37
#[test]
fn one_fix_expands_an_include_from_the_source_it_repairs() {
    for cache in [CacheMode::Off, CacheMode::On] {
        let tree = TempTree::fixture("clean");
        let clean = tree.files();
        // The fix rewrites the setext heading of the source as an ATX
        // heading, and the include in the overview must hold the ATX form.
        let source = tree.read("knowledge/req-cli.md");
        tree.write(
            "knowledge/req-cli.md",
            &source.replace("## User Stories\n", "User Stories\n------------\n"),
        );
        let report = tree.validate(true, cache);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(tree.files(), clean, "{cache:?}");
        let again = tree.validate(true, cache);
        assert!(again.fixes.is_empty(), "{:?}", again.fixes);
    }
}

// sokf-tests: SOKF_VALIDATION_22
#[test]
fn the_fix_rewrites_a_resource_with_no_id_line_or_of_several_lines() {
    let tree = TempTree::fixture("assets");
    let sidecar = "knowledge/data/rates.csv.md";
    let original = tree.read(sidecar);
    tree.write(
        sidecar,
        &original.replace("id: rates\n", "").replace(
            "resource: /knowledge/data/rates.csv",
            "resource: /wrong/rates.csv",
        ),
    );
    tree.validate(true, CacheMode::Off);
    assert_eq!(
        tree.read(sidecar),
        original.replace("id: rates\n", ""),
        "the fix keeps the other lines"
    );
    tree.write(
        sidecar,
        &original.replace(
            "resource: /knowledge/data/rates.csv",
            "resource:\n  - /wrong/a.csv\n  - /wrong/b.csv",
        ),
    );
    tree.validate(true, CacheMode::Off);
    assert_eq!(tree.read(sidecar), original);
}

// A marker inside inline code is text that quotes a marker, not a marker.
#[test]
fn a_sokf_only_marker_in_inline_code_is_text() {
    let tree = TempTree::fixture("clean");
    let path = "knowledge/decisions/decision-single-binary.md";
    let text = tree.read(path);
    tree.write(
        path,
        &text.replace(
            "## Decision\n\n",
            "## Decision\n\nThe marker `<!-- sokf:only state=open -->` is text here.\n\n",
        ),
    );
    assert_eq!(errors(&tree), Vec::<String>::new());
}
