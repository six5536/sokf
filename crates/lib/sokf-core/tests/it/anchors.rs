//! These tests cover the anchor that every key has: `--fix` and `sokf new`
//! write it, validation checks that it is placed where its key is, no rule
//! reads it, and every fragment refers to it by its slug.

use common::{TempTree, json, lined, new_options};
use proptest::prelude::*;
use sokf_core::{CacheMode, NewOptions, new};

use crate::common;

/// `TERM_SCHEMA` captures keys in the three places an anchor goes: terms in
/// a bullet list, nodes in a diagram, and parts in headings. It prohibits
/// the pattern `<a `, so a document that passes shows that no rule reads an
/// anchor.
const TERM_SCHEMA: &str = r"---
type: Schema
id: schema-term
description: Terms, the nodes of a diagram, and parts.
---

````yaml
line-limit: 0
prohibited-patterns:
  - pattern: '<a '
    message: 'The body contains no HTML anchor.'
frontmatter:
  type:
    const: Term
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        each:
          pattern: '^`(?P<term>[^`]+)` - '
          key: term
          unique: knowledge
          example: '`Short code` - Seven letters.'
      - kind: code
        lang: text
        each:
          lines:
            - template: 'node ${name:slug}'
              key: name
              example: node alpha
  - heading: { template: 'Part: ${part:slug}', key: part, example: 'Part: one' }
    level: 2
    repeat: true
    blocks:
      - kind: prose
        example: The first part.
````
";

const GOOD: &str = "---\ntype: Term\nid: terms\ntitle: Terms\n---\n\n# Terms\n\n- <a id=\"short-code\"></a>`Short code` - Seven letters.\n- <a id=\"visit\"></a>`Visit` - One opening of [a part](#one).\n\n<a id=\"alpha\"></a><a id=\"beta\"></a>\n\n```text\nnode alpha\nnode beta\n```\n\n## <a id=\"one\"></a>Part: one\n\nThe first part.\n";

/// `bare` returns `GOOD` with every anchor removed.
fn bare() -> String {
    GOOD.replace("<a id=\"short-code\"></a>", "")
        .replace("<a id=\"visit\"></a>", "")
        .replace("<a id=\"alpha\"></a><a id=\"beta\"></a>\n\n", "")
        .replace("<a id=\"one\"></a>", "")
}

fn tree(doc: &str) -> TempTree {
    TempTree::clean_with(&[
        ("knowledge/schemas/term.md", TERM_SCHEMA),
        ("knowledge/terms.md", doc),
    ])
}

/// `errors` validates the tree and returns every error on the terms
/// document or its schema, each as `line: message`.
fn errors(tree: &TempTree, fix: bool) -> Vec<String> {
    tree.validate_all(fix, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| matches!(f.path.file_name(), "terms.md" | "term.md"))
        .map(lined)
        .collect()
}

// sokf-tests: SOKF_FORMAT_83
// sokf-tests: SOKF_FORMAT_84
#[test]
fn an_anchored_document_passes_and_no_rule_reads_an_anchor() {
    let t = tree(GOOD);
    assert!(errors(&t, false).is_empty(), "{:?}", errors(&t, false));
}

// sokf-tests: SOKF_FORMAT_83
// sokf-tests: SOKF_VALIDATION_28
#[test]
fn a_missing_anchor_is_reported_where_its_key_is_and_fix_writes_it() {
    let t = tree(&bare());
    assert_eq!(
        errors(&t, false),
        vec![
            "9: key `Short code` has no anchor; its repair writes `<a id=\"short-code\"></a>` before it",
            "10: key `Visit` has no anchor; its repair writes `<a id=\"visit\"></a>` before it",
            "12: key `alpha` has no anchor; its repair writes `<a id=\"alpha\"></a>` before it",
            "12: key `beta` has no anchor; its repair writes `<a id=\"beta\"></a>` before it",
            "17: key `one` has no anchor; its repair writes `<a id=\"one\"></a>` before it",
        ]
    );
    assert_eq!(errors(&t, true), [] as [std::string::String; 0]);
    assert_eq!(t.read("knowledge/terms.md"), GOOD);
}

// sokf-tests: SOKF_FORMAT_83
// sokf-tests: SOKF_FORMAT_84
// sokf-tests: SOKF_VALIDATION_28
// sokf-tests: SOKF_VALIDATION_29
#[test]
fn a_misplaced_an_orphan_and_a_repeated_anchor_are_reported_and_fixed() {
    let doc = GOOD
        .replace(
            "- <a id=\"visit\"></a>`Visit` - One",
            "- `Visit` - <a id=\"visit\"></a>One",
        )
        .replace("The first part.", "The first <a id=\"gone\"></a>part.")
        .replace("# Terms\n", "# Terms <a id=\"one\"></a>\n");
    let t = tree(&doc);
    assert_eq!(
        errors(&t, false),
        vec![
            "7: the anchor `#one` occurs more than once; its repair keeps one",
            "10: the anchor `#visit` is not where its key `Visit` is; its repair moves it",
            "21: the anchor `#gone` names no key of this document; its repair removes it",
        ]
    );
    assert_eq!(errors(&t, true), [] as [std::string::String; 0]);
    assert_eq!(t.read("knowledge/terms.md"), GOOD);
}

// sokf-tests: SOKF_FORMAT_83
#[test]
fn an_anchor_in_code_is_text() {
    let doc = GOOD.replace(
        "The first part.",
        "The first part, written `<a id=\"one\"></a>`.",
    );
    let t = tree(&doc);
    assert!(errors(&t, false).is_empty(), "{:?}", errors(&t, false));
}

// sokf-tests: SOKF_FORMAT_72
// sokf-tests: SOKF_FORMAT_73
// sokf-tests: SOKF_VALIDATION_32
#[test]
fn a_fragment_names_its_anchor_by_slug_and_fix_rewrites_the_text_form() {
    let doc = GOOD.replace(
        "The first part.",
        "The first part, after [a visit][sokf:terms#Visit] and [a term](#Short code).",
    );
    let doc = doc.replace("(#Short code)", "(#Short%20code)");
    let t = tree(&doc);
    assert_eq!(
        errors(&t, false),
        vec![
            "0: the definition block is missing; run `sokf validate --fix`",
            "21: `#Short%20code` matches no heading or key in this document",
            "21: `[sokf:terms#Visit]` gives the fragment as text, not as a slug. Use `#visit`, which `--fix` writes for you",
        ]
    );
    let t = tree(&GOOD.replace(
        "The first part.",
        "The first part, after [a visit][sokf:terms#Visit] and [a part](#One).",
    ));
    assert_eq!(
        errors(&t, false),
        vec![
            "0: the definition block is missing; run `sokf validate --fix`",
            "21: `#One` uses the heading text. Use the slug `#one` instead (`--fix` makes this change)",
            "21: `[sokf:terms#Visit]` gives the fragment as text, not as a slug. Use `#visit`, which `--fix` writes for you",
        ]
    );
    assert_eq!(errors(&t, true), [] as [std::string::String; 0]);
    let fixed = t.read("knowledge/terms.md");
    assert!(
        fixed.contains("[a visit][sokf:terms#visit] and [a part](#one)."),
        "{fixed}"
    );
    assert!(
        fixed.ends_with("[sokf:terms#visit]: /knowledge/terms.md#visit\n"),
        "{fixed}"
    );
}

// sokf-tests: SOKF_FORMAT_68
#[test]
fn two_keys_or_a_key_and_a_heading_may_not_share_a_slug() {
    let doc = GOOD.replace(
        "- <a id=\"visit\"></a>`Visit` - One",
        "- <a id=\"visit\"></a>`Visit` - One\n- `visit` - Two",
    );
    let t = tree(&doc);
    assert_eq!(
        errors(&t, false),
        vec![
            "11: key `visit` has the slug `visit` of key `Visit` at line 10; each key needs a slug of its own"
        ]
    );
    // `Part one` and the heading `Part: one` both have the slug `part-one`.
    let doc = GOOD.replace(
        "- <a id=\"visit\"></a>`Visit` - One",
        "- <a id=\"visit\"></a>`Visit` - One\n- `Part one` - Two",
    );
    let got: Vec<String> = errors(&tree(&doc), false)
        .into_iter()
        .filter(|e| !e.contains("has no anchor"))
        .collect();
    assert_eq!(
        got,
        vec![
            "11: key `Part one` has the slug `part-one` of the heading at line 20; keys and headings share one namespace"
        ]
    );
}

// A repeated heading takes the slug that GitHub gives it, so a link to the
// second `Note: two` resolves as `#note-two-1`, and a key with that slug
// clashes with it.
// sokf-tests: SOKF_FORMAT_155
// sokf-tests: SOKF_FORMAT_68
#[test]
fn a_repeated_heading_is_linked_by_its_suffix_and_a_key_may_not_take_it() {
    // The schema gains a section of notes, whose headings capture no key.
    let with_notes = |doc: &str| {
        let t = tree(doc);
        t.write(
            "knowledge/schemas/term.md",
            &TERM_SCHEMA.replace(
                "\n````\n",
                "\n  - heading: { template: 'Note: ${}', example: 'Note: two' }\n    level: 2\n    repeat: true\n    blocks:\n      - kind: prose\n        example: The note.\n````\n",
            ),
        );
        t
    };
    let doc = format!(
        "{GOOD}\n## Note: two\n\nOne [link](#note-two-1).\n\n## Note: two\n\nThe second.\n"
    );
    let t = with_notes(&doc);
    assert!(errors(&t, false).is_empty(), "{:?}", errors(&t, false));
    let doc = doc.replace(
        "- <a id=\"visit\"></a>`Visit` - One",
        "- <a id=\"visit\"></a>`Visit` - One\n- <a id=\"note-two-1\"></a>`Note two 1` - Two",
    );
    let got: Vec<String> = errors(&with_notes(&doc), false);
    assert_eq!(
        got,
        vec![
            "11: key `Note two 1` has the slug `note-two-1` of the heading at line 28; keys and headings share one namespace"
        ],
        "{got:?}"
    );
}

// An anchor belongs to the document that captures its key, so an include
// copies the body without its anchors.
// sokf-tests: SOKF_FORMAT_85
#[test]
fn an_include_leaves_the_anchors_out() {
    let t = TempTree::fixture("clean");
    let overview = t.read("knowledge/overview.md");
    assert!(
        t.read("knowledge/req-cli.md")
            .contains("<a id=\"cli-1\"></a>")
    );
    let interior = overview
        .split("<!-- sokf:include req-cli -->")
        .nth(1)
        .unwrap();
    assert!(!interior.contains("<a id="), "{overview}");
    let report = t.validate_all(false, CacheMode::Off);
    assert!(report.errors.is_empty(), "{}", json(&report));
}

// sokf-tests: SOKF_FORMAT_84
// sokf-tests: CLI-6_AC-8
#[test]
fn a_new_document_carries_its_anchors() {
    let t = tree(GOOD);
    let made = new(
        &t.root(),
        &NewOptions {
            cwd: t.dir().join("knowledge"),
            id: Some("more".to_string()),
            write: true,
            ..new_options(t.dir(), "Term")
        },
    )
    .unwrap();
    for anchor in [
        "- <a id=\"short-code\"></a>`Short code` -",
        "<a id=\"alpha\"></a>\n\n```text",
        "## <a id=\"one\"></a>Part: one",
    ] {
        assert!(made.document.contains(anchor), "{}", made.document);
    }
    let rel = made.path.to_string();
    t.drop_guidance(rel.trim_start_matches('/'));
    // Validation reports two expected errors: a duplicate term, because the
    // new document repeats the terms of `terms.md`, and a stale index, because
    // the index doesn't list the new document yet. It reports no others.
    let report = t.validate_all(false, CacheMode::Off);
    let other: Vec<&String> = report
        .errors
        .iter()
        .map(|f| &f.message)
        .filter(|m| !m.contains("is also captured by") && !m.contains("the index is stale"))
        .collect();
    assert!(other.is_empty(), "{other:?}");
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(48))]

    /// Whatever anchors are missing, moved, or added, one run of `--fix`
    /// writes exactly the anchor that each key needs, and a second run
    /// changes nothing.
    // sokf-tests: FORMAT_P-12
    #[test]
    fn fix_writes_each_anchor_once_and_is_a_fixed_point(
        drop in prop::collection::vec(any::<bool>(), 4),
        orphan in any::<bool>(),
        moved in any::<bool>(),
    ) {
        let mut doc = GOOD.to_string();
        let anchors = [
            "<a id=\"short-code\"></a>",
            "<a id=\"visit\"></a>",
            "<a id=\"alpha\"></a><a id=\"beta\"></a>\n\n",
            "<a id=\"one\"></a>",
        ];
        for (a, d) in anchors.iter().zip(&drop) {
            if *d {
                doc = doc.replacen(a, "", 1);
            }
        }
        if orphan {
            doc = doc.replace("The first part.", "The first <a id=\"stray\"></a>part.");
        }
        if moved && !drop[1] {
            doc = doc.replace("<a id=\"visit\"></a>`Visit` - ", "`Visit` - <a id=\"visit\"></a>");
        }
        let t = tree(&doc);
        prop_assert!(errors(&t, true).is_empty());
        prop_assert_eq!(t.read("knowledge/terms.md"), GOOD);
        let again = t.validate_all(true, CacheMode::Off);
        prop_assert!(
            !json(&again).contains("terms.md"),
            "{}",
            json(&again)
        );
    }
}
