//! These property suites test the incremental run, which contract-cache
//! specifies. Each case applies random edits to a fixture. After each
//! edit, the report of a warm run must equal the report of a run with no
//! cache.

use std::fs;

use common::{
    TempTree, age,
    incremental::{cold, edit, graph_json, run},
};
use proptest::prelude::*;
use sokf_core::CacheMode;

use crate::common;

/// A `Step` is one edit that a property case can make to the clean fixture.
#[derive(Debug, Clone)]
enum Step {
    Append(usize, String),
    Retitle(usize, String),
    RenameId(usize),
    Cite(usize, usize),
    Touch(usize),
    Delete(usize),
    Create(String),
    Schema,
    SchemaRule,
    SchemaType,
    Formats(String),
    Rename,
    IndexEdit(usize),
    CreateDeep(String),
    RemoveDeep,
    Readme,
    Source,
}

const INDEXES: [&str; 3] = [
    "knowledge/index.md",
    "knowledge/decisions/index.md",
    "knowledge/issues/open/index.md",
];

const DOCS: [&str; 4] = [
    "knowledge/overview.md",
    "knowledge/req-cli.md",
    "knowledge/decisions/decision-single-binary.md",
    "knowledge/issues/open/issue-001-retry.md",
];
const IDS: [&str; 4] = [
    "overview",
    "req-cli",
    "decision-single-binary",
    "issue-001-retry",
];

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0..4usize, "[a-z ]{1,12}").prop_map(|(d, t)| Step::Append(d, t)),
        (0..4usize, "[A-Z][a-z ]{1,10}").prop_map(|(d, t)| Step::Retitle(d, t)),
        (0..4usize).prop_map(Step::RenameId),
        (0..4usize, 0..4usize).prop_map(|(d, t)| Step::Cite(d, t)),
        (0..4usize).prop_map(Step::Touch),
        (0..4usize).prop_map(Step::Delete),
        "[a-z]{3,6}".prop_map(Step::Create),
        Just(Step::Schema),
        Just(Step::SchemaRule),
        Just(Step::SchemaType),
        "[a-z]{2,5}".prop_map(Step::Formats),
        Just(Step::Rename),
        (0..3usize).prop_map(Step::IndexEdit),
        "[a-z]{3,6}".prop_map(Step::CreateDeep),
        Just(Step::RemoveDeep),
        Just(Step::Readme),
        Just(Step::Source),
    ]
}

#[expect(clippy::too_many_lines, reason = "one arm for each kind of step")]
fn apply(tree: &TempTree, step: &Step) {
    let path = |d: usize| tree.dir().join(DOCS[d]);
    match step {
        Step::Append(d, text) => {
            if path(*d).exists() {
                let t = tree.read(DOCS[*d]);
                let edited = match t.split_once("<!-- sokf:links -->") {
                    Some((head, tail)) => format!("{head}{text}\n\n<!-- sokf:links -->{tail}"),
                    None => format!("{t}\n{text}\n"),
                };
                edit(tree, DOCS[*d], &edited);
            }
        }
        Step::Retitle(d, title) => {
            if path(*d).exists() {
                let t = tree.read(DOCS[*d]);
                let mut lines: Vec<String> = t.lines().map(str::to_string).collect();
                if let Some(l) = lines.iter_mut().find(|l| l.starts_with("title: ")) {
                    *l = format!("title: {title}");
                }
                edit(tree, DOCS[*d], &(lines.join("\n") + "\n"));
            }
        }
        Step::RenameId(d) => {
            if path(*d).exists() {
                let t = tree.read(DOCS[*d]);
                edit(
                    tree,
                    DOCS[*d],
                    &t.replacen(
                        &format!("id: {}", IDS[*d]),
                        &format!("id: {}-x", IDS[*d]),
                        1,
                    ),
                );
            }
        }
        Step::Cite(d, t) => {
            if path(*d).exists() {
                let text = tree.read(DOCS[*d]);
                let line = format!("See [that][sokf:{}].", IDS[*t]);
                let edited = match text.split_once("<!-- sokf:links -->") {
                    Some((head, tail)) => format!("{head}{line}\n\n<!-- sokf:links -->{tail}"),
                    None => format!("{text}\n{line}\n"),
                };
                edit(tree, DOCS[*d], &edited);
            }
        }
        Step::Touch(d) => {
            if path(*d).exists() {
                let t = tree.read(DOCS[*d]);
                edit(tree, DOCS[*d], &t);
            }
        }
        Step::Delete(d) => {
            let _ = fs::remove_file(path(*d));
        }
        Step::Create(slug) => edit(
            tree,
            &format!("knowledge/{slug}.md"),
            &format!(
                "---\ntype: Decision\nid: {slug}\ntitle: {slug}\ndescription: Made up.\n---\n\n# {slug}\n\n## Context\n\nSome.\n\n## Decision\n\nSome.\n\n## Consequences\n\n- Some.\n"
            ),
        ),
        Step::Schema => {
            let t = tree.read("knowledge/schemas/decision.md");
            edit(tree, "knowledge/schemas/decision.md", &format!("{t}\n"));
        }
        Step::SchemaRule => {
            let t = tree.read("knowledge/schemas/decision.md");
            let edited = if t.contains("max: 8") {
                t.replacen("max: 8", "max: 1", 1)
            } else {
                t.replacen("max: 1", "max: 8", 1)
            };
            edit(tree, "knowledge/schemas/decision.md", &edited);
        }
        Step::SchemaType => {
            let t = tree.read("knowledge/schemas/decision.md");
            let edited = if t.contains("const: Decision\n") {
                t.replacen("const: Decision\n", "const: Choice\n", 1)
            } else {
                t.replacen("const: Choice\n", "const: Decision\n", 1)
            };
            edit(tree, "knowledge/schemas/decision.md", &edited);
        }
        Step::Formats(name) => edit(
            tree,
            ".sokf/formats.yaml",
            &format!("registry: {{}}\ntypes:\n  {name}: '[a-z]+'\n"),
        ),
        Step::Rename => {
            let t = tree.read("knowledge/manifest.sokf.yaml");
            let edited = if t.contains("name: fixture\n") {
                t.replacen("name: fixture\n", "name: fixtures\n", 1)
            } else {
                t.replacen("name: fixtures\n", "name: fixture\n", 1)
            };
            edit(tree, "knowledge/manifest.sokf.yaml", &edited);
        }
        Step::IndexEdit(i) => {
            let t = tree.read(INDEXES[*i]);
            let edited = match t.strip_suffix("\nnot generated\n") {
                Some(original) => format!("{original}\n"),
                None => format!("{t}\nnot generated\n"),
            };
            edit(tree, INDEXES[*i], &edited);
        }
        Step::CreateDeep(slug) => edit(
            tree,
            &format!("knowledge/deep/{slug}.md"),
            &format!(
                "---\ntype: Decision\nid: deep-{slug}\ntitle: {slug}\ndescription: Made up.\n---\n\n# {slug}\n\n## Context\n\nSome.\n\n## Decision\n\nSome.\n\n## Consequences\n\n- Some.\n"
            ),
        ),
        Step::RemoveDeep => {
            let _ = fs::remove_dir_all(tree.dir().join("knowledge/deep"));
        }
        Step::Readme => {
            let t = tree.read("README.md");
            let edited = match t.strip_suffix("\nA line more.\n") {
                Some(original) => format!("{original}\n"),
                None => format!("{t}\nA line more.\n"),
            };
            edit(tree, "README.md", &edited);
        }
        Step::Source => {
            // The edit goes inside the region that the readme includes, so
            // the include goes stale. A line added at the end of the file
            // would change nothing that the include copies.
            let t = tree.read("src/main.rs");
            edit(
                tree,
                "src/main.rs",
                &t.replacen("fn cli()", "// changed\nfn cli()", 1),
            );
        }
    }
}

// sokf-tests: CACHE_P-1
// sokf-tests: CLI_P-1
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(24))]

    #[test]
    fn a_warm_run_equals_a_cold_run_after_any_edits(steps in prop::collection::vec(step(), 1..5)) {
        let tree = TempTree::fixture("clean");
        age(tree.dir(), 10);
        let (_, _, warm, _) = run(&tree);
        prop_assert_eq!(warm, cold(&tree));
        for step in &steps {
            apply(&tree, step);
            // Before the check, the cached graph comes from the store and
            // the changed documents, and it must equal the graph built
            // without the cache. After the check, the two graphs must still
            // be equal.
            prop_assert_eq!(graph_json(&tree, CacheMode::On), graph_json(&tree, CacheMode::Off), "graph before {:?}", step);
            let (_, _, warm, _) = run(&tree);
            prop_assert_eq!(warm, cold(&tree), "after {:?}", step);
            prop_assert_eq!(graph_json(&tree, CacheMode::On), graph_json(&tree, CacheMode::Off), "graph after {:?}", step);
        }
    }
}

/// A `GraphStep` is one edit that a property case can make to the graph
/// fixture, which carries findings of its own.
#[derive(Debug, Clone)]
enum GraphStep {
    Retitle(usize, String),
    Append(usize, String),
    Link(usize, usize),
    Delete(usize),
    Create(String),
}

const GRAPH_DOCS: [&str; 3] = [
    "knowledge/config.md",
    "knowledge/core/planner.md",
    "knowledge/core/executor.md",
];
const GRAPH_IDS: [&str; 3] = ["config", "planner", "executor"];

fn graph_step() -> impl Strategy<Value = GraphStep> {
    prop_oneof![
        (0..3usize, "[A-Z][a-z ]{1,10}").prop_map(|(d, t)| GraphStep::Retitle(d, t)),
        (0..3usize, "[a-z ]{1,12}").prop_map(|(d, t)| GraphStep::Append(d, t)),
        (0..3usize, 0..3usize).prop_map(|(d, t)| GraphStep::Link(d, t)),
        (0..3usize).prop_map(GraphStep::Delete),
        "[a-z]{3,6}".prop_map(GraphStep::Create),
    ]
}

fn apply_graph(tree: &TempTree, step: &GraphStep) {
    let path = |d: usize| tree.dir().join(GRAPH_DOCS[d]);
    let insert = |d: usize, line: &str| {
        let text = tree.read(GRAPH_DOCS[d]);
        let edited = match text.split_once("<!-- sokf:links -->") {
            Some((head, tail)) => format!("{head}{line}\n\n<!-- sokf:links -->{tail}"),
            None => format!("{text}\n{line}\n"),
        };
        edit(tree, GRAPH_DOCS[d], &edited);
    };
    match step {
        GraphStep::Retitle(d, title) => {
            if path(*d).exists() {
                let t = tree.read(GRAPH_DOCS[*d]);
                let mut lines: Vec<String> = t.lines().map(str::to_string).collect();
                match lines.iter_mut().find(|l| l.starts_with("title: ")) {
                    Some(l) => *l = format!("title: {title}"),
                    None => lines.insert(2, format!("title: {title}")),
                }
                edit(tree, GRAPH_DOCS[*d], &(lines.join("\n") + "\n"));
            }
        }
        GraphStep::Append(d, text) => {
            if path(*d).exists() {
                insert(*d, text);
            }
        }
        GraphStep::Link(d, t) => {
            if path(*d).exists() {
                insert(*d, &format!("See [that][sokf:{}].", GRAPH_IDS[*t]));
            }
        }
        GraphStep::Delete(d) => {
            let _ = fs::remove_file(path(*d));
        }
        GraphStep::Create(slug) => edit(
            tree,
            &format!("knowledge/core/{slug}.md"),
            &format!(
                "---\ntype: Subsystem\nid: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\nSee [it][sokf:config].\n"
            ),
        ),
    }
}

// sokf-tests: CACHE_P-1
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(16))]

    // sokf-tests: CLI_P-1
    #[test]
    fn a_warm_run_equals_a_cold_run_on_a_tree_with_findings(steps in prop::collection::vec(graph_step(), 1..5)) {
        let tree = TempTree::fixture("graph");
        age(tree.dir(), 10);
        let (_, _, warm, _) = run(&tree);
        prop_assert_eq!(warm, cold(&tree));
        for step in &steps {
            apply_graph(&tree, step);
            let (_, _, warm, _) = run(&tree);
            prop_assert_eq!(warm, cold(&tree), "after {:?}", step);
        }
    }
}
