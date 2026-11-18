//! These tests check the warnings at the citations of a deprecated document
//! (`SOKF_VALIDATION_57` to `SOKF_VALIDATION_66`): a body link, a marker, and an
//! include block each warn, and name the document that supersedes the
//! deprecated one, whichever end records the link. A document that
//! supersedes it, an archived document, and a deprecated document cite it
//! without a warning, and so does a frontmatter `links` entry.

use common::{TempTree, incremental::edit, json};
use sokf_core::{CacheMode, Report};

use crate::common;

/// The schema of a page: a title, then any text.
const PAGE_SCHEMA: &str = r"---
type: Schema
id: schema-page
description: A page of free text.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Page
links:
  - rel: supersedes
    to-type: Page
    min: 0
  - rel: superseded-by
    to-type: Page
    min: 0
  - rel: relates-to
    to-type: Page
    min: 0
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The text of the page.
      - kind: include
        optional: true
        example: |
          <!-- sokf:include page-x -->
          <!-- /sokf:include -->
  - heading: Part
    level: 2
    optional: true
    blocks:
      - kind: prose
        example: A part of the text.
````
";

/// The schema of a record. Every record is archived.
const RECORD_SCHEMA: &str = r"---
type: Schema
id: schema-record
description: A record of past work, which is archived.
---

````yaml
line-limit: 0
archived: true
frontmatter:
  type:
    const: Record
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The work of the record.
````
";

/// `page` returns a page with the frontmatter lines `extra` and the body
/// `body`.
fn page(id: &str, extra: &str, body: &str) -> String {
    format!("---\ntype: Page\nid: {id}\ntitle: {id}\n{extra}---\n\n# {id}\n\n{body}\n")
}

/// The deprecated page, with a heading that a link can cite.
fn old_page() -> String {
    page(
        "page-old",
        "status: deprecated\n",
        "The old text.\n\n## Part\n\nA part of the old text.",
    )
}

/// The deprecated page, which names `to` as its replacement with a
/// `superseded-by` link.
fn old_page_superseded_by(to: &str) -> String {
    page(
        "page-old",
        &format!("status: deprecated\nlinks:\n  - rel: superseded-by\n    to: {to}\n"),
        &format!(
            "The old text, which [its replacement][sokf:{to}] replaces.\n\n## Part\n\nA part of the old text."
        ),
    )
}

/// The page that supersedes the deprecated page.
fn new_page() -> String {
    page(
        "page-new",
        "links:\n  - rel: supersedes\n    to: page-old\n",
        "This page replaces [the old page][sokf:page-old].",
    )
}

/// The page that cites the deprecated page by a body link, by a link to its
/// heading, and by an include block, and declares a `links` entry to it.
fn user_page(lead: &str) -> String {
    page(
        "page-user",
        "links:\n  - rel: relates-to\n    to: page-old\n",
        &format!(
            "{lead}See [the old page][sokf:page-old].\n\nSee [its part][sokf:page-old#part].\n\n<!-- sokf:include page-old -->\n<!-- /sokf:include -->"
        ),
    )
}

/// `tree` returns a tree that holds the pages, a source file with a marker
/// that cites the deprecated page, a closed record and a second deprecated
/// page that cite it, and fixes the tree.
fn tree() -> TempTree {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/page.md", PAGE_SCHEMA);
    tree.write("knowledge/schemas/record.md", RECORD_SCHEMA);
    tree.write("knowledge/page-old.md", &old_page());
    tree.write("knowledge/page-new.md", &new_page());
    tree.write("knowledge/page-user.md", &user_page(""));
    tree.write(
        "knowledge/record-log.md",
        "---\ntype: Record\nid: record-log\ntitle: record-log\n---\n\n# record-log\n\nThe work moved off [the old page][sokf:page-old].\n",
    );
    tree.write(
        "knowledge/page-older.md",
        &page(
            "page-older",
            "status: deprecated\n",
            "An older page that cites [the old page][sokf:page-old].",
        ),
    );
    // The test writes the marker in two parts, so that the validator of this
    // repository doesn't read it as a marker of this file.
    tree.write(
        "src/uses.rs",
        &format!("// sokf-{}: page-old\nfn uses() {{}}\n", "implements"),
    );
    tree.validate(true, CacheMode::Off);
    tree
}

/// `warned` returns the warnings of `report` about a deprecated document, as
/// `path:line: message`.
fn warned(report: &Report) -> Vec<String> {
    report
        .warnings
        .iter()
        .filter(|f| f.message.contains("deprecated"))
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

/// The warnings of the tree that `tree` writes, with the user page's text
/// starting at line 12.
const WARNINGS: [&str; 4] = [
    "/knowledge/page-user.md:12: link to `page-old`, which is deprecated: `page-new` supersedes it (SOKF_VALIDATION_57)",
    "/knowledge/page-user.md:14: link to `page-old#part`, in `page-old`, which is deprecated: `page-new` supersedes it (SOKF_VALIDATION_57)",
    "/knowledge/page-user.md:16: `sokf:include` includes `page-old`, which is deprecated: `page-new` supersedes it (SOKF_VALIDATION_59)",
    "/src/uses.rs:1: `sokf-implements:` cites `page-old`, which is deprecated: `page-new` supersedes it (SOKF_VALIDATION_58)",
];

// sokf-tests: SOKF_VALIDATION_57
// sokf-tests: SOKF_VALIDATION_58
// sokf-tests: SOKF_VALIDATION_59
// sokf-tests: SOKF_VALIDATION_60
// sokf-tests: SOKF_VALIDATION_61
// sokf-tests: SOKF_VALIDATION_62
// sokf-tests: SOKF_VALIDATION_63
// sokf-tests: SOKF_VALIDATION_64
// sokf-tests: CLI-4_AC-36
#[test]
fn each_citation_of_a_deprecated_document_warns_and_names_its_replacement() {
    let tree = tree();
    for mode in [CacheMode::Off, CacheMode::On, CacheMode::On] {
        let report = tree.validate(false, mode);
        assert_eq!(warned(&report), WARNINGS, "{mode:?}\n{}", json(&report));
        assert!(report.errors.is_empty(), "{mode:?}\n{}", json(&report));
    }
}

// Without a `supersedes` link, a warning names no replacement.
// sokf-tests: SOKF_VALIDATION_57
// sokf-tests: SOKF_VALIDATION_64
#[test]
fn a_warning_names_no_replacement_when_no_document_supersedes_the_document() {
    let tree = tree();
    tree.write(
        "knowledge/page-new.md",
        &page(
            "page-new",
            "",
            "This page replaces [the old page][sokf:page-old].",
        ),
    );
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(
        warned(&report)[0],
        "/knowledge/page-new.md:9: link to `page-old`, which is deprecated (SOKF_VALIDATION_57)",
        "{}",
        json(&report)
    );
    assert_eq!(warned(&report).len(), 5, "{}", json(&report));
}

// With the marker check off, the run reads no marker, and a marker gets no
// warning.
// sokf-tests: SOKF_VALIDATION_58
#[test]
fn a_marker_gets_no_warning_when_the_marker_check_is_off() {
    let tree = tree();
    tree.write(
        ".sokf/config.toml",
        "[knowledge]\nroots = [\"/knowledge\"]\n\n[markers]\ncheck = \"off\"\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(warned(&report), WARNINGS[..3], "{}", json(&report));
}

// A deprecated document that becomes stable warns no more, and the cache
// follows each change: a moved line, a new and a dropped `supersedes`
// link, and the status.
// sokf-tests: SOKF_VALIDATION_57
// sokf-tests: SOKF_VALIDATION_64
#[test]
fn the_cache_follows_each_change_to_a_citation_of_a_deprecated_document() {
    let tree = tree();
    assert_eq!(warned(&tree.validate(false, CacheMode::On)), WARNINGS);
    // A line before the links moves each warning down by two lines.
    edit(&tree, "knowledge/page-user.md", &user_page("Read on.\n\n"));
    let moved: Vec<String> = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(moved, warned(&tree.validate(false, CacheMode::Off)));
    assert!(
        moved[0].starts_with("/knowledge/page-user.md:14: "),
        "{moved:?}"
    );
    // A second page that supersedes the deprecated one joins the name.
    edit(
        &tree,
        "knowledge/page-newer.md",
        &page(
            "page-newer",
            "links:\n  - rel: supersedes\n    to: page-old\n",
            "This page also replaces [the old page][sokf:page-old].",
        ),
    );
    tree.validate(true, CacheMode::On);
    let named = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(named, warned(&tree.validate(false, CacheMode::Off)));
    assert!(
        named[0].ends_with("`page-new` and `page-newer` supersede it (SOKF_VALIDATION_57)"),
        "{named:?}"
    );
    // A page that drops its `supersedes` entry keeps its body link, which
    // warns, and leaves the name.
    edit(
        &tree,
        "knowledge/page-newer.md",
        &page(
            "page-newer",
            "",
            "This page also replaces [the old page][sokf:page-old].",
        ),
    );
    let dropped = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(dropped, warned(&tree.validate(false, CacheMode::Off)));
    assert_eq!(dropped.len(), 5, "{dropped:?}");
    assert!(
        dropped
            .iter()
            .all(|w| w.contains(": `page-new` supersedes it")),
        "{dropped:?}"
    );
    // A stable document warns no more.
    edit(
        &tree,
        "knowledge/page-old.md",
        &old_page().replace("status: deprecated\n", ""),
    );
    tree.validate(true, CacheMode::On);
    assert_eq!(
        warned(&tree.validate(false, CacheMode::On)),
        Vec::<String>::new()
    );
}

// A page that declares a `supersedes` entry and has no body link that
// mirrors it (an error of its own) still names itself in each warning, and
// the cache follows the entry.
// sokf-tests: SOKF_VALIDATION_64
#[test]
fn a_replacement_that_only_a_links_entry_declares_is_named() {
    let tree = tree();
    let only_links = page(
        "page-new",
        "links:\n  - rel: supersedes\n    to: page-old\n",
        "This page replaces the old page.",
    );
    tree.write("knowledge/page-new.md", &only_links);
    for mode in [CacheMode::Off, CacheMode::On, CacheMode::On] {
        let report = tree.validate(false, mode);
        assert_eq!(warned(&report), WARNINGS, "{mode:?}\n{}", json(&report));
        assert!(
            report
                .errors
                .iter()
                .all(|f| f.criterion == "SOKF_FORMAT_75"),
            "{mode:?}\n{}",
            json(&report)
        );
    }
    // Without the entry, no document supersedes the deprecated page.
    edit(
        &tree,
        "knowledge/page-new.md",
        &page("page-new", "", "This page replaces the old page."),
    );
    let dropped = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(dropped, warned(&tree.validate(false, CacheMode::Off)));
    assert!(
        dropped.iter().all(|w| !w.contains("supersede")),
        "{dropped:?}"
    );
}

/// `fixed` fixes `tree`, and checks that a second fix changes nothing.
fn fixed(tree: &TempTree) {
    tree.validate(true, CacheMode::Off);
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A deprecated page that names its replacement with `superseded-by` gets
// the same warnings as one that its replacement names with `supersedes`, and
// the replacement cites it without a warning.
// sokf-tests: SOKF_VALIDATION_65
// sokf-tests: SOKF_VALIDATION_66
#[test]
fn a_replacement_that_the_deprecated_document_names_is_named_and_cites_it_freely() {
    let tree = tree();
    tree.write(
        "knowledge/page-new.md",
        &page(
            "page-new",
            "",
            "This page replaces [the old page][sokf:page-old].",
        ),
    );
    tree.write("knowledge/page-old.md", &old_page_superseded_by("page-new"));
    fixed(&tree);
    for mode in [CacheMode::Off, CacheMode::On, CacheMode::On] {
        let report = tree.validate(false, mode);
        assert_eq!(warned(&report), WARNINGS, "{mode:?}\n{}", json(&report));
        assert!(report.errors.is_empty(), "{mode:?}\n{}", json(&report));
    }
}

// A replacement that both ends record is named once.
// sokf-tests: SOKF_VALIDATION_64
// sokf-tests: SOKF_VALIDATION_65
#[test]
fn a_replacement_that_both_links_record_is_named_once() {
    let tree = tree();
    tree.write("knowledge/page-old.md", &old_page_superseded_by("page-new"));
    fixed(&tree);
    for mode in [CacheMode::Off, CacheMode::On, CacheMode::On] {
        let report = tree.validate(false, mode);
        assert_eq!(warned(&report), WARNINGS, "{mode:?}\n{}", json(&report));
        assert!(report.errors.is_empty(), "{mode:?}\n{}", json(&report));
    }
}

// The name follows a `superseded-by` target that comes, goes, and changes
// its id, with the cache as without it, and a target that is no document
// names nothing.
// sokf-tests: SOKF_VALIDATION_65
#[test]
fn the_cache_follows_a_replacement_that_the_deprecated_document_names() {
    let tree = tree();
    tree.write(
        "knowledge/page-old.md",
        &old_page_superseded_by("page-newer"),
    );
    fixed(&tree);
    let absent = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(absent, WARNINGS, "{absent:?}");
    // The target comes, and joins the name.
    edit(
        &tree,
        "knowledge/page-newer.md",
        &page("page-newer", "", "This page also replaces the old page."),
    );
    let named = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(named, warned(&tree.validate(false, CacheMode::Off)));
    assert_eq!(named.len(), 4, "{named:?}");
    assert!(
        named
            .iter()
            .all(|w| w.contains(": `page-new` and `page-newer` supersede it")),
        "{named:?}"
    );
    // The target goes, and leaves the name.
    std::fs::remove_file(tree.dir().join("knowledge/page-newer.md")).unwrap();
    let gone = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(gone, warned(&tree.validate(false, CacheMode::Off)));
    assert_eq!(gone, WARNINGS, "{gone:?}");
    // A target that is a file and no document names nothing.
    edit(
        &tree,
        "knowledge/page-old.md",
        &old_page_superseded_by("/src/uses.rs"),
    );
    let file = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(file, warned(&tree.validate(false, CacheMode::Off)));
    assert_eq!(file, WARNINGS, "{file:?}");
    // A target named by its path changes its id, and the name follows it.
    edit(
        &tree,
        "knowledge/page-newer.md",
        &page("page-newer", "", "This page also replaces the old page."),
    );
    edit(
        &tree,
        "knowledge/page-old.md",
        &old_page_superseded_by("/knowledge/page-newer.md"),
    );
    let by_path = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(by_path, warned(&tree.validate(false, CacheMode::Off)));
    assert_eq!(by_path.len(), 4, "{by_path:?}");
    assert!(
        by_path
            .iter()
            .all(|w| w.contains(": `page-new` and `page-newer` supersede it")),
        "{by_path:?}"
    );
    edit(
        &tree,
        "knowledge/page-newer.md",
        &page("page-renamed", "", "This page also replaces the old page."),
    );
    let renamed = warned(&tree.validate(false, CacheMode::On));
    assert_eq!(renamed, warned(&tree.validate(false, CacheMode::Off)));
    assert_eq!(renamed.len(), 4, "{renamed:?}");
    assert!(
        renamed
            .iter()
            .all(|w| w.contains(": `page-new` and `page-renamed` supersede it")),
        "{renamed:?}"
    );
}

// A deprecated page that names itself with `superseded-by` names no
// replacement of its own, because a document can't replace itself.
// sokf-tests: SOKF_VALIDATION_65
#[test]
fn a_deprecated_document_that_names_itself_is_no_replacement() {
    let tree = tree();
    tree.write("knowledge/page-old.md", &old_page_superseded_by("page-old"));
    fixed(&tree);
    for mode in [CacheMode::Off, CacheMode::On, CacheMode::On] {
        let report = tree.validate(false, mode);
        assert_eq!(warned(&report), WARNINGS, "{mode:?}\n{}", json(&report));
        assert!(report.errors.is_empty(), "{mode:?}\n{}", json(&report));
    }
}

// Each warning spells each replacement as the store of the citing file
// spells it: a replacement in another store keeps its store prefix, so it
// keeps its own name beside a replacement with the same id, whichever end
// records the link.
// sokf-tests: SOKF_VALIDATION_64
// sokf-tests: SOKF_VALIDATION_65
#[test]
fn a_replacement_in_another_store_is_named_with_its_store() {
    let tree = tree();
    tree.add_other_store();
    tree.write("other/schemas/page.md", PAGE_SCHEMA);
    tree.write(
        "other/page-new.md",
        &page("page-new", "", "This page replaces the old page."),
    );
    tree.write(
        "other/page-newest.md",
        &page(
            "page-newest",
            "links:\n  - rel: supersedes\n    to: fixture:page-old\n",
            "This page replaces [the old page][sokf:fixture:page-old].",
        ),
    );
    tree.write(
        "other/page-reader.md",
        &page(
            "page-reader",
            "",
            "Read [the old page][sokf:fixture:page-old].",
        ),
    );
    tree.write(
        "knowledge/page-old.md",
        &old_page_superseded_by("other:page-new"),
    );
    fixed(&tree);
    let here = ": `other:page-new`, `other:page-newest`, and `page-new` supersede it";
    let there = ": `fixture:page-new`, `page-new`, and `page-newest` supersede it";
    for mode in [CacheMode::Off, CacheMode::On, CacheMode::On] {
        let report = tree.validate(false, mode);
        let named = warned(&report);
        assert_eq!(named.len(), 5, "{mode:?}\n{}", json(&report));
        for w in &named {
            let spelled = if w.starts_with("/other/") {
                there
            } else {
                here
            };
            assert!(w.contains(spelled), "{mode:?}: {w}");
        }
        assert!(report.errors.is_empty(), "{mode:?}\n{}", json(&report));
    }
}
