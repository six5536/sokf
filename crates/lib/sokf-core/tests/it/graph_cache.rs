//! These tests compare the graph built with the cache against the graph
//! built without it, after edits that change what the edges of an unchanged
//! document resolve to (CACHE_P-1).

use common::{
    TempTree,
    graph_rows::{graph_of, query},
};
use sokf_core::{CacheMode, graph::bounded};

use crate::common;

/// `graph` returns the bounded JSON of the graph from `start` at `depth`,
/// or the error that the query returns.
fn graph(tree: &TempTree, start: &[&str], depth: usize, cache: CacheMode) -> String {
    match graph_of(tree, query(start, depth), cache) {
        Ok((g, r)) => serde_json::to_string_pretty(&bounded(&g, &r)).unwrap(),
        Err(e) => format!("error: {e}"),
    }
}

/// `same` asserts that the graph from each start is the same at depths 1 and
/// 2 with the cache as without it, and that `validate` prints the same report.
/// `step` names the step in a failure message.
fn same(tree: &TempTree, starts: &[&[&str]], step: &str) {
    for start in starts {
        for depth in 1..3 {
            assert_eq!(
                graph(tree, start, depth, CacheMode::On),
                graph(tree, start, depth, CacheMode::Off),
                "{step}: {start:?} at depth {depth}"
            );
        }
    }
    assert_eq!(
        common::json(&tree.validate(false, CacheMode::On)),
        common::json(&tree.validate(false, CacheMode::Off)),
        "{step}: validate"
    );
}

// sokf-tests: CACHE_P-1
// sokf-tests: CACHE-1_AC-38
// sokf-tests: CACHE-1_AC-44
// sokf-tests: CACHE-1_AC-45
#[test]
fn a_file_and_an_outside_directory_print_the_same_with_the_cache() {
    let tree = TempTree::fixture("graph");
    common::age(tree.dir(), 10);
    same(
        &tree,
        &[
            &["crates/lib/example-core/src/planner.rs"],
            &["crates/lib/example-core/src"],
            &["crates"],
            &["planner", "crates/app/sokf/src/main.rs"],
        ],
        "first",
    );
}

// sokf-tests: CACHE_P-1
#[test]
fn a_file_that_leaves_an_additional_files_glob_loses_its_edges() {
    let tree = TempTree::fixture("clean");
    common::age(tree.dir(), 10);
    let starts: &[&[&str]] = &[&["src/main.rs"], &["README.md"]];
    same(&tree, starts, "before");
    let schema = "knowledge/schemas/readme.md";
    let text = tree.read(schema);
    tree.write(
        schema,
        &text.replacen(
            "additional-files: /README.md",
            "additional-files: /NOPE.md",
            1,
        ),
    );
    same(&tree, starts, "after");
}

// sokf-tests: CACHE_P-1
#[test]
fn an_asset_that_comes_back_is_what_its_sidecar_names_again() {
    let tree = TempTree::fixture("assets");
    common::age(tree.dir(), 10);
    let starts: &[&[&str]] = &[&["album"], &["rates"], &["knowledge/data/rates.csv"]];
    same(&tree, starts, "before");
    let asset = "knowledge/data/rates.csv";
    let text = tree.read(asset);
    std::fs::remove_file(tree.dir().join(asset)).unwrap();
    same(&tree, starts, "gone");
    tree.write(asset, &text);
    same(&tree, starts, "back");
}
