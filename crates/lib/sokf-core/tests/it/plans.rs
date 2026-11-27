//! These tests check planned paths (`SOKF_SCHEMA_52`, `SOKF_SCHEMA_53`): an open
//! plan lets a page link, list, and include a file before it exists, and the
//! checks of the page follow the plan through the cache.

use common::{TempTree, json};
use sokf_core::CacheMode;

use crate::common;

const PLAN_SCHEMA: &str = r"---
type: Schema
id: schema-plan
description: A plan of work.
---

````yaml
line-limit: 0
plans-paths:
  - section: files
    only: { state: [open] }
lifecycle:
  - { key: state, states: [open, closed] }
frontmatter:
  type:
    const: Plan
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The plan adds a parser.
  - heading: Files
    level: 2
    rule-id: files
    blocks:
      - kind: bullet-list
        each:
          pattern: '^\['
          example: '[`/src/parse.rs`](/src/parse.rs): the parser.'
````
";

const PAGE_SCHEMA: &str = r"---
type: Schema
id: schema-page
description: A page about code.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Page
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The parser reads the rules.
      - kind: include
        optional: true
        source-pattern: '^/'
        example: |
          <!-- sokf:include /src/parse.rs -->
          <!-- /sokf:include -->
````
";

fn plan(state: &str) -> String {
    format!(
        "---\ntype: Plan\nid: plan-parser\ntitle: Parser\nstate: {state}\n---\n\n# Parser\n\nThe plan adds a parser.\n\n## Files\n\n- [`/src/parse.rs`](/src/parse.rs): the parser.\n"
    )
}

const PAGE: &str = "---\ntype: Page\nid: page-parser\ntitle: Parser\nresource: /src/parse.rs\n---\n\n# Parser\n\nThe [parser](/src/parse.rs) reads the rules.\n\n<!-- sokf:include /src/parse.rs -->\n```rust\nfn parse() {}\n```\n<!-- /sokf:include -->\n";

/// `keys` returns the criterion of each error on the page, in a run with
/// the cache in `mode`, with `fix` applied first.
fn keys(tree: &TempTree, fix: bool, mode: CacheMode) -> Vec<String> {
    tree.validate(fix, mode)
        .errors
        .iter()
        .filter(|f| f.path.as_str() == "/knowledge/page-parser.md")
        .map(|f| f.criterion.clone())
        .collect()
}

// sokf-tests: SOKF_SCHEMA_53
#[test]
fn an_open_plan_lets_a_page_link_list_and_include_a_file_before_it_exists() {
    let tree = TempTree::fixture("new");
    tree.write("knowledge/schemas/plan.md", PLAN_SCHEMA);
    tree.write("knowledge/schemas/page.md", PAGE_SCHEMA);
    tree.write("knowledge/page-parser.md", PAGE);
    // With no plan, the page's link, resource, and include each fail.
    for mode in [CacheMode::Off, CacheMode::On] {
        let mut got = keys(&tree, false, mode);
        got.sort();
        assert_eq!(
            got,
            ["SOKF_FORMAT_30", "SOKF_FORMAT_76", "SOKF_FORMAT_87"],
            "{mode:?}"
        );
    }
    // An open plan of the file clears them, and the fix keeps the sketch.
    tree.write("knowledge/plan-parser.md", &plan("open"));
    assert_eq!(keys(&tree, true, CacheMode::On), Vec::<String>::new());
    assert_eq!(keys(&tree, false, CacheMode::Off), Vec::<String>::new());
    assert!(
        tree.read("knowledge/page-parser.md")
            .contains("fn parse() {}")
    );
    // After the file exists, the fix fills the include from it.
    tree.write("src/parse.rs", "pub fn parse(text: &str) {}\n");
    assert_eq!(keys(&tree, true, CacheMode::On), Vec::<String>::new());
    assert!(
        tree.read("knowledge/page-parser.md")
            .contains("pub fn parse(text: &str) {}"),
        "{}",
        tree.read("knowledge/page-parser.md")
    );
    // A closed plan plans nothing, so a missing file fails again.
    std::fs::remove_file(tree.dir().join("src/parse.rs")).unwrap();
    tree.write("knowledge/plan-parser.md", &plan("closed"));
    let report = tree.validate(false, CacheMode::On);
    let mut got = keys(&tree, false, CacheMode::On);
    got.sort();
    assert_eq!(
        got,
        ["SOKF_FORMAT_30", "SOKF_FORMAT_76", "SOKF_FORMAT_87"],
        "{}",
        json(&report)
    );
}

// sokf-tests: SOKF_SCHEMA_52
#[test]
fn a_plan_must_name_a_section_of_its_schema() {
    let tree = TempTree::fixture("new");
    tree.write(
        "knowledge/schemas/plan.md",
        &PLAN_SCHEMA.replace("  - section: files\n", "  - section: nowhere\n"),
    );
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.criterion == "SOKF_SCHEMA_52"),
        "{}",
        json(&report)
    );
}

// A schema's `current-links` keeps a `Planned:` paragraph from linking a
// closed plan, which the Plan schema archives.
// sokf-tests: SOKF_SCHEMA_54
#[test]
fn a_promise_links_no_archived_document() {
    let tree = TempTree::fixture("new");
    tree.write(
        "knowledge/schemas/plan.md",
        &PLAN_SCHEMA.replace(
            "  - { key: state, states: [open, closed] }\n",
            "  - { key: state, states: [open, closed], archived: [closed] }\n",
        ),
    );
    tree.write(
        "knowledge/schemas/page.md",
        &PAGE_SCHEMA.replace(
            "line-limit: 0\n",
            "line-limit: 0\ncurrent-links: ['^Planned: ']\n",
        ),
    );
    tree.write("knowledge/plan-parser.md", &plan("open"));
    tree.write(
        "knowledge/page-parser.md",
        "---\ntype: Page\nid: page-parser\ntitle: Parser\n---\n\n# Parser\n\nPlanned: The parser reads the rules, as [the plan][sokf:plan-parser] describes.\n",
    );
    for mode in [CacheMode::Off, CacheMode::On] {
        assert_eq!(keys(&tree, true, mode), Vec::<String>::new(), "{mode:?}");
    }
    tree.write("knowledge/plan-parser.md", &plan("closed"));
    for mode in [CacheMode::On, CacheMode::Off] {
        assert_eq!(keys(&tree, false, mode), ["SOKF_SCHEMA_54"], "{mode:?}");
    }
}

/// `dir_plan` returns a plan in `state` whose Files section links the
/// directory `/src/fixtures/`, with the trailing slash.
fn dir_plan(state: &str) -> String {
    plan(state).replace(
        "- [`/src/parse.rs`](/src/parse.rs): the parser.",
        "- [`/src/fixtures/`](/src/fixtures/): the fixtures of the parser.",
    )
}

// sokf-tests: SOKF_SCHEMA_55
#[test]
fn a_planned_directory_plans_every_path_under_it() {
    let tree = TempTree::fixture("new");
    tree.write("knowledge/schemas/plan.md", PLAN_SCHEMA);
    tree.write("knowledge/schemas/page.md", PAGE_SCHEMA);
    tree.write(
        "knowledge/page-parser.md",
        "---\ntype: Page\nid: page-parser\ntitle: Parser\nresource: /src/fixtures/a.json\n---\n\n# Parser\n\nThe parser reads [the fixtures](/src/fixtures/), such as [one](/src/fixtures/a.json) and [another](/src/fixtures/deep/b.json).\n",
    );
    // With no plan, the resource and each link fail. The cached run checks
    // the page first, so that the plan must make the page dirty.
    for mode in [CacheMode::Off, CacheMode::On] {
        let mut got = keys(&tree, false, mode);
        got.sort();
        assert_eq!(
            got,
            [
                "SOKF_FORMAT_30",
                "SOKF_FORMAT_76",
                "SOKF_FORMAT_76",
                "SOKF_FORMAT_76"
            ],
            "{mode:?}"
        );
    }
    // An open plan of the directory plans the directory and every path under
    // it, and the plan itself, which links the directory, passes.
    tree.write("knowledge/plan-parser.md", &dir_plan("open"));
    for mode in [CacheMode::On, CacheMode::Off] {
        let report = tree.validate(true, mode);
        let ours: Vec<_> = report
            .errors
            .iter()
            .filter(|f| f.path.as_str().ends_with("-parser.md"))
            .collect();
        assert!(ours.is_empty(), "{mode:?}: {}", json(&report));
    }
    assert!(
        tree.read("knowledge/plan-parser.md")
            .contains("(/src/fixtures/)")
    );
    // A closed plan plans nothing, so the paths fail again.
    tree.write("knowledge/plan-parser.md", &dir_plan("closed"));
    for mode in [CacheMode::On, CacheMode::Off] {
        assert_eq!(keys(&tree, false, mode).len(), 4, "{mode:?}");
    }
}

// A planning section whose heading a formatter spells another way still
// plans its links, and a heading that the check gives to another section
// plans nothing, as the check reads the headings.
// sokf-tests: SOKF_SCHEMA_53
// sokf-tests: SOKF_SCHEMA_109
#[test]
fn a_respelt_planning_section_still_plans() {
    let tree = TempTree::fixture("new");
    tree.write(
        "knowledge/schemas/plan.md",
        &PLAN_SCHEMA
            .replace("  - heading: Files\n", "  - heading: '*Files*'\n")
            .replace(
                "          example: '[`/src/parse.rs`](/src/parse.rs): the parser.'\n",
                "          example: '[`/src/parse.rs`](/src/parse.rs): the parser.'\n  - heading: Notes\n    level: 2\n    optional: true\n    blocks: []\n",
            ),
    );
    tree.write("knowledge/schemas/page.md", PAGE_SCHEMA);
    tree.write("knowledge/page-parser.md", PAGE);
    tree.write(
        "knowledge/plan-parser.md",
        &plan("open").replace("## Files", "## _Files_"),
    );
    for mode in [CacheMode::Off, CacheMode::On] {
        let report = tree.validate(true, mode);
        let ours: Vec<_> = report
            .errors
            .iter()
            .filter(|f| f.path.as_str().ends_with("-parser.md"))
            .collect();
        assert!(ours.is_empty(), "{mode:?}: {}", json(&report));
        let again = tree.validate(true, mode);
        assert!(again.fixes.is_empty(), "{mode:?}: {}", json(&again));
    }
    // A template of the planning section takes no heading that another
    // section gives exactly.
    tree.write(
        "knowledge/schemas/plan.md",
        &tree.read("knowledge/schemas/plan.md").replace(
            "  - heading: '*Files*'\n",
            "  - heading: { template: '${}', example: Files }\n",
        ),
    );
    tree.write(
        "knowledge/plan-parser.md",
        &plan("open").replace("## Files", "## Notes"),
    );
    for mode in [CacheMode::Off, CacheMode::On] {
        let mut got = keys(&tree, false, mode);
        got.sort();
        assert_eq!(
            got,
            ["SOKF_FORMAT_30", "SOKF_FORMAT_76", "SOKF_FORMAT_87"],
            "{mode:?}"
        );
    }
}

// A pattern of `current-links` reads a paragraph in tramlines' fixed
// spelling, so a pattern for bold text matches a paragraph whose bold a
// formatter spells either way.
// sokf-tests: SOKF_SCHEMA_54
// sokf-tests: SOKF_SCHEMA_109
#[test]
fn a_promise_reads_in_the_fixed_spelling() {
    let tree = TempTree::fixture("new");
    tree.write(
        "knowledge/schemas/plan.md",
        &PLAN_SCHEMA.replace(
            "  - { key: state, states: [open, closed] }\n",
            "  - { key: state, states: [open, closed], archived: [closed] }\n",
        ),
    );
    tree.write(
        "knowledge/schemas/page.md",
        &PAGE_SCHEMA.replace(
            "line-limit: 0\n",
            "line-limit: 0\ncurrent-links: ['^\\*\\*Planned:\\*\\* ']\n",
        ),
    );
    tree.write("knowledge/plan-parser.md", &plan("closed"));
    for bold in ["__Planned:__", "**Planned:**"] {
        tree.write(
            "knowledge/page-parser.md",
            &format!("---\ntype: Page\nid: page-parser\ntitle: Parser\n---\n\n# Parser\n\n{bold}  The parser reads the rules,\nas [the plan][sokf:plan-parser] describes.\n"),
        );
        for mode in [CacheMode::Off, CacheMode::On] {
            assert_eq!(
                keys(&tree, true, mode),
                ["SOKF_SCHEMA_54"],
                "{bold} {mode:?}"
            );
            let again = tree.validate(true, mode);
            assert!(again.fixes.is_empty(), "{bold} {mode:?}: {}", json(&again));
        }
    }
}

// A heading that stands in place of a planning section bound to the
// frontmatter, and reads otherwise than the value, plans nothing, as it
// holds no section in `sokf guide`.
// sokf-tests: SOKF_SCHEMA_109
#[test]
fn a_heading_that_reads_otherwise_than_its_bound_value_plans_nothing() {
    let tree = TempTree::fixture("new");
    tree.write(
        "knowledge/schemas/plan.md",
        &PLAN_SCHEMA
            .replace(
                "  - heading: { from: title }\n    level: 1\n",
                "  - heading: { from: title }\n    level: 1\n    rule-id: files\n",
            )
            .replace(
                "    rule-id: files\n    blocks:\n      - kind: bullet-list",
                "    blocks:\n      - kind: bullet-list",
            ),
    );
    tree.write("knowledge/schemas/page.md", PAGE_SCHEMA);
    tree.write("knowledge/page-parser.md", PAGE);
    tree.write(
        "knowledge/plan-parser.md",
        &plan("open")
            .replace("title: Parser", "title: The *parser*")
            .replace("# Parser", "# The _parser_"),
    );
    for mode in [CacheMode::Off, CacheMode::On] {
        let got = keys(&tree, true, mode);
        assert!(got.is_empty(), "{mode:?}: {got:?}");
        let again = tree.validate(true, mode);
        assert!(again.fixes.is_empty(), "{mode:?}: {}", json(&again));
    }
    tree.write(
        "knowledge/plan-parser.md",
        &plan("open")
            .replace("title: Parser", "title: The *parser*")
            .replace("# Parser", "# A parser"),
    );
    for mode in [CacheMode::Off, CacheMode::On] {
        let mut got = keys(&tree, false, mode);
        got.sort();
        assert_eq!(
            got,
            ["SOKF_FORMAT_30", "SOKF_FORMAT_76", "SOKF_FORMAT_87"],
            "{mode:?}"
        );
    }
}

// A pattern of `current-links` that names a spelling which the reading of
// a paragraph never holds draws one warning for each such run, on the line
// of the pattern, as the engine warns about a pattern of its own.
// sokf-tests: SOKF_SCHEMA_110
// sokf-tests: EXT-2_AC-5
#[test]
fn a_promise_pattern_in_another_spelling_is_warned_about() {
    let tree = TempTree::fixture("new");
    tree.write("knowledge/schemas/plan.md", PLAN_SCHEMA);
    tree.write(
        "knowledge/schemas/page.md",
        &PAGE_SCHEMA.replace(
            "line-limit: 0\n",
            "line-limit: 0\ncurrent-links:\n  - '^\\*Planned:\\* '\n  - '^\\*\\*Planned:\\*\\* '\n  - '^__Due:__ |^Planned: '\n",
        ),
    );
    let report = tree.validate(false, CacheMode::Off);
    let warned: Vec<(Option<usize>, String)> = report
        .warnings
        .iter()
        .filter(|f| f.path.as_str() == "/knowledge/schemas/page.md")
        .inspect(|f| assert_eq!(f.criterion, "SOKF_SCHEMA_110"))
        .map(|f| (f.line, f.message.clone()))
        .collect();
    assert_eq!(
        warned,
        [
            (
                Some(10),
                "the pattern `^\\*Planned:\\* ` names `*Planned:* `, which reads `_Planned:_ `; a pattern reads the fixed spelling, so it never matches that text outside a code span".to_string()
            ),
            (
                Some(12),
                "the pattern `^__Due:__ |^Planned: ` names `__Due:__ `, which reads `**Due:** `; a pattern reads the fixed spelling, so it never matches that text outside a code span".to_string()
            ),
        ],
        "{}",
        json(&report)
    );
    assert!(
        report
            .errors
            .iter()
            .all(|f| f.path.as_str() != "/knowledge/schemas/page.md"),
        "{}",
        json(&report)
    );
}
