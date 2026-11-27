//! Tests of cross-document coverage. A schema's `cited-by` requires
//! documents of another type to cite the documents or keys of the schema's
//! type. These tests also check what counts as a citation.

use common::{TempTree, json, on, said};
use sokf_core::CacheMode;

use crate::common;

/// The schema of a contract whose criteria are keys. A Stories document must
/// cite each criterion of a public contract.
const CONTRACT_SCHEMA: &str = r"---
type: Schema
id: schema-contract
description: A contract whose public criteria a story accepts.
---

````yaml
line-limit: 0
lifecycle:
  - { key: scope, states: [public, internal] }
frontmatter:
  type:
    const: Contract
cited-by:
  - from-type: Stories
    keys: criteria
    only: { scope: [public] }
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        rule-id: criteria
        each:
          pattern: '^(?P<id>C-\d+): .+$'
          key: id
          unique: knowledge
          example: 'C-1: it works'
````
";

/// The schema of a story. A story is free prose that cites criteria.
pub(crate) const STORIES_SCHEMA: &str = r"---
type: Schema
id: schema-stories
description: Stories that cite the criteria accepting them.
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

fn contract(id: &str, scope: &str, criteria: &[&str]) -> String {
    let items: Vec<String> = criteria
        .iter()
        .map(|c| format!("- {c}: it works"))
        .collect();
    format!(
        "---\ntype: Contract\nid: {id}\ntitle: {id}\nscope: {scope}\n---\n\n# {id}\n\n{}\n",
        items.join("\n")
    )
}

pub(crate) fn stories(id: &str, body: &str) -> String {
    format!("---\ntype: Stories\nid: {id}\ntitle: {id}\n---\n\n# {id}\n\n{body}\n")
}

/// `messages` returns the coverage findings on `file` and drops the others.
/// A hand-written document has no definition block. `--fix` writes that
/// block, and these tests don't check it.
fn messages(tree: &TempTree, file: &str) -> Vec<String> {
    on(&tree.validate(false, CacheMode::Off).errors, file)
        .map(|f| said(&f.message))
        .filter(|m| m.contains(" document cites "))
        .collect()
}

// sokf-tests: EXT-9_AC-1
// sokf-tests: EXT-9_AC-3
// sokf-tests: EXT-9_AC-10
#[test]
fn an_uncited_key_is_reported_at_its_line_and_a_cited_one_is_not() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1", "C-2"]),
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories("stories-a", "Accepted by [C-1][sokf:#c-1]."),
    );
    let report = tree.validate(false, CacheMode::Off);
    let on_contract: Vec<_> = on(&report.errors, "contract-a.md").collect();
    assert_eq!(on_contract.len(), 1, "{}", json(&report));
    let f = on_contract[0];
    assert_eq!(f.line, Some(11));
    assert_eq!(
        f.message,
        "no `Stories` document cites key `C-2` (schema schema-contract, cited-by `Stories` of `criteria`)"
    );
}

// sokf-tests: EXT-9_AC-4
#[test]
fn only_exempts_a_document_the_entry_does_not_admit() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "internal", &["C-1", "C-2"]),
    );
    assert_eq!(messages(&tree, "contract-a.md"), Vec::<String>::new());
}

// sokf-tests: EXT-9_AC-1
// sokf-tests: EXT-9_AC-3
#[test]
fn an_uncited_document_is_reported_with_no_line_and_a_part_cites_the_whole() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/design.md",
        "---\ntype: Schema\nid: schema-design\ndescription: A design the map asks for.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Design\ncited-by:\n  - from-type: Map\nsections:\n  - heading: { from: title }\n    level: 1\n  - heading: Parts\n    level: 2\n````\n",
    );
    tree.write(
        "knowledge/schemas/big-picture.md",
        "---\ntype: Schema\nid: schema-big-picture\ndescription: The map.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Map\nsections:\n  - heading: { from: title }\n    level: 1\n````\n",
    );
    for id in ["design-a", "design-b"] {
        tree.write(
            &format!("knowledge/{id}.md"),
            &format!("---\ntype: Design\nid: {id}\ntitle: {id}\n---\n\n# {id}\n\nProse.\n\n## Parts\n\nProse.\n"),
        );
    }
    tree.write(
        "knowledge/map.md",
        "---\ntype: Map\nid: map\ntitle: Map\n---\n\n# Map\n\nSee [the parts of A][sokf:design-a#parts].\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let designs: Vec<(String, Option<usize>, String)> = report
        .errors
        .iter()
        .filter(|f| f.path.file_name().starts_with("design-"))
        .map(|f| (f.path.file_name().to_string(), f.line, f.message.clone()))
        .collect();
    assert_eq!(
        designs,
        vec![(
            "design-b.md".to_string(),
            None,
            "no `Map` document cites this document (schema schema-design, cited-by `Map`)"
                .to_string()
        )],
        "{}",
        json(&report)
    );
}

// sokf-tests: EXT-9_AC-2
#[test]
fn a_citation_from_another_store_counts() {
    let tree = TempTree::fixture("clean");
    tree.write(
        ".sokf/config.toml",
        "[knowledge]\nroots = [\"/knowledge\", \"/other\"]\n",
    );
    tree.write(
        "other/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: other\nscope: test\ndescription: Another store.\n",
    );
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("other/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1"]),
    );
    tree.write(
        "other/stories-a.md",
        &stories("stories-a", "Accepted by [C-1][sokf:fixture:#c-1]."),
    );
    assert_eq!(messages(&tree, "contract-a.md"), Vec::<String>::new());
}

// sokf-tests: EXT-9_AC-2
#[test]
fn included_text_does_not_cite() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1"]),
    );
    tree.write("knowledge/quoted.md", &stories("quoted", "Quotes nothing."));
    // The story mentions C-1 only inside an include region, which the
    // parser masks. sokf reports the stale region as a separate error.
    tree.write(
        "knowledge/stories-a.md",
        &stories(
            "stories-a",
            "Prose.\n\n<!-- sokf:include quoted -->\nQuotes [C-1][sokf:#c-1].\n<!-- /sokf:include -->",
        ),
    );
    assert_eq!(
        messages(&tree, "contract-a.md"),
        vec!["no `Stories` document cites key `C-1`"]
    );
}

// sokf-tests: EXT-9_AC-2
#[test]
fn a_self_citation_does_not_count_and_another_document_of_the_type_does() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\ndescription: Notes that cite each other's terms.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\ncited-by:\n  - from-type: Note\n    keys: terms\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        rule-id: terms\n        each:\n          pattern: '^(?P<t>T-\\d+): .+$'\n          key: t\n          unique: knowledge\n          example: 'T-1: a term'\n      - kind: prose\n        min: 0\n````\n",
    );
    tree.write(
        "knowledge/note-a.md",
        "---\ntype: Note\nid: note-a\ntitle: A\n---\n\n# A\n\n- T-1: a term\n- T-2: a term\n\nSee [T-1][sokf:#t-1] and [T-2][sokf:#t-2].\n",
    );
    tree.write(
        "knowledge/note-b.md",
        "---\ntype: Note\nid: note-b\ntitle: B\n---\n\n# B\n\n- T-3: a term\n\nSee [T-2][sokf:#t-2] and [T-3][sokf:#t-3].\n",
    );
    assert_eq!(
        messages(&tree, "note-a.md"),
        vec!["no `Note` document cites key `T-1`"]
    );
    assert_eq!(
        messages(&tree, "note-b.md"),
        vec!["no `Note` document cites key `T-3`"]
    );
}

// sokf-tests: EXT-9_AC-2
#[test]
fn a_citing_document_with_errors_still_cites() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1"]),
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories(
            "stories-a",
            "Accepted by [C-1][sokf:#c-1], and [nothing][sokf:nowhere].",
        ),
    );
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.path.file_name() == "stories-a.md"),
        "{}",
        json(&report)
    );
    assert_eq!(messages(&tree, "contract-a.md"), Vec::<String>::new());
}

// sokf-tests: EXT-9_AC-2
#[test]
fn a_document_whose_type_has_no_schema_still_cites() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1"]),
    );
    // No schema governs `Stories`. The contract schema's cited-by entry gives
    // only the type's name.
    tree.write(
        "knowledge/stories-a.md",
        &stories("stories-a", "Accepted by [C-1][sokf:#c-1]."),
    );
    assert_eq!(messages(&tree, "contract-a.md"), Vec::<String>::new());
}

// sokf-tests: EXT-9_AC-3
#[test]
fn the_entry_severity_puts_the_finding_among_the_warnings() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/contract.md",
        &CONTRACT_SCHEMA.replace(
            "    keys: criteria\n",
            "    keys: criteria\n    severity: warn\n",
        ),
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1"]),
    );
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(messages(&tree, "contract-a.md"), Vec::<String>::new());
    let warned: Vec<String> = on(&report.warnings, "contract-a.md")
        .map(|f| said(&f.message))
        .collect();
    assert_eq!(warned, vec!["no `Stories` document cites key `C-1`"]);
}

// sokf-tests: EXT-9_AC-9
#[test]
fn the_entry_message_follows_the_text_of_each_key_finding() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/contract.md",
        &CONTRACT_SCHEMA.replace(
            "    keys: criteria\n",
            "    keys: criteria\n    message: A story that a criterion delivers links the criterion.\n",
        ),
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1", "C-2"]),
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories("stories-a", "Accepted by [C-1][sokf:#c-1]."),
    );
    let report = tree.validate(false, CacheMode::Off);
    let on_contract: Vec<String> = on(&report.errors, "contract-a.md")
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(
        on_contract,
        vec![
            "no `Stories` document cites key `C-2`. A story that a criterion delivers links the criterion. (schema schema-contract, cited-by `Stories` of `criteria`)"
        ],
        "{}",
        json(&report)
    );
}

// One entry of the schema declares a message and the other doesn't, and
// each finding carries only its own entry's message.
// sokf-tests: EXT-9_AC-9
// sokf-tests: EXT-9_AC-10
#[test]
fn each_document_finding_carries_only_its_own_entry_message() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/design.md",
        "---\ntype: Schema\nid: schema-design\ndescription: A design that the map and a story ask for.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Design\ncited-by:\n  - from-type: Map\n    message: The map links each design of the project.\n  - from-type: Stories\nsections:\n  - heading: { from: title }\n    level: 1\n````\n",
    );
    tree.write(
        "knowledge/design-a.md",
        "---\ntype: Design\nid: design-a\ntitle: design-a\n---\n\n# design-a\n\nProse.\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    let on_design: Vec<String> = on(&report.errors, "design-a.md")
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(
        on_design,
        vec![
            "no `Map` document cites this document. The map links each design of the project. (schema schema-design, cited-by `Map`)",
            "no `Stories` document cites this document (schema schema-design, cited-by `Stories`)",
        ],
        "{}",
        json(&report)
    );
}

// sokf-tests: EXT-9_AC-1
#[test]
fn a_section_rule_id_covers_its_heading_keys() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/big-picture.md",
        "---\ntype: Schema\nid: schema-big-picture\ndescription: Flows a story realises.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Map\ncited-by:\n  - from-type: Stories\n    keys: flows\nsections:\n  - heading: { from: title }\n    level: 1\n  - heading: { template: 'Flow ${n:int}', key: n, unique: knowledge, example: Flow 1 }\n    level: 2\n    repeat: true\n    rule-id: flows\n````\n",
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/map.md",
        "---\ntype: Map\nid: map\ntitle: Map\n---\n\n# Map\n\nProse.\n\n## Flow 1\n\nProse.\n\n## Flow 2\n\nProse.\n",
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories("stories-a", "Realised by [Flow 2][sokf:map#flow-2]."),
    );
    let report = tree.validate(false, CacheMode::Off);
    let on_map: Vec<(Option<usize>, String)> = on(&report.errors, "map.md")
        .map(|f| (f.line, said(&f.message)))
        .collect();
    assert_eq!(
        on_map,
        vec![(Some(11), "no `Stories` document cites key `1`".to_string())],
        "{}",
        json(&report)
    );
}

// sokf-tests: EXT-9_AC-2
#[test]
fn any_of_the_listed_types_cites() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/contract.md",
        &CONTRACT_SCHEMA.replace("from-type: Stories", "from-type: [Stories, Map]"),
    );
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/schemas/big-picture.md",
        "---\ntype: Schema\nid: schema-big-picture\ndescription: The map.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Map\nsections:\n  - heading: { from: title }\n    level: 1\n````\n",
    );
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1", "C-2"]),
    );
    tree.write(
        "knowledge/map.md",
        "---\ntype: Map\nid: map\ntitle: Map\n---\n\n# Map\n\nSee [C-1][sokf:#c-1].\n",
    );
    assert_eq!(
        messages(&tree, "contract-a.md"),
        vec!["no `Stories` or `Map` document cites key `C-2`"]
    );
}

// The test adds a link to a contract and then removes it, and leaves the
// contract unchanged. The contract's citations change, so the second round
// checks the contract again. That check reads the keys that the contract
// captures, so the cache store must keep those keys.
// sokf-tests: EXT-9_AC-1
// sokf-tests: CACHE_P-1
#[test]
fn a_contract_checked_again_for_its_citations_keeps_its_keys() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-a.md",
        &contract("contract-a", "public", &["C-1", "C-2"]),
    );
    tree.write(
        "knowledge/stories-a.md",
        &stories("stories-a", "No link yet."),
    );
    common::age(tree.dir(), 10);
    assert_eq!(tree.findings(CacheMode::On), tree.findings(CacheMode::Off));
    let key = || sokf_core::locate(&tree.root(), &["#c-2".to_string()], false, CacheMode::On);
    assert!(key().is_ok());
    for body in [
        "Accepted by [contract A][sokf:contract-a].",
        "No link again.",
    ] {
        tree.write("knowledge/stories-a.md", &stories("stories-a", body));
        common::age_file(&tree.dir().join("knowledge/stories-a.md"), 10);
        assert_eq!(
            tree.findings(CacheMode::On),
            tree.findings(CacheMode::Off),
            "{body}"
        );
        assert!(key().is_ok(), "{body}");
    }
}

// A new document cites a document that had no citation. The first round
// checks the new document. The second round checks the cited document again
// and must count the citation that the first round found. The cache store
// doesn't hold that citation yet.
// sokf-tests: EXT-9_AC-1
// sokf-tests: CACHE_P-1
#[test]
fn a_new_citing_document_clears_the_warning_through_the_cache() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/design.md",
        "---\ntype: Schema\nid: schema-design\ndescription: A design the map asks for.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Design\ncited-by:\n  - from-type: Map\n    severity: warn\nsections:\n  - heading: { from: title }\n    level: 1\n````\n",
    );
    tree.write(
        "knowledge/schemas/big-picture.md",
        "---\ntype: Schema\nid: schema-big-picture\ndescription: The map.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Map\nsections:\n  - heading: { from: title }\n    level: 1\n````\n",
    );
    tree.write(
        "knowledge/design-a.md",
        "---\ntype: Design\nid: design-a\ntitle: design-a\n---\n\n# design-a\n\nProse.\n",
    );
    common::age(tree.dir(), 10);
    let before = tree.findings(CacheMode::On);
    assert!(
        before.iter().any(|f| f.contains("no `Map` document cites")),
        "{before:?}"
    );
    tree.write(
        "knowledge/map.md",
        "---\ntype: Map\nid: map\ntitle: Map\n---\n\n# Map\n\nSee [A][sokf:design-a].\n",
    );
    common::age_file(&tree.dir().join("knowledge/map.md"), 10);
    let warm = tree.findings(CacheMode::On);
    assert_eq!(warm, tree.findings(CacheMode::Off));
    assert!(
        !warm.iter().any(|f| f.contains("no `Map` document cites")),
        "{warm:?}"
    );
}
