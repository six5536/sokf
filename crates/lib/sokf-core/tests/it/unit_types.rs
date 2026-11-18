//! These tests check the `to-type` of a unit rule (`SOKF_SCHEMA_56`): each
//! `sokf:` link in a unit that the rule accepts reaches a document of one of
//! the rule's types, and a link with a fragment counts by the type of its
//! document.

use common::{TempTree, json};
use sokf_core::CacheMode;

use crate::common;

/// The schema of a contract, whose parts are keys of knowledge scope.
const CONTRACT_SCHEMA: &str = r"---
type: Schema
id: schema-contract
description: A contract with parts.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Contract
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        each:
          pattern: '^(?P<id>CLI-\d+): .+$'
          key: id
          unique: knowledge
          example: 'CLI-1: the command prints the graph'
````
";

/// The schema of an issue, which a story must not name as its contract.
const ISSUE_SCHEMA: &str = r"---
type: Schema
id: schema-issue
description: One piece of work.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Issue
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The portal shows the spending of each team.
````
";

/// The schema of a story, whose `Contracts:` line links only contracts.
const STORIES_SCHEMA: &str = r"---
type: Schema
id: schema-stories
description: Stories and the contracts that deliver them.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Stories
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        sequence:
          - pattern: '^A user can .+\.$'
            example: A user can print the graph.
          - pattern: '^Contracts: .+\.$'
            to-type: Contract
            example: 'Contracts: [CLI][sokf:contract-cli].'
````
";

fn tree(contracts: &str) -> TempTree {
    let tree = TempTree::empty("unit-types");
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: unit-types\nscope: test\n",
    );
    tree.write("knowledge/schemas/contract.md", CONTRACT_SCHEMA);
    tree.write("knowledge/schemas/issue.md", ISSUE_SCHEMA);
    tree.write("knowledge/schemas/stories.md", STORIES_SCHEMA);
    tree.write(
        "knowledge/contract-cli.md",
        "---\ntype: Contract\nid: contract-cli\ntitle: CLI\n---\n\n# CLI\n\n- CLI-6: the command prints the graph\n",
    );
    tree.write(
        "knowledge/issue-001-feature-spending-portal.md",
        "---\ntype: Issue\nid: issue-001-feature-spending-portal\ntitle: Spending portal\n---\n\n# Spending portal\n\nThe portal shows the spending of each team.\n",
    );
    tree.write("knowledge/stories-graph.md", &stories(contracts));
    tree
}

fn stories(contracts: &str) -> String {
    format!(
        "---\ntype: Stories\nid: stories-graph\ntitle: Graph\n---\n\n# Graph\n\nA user can print the graph.\n\n{contracts}\n"
    )
}

/// `typed` returns each `SOKF_SCHEMA_56` error of a run with `--fix`, as
/// `line: message`. The fix writes the definition block.
fn typed(tree: &TempTree, mode: CacheMode) -> Vec<String> {
    let report = tree.validate(true, mode);
    let others: Vec<_> = report
        .errors
        .iter()
        .filter(|f| f.criterion != "SOKF_SCHEMA_56")
        .collect();
    assert!(others.is_empty(), "{}", json(&report));
    report
        .errors
        .iter()
        .map(|f| format!("{}: {}", f.line.unwrap_or(0), f.message))
        .collect()
}

// sokf-tests: EXT-5_AC-2
// sokf-tests: SOKF_SCHEMA_56
#[test]
fn a_link_to_a_document_of_another_type_is_an_error_at_its_line() {
    let tree = tree("Contracts: [PORTAL][sokf:issue-001-feature-spending-portal].");
    for mode in [CacheMode::Off, CacheMode::On] {
        assert_eq!(
            typed(&tree, mode),
            [
                "11: `[sokf:issue-001-feature-spending-portal]` links a document of type `Issue`, but its rule accepts only a link to a document of type `Contract` (schema schema-stories, `to-type` of the rule at line 20)"
            ],
            "{mode:?}"
        );
    }
}

// sokf-tests: EXT-5_AC-2
#[test]
fn a_link_to_a_document_of_the_type_passes() {
    let tree = tree("Contracts: [CLI][sokf:contract-cli].");
    for mode in [CacheMode::Off, CacheMode::On] {
        assert_eq!(typed(&tree, mode), Vec::<String>::new(), "{mode:?}");
    }
}

// The first link has a fragment that names a key of the contract, and the
// second names the key alone, which resolves in the store.
// sokf-tests: EXT-5_AC-3
#[test]
fn a_link_with_a_fragment_counts_by_the_type_of_its_document() {
    let tree = tree("Contracts: [CLI-6][sokf:contract-cli#cli-6], [CLI-6][sokf:#cli-6].");
    for mode in [CacheMode::Off, CacheMode::On] {
        assert_eq!(typed(&tree, mode), Vec::<String>::new(), "{mode:?}");
    }
    tree.write(
        "knowledge/stories-graph.md",
        &stories("Contracts: [CLI][sokf:issue-001-feature-spending-portal#spending-portal]."),
    );
    for mode in [CacheMode::Off, CacheMode::On] {
        assert_eq!(typed(&tree, mode).len(), 1, "{mode:?}");
    }
}

// The cache checks the story again when the type of its target changes,
// although the story itself is unchanged.
// sokf-tests: EXT-5_AC-2
#[test]
fn a_change_of_the_target_type_reaches_a_cached_story() {
    let tree = tree("Contracts: [CLI-6][sokf:#cli-6].");
    assert_eq!(typed(&tree, CacheMode::On), Vec::<String>::new());
    tree.write(
        "knowledge/schemas/contract.md",
        &CONTRACT_SCHEMA.replace("const: Contract", "const: Spec"),
    );
    tree.write(
        "knowledge/contract-cli.md",
        "---\ntype: Spec\nid: contract-cli\ntitle: CLI\n---\n\n# CLI\n\n- CLI-6: the command prints the graph\n",
    );
    for mode in [CacheMode::On, CacheMode::Off] {
        assert_eq!(
            typed(&tree, mode),
            [
                "11: `[sokf:#cli-6]` links a document of type `Spec`, but its rule accepts only a link to a document of type `Contract` (schema schema-stories, `to-type` of the rule at line 20)"
            ],
            "{mode:?}"
        );
    }
}

// A unit rule without `to-type` lets a link reach a document of any type.
// sokf-tests: EXT-5_AC-2
#[test]
fn a_rule_without_to_type_accepts_any_target() {
    let tree = tree("Contracts: [PORTAL][sokf:issue-001-feature-spending-portal].");
    tree.write(
        "knowledge/schemas/stories.md",
        &STORIES_SCHEMA.replace("            to-type: Contract\n", ""),
    );
    assert_eq!(typed(&tree, CacheMode::Off), Vec::<String>::new());
}

// The second run reads the story again, whose schema changed, and finds
// the type of the contract that the bare key names, as a run without the
// cache does.
// sokf-tests: EXT-5_AC-3
#[test]
fn a_bare_key_counts_by_its_document_in_a_cached_run() {
    let tree = tree("Contracts: [CLI-6][sokf:#cli-6].");
    assert_eq!(typed(&tree, CacheMode::On), Vec::<String>::new());
    tree.write(
        "knowledge/schemas/stories.md",
        &STORIES_SCHEMA.replace("to-type: Contract", "to-type: [Issue, Note]"),
    );
    for mode in [CacheMode::On, CacheMode::Off] {
        assert_eq!(
            typed(&tree, mode),
            [
                "11: `[sokf:#cli-6]` links a document of type `Contract`, but its rule accepts only a link to a document of type `Issue` or `Note` (schema schema-stories, `to-type` of the rule at line 20)"
            ],
            "{mode:?}"
        );
    }
}
