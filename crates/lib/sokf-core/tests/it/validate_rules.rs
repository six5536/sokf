//! These tests check the rules that a schema applies to a document's units
//! (bound text, shapes, guarded rules, and lines), and that each finding
//! identifies the rule it came from.

use common::{TempTree, json, lined, on, said};
use sokf_core::CacheMode;

use crate::common;

const MEMO_SCHEMA: &str = "---\ntype: Schema\nid: schema-memo\ndescription: A memo.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Memo\n  description:\n    optional: true\n    example: The move rewrites both indexes.\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        max: { hard: 1, when-at-limit: 'Write only the description.' }\n        each: { text: { from: description } }\n      - kind: blockquote\n        max: { hard: 1, when-at-limit: 'Write only the title.' }\n        each: { text: { from: title } }\n      - kind: bullet-list\n        sequence:\n          - text: DONE\n          - text: { from: title }\n      - kind: numbered-list\n        each: { text: STEP }\n````\n";

// sokf-tests: CLI-4_AC-4
#[test]
fn bound_text_is_checked_against_the_frontmatter_and_fix_rewrites_it() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/memo.md", MEMO_SCHEMA);
    tree.write(
        "knowledge/memo-a.md",
        "---\ntype: Memo\nid: memo-a\ntitle: Alpha\ndescription: >\n  One line that\n  wraps in the file.\n---\n\n# Alpha\n\nOld text that\nalso wraps.\n\n> Wrong\n\n- done\n- Alpha\n\n1. step one\n   continued\n2. STEP\n",
    );
    tree.write(
        "knowledge/memo-b.md",
        "---\ntype: Memo\nid: memo-b\ntitle: Beta\n---\n\n# Beta\n\nNo description above.\n\n> Beta\n\n- DONE\n- Beta\n\n1. STEP\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let messages: Vec<String> = report
        .errors
        .iter()
        .filter(|f| f.path.file_name() != "index.md")
        .map(|f| format!("{}: {}", f.path.relative(), said(&f.message)))
        .collect();
    assert_eq!(
        messages,
        vec![
            "knowledge/memo-a.md: `Old text that also wraps.` must equal the frontmatter `description`: `One line that wraps in the file.`",
            "knowledge/memo-a.md: `Wrong` must equal the frontmatter `title`: `Alpha`",
            "knowledge/memo-a.md: `done` must read `DONE`",
            "knowledge/memo-a.md: `step one continued` must read `STEP`",
            "knowledge/memo-b.md: `No description above.` is bound to the frontmatter `description`, which is absent",
        ],
        "{}",
        json(&report)
    );
    let fixed = tree.validate(true, CacheMode::Off);
    assert_eq!(
        tree.read("knowledge/memo-a.md"),
        "---\ntype: Memo\nid: memo-a\ntitle: Alpha\ndescription: >\n  One line that\n  wraps in the file.\n---\n\n# Alpha\n\nOne line that wraps in the file.\n\n> Alpha\n\n- DONE\n- Alpha\n\n1. STEP\n2. STEP\n"
    );
    // The fix doesn't repair text bound to an absent key. The finding stays.
    let left: Vec<&str> = fixed
        .errors
        .iter()
        .filter(|f| f.path.file_name() != "index.md")
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(left.len(), 1, "{}", json(&fixed));
    assert!(left[0].contains("which is absent"));
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

#[test]
fn text_is_refused_beside_a_pattern_and_on_a_row() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/bad.md",
        "---\ntype: Schema\nid: schema-bad\ndescription: Bad.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Bad\nsections:\n  - heading: A\n    level: 1\n    blocks:\n      - kind: prose\n        each: { text: A, pattern: '^A$', example: 'A' }\n      - kind: table\n        columns: [X]\n        each: { text: A }\n````\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let messages: Vec<&str> = report
        .errors
        .iter()
        .filter(|f| f.path.relative() == "knowledge/schemas/bad.md")
        .map(|f| f.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`text` and `pattern` beside each other: a rule has one shape")),
        "{}",
        json(&report)
    );
    assert!(
        messages
            .iter()
            .any(|m| m
                .contains("`text` applies to a paragraph, an item or a line, not a `table` block")),
        "{}",
        json(&report)
    );
}

const DIAGRAM_SCHEMA: &str = r#"---
type: Schema
id: schema-diagram
description: A diagram, with a section for each node and a list of notes.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Diagram
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: code
        lang: mermaid
        rule-id: nodes
        each:
          lines:
            - text: flowchart LR
            - select-pattern: '^[a-z][a-z0-9]*\['
              rule-id: node
              min: 2
              max: { hard: 3, when-at-limit: 'Split the diagram in two.' }
              pattern: '^(?P<k>[a-z][a-z0-9]*)\['
              key: k
              example: '  one["One"]'
            - select-pattern: '\-\->'
              min: 1
              pattern: '\-\->\|"'
              example: '  one -->|"calls"| two'
      - kind: bullet-list
        optional: true
        rules:
          - select-pattern: '^NOTE: '
            min: 0
            pattern: '\.$'
            example: 'NOTE: One.'
  - heading: { pattern: '^Node: (?P<k>[a-z][a-z0-9]*)$', key: k, example: 'Node: one' }
    guidance: One section per node the diagram declares.
    level: 2
    repeat: true
    for-each-key-of: nodes
    blocks: []
````
"#;

// sokf-tests: CLI-11_AC-1
#[test]
fn guarded_rules_govern_lines_and_units() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/diagram.md", DIAGRAM_SCHEMA);
    tree.write(
        "knowledge/diagram-bad.md",
        "---\ntype: Diagram\nid: diagram-bad\ntitle: Bad\n---\n\n# Bad\n\n```mermaid\nflowchart LR\n    one[\"One\"]\n    two[\"Two\"]\n    three[\"Three\"]\n    four[\"Four\"]\n    one --> two\n```\n\n- NOTE: no period\n- plain\n\n## Node: one\n\n## Node: two\n\n## Node: three\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let messages: Vec<String> = on(&report.errors, "diagram-bad.md").map(lined).collect();
    assert_eq!(
        messages,
        vec![
            "14: no `/^Node: (?P<k>[a-z][a-z0-9]*)$/` section for key `four`",
            "14: the lines matching `^[a-z][a-z0-9]*\\[`: at most 3 are allowed; there are 4. When at the limit: Split the diagram in two.",
            "15: `one --> two` does not match `\\-\\->\\|\"`; a line of this kind reads `  one -->|\"calls\"| two`",
            "18: `NOTE: no period` does not match `\\.$`; an item of this kind reads `NOTE: One.`",
            "19: `plain` matches no rule of the block; declare it or set `units-open`",
        ],
        "{}",
        json(&report)
    );
}

#[test]
fn two_sections_for_one_key_are_reported() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/diagram.md", DIAGRAM_SCHEMA);
    tree.write(
        "knowledge/diagram-dup.md",
        "---\ntype: Diagram\nid: diagram-dup\ntitle: Dup\n---\n\n# Dup\n\n```mermaid\nflowchart LR\n    one[\"One\"]\n    two[\"Two\"]\n    one -->|\"calls\"| two\n```\n\n## Node: one\n\n## Node: one\n\n## Node: two\n",
    );
    let messages: Vec<String> = on(
        &tree.validate(false, CacheMode::Off).errors,
        "diagram-dup.md",
    )
    .map(|f| said(&f.message))
    .collect();
    assert_eq!(
        messages,
        vec!["key `one` is captured more than once; it was first captured at line 16"]
    );
}

const REFUSED_SCHEMA: &str = r"---
type: Schema
id: schema-refused
description: Three rules in places they do not belong.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Refused
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        each:
          lines:
            - pattern: '.+'
              example: 'text one'
      - kind: code
        lang: any
        each:
          lines:
            - text: LABEL
      - kind: bullet-list
        each:
          select-pattern: '^a'
      - kind: numbered-list
        each: { pattern: '.+', example: 'text one' }
        rules:
          - select-pattern: '^a'
            min: 0
      - kind: blockquote
        rules:
          - select-pattern: '^NAME: '
            min: 0
            text: { from: title }
````
";

#[test]
fn a_guard_and_lines_are_refused_where_they_do_not_belong() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/refused.md", REFUSED_SCHEMA);
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "refused.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        messages,
        vec![
            "`lines` applies to a `code` or `html` block, not a `prose` block (TLMD_90)",
            "`select-pattern` applies to a rule under `rules` or `lines`, which chooses what it governs (TLMD_91)",
            "`rules` beside `each` or `sequence`: a block takes one of the three (TLMD_47)",
            "`text` beside `select-pattern`: the repair rewrites the unit whole and would drop what the guard selected on (TLMD_142)",
        ]
    );
}

// A finding from a schema identifies the rule that produced it, so a reader
// can find that rule in the schema. The finding names the rule's `rule-id`
// when the rule declares one. Otherwise it names the rule's section and the
// position of its block.
#[test]
fn a_schema_finding_names_the_rule_it_came_from() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/diagram.md", DIAGRAM_SCHEMA);
    tree.write(
        "knowledge/diagram-bad.md",
        "---\ntype: Diagram\nid: diagram-bad\ntitle: Bad\n---\n\n# Bad\n\n```mermaid\nflowchart LR\n    one[\"One\"]\n    two[\"Two\"]\n    one --> two\n```\n\n- NOTE: no period\n\n## Node: one\n\n## Node: two\n",
    );
    let messages: Vec<String> = on(
        &tree.validate(false, CacheMode::Off).errors,
        "diagram-bad.md",
    )
    .map(|f| f.message.clone())
    .collect();
    assert_eq!(
        messages,
        vec![
            "`one --> two` does not match `\\-\\->\\|\"`; a line of this kind reads `  one -->|\"calls\"| two` (schema schema-diagram, rule `nodes`)",
            "`NOTE: no period` does not match `\\.$`; an item of this kind reads `NOTE: One.` (schema schema-diagram, section \"<title>\" block 2)",
        ]
    );
}

// A schema's fence must be plain YAML: an anchor, an alias, or a tag in it
// is an error on the schema, reported at the line that holds it.
// sokf-tests: EXT-10_AC-1
#[test]
fn a_schema_with_an_anchor_alias_or_tag_is_refused() {
    let tree = TempTree::fixture("clean");
    let schema = |id: &str, line: &str| {
        format!(
            "---\ntype: Schema\nid: schema-{id}\ndescription: Plain YAML only.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: {line}\nsections: []\n````\n"
        )
    };
    tree.write(
        "knowledge/schemas/anchored.md",
        &schema("anchored", "&a Anchored"),
    );
    tree.write("knowledge/schemas/aliased.md", &schema("aliased", "*a"));
    tree.write(
        "knowledge/schemas/tagged.md",
        &schema("tagged", "!!str Tagged"),
    );
    let mut messages: Vec<String> = tree
        .validate(false, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| f.path.relative().starts_with("knowledge/schemas/"))
        .filter(|f| f.path.file_name() != "index.md")
        .map(|f| {
            format!(
                "{}:{}: {}",
                f.path.file_name(),
                f.line.unwrap_or(0),
                sokf_core::report_message(f)
            )
        })
        .collect();
    messages.sort();
    assert_eq!(messages.len(), 3, "{messages:?}");
    for (m, (file, word)) in messages.iter().zip([
        ("aliased.md", "alias at line 11: aliases"),
        ("anchored.md", "anchor at line 11: anchors"),
        ("tagged.md", "tag at line 11: tags"),
    ]) {
        assert_eq!(
            m,
            &format!("{file}:11: schema: YAML {word} are not read (SOKF_SCHEMA_2)")
        );
    }
}

// A document's frontmatter must be plain YAML: an anchor, an alias, or a
// tag in it is an error on the document, reported at the line that holds it.
// sokf-tests: FORMAT-4_AC-1
#[test]
fn a_frontmatter_with_an_anchor_alias_or_tag_is_refused() {
    let tree = TempTree::fixture("clean");
    let original = tree.read("knowledge/decisions/decision-single-binary.md");
    for (file, title) in [
        ("anchored", "&a Ship one binary"),
        ("aliased", "*a"),
        ("tagged", "!!str Ship one binary"),
    ] {
        let id = format!("decision-{file}");
        let text = original
            .replace("id: decision-single-binary", &format!("id: {id}"))
            .replace("title: Ship one binary", &format!("title: {title}"));
        tree.write(&format!("knowledge/decisions/{id}.md"), &text);
    }
    let mut messages: Vec<String> = tree
        .validate(false, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| f.message.contains("YAML "))
        .map(|f| {
            format!(
                "{}:{}: {}",
                f.path.file_name(),
                f.line.unwrap_or(0),
                sokf_core::report_message(f)
            )
        })
        .collect();
    messages.sort();
    assert_eq!(messages.len(), 3, "{messages:?}");
    for (m, (file, word)) in messages.iter().zip([
        ("decision-aliased.md", "alias at line 4: aliases"),
        ("decision-anchored.md", "anchor at line 4: anchors"),
        ("decision-tagged.md", "tag at line 4: tags"),
    ]) {
        assert_eq!(
            m,
            &format!("{file}:4: frontmatter: YAML {word} are not read (SOKF_FORMAT_18)")
        );
    }
}

/// A gated `links` rule is sokf's own rule, so sokf's host gives its gate to
/// the engine, which checks the gate against the markers of each scaffold.
// sokf-tests: EXT-1_AC-3
#[test]
fn a_gated_link_needs_its_marker_in_the_scaffold() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/task.md",
        "---\ntype: Schema\nid: schema-task\ntitle: Task Schema\ndescription: A task with a gated link.\n---\n\n````yaml\nline-limit: 0\nlifecycle:\n  - { key: state, states: [open, closed] }\nfrontmatter:\n  type:\n    const: Task\nlinks:\n  - rel: implements\n    to-type: Task\n    min: 1\n    only: { state: closed }\n    guidance: The task that this one completes.\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        example: The task text.\n````\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let gated: Vec<_> = report
        .errors
        .iter()
        .filter(|f| f.path.as_str() == "/knowledge/schemas/task.md")
        .filter(|f| f.criterion == "TLMD_122")
        .collect();
    assert_eq!(gated.len(), 1, "{:#?}", report.errors);
    assert_eq!(gated[0].line, Some(16));
    assert!(
        gated[0].message.contains("`- rel: implements`"),
        "{}",
        gated[0].message
    );
}
