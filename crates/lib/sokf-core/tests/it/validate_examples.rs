//! These tests check a schema's examples and what a schema must explain:
//! an example that its own rule refuses, guidance that repeats the example,
//! a regex with no example, the lower bound of a guarded rule, and the keys
//! that a format declares.

use common::{TempTree, lined, on, reported};
use sokf_core::CacheMode;

use crate::common;

const REFUSED_EXAMPLE_SCHEMA: &str = r#"---
type: Schema
id: schema-badexample
description: Each rule refuses the example it declares.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: BadExample
  stage:
    enum: [one, two]
    example: three
formats:
  shout:
    pattern: '^[A-Z ]+$'
    example: not upper case
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        optional: true
        rules:
          - select-pattern: '^NOTE: '
            min: 0
            example: a note that no guard selects
      - kind: prose
        example: "Text the block stands for."
        each:
          prohibited-patterns: [{pattern: 'Text', message: 'An item contains no Text.'}]
  - heading: { pattern: '^Item: (.+)$', example: Not an item }
    level: 2
    optional: true
    blocks: []
````
"#;

// A rule's `example` is text of the kind that the rule demands, so the
// example must pass that rule, whether or not a scaffold writes the example.
#[test]
fn an_example_its_own_rule_refuses_is_reported() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/badexample.md", REFUSED_EXAMPLE_SCHEMA);
    // The scaffold writes the examples, so the scaffold fails too. This
    // test checks only that each example matches its own rule.
    let messages: Vec<String> = on(
        &tree.validate(false, CacheMode::Off).errors,
        "badexample.md",
    )
    .filter(|f| !f.message.starts_with("the scaffold fails"))
    .map(reported)
    .collect();
    assert_eq!(
        messages,
        vec![
            "13: the example of `stage` is not one of `one`, `two` (schema schema-badexample, key `stage`) (TL_64)",
            "17: the example `not upper case` of format `shout` does not match `^[A-Z ]+$` (TL_65)",
            "26: the example `a note that no guard selects` does not match `select-pattern` `^NOTE: `, so this rule would not govern it (schema schema-badexample, section \"<title>\" block 1) (TL_64)",
            "29: the example `Text the block stands for.` is refused: `Text` is refused: An item contains no Text. (schema schema-badexample, section \"<title>\" block 2) (TL_64)",
            "33: the example `Not an item` does not match `^Item: (.+)$` (schema schema-badexample, section \"/^Item: (.+)$/\") (TL_64)",
        ]
    );
}

// Guidance says what the example cannot, so guidance that contains the
// same sentence as the example adds nothing.
#[test]
fn guidance_that_repeats_the_example_is_reported() {
    let tree = TempTree::fixture("clean");
    let schema = BARE_REGEX_SCHEMA.replace(
        "      - kind: bullet-list\n",
        "      - kind: bullet-list\n        guidance: \"NOTE: the parser caches its result\"\n        example: 'NOTE: the parser caches its result'\n",
    );
    tree.write("knowledge/schemas/bare.md", &schema);
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "bare.md")
        .map(reported)
        .collect();
    assert!(
        messages.iter().any(|m| m
            .starts_with("21: `guidance` and `example` hold the same words: ")
            && m.ends_with(" (TL_68)")),
        "{messages:?}"
    );
    // Guidance that says what the example cannot say passes the check.
    tree.write(
        "knowledge/schemas/bare.md",
        &schema.replace(
            "        guidance: \"NOTE: the parser caches its result\"\n",
            "        guidance: One aside per thing the reader would otherwise trip on.\n",
        ),
    );
    let left: Vec<String> = tree
        .validate(false, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| f.path.file_name() == "bare.md" && f.criterion == "TL_68")
        .map(reported)
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

// A literal heading already states the exact text, so an `example` beside it
// adds nothing and the grammar refuses it.
#[test]
fn an_example_beside_a_literal_heading_is_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/badexample.md",
        &REFUSED_EXAMPLE_SCHEMA.replace(
            "  - heading: { pattern: '^Item: (.+)$', example: Not an item }\n",
            "  - heading: Item\n    example: Not an item\n",
        ),
    );
    let messages: Vec<String> = on(
        &tree.validate(false, CacheMode::Off).errors,
        "badexample.md",
    )
    .map(reported)
    .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("unknown key `example` in `section`")),
        "{messages:?}"
    );
}

const BARE_REGEX_SCHEMA: &str = r"---
type: Schema
id: schema-bare
description: No regex here declares an example.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Bare
  owner:
    pattern: '^[a-z]+$'
prohibited-patterns:
  - pattern: '\bTODO\b'
    message: 'A finished document contains no TODO.'
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        max: 3
        each:
          pattern: '^NOTE: '
      - kind: include
        source-pattern: '^/src/'
  - heading: { pattern: '^Item: (.+)$' }
    level: 2
    optional: true
    blocks: []
````
";

// A regex alone doesn't tell the writer what text to write, so a rule needs
// an example. Each of the four rules whose error quotes a regex must declare
// an `example`. `max: 3` needs none: a reader understands a count without
// help. A prohibited pattern needs none: its error quotes the text it matched.
#[test]
fn a_rule_that_declares_a_regex_explains_itself() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/bare.md", BARE_REGEX_SCHEMA);
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "bare.md")
        .map(reported)
        .collect();
    // Each finding sits on the line of the rule that declares the regex: the
    // frontmatter constraint, the unit, the block, and the heading.
    let says = |line: usize, key: &str| {
        format!(
            "{line}: a rule that declares `{key}` carries an `example` of it: a failure shows the example, and the scaffold writes it (TL_66)"
        )
    };
    assert_eq!(
        messages,
        vec![
            says(13, "pattern"),
            says(24, "pattern"),
            says(25, "source-pattern"),
            says(27, "pattern"),
        ]
    );
    // After the rule declares an example, its error goes away.
    tree.write(
        "knowledge/schemas/bare.md",
        &BARE_REGEX_SCHEMA.replace(
            "          pattern: '^NOTE: '\n",
            "          pattern: '^NOTE: '\n          example: \"NOTE: the parser caches its result\"\n",
        ),
    );
    let left: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "bare.md")
        .map(reported)
        .collect();
    assert_eq!(
        left,
        vec![
            says(13, "pattern"),
            says(26, "source-pattern"),
            says(28, "pattern"),
        ]
    );
}

const UNBOUNDED_SCHEMA: &str = r"---
type: Schema
id: schema-unbounded
description: One guard states its lower bound and one leaves it out.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Unbounded
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        rules:
          - select-pattern: '^NOTE: '
            min: 0
            max: 3
          - select-pattern: '^TODO: '
            max: 0
      - kind: code
        lang: text
        each:
          lines:
            - pattern: '.+'
              example: 'text one'
````
";

// A guard says which units a rule governs, not that there is one of them,
// so a guarded rule must state for itself whether it demands any. A rule
// with no guard keeps its default `min` of one, and the line rule here
// relies on that default.
#[test]
fn a_guarded_rule_states_its_own_lower_bound() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/unbounded.md", UNBOUNDED_SCHEMA);
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "unbounded.md")
        .map(reported)
        .collect();
    assert_eq!(
        messages,
        vec![
            "21: a guarded rule states `min`: `min: 0` governs such units without demanding one, and a number demands that many (TLMD_92)",
        ]
    );
}

const KEYED_SCHEMA: &str = r"---
type: Schema
id: schema-keyed
description: The criteria use a format whose pattern lets the key be left out.
---

````yaml
line-limit: 0
formats:
  criterion:
    key: id
    pattern: '^(?:(?P<id>[A-Z]+-\d+_AC-\d+): )?.+$'
    example: 'EX-1_AC-1: the system SHALL do one thing'
frontmatter:
  type:
    const: Keyed
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        each:
          format: criterion
      - kind: numbered-list
        optional: true
        each:
          format: criterion
          key-optional: true
````
";

#[test]
fn a_key_a_format_declares_is_captured_or_reported() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/keyed.md", KEYED_SCHEMA);
    tree.write(
        "knowledge/keyed-a.md",
        "---\ntype: Keyed\nid: keyed-a\ntitle: Alpha\n---\n\n# Alpha\n\n- EX-1_AC-1: the system SHALL do one thing\n- the system SHALL do a second thing\n- EX-1-AC-3: the system SHALL do a third thing\n\n1. the system SHALL be waived\n",
    );
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "keyed-a.md")
        .map(lined)
        .collect();
    assert_eq!(
        messages,
        vec![
            "10: `the system SHALL do a second thing` carries no key: the format `criterion` captures `id`",
            "11: `EX-1-AC-3: the system SHALL do a third thing` carries no key: the format `criterion` captures `id`",
        ],
        "the list that waives its key produces no error"
    );
}

#[test]
fn key_optional_without_a_key_is_refused() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/loose.md",
        "---\ntype: Schema\nid: schema-loose\ndescription: The list waives a key that no format declares.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Loose\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        example: 'One item.'\n        each:\n          key-optional: true\n````\n",
    );
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "loose.md")
        .map(reported)
        .collect();
    assert_eq!(
        messages,
        vec!["19: `key-optional` without a `key` or a `format` that declares one (TL_47)"]
    );
}
