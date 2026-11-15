//! These property tests check the grammar rules: the lines a guard governs,
//! the keys a keyed format captures, the code spans a prohibition skips, the
//! way the block rules divide a run of blocks, the keys a citing type must
//! cite, and the guidance markers a template writes. Each test checks what a
//! rule governs and what the validator reports.

use std::fmt::Write as _;

use common::{TempTree, json, on};
use proptest::prelude::*;
use sokf_core::CacheMode;

use crate::common;

/// `PROBE_SCHEMA` is a schema whose line rule selects `SEL` lines and fails
/// every one of them, so a finding stands for a selected line and nothing
/// else.
const PROBE_SCHEMA: &str = r"---
type: Schema
id: schema-probe
description: Every selected line fails, so findings count selections.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Probe
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: code
        lang: text
        example: plain
        each:
          lines:
            - select-pattern: '^SEL'
              min: 0
              pattern: 'NEVERMATCHESANYTHING'
              example: 'SEL NEVERMATCHESANYTHING'
````
";

/// `arb_lines` generates the content lines of a fence: some that the guard
/// selects and some that it doesn't, none of them blank or a fence marker.
fn arb_lines() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            "SEL[a-z]{0,4}".prop_map(|s| s.clone()),
            "[a-z]{1,5}".prop_map(|s| s.clone()),
        ],
        1..12,
    )
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    /// A guard governs the lines that match it and no others. Each failure
    /// reports the line of the file that the guard read.
    #[test]
    fn a_guard_governs_its_lines_and_names_them(lines in arb_lines()) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/probe.md", PROBE_SCHEMA);
        let body = lines.join("\n");
        let doc = format!(
            "---\ntype: Probe\nid: probe-a\ntitle: Alpha\n---\n\n# Alpha\n\n```text\n{body}\n```\n"
        );
        tree.write("knowledge/probe-a.md", &doc);
        let report = tree.validate(false, CacheMode::Off);
        let reported: Vec<usize> = report
            .errors
            .iter()
            .filter(|f| f.path.file_name() == "probe-a.md" && f.message.contains("does not match `NEVERMATCHESANYTHING`"))
            .filter_map(|f| f.line)
            .collect();
        // The fence opens on line 9, so its first content line is 10.
        let expected: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with("SEL"))
            .map(|(i, _)| 10 + i)
            .collect();
        prop_assert_eq!(reported, expected, "{}", json(&report));
    }
}

/// `CAPTURE_SCHEMA` binds its criteria to the shared Easy Approach to
/// Requirements Syntax (EARS) format. It requires its second section once
/// for each key that the criteria capture. The test
/// therefore sees a captured key as a missing section, and a criterion that
/// captures no key as a finding at its line.
const CAPTURE_SCHEMA: &str = r"---
type: Schema
id: schema-capture
description: Criteria, and one section per key they capture.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Capture
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        rule-id: criteria
        each:
          format: requirement-ears
  - heading: { pattern: '^Covered: (?P<k>.+)$', key: k, example: 'Covered: text' }
    guidance: One section per criterion covered.
    level: 2
    optional: true
    repeat: true
    for-each-key-of: criteria
    blocks: []
````
";

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(128))]

    /// When the format captures a key from a criterion, the missing-section
    /// finding reports that key. When the format captures no key, the
    /// validator reports that the criterion doesn't match the format. Each
    /// criterion gets exactly one of these two findings.
    #[test]
    fn a_bound_unit_is_captured_or_reported(shapes in prop::collection::vec(any::<bool>(), 1..8)) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/capture.md", CAPTURE_SCHEMA);
        let items: Vec<String> = shapes
            .iter()
            .enumerate()
            .map(|(i, well_formed)| {
                let id = if *well_formed {
                    format!("EX-1_AC-{}", i + 1)
                } else {
                    format!("EX1_AC-{}", i + 1)
                };
                format!("- {id} [ubiquitous]: the system SHALL do a thing.")
            })
            .collect();
        let body = items.join("\n");
        tree.write(
            "knowledge/capture-a.md",
            &format!("---\ntype: Capture\nid: capture-a\ntitle: Alpha\n---\n\n# Alpha\n\n{body}\n"),
        );
        let report = tree.validate(false, CacheMode::Off);
        let mine: Vec<&str> = on(&report.errors, "capture-a.md")
            .map(|f| f.message.as_str())
            .collect();
        let captured = mine.iter().filter(|m| m.contains("section for key `")).count();
        let reported = mine
            .iter()
            .filter(|m| m.contains("matches none of the forms") || m.contains("does not match the"))
            .count();
        let well_formed = shapes.iter().filter(|b| **b).count();
        prop_assert_eq!(captured, well_formed, "{:?}", mine);
        prop_assert_eq!(reported, shapes.len() - well_formed);
        prop_assert_eq!(mine.len(), shapes.len());
    }
}

/// `BOLD_SCHEMA` bans bold anywhere in a body.
const BOLD_SCHEMA: &str = r"---
type: Schema
id: schema-bold
description: No bold text.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Bold
prohibited-patterns:
  - pattern: '\*\*'
    message: 'The body contains no bold text.'
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        example: 'One item.'
````
";

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(128))]

    /// A prohibition reads a line with its code spans blanked, so a
    /// pattern quoted as code doesn't count as prose. A finding still
    /// reports the line that the prohibition read.
    #[test]
    fn a_prohibition_does_not_read_a_code_span(quoted in prop::collection::vec(any::<bool>(), 1..6)) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/bold.md", BOLD_SCHEMA);
        let items: Vec<String> = quoted
            .iter()
            .map(|in_code| {
                if *in_code {
                    "- a `/**/` glob".to_string()
                } else {
                    "- a **bold** word".to_string()
                }
            })
            .collect();
        let body = items.join("\n");
        tree.write(
            "knowledge/bold-a.md",
            &format!("---\ntype: Bold\nid: bold-a\ntitle: Alpha\n---\n\n# Alpha\n\n{body}\n"),
        );
        let report = tree.validate(false, CacheMode::Off);
        let reported: Vec<usize> = on(&report.errors, "bold-a.md")
            .filter_map(|f| f.line)
            .collect();
        // The list starts on line 9.
        let expected: Vec<usize> = quoted
            .iter()
            .enumerate()
            .filter(|(_, in_code)| !**in_code)
            .map(|(i, _)| 9 + i)
            .collect();
        prop_assert_eq!(reported, expected, "{}", json(&report));
    }
}

/// The body of `RUN_SCHEMA` is a lead paragraph, an optional quote, a
/// labelled paragraph, and a list. In this shape, the rules can divide a
/// coalesced run of paragraphs in more than one way.
const RUN_SCHEMA: &str = r"---
type: Schema
id: schema-run
description: A lead, an optional quote, a label, a list.
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
        example: 'One line of prose.'
        max: 1
      - kind: blockquote
        example: 'One quoted line.'
        optional: true
        max: 1
      - kind: prose
        max: 1
        each:
          text: CRITERIA
      - kind: bullet-list
        example: 'One item.'
````
";

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(128))]

    /// However many paragraphs a run holds, one rule matches each paragraph
    /// or the validator reports it once. The shape passes with the quote and
    /// without it. A finding comes only from a paragraph that no rule has
    /// room for, because each rule already holds its maximum.
    #[test]
    fn dividing_a_run_conserves_its_paragraphs(quote in any::<bool>(), extra in 0usize..3) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/run.md", RUN_SCHEMA);
        let mut body = String::from("The lead.\n\n");
        if quote {
            body.push_str("> Why.\n\n");
        }
        body.push_str("CRITERIA\n\n");
        for i in 0..extra {
            let _ = write!(body, "Spare paragraph {i}.\n\n");
        }
        body.push_str("- one\n");
        tree.write(
            "knowledge/run-a.md",
            &format!("---\ntype: Run\nid: run-a\ntitle: Alpha\n---\n\n# Alpha\n\n{body}"),
        );
        let report = tree.validate(false, CacheMode::Off);
        let mine = on(&report.errors, "run-a.md")
            .count();
        if extra == 0 {
            prop_assert_eq!(mine, 0);
        } else {
            prop_assert!(mine > 0);
        }
    }
}

/// In `RUN_BOUNDED_SCHEMA`, two prose rules share one run of paragraphs: an
/// optional lead of at most two paragraphs, then the claim.
const RUN_BOUNDED_SCHEMA: &str = r"---
type: Schema
id: schema-bounded
description: An optional bounded lead, then the claim.
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
        max: 2
        each:
          pattern: '^ASIDE '
          example: 'ASIDE one'
      - kind: prose
        max: 1
        each:
          pattern: '^CLAIM '
          example: 'CLAIM one'
````
";

/// `REFUSE_KEY_SCHEMA` has two list rules. The first is optional, captures a
/// key, and accepts no list. The second follows it, accepts every list, and
/// captures the same key.
const REFUSE_KEY_SCHEMA: &str = r"---
type: Schema
id: schema-refusekey
description: An optional keyed position that refuses every block.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: RefuseKey
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        optional: true
        each:
          pattern: '^(?P<k>[A-Z]+-\d+): .*NEVER$'
          key: k
          example: 'REQ-1: NEVER'
          unique: repository
      - kind: bullet-list
        each:
          pattern: '^(?P<k>[A-Z]+-\d+): '
          key: k
          example: 'REQ-1: one'
          unique: repository
````
";

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(128))]

    /// Whatever the run holds, the validator finds a way to divide it that
    /// keeps within each rule's bounds and matches each rule's pattern. A
    /// lead of zero to two asides followed by one claim passes. The only
    /// finding comes from paragraphs after the claim. No rule has room for
    /// them.
    #[test]
    fn a_division_the_positions_accept_is_found(leads in 0usize..3, spare in 0usize..3) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/bounded.md", RUN_BOUNDED_SCHEMA);
        let mut body = String::new();
        for i in 0..leads {
            let _ = write!(body, "ASIDE {i}.\n\n");
        }
        body.push_str("CLAIM one.\n\n");
        for i in 0..spare {
            let _ = write!(body, "Spare {i}.\n\n");
        }
        tree.write(
            "knowledge/bounded-a.md",
            &format!("---\ntype: Bounded\nid: bounded-a\ntitle: Alpha\n---\n\n# Alpha\n\n{body}"),
        );
        let report = tree.validate(false, CacheMode::Off);
        let mine: Vec<String> = on(&report.errors, "bounded-a.md")
            .map(|f| f.message.clone())
            .collect();
        if spare == 0 {
            prop_assert!(mine.is_empty(), "{mine:?}");
        } else {
            // The spares are one run, so they are one block too many.
            prop_assert_eq!(mine.len(), 1, "{:?}", mine);
            prop_assert!(mine[0].contains("remains after the last prose position that takes the run"), "{:?}", mine);
        }
    }

    /// However many items the list holds, the rule that refused the list
    /// leaves nothing behind. If it left anything, its keys would clash with
    /// the keys of the rule that accepted the list, and its repairs would
    /// rewrite the list.
    #[test]
    fn a_refusal_leaves_nothing_behind(items in 1usize..4) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/refusekey.md", REFUSE_KEY_SCHEMA);
        let mut body = String::new();
        for i in 0..items {
            let _ = writeln!(body, "- <a id=\"ac-{0}\"></a>AC-{0}: item {i}", i + 1);
        }
        tree.write(
            "knowledge/refusekey-a.md",
            &format!("---\ntype: RefuseKey\nid: refusekey-a\ntitle: Alpha\n---\n\n# Alpha\n\n{body}"),
        );
        let report = tree.validate(true, CacheMode::Off);
        let mine: Vec<String> = on(&report.errors, "refusekey-a.md")
            .map(|f| f.message.clone())
            .collect();
        prop_assert!(mine.is_empty(), "{mine:?}");
        prop_assert!(!json(&report).contains("refusekey-a.md"), "{}", json(&report));
    }
}

/// The body of `GUIDED_SCHEMA` is paragraphs, so a guidance line can stand
/// anywhere between them.
const GUIDED_SCHEMA: &str = r"---
type: Schema
id: schema-guided
description: Paragraphs, and nothing else.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Guided
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The cache is rebuilt on the next run.
````
";

/// `COVERED_SCHEMA` is a schema for a type whose keys a `Stories` document
/// must cite.
const COVERED_SCHEMA: &str = r"---
type: Schema
id: schema-covered
description: A story cites every key, or the validator reports the key.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Covered
cited-by:
  - from-type: Stories
    keys: keys
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        rule-id: keys
        each:
          pattern: '^(?P<id>K-\d+): .+$'
          key: id
          unique: knowledge
          example: 'K-1: a key'
````
";

/// `CITER_SCHEMA` is a schema for the citing type, whose body is free prose.
const CITER_SCHEMA: &str = r"---
type: Schema
id: schema-stories
description: Stories that cite keys.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Stories
sections:
  - heading: { from: title }
    level: 1
````
";

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    /// For any set of captured keys, and any subset of them that a document
    /// of the required type cites, the uncited-key findings are exactly the
    /// complement: one finding for each uncited key, at the key's line.
    // sokf-tests: EXT_P-2
    // sokf-tests: EXT-9_AC-1
    #[test]
    fn the_uncited_keys_are_exactly_the_complement(
        cited in prop::collection::vec(any::<bool>(), 1..9),
    ) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/covered.md", COVERED_SCHEMA);
        tree.write("knowledge/schemas/stories.md", CITER_SCHEMA);
        let mut lines: Vec<String> = vec![
            "---".into(), "type: Covered".into(), "id: covered-a".into(), "title: Alpha".into(),
            "---".into(), String::new(), "# Alpha".into(), String::new(),
        ];
        let mut expected: Vec<(usize, String)> = Vec::new();
        let mut citations: Vec<String> = Vec::new();
        for (i, is_cited) in cited.iter().enumerate() {
            let key = format!("K-{}", i + 1);
            lines.push(format!("- {key}: a key"));
            if *is_cited {
                citations.push(format!("[{key}][sokf:#k-{}]", i + 1));
            } else {
                expected.push((lines.len(), format!("no `Stories` document cites key `{key}`")));
            }
        }
        tree.write("knowledge/covered-a.md", &(lines.join("\n") + "\n"));
        let body = if citations.is_empty() { "Cites nothing.".to_string() } else { format!("Cites {}.", citations.join(", ")) };
        tree.write(
            "knowledge/stories-a.md",
            &format!("---\ntype: Stories\nid: stories-a\ntitle: S\n---\n\n# S\n\n{body}\n"),
        );
        let report = tree.validate(false, CacheMode::Off);
        let reported: Vec<(usize, String)> = on(&report.errors, "covered-a.md")
            .map(|f| (f.line.unwrap_or(0), common::said(&f.message)))
            .collect();
        prop_assert_eq!(reported, expected, "{}", json(&report));
    }

    /// Guidance never passes. Each `sokf:guidance` line that a document
    /// keeps, in its body or its frontmatter, is one error at that line. A
    /// repair leaves every such line as it was.
    // sokf-tests: FORMAT_P-4
    // sokf-tests: FORMAT-3_AC-1
    // sokf-tests: FORMAT-3_AC-2
    #[test]
    fn guidance_is_an_error_a_repair_keeps(
        parts in prop::collection::vec(any::<bool>(), 1..8),
        in_frontmatter in any::<bool>(),
    ) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/guided.md", GUIDED_SCHEMA);
        let mut lines: Vec<String> = vec!["---".into(), "type: Guided".into(), "id: guided-a".into()];
        if in_frontmatter {
            lines.push("# sokf:guidance The title a reader searches for.".into());
        }
        lines.extend(["title: Alpha".into(), "---".into(), String::new(), "# Alpha".into(), String::new()]);
        // A paragraph always opens the body, so the prose block is there.
        lines.push("The index is rebuilt on every run.".into());
        lines.push(String::new());
        for (i, guidance) in parts.iter().enumerate() {
            lines.push(if *guidance {
                format!("<!-- sokf:guidance Part {i} says why. -->")
            } else {
                format!("Paragraph {i} of the note.")
            });
            lines.push(String::new());
        }
        let doc = lines.join("\n");
        let expected: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains("sokf:guidance"))
            .map(|(i, _)| i + 1)
            .collect();
        tree.write("knowledge/guided-a.md", &doc);
        let report = tree.validate(true, CacheMode::Off);
        let reported: Vec<usize> = report
            .errors
            .iter()
            .filter(|f| f.path.file_name() == "guided-a.md" && f.message.contains("sokf:guidance"))
            .filter_map(|f| f.line)
            .collect();
        prop_assert_eq!(reported, expected, "{}", json(&report));
        let after = tree.read("knowledge/guided-a.md");
        let kept: Vec<&str> = after.lines().filter(|l| l.contains("sokf:guidance")).collect();
        let before: Vec<&str> = doc.lines().filter(|l| l.contains("sokf:guidance")).collect();
        prop_assert_eq!(kept, before);
    }
}
