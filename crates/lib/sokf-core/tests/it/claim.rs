//! These tests check which block each schema position claims. In each test,
//! two or more positions in one section take blocks of the same kind, and
//! each position must claim the block that meets its own rules.

use common::{TempTree, json, lined, new_options, on, reported};
use sokf_core::{CacheMode, NewOptions, new};

use crate::common;

/// `SHAPES_SCHEMA` has a section for each of six shapes: an optional quote
/// before a required one, tagged code fences, a closing list, a repeated
/// group, quotes in free order, and a bounded run of paragraphs. In each
/// section, two positions take blocks of the same kind, so the validator
/// can't tell from the kind of a block alone which position claims it.
const SHAPES_SCHEMA: &str = r"---
type: Schema
id: schema-shapes
description: Each section holds a shape in which two positions take blocks of the same kind.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Shapes
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Optional Quote
    level: 2
    guidance: An optional note before a required warning.
    blocks:
      - kind: blockquote
        optional: true
        each:
          pattern: '^NOTE: '
          example: 'NOTE: one'
      - kind: blockquote
        each:
          pattern: '^WARNING: '
          example: 'WARNING: one'
  - heading: Tagged Code
    level: 2
    guidance: The inputs, then the output, told apart by their tag.
    blocks:
      - kind: code
        example: 'a: 1'
        repeat: true
        lang: [yaml]
      - kind: code
        example: 'done'
        lang: [text]
  - heading: Closing List
    level: 2
    guidance: The steps, then the close.
    blocks:
      - kind: bullet-list
        repeat: true
        each:
          pattern: '^STEP '
          example: 'STEP one'
      - kind: bullet-list
        each:
          pattern: '^DONE '
          example: 'DONE one'
  - heading: Grouped
    level: 2
    guidance: A claim and its evidence, then a closing paragraph.
    blocks:
      - kind: group
        repeat: true
        blocks:
          - kind: prose
            max: { hard: 1, when-at-limit: 'Open another group for the next claim.' }
            each:
              pattern: '^CLAIM '
              example: 'CLAIM one'
          - kind: blockquote
            example: 'One quoted line.'
      - kind: prose
        max: { hard: 1, when-at-limit: 'Put the rest in the preceding claims.' }
        each:
          pattern: '^CLOSE '
          example: 'CLOSE one'
  - heading: Free Order
    level: 2
    guidance: Two quotes whose order the schema frees.
    blocks-ordered: false
    blocks:
      - kind: blockquote
        each:
          pattern: '^NOTE: '
          example: 'NOTE: one'
      - kind: blockquote
        each:
          pattern: '^WARNING: '
          example: 'WARNING: one'
  - heading: Bounded Run
    level: 2
    guidance: An optional run of asides, then the claim.
    blocks:
      - kind: prose
        optional: true
        max: { hard: 2, when-at-limit: 'Fold the rest into the claim.' }
        each:
          pattern: '^ASIDE '
          example: 'ASIDE one'
      - kind: prose
        max: { hard: 1, when-at-limit: 'Write another claim in a document of its own.' }
        each:
          pattern: '^CLAIM '
          example: 'CLAIM one'
````
";

/// `SHAPES_BODY` is the body of the test document. It contains each of the
/// six shapes in a form that passes only when each position claims the
/// block that meets its own rules.
const SHAPES_BODY: &str = r"# Hard

## Optional Quote

> WARNING: mind the gap.

## Tagged Code

```yaml
a: 1
```

```yaml
b: 2
```

```text
done
```

## Closing List

- STEP one
- STEP two

* DONE all

## Grouped

CLAIM one.

> Because.

CLAIM two.

> Also.

CLOSE it.

## Free Order

> WARNING: mind the gap.

> NOTE: read on.

## Bounded Run

ASIDE one.

CLAIM two.
";

fn shapes(tree: &TempTree, body: &str) {
    tree.write("knowledge/schemas/shapes.md", SHAPES_SCHEMA);
    tree.write(
        "knowledge/hard.md",
        &format!("---\ntype: Shapes\nid: hard\ntitle: Hard\n---\n\n{body}"),
    );
}

#[test]
fn every_shape_a_position_claiming_on_kind_could_not_express() {
    let tree = TempTree::fixture("clean");
    shapes(&tree, SHAPES_BODY);
    let report = tree.validate(false, CacheMode::Off);
    let mine: Vec<&sokf_core::Finding> = report
        .errors
        .iter()
        .filter(|f| f.path.file_name() == "hard.md" || f.path.file_name() == "shapes.md")
        .collect();
    assert!(mine.is_empty(), "{}", json(&report));
}

#[test]
fn a_block_nobody_claims_is_still_one_too_many() {
    let tree = TempTree::fixture("clean");
    // The Closing List section declares two lists, and this body gives it a
    // third.
    shapes(
        &tree,
        &SHAPES_BODY.replace("* DONE all\n", "* DONE all\n\n- STEP three\n"),
    );
    let report = tree.validate(false, CacheMode::Off);
    let mine: Vec<String> = on(&report.errors, "hard.md")
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(
        mine,
        vec!["unexpected bullet-list block (schema schema-shapes, section \"Closing List\")"],
        "{}",
        json(&report)
    );
}

/// `STRAY_SCHEMA` declares one fence in each of its two sections and no
/// prose position anywhere. The test document puts a paragraph before the
/// fence in one section, and a paragraph with no fence in the other.
const STRAY_SCHEMA: &str = r"---
type: Schema
id: schema-stray
description: One fence per section, and no prose position anywhere.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Stray
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: In The Way
    level: 2
    guidance: The fence, whatever stands before it.
    blocks:
      - kind: code
        example: 'done'
        lang: [text]
  - heading: Nothing Fits
    level: 2
    guidance: The fence, where the body holds none.
    blocks:
      - kind: code
        example: 'done'
        lang: [text]
````
";

// A stray paragraph is reported once, at its own line, and the fence after
// it still satisfies its rule. When no fence follows the paragraph, the
// single error states both the kind the position expects and the kind it
// found.
#[test]
fn a_block_in_the_way_is_reported_once_and_the_position_waits() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/stray.md", STRAY_SCHEMA);
    tree.write(
        "knowledge/stray.md",
        "---\ntype: Stray\nid: stray\ntitle: Stray\n---\n\n# Stray\n\n## In The Way\n\nA paragraph in the way.\n\n```text\nkept\n```\n\n## Nothing Fits\n\nA paragraph and no fence.\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let mine: Vec<String> = on(&report.errors, "stray.md").map(lined).collect();
    assert_eq!(
        mine,
        vec![
            "11: unexpected prose block".to_string(),
            "19: expected a code block; found a prose block".to_string(),
        ],
        "{}",
        json(&report)
    );
}

/// `REFUSAL_SCHEMA` has a position that refuses a block of its own kind,
/// and a position whose broken rule raises a warning rather than an error.
const REFUSAL_SCHEMA: &str = r"---
type: Schema
id: schema-refusal
description: A refused block, and a warning that keeps its block.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Refusal
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Tagged
    level: 2
    guidance: The inputs, then the output.
    blocks:
      - kind: code
        example: 'a: 1'
        repeat: true
        lang: [yaml]
      - kind: code
        example: 'done'
        lang: [text]
  - heading: Warned
    level: 2
    guidance: A note that need not end in a period, then the warning.
    blocks:
      - kind: blockquote
        optional: true
        each:
          pattern: '\.$'
          example: 'One.'
          severity: warn
      - kind: blockquote
        each:
          pattern: '^WARNING: '
          example: 'WARNING: one'
````
";

#[test]
fn a_refusal_says_why_and_a_warning_keeps_its_block() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/refusal.md", REFUSAL_SCHEMA);
    tree.write(
        "knowledge/refused.md",
        "---\ntype: Refusal\nid: refused\ntitle: Refused\n---\n\n# Refused\n\n## Tagged\n\n```text\ndone\n```\n\n```yaml\na: 1\n```\n\n## Warned\n\n> A note without a period\n\n> WARNING: mind the gap.\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let errors: Vec<String> = on(&report.errors, "refused.md")
        .map(|f| f.message.clone())
        .collect();
    // The error for the refused block states the reason, and doesn't say that
    // the position expected a block of the kind it declined.
    assert_eq!(
        errors,
        vec![
            "the code block is tagged `text`; expected `yaml` (schema schema-refusal, section \"Tagged\" block 1)",
            "unexpected code block (schema schema-refusal, section \"Tagged\")",
        ],
        "{}",
        json(&report)
    );
    // The warning is reported, and the block it was raised on stays
    // claimed: a released block would be one too many.
    let warnings: Vec<String> = on(&report.warnings, "refused.md")
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(
        warnings,
        vec![
            "`A note without a period` does not match `\\.$`; a paragraph of this kind reads One. (schema schema-refusal, section \"Warned\" block 1)"
        ],
        "{}",
        json(&report)
    );
}

/// `LEAK_SCHEMA` has an optional position with a fixed `text` that cannot
/// accept the document's list. If the repair for that `text` survived the
/// refusal, it would rewrite the item that the next position claims and
/// takes its key from.
const LEAK_SCHEMA: &str = r#"---
type: Schema
id: schema-leak
description: An optional keyed position that refuses every block.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Leak
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        optional: true
        each:
          text: "NEVER-1: this line"
      - kind: bullet-list
        each:
          template: '${id:requirement-id}: the one item'
          key: id
          unique: repository
          example: "AC-1: the one item"
````
"#;

#[test]
fn a_refusal_leaves_no_capture_and_no_repair() {
    let tree = TempTree::fixture("clean");
    tree.take_requirement_formats();
    tree.write("knowledge/schemas/leak.md", LEAK_SCHEMA);
    // The item satisfies the second position and not the first, so the
    // first position refuses it. Its `text` would produce a repair, and the
    // validator must discard that repair with the refusal. A leaked repair
    // would rewrite the item to the text of the declining position. The
    // validator captures a key only from text that has the position's shape,
    // so the refused position captures no key.
    let doc = "---\ntype: Leak\nid: leaked\ntitle: Leaked\n---\n\n# Leaked\n\n- <a id=\"ac-1\"></a>AC-1: the one item\n";
    tree.write("knowledge/leaked.md", doc);
    let report = tree.validate(true, CacheMode::Off);
    assert!(report.errors.is_empty(), "{}", json(&report));
    // The fix adds the new files to the indexes and leaves the document
    // unchanged.
    assert!(!json(&report).contains("leaked.md"), "{}", json(&report));
    assert_eq!(tree.read("knowledge/leaked.md"), doc);
}

/// `BOUNDED_SCHEMA` declares an optional lead of up to four paragraphs,
/// then the claim. This shape makes a position give back part of a run.
const BOUNDED_SCHEMA: &str = r"---
type: Schema
id: schema-bounded
description: An optional lead bounded at four, then the claim.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Bounded
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        optional: true
        max: { hard: 4, when-at-limit: 'Fold the rest into the claim.' }
        each:
          pattern: '^ASIDE '
          example: 'ASIDE one'
      - kind: prose
        max: { hard: 1, when-at-limit: 'Write another claim in a document of its own.' }
        each:
          pattern: '^CLAIM '
          example: 'CLAIM one'
````
";

fn bounded(tree: &TempTree, name: &str, body: &str) {
    tree.write("knowledge/schemas/bounded.md", BOUNDED_SCHEMA);
    tree.write(
        &format!("knowledge/{name}.md"),
        &format!("---\ntype: Bounded\nid: {name}\ntitle: Doc\n---\n\n# Doc\n\n{body}"),
    );
}

fn messages(report: &sokf_core::Report, name: &str) -> Vec<String> {
    report
        .errors
        .iter()
        .filter(|f| f.path.file_name() == format!("{name}.md"))
        .map(|f| format!("{}: {}", f.line.unwrap_or(0), f.message.clone()))
        .collect()
}

#[test]
fn a_lead_takes_the_paragraphs_it_accepts_and_no_more() {
    let tree = TempTree::fixture("clean");
    // The lead can take four paragraphs and accepts three, so the claim
    // gets the fourth: the position gives back the paragraph it can't
    // accept.
    bounded(
        &tree,
        "given-back",
        "ASIDE one.\n\nASIDE two.\n\nASIDE three.\n\nCLAIM four.\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        messages(&report, "given-back").is_empty(),
        "{}",
        json(&report)
    );
}

#[test]
fn a_position_that_must_claim_takes_the_rest_of_the_run() {
    let tree = TempTree::fixture("clean");
    // No paragraph after the lead is a claim, so the required position
    // takes the rest of the run and reports each paragraph, and its `max`,
    // which decides no extent, reports the count.
    bounded(
        &tree,
        "no-claim",
        "ASIDE one.\n\nNEITHER two.\n\nNEITHER three.\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(
        messages(&report, "no-claim"),
        vec![
            "11: `NEITHER two.` does not match `^CLAIM `; a paragraph of this kind reads CLAIM one (schema schema-bounded, section \"<title>\" block 2)".to_string(),
            "11: the prose block allows at most 1 paragraph; it has 2. When at the limit: Write another claim in a document of its own. (schema schema-bounded, section \"<title>\" block 2)".to_string(),
            "13: `NEITHER three.` does not match `^CLAIM `; a paragraph of this kind reads CLAIM one (schema schema-bounded, section \"<title>\" block 2)".to_string(),
        ],
        "{}",
        json(&report)
    );
}

/// `GROUP_RUN_SCHEMA` declares a group whose inner positions divide a run
/// of paragraphs. A repetition that can't complete leaves the run divided
/// unless the validator puts it back together. `repeat` is redundant on a
/// group and must stay harmless.
const GROUP_RUN_SCHEMA: &str = r"---
type: Schema
id: schema-grouprun
description: A group whose inner positions divide a run of paragraphs.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: GroupRun
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: group
        repeat: true
        blocks:
          - kind: prose
            max: { hard: 1, when-at-limit: 'Open another group for the next A line.' }
            each:
              pattern: '^A '
              example: 'A one'
          - kind: prose
            max: { hard: 1, when-at-limit: 'Open another group for the next B line.' }
            each:
              pattern: '^B '
              example: 'B one'
````
";

#[test]
fn a_repetition_that_fails_puts_the_run_back_together() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/grouprun.md", GROUP_RUN_SCHEMA);
    // The document holds one whole repetition, then two paragraphs that
    // can't make another. The second repetition divides the run before it
    // fails. Its opening paragraph belongs to the group, so the group keeps
    // it. The error on the next paragraph says why the repetition failed.
    // The validator never reports the run piece by piece as separate groups.
    tree.write(
        "knowledge/grouprun-a.md",
        "---\ntype: GroupRun\nid: grouprun-a\ntitle: Alpha\n---\n\n# Alpha\n\nA one.\n\nB two.\n\nA three.\n\nC four.\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(
        messages(&report, "grouprun-a"),
        vec![
            "15: `C four.` does not match `^B `; a paragraph of this kind reads B one (schema schema-grouprun, section \"<title>\" block 1.2)".to_string()
        ],
        "{}",
        json(&report)
    );
}

/// `STORIES_SCHEMA` is a copy of the Stories schema as it stood when these
/// tests were written, because the repository's own copy now comes from a
/// pack and changes with it. Its stories put an optional `Planned:` paragraph
/// and an optional blockquote between two required kinds of paragraph.
const STORIES_SCHEMA: &str = include_str!("../fixtures/schemas/stories.md");

// A story that puts its blockquote before its `Planned:` paragraph draws
// one error, which names the paragraph as out of place, and the
// `Contracts:` and `Flows:` paragraphs after it still reach their own
// positions and draw none.
#[test]
fn an_optional_block_before_an_earlier_one_is_reported_out_of_place() {
    let tree = TempTree::fixture("clean");
    tree.take_requirement_formats();
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    let made = new(
        &tree.root(),
        &NewOptions {
            slug: Some("assets".into()),
            write: true,
            ..new_options(tree.dir(), "Stories")
        },
    )
    .unwrap();
    let rel = made.path.to_string().trim_start_matches('/').to_string();
    tree.drop_guidance(&rel);
    let text = tree
        .read(&rel)
        .replace("<!-- sokf:optional -->\n\n", "")
        .replace("<!-- /sokf:optional -->\n\n", "");
    let planned = text.find("Planned: ").unwrap();
    let quote = text[planned..].find("> ").unwrap() + planned;
    let end = text[quote..].find("\n\n").unwrap() + quote + 2;
    let moved = format!(
        "{}{}{}{}",
        &text[..planned],
        &text[quote..end],
        &text[planned..quote],
        &text[end..]
    );
    tree.write(&rel, &moved);
    // The fixture holds none of the documents that the story links, so the
    // test leaves out the errors of those links.
    let report = tree.validate(false, CacheMode::Off);
    let mine: Vec<String> = on(&report.errors, "stories-assets.md")
        .map(reported)
        .filter(|f| !f.ends_with("(SOKF_FORMAT_61)"))
        .collect();
    assert_eq!(
        mine,
        vec![
            "22: the prose block is out of place; the blocks are in the order the schema lists them (schema schema-stories, section \"${id:story-id} ${:article} ${:user} can ${}\" block 2) (TLMD_169)",
        ],
        "{moved}"
    );
}
