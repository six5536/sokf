//! These tests check how validation matches a run of units against a
//! schema's rules: a patch to a shared format, a ban on a pattern, a run of
//! paragraphs that two rules divide between them, a bound on the number of
//! units a rule takes, and a key that must stay unique across documents.

use common::{TempTree, json, lined, on, said};
use sokf_core::CacheMode;

use crate::common;

#[test]
fn a_format_patch_states_only_its_difference() {
    let tree = TempTree::fixture("clean");
    tree.take_requirement_formats();
    // The schema patches the shared story format and drops its key, because
    // the heading holds the key. A null in the patch removes the field, as
    // in an RFC 7386 merge patch, so the patch also removes the template.
    tree.write(
        "knowledge/schemas/story.md",
        "---\ntype: Schema\nid: schema-story\ndescription: A story with no key of its own.\n---\n\n````yaml\nline-limit: 0\nformats:\n  user-story:\n    patch: true\n    key: null\n    template: null\n    pattern: '^AS AN? .+, I WANT .+, SO THAT .+\\.$'\n    example: \"AS A user, I WANT a thing, SO THAT a benefit.\"\nfrontmatter:\n  type:\n    const: Story\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        each:\n          format: user-story\n````\n",
    );
    // The patch inherits a ban, and that ban still applies. Validation doesn't
    // demand the key that the patch removed.
    tree.write(
        "knowledge/story-a.md",
        "---\ntype: Story\nid: story-a\ntitle: Alpha\n---\n\n# Alpha\n\nAS A user, I WANT a thing, SO THAT the system SHALL do it.\n",
    );
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "story-a.md")
        .map(|f| said(&f.message))
        .collect();
    assert_eq!(
        messages,
        vec![
            "`SHALL` is refused: A user story states a need, so it holds no SHALL, SHOULD, or MUST. A paragraph of this kind reads `AS A user, I WANT a thing, SO THAT a benefit.`"
        ]
    );
    // Validation reports a patch of a format that no lower layer defines.
    tree.write(
        "knowledge/schemas/ghost.md",
        "---\ntype: Schema\nid: schema-ghost\ndescription: A patch of nothing.\n---\n\n````yaml\nline-limit: 0\nformats:\n  nowhere:\n    patch: true\n    example: x\nfrontmatter:\n  type:\n    const: Ghost\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n````\n",
    );
    let ghost: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "ghost.md")
        .map(sokf_core::report_message)
        .collect();
    assert_eq!(
        ghost,
        vec!["format `nowhere` patches no farther entry (TL_41)"]
    );
}

#[test]
fn a_prohibition_skips_a_code_span_across_lines() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/bold.md",
        "---\ntype: Schema\nid: schema-bold\ndescription: No bold text.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Bold\nprohibited-patterns:\n  - pattern: '\\*\\*'\n    message: The body contains no bold text.\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        example: 'One line of prose.'\n````\n",
    );
    // The code span opens on one line and closes on the next.
    tree.write(
        "knowledge/bold-a.md",
        "---\ntype: Bold\nid: bold-a\ntitle: Alpha\n---\n\n# Alpha\n\nA glob `/**/ and\nmore **stars**` quoted, then a **bold** word.\n",
    );
    let reported: Vec<usize> = on(&tree.validate(false, CacheMode::Off).errors, "bold-a.md")
        .filter_map(|f| f.line)
        .collect();
    assert_eq!(reported, vec![10]);
}

const RUN_SCHEMA: &str = r"---
type: Schema
id: schema-run
description: A lead paragraph, an optional quote, a label, a list.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Run
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        max: { hard: 1, when-at-limit: 'Put the detail in the list.' }
        each:
          pattern: '^[A-Z][a-z]'
          example: 'The lead.'
      - kind: blockquote
        example: 'One quoted line.'
        optional: true
        max: { hard: 1, when-at-limit: 'Put the rest of the reason in the lead.' }
      - kind: prose
        max: { hard: 1, when-at-limit: 'Remove the repeated CRITERIA line.' }
        each:
          text: CRITERIA
      - kind: bullet-list
        example: 'One item.'
````
";

#[test]
fn consecutive_prose_rules_divide_one_run() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/run.md", RUN_SCHEMA);
    // With the quote between them, the label is a block of its own.
    tree.write(
        "knowledge/run-a.md",
        "---\ntype: Run\nid: run-a\ntitle: Alpha\n---\n\n# Alpha\n\nThe lead.\n\n> Why.\n\nCRITERIA\n\n- one\n",
    );
    // Without it, the lead and the label are one run of two paragraphs.
    tree.write(
        "knowledge/run-b.md",
        "---\ntype: Run\nid: run-b\ntitle: Beta\n---\n\n# Beta\n\nThe lead.\n\nCRITERIA\n\n- one\n",
    );
    // A third paragraph belongs to no rule.
    tree.write(
        "knowledge/run-c.md",
        "---\ntype: Run\nid: run-c\ntitle: Gamma\n---\n\n# Gamma\n\nThe lead.\n\nCRITERIA\n\nOne paragraph too many.\n\n- one\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let messages: Vec<String> = report
        .errors
        .iter()
        .filter(|f| f.path.file_name().starts_with("run-"))
        .map(|f| {
            format!(
                "{}:{}: {}",
                f.path.file_name(),
                f.line.unwrap_or(0),
                said(&f.message)
            )
        })
        .collect();
    // Only the third document fails, and one error reports the paragraph
    // that belongs to no rule. The list after it is still the list the
    // schema declares, so nothing is reported against it.
    assert_eq!(
        messages,
        vec![
            "run-c.md:13: `One paragraph too many.` remains after the last prose position that takes the run"
        ],
        "{}",
        json(&report)
    );
}

#[test]
fn lines_govern_an_html_block_and_bounds_hold_over_units() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/page.md",
        "---\ntype: Schema\nid: schema-page\ndescription: An html block and a bounded list.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Page\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: html\n        each:\n          lines-open: true\n          lines:\n            - select-pattern: '<script'\n              rule-id: script\n              min: 0\n              max: { hard: 0, when-at-limit: 'Remove the script: the page runs no code.' }\n      - kind: bullet-list\n        rules:\n          - pattern: '^NOTE: '\n            min: 2\n            example: 'NOTE: one'\n````\n",
    );
    // The line rule for `<script` allows no such line, so a block with no
    // script, such as the template's, passes. The `NOTE: ` rule on the
    // list demands at least two items.
    tree.write(
        "knowledge/page-a.md",
        "---\ntype: Page\nid: page-a\ntitle: Alpha\n---\n\n# Alpha\n\n<div>\n<script>alert(1)</script>\n</div>\n\n- NOTE: only one\n",
    );
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "page-a.md")
        .map(lined)
        .collect();
    assert_eq!(
        messages,
        vec![
            "10: the lines matching `<script`: at most 0 are allowed; there is 1. When at the limit: Remove the script: the page runs no code.",
            "13: the items matching `^NOTE: `: at least 2 are needed; there is 1"
        ]
    );
}

#[test]
fn a_free_block_order_never_divides_a_run() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/free.md",
        "---\ntype: Schema\nid: schema-free\ndescription: Two prose rules, order free.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Free\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks-ordered: false\n    blocks:\n      - kind: prose\n        example: 'One line of prose.'\n        max: { hard: 1, when-at-limit: 'Put the label in a block of its own.' }\n      - kind: prose\n        max: { hard: 1, when-at-limit: 'Remove the repeated LABEL line.' }\n        optional: true\n        each:\n          text: LABEL\n````\n",
    );
    // One run of two paragraphs stays one block, so the label never reaches
    // the second rule and the block holds one paragraph too many.
    tree.write(
        "knowledge/free-a.md",
        "---\ntype: Free\nid: free-a\ntitle: Alpha\n---\n\n# Alpha\n\nThe lead.\n\nLABEL\n",
    );
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "free-a.md")
        .map(|f| said(&f.message))
        .collect();
    assert_eq!(
        messages,
        vec![
            "the prose block allows at most 1 paragraph; it has 2. When at the limit: Put the label in a block of its own."
        ]
    );
}

#[test]
fn a_paired_key_may_be_unique_beyond_its_document() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/comp.md",
        "---\ntype: Schema\nid: schema-comp\ndescription: One section per component, keys unique repository-wide.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Comp\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        rule-id: components\n        each:\n          pattern: '^(?P<k>[a-z][a-z0-9-]*):'\n          key: k\n          example: 'parser: reads the source'\n          unique: repository\n  - heading: { pattern: '^(?P<k>[a-z][a-z0-9-]*) detail$', key: k, unique: repository, example: parser detail }\n    guidance: One section per component.\n    level: 2\n    repeat: true\n    for-each-key-of: components\n    blocks: []\n````\n",
    );
    tree.write(
        "knowledge/comp-a.md",
        "---\ntype: Comp\nid: comp-a\ntitle: Alpha\n---\n\n# Alpha\n\n- logging: writes the log\n- config: reads the settings\n\n## logging detail\n\n## config detail\n",
    );
    let clean = tree.validate(false, CacheMode::Off);
    let mine: Vec<&str> = on(&clean.errors, "comp-a.md")
        .map(|f| f.message.as_str())
        .collect();
    assert!(mine.is_empty(), "{}", json(&clean));
    // A second document that uses the same keys clashes with the first, and
    // validation reports the clash in both documents.
    tree.write(
        "knowledge/comp-b.md",
        "---\ntype: Comp\nid: comp-b\ntitle: Beta\n---\n\n# Beta\n\n- logging: writes the log\n\n## logging detail\n",
    );
    let clashing = tree.validate(false, CacheMode::Off);
    let paths: Vec<String> = clashing
        .errors
        .iter()
        .filter(|f| f.message.contains("is also captured by"))
        .map(|f| f.path.file_name().to_string())
        .collect();
    assert_eq!(
        paths,
        vec!["comp-a.md", "comp-a.md", "comp-b.md", "comp-b.md"],
        "{}",
        json(&clashing)
    );
}

#[test]
fn fix_repairs_a_paragraph_that_came_from_a_divided_run() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/run.md", RUN_SCHEMA);
    // The lead and the label are one run, and the matcher divides it into two
    // halves. The label that the repair rewrites is the second half.
    tree.write(
        "knowledge/run-d.md",
        "---\ntype: Run\nid: run-d\ntitle: Delta\n---\n\n# Delta\n\nThe lead.\n\ncriteria\n\n- one\n",
    );
    let fixed = tree.validate(true, CacheMode::Off);
    assert_eq!(
        tree.read("knowledge/run-d.md"),
        "---\ntype: Run\nid: run-d\ntitle: Delta\n---\n\n# Delta\n\nThe lead.\n\nCRITERIA\n\n- one\n"
    );
    assert!(
        fixed
            .errors
            .iter()
            .all(|f| f.path.file_name() != "run-d.md"),
        "{}",
        json(&fixed)
    );
    // The repair is idempotent: a second run reports nothing for the
    // document.
    let again = tree.validate(false, CacheMode::Off);
    assert!(
        again
            .errors
            .iter()
            .all(|f| f.path.file_name() != "run-d.md"),
        "{}",
        json(&again)
    );
}

/// This schema declares three bans. The ban on the whole document skips code.
/// A second ban applies to the rule for the code fence, and a third applies to the
/// format that the bullet list uses.
const BANNED_SCHEMA: &str = r"---
type: Schema
id: schema-banned
description: Bans at the document, on a fence and through a format.
---

````yaml
line-limit: 0
prohibited-patterns: [{pattern: '\bTODO\b', message: 'A finished document contains no TODO.'}]
frontmatter:
  type:
    const: Banned
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: code
        lang: text
        each:
          prohibited-patterns: [{pattern: '\bTODO\b', message: 'A finished document contains no TODO.'}]
          example: cargo test --workspace
      - kind: bullet-list
        each:
          format: user-story
````
";

// A ban on a unit rule applies to the text of the unit, fence included. A
// ban on a format applies to every line bound to the format. The
// document's own ban still skips code, so validation reports the fence
// once.
#[test]
fn a_unit_ban_and_a_format_ban_read_what_their_rule_governs() {
    let tree = TempTree::fixture("clean");
    tree.take_requirement_formats();
    tree.write("knowledge/schemas/banned.md", BANNED_SCHEMA);
    tree.write(
        "knowledge/banned-a.md",
        "---\ntype: Banned\nid: banned-a\ntitle: Alpha\n---\n\n# Alpha\n\n```text\nTODO pin the toolchain\n```\n\n- CLI-9: AS A user, I WANT a thing, SO THAT it SHALL work.\n",
    );
    let messages: Vec<String> = on(&tree.validate(false, CacheMode::Off).errors, "banned-a.md")
        .map(lined)
        .collect();
    assert_eq!(
        messages,
        vec![
            "9: `TODO` is refused: A finished document contains no TODO. A line of this kind reads `cargo test --workspace`",
            "13: `SHALL` is refused: A user story states a need, so it holds no SHALL, SHOULD, or MUST. An item of this kind reads `ST-1: AS A user, I WANT a click on a file name to open the file in the editor, SO THAT I go from a compiler error to the code in one click.`",
        ]
    );
}
