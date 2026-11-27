//! These tests cover templates and the grammar of a block's rules: a fence is
//! strict, a counter counts lines without hiding any of them, the validator
//! checks the productions of a list for their order and the variants of a
//! format for their examples, and every shape matches a unit's text after
//! collapsing its whitespace.

use common::{TempTree, lined, on, said};
use proptest::prelude::*;
use sokf_core::CacheMode;

use crate::common;

/// `FENCE_SCHEMA` declares a fence of flowchart lines: one production for the
/// type line, one for each node, a counter that counts the nodes of class
/// `block`, and nothing for any other line.
const FENCE_SCHEMA: &str = r#"---
type: Schema
id: schema-fence
description: A fence whose lines are a grammar.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Fence
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: code
        lang: mermaid
        each:
          lines:
            - text: flowchart LR
            - template: '${id:word}["${name}"]:::${:word}'
              min: 1
              example: '  one["One"]:::block'
            - select-pattern: ':::block$'
              min: 1
````
"#;

fn fence(tree: &TempTree, body: &str) -> Vec<String> {
    tree.write(
        "knowledge/fence-a.md",
        &format!(
            "---\ntype: Fence\nid: fence-a\ntitle: Alpha\n---\n\n# Alpha\n\n```mermaid\n{body}```\n"
        ),
    );
    on(&tree.validate(false, CacheMode::Off).errors, "fence-a.md")
        .map(lined)
        .collect()
}

fn tree_with(schema: &str) -> TempTree {
    TempTree::clean_with(&[("knowledge/schemas/fence.md", schema)])
}

// A counter counts the lines its guard matches but does not consume any of
// them, so the validator still reports a malformed line that the counter counts. It
// reports the line against the production that matches the most of it, at
// the column where that production stops matching.
#[test]
fn a_counter_counts_a_malformed_line_and_hides_it_from_nothing() {
    let tree = tree_with(FENCE_SCHEMA);
    assert_eq!(
        fence(
            &tree,
            "flowchart LR\n  one[\"One\"]:::block\n  two[Two]:::block\n"
        ),
        vec![
            "12: `two[Two]:::block` matches no rule of the fence; the nearest fails at column 5: expected `\"`, found `Two]:::block`; a line of this kind reads `  one[\"One\"]:::block`"
        ]
    );
}

// A fence is a grammar: a line that no rule matches is an error, unless the
// block sets `lines-open`, which allows such lines.
#[test]
fn a_stray_line_fails_a_strict_fence_and_passes_an_open_one() {
    let tree = tree_with(FENCE_SCHEMA);
    let stray = "flowchart LR\n  one[\"One\"]:::block\n  %% a note\n";
    assert_eq!(
        fence(&tree, stray),
        vec![
            "12: `%% a note` matches no rule of the fence; a line of this kind reads `  one[\"One\"]:::block`; declare it or set `lines-open`"
        ]
    );
    let open = tree_with(&FENCE_SCHEMA.replace(
        "        each:\n          lines:\n",
        "        each:\n          lines-open: true\n          lines:\n",
    ));
    assert_eq!(fence(&open, stray), [] as [std::string::String; 0]);
    // A blank line and an indent are layout, not lines.
    assert_eq!(
        fence(&tree, "flowchart LR\n\n      one[\"One\"]:::block\n"),
        [] as [std::string::String; 0]
    );
}

fn schema_errors(tree: &TempTree) -> Vec<String> {
    on(&tree.validate(false, CacheMode::Off).errors, "fence.md")
        .map(|f| said(&f.message))
        .collect()
}

// When an earlier production matches the example of a later one, the later
// production never matches a line, and the validator reports an error when
// it loads the schema.
#[test]
fn a_production_an_earlier_one_hides_is_refused() {
    let tree = tree_with(&FENCE_SCHEMA.replace(
        "            - select-pattern: ':::block$'\n",
        "            - template: '${id:word}[\"${name}\"]:::block'\n              example: '  two[\"Two\"]:::block'\n            - select-pattern: ':::block$'\n",
    ));
    let got = schema_errors(&tree);
    assert!(
        got.iter().any(|m| m.contains(
            "the example `  two[\"Two\"]:::block` is taken by the rule at line 21 first, so this rule takes nothing"
        )),
        "{got:?}"
    );
}

// The example of each variant must match that variant and no other, so that
// each line matches exactly one variant.
#[test]
fn a_variant_whose_example_another_variant_takes_is_refused() {
    let tree = tree_with(&FENCE_SCHEMA.replace(
        "line-limit: 0\n",
        "line-limit: 0\nformats:\n  kind:\n    variants:\n      any:\n        template: 'kind ${}'\n        example: kind a\n      a:\n        text: kind a\n        example: kind a\n",
    ));
    let got = schema_errors(&tree);
    assert!(
        got.iter().any(|m| m
            .contains("the example of variant `any` of format `kind` also matches variant `a`")),
        "{got:?}"
    );
}

/// `WRAP_SCHEMA` requires each paragraph of a document to match a template.
const WRAP_SCHEMA: &str = r"---
type: Schema
id: schema-wrap
description: A template over a paragraph that wraps.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Wrap
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        each:
          template: 'The ${what} SHALL ${}.'
          example: The tool SHALL print the report.
````
";

fn wrap(body: &str) -> Vec<String> {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/wrap.md", WRAP_SCHEMA);
    tree.write(
        "knowledge/wrap-a.md",
        &format!("---\ntype: Wrap\nid: wrap-a\ntitle: Alpha\n---\n\n# Alpha\n\n{body}"),
    );
    on(&tree.validate(false, CacheMode::Off).errors, "wrap-a.md")
        .map(lined)
        .collect()
}

// Every shape collapses each run of whitespace in a unit's text to one space
// before matching, so a paragraph that wraps at any space, or contains
// repeated spaces, passes wherever its one-line form passes. A failure
// reports the line of the file where the text stops matching.
#[test]
fn a_wrapped_unit_passes_its_shape_and_fails_on_its_own_line() {
    assert_eq!(
        wrap("The tool SHALL print the report.\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        wrap("The   tool\nSHALL print\n  the report.\n"),
        [] as [std::string::String; 0]
    );
    let got = wrap("The tool\nSHALL print\nthe report\n");
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].starts_with("11: "), "{got:?}");
}

/// `ordered` builds a schema with one bullet list and one format: the two
/// rules of the list, and the two variants of the format, are the two given
/// shapes in order, each with its example.
fn ordered(first: &str, first_example: &str, second: &str, second_example: &str) -> String {
    format!(
        "---\ntype: Schema\nid: schema-fence\ndescription: Two shapes in order.\n---\n\n````yaml\nline-limit: 0\nformats:\n  pair:\n    variants:\n      one:\n        {first}\n        example: '{first_example}'\n      two:\n        {second}\n        example: '{second_example}'\nfrontmatter:\n  type:\n    const: Fence\nsections:\n  - heading: {{ from: title }}\n    level: 1\n    blocks:\n      - kind: bullet-list\n        rules:\n          - {first}\n            min: 0\n            example: '{first_example}'\n          - {second}\n            min: 0\n            example: '{second_example}'\n````\n"
    )
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(16))]

    /// Two exact texts never match each other's example. A rule hides a later
    /// rule only when the earlier rule matches the later rule's example, but no
    /// two variants can match the same line, whatever their order, because
    /// each line must match exactly one variant.
    #[test]
    fn a_shape_hides_the_example_of_a_later_one_only_where_it_takes_it(
        a in "[a-z]{2,6}",
        b in "[A-Z]{2,6}",
    ) {
        let text_a = format!("text: {a}");
        let text_b = format!("text: {b}");
        let any = "template: 'x ${}'".to_string();
        let tree = tree_with(&ordered(&text_a, &a, &text_b, &b));
        prop_assert!(schema_errors(&tree).is_empty(), "{:?}", schema_errors(&tree));
        let narrow = format!("text: x {a}");
        let tree = tree_with(&ordered(&narrow, &format!("x {a}"), &any, "x y"));
        let got = schema_errors(&tree);
        prop_assert!(got.iter().all(|m| !m.contains("first, so this rule takes nothing")), "{got:?}");
        prop_assert!(got.iter().any(|m| m.contains("variant `one` of format `pair` also matches variant `two`")), "{got:?}");
        let tree = tree_with(&ordered(&any, "x y", &format!("text: x {a}"), &format!("x {a}")));
        let got = schema_errors(&tree);
        prop_assert!(got.iter().any(|m| m.contains("first, so this rule takes nothing")), "{got:?}");
        prop_assert!(got.iter().any(|m| m.contains("also matches variant `one`")), "{got:?}");
    }
}
