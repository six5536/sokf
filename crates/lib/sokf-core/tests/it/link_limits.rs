//! These tests check the `max` of a `links` rule, a limit with its
//! `when-at-limit` (EXT-8): the grammar's findings on a `max` that isn't such
//! a limit, the check of a hard limit and of a soft one, and the guidance
//! line that the scaffold writes before the rule's entries.

use std::fmt::Write;

use common::{TempTree, new_options};
use sokf_core::{CacheMode, new};

use crate::common;

/// `schema` returns a schema of notes whose `relates-to` rule declares
/// `max`, written as the YAML of the rule.
fn schema(max: &str) -> String {
    format!(
        r"---
type: Schema
id: schema-note
title: Note Schema
description: A note that relates to other notes.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Note
  id:
    template: 'note-${{:slug}}'
links:
  - rel: relates-to
    to-type: Note
    min: 0
    max: {max}
    guidance: A relates-to link points to a note on the same subject.
sections:
  - heading: {{ from: title }}
    level: 1
    blocks:
      - kind: prose
        example: The kettle needs descaling.
````
"
    )
}

/// `note` returns a note with the id `note-<slug>` that relates to each note
/// of `to`, in its frontmatter and in its body.
fn note(slug: &str, to: &[&str]) -> String {
    if to.is_empty() {
        return format!(
            "---\ntype: Note\nid: note-{slug}\ntitle: {slug}\n---\n\n# {slug}\n\nThe kettle needs descaling.\n"
        );
    }
    let mut links = String::new();
    let mut definitions = String::new();
    for t in to {
        let _ = write!(links, "  - rel: relates-to\n    to: note-{t}\n");
        let _ = writeln!(definitions, "[sokf:note-{t}]: /knowledge/note-{t}.md");
    }
    let body: Vec<String> = to.iter().map(|t| format!("[{t}][sokf:note-{t}]")).collect();
    format!(
        "---\ntype: Note\nid: note-{slug}\ntitle: {slug}\nlinks:\n{links}---\n\n# {slug}\n\nThe kettle needs descaling, as {} say.\n\n<!-- sokf:links -->\n\n{definitions}",
        body.join(", ")
    )
}

/// `tree` returns a store whose schema of notes declares `max` on its
/// `relates-to` rule, with a note that relates to three others.
fn tree(max: &str) -> TempTree {
    let tree = TempTree::empty("link-limits");
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: link-limits\nscope: test\n",
    );
    tree.write("knowledge/schemas/note.md", &schema(max));
    tree.write(
        "knowledge/note-kettle.md",
        &note("kettle", &["vinegar", "descaler", "filter"]),
    );
    for slug in ["vinegar", "descaler", "filter"] {
        tree.write(&format!("knowledge/note-{slug}.md"), &note(slug, &[]));
    }
    tree
}

/// `errors` returns each error of a run with `--fix`, which writes the
/// indexes, as `path:line: message (key)`, after it checks that a second
/// run fixes nothing.
fn errors(tree: &TempTree, mode: CacheMode) -> Vec<String> {
    tree.validate(true, mode);
    let report = tree.validate(true, mode);
    assert!(report.fixes.is_empty(), "{:?}", report.fixes);
    report
        .errors
        .iter()
        .map(|f| {
            let line = f.line.unwrap_or(0);
            format!("{}:{line}: {} ({})", f.path, f.message, f.criterion)
        })
        .collect()
}

// A bare number keeps the check that the schema had only as a hard limit,
// so the finding shows that map. A map without its text, with both kinds,
// or with an empty text is refused too.
// sokf-tests: EXT-8_AC-4
#[test]
fn a_max_that_is_no_limit_with_its_text_breaks_the_grammar() {
    for (max, finding) in [
        (
            "2",
            "`max` is a map of a limit and its `when-at-limit` text, not a number: write `max: { hard: 2, when-at-limit: \"…\" }` to keep the check, or `soft` for a limit that is never checked (TL_137)",
        ),
        ("{ hard: 2 }", "`limit` requires `when-at-limit` (TL_103)"),
        (
            "{ soft: 1, hard: 2, when-at-limit: 'Split the note.' }",
            "`limit` needs exactly one of `soft`, `hard` (TL_104)",
        ),
        (
            "{ hard: 2, when-at-limit: ' ' }",
            "`when-at-limit` of `max` is empty: it holds the text that tells the writer what to do at the limit (TL_138)",
        ),
    ] {
        let tree = tree(max);
        let on_schema: Vec<String> = errors(&tree, CacheMode::Off)
            .into_iter()
            .filter(|e| e.starts_with("/knowledge/schemas/note.md"))
            .collect();
        assert_eq!(
            on_schema,
            [format!("/knowledge/schemas/note.md:19: {finding}")],
            "{max}"
        );
    }
}

// A `min` above the count of a soft or a hard `max` is a mistake in the
// schema, and a hard limit of 0 contradicts a `min` of one.
// sokf-tests: EXT-8_AC-7
#[test]
fn a_min_above_the_max_breaks_the_schema() {
    for (min, count, kind) in [(4, 3, "soft"), (4, 3, "hard"), (1, 0, "hard")] {
        let max = format!("{{ {kind}: {count}, when-at-limit: 'Split the note.' }}");
        let max = max.as_str();
        let tree = tree(max);
        let schema = schema(max).replace("    min: 0\n", &format!("    min: {min}\n"));
        tree.write("knowledge/schemas/note.md", &schema);
        let finding =
            format!("/knowledge/schemas/note.md:16: `min` {min} is above `max` {count} (TL_98)");
        let found = errors(&tree, CacheMode::Off);
        assert!(found.contains(&finding), "{max}: {found:?}");
    }
}

// The finding on too many links ends with the way out, its whitespace
// squashed, and names one entry in the singular.
// sokf-tests: EXT-8_AC-2
// sokf-tests: SOKF_SCHEMA_33
// sokf-tests: SOKF_SCHEMA_103
#[test]
fn more_links_than_a_hard_limit_quote_the_way_out() {
    for (count, entries) in [(2, "entries"), (1, "entry")] {
        let tree = tree(&format!(
            "{{ hard: {count}, when-at-limit: 'Split the note  by\tsubject.' }}"
        ));
        for mode in [CacheMode::Off, CacheMode::On] {
            assert_eq!(
                errors(&tree, mode),
                [format!(
                    "/knowledge/note-kettle.md:6: `links` allows at most {count} `relates-to` {entries}, but it has 3. When at the limit: Split the note by subject. (schema schema-note, link `relates-to`) (SOKF_SCHEMA_33)"
                )],
                "{count} {mode:?}"
            );
        }
    }
    let tree = tree("{ hard: 3, when-at-limit: 'Split the note.' }");
    assert_eq!(errors(&tree, CacheMode::Off), Vec::<String>::new());
}

// sokf-tests: EXT-8_AC-3
#[test]
fn a_soft_limit_bounds_no_links() {
    let tree = tree("{ soft: 1, when-at-limit: 'Split the note.' }");
    for mode in [CacheMode::Off, CacheMode::On] {
        assert_eq!(errors(&tree, mode), Vec::<String>::new(), "{mode:?}");
    }
}

// The guidance line before the rule's entries holds the rule's guidance,
// then sokf's words for the limit, which name the `rel`, and its text. A
// rule without guidance has a line with the limit alone, and a rule that a
// hard limit of 0 bans has no entry. A store in a language that sokf has
// no words for gets the limit in the syntax of the schema.
// sokf-tests: EXT-8_AC-5
// sokf-tests: EXT-8_AC-6
// sokf-tests: CLI-6_AC-120
// sokf-tests: CLI-6_AC-122
#[test]
fn the_scaffold_writes_the_limit_after_the_rules_guidance() {
    let tree = tree("{ hard: 2, when-at-limit: 'Split the note by subject.' }");
    let mut o = new_options(tree.dir(), "Note");
    o.slug = Some("milk".into());
    let line = |tree: &TempTree| {
        let doc = new(&tree.root(), &o).unwrap().document;
        let at = doc.find("links:\n").expect(&doc);
        doc[at..].lines().nth(1).unwrap().to_string()
    };
    assert_eq!(
        line(&tree),
        "  # sokf:guidance A relates-to link points to a note on the same subject. At most 2 `relates-to` links. Never leave out or merge content to stay within the count. When at the limit: Split the note by subject."
    );
    let unguided = |max: &str| {
        schema(max).replace(
            "    guidance: A relates-to link points to a note on the same subject.\n",
            "",
        )
    };
    tree.write(
        "knowledge/schemas/note.md",
        &unguided("{ soft: 1, when-at-limit: 'Split the note by subject.' }"),
    );
    assert_eq!(
        line(&tree),
        "  # sokf:guidance Usually up to 1 `relates-to` link. Never leave out or merge content to stay within the count. When at the limit: Split the note by subject."
    );
    // A ban leaves the rule out, so the schema's own scaffold passes it.
    tree.write(
        "knowledge/schemas/note.md",
        &schema("{ hard: 0, when-at-limit: 'Name the other notes in the text.' }"),
    );
    let doc = new(&tree.root(), &o).unwrap().document;
    assert!(!doc.contains("links:"), "{doc}");
    let on_schema: Vec<String> = errors(&tree, CacheMode::Off)
        .into_iter()
        .filter(|e| e.starts_with("/knowledge/schemas/"))
        .collect();
    assert_eq!(on_schema, Vec::<String>::new());
    tree.write(
        "knowledge/schemas/note.md",
        &unguided("{ hard: 2, when-at-limit: 'Split the note by subject.' }"),
    );
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: link-limits\nscope: test\nprimary-language: de\n",
    );
    assert_eq!(
        line(&tree),
        "  # sokf:guidance `hard: 2`, `when-at-limit`: Split the note by subject."
    );
}
