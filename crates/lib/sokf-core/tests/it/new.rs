//! These tests run `sokf new` through the core, over a copy of the `new`
//! fixture in a temporary directory, and cover every option, every refusal,
//! and `--write`.

use std::path::PathBuf;

use common::{TempTree, examples, json, new_options, not_missing};
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
// sokf-tests: CLI-6_AC-126
#[test]
fn each_state_combination_is_seeded_gated_filled_and_placed() {
    let tree = TempTree::fixture("new");
    // An open bug keeps its Reproduction section, as a heading alone: the
    // skeleton holds no example and no guidance, and the guide holds the
    // example of each part.
    let r = issue(&tree, "retry-logic", &[]).unwrap();
    assert_eq!(r.id, "issue-002-retry-logic");
    assert_eq!(
        r.path.as_str(),
        "/knowledge/issues/open/bug/issue-002-retry-logic.md"
    );
    assert_eq!(
        r.document,
        "---\ntype: Issue\nid: issue-002-retry-logic\ntitle: Retry logic\nstate: open\nkind: bug\n---\n\n# Retry logic\n\n## Reproduction\n"
    );
    let all = examples(&r).join("\n");
    assert!(
        all.contains("The index writer drops the entry of a moved file.")
            && all.contains("1. Move a document with `--fix`."),
        "{all}"
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
        "---\ntype: Issue\nid: issue-002-done\ntitle: Done\nstate: closed\nkind: feature\n---\n\n# Done\n\n## Conclusion\n"
    );
    assert!(
        examples(&r)
            .iter()
            .any(|e| e == "The move now rewrites both indexes it touches."),
        "{:?}",
        examples(&r)
    );
}

// sokf-tests: CLI-6_AC-9
// sokf-tests: CLI-6_AC-10
// sokf-tests: CLI-6_AC-20
// sokf-tests: CLI_P-24
// sokf-tests: CLI-6_AC-126
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
    // The untouched skeleton reports each part that its writer has still to
    // write as missing, and nothing else.
    let report = tree.validate(true, CacheMode::Off);
    let own = report.errors.iter().filter(|e| e.path == r.path).count();
    assert!(own > 0, "{}", json(&report));
    assert_eq!(
        not_missing(&report, r.path.as_str()),
        Vec::<String>::new(),
        "{}",
        json(&report)
    );
    // When a type has no sequence number, `sokf new` refuses to write the same
    // document twice, because the file already exists.
    let mut note = new_options(tree.dir(), "Note");
    note.id = Some("note-twice".into());
    note.title = Some("Twice".into());
    note.write = true;
    assert!(new(&tree.root(), &note).unwrap().created);
    let e = err(new(&tree.root(), &note));
    assert!(e.contains("exists, so sokf writes nothing"), "{e}");
    // A note's skeleton lacks only its paragraph, and with it written the
    // note validates clean.
    let mut o = new_options(tree.dir(), "Note");
    o.id = Some("note-clean".into());
    o.title = Some("Clean".into());
    o.write = true;
    let r = new(&tree.root(), &o).unwrap();
    let report = tree.validate(true, CacheMode::Off);
    let own: Vec<&str> = report
        .errors
        .iter()
        .filter(|e| e.path == r.path)
        .map(|e| e.criterion.as_str())
        .collect();
    assert_eq!(own, ["TLMD_152"], "{}", json(&report));
    tree.write(
        "knowledge/note-clean.md",
        &(r.document.clone() + "\nEggs and the dark bread.\n"),
    );
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
        "---\ntype: Note\nid: note-retry\ntitle: Retry\n---\n\n# Retry\n"
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
    // The skeleton of a schema holds its frontmatter alone, and the guide
    // gives the fence of the schema as the example of the preamble.
    assert!(!r.document.contains("```"), "{}", r.document);
    let preamble = r.guide.as_ref().and_then(|g| g.preamble.as_ref());
    assert!(
        preamble
            .and_then(|p| p.example.as_deref())
            .is_some_and(|e| e.starts_with("````yaml\n")),
        "{preamble:?}"
    );
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
        "---\ntype: Image\nid: flowers\ntitle: Flowers\nresource: /knowledge/media/draft/flowers.png\nstate: draft\n---\n"
    );
    // After the sidecar is written, its line under the picture written, and
    // the index fixed, the asset and its sidecar validate clean.
    o.write = true;
    new(&tree.root(), &o).unwrap();
    tree.write(
        "knowledge/media/draft/flowers.png.md",
        &(r.document.clone() + "\nFlowers on the kitchen table.\n"),
    );
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
// sokf-tests: CLI-6_AC-130
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
    assert_eq!(keys, ["id", "path", "document", "guide"]);
    assert_eq!(value["path"], "/knowledge/issues/open/bug/issue-002-x.md");
    // The object holds the guide of the new document, as `sokf guide --json`
    // gives it for the document.
    assert_eq!(value["guide"]["path"], value["path"]);
    assert_eq!(value["guide"]["id"], "issue-002-x");
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
            "        max: { hard: 3, when-at-limit: \"Start another note for the rest.\" }\n        example: Milk, bread, and the good coffee.\n",
            "        max: { hard: 3, when-at-limit: \"Start another note for the rest.\" }\n        each: { text: { from: title } }\n",
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

/// `BOUND_SCHEMA` binds the paragraph of its title section to the title, and
/// requires one `part-of` link.
const BOUND_SCHEMA: &str = "---\ntype: Schema\nid: schema-boundp\ntitle: Bound Schema\ndescription: A paragraph bound to the title, and a required link.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Boundp\n  id:\n    template: \"boundp-${:slug}\"\nlinks:\n  - rel: part-of\n    guidance: The note that this one belongs to.\n  - rel: relates-to\n    min: 0\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        each:\n          text: { from: title }\n````\n";

// A skeleton writes `links` empty where a rule requires a link, and writes no
// entry, so the check reports the link as missing, and the guide gives the
// example entry of each rule.
// sokf-tests: CLI-6_AC-128
#[test]
fn a_required_link_leaves_links_empty() {
    let tree = TempTree::fixture("new");
    tree.write("knowledge/schemas/boundp.md", BOUND_SCHEMA);
    let mut o = new_options(tree.dir(), "Boundp");
    o.slug = Some("milk".into());
    o.title = Some("Milk".into());
    o.write = true;
    let r = new(&tree.root(), &o).unwrap();
    assert_eq!(
        r.document,
        "---\ntype: Boundp\nid: boundp-milk\ntitle: Milk\nlinks:\n---\n\n# Milk\n\nMilk\n"
    );
    let guide = r.guide.as_ref().unwrap();
    assert_eq!(
        guide.links[0].example,
        "links:\n  - rel: part-of\n    to: target-id"
    );
    let report = tree.validate(true, CacheMode::Off);
    let own: Vec<&str> = report
        .errors
        .iter()
        .filter(|e| e.path == r.path)
        .map(|e| e.criterion.as_str())
        .collect();
    assert_eq!(own, ["SOKF_SCHEMA_33"], "{}", json(&report));
    // Without a required link, the skeleton holds no `links` at all.
    tree.write(
        "knowledge/schemas/boundp.md",
        &BOUND_SCHEMA.replace(
            "  - rel: part-of\n    guidance: The note that this one belongs to.\n",
            "",
        ),
    );
    o.slug = Some("bread".into());
    o.write = false;
    let r = new(&tree.root(), &o).unwrap();
    assert!(!r.document.contains("links:"), "{}", r.document);
}

// A title that the paragraph bound to it can't hold leaves no document of
// the schema that passes, so `sokf new` refuses it, names the rule, and
// writes nothing.
// sokf-tests: CLI-6_AC-131
#[test]
fn a_title_that_no_bound_unit_holds_is_refused() {
    let tree = TempTree::fixture("new");
    tree.write("knowledge/schemas/boundp.md", BOUND_SCHEMA);
    let before = tree.files();
    let mut o = new_options(tree.dir(), "Boundp");
    o.slug = Some("milk".into());
    o.title = Some("<div>".into());
    o.write = true;
    let e = err(new(&tree.root(), &o));
    assert!(
        e.starts_with("new: the new document fails the schema of `Boundp`")
            && e.contains("/knowledge/schemas/boundp.md:")
            && e.contains("`<div>`"),
        "{e}"
    );
    assert_eq!(tree.files(), before);
}

// A new Schema document holds its frontmatter alone, and its check reports
// the description and the fence that its writer writes, each once: the
// missing fence is a missing block of the preamble, and no second finding
// says that the document has no schema.
// sokf-tests: CLI_P-24
#[test]
fn a_new_schema_lacks_its_description_and_its_fence_once() {
    let tree = TempTree::fixture("new");
    let mut o = new_options(tree.dir(), "Schema");
    o.id = Some("schema-task".into());
    o.write = true;
    let r = new(&tree.root(), &o).unwrap();
    let report = tree.validate(true, CacheMode::Off);
    let own: Vec<&str> = report
        .errors
        .iter()
        .filter(|e| e.path == r.path)
        .map(|e| e.criterion.as_str())
        .collect();
    assert_eq!(own, ["TL_143", "TLMD_152"], "{}", json(&report));
    // A fence that holds no YAML still draws the finding of its schema.
    tree.write(
        r.path.as_str().trim_start_matches('/'),
        &format!(
            "{}\n````yaml\n: [\n````\n",
            r.document.replace("description:", "description: A task.")
        ),
    );
    let report = tree.validate(true, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.path == r.path && e.criterion == "SOKF_SCHEMA_2"),
        "{}",
        json(&report)
    );
}

// A sidecar's skeleton binds a heading to `resource` as it binds one to the
// title: the heading reads the path of the asset that `--for` names.
// sokf-tests: CLI-6_AC-126
// sokf-tests: CLI-6_AC-14
#[test]
fn a_heading_bound_to_the_resource_takes_the_assets_path() {
    let tree = TempTree::fixture("new");
    let schema = tree.read("knowledge/schemas/image.md").replace(
        "preamble:\n  guidance: One line under the picture.\n  blocks:\n",
        "sections:\n  - heading: { from: resource }\n    level: 1\n    blocks:\n",
    );
    tree.write("knowledge/schemas/image.md", &schema);
    let mut o = new_options(tree.dir(), "Image");
    o.id = Some("flowers".into());
    o.for_asset = Some(PathBuf::from("knowledge/media/draft/flowers.png"));
    let r = new(&tree.root(), &o).unwrap();
    assert!(
        r.document
            .ends_with("---\n\n# /knowledge/media/draft/flowers.png\n"),
        "{}",
        r.document
    );
    assert_eq!(
        r.document
            .matches("resource: /knowledge/media/draft/flowers.png")
            .count(),
        1,
        "{}",
        r.document
    );
}
