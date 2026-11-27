//! These tests cover the anchor that every key has: `--fix` and `sokf new`
//! write it, validation checks that it is placed where its key is, no rule
//! reads it, and every fragment refers to it by its slug.

use std::collections::BTreeSet;

use common::{
    TempTree, age,
    incremental::{cold, edit, run, set},
    json, lined, lines, new_options,
};
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
            "0: the definition block is missing",
            "21: `#Short%20code` matches no heading or key in this document",
            "21: `[sokf:terms#Visit]` gives the fragment as text, not as a slug. Use `#visit`",
        ]
    );
    let t = tree(&GOOD.replace(
        "The first part.",
        "The first part, after [a visit][sokf:terms#Visit] and [a part](#One).",
    ));
    assert_eq!(
        errors(&t, false),
        vec![
            "0: the definition block is missing",
            "21: `#One` uses the heading text. Use the slug `#one` instead",
            "21: `[sokf:terms#Visit]` gives the fragment as text, not as a slug. Use `#visit`",
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

/// `kept_links` returns the documents that a warm run checked and its
/// findings, after it checks that a cold run gives the same findings, and
/// the findings among them on a reference to `cli-1_ac-2`.
fn kept_links(tree: &TempTree) -> (BTreeSet<String>, Vec<String>, Vec<String>) {
    let (dirty, _, warm, _) = run(tree);
    assert_eq!(warm, cold(tree));
    let links = on_kept(&warm);
    (dirty, warm, links)
}

/// `on_kept` returns the findings on a link or a marker that names
/// `cli-1_ac-2`, and leaves out those on the anchor itself.
fn on_kept(findings: &[String]) -> Vec<String> {
    findings
        .iter()
        .filter(|f| f.contains("cli-1_ac-2") && !f.contains("the anchor `#cli-1_ac-2`"))
        .cloned()
        .collect()
}

// While an item breaks its rule, the engine keeps its anchor, so a link to
// the anchor from another document or from its own, and a marker that names
// it, resolve, whether the run reads the cache or not. A bare fragment names
// a key, so it doesn't resolve. A change that leaves the anchors as they are,
// but takes the anchor from the item, reaches the cache too.
// sokf-tests: SOKF_FORMAT_71
// sokf-tests: SOKF_FORMAT_73
// sokf-tests: SOKF_FORMAT_145
// sokf-tests: CACHE-1_AC-4
// sokf-tests: CACHE-1_AC-14
#[test]
fn a_link_to_the_anchor_of_a_broken_item_resolves_on_a_cold_and_a_warm_cache() {
    let tree = TempTree::fixture("clean");
    let decision = "knowledge/decisions/decision-single-binary.md";
    tree.write(
        decision,
        &tree.read(decision).replace(
            "the daemon is a subcommand.",
            "the daemon is a subcommand, as [the parse criterion][sokf:req-cli#cli-1_ac-2] needs and [its key][sokf:#cli-1_ac-2] names.",
        ),
    );
    let req = "knowledge/req-cli.md";
    tree.write(
        req,
        &tree.read(req).replace(
            "repetitive CLI arguments.",
            "repetitive CLI arguments, as [the parse criterion](#cli-1_ac-2) shows.",
        ),
    );
    let main = "src/main.rs";
    tree.write(
        main,
        &tree.read(main).replace(
            "fn main() {",
            // sokf:ignore-next-line
            "// sokf-implements: req-cli#cli-1_ac-2\nfn main() {",
        ),
    );
    let fixed = tree.validate_all(true, CacheMode::Off).errors;
    assert!(fixed.is_empty(), "{fixed:?}");
    age(tree.dir(), 10);
    let (_, warm, _) = kept_links(&tree);
    assert!(warm.is_empty(), "{warm:?}");
    let good = tree.read(req);
    let broken =
        |text: &str| text.replace("parse, THEN the system SHALL", "parse, the system will");
    edit(&tree, req, &broken(&good));
    let (_, warm, links) = kept_links(&tree);
    assert!(
        warm.iter()
            .any(|f| f.contains("does not match the `unwanted` form")),
        "{warm:?}"
    );
    let bare = |links: &[String]| {
        links.len() == 1 && links[0].contains("`[sokf:#cli-1_ac-2]` refers to a key")
    };
    assert!(bare(&links), "{links:?}");
    // The run checks the decision again and not the requirement, so the
    // link resolves against the kept anchor in the store.
    let text = tree.read(decision);
    edit(
        &tree,
        decision,
        &text.replace("the wrong one.", "the wrong one again."),
    );
    let (dirty, _, links) = kept_links(&tree);
    assert_eq!(dirty, set(&[&format!("/{decision}")]));
    assert!(bare(&links), "{links:?}");
    // A fix of the decision keeps the definition of the link.
    let text = tree.read(decision);
    edit(
        &tree,
        decision,
        &text.replace("one again.", "one once more."),
    );
    let fixed = lines(&tree.validate_all(true, CacheMode::On));
    assert!(bare(&on_kept(&fixed)), "{fixed:?}");
    assert!(
        tree.read(decision)
            .contains("[sokf:req-cli#cli-1_ac-2]: /knowledge/req-cli.md#cli-1_ac-2\n"),
        "{}",
        tree.read(decision)
    );
    // The item passes again with another key, and the anchor names no key.
    let renamed = good.replace("</a>CLI-1_AC-2 [unwanted]", "</a>CLI-1_AC-3 [unwanted]");
    edit(&tree, req, &renamed);
    let (_, _, links) = kept_links(&tree);
    assert_eq!(links.len(), 4, "{links:?}");
    // The item breaks its rule again, and it holds the anchor once more.
    edit(&tree, req, &broken(&renamed));
    let (_, _, links) = kept_links(&tree);
    assert!(bare(&links), "{links:?}");
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
    // The skeleton holds no part whose text an example gives, so it
    // captures no key and holds no anchor, and the guide gives each key in
    // its example.
    assert!(!made.document.contains("<a id="), "{}", made.document);
    let all = crate::common::examples(&made).join("\n");
    for key in ["`Short code` -", "```text", "## Part: one"] {
        assert!(all.contains(key), "{all}");
    }
    let report = t.validate_all(false, CacheMode::Off);
    assert_eq!(
        crate::common::not_missing(&report, made.path.as_str()),
        Vec::<String>::new(),
        "{}",
        crate::common::json(&report)
    );
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

// A key that holds emphasis is captured in tramlines' fixed spelling, and
// its anchor, and every fragment that names it, takes the slug of its plain
// text, whichever spelling the document writes.
// sokf-tests: SOKF_FORMAT_72
#[test]
fn a_key_with_emphasis_is_named_by_the_slug_of_its_plain_text() {
    let schema = TERM_SCHEMA
        .replace(
            "          pattern: '^`(?P<term>[^`]+)` - '\n",
            "          pattern: '^(?P<term>_[A-Z][a-z]+_) - '\n",
        )
        .replace(
            "          example: '`Short code` - Seven letters.'\n",
            "          example: '_Short_ - Five letters.'\n",
        );
    for term in ["*Short*", "_Short_"] {
        let doc = GOOD
            .replace(
                "- <a id=\"short-code\"></a>`Short code` - Seven letters.\n- <a id=\"visit\"></a>`Visit` - One opening of [a part](#one).\n",
                &format!("- {term} - Five letters, as [the term](#short) says.\n"),
            );
        let other = GOOD
            .replace("id: terms\ntitle: Terms", "id: others\ntitle: Others")
            .replace("# Terms", "# Others")
            .replace(
                "- <a id=\"short-code\"></a>`Short code` - Seven letters.\n- <a id=\"visit\"></a>`Visit` - One opening of [a part](#one).\n",
                "- <a id=\"long\"></a>_Long_ - Four letters, unlike [the short term][sokf:#short] of [the terms][sokf:terms#short].\n",
            );
        let t = TempTree::clean_with(&[
            ("knowledge/schemas/term.md", &schema),
            ("knowledge/terms.md", &doc),
            ("knowledge/others.md", &other),
        ]);
        let errors = |t: &TempTree, fix: bool| -> Vec<String> {
            t.validate_all(fix, CacheMode::Off)
                .errors
                .iter()
                .filter(|f| {
                    f.path.as_str().starts_with("/knowledge/")
                        && !f.path.as_str().ends_with("index.md")
                })
                .map(lined)
                .collect()
        };
        let found = errors(&t, true);
        assert!(found.is_empty(), "{term}: {found:?}");
        let fixed = t.read("knowledge/terms.md");
        assert!(
            fixed.contains(&format!("- <a id=\"short\"></a>{term} - Five letters")),
            "{fixed}"
        );
        let left = errors(&t, false);
        assert!(left.is_empty(), "{term}: {left:?}");
        let again = t.validate_all(true, CacheMode::Off);
        assert!(again.fixes.is_empty(), "{term}: {}", json(&again));
    }
}

// A key that a line of a fence captures has the slug of its text as
// written for its id, so a fragment that gives the key's text resolves to
// it and `--fix` writes that id, though the plain text of the key differs.
// sokf-tests: SOKF_FORMAT_72
// sokf-tests: SOKF_VALIDATION_32
#[test]
fn a_fragment_names_a_key_of_a_fence_by_the_slug_of_its_text() {
    let schema = TERM_SCHEMA.replace(
        "            - template: 'node ${name:slug}'\n",
        "            - template: 'node ${name}'\n",
    );
    let doc = GOOD
        .replace("node beta", "node Option<T>")
        .replace("<a id=\"beta\"></a>", "<a id=\"optiont\"></a>")
        .replace(
            "The first part.",
            "The first part, after [an option][sokf:terms#Option<T>].",
        );
    let t = TempTree::clean_with(&[
        ("knowledge/schemas/term.md", &schema),
        ("knowledge/terms.md", &doc),
    ]);
    assert_eq!(
        errors(&t, false),
        vec![
            "0: the definition block is missing",
            "21: `[sokf:terms#Option<T>]` gives the fragment as text, not as a slug. Use `#optiont`",
        ]
    );
    let left = errors(&t, true);
    assert!(left.is_empty(), "{left:?}");
    let fixed = t.read("knowledge/terms.md");
    assert!(
        fixed.contains("[an option][sokf:terms#optiont]."),
        "{fixed}"
    );
    let report = t.validate_all(true, CacheMode::Off);
    assert!(report.fixes.is_empty(), "{}", json(&report));
}

// A bare fragment that gives a key's text is rewritten only to the id that
// first names a key: where the slug of the text names a key of a fence that
// two documents capture, `--fix` leaves the fragment and doesn't rewrite it
// to a key of prose whose plain text has the same slug.
// sokf-tests: SOKF_FORMAT_72
#[test]
fn a_bare_fragment_is_never_rewritten_to_another_key() {
    let schema = TERM_SCHEMA
        .replace(
            "          pattern: '^`(?P<term>[^`]+)` - '\n",
            "          pattern: '^(?P<term>_[A-Z][a-z]+_) - '\n",
        )
        .replace(
            "          example: '`Short code` - Seven letters.'\n",
            "          example: '_Short_ - Five letters.'\n",
        )
        .replace(
            "            - template: 'node ${name:slug}'\n              key: name\n",
            "            - template: 'node ${name}'\n              key: name\n              unique: knowledge\n",
        );
    let doc = |id: &str, node: &str, term: &str, tail: &str| {
        GOOD.replace("id: terms\ntitle: Terms", &format!("id: {id}\ntitle: Terms"))
            .replace(
                "- <a id=\"short-code\"></a>`Short code` - Seven letters.\n- <a id=\"visit\"></a>`Visit` - One opening of [a part](#one).\n",
                &format!("- {term} - Five letters{tail}.\n"),
            )
            .replace("node beta", &format!("node {node}"))
            .replace("<a id=\"beta\"></a>", "")
    };
    let t = TempTree::clean_with(&[
        ("knowledge/schemas/term.md", &schema),
        ("knowledge/a.md", &doc("a", "_Short_", "_Long_", "")),
        ("knowledge/b.md", &doc("b", "_Short_", "_Tall_", "")),
        (
            "knowledge/c.md",
            &doc(
                "c",
                "gamma",
                "*Short*",
                ", unlike [the fence key][sokf:#_Short_]",
            ),
        ),
    ]);
    t.validate_all(true, CacheMode::Off);
    let c = t.read("knowledge/c.md");
    assert!(c.contains("[the fence key][sokf:#_Short_]"), "{c}");
    let again = t.validate_all(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}
