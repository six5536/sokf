//! When a repetition of a group fails, the engine reports why. The engine
//! reports the findings of every repetition the group needs to reach its
//! minimum. It also reports the findings of a later repetition when no block
//! after the group matches that repetition's blocks. Those blocks stay in the
//! group.

use common::{TempTree, on, said};
use sokf_core::CacheMode;

use crate::common;

/// The schema of a document whose section holds captioned diagrams and can
/// end with a closing paragraph.
const FIGURE_SCHEMA: &str = r"---
type: Schema
id: schema-figure
description: Captioned diagrams, and a closing paragraph.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Figure
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: group
        min: 1
        max: { hard: MAX, when-at-limit: 'Each figure past the limit goes in a document of its own.' }
        blocks:
          - kind: prose
            max: { hard: 1, when-at-limit: 'Merge the captions of the diagram.' }
            each:
              pattern: '^Figure: '
              example: 'Figure: the parts.'
          - kind: code
            lang: text
            each:
              lines-open: true
              lines:
                - select-pattern: '->'
                  min: 1
                  example: 'a -> b'
      - kind: prose
        optional: true
        each:
          pattern: '^Note: '
          example: 'Note: the end.'
````
";

fn errors(max: &str, body: &str) -> Vec<String> {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/figure.md",
        &FIGURE_SCHEMA.replace("MAX", max),
    );
    tree.write(
        "knowledge/figure.md",
        &format!("---\ntype: Figure\nid: figure\ntitle: Figure\n---\n\n# Figure\n\n{body}"),
    );
    on(&tree.validate(false, CacheMode::Off).errors, "figure.md")
        .map(|f| said(&f.message))
        .collect()
}

const GOOD: &str = "Figure: one.\n\n```text\na -> b\n```\n";

#[test]
fn a_needed_repetition_missing_its_opening_says_so() {
    assert_eq!(
        errors("3", "```text\na -> b\n```\n"),
        vec!["expected a prose block; found a code block"]
    );
}

#[test]
fn a_needed_repetition_that_fails_gives_its_reason() {
    assert_eq!(
        errors("3", "Figure: one.\n\n```text\na b\n```\n"),
        vec!["the lines matching `->`: at least 1 is needed; there are 0"]
    );
}

#[test]
fn a_later_repetition_nothing_else_takes_gives_its_reason() {
    assert_eq!(
        errors("3", &format!("{GOOD}\n```text\na -> b\n```\n")),
        vec!["expected a prose block; found a code block"]
    );
    assert_eq!(
        errors("3", &format!("{GOOD}\nFigure: two.\n\n```text\na b\n```\n")),
        vec!["the lines matching `->`: at least 1 is needed; there are 0"]
    );
}

#[test]
fn a_block_a_later_position_takes_stays_there() {
    assert_eq!(
        errors("3", &format!("{GOOD}\nNote: the end.\n")),
        [] as [std::string::String; 0]
    );
}

#[test]
fn a_refused_group_sets_aside_every_block_it_spanned() {
    assert_eq!(
        errors("1", &format!("{GOOD}\n{GOOD}")),
        vec![
            "the group block allows at most 1 unit; it has 2. When at the limit: Each figure past the limit goes in a document of its own."
        ]
    );
}
