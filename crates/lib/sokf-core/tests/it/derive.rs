//! When a schema declares no `templates`, sokf derives them from the
//! schema's rules, and checks the derived scaffold as it checks an
//! authored one.

// sokf-tests: CLI-6_AC-1

use common::{TempTree, json, new_options, on};
use sokf_core::{CacheMode, NewOptions, new};

use crate::common;

/// `DERIVED_SCHEMA` declares no `templates` at all. It has a lifecycle, a
/// gated section, a bound format, a section for each captured key, a
/// guarded list, and a fence that sokf reads line by line.
const DERIVED_SCHEMA: &str = r#"---
type: Schema
id: schema-derived
title: Derived Schema
description: Every part of the template comes from a rule.
---

````yaml
line-limit: 0
lifecycle:
  - { key: state, states: [draft, done] }
frontmatter:
  type:
    const: Derived
  id:
    template: "derived-${:seq}-${:slug}"
  owner:
    pattern: '^[a-z]+$'
    guidance: The shape this rule demands.
    example: someone
  tags:
    list: true
    optional: true
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Summary
    level: 2
    blocks:
      - kind: prose
        example: 'One line of prose.'
        max: 1
  - heading: Criteria
    level: 2
    rule-id: criteria
    blocks:
      - kind: bullet-list
        each:
          format: requirement-ears
  - heading: { template: 'Covered: ${key}', key: key, example: "Covered: EX-1_AC-1" }
    level: 2
    repeat: true
    for-each-key-of: criteria
    blocks: []
  - heading: Shape
    level: 2
    blocks:
      - kind: code
        lang: mermaid
        each:
          lines:
            - pattern: '^flowchart'
              example: flowchart LR
            - pattern: '\-\->'
              example: '  one -->|"calls"| two'
      - kind: table
        columns: [Name, Meaning]
        each:
          example: one | what it means
      - kind: blockquote
        example: 'One quoted line.'
        max: 1
  - heading: Outcome
    level: 2
    only: { state: done }
    blocks:
      - kind: prose
        max: 1
        each:
          example: What was done.
````
"#;

fn write_schema(tree: &TempTree, text: &str) {
    tree.write("knowledge/schemas/derived.md", text);
}

#[test]
fn a_schema_with_no_templates_derives_one_per_state_and_passes() {
    let tree = TempTree::fixture("clean");
    write_schema(&tree, DERIVED_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "derived.md"),
        "{}",
        json(&report)
    );
}

// `sokf new` copies the scaffold, so a document that it creates from a
// schema with no template passes validation.
// sokf-tests: CLI-6_AC-1
#[test]
fn new_creates_a_document_from_the_derived_template() {
    let tree = TempTree::fixture("clean");
    write_schema(&tree, DERIVED_SCHEMA);
    for state in ["draft", "done"] {
        let result = new(
            &tree.root(),
            &NewOptions {
                slug: Some(format!("thing-{state}")),
                states: vec![("state".into(), state.into())],
                write: true,
                ..new_options(tree.dir(), "Derived")
            },
        )
        .unwrap();
        assert!(result.created, "{}", result.path);
        tree.drop_guidance(result.path.to_string().trim_start_matches('/'));
        assert!(
            !result.document.contains("sokf:only"),
            "{}",
            result.document
        );
        assert_eq!(result.document.contains("## Outcome"), state == "done");
    }
    let report = tree.validate(true, CacheMode::Off);
    assert!(report.errors.is_empty(), "{}", json(&report));
}

// When a block has no `example`, no bound format, and no `text`, the
// derivation fails with an error that names the kind of the block, because
// the template writes no text of its own.
#[test]
fn a_rule_the_derivation_cannot_satisfy_is_named() {
    let tree = TempTree::fixture("clean");
    write_schema(
        &tree,
        &DERIVED_SCHEMA.replace(
            "      - kind: blockquote\n        example: 'One quoted line.'\n",
            "      - kind: blockquote\n",
        ),
    );
    let report = tree.validate(false, CacheMode::Off);
    let messages: Vec<String> = on(&report.errors, "derived.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        messages,
        vec![
            "the schema derives no scaffold: a `blockquote` block has no `example`, and the scaffold writes no text of its own; give the block an `example` (TLMD_116)",
        ],
        "{}",
        json(&report)
    );
}

/// `CASCADE_SCHEMA` has a section and a block that both need an `example`.
const CASCADE_SCHEMA: &str = r#"---
type: Schema
id: schema-cascade
title: Cascade Schema
description: Two rules lack the example that a template needs.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Cascade
  id:
    template: "cascade-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Summary
    level: 2
    guidance: What the reader takes away.
  - heading: Notes
    level: 2
    blocks:
      - kind: bullet-list
````
"#;

// sokf reports each refusal on its own line, with the schema line that
// declares its rule. One pass reports every rule that fails, so an author
// who fixes one rule doesn't have to run the tool again to find the next.
#[test]
fn every_refusal_is_named_on_its_own_line_in_one_pass() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/cascade.md", CASCADE_SCHEMA);
    // The schema check reports both rules.
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "cascade.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        messages,
        vec![
            "the schema derives no scaffold: a section with no `blocks` holds one prose block, and the scaffold writes no text of its own; declare the block and its `example` (TLMD_116)",
            "the schema derives no scaffold: a `bullet-list` block has no `example`, and the scaffold writes no text of its own; give the block an `example` (TLMD_116)",
        ]
    );
    // `sokf new` reports the same refusals, one on each line, each with the
    // line of the schema it comes from, and then says how to fix them.
    let mut o = new_options(tree.dir(), "Cascade");
    o.slug = Some("thing".into());
    let said = new(&tree.root(), &o).unwrap_err().to_string();
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(
        lines[0],
        "new: the schema of `Cascade` derives no scaffold:"
    );
    assert!(
        lines[1].starts_with("  /knowledge/schemas/cascade.md:")
            && lines[1].ends_with("declare the block and its `example`"),
        "{said}"
    );
    assert!(
        lines[2].starts_with("  /knowledge/schemas/cascade.md:")
            && lines[2].ends_with("give the block an `example`"),
        "{said}"
    );
    assert_eq!(lines[3], "write the `example` each rule asks for");
}

/// `BOUND_SCHEMA` declares a type whose id is fixed and whose heading is
/// bound to `title`.
const BOUND_SCHEMA: &str = r"---
type: Schema
id: schema-bound
title: Bound Schema
description: The heading is the title, and the id fixes both.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Bound
  id:
    const: the-bound-note
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: 'One line of prose.'
        max: 1
````
";

// `sokf new` fills the title from the id. If the scaffold wrote any other
// title, the frontmatter and the heading bound to the title would
// disagree, so the derivation writes the title that `new` fills.
// sokf-tests: CLI-6_AC-1
#[test]
fn the_derived_title_is_the_one_new_fills() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/bound.md", BOUND_SCHEMA);
    let mut o = new_options(tree.dir(), "Bound");
    o.write = true;
    let made = new(&tree.root(), &o).unwrap();
    assert!(
        made.document.contains("title: The bound note")
            && made.document.contains("# The bound note"),
        "{}",
        made.document
    );
    tree.drop_guidance(made.path.to_string().trim_start_matches('/'));
    let report = tree.validate(true, CacheMode::Off);
    assert!(report.errors.is_empty(), "{}", json(&report));
}

/// `STATED_SCHEMA` gives an example of the whole fence of its diagram, and
/// its line rule captures a key from each box in it.
const STATED_SCHEMA: &str = r#"---
type: Schema
id: schema-stated
title: Stated Schema
description: The fence has an example, and the line rules read its keys.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Stated
  id:
    template: "stated-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Shape
    level: 2
    blocks:
      - kind: code
        lang: mermaid
        rule-id: boxes
        each:
          example: |
            flowchart LR
              one["One"]:::box
              two["Two"]:::box
              one --> two
          lines-open: true
          lines:
            - pattern: '^\w+\["(?P<name>[^"]+)"\]:::box$'
              key: name
              example: '  one["One"]:::box'
  - heading: { template: 'Box: ${name}', key: name, example: "Box: One" }
    level: 2
    repeat: true
    for-each-key-of: boxes
    guidance: One box expanded.
    blocks: []
````
"#;

// When the derivation builds a diagram one line at a time, a diagram of
// five classes becomes five copies of one node. A rule can give an example
// of the whole fence instead, and the template takes its keys from that
// example.
// sokf-tests: CLI-6_AC-8
#[test]
fn a_fence_the_schema_states_is_written_whole_and_gives_up_its_keys() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/stated.md", STATED_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "stated.md"),
        "{}",
        json(&report)
    );
    let mut o = new_options(tree.dir(), "Stated");
    o.slug = Some("thing".into());
    o.write = true;
    let made = new(&tree.root(), &o).unwrap();
    // The fence is the example that the rule gives, not one line for each
    // line rule.
    assert!(
        made.document
            .contains("flowchart LR\n  one[\"One\"]:::box\n  two[\"Two\"]:::box\n  one --> two\n"),
        "{}",
        made.document
    );
    // Each of the two keys in the fence has the section that the schema
    // demands, and that section carries the key's anchor (SOKF_FORMAT_84).
    assert!(
        made.document.contains("## <a id=\"one\"></a>Box: One")
            && made.document.contains("## <a id=\"two\"></a>Box: Two"),
        "{}",
        made.document
    );
    tree.drop_guidance(made.path.to_string().trim_start_matches('/'));
    let after = tree.validate(true, CacheMode::Off);
    assert!(after.errors.is_empty(), "{}", json(&after));
}

/// `OPTIONAL_SCHEMA` has three optional sections: two that the derivation
/// can write, and one whose rule gives it nothing to write.
const OPTIONAL_SCHEMA: &str = r#"---
type: Schema
id: schema-optional
title: Optional Schema
description: An optional part is a prompt, not a thing to leave out.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Opt
  id:
    template: "opt-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Notes
    level: 2
    optional: true
    guidance: What the reader should know.
    blocks:
      - kind: bullet-list
        example: 'One item.'
  - heading: Aside
    level: 2
    optional: true
    guidance: Anything the notes do not cover.
    blocks:
      - kind: prose
        example: 'One line of prose.'
  - heading: Shape
    level: 2
    optional: true
    guidance: A shape with nothing to write.
    blocks:
      - kind: prose
````
"#;

// A template is a sheet of prompts, so the derivation writes an optional
// part when it can. When a rule inside the part gives nothing to write,
// the derivation leaves the part out instead of refusing, and a warning
// describes the block whose rule kept it out.
// sokf-tests: CLI-6_AC-7
#[test]
fn an_optional_part_is_written_where_it_can_be_and_named_where_it_cannot() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/optional.md", OPTIONAL_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "optional.md"),
        "{}",
        json(&report)
    );
    let warned: Vec<String> = on(&report.warnings, "optional.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        warned,
        vec![
            "an optional part is left out of the scaffold: a `prose` block has no `example`, and the scaffold writes no text of its own; give the block an `example` (TLMD_124)",
        ]
    );
    let mut o = new_options(tree.dir(), "Opt");
    o.slug = Some("thing".into());
    o.write = true;
    let made = new(&tree.root(), &o).unwrap();
    assert!(!made.document.contains("## Shape"), "{}", made.document);
    // The part the derivation wrote carries its marker, so the reader of
    // the created document knows the schema does not demand it.
    assert!(
        made.document.contains(
            "<!-- sokf:optional -->\n\n<!-- sokf:guidance What the reader should know. -->\n## Notes"
        ),
        "{}",
        made.document
    );
    assert!(
        made.document.contains("\n<!-- /sokf:optional -->\n"),
        "{}",
        made.document
    );
    // Each optional section is a region of its own, so the author keeps
    // one and deletes the next.
    assert_eq!(made.document.matches("<!-- sokf:optional -->").count(), 2);
    assert_eq!(made.document.matches("<!-- /sokf:optional -->").count(), 2);
    assert!(made.document.contains("## Aside"), "{}", made.document);
    // A marker left in the document is untidy, not wrong: the document has
    // no errors, and validation gives one warning for each decision that
    // the author hasn't made.
    tree.drop_guidance(made.path.to_string().trim_start_matches('/'));
    let after = tree.validate(true, CacheMode::Off);
    assert!(after.errors.is_empty(), "{}", json(&after));
    let warned: Vec<String> = on(&after.warnings, "opt-thing.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        warned,
        vec![
            "a `sokf:optional` region is left here: keep what it holds and drop both markers, or drop the region (TLMD_11)",
            "a `sokf:optional` region is left here: keep what it holds and drop both markers, or drop the region (TLMD_11)",
        ]
    );
}

/// `GUIDED_SCHEMA` gives guidance at every level where it can: the schema,
/// a key, a link that the schema doesn't demand, and a section.
const GUIDED_SCHEMA: &str = r#"---
type: Schema
id: schema-guided
title: Guided Schema
description: The schema gives guidance at every level where it can stand.
---

````yaml
line-limit: 0
guidance: One decision, and the reason it was taken.
frontmatter:
  type:
    const: Guided
  id:
    template: "guided-${:slug}"
  owner:
    pattern: '^[a-z]+$'
    example: someone
    guidance: The person who answers for the decision.
links:
  - rel: relates-to
    min: 0
    guidance: A decision this one depends on.
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Reason
    level: 2
    guidance: What made the other options worse.
    blocks:
      - kind: prose
        example: The cache cannot hold a second copy of the index.
````
"#;

// The template writes each piece of guidance on a line of its own before
// the part it guides, and validation reports an error for each guidance
// line left in a created document.
// sokf-tests: FORMAT-3_AC-1
// sokf-tests: FORMAT-3_AC-2
// sokf-tests: EXT-8_AC-1
// sokf-tests: CLI-6_AC-7
#[test]
fn guidance_stands_before_what_it_guides_and_may_not_stay() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/guided.md", GUIDED_SCHEMA);
    let mut o = new_options(tree.dir(), "Guided");
    o.slug = Some("thing".into());
    o.write = true;
    let made = new(&tree.root(), &o).unwrap();
    let doc = &made.document;
    for wanted in [
        "title: Thing\n# sokf:guidance The person who answers for the decision.\nowner: someone\n",
        "links:\n  # sokf:guidance A decision this one depends on.\n  - rel: relates-to\n",
        "---\n\n<!-- sokf:guidance The writer replaces each of the following examples with their own text. One decision, and the reason it was taken. -->\n\n# Thing\n",
        "<!-- sokf:guidance What made the other options worse. -->\n## Reason\n",
    ] {
        assert!(doc.contains(wanted), "{wanted}\n---\n{doc}");
    }
    // Each guidance line left is an error at that line, and the repair pass
    // leaves the line in place.
    let rel = made.path.to_string().trim_start_matches('/').to_string();
    let report = tree.validate(true, CacheMode::Off);
    let left: Vec<usize> = report
        .errors
        .iter()
        .filter(|f| f.path.file_name() == "guided-thing.md" && f.message.contains("sokf:guidance"))
        .filter_map(|f| f.line)
        .collect();
    let lines: Vec<usize> = doc
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("sokf:guidance"))
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(left, lines, "{}", json(&report));
    assert_eq!(&tree.read(&rel), doc);
    // No error message quotes the text of the guidance.
    assert!(
        report
            .errors
            .iter()
            .all(|f| !f.message.contains("answers for")),
        "{}",
        json(&report)
    );
    tree.drop_guidance(&rel);
    let after = tree.validate(true, CacheMode::Off);
    assert!(
        after
            .errors
            .iter()
            .all(|f| !f.message.contains("sokf:guidance")),
        "{}",
        json(&after)
    );
}

/// `NESTED_SCHEMA` declares list items that each carry a nested list.
const NESTED_SCHEMA: &str = r#"---
type: Schema
id: schema-nested
title: Nested Schema
description: A list item carries a nested list below its own text.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Nest
  id:
    template: "nest-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Parts
    level: 2
    blocks:
      - kind: bullet-list
        each:
          template: '`${name}` ${}'
          key: name
          example: '`Lookup` finds the address of a code.'
          blocks:
            - kind: bullet-list
              each:
                template: 'Code: ${}'
                example: 'Code: lookup.go'
              max: 1
````
"#;

// The scaffold writes a list item's nested blocks under the item,
// indented to the item's text, so that a scaffold of a schema that demands
// nested blocks passes that schema.
#[test]
fn a_list_item_is_written_with_its_nested_blocks() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/nested.md", NESTED_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.file_name() != "nested.md"),
        "{}",
        json(&report)
    );
    let mut o = new_options(tree.dir(), "Nest");
    o.slug = Some("thing".into());
    let made = new(&tree.root(), &o).unwrap();
    assert!(
        made.document
            .contains("`Lookup` finds the address of a code.\n  - Code: lookup.go\n"),
        "{}",
        made.document
    );
}

/// `PORTS_SCHEMA` requires a table whose columns vary from document to
/// document, and names the header of its scaffold under `example-columns`.
const PORTS_SCHEMA: &str = r#"---
type: Schema
id: schema-ports
title: Ports Schema
description: A table whose columns each document chooses.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Ports
  id:
    template: "ports-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: table
        columns: []
        columns-open: true
        example-columns: [Port, Program]
        each:
          example: edit | acme
````
"#;

// A table with `columns: []` and `columns-open: true` takes the header of
// its scaffold from `example-columns`, so the scaffold is a table that
// passes the schema, and `sokf new` writes a document that validates.
// sokf-tests: CLI-6_AC-1
#[test]
fn a_table_with_no_columns_writes_the_header_of_its_example_columns() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/ports.md", PORTS_SCHEMA);
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(
        on(&report.errors, "ports.md").count(),
        0,
        "{}",
        json(&report)
    );
    let mut o = new_options(tree.dir(), "Ports");
    o.slug = Some("editors".into());
    o.write = true;
    let made = new(&tree.root(), &o).unwrap();
    assert!(
        made.document
            .contains("\n| Port | Program |\n| --- | --- |\n| edit | acme |\n"),
        "{}",
        made.document
    );
    tree.drop_guidance(made.path.to_string().trim_start_matches('/'));
    let report = tree.validate(true, CacheMode::Off);
    assert!(report.errors.is_empty(), "{}", json(&report));
}

// The schema check refuses `example-columns` beside declared columns, and
// an example row whose cells differ in number from `example-columns`,
// because the scaffold would otherwise write a header that the columns
// contradict, or rows that don't fit their header.
#[test]
fn example_columns_that_do_not_fit_the_table_are_refused() {
    for (from, to, code) in [
        ("columns: []", "columns: [Port, Program]", "TLMD_207"),
        (
            "example-columns: [Port, Program]",
            "example-columns: [Port]",
            "TLMD_208",
        ),
    ] {
        let tree = TempTree::fixture("clean");
        tree.write(
            "knowledge/schemas/ports.md",
            &PORTS_SCHEMA.replace(from, to),
        );
        let report = tree.validate(false, CacheMode::Off);
        let messages: Vec<String> = on(&report.errors, "ports.md")
            .map(sokf_core::report_message)
            .collect();
        assert!(
            messages.len() == 1
                && messages[0].contains("`example-columns`")
                && messages[0].ends_with(&format!("({code})")),
            "{}",
            json(&report)
        );
    }
}
