//! These tests check the incremental run (contract-cache): a change dirties
//! the documents that read what changed and no other document, and the
//! report of a warm run equals the report of a run with no cache.

use std::fs;

use common::{
    TempTree, age,
    incremental::{cold, edit, held, run, run_hinted, set},
    lines,
};
use sokf_core::CacheMode;

use crate::common;

// sokf-tests: CACHE-1_AC-4
// sokf-tests: CACHE-1_AC-5
// sokf-tests: CACHE-1_AC-17
// sokf-tests: CACHE-1_AC-25
// sokf-tests: CACHE-3_AC-1
// sokf-tests: CACHE_P-3
// sokf-tests: CLI-4_AC-20
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the test walks one scenario in order, which splitting would hide"
)]
fn a_change_dirties_its_readers_and_no_other_document() {
    let tree = TempTree::fixture("clean");
    // The requirement schema takes `requirement-ears` from the repository
    // layer, rather than from its own formats, so that a change of that
    // layer reaches it.
    let rel = "knowledge/schemas/requirement.md";
    let schema = tree.read(rel);
    let own = schema
        .find("\ntypes:\n")
        .expect("the schema declares its formats");
    tree.write(rel, &format!("{}\n````\n", &schema[..own]));
    tree.take_requirement_formats();
    age(tree.dir(), 10);
    let (dirty, full, findings, _) = run(&tree);
    assert!(full);
    assert!(findings.is_empty(), "{findings:?}");
    // The first run checks the four documents, the five schema documents,
    // and the readme.
    assert_eq!(dirty.len(), 10, "{dirty:?}");

    // When nothing changes, no document is dirty and the run reads no
    // document.
    let (dirty, full, findings, read) = run(&tree);
    assert!(!full);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert!(findings.is_empty(), "{findings:?}");
    // The run reads the manifest alone: no schema, no index, and no
    // additional file.
    assert_eq!(read, 1, "read {read} files");

    // A body edit that changes no fact dirties the document alone.
    let rel = "knowledge/decisions/decision-single-binary.md";
    let text = tree.read(rel);
    edit(&tree, rel, &format!("{text}\nA line more.\n"));
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(
        dirty,
        set(&["/knowledge/decisions/decision-single-binary.md"])
    );
    assert_eq!(findings, cold(&tree));
    // The run holds only the changed document, and leaves the files that the
    // schemas and the readme read on disk.
    assert_eq!(
        held(),
        set(&["/knowledge/decisions/decision-single-binary.md"])
    );

    // A title is a fact that its readers read. The overview links to the
    // requirement and includes it, so a change to the title dirties the
    // overview. The decision does neither, so it stays clean.
    let rel = "knowledge/req-cli.md";
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text.replacen("title: CLI requirement", "title: The CLI", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(
        dirty,
        set(&["/knowledge/overview.md", "/knowledge/req-cli.md"])
    );
    assert_eq!(findings, cold(&tree));

    // An edit to the body of an included document dirties the overview,
    // which includes it. The warm run and the cold run both report that
    // the overview's include is stale.
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text.replacen("> Enables persistent", "> Enables durable", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert!(dirty.contains("/knowledge/overview.md"), "{dirty:?}");
    assert!(!dirty.contains("/knowledge/decisions/decision-single-binary.md"));
    assert!(
        findings.iter().any(|f| f.contains("is stale")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));

    // A write that changes nothing dirties no document.
    let text = tree.read(rel);
    edit(&tree, rel, &text);
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(findings, cold(&tree));

    // A schema edit dirties the schema and the documents of its type, and
    // no other document.
    let rel = "knowledge/schemas/decision.md";
    let text = tree.read(rel);
    edit(&tree, rel, &text.replacen("hard: 8", "hard: 1", 1));
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(
        dirty,
        set(&[
            "/knowledge/decisions/decision-single-binary.md",
            "/knowledge/schemas/decision.md"
        ])
    );
    assert!(
        findings
            .iter()
            .any(|f| f.contains("decision-single-binary")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));

    // An edit to the formats file that adds a type that no schema uses
    // dirties no document.
    let formats = crate::common::requirement_formats();
    edit(
        &tree,
        ".sokf/formats.yaml",
        &formats.replacen("types:\n", "types:\n  extra: '[a-z]+'\n", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(findings, cold(&tree));
    // An edit that redefines a type that the `requirement-ears` format uses changes
    // that format. The requirements are dirty, because their schema uses
    // `requirement-ears`. Every schema document is dirty too, because the check of a
    // schema document reads the examples of the whole registry. No other
    // document is dirty.
    edit(
        &tree,
        ".sokf/formats.yaml",
        &formats.replacen("_AC-\\d+'", "_AC-\\d{1,4}'", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(
        dirty,
        set(&[
            "/knowledge/req-cli.md",
            "/knowledge/schemas/decision.md",
            "/knowledge/schemas/issue.md",
            "/knowledge/schemas/overview.md",
            "/knowledge/schemas/readme.md",
            "/knowledge/schemas/requirement.md",
        ])
    );
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-1_AC-26
// sokf-tests: CACHE_P-1
#[test]
fn an_included_source_change_checks_its_includer_with_no_read_of_it() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // A line added inside the region that the readme includes makes the
    // include stale, as the cold run reports, and dirties the readme alone.
    let text = tree.read("src/main.rs");
    edit(
        &tree,
        "src/main.rs",
        &text.replacen("fn cli()", "// changed\nfn cli()", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(dirty, set(&["/README.md"]));
    assert!(findings.iter().any(|f| f.contains("stale")), "{findings:?}");
    assert_eq!(findings, cold(&tree));
    // The line is taken out again. A run whose hint lists only the source
    // file dirties the readme and no other document.
    edit(&tree, "src/main.rs", &text);
    let (dirty, findings, _) = run_hinted(&tree, &["/src/main.rs"]);
    assert_eq!(dirty, set(&["/README.md"]));
    assert!(findings.is_empty(), "{findings:?}");
    assert_eq!(findings, cold(&tree));
    // When nothing changes, no reader is dirty.
    let (dirty, _, _, _) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    // When the source file is removed, the include points to a missing
    // file, and the warm run reports what the cold run reports.
    std::fs::remove_file(tree.dir().join("src/main.rs")).unwrap();
    let (dirty, _, findings, _) = run(&tree);
    assert_eq!(dirty, set(&["/README.md"]));
    assert!(!findings.is_empty(), "{findings:?}");
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-1_AC-20
#[test]
fn an_additional_file_is_checked_when_it_or_its_glob_changes() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // An edit to the readme dirties the readme alone, and its finding
    // matches the cold run's.
    let text = tree.read("README.md");
    edit(&tree, "README.md", &format!("{text}\nA line more.\n"));
    let (dirty, full, findings, read) = run(&tree);
    assert!(!full);
    assert_eq!(dirty, set(&["/README.md"]));
    assert!(
        findings.iter().any(|f| f.starts_with("/README.md")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    // The run reads the readme, the files that its schema reads, and the
    // source file that its include refers to.
    assert!(read <= 9, "read {read} files");
    edit(&tree, "README.md", &text);
    let (dirty, _, findings, _) = run(&tree);
    assert_eq!(dirty, set(&["/README.md"]));
    assert!(findings.is_empty(), "{findings:?}");
    // When the glob no longer matches the readme, no schema governs it, so
    // the run no longer checks it and drops its stored output. When the
    // glob is restored, the run checks the readme again.
    let schema = "knowledge/schemas/readme.md";
    let yaml = tree.read(schema);
    edit(
        &tree,
        schema,
        &yaml.replacen(
            "additional-files: /README.md",
            "additional-files: /NOPE.md",
            1,
        ),
    );
    edit(&tree, "README.md", &format!("{text}\nA line more.\n"));
    let (dirty, _, findings, _) = run(&tree);
    assert!(!dirty.contains("/README.md"), "{dirty:?}");
    assert!(
        !findings.iter().any(|f| f.starts_with("/README.md")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    edit(&tree, schema, &yaml);
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/README.md"), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.starts_with("/README.md")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-1_AC-18
// sokf-tests: CACHE-1_AC-19
#[test]
fn a_change_touches_its_directory_and_the_other_indexes_stand() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // An index edited by hand affects only its own directory: the run
    // reports one finding, that the index is stale, and no document is
    // dirty.
    let rel = "knowledge/decisions/index.md";
    let text = tree.read(rel);
    edit(&tree, rel, &format!("{text}\nnot generated\n"));
    let (dirty, full, findings, read) = run(&tree);
    assert!(!full);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].starts_with("/knowledge/decisions/index.md"));
    assert_eq!(findings, cold(&tree));
    assert!(read <= 8, "read {read} files");
    // After the index is restored by hand, the run no longer reports the
    // finding, and still no document is dirty.
    edit(&tree, rel, &text);
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert!(findings.is_empty(), "{findings:?}");
    // A document in a new directory dirties that document alone. The new
    // directory needs an index, and the listing of its parent changes, but
    // the indexes of the other directories do not change.
    edit(
        &tree,
        "knowledge/deep/choice.md",
        "---\ntype: Decision\nid: choice\ntitle: Choice\ndescription: Made up.\n---\n\n# Choice\n\n## Context\n\nSome.\n\n## Decision\n\nSome.\n\n## Consequences\n\n- Some.\n",
    );
    let (dirty, _, findings, read) = run(&tree);
    assert_eq!(dirty, set(&["/knowledge/deep/choice.md"]));
    let paths: Vec<&str> = findings
        .iter()
        .map(|f| f.split(':').next().unwrap())
        .collect();
    assert_eq!(
        paths,
        vec!["/knowledge/deep/index.md", "/knowledge/index.md"],
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    // The run reads the new document, the files that its schema reads, and
    // the parent's index.
    assert!(read <= 10, "read {read} files");
    // After the directory and its document are removed, the findings
    // disappear, and the next run reports nothing about the removed
    // directory.
    fs::remove_dir_all(tree.dir().join("knowledge/deep")).unwrap();
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert!(findings.is_empty(), "{findings:?}");
    let (_, _, findings, _) = run(&tree);
    assert!(findings.is_empty(), "{findings:?}");
}

// sokf-tests: CACHE-1_AC-16
#[test]
fn a_store_rename_dirties_the_documents_that_name_the_store() {
    // The tree holds the clean fixture, `test/fixture`, and a second store,
    // `test/other`, whose note cites the requirement through the name of
    // the first store.
    let tree = TempTree::fixture("clean");
    tree.add_other_store();
    tree.write(
        "other/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\ntitle: Note Schema\ndescription: A note.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nlinks:\n  - rel: references\n    min: 0\nsections:\n  - heading: Note\n    level: 1\n    blocks:\n      - kind: prose\n        repeat: true\n        example: Milk, bread, and the good coffee.\n````\n",
    );
    tree.write(
        "other/note.md",
        "---\ntype: Note\nid: note\nlinks:\n  - rel: references\n    to: fixture:req-cli\n---\n\n# Note\n\nSee [the CLI][sokf:fixture:req-cli].\n\n<!-- sokf:links -->\n\n[sokf:fixture:req-cli]: /knowledge/req-cli.md\n",
    );
    tree.validate_all(true, CacheMode::Off);
    age(tree.dir(), 10);
    let (_, _, findings, _) = run(&tree);
    assert!(findings.is_empty(), "{findings:?}");
    // When the first store is renamed, the note's link resolves to
    // nothing, the heading of the store's index is stale, and the store's
    // documents stay clean.
    let rel = "knowledge/manifest.sokf.yaml";
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text.replacen("name: fixture", "name: fixtures", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(dirty, set(&["/other/note.md"]), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.starts_with("/other/note.md")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    // A `renamed-from` entry records the former identity, so the link
    // resolves again, and the run checks the note again.
    edit(
        &tree,
        rel,
        &text.replacen(
            "name: fixture",
            "name: fixtures\nrenamed-from:\n  - test/fixture",
            1,
        ),
    );
    let (dirty, _, findings, _) = run(&tree);
    assert_eq!(dirty, set(&["/other/note.md"]), "{dirty:?}");
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-1_AC-15
#[test]
fn a_schema_that_changes_its_type_dirties_both_types() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // When the decision schema declares another type, the decision loses
    // its schema. The decision and the schema are dirty, and no other
    // document is dirty.
    let rel = "knowledge/schemas/decision.md";
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text.replacen("const: Decision", "const: Choice", 1),
    );
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(
        dirty,
        set(&[
            "/knowledge/decisions/decision-single-binary.md",
            "/knowledge/schemas/decision.md"
        ])
    );
    assert!(
        findings.iter().any(|f| f.contains("has no schema")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    // When the edit is undone, the same two documents are dirty, and the
    // run no longer reports the finding.
    edit(&tree, rel, &text);
    let (dirty, _, findings, _) = run(&tree);
    assert_eq!(dirty.len(), 2, "{dirty:?}");
    assert!(findings.is_empty(), "{findings:?}");
}

// sokf-tests: CACHE-1_AC-4
#[test]
fn an_id_that_moves_dirties_the_documents_that_name_it() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // A change to the requirement's id breaks the links of the overview,
    // and the decision stays clean.
    let rel = "knowledge/req-cli.md";
    let text = tree.read(rel);
    edit(&tree, rel, &text.replacen("id: req-cli", "id: req-cmd", 1));
    let (dirty, full, findings, _) = run(&tree);
    assert!(!full);
    assert_eq!(
        dirty,
        set(&["/knowledge/overview.md", "/knowledge/req-cli.md"])
    );
    assert!(
        findings.iter().any(|f| f.contains("resolves to")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    // A new document with the overview's id dirties the overview.
    edit(
        &tree,
        "knowledge/twin.md",
        "---\ntype: Overview\nid: overview\ntitle: Twin\ndescription: A twin.\nlinks:\n  - rel: references\n    to: req-cmd\n---\n\n# Twin\n\nSee [it][sokf:req-cmd].\n\n<!-- sokf:links -->\n\n[sokf:req-cmd]: /knowledge/req-cli.md\n",
    );
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/overview.md"), "{dirty:?}");
    assert!(dirty.contains("/knowledge/twin.md"), "{dirty:?}");
    assert!(
        findings.iter().any(|f| f.contains("also uses id")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
    // Deleting the document with the overview's id changes an input of the
    // overview, so the overview is dirty again, and the finding that the id
    // is shared disappears.
    fs::remove_file(tree.dir().join("knowledge/twin.md")).unwrap();
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/knowledge/overview.md"), "{dirty:?}");
    assert!(
        !findings.iter().any(|f| f.contains("also uses id")),
        "{findings:?}"
    );
    assert_eq!(findings, cold(&tree));
}

/// `fix` validates the tree with fixing on and the cache on. It returns
/// the operations that it applied and the findings that remain.
fn fix(tree: &TempTree) -> (Vec<String>, Vec<String>) {
    let report = tree.validate_all(true, CacheMode::On);
    let ops: Vec<String> = report.fixes.iter().map(|f| format!("{f:?}")).collect();
    (ops, lines(&report))
}

// sokf-tests: CACHE_P-1
// sokf-tests: CACHE-1_AC-1
#[test]
fn a_fix_rewrites_the_documents_with_findings_and_the_readers_of_a_move() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // The overview cites the issue, and its definition block holds the
    // path of the issue. When the issue closes, one fix moves the issue and
    // rewrites the overview.
    let rel = "knowledge/overview.md";
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text
            .replacen(
                "governed by [its schema][sokf:schema-requirement].",
                "governed by [its schema][sokf:schema-requirement]; see [the issue][sokf:issue-001-retry].",
                1,
            )
            .replacen(
                "[sokf:req-cli#acceptance-criteria]: /knowledge/req-cli.md#acceptance-criteria\n",
                "[sokf:issue-001-retry]: /knowledge/issues/open/issue-001-retry.md\n[sokf:req-cli#acceptance-criteria]: /knowledge/req-cli.md#acceptance-criteria\n",
                1,
            ),
    );
    let (_, _, findings, _) = run(&tree);
    assert!(findings.is_empty(), "{findings:?}");
    let issue = "knowledge/issues/open/issue-001-retry.md";
    let text = tree.read(issue);
    edit(
        &tree,
        issue,
        &text.replacen("state: open", "state: closed", 1),
    );
    let (ops, findings) = fix(&tree);
    assert!(ops.iter().any(|o| o.contains("Moved")), "{ops:?}");
    assert!(
        ops.iter()
            .any(|o| o.contains("Rewrote") && o.contains("overview.md")),
        "{ops:?}"
    );
    // A closed issue needs a conclusion, and the missing conclusion is the
    // one finding left in both reports.
    assert_eq!(findings, cold(&tree));
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("Conclusion"), "{findings:?}");
    assert!(
        tree.dir()
            .join("knowledge/issues/closed/issue-001-retry.md")
            .exists()
    );
    assert!(
        tree.read(rel)
            .contains("/knowledge/issues/closed/issue-001-retry.md")
    );
    // A fragment written as a key's text is rewritten as its slug, and
    // nothing else is touched.
    let text = tree.read(rel);
    edit(
        &tree,
        rel,
        &text.replacen("[sokf:#cli-1]", "[sokf:#CLI-1]", 1),
    );
    let (ops, findings) = fix(&tree);
    assert_eq!(ops.len(), 1, "{ops:?}");
    assert!(ops[0].contains("overview.md"), "{ops:?}");
    assert_eq!(findings, cold(&tree));
    assert!(tree.read(rel).contains("[sokf:#cli-1]"));
    // When a document is written again unchanged, its modification time is
    // within 2 seconds of the last sync, so the run reads it and holds it in
    // memory. The fix also reads the documents that its links point to, and
    // rewrites nothing.
    let text = tree.read(rel);
    tree.write(rel, &text);
    let (ops, findings) = fix(&tree);
    assert!(ops.is_empty(), "{ops:?}");
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-3_AC-2
// sokf-tests: CACHE_P-1
#[test]
fn a_fix_plans_over_the_held_documents_and_not_over_what_they_read() {
    // The executor links to the planner, which links to the configuration.
    let tree = TempTree::fixture("graph");
    age(tree.dir(), 10);
    run(&tree);
    fix(&tree);
    // The executor is written again unchanged, so its modification time is
    // within 2 seconds of the last sync, and the run reads it and holds it in
    // memory. To plan its rewrites, the fix reads the planner for the
    // executor and leaves the documents that the planner links to unread.
    // The plan therefore covers the executor alone. The fix rewrites nothing.
    let rel = "knowledge/core/executor.md";
    let text = tree.read(rel);
    tree.write(rel, &text);
    let (ops, findings) = fix(&tree);
    assert!(ops.is_empty(), "{ops:?}");
    assert_eq!(findings, cold(&tree));
}
