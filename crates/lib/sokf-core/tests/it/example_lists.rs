//! A rule can give a list of examples, and a repeating section, or a
//! section that pairs with the keys of another rule, can give further
//! occurrences of itself. The scaffold writes every one of them, so
//! that a new document shows several units rather than one, and the checker
//! checks each one.

use common::{TempTree, json, new_options, on};
use sokf_core::{CacheMode, NewOptions, new};

use crate::common;

/// `LISTED_SCHEMA` gives a list of examples to each kind of rule: an `each`
/// rule, a block, a guarded rule with `min: 0`, the positions of a
/// sequence, and a code block. Its last sections repeat with further
/// occurrences, and a list pairs with the keys of their headings.
const LISTED_SCHEMA: &str = r#"---
type: Schema
id: schema-listed
title: Listed Schema
description: Every rule gives several examples.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Listed
  id:
    template: "listed-${:seq}-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Parts
    level: 2
    blocks:
      - kind: bullet-list
        each:
          template: '${name:word}: ${rest:text}'
          example:
            - 'Index: maps each key to its document.'
            - 'Cache: holds the parsed documents.'
            - 'Lock: keeps two writers apart.'
  - heading: Steps
    level: 2
    blocks:
      - kind: numbered-list
        min: 4
        each:
          example: [Read the file., Parse the blocks.]
  - heading: Limits
    level: 2
    blocks:
      - kind: bullet-list
        max: { hard: 2, when-at-limit: 'Keep the limits that bind first.' }
        example: [The index fits in memory., One writer holds the lock., A run takes one second.]
  - heading: Notes
    level: 2
    blocks:
      - kind: prose
        rules:
          - pattern: '^Note: '
            rule-id: note
            min: 0
            max: { hard: 2, when-at-limit: 'Fold the rest into an aside.' }
            example: ['Note: the cache is local.', 'Note: the lock is advisory.', 'Note: the index is rebuilt.']
          - pattern: '^Aside: '
            min: 0
            example: 'Aside: the log rotates daily.'
  - heading: Order
    level: 2
    blocks:
      - kind: bullet-list
        sequence:
          - template: 'First: ${step:text}'
            example: ['First: open the file.', 'First: lock the index.']
          - template: 'Then: ${step:text}'
            repeat: true
            example: ['Then: read a block.', 'Then: check the block.']
  - heading: Samples
    level: 2
    blocks:
      - kind: code
        lang: sh
        repeat: true
        example: ['sokf validate', 'sokf new Note']
  - heading: { template: 'Part ${name:word}', key: name, example: Part Parser }
    level: 2
    repeat: true
    rule-id: part
    blocks:
      - kind: prose
        example: The parser reads a document into blocks.
    examples:
      - |
        ## Part Checker

        The checker matches each block against its rule.
      - |
        ## Part Writer

        The writer applies each repair.
  - heading: Owners
    level: 2
    blocks:
      - kind: bullet-list
        each:
          template: '${part:word} belongs to ${who:word}.'
          key: part
          for-each-key-of: part
          example: Parser belongs to Ana.
````
"#;

/// `errors` returns the messages of the errors on the schema `listed.md`.
fn errors(tree: &TempTree) -> Vec<String> {
    on(&tree.validate(false, CacheMode::Off).errors, "listed.md")
        .map(|f| f.message.clone())
        .collect()
}

/// `scaffold` writes `schema` and returns the body of its scaffold, as the
/// guide of a new document gives it: the example of each section, in order,
/// each without its guidance, its markers, and its anchors.
fn scaffold(tree: &TempTree, schema: &str) -> String {
    tree.write("knowledge/schemas/listed.md", schema);
    let options = NewOptions {
        slug: Some("thing".into()),
        ..new_options(tree.dir(), "Listed")
    };
    let made = new(&tree.root(), &options).unwrap();
    let sections: Vec<String> = made
        .guide
        .unwrap()
        .sections
        .into_iter()
        .filter_map(|s| s.example)
        .collect();
    sections.join("\n\n") + "\n"
}

// A schema that gives lists passes its own check: each derived scaffold,
// with every unit that the lists write, satisfies the schema.
#[test]
fn a_schema_with_lists_derives_a_scaffold_that_passes() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/listed.md", LISTED_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "listed.md"),
        "{}",
        json(&report)
    );
}

// An `each` rule writes one item for each text of its list, in order.
#[test]
fn a_list_writes_one_unit_for_each_text_in_order() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains(
            "- Index: maps each key to its document.\n- Cache: holds the parsed documents.\n- Lock: keeps two writers apart.\n"
        ),
        "{doc}"
    );
}

// A `min` above the length of the list repeats the last text.
#[test]
fn a_min_above_the_list_repeats_the_last_text() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains(
            "1. Read the file.\n2. Parse the blocks.\n3. Parse the blocks.\n4. Parse the blocks.\n"
        ),
        "{doc}"
    );
}

// The `max` of the block and the `max` of the rule each cap the list.
#[test]
fn a_max_caps_the_list() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains("- The index fits in memory.\n- One writer holds the lock.\n\n"),
        "{doc}"
    );
    assert!(!doc.contains("A run takes one second."), "{doc}");
    assert!(!doc.contains("Note: the index is rebuilt."), "{doc}");
}

// A guarded rule with `min: 0` writes its list, and a single example with
// `min: 0` still writes nothing.
#[test]
fn a_rule_with_min_zero_writes_its_list() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains("Note: the cache is local.\n\nNote: the lock is advisory.\n"),
        "{doc}"
    );
    assert!(!doc.contains("Aside:"), "{doc}");
}

// A position of a sequence that repeats writes its whole list, and any
// other position writes its first text.
#[test]
fn a_repeating_position_writes_its_list_and_another_its_first_text() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains("- First: open the file.\n- Then: read a block.\n- Then: check the block.\n"),
        "{doc}"
    );
    assert!(!doc.contains("First: lock the index."), "{doc}");
}

// A code block writes one fence for each text of its list.
#[test]
fn a_code_block_writes_one_fence_for_each_text() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains("```sh\nsokf validate\n```\n\n```sh\nsokf new Note\n```\n"),
        "{doc}"
    );
}

// A repeating section writes its derived occurrence, then each further
// occurrence unchanged, and a later pairing sees the keys of all of them.
#[test]
fn further_occurrences_follow_the_derived_one() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, LISTED_SCHEMA);
    assert!(
        doc.contains(
            "Part Parser\n\nThe parser reads a document into blocks.\n\n## Part Checker\n\nThe checker matches each block against its rule.\n\n## Part Writer\n\nThe writer applies each repair.\n"
        ),
        "{doc}"
    );
    assert!(
        doc.contains(
            "- Parser belongs to Ana.\n- Checker belongs to Ana.\n- Writer belongs to Ana.\n"
        ),
        "{doc}"
    );
}

// Each text of a list is checked against its rule, and the failure names
// the position of the text that fails.
#[test]
fn a_bad_second_text_is_reported_by_its_position() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace(
            "            - 'Cache: holds the parsed documents.'\n",
            "            - 'the cache holds documents'\n",
        ),
    );
    let messages = errors(&tree);
    assert!(
        messages.iter().any(|m| m.starts_with(
            "item 2 of `example`: the example `the cache holds documents` fails at column 4"
        )),
        "{messages:?}"
    );
}

// Guidance that repeats any text of a list only says again what the list
// already says.
#[test]
fn guidance_that_repeats_a_text_of_a_list_is_reported() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace(
            "        max: { hard: 2, when-at-limit: 'Keep the limits that bind first.' }\n        example: [",
            "        max: { hard: 2, when-at-limit: 'Keep the limits that bind first.' }\n        guidance: One writer holds the lock.\n        example: [",
        ),
    );
    let messages = errors(&tree);
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("`guidance` and `example` hold the same words")),
        "{messages:?}"
    );
}

// An empty list gives the scaffold nothing to write.
#[test]
fn an_empty_list_is_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace(
            "example: [Read the file., Parse the blocks.]",
            "example: []",
        ),
    );
    let messages = errors(&tree);
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("`example` is an empty list")),
        "{messages:?}"
    );
}

// A code block that holds one fence can't hold the fences of a list.
#[test]
fn a_list_on_a_code_block_that_does_not_repeat_is_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace(
            "        lang: sh\n        repeat: true\n",
            "        lang: sh\n",
        ),
    );
    let messages = errors(&tree);
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("a list `example` on a `code` block")),
        "{messages:?}"
    );
}

// Only a repeating section has further occurrences.
#[test]
fn examples_on_a_section_that_does_not_repeat_are_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace(
            "    level: 2\n    repeat: true\n    rule-id: part\n",
            "    level: 2\n    rule-id: part\n",
        ),
    );
    let messages = errors(&tree);
    assert!(
        messages.iter().any(|m| m.starts_with(
            "`examples` holds further occurrences of a section, so the section declares `repeat: true`"
        )),
        "{messages:?}"
    );
}

// Each further occurrence begins with a heading of the section's level.
#[test]
fn an_occurrence_whose_heading_has_the_wrong_level_is_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace("        ## Part Writer\n", "        ### Part Writer\n"),
    );
    let messages = errors(&tree);
    assert!(
        messages.iter().any(|m| m.starts_with(
            "item 2 of `examples` begins with `### Part Writer`, but each occurrence begins with a heading of level 2"
        )),
        "{messages:?}"
    );
}

// The check of the scaffold checks the body of each further occurrence.
#[test]
fn an_occurrence_that_breaks_the_section_fails_the_scaffold() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &LISTED_SCHEMA.replace("\n        The writer applies each repair.\n", ""),
    );
    let messages = errors(&tree);
    assert!(
        messages.iter().any(|m| m.starts_with("the scaffold fails")),
        "{messages:?}"
    );
}

/// `PAIRED_SCHEMA` pairs a repeating section with the keys of a list, and
/// gives the section one further occurrence in `examples`. The list
/// captures three keys, so the scaffold writes the second occurrence from
/// the text and the third from the rules.
const PAIRED_SCHEMA: &str = r#"---
type: Schema
id: schema-listed
title: Listed Schema
description: A section pairs with the keys of a list.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Listed
  id:
    template: "listed-${:seq}-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Parts
    level: 2
    blocks:
      - kind: bullet-list
        each:
          template: '${name:word}: ${rest:text}'
          key: name
          key-of: parts
          example:
            - 'Parser: reads a document into blocks.'
            - 'Checker: matches each block against its rule.'
            - 'Writer: applies each repair.'
  - heading: { template: 'Part ${name:word}', key: name, example: Part Parser }
    level: 2
    repeat: true
    for-each-key-of: parts
    blocks:
      - kind: prose
        example: The part has one job.
    examples:
      - |
        ## Part Checker

        The checker matches each block against its rule.
````
"#;

// A section that pairs with another rule can carry further occurrences,
// and a schema that gives them passes its own check.
#[test]
fn examples_on_a_paired_section_are_accepted() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/listed.md", PAIRED_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "listed.md"),
        "{}",
        json(&report)
    );
}

// The occurrence of the first key comes from the rules, the occurrence of
// the second key from the text, and the occurrence of the third key, after
// the texts run out, from the rules again.
#[test]
fn a_paired_section_writes_later_keys_from_its_examples() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree, PAIRED_SCHEMA);
    let parser = doc
        .find("Part Parser\n\nThe part has one job.\n")
        .expect(&doc);
    let checker = doc
        .find("Part Checker\n\nThe checker matches each block against its rule.\n")
        .expect(&doc);
    let writer = doc
        .find("Part Writer\n\nThe part has one job.\n")
        .expect(&doc);
    assert!(parser < checker && checker < writer, "{doc}");
    assert_eq!(doc.matches("The part has one job.").count(), 2, "{doc}");
}

// A text is checked as the occurrence of its key, so a text whose heading
// captures another key fails the scaffold.
#[test]
fn a_paired_occurrence_with_the_wrong_key_fails_the_scaffold() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &PAIRED_SCHEMA.replace("        ## Part Checker\n", "        ## Part Lock\n"),
    );
    let messages = errors(&tree);
    assert!(
        messages.iter().any(|m| m.starts_with("the scaffold fails")
            && m.contains("names key `Lock`, which is not a key of set `parts`")),
        "{messages:?}"
    );
}

// A section that doesn't repeat has no further occurrences, even when it
// pairs with another rule.
#[test]
fn examples_on_a_paired_section_that_does_not_repeat_are_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/listed.md",
        &PAIRED_SCHEMA.replace(
            "    repeat: true\n    for-each-key-of",
            "    for-each-key-of",
        ),
    );
    let messages = errors(&tree);
    assert!(
        messages.iter().any(|m| m.starts_with(
            "`examples` holds further occurrences of a section, so the section declares `repeat: true`"
        )),
        "{messages:?}"
    );
}

// The further occurrences of an optional section lie in the same optional
// region as the occurrence that the scaffold derives, so the scaffold passes
// its schema, and the guide gives them in the one example of the section.
#[test]
fn further_occurrences_of_an_optional_section_share_its_region() {
    let tree = TempTree::fixture("clean");
    let schema = LISTED_SCHEMA.replace(
        "    repeat: true\n    rule-id: part\n",
        "    repeat: true\n    optional: true\n    rule-id: part\n",
    );
    tree.write("knowledge/schemas/listed.md", &schema);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "listed.md"),
        "{}",
        json(&report)
    );
    // The guide gives the section, optional, with every occurrence that the
    // scaffold writes in its one example.
    tree.write("knowledge/schemas/listed.md", &schema);
    let options = NewOptions {
        slug: Some("thing".into()),
        ..new_options(tree.dir(), "Listed")
    };
    let made = new(&tree.root(), &options).unwrap();
    let part = made
        .guide
        .unwrap()
        .sections
        .into_iter()
        .find(|s| s.rule_id.as_deref() == Some("part"))
        .unwrap();
    assert!(part.optional);
    let example = part.example.unwrap();
    assert!(
        example.contains("Part Parser") && example.contains("Part Writer"),
        "{example}"
    );
}
