//! These tests run `sokf new` through the core, over a copy of the `new`
//! fixture in a temporary directory, and cover every option, every refusal,
//! and `--write`.

use std::path::PathBuf;

use common::{TempTree, json, new_options};
use sokf_core::{CacheMode, NewResult, new};

use crate::common;

fn states(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

pub(crate) fn issue(
    tree: &TempTree,
    slug: &str,
    pairs: &[(&str, &str)],
) -> sokf_core::Result<NewResult> {
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some(slug.into());
    o.states = states(pairs);
    new(&tree.root(), &o)
}

pub(crate) fn err(r: sokf_core::Result<NewResult>) -> String {
    match r {
        Ok(r) => panic!("expected a refusal, got {}", json_result(&r)),
        Err(e) => e.to_string(),
    }
}

fn json_result(r: &NewResult) -> String {
    serde_json::to_string(r).unwrap()
}

// sokf-tests: CLI-6_AC-2
// sokf-tests: CLI-6_AC-7
// sokf-tests: CLI-6_AC-11
// sokf-tests: SOKF_SCHEMA_26
// sokf-tests: CLI-6_AC-1
// sokf-tests: CLI-6_AC-3
// sokf-tests: CLI-6_AC-4
// sokf-tests: CLI-6_AC-5
// sokf-tests: CLI-6_AC-6
#[test]
fn each_state_combination_is_seeded_gated_filled_and_placed() {
    let tree = TempTree::fixture("new");
    // An open bug keeps its Reproduction section.
    let r = issue(&tree, "retry-logic", &[]).unwrap();
    assert_eq!(r.id, "issue-002-retry-logic");
    assert_eq!(
        r.path.as_str(),
        "/knowledge/issues/open/bug/issue-002-retry-logic.md"
    );
    assert_eq!(
        r.document,
        "---\ntype: Issue\nid: issue-002-retry-logic\ntitle: Retry logic\nstate: open\nkind: bug\nlinks:\n  # sokf:guidance The decision the issue delivers.\n  - rel: implements\n    to: target-id\n---\n\n<!-- sokf:guidance The writer replaces each of the following examples with their own text. -->\n\n# Retry logic\n\nThe index writer drops the entry of a moved file.\n\n<!-- sokf:guidance Steps that show the bug. -->\n## Reproduction\n\n1. Move a document with `--fix`.\n"
    );
    assert!(!r.created);
    // An open feature drops its Reproduction section.
    let r = issue(&tree, "dark-mode", &[("kind", "feature")]).unwrap();
    assert_eq!(
        r.path.as_str(),
        "/knowledge/issues/open/feature/issue-002-dark-mode.md"
    );
    assert!(!r.document.contains("Reproduction"), "{}", r.document);
    assert!(r.document.contains("kind: feature\n"));
    // `sokf new` creates a closed bug with the requested states.
    let r = issue(&tree, "old-bug", &[("state", "closed")]).unwrap();
    assert_eq!(
        r.path.as_str(),
        "/knowledge/issues/closed/bug/issue-002-old-bug.md"
    );
    assert!(r.document.starts_with(
        "---\ntype: Issue\nid: issue-002-old-bug\ntitle: Old bug\nstate: closed\nkind: bug\n"
    ));
    assert!(r.document.contains("# Old bug\n"));
    // A closed feature gains a Conclusion section.
    let r = issue(&tree, "done", &[("state", "closed"), ("kind", "feature")]).unwrap();
    assert_eq!(
        r.document,
        "---\ntype: Issue\nid: issue-002-done\ntitle: Done\nstate: closed\nkind: feature\nlinks:\n  # sokf:guidance The decision the issue delivers.\n  - rel: implements\n    to: target-id\n---\n\n<!-- sokf:guidance The writer replaces each of the following examples with their own text. -->\n\n# Done\n\nThe index writer drops the entry of a moved file.\n\n<!-- sokf:guidance What was done. -->\n## Conclusion\n\nThe move now rewrites both indexes it touches.\n"
    );
}

// sokf-tests: CLI-6_AC-9
// sokf-tests: CLI-6_AC-10
// sokf-tests: CLI-6_AC-20
// sokf-tests: CLI_P-24
#[test]
fn a_written_document_validates_but_for_its_placeholder_references() {
    let tree = TempTree::fixture("new");
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("retry-logic".into());
    o.write = true;
    let r = new(&tree.root(), &o).unwrap();
    assert!(r.created);
    assert_eq!(
        tree.read("knowledge/issues/open/bug/issue-002-retry-logic.md"),
        r.document
    );
    // The index stays stale until validation fixes it. After the author reads
    // and deletes the guidance, only the placeholder reference fails.
    tree.drop_guidance("knowledge/issues/open/bug/issue-002-retry-logic.md");
    let report = tree.validate(true, CacheMode::Off);
    let own: Vec<&str> = report
        .errors
        .iter()
        .filter(|e| e.path == r.path)
        .map(|e| e.message.as_str())
        .collect();
    assert!(!own.is_empty(), "{}", json(&report));
    assert!(own.iter().all(|m| m.contains("target-id")), "{own:?}");
    // When a type has no sequence number, `sokf new` refuses to write the same
    // document twice, because the file already exists.
    let mut note = new_options(tree.dir(), "Note");
    note.id = Some("note-twice".into());
    note.title = Some("Twice".into());
    note.write = true;
    assert!(new(&tree.root(), &note).unwrap().created);
    let e = err(new(&tree.root(), &note));
    assert!(e.contains("exists, so sokf writes nothing"), "{e}");
    // A note links nowhere, so with its guidance gone it validates clean.
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("note-clean".into());
    o.title = Some("Clean".into());
    o.write = true;
    let r = new(&tree.root(), &o).unwrap();
    tree.drop_guidance("knowledge/note-clean.md");
    let report = tree.validate(true, CacheMode::Off);
    assert!(
        report.errors.iter().all(|e| e.path != r.path),
        "{}",
        json(&report)
    );
}

// sokf-tests: CLI-6_AC-19
// sokf-tests: CLI-6_AC-38
#[test]
fn state_refusals() {
    let tree = TempTree::fixture("new");
    for (pairs, needle) in [
        (states(&[("phase", "a")]), "declares no `phase`"),
        (states(&[("state", "done")]), "not a state of `state`"),
        (
            states(&[("state", "open"), ("state", "closed")]),
            "given twice",
        ),
    ] {
        let mut o = new_options(tree.dir(), "Issue");
        o.slug = Some("x".into());
        o.states = pairs;
        let e = err(new(&tree.root(), &o));
        assert!(e.starts_with("new: "), "{e}");
        assert!(e.contains(needle), "{e}");
    }
    // A type with no lifecycle accepts no state.
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("note-x".into());
    o.states = states(&[("state", "open")]);
    assert!(err(new(&tree.root(), &o)).contains("declares no `state`"));
}

// sokf-tests: CLI-6_AC-18
// sokf-tests: CLI-6_AC-17
// sokf-tests: CLI-6_AC-28
// sokf-tests: CLI-6_AC-29
// sokf-tests: CLI-6_AC-34
// sokf-tests: CLI-6_AC-39
// sokf-tests: CLI-6_AC-40
// sokf-tests: CLI-6_AC-42
// sokf-tests: CLI-6_AC-49
#[test]
fn a_type_with_no_template_takes_an_id_and_a_dir() {
    let tree = TempTree::fixture("new");
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("note-retry".into());
    o.title = Some("Retry".into());
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(r.id, "note-retry");
    assert_eq!(r.path.as_str(), "/knowledge/note-retry.md");
    assert_eq!(
        r.document,
        "---\ntype: Note\nid: note-retry\ntitle: Retry\n---\n\n<!-- sokf:guidance The writer replaces each of the following examples with their own text. -->\n\n# Retry\n\nMilk, bread, and the good coffee.\n"
    );
    // `--dir` accepts a path with or without a leading slash.
    for dir in ["knowledge/notes", "/knowledge/notes"] {
        let mut o = o.clone();
        o.dir = Some(PathBuf::from(dir));
        let r = new(&tree.root(), &o).unwrap();
        assert_eq!(r.path.as_str(), "/knowledge/notes/note-retry.md");
    }
    // The schema sets a pattern for the title, so a title is needed, and
    // the one that `--title` gives must match it.
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("note-retry-logic".into());
    let e = err(new(&tree.root(), &o));
    assert!(e.contains("`--title \"Groceries\"`"), "{e}");
    o.title = Some("Retry logic".into());
    let e = err(new(&tree.root(), &o));
    assert!(
        e.contains("doesn't match the schema") && e.contains("^[A-Z][a-z]+$"),
        "{e}"
    );
    for (o, needle) in [
        (
            {
                let mut o = new_options(tree.dir(), "Note");
                o.id = Some("n".into());
                o.dir = Some(PathBuf::from("src"));
                o
            },
            "outside the knowledge",
        ),
        (
            {
                let mut o = new_options(tree.dir(), "Note");
                o.id = Some("n".into());
                o.dir = Some(PathBuf::from("../elsewhere"));
                o
            },
            "outside the repository",
        ),
        (
            {
                let mut o = new_options(tree.dir(), "Note");
                o.slug = Some("n".into());
                o
            },
            "has no id template, so use `--id`",
        ),
        (new_options(tree.dir(), "Note"), "`--id` is required"),
        (
            {
                let mut o = new_options(tree.dir(), "Note");
                o.id = Some("Not A Slug".into());
                o
            },
            "is not a slug",
        ),
        (
            {
                let mut o = new_options(tree.dir(), "Issue");
                o.id = Some("issue-009-x".into());
                o
            },
            "has an id template, so use `--slug`",
        ),
        (
            {
                let mut o = new_options(tree.dir(), "Issue");
                o.slug = Some("x".into());
                o.dir = Some(PathBuf::from("knowledge/issues"));
                o
            },
            "the requested states fix the directory of the document",
        ),
        (new_options(tree.dir(), "Nope"), "has no schema"),
    ] {
        let e = err(new(&tree.root(), &o));
        assert!(e.contains(needle), "{e}");
    }
}

// sokf-tests: CLI-6_AC-6
// sokf-tests: CLI-6_AC-27
// sokf-tests: CLI-6_AC-30
// sokf-tests: CLI-6_AC-41
#[test]
fn a_schema_is_created_from_the_grammar_template() {
    let tree = TempTree::fixture("new");
    let mut o = new_options(tree.dir(), "Schema");
    o.id = Some("schema-task".into());
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(r.path.as_str(), "/knowledge/schemas/schema-task.md");
    assert!(
        r.document
            .starts_with("---\ntype: Schema\nid: schema-task\ntitle: Schema task\n"),
        "{}",
        r.document
    );
    assert!(r.document.contains("````yaml\n"), "{}", r.document);
    // With two roots, the author must give the schemas directory with `--dir`.
    tree.add_other_store();
    tree.write("other/schemas/index.md", "# schemas\n");
    tree.write("other/index.md", "# other\n");
    let e = err(new(&tree.root(), &o));
    assert!(
        e.contains("more than one root") && e.contains("/other/schemas"),
        "{e}"
    );
    let mut with = o.clone();
    with.dir = Some(PathBuf::from("other/schemas"));
    let r = new(&tree.root(), &with).unwrap();
    assert_eq!(r.path.as_str(), "/other/schemas/schema-task.md");
    with.dir = Some(PathBuf::from("other"));
    let e = err(new(&tree.root(), &with));
    assert!(e.contains("isn't the `schemas/` directory"), "{e}");
}

// sokf-tests: CLI-6_AC-14
// sokf-tests: CLI-6_AC-22
// sokf-tests: CLI-6_AC-43
// sokf-tests: CLI-6_AC-44
// sokf-tests: CLI-6_AC-45
// sokf-tests: CLI-6_AC-46
#[test]
fn a_sidecar_is_created_beside_its_asset() {
    let tree = TempTree::fixture("new");
    let mut o = new_options(tree.dir(), "Image");
    o.id = Some("flowers".into());
    o.for_asset = Some(PathBuf::from("knowledge/media/draft/flowers.png"));
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(r.path.as_str(), "/knowledge/media/draft/flowers.png.md");
    assert_eq!(
        r.document,
        "---\ntype: Image\nid: flowers\ntitle: Flowers\nresource: /knowledge/media/draft/flowers.png\nstate: draft\n---\n\n<!-- sokf:guidance The writer replaces each of the following examples with their own text. -->\n\n<!-- sokf:guidance One line under the picture. -->\nFlowers on the kitchen table.\n"
    );
    // After the sidecar is written, its guidance deleted, and the index fixed,
    // the asset and its sidecar validate clean.
    o.write = true;
    new(&tree.root(), &o).unwrap();
    tree.drop_guidance("knowledge/media/draft/flowers.png.md");
    let report = tree.validate(true, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|e| !e.path.as_str().contains("flowers")),
        "{}",
        json(&report)
    );
    tree.write("knowledge/media/draft/sunset.png", "");
    tree.write(
        "knowledge/media/draft/sunset.png.md",
        "---\ntype: Image\nid: sunset\ntitle: Sunset\nstate: draft\nresource: /knowledge/media/draft/sunset.png\n---\n\nA sunset.\n",
    );
    for (path, needle) in [
        ("README.md", "is not a path in the repository"),
        ("knowledge/notes/note-existing.md", "is not an asset"),
        ("knowledge/media/draft/sunset.png", "already has a sidecar"),
        (
            "knowledge/media/published/moved.png",
            "but the requested states put sidecars in /knowledge/media/draft",
        ),
    ] {
        let mut o = new_options(tree.dir(), "Image");
        o.id = Some("x".into());
        o.for_asset = Some(PathBuf::from(path));
        let e = err(new(&tree.root(), &o));
        assert!(e.contains(needle), "{path}: {e}");
    }
    // `sokf new` accepts the misplaced asset when the requested states match its
    // directory.
    let mut o = new_options(tree.dir(), "Image");
    o.id = Some("moved".into());
    o.for_asset = Some(PathBuf::from("knowledge/media/published/moved.png"));
    o.states = states(&[("state", "published")]);
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(r.path.as_str(), "/knowledge/media/published/moved.png.md");
    assert!(r.document.contains("state: published\n"));
    // `sokf new` requires `--for` for a sidecar type and refuses it for any other type.
    let mut o = new_options(tree.dir(), "Image");
    o.id = Some("x".into());
    assert!(err(new(&tree.root(), &o)).contains("so give `--for <PATH>` with the asset"));
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("x".into());
    o.for_asset = Some(PathBuf::from("knowledge/media/draft/flowers.png"));
    assert!(err(new(&tree.root(), &o)).contains("declares no `asset-files`"));
}

// sokf-tests: CLI-6_AC-12
// sokf-tests: CLI-6_AC-13
// sokf-tests: CLI-6_AC-50
#[test]
fn the_json_object_and_the_id_alone() {
    let tree = TempTree::fixture("new");
    let r = issue(&tree, "x", &[]).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json_result(&r)).unwrap();
    let keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["id", "path", "document"]);
    assert_eq!(value["path"], "/knowledge/issues/open/bug/issue-002-x.md");
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("x".into());
    o.id_only = true;
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(r.id, "issue-002-x");
    assert_eq!(r.document, "");
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("n".into());
    o.id_only = true;
    assert!(err(new(&tree.root(), &o)).contains("so give the id with `--id`"));
}

// sokf-tests: CLI-6_AC-15
// sokf-tests: CLI-6_AC-31
// sokf-tests: CLI-6_AC-32
#[test]
fn the_default_slug_comes_from_the_assets_filename() {
    let tree = TempTree::fixture("new");
    let schema = tree.read("knowledge/schemas/image.md").replace(
        "  resource:\n",
        "  id:\n    template: \"img-${:slug}\"\n  resource:\n",
    );
    tree.write("knowledge/schemas/image.md", &schema);
    tree.write("knowledge/media/draft/Wild Flowers 2.png", "");
    let mut o = new_options(tree.dir(), "Image");
    o.for_asset = Some(PathBuf::from("knowledge/media/draft/Wild Flowers 2.png"));
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(r.id, "img-wild-flowers-2");
    assert_eq!(
        r.path.as_str(),
        "/knowledge/media/draft/Wild Flowers 2.png.md"
    );
    assert!(
        r.document.contains("title: Wild flowers 2\n"),
        "{}",
        r.document
    );
    // `--slug` overrides the stem. A stem with no letters needs it.
    o.slug = Some("bloom".into());
    assert_eq!(new(&tree.root(), &o).unwrap().id, "img-bloom");
    tree.write("knowledge/media/draft/---.png", "");
    let mut o = new_options(tree.dir(), "Image");
    o.for_asset = Some(PathBuf::from("knowledge/media/draft/---.png"));
    assert!(err(new(&tree.root(), &o)).contains("can't be made into a slug"));
}

// sokf-tests: CLI-6_AC-36
#[test]
fn a_bound_paragraph_follows_the_filled_title() {
    let tree = TempTree::fixture("new");
    let schema = tree.read("knowledge/schemas/note.md");
    tree.write(
        "knowledge/schemas/note.md",
        &schema.replace(
            "        max: 3\n        example: Milk, bread, and the good coffee.\n",
            "        max: 3\n        each: { text: { from: title } }\n",
        ),
    );
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("note-retry".into());
    o.title = Some("Retry".into());
    let r = new(&tree.root(), &o).unwrap();
    assert!(r.document.ends_with("# Retry\n\nRetry\n"), "{}", r.document);
}

// sokf-tests: CLI-6_AC-23
#[test]
fn a_lifecycle_key_in_the_id_template_is_filled_and_checked() {
    let tree = TempTree::fixture("new");
    let schema = tree.read("knowledge/schemas/issue.md");
    tree.write(
        "knowledge/schemas/issue.md",
        &schema.replace("issue-${:seq}-${:slug}", "issue-${:seq}-${kind}-${:slug}"),
    );
    // The existing issues take the new template too: `issue-<n>-<kind>-<slug>`.
    for (rel, text) in tree.files() {
        let field = |key: &str| {
            text.split(&format!("\n{key}: "))
                .nth(1)
                .and_then(|rest| rest.lines().next())
                .map(str::to_string)
        };
        if !rel.starts_with("knowledge/issues/") {
            continue;
        }
        let (Some(id), Some(kind)) = (field("id"), field("kind")) else {
            continue;
        };
        let (num, slug) = id.trim_start_matches("issue-").split_once('-').unwrap();
        tree.write(
            &rel,
            &text.replace(&id, &format!("issue-{num}-{kind}-{slug}")),
        );
    }
    let r = issue(&tree, "dark-mode", &[("kind", "feature")]).unwrap();
    assert!(r.id.ends_with("-feature-dark-mode"), "{}", r.id);
    let r = issue(&tree, "retry-logic", &[("kind", "bug")]).unwrap();
    assert!(r.id.ends_with("-bug-retry-logic"), "{}", r.id);
    // Validation reports a document whose id contains a kind that differs from
    // its `kind` field.
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("retry-logic".into());
    o.states = states(&[("kind", "bug")]);
    o.write = true;
    let r = new(&tree.root(), &o).unwrap();
    let path = r.path.to_string().trim_start_matches('/').to_string();
    let text = tree.read(&path);
    tree.write(&path, &text.replace("\nkind: bug\n", "\nkind: feature\n"));
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report.errors.iter().any(|e| e
            .message
            .contains("holds `bug` for `kind`, but `kind` is `feature`")),
        "{}",
        json(&report)
    );
}
