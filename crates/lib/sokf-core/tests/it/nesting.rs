//! These tests check that a section can sit inside another, take its number
//! from its parent, and pair its items with the keys of another rule.

use common::{TempTree, new_options, on, said};
use proptest::prelude::*;
use sokf_core::{CacheMode, NewOptions, new};

use crate::common;

/// A schema for a library whose shelves each list their books and might hold
/// an index of them.
const SHELF_SCHEMA: &str = r"---
type: Schema
id: schema-shelf
description: A library, its shelves, the books on each shelf, and an index of each shelf.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Shelf
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        rule-id: shelves
        each:
          pattern: '^(?P<k>[A-Z][a-z]+)$'
          key: k
          example: Poetry
  - heading: { pattern: '^\d+ Shelf: (?P<k>[A-Z][a-z]+)$', key: k, example: '1 Shelf: Poetry' }
    level: 2
    repeat: true
    for-each-key-of: shelves
    rule-id: shelf
    blocks:
      - kind: bullet-list
        each:
          rule-id: books
          pattern: '^book (?P<k>[a-z]+)$'
          key: k
          example: book odes
  - heading: { pattern: '^\d+\.\d+ Index$', example: '1.1 Index' }
    level: 3
    parent: shelf
    number-from-parent: true
    optional: true
    blocks:
      - kind: bullet-list
        each:
          pattern: '^index (?P<k>[a-z]+)$'
          key: k
          example: index odes
          for-each-key-of: books
````
";

const GOOD: &str = "---\ntype: Shelf\nid: library\ntitle: Library\n---\n\n# Library\n\n- Poetry\n- Prose\n\n## 1 Shelf: Poetry\n\n- book odes\n- book sonnets\n\n### 1.1 Index\n\n- index odes\n- index sonnets\n\n## 2 Shelf: Prose\n\n- book essays\n\n### 2.1 Index\n\n- index essays\n";

/// `errors` returns the messages of the errors that the library document
/// raises.
fn errors(schema: &str, doc: &str) -> Vec<String> {
    let tree = TempTree::clean_with(&[
        ("knowledge/schemas/shelf.md", schema),
        ("knowledge/library.md", doc),
    ]);
    tree.validate(false, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| {
            let name = f.path.file_name();
            name == "library.md" || name == "shelf.md"
        })
        .map(|f| said(&f.message))
        .collect()
}

fn with(from: &str, to: &str) -> String {
    assert!(GOOD.contains(from), "{from}");
    GOOD.replacen(from, to, 1)
}

#[test]
fn subsections_inside_each_parent_pass() {
    assert_eq!(errors(SHELF_SCHEMA, GOOD), [] as [std::string::String; 0]);
}

#[test]
fn a_key_with_no_unit_is_named_even_where_the_section_is_absent() {
    let doc = with("\n### 2.1 Index\n\n- index essays\n", "");
    assert_eq!(
        errors(SHELF_SCHEMA, &doc),
        vec!["no item names key `essays` of rule `books` under `## 2 Shelf: Prose`"]
    );
}

#[test]
fn a_unit_under_the_wrong_parent_is_an_orphan_there() {
    let doc = with("- index essays\n", "- index essays\n- index odes\n");
    assert_eq!(
        errors(SHELF_SCHEMA, &doc),
        vec!["key `odes` is not one rule `books` captured under `## 2 Shelf: Prose`"]
    );
}

#[test]
fn a_subsection_before_any_parent_is_outside_it() {
    let doc = with("## 1 Shelf: Poetry", "### 0.1 Index\n\n## 1 Shelf: Poetry");
    let got = errors(SHELF_SCHEMA, &doc);
    assert!(
        got.contains(
            &"heading `### 0.1 Index` lies outside every `## /^\\d+ Shelf: (?P<k>[A-Z][a-z]+)$/` section; it belongs inside one"
                .to_string()
        ),
        "{got:?}"
    );
}

#[test]
fn a_subsection_counts_within_its_parent() {
    let doc = with(
        "### 2.1 Index\n\n- index essays\n",
        "### 2.1 Index\n\n- index essays\n\n### 2.1 Index\n\n- index essays\n",
    );
    let got = errors(SHELF_SCHEMA, &doc);
    assert!(
        got.contains(
            &"heading `### 2.1 Index` occurs more than once under `## 2 Shelf: Prose`".to_string()
        ),
        "{got:?}"
    );
    // A subsection that is not optional must appear under each parent.
    let required = SHELF_SCHEMA.replace("    optional: true\n", "");
    let doc = with("\n### 2.1 Index\n\n- index essays\n", "");
    let got = errors(&required, &doc);
    assert!(
        got.contains(
            &"section `### /^\\d+\\.\\d+ Index$/` is missing under `## 2 Shelf: Prose`".to_string()
        ),
        "{got:?}"
    );
}

#[test]
fn a_subsection_is_numbered_from_its_parent() {
    let doc = with("### 2.1 Index", "### 1.2 Index");
    assert_eq!(
        errors(SHELF_SCHEMA, &doc),
        vec!["heading `### 1.2 Index` must be numbered `2.<n>`, under `## 2 Shelf: Prose`"]
    );
    let unnumbered = SHELF_SCHEMA
        .replace(
            r"'^\d+ Shelf: (?P<k>[A-Z][a-z]+)$'",
            r"'^Shelf: (?P<k>[A-Z][a-z]+)$'",
        )
        .replace("'1 Shelf: Poetry'", "'Shelf: Poetry'");
    let doc = GOOD
        .replace("## 1 Shelf: Poetry", "## Shelf: Poetry")
        .replace("## 2 Shelf: Prose", "## Shelf: Prose");
    let got = errors(&unnumbered, &doc);
    assert!(
        got.contains(
            &"heading `## Shelf: Poetry` carries no number for `### 1.1 Index` to extend"
                .to_string()
        ),
        "{got:?}"
    );
}

#[test]
fn a_pairing_repeats_a_key_only_against_itself() {
    // `books` and the index that pairs with it both capture `odes`, as the
    // pairing expects. The same key twice in one index is a repeat.
    let doc = with("- index sonnets\n", "- index sonnets\n- index odes\n");
    assert_eq!(
        errors(SHELF_SCHEMA, &doc),
        vec!["key `odes` is captured more than once; it was first captured at line 19"]
    );
}

#[test]
fn two_rules_of_one_section_pair_each_on_its_own() {
    let schema = SHELF_SCHEMA.replace(
        "          for-each-key-of: books\n````",
        "          for-each-key-of: books\n      - kind: table\n        columns: [Book, Place]\n        each:\n          pattern: '^(?P<k>[a-z]+) \\|'\n          key: k\n          example: odes | top\n          for-each-key-of: books\n````",
    );
    let doc = with(
        "- index sonnets\n",
        "- index sonnets\n\n| Book | Place |\n| --- | --- |\n| odes | top |\n",
    )
    .replace(
        "- index essays\n",
        "- index essays\n\n| Book | Place |\n| --- | --- |\n| essays | low |\n",
    );
    // `odes` in the list and in the table makes two pairings, not a repeat.
    // `sonnets` in the list does not count for the table.
    assert_eq!(
        errors(&schema, &doc),
        vec!["no row names key `sonnets` of rule `books` under `## 1 Shelf: Poetry`"]
    );
}

/// In this schema, the diagram has one rule for node lines and another for
/// edge lines. Each rule captures its own keys, and a list beside the diagram
/// pairs with the edge keys.
const EDGE_SCHEMA: &str = r#"---
type: Schema
id: schema-graph
description: A graph drawn once, with a section per node and a list of its edges.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Graph
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: code
        lang: mermaid
        rule-id: nodes
        each:
          lines:
            - select-pattern: '^[a-z]+\['
              min: 1
              pattern: '^(?P<k>[a-z]+)\['
              key: k
              example: '  one["One"]'
            - select-pattern: '-'
              rule-id: edges
              min: 1
              pattern: '^(?P<k>[a-z]+-[a-z]+)$'
              key: k
              example: '  one-two'
      - kind: bullet-list
        each:
          pattern: '^edge (?P<k>[a-z]+-[a-z]+)$'
          key: k
          example: edge one-two
          for-each-key-of: edges
  - heading: { pattern: '^Node: (?P<k>[a-z]+)$', key: k, example: 'Node: one' }
    level: 2
    repeat: true
    for-each-key-of: nodes
    blocks: []
````
"#;

#[test]
fn a_line_rule_names_its_own_keys() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/graph.md", EDGE_SCHEMA);
    tree.write(
        "knowledge/graph.md",
        "---\ntype: Graph\nid: graph\ntitle: Graph\n---\n\n# Graph\n\n```mermaid\n  one[\"One\"]\n  two[\"Two\"]\n  one-two\n```\n\n- edge one-two\n\n## Node: one\n\n## Node: two\n",
    );
    let errors: Vec<String> = tree
        .validate(false, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| f.path.file_name() != "index.md")
        .map(|f| said(&f.message))
        .collect();
    // The edge is not a node, so the schema requires no `Node:` section for it.
    assert!(errors.is_empty(), "{errors:?}");
}

/// `schema_errors` returns the messages of the errors that a schema raises.
fn schema_errors(schema: &str) -> Vec<String> {
    let schema = format!(
        "---\ntype: Schema\nid: schema-shelf\ndescription: A shelf.\n---\n\n````yaml\n{schema}````\n"
    );
    let tree = TempTree::clean_with(&[("knowledge/schemas/shelf.md", &schema)]);
    on(&tree.validate(false, CacheMode::Off).errors, "shelf.md")
        .map(|f| said(&f.message))
        .collect()
}

const HEAD: &str = "line-limit: 0\nfrontmatter:\n  type:\n    const: Shelf\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n  - heading: Shelf\n    level: 2\n    rule-id: shelf\n    blocks:\n      - kind: bullet-list\n        rule-id: books\n        each:\n          pattern: '^(?P<k>[a-z]+)$'\n          key: k\n          example: odes\n";

#[test]
fn a_parent_is_a_section_above_listed_before() {
    let child = |parent: &str, level: u8| {
        format!("  - heading: Index\n    level: {level}\n    parent: {parent}\n    blocks: []\n")
    };
    let cases = [
        (child("nothing", 3), "`parent` names no rule `nothing`"),
        (
            child("books", 3),
            "`parent` names `books`, which is not a section",
        ),
        (
            child("shelf", 2),
            "`parent` `shelf` is level 2; a section lies inside one of a smaller level, and this one is level 2",
        ),
        (
            format!(
                "  - heading: Other\n    level: 2\n    blocks: []\n{}",
                child("shelf", 3)
            ),
            "section `Index` is not listed after its `parent` `shelf` with only its subsections between",
        ),
    ];
    for (tail, want) in cases {
        let got = schema_errors(&format!("{HEAD}{tail}"));
        assert!(got.iter().any(|e| e.contains(want)), "{want}: {got:?}");
    }
    let got = schema_errors(&format!(
        "{HEAD}  - heading: Index\n    level: 3\n    number-from-parent: true\n    blocks: []\n"
    ));
    assert!(
        got.iter().any(|e| e.contains("requires `parent`")),
        "{got:?}"
    );
}

#[test]
fn a_pairing_rule_captures_a_key_of_a_rule_that_exists() {
    let list = |each: &str| {
        format!(
            "{HEAD}  - heading: Index\n    level: 2\n    blocks:\n      - kind: bullet-list\n        each:\n{each}"
        )
    };
    let cases = [
        (
            list(
                "          pattern: '^[a-z]+$'\n          example: odes\n          for-each-key-of: books\n",
            ),
            "`for-each-key-of` on a rule that captures no key",
        ),
        (
            list(
                "          pattern: '^(?P<k>[a-z]+)$'\n          key: k\n          example: odes\n          for-each-key-of: nothing\n",
            ),
            "`for-each-key-of` names no rule `nothing`",
        ),
        (
            list(
                "          pattern: '^(?P<k>[a-z]+)$'\n          key: k\n          example: odes\n          rule-id: books\n",
            ),
            "`rule-id` `books` is declared twice",
        ),
        (
            list(
                "          pattern: '^(?P<k>[a-z]+)$'\n          key: k\n          example: odes\n          rule-id: Not Slug\n",
            ),
            "`rule-id` must be a slug",
        ),
    ];
    for (schema, want) in cases {
        let got = schema_errors(&schema);
        assert!(got.iter().any(|e| e.contains(want)), "{want}: {got:?}");
    }
}

#[test]
fn a_derived_template_writes_the_subsection_inside_its_parent() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/shelf.md", SHELF_SCHEMA);
    let opts = NewOptions {
        cwd: tree.dir().join("knowledge"),
        id: Some("library".to_string()),
        ..new_options(tree.dir(), "Shelf")
    };
    let document = new(&tree.root(), &opts).unwrap().document;
    let body = document.split("\n---\n").nth(1).unwrap();
    let headings_and_items: Vec<&str> = body
        .lines()
        .filter(|l| l.starts_with('#') || l.starts_with("- "))
        .collect();
    assert_eq!(
        headings_and_items,
        vec![
            "# Library",
            "- Poetry",
            "## <a id=\"poetry\"></a>1 Shelf: Poetry",
            "- <a id=\"odes\"></a>book odes",
            "### 1.1 Index",
            "- index odes",
        ]
    );
    assert!(!document.contains("sokf:optional"), "{document}");
    // The schema raises no error, because the template derived from it
    // validates against it.
    let schema_findings: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "shelf.md")
        .map(|f| said(&f.message))
        .collect();
    assert!(schema_findings.is_empty(), "{schema_findings:?}");
}

#[test]
fn a_section_returns_after_its_subsections_only() {
    // When two repeating sections are siblings, the first must not appear
    // again after the second.
    let schema = SHELF_SCHEMA
        .replace(
            "    parent: shelf\n    number-from-parent: true\n",
            "    repeat: true\n",
        )
        .replace("    level: 3\n", "    level: 2\n")
        .replace(
            "          pattern: '^index (?P<k>[a-z]+)$'\n          key: k\n",
            "          pattern: '^index [a-z]+$'\n",
        )
        .replace("          for-each-key-of: books\n", "");
    let doc = GOOD.replace("### ", "## ");
    let got = errors(&schema, &doc);
    assert!(
        got.contains(
            &"heading `## 2 Shelf: Prose` is out of place; the sections are in the order the schema lists them"
                .to_string()
        ),
        "{got:?}"
    );
}

/// `Nested` describes one generated subsection: whether it is optional,
/// whether it pairs with its shelf's books, whether it takes its number from
/// the shelf, and the states in which the shelf and the subsection apply.
#[derive(Debug, Clone)]
struct Nested {
    optional: bool,
    paired: bool,
    numbered: bool,
    shelf_gate: Option<&'static str>,
    index_gate: Option<&'static str>,
}

fn arb_nested() -> impl Strategy<Value = Nested> {
    let gate = || prop::sample::select(vec![None, Some("draft"), Some("done")]);
    (any::<bool>(), any::<bool>(), any::<bool>(), gate(), gate()).prop_map(
        |(optional, paired, numbered, shelf_gate, index_gate)| Nested {
            optional,
            paired,
            numbered,
            shelf_gate,
            index_gate,
        },
    )
}

/// `nested_schema` returns a schema of shelves, each holding a subsection
/// whose options come from `n`.
fn nested_schema(n: &Nested) -> String {
    let gate =
        |g: Option<&str>| g.map_or_else(String::new, |g| format!("    only: {{ state: {g} }}\n"));
    let mut s = String::from(
        "---\ntype: Schema\nid: schema-shelf\ndescription: Shelves, and a subsection inside each.\n---\n\n````yaml\nline-limit: 0\nlifecycle:\n  - { key: state, states: [draft, done] }\nfrontmatter:\n  type:\n    const: Shelf\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        rule-id: shelves\n        each:\n          pattern: '^(?P<k>[A-Z][a-z]+)$'\n          key: k\n          example: Poetry\n  - heading: { pattern: '^\\d+ Shelf: (?P<k>[A-Z][a-z]+)$', key: k, example: '1 Shelf: Poetry' }\n    level: 2\n    repeat: true\n    for-each-key-of: shelves\n    rule-id: shelf\n",
    );
    s.push_str(&gate(n.shelf_gate));
    s.push_str("    blocks:\n      - kind: bullet-list\n        each:\n          rule-id: books\n          pattern: '^book (?P<k>[a-z]+)$'\n          key: k\n          example: book odes\n");
    s.push_str("  - heading: { pattern: '^\\d+\\.\\d+ Index$', example: '1.1 Index' }\n    level: 3\n    parent: shelf\n");
    s.push_str(&gate(n.index_gate));
    if n.numbered {
        s.push_str("    number-from-parent: true\n");
    }
    if n.optional {
        s.push_str("    optional: true\n");
    }
    s.push_str("    blocks:\n      - kind: bullet-list\n        each:\n");
    if n.paired {
        s.push_str("          pattern: '^index (?P<k>[a-z]+)$'\n          key: k\n          example: index odes\n          for-each-key-of: books\n");
    } else {
        s.push_str("          pattern: '^index [a-z]+$'\n          example: index odes\n");
    }
    s.push_str("````\n");
    s
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(48))]

    /// The templates that sokf derives from a schema with a subsection
    /// validate against that schema, and `sokf new` writes a valid document
    /// in each state.
    #[test]
    fn a_derivation_with_a_parent_passes_its_own_schema(n in arb_nested()) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/shelf.md", &nested_schema(&n));
        let report = tree.validate(false, CacheMode::Off);
        let mine: Vec<String> = on(&report.errors, "shelf.md")
            .map(|f| said(&f.message))
            .collect();
        prop_assert!(mine.is_empty(), "{mine:?}");
        for state in ["draft", "done"] {
            let made = new(&tree.root(), &NewOptions {
                cwd: tree.dir().join("knowledge"),
                id: Some(format!("shelf-{state}")),
                states: vec![("state".into(), state.to_string())],
                write: true,
                ..new_options(tree.dir(), "Shelf")
            });
            prop_assert!(made.is_ok(), "{:?}", made.as_ref().err());
            if let Ok(made) = made {
                tree.drop_guidance(made.path.as_str().trim_start_matches('/'));
            }
        }
        let after = tree.validate(true, CacheMode::Off);
        prop_assert!(after.errors.is_empty(), "{}", common::json(&after));
    }
}
