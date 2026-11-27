//! Tests of the coverage of a key that its rule, or another rule of its
//! document, captures more than once: the key is filed under the id of its
//! anchor, reported once, and reported at the capture that holds that anchor.

use common::{TempTree, json, on, said};
use sokf_core::CacheMode;

use crate::{
    cited_by::{STORIES_SCHEMA, stories},
    common,
};

// sokf-tests: EXT-9_AC-1
// sokf-tests: EXT-9_AC-12
#[test]
fn a_key_captured_twice_by_one_rule_is_reported_once() {
    let tree = TempTree::fixture("clean");
    // The list and the fence both capture the key under the section's id.
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two captures.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>P-\\d+): .+$'\n          key: p\n          unique: knowledge\n          example: 'P-1: a part'\n      - kind: code\n        lang: text\n        example: 'P-1'\n        each:\n          lines:\n            - pattern: '^(?P<p>P-\\d+)$'\n              key: p\n              unique: knowledge\n              example: 'P-1'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- P-1: a part\n\n```text\nP-1\n```\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let on_doc: Vec<(Option<usize>, String)> = on(&report.errors, "twice-a.md")
        .map(|f| (f.line, said(&f.message)))
        .filter(|(_, m)| m.contains(" document cites "))
        .collect();
    assert_eq!(
        on_doc,
        vec![(Some(9), "no `Stories` document cites key `P-1`".to_string())],
        "{}",
        json(&report)
    );
}

// A key that a line of a fence repeats in another spelling than the item
// that holds its anchor is one key, filed under the id of that anchor, so
// a bare link to that id cites it.
// sokf-tests: EXT-9_AC-1
#[test]
fn a_key_that_a_fence_repeats_is_filed_under_its_anchor() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two spellings.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>_[A-Z][a-z]+_): .+$'\n          key: p\n          unique: knowledge\n          example: '_Ab_: a part'\n      - kind: code\n        lang: text\n        example: '_Ab_'\n        each:\n          lines:\n            - pattern: '^(?P<p>_[A-Z][a-z]+_)$'\n              key: p\n              key-repeats: true\n              unique: knowledge\n              example: '_Ab_'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- *Ab*: a part\n\n```text\n_Ab_\n```\n",
    );
    tree.write(
        "knowledge/stories-b.md",
        &stories("stories-b", "It names [the part][sokf:#ab] alone."),
    );
    let report = tree.validate(true, CacheMode::Off);
    let ours: Vec<String> = report
        .errors
        .iter()
        .filter(|f| matches!(f.path.file_name(), "twice-a.md" | "stories-b.md"))
        .map(|f| f.message.clone())
        .collect();
    assert!(ours.is_empty(), "{ours:?}");
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A line of a fence that pairs with an item by the plain text of its key
// is filed under the item's anchor, so the key is reported once.
// sokf-tests: EXT-9_AC-1
#[test]
fn a_key_that_a_fence_pairs_with_is_one_key() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two spellings.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>[A-Z][a-z]+): .+$'\n          key: p\n          unique: knowledge\n          example: 'Ab: a part'\n      - kind: code\n        lang: text\n        example: 'Ab'\n        each:\n          lines:\n            - pattern: '^(?P<p>\\S+)$'\n              key: p\n              for-each-key-of: parts\n              example: 'Ab'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- Ab: a part\n\n```text\n_Ab_\n```\n",
    );
    let report = tree.validate(true, CacheMode::Off);
    let ours: Vec<(String, Option<usize>, String)> = report
        .errors
        .iter()
        .filter(|f| f.path.as_str().ends_with("-a.md"))
        .map(|f| (f.path.to_string(), f.line, said(&f.message)))
        .collect();
    assert_eq!(
        ours,
        vec![(
            "/knowledge/twice-a.md".to_string(),
            Some(9),
            "no `Stories` document cites key `Ab`".to_string()
        )],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// An item and a line of a fence with `key-repeats` whose keys differ as
// written are two keys, each under its own anchor, though their plain texts
// are the same, so a link to the item's anchor cites only the item's key.
// sokf-tests: EXT-9_AC-1
#[test]
fn two_keys_with_one_plain_text_keep_their_anchors() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: Two keys, one plain text.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>[A-Z][a-z]+): .+$'\n          key: p\n          unique: knowledge\n          example: 'Ab: a part'\n      - kind: code\n        lang: text\n        example: '_Ab_'\n        each:\n          lines:\n            - pattern: '^(?P<p>_[A-Z][a-z]+_)$'\n              key: p\n              key-repeats: true\n              unique: knowledge\n              example: '_Ab_'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- Ab: a part\n\n```text\n_Ab_\n```\n",
    );
    tree.write(
        "knowledge/stories-b.md",
        &stories("stories-b", "It names [the part][sokf:#ab] alone."),
    );
    let report = tree.validate(true, CacheMode::Off);
    let ours: Vec<(String, Option<usize>, String)> = report
        .errors
        .iter()
        .filter(|f| matches!(f.path.file_name(), "twice-a.md" | "stories-b.md"))
        .map(|f| (f.path.to_string(), f.line, said(&f.message)))
        .collect();
    assert_eq!(
        ours,
        vec![(
            "/knowledge/twice-a.md".to_string(),
            Some(14),
            "no `Stories` document cites key `_Ab_`".to_string()
        )],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A key that a line of a fence defines, and that a heading pairs with in
// another spelling, takes the anchor that the heading holds, so a link to
// the heading cites the key of the fence.
// sokf-tests: EXT-9_AC-1
#[test]
fn a_key_of_a_fence_that_a_heading_pairs_with_takes_the_headings_anchor() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: A fence and its headings.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: nodes\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: code\n        lang: text\n        rule-id: nodes\n        example: '_Ab_'\n        each:\n          lines:\n            - pattern: '^(?P<p>_[A-Z][a-z]+_)$'\n              key: p\n              unique: knowledge\n              example: '_Ab_'\n  - heading: { template: '${p}', key: p, example: 'Ab' }\n    level: 2\n    for-each-key-of: nodes\n    repeat: true\n    blocks:\n      - kind: prose\n        example: The node holds the rest.\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n```text\n_Ab_\n```\n\n## Ab\n\nThe node holds the rest.\n",
    );
    tree.write(
        "knowledge/stories-b.md",
        &stories("stories-b", "It names [the node][sokf:twice-a#ab]."),
    );
    let report = tree.validate(true, CacheMode::Off);
    let ours: Vec<String> = report
        .errors
        .iter()
        .filter(|f| matches!(f.path.file_name(), "twice-a.md" | "stories-b.md"))
        .map(|f| f.message.clone())
        .collect();
    assert!(ours.is_empty(), "{ours:?}\n{}", json(&report));
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A key that its rule captures twice, with another key captured between
// the two, is reported once, at the item that holds its anchor.
// sokf-tests: EXT-9_AC-12
#[test]
fn a_key_captured_twice_around_another_is_reported_once() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two captures.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>P-\\d+): .+$'\n          key: p\n          unique: knowledge\n          example: 'P-1: a part'\n      - kind: code\n        lang: text\n        example: 'P-1'\n        each:\n          lines:\n            - pattern: '^(?P<p>P-\\d+)$'\n              key: p\n              key-repeats: true\n              unique: knowledge\n              example: 'P-1'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- P-1: a part\n- P-2: another part\n\n```text\nP-1\n```\n",
    );
    let report = tree.validate(true, CacheMode::Off);
    let on_doc: Vec<(Option<usize>, String)> = on(&report.errors, "twice-a.md")
        .map(|f| (f.line, said(&f.message)))
        .collect();
    assert_eq!(
        on_doc,
        vec![
            (Some(9), "no `Stories` document cites key `P-1`".to_string()),
            (
                Some(10),
                "no `Stories` document cites key `P-2`".to_string()
            ),
        ],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A key that a list captures first and a heading captures later is cited
// by the slug of that heading, as a key that only a heading captures is,
// and an uncited one is reported at that heading, which holds its anchor.
// sokf-tests: EXT-9_AC-1
// sokf-tests: EXT-9_AC-12
// sokf-tests: EXT-9_AC-13
#[test]
fn a_key_cited_by_the_heading_of_a_later_capture_is_cited() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two captures.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>P-\\d+): .+$'\n          key: p\n          key-of: parts\n          key-repeats: true\n          unique: knowledge\n          example: 'P-1: a part'\n  - heading: { template: '${p}: ${}', key: p, unique: knowledge, example: 'P-1: The part' }\n    level: 2\n    rule-id: parts\n    repeat: true\n    blocks:\n      - kind: prose\n        example: The part holds the rest.\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- P-1: a part\n- P-2: another part\n\n## P-1: The part\n\nThe part holds the rest.\n\n## P-2: The other part\n\nThe part holds the rest.\n",
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories(
            "stories-a",
            "It needs [the part][sokf:twice-a#p-1-the-part].",
        ),
    );
    let report = tree.validate(true, CacheMode::Off);
    let on_doc: Vec<(Option<usize>, String)> = on(&report.errors, "twice-a.md")
        .map(|f| (f.line, said(&f.message)))
        .collect();
    assert_eq!(
        on_doc,
        vec![(
            Some(16),
            "no `Stories` document cites key `P-2`".to_string()
        )],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A key that a line of a fence repeats before the item that defines it is
// reported at the item, which holds its anchor, not at the line of the
// fence, which only refers to it.
// sokf-tests: EXT-9_AC-12
#[test]
fn an_uncited_key_is_reported_at_the_capture_that_holds_its_anchor() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two captures.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: code\n        lang: text\n        example: 'P-1'\n        each:\n          lines:\n            - pattern: '^(?P<p>P-\\d+)$'\n              key: p\n              key-repeats: true\n              unique: knowledge\n              example: 'P-1'\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>P-\\d+): .+$'\n          key: p\n          unique: knowledge\n          example: 'P-1: a part'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n```text\nP-1\nP-2\n```\n\n- P-1: a part\n- P-2: another part\n",
    );
    let report = tree.validate(true, CacheMode::Off);
    let on_doc: Vec<(Option<usize>, String)> = on(&report.errors, "twice-a.md")
        .map(|f| (f.line, said(&f.message)))
        .collect();
    assert_eq!(
        on_doc,
        vec![
            (
                Some(14),
                "no `Stories` document cites key `P-1`".to_string()
            ),
            (
                Some(15),
                "no `Stories` document cites key `P-2`".to_string()
            ),
        ],
        "{}",
        json(&report)
    );
    // A run that writes the store, and a run that reads it, report the same
    // lines.
    let cached = tree.validate(true, CacheMode::On);
    assert!(cached.fixes.is_empty(), "{}", json(&cached));
    let again = tree.validate(false, CacheMode::On);
    let lines = |r: &sokf_core::Report| -> Vec<Option<usize>> {
        on(&r.errors, "twice-a.md").map(|f| f.line).collect()
    };
    assert_eq!(
        lines(&cached),
        vec![Some(14), Some(15)],
        "{}",
        json(&cached)
    );
    assert_eq!(lines(&again), vec![Some(14), Some(15)], "{}", json(&again));
}

// A line of a fence that pairs with an item, and that stands before it in
// another spelling, is one key with the item: the key is reported at the
// item, which holds its anchor, and named as the item reads it.
// sokf-tests: EXT-9_AC-12
#[test]
fn an_uncited_key_is_named_as_the_capture_that_holds_its_anchor_reads_it() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two spellings.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: code\n        lang: text\n        example: 'Ab'\n        each:\n          lines:\n            - pattern: '^(?P<p>\\S+)$'\n              key: p\n              for-each-key-of: parts\n              example: 'Ab'\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>[A-Z][a-z]+): .+$'\n          key: p\n          unique: knowledge\n          example: 'Ab: a part'\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n```text\n_Ab_\n```\n\n- Ab: a part\n",
    );
    let report = tree.validate(true, CacheMode::Off);
    let ours: Vec<(String, Option<usize>, String)> = report
        .errors
        .iter()
        .filter(|f| f.path.as_str().ends_with("-a.md"))
        .map(|f| (f.path.to_string(), f.line, said(&f.message)))
        .collect();
    assert_eq!(
        ours,
        vec![(
            "/knowledge/twice-a.md".to_string(),
            Some(13),
            "no `Stories` document cites key `Ab`".to_string()
        )],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A heading that pairs with a key that a list defines holds the key's
// anchor, so a link to the heading's slug cites the key, and an uncited key
// is reported at its heading.
// sokf-tests: EXT-9_AC-12
// sokf-tests: EXT-9_AC-13
#[test]
fn a_key_cited_by_the_heading_that_pairs_with_it_is_cited() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: A list and its headings.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    rule-id: parts\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>P-\\d+): .+$'\n          key: p\n          unique: knowledge\n          example: 'P-1: a part'\n  - heading: { template: '${p}: ${}', key: p, example: 'P-1: The part' }\n    level: 2\n    for-each-key-of: parts\n    repeat: true\n    blocks:\n      - kind: prose\n        example: The part holds the rest.\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- P-1: a part\n- P-2: another part\n\n## P-1: The part\n\nThe part holds the rest.\n\n## P-2: The other part\n\nThe part holds the rest.\n",
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories(
            "stories-a",
            "It needs [the part][sokf:twice-a#p-1-the-part].",
        ),
    );
    let report = tree.validate(true, CacheMode::Off);
    let ours: Vec<(String, Option<usize>, String)> = report
        .errors
        .iter()
        .filter(|f| f.path.as_str().ends_with("-a.md"))
        .map(|f| (f.path.to_string(), f.line, said(&f.message)))
        .collect();
    assert_eq!(
        ours,
        vec![(
            "/knowledge/twice-a.md".to_string(),
            Some(16),
            "no `Stories` document cites key `P-2`".to_string()
        )],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}

// A heading whose slug is the key's id holds the key's anchor without one
// that the engine places, so an uncited key is reported at its heading,
// after the item that repeats it.
// sokf-tests: EXT-9_AC-12
#[test]
fn an_uncited_key_is_reported_at_the_heading_whose_slug_names_it() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/twice.md",
        "---\ntype: Schema\nid: schema-twice\ndescription: One key, two captures.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Twice\ncited-by:\n  - from-type: Stories\n    keys: parts\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<p>P-\\d+): .+$'\n          key: p\n          key-of: parts\n          key-repeats: true\n          unique: knowledge\n          example: 'P-1: a part'\n  - heading: { template: '${p}', key: p, unique: knowledge, example: 'P-1' }\n    level: 2\n    rule-id: parts\n    repeat: true\n    blocks:\n      - kind: prose\n        example: The part holds the rest.\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/twice-a.md",
        "---\ntype: Twice\nid: twice-a\ntitle: A\n---\n\n# A\n\n- P-1: a part\n- P-2: another part\n\n## P-1\n\nThe part holds the rest.\n\n## P-2\n\nThe part holds the rest.\n",
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories("stories-a", "It needs [the part][sokf:twice-a#p-1]."),
    );
    let report = tree.validate(true, CacheMode::Off);
    let on_doc: Vec<(Option<usize>, String)> = on(&report.errors, "twice-a.md")
        .map(|f| (f.line, said(&f.message)))
        .collect();
    assert_eq!(
        on_doc,
        vec![(
            Some(16),
            "no `Stories` document cites key `P-2`".to_string()
        )],
        "{}",
        json(&report)
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{}", json(&again));
}
