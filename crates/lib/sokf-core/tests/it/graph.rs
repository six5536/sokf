//! These tests run `api::graph` on a fixture with one query for each
//! scenario and compare the bounded object with a JSON snapshot. They also
//! test `api::locate`.

use std::path::PathBuf;

use common::{
    TempTree,
    graph_rows::{graph_of, query},
};
use sokf_core::{CacheMode, Error, Expr, GraphQuery, graph::bounded, locate};

use crate::common;

fn exprs(sources: &[&str]) -> Vec<Expr> {
    sources.iter().map(|s| Expr::compile(s).unwrap()).collect()
}

/// `json` runs `query` on `tree` with the cache off and returns the bounded
/// object as pretty JSON, for a snapshot.
fn json(tree: &TempTree, query: GraphQuery) -> String {
    json_with(tree, query, CacheMode::Off)
}

fn json_with(tree: &TempTree, query: GraphQuery, cache: CacheMode) -> String {
    let (graph, result) = graph_of(tree, query, cache).unwrap();
    serde_json::to_string_pretty(&bounded(&graph, &result)).unwrap()
}

// sokf-tests: CACHE-1_AC-22
// sokf-tests: CACHE_P-1
#[test]
fn the_graph_through_the_store_equals_the_cold_graph() {
    let tree = TempTree::fixture("graph");
    common::age(tree.dir(), 10);
    tree.validate(false, CacheMode::On);
    let queries = || {
        vec![
            query(&[], 0),
            query(&[], 1),
            query(&["planner"], 2),
            query(&["knowledge/core"], 1),
            query(&["#nothing-here"], 1),
            GraphQuery {
                selector: exprs(&["type=='Subsystem'"]),
                ..query(&[], 1)
            },
            GraphQuery {
                to: exprs(&["status=='stable'"]),
                ..query(&["planner"], 1)
            },
        ]
    };
    let same = |tree: &TempTree| {
        for q in queries() {
            let warm = graph_of(tree, q.clone(), CacheMode::On);
            let cold = graph_of(tree, q.clone(), CacheMode::Off);
            match (warm, cold) {
                (Ok((wg, wr)), Ok((cg, cr))) => assert_eq!(
                    serde_json::to_string_pretty(&bounded(&wg, &wr)).unwrap(),
                    serde_json::to_string_pretty(&bounded(&cg, &cr)).unwrap(),
                    "{:?}",
                    q.start
                ),
                (Err(w), Err(c)) => assert_eq!(w.to_string(), c.to_string()),
                (w, c) => panic!("{:?}: warm {w:?} against cold {c:?}", q.start),
            }
        }
    };
    same(&tree);
    // Add a link and run no check: the graph computes the edges of the
    // changed document and reads the other edges from the store.
    let rel = "knowledge/core/executor.md";
    let text = tree.read(rel);
    tree.write(
        rel,
        &text.replacen(
            "Runs the plan the [planner][sokf:planner] produces.",
            "Runs the plan the [planner][sokf:planner] produces, with [config][sokf:config].",
            1,
        ),
    );
    same(&tree);
    // Remove a document: its edges disappear, and so does the target that
    // the executor linked to.
    std::fs::remove_file(tree.dir().join("knowledge/config.md")).unwrap();
    same(&tree);
    tree.validate(false, CacheMode::On);
    same(&tree);
    assert_ne!(
        json_with(&tree, query(&[], 1), CacheMode::On),
        String::new()
    );
}

// sokf-tests: CLI-7_AC-1
// sokf-tests: CLI-7_AC-2
// sokf-tests: CLI-7_AC-3
// sokf-tests: CLI-7_AC-5
// sokf-tests: CLI-7_AC-6
// sokf-tests: CLI-7_AC-12
// sokf-tests: CLI-7_AC-13
// sokf-tests: CLI-7_AC-14
// sokf-tests: CLI-7_AC-39
#[test]
fn the_example_knowledge_at_every_depth() {
    let tree = TempTree::fixture("graph");
    // Without `--depth`, the layout shows no edge, so depth 0 and depth 1
    // give the same graph.
    insta::assert_snapshot!("no_start_depth_0", json(&tree, query(&[], 0)));
    assert_eq!(json(&tree, query(&[], 1)), json(&tree, query(&[], 0)));
    let deep = GraphQuery {
        layout_edges: true,
        ..query(&[], 1)
    };
    assert!(json(&tree, deep).contains("\"rel\": \"depends-on\""));
    let every = |depth| query(&["config", "executor", "planner"], depth);
    insta::assert_snapshot!("every_document_depth_1", json(&tree, every(1)));
    // Depth 3 adds nothing to depth 2, because every document target is
    // already a row.
    assert_eq!(json(&tree, every(3)), json(&tree, every(2)));
}

// sokf-tests: CLI-7_AC-4
// sokf-tests: CLI-7_AC-35
// sokf-tests: CLI-7_AC-37
// sokf-tests: CLI_P-13
#[test]
fn starts_by_id_by_path_by_file_and_by_outside_directory() {
    let tree = TempTree::fixture("graph");
    let by_id = json(&tree, query(&["planner"], 1));
    insta::assert_snapshot!("planner_depth_1", by_id);
    assert_eq!(json(&tree, query(&["knowledge/core/planner.md"], 1)), by_id);
    assert_eq!(
        json(&tree, query(&["/knowledge/core/planner.md"], 1)),
        by_id
    );
    insta::assert_snapshot!(
        "file_start",
        json(&tree, query(&["crates/lib/example-core/src/planner.rs"], 2))
    );
    insta::assert_snapshot!("outside_directory", json(&tree, query(&["crates"], 0)));
    insta::assert_snapshot!("repository_root", json(&tree, query(&["."], 1)));
}

// sokf-tests: CLI-7_AC-8
// sokf-tests: CLI-7_AC-9
// sokf-tests: CLI-7_AC-10
// sokf-tests: CLI-7_AC-46
#[test]
fn each_predicate_position_and_a_predicate_that_matches_nothing() {
    let tree = TempTree::fixture("graph");
    let selected = GraphQuery {
        selector: exprs(&["id=='planner'", "id=='config'"]),
        ..query(&[], 0)
    };
    insta::assert_snapshot!("selector", json(&tree, selected));
    let edges = GraphQuery {
        edge: exprs(&["rel=='depended-on-by'"]),
        ..query(&["planner"], 1)
    };
    insta::assert_snapshot!("edge", json(&tree, edges));
    let to = GraphQuery {
        to: exprs(&["title=='Configuration'"]),
        ..query(&["planner"], 1)
    };
    insta::assert_snapshot!("to", json(&tree, to));
    let nothing = GraphQuery {
        selector: exprs(&["type=='Nope'"]),
        ..query(&[], 1)
    };
    assert_eq!(
        json(&tree, nothing),
        "{\n  \"start\": [\n    \"/knowledge\"\n  ],\n  \"nodes\": [],\n  \"edges\": [],\n  \"summary\": {\n    \"documents\": {\n      \"shown\": 0,\n      \"total\": 4\n    },\n    \"archived\": {\n      \"shown\": 0,\n      \"total\": 0\n    }\n  }\n}"
    );
}

// sokf-tests: CLI_P-14
#[test]
fn a_query_projects_the_bounded_object() {
    let tree = TempTree::fixture("graph");
    let q = GraphQuery {
        query: Some(Expr::compile("nodes[?kind=='document'].id").unwrap()),
        ..query(&[], 1)
    };
    let (_, result) = graph_of(&tree, q, CacheMode::Off).unwrap();
    assert_eq!(result.rows, [] as [sokf_core::Row; 0]);
    assert_eq!(
        result.projection,
        Some(serde_json::json!([
            "config",
            "executor",
            "planner",
            "subsystem"
        ]))
    );
}

// sokf-tests: CLI-7_AC-19
// sokf-tests: CLI-7_AC-33
#[test]
fn an_ambiguous_start_is_refused() {
    let tree = TempTree::fixture("graph");
    // Make `planner` both an id and a path.
    tree.write("planner", "");
    let err = graph_of(&tree, query(&["planner"], 1), CacheMode::Off).unwrap_err();
    assert!(matches!(err, Error::Start(_)));
    assert_eq!(
        err.to_string(),
        "graph: `planner` is both an id (/knowledge/core/planner.md) and a path (/planner)"
    );
    let two = TempTree::fixture("two-roots");
    let err = graph_of(&two, query(&["shared"], 1), CacheMode::Off)
        .unwrap_err()
        .to_string();
    assert_eq!(
        err,
        "graph: `shared` identifies more than one document: /knowledge/shared.md, /other/shared.md. Write `first:shared` or `second:shared`"
    );
}

// sokf-tests: CLI-7_AC-3
#[test]
fn two_roots_are_one_graph_with_a_cross_root_path_link() {
    let tree = TempTree::fixture("two-roots");
    insta::assert_snapshot!(
        "two_roots",
        json(&tree, query(&["/knowledge/shared.md"], 1))
    );
}

// sokf-tests: CLI-7_AC-3
#[test]
fn no_start_starts_from_each_root_in_the_order_of_the_config() {
    let tree = TempTree::fixture("two-roots");
    let starts = |tree: &TempTree| {
        let (_, result) = graph_of(tree, query(&[], 0), CacheMode::Off).unwrap();
        let rows: Vec<String> = result
            .rows
            .iter()
            .map(|r| r.node.as_str().to_string())
            .collect();
        (result.start, rows)
    };
    let (start, rows) = starts(&tree);
    assert_eq!(rows, ["/knowledge", "/other"]);
    assert_eq!(
        start
            .iter()
            .map(sokf_core::NodeKey::as_str)
            .collect::<Vec<_>>(),
        rows
    );
    // The order of the roots in the config, not the order of their paths,
    // sets the order of the rows.
    tree.write(
        ".sokf/config.toml",
        "[knowledge]\nroots = [\"/other\", \"/knowledge\"]\n",
    );
    let (start, rows) = starts(&tree);
    assert_eq!(rows, ["/other", "/knowledge"]);
    assert_eq!(
        start
            .iter()
            .map(sokf_core::NodeKey::as_str)
            .collect::<Vec<_>>(),
        rows
    );
}

// sokf-tests: CLI-8_AC-1
// sokf-tests: CLI-8_AC-6
// sokf-tests: CLI-8_AC-7
// sokf-tests: CLI-8_AC-11
#[test]
fn locate_prints_absolute_paths_or_fails_whole() {
    let tree = TempTree::fixture("graph");
    let paths = locate(&tree.root(), &["planner".into()], false, CacheMode::Off).unwrap();
    assert_eq!(paths[0].path, tree.dir().join("knowledge/core/planner.md"));
    assert_eq!(paths[0].id, "planner");
    assert!(paths[0].asset.is_none());
    assert!(paths[0].path.is_absolute() && paths[0].path.exists());
    let paths = locate(
        &tree.root(),
        &["planner".into(), "config".into()],
        false,
        CacheMode::Off,
    )
    .unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[1].path, tree.dir().join("knowledge/config.md"));
    let err = locate(
        &tree.root(),
        &["planner".into(), "planer".into()],
        false,
        CacheMode::Off,
    )
    .unwrap_err();
    assert!(matches!(err, Error::Locate(_)));
    assert_eq!(err.to_string(), "locate: `planer` identifies no document");
    // `--asset` fails on an id that doesn't identify a sidecar.
    let err = locate(&tree.root(), &["planner".into()], true, CacheMode::Off).unwrap_err();
    assert_eq!(
        err.to_string(),
        "locate: `planner` identifies /knowledge/core/planner.md, which is not a sidecar and has no asset"
    );
    let two = TempTree::fixture("two-roots");
    let err = locate(&two.root(), &["shared".into()], false, CacheMode::Off).unwrap_err();
    assert_eq!(
        err.to_string(),
        "locate: `shared` identifies more than one document: /knowledge/shared.md, /other/shared.md. Write `first:shared` or `second:shared`"
    );
    let only: Vec<PathBuf> = locate(&two.root(), &["only".into()], false, CacheMode::Off)
        .unwrap()
        .into_iter()
        .map(|l| l.path)
        .collect();
    assert_eq!(only, vec![two.dir().join("other/only.md")]);
}

// sokf-tests: CLI_P-1
#[test]
fn a_warm_cache_gives_the_same_graph() {
    let tree = TempTree::fixture("graph");
    let run = |cache| graph_of(&tree, query(&["planner", "knowledge"], 2), cache).unwrap();
    let (g1, r1) = run(CacheMode::On);
    assert!(tree.dir().join(".sokf/cache/.gitignore").exists());
    let (g2, r2) = run(CacheMode::On);
    let (g3, r3) = run(CacheMode::Off);
    assert_eq!(bounded(&g1, &r1), bounded(&g2, &r2));
    assert_eq!(bounded(&g1, &r1), bounded(&g3, &r3));
}

// sokf-tests: CLI-7_AC-25
// sokf-tests: CLI-8_AC-4
#[test]
fn assets_are_nodes_in_the_layout_and_locate_finds_both_files() {
    let tree = TempTree::fixture("assets");
    insta::assert_snapshot!("assets_depth_1", json(&tree, query(&[], 1)));
    insta::assert_snapshot!(
        "asset_start",
        json(&tree, query(&["knowledge/media/draft/flowers.png"], 2))
    );
    // A selector leaves every asset out of the layout, and `--to` hides an
    // edge to an asset.
    let selected = GraphQuery {
        selector: exprs(&["type=='Image'"]),
        ..query(&[], 0)
    };
    let text = json(&tree, selected);
    assert!(!text.contains("\"kind\": \"asset\""), "{text}");
    let to = GraphQuery {
        to: exprs(&["type=='Note'"]),
        ..query(&["flowers"], 1)
    };
    let text = json(&tree, to);
    assert!(!text.contains("\"rel\": \"describes\""), "{text}");
    // `locate` finds the sidecar, and with `--asset` it finds the asset.
    let found = locate(&tree.root(), &["flowers".into()], false, CacheMode::Off).unwrap();
    assert_eq!(
        found[0].path,
        tree.dir().join("knowledge/media/draft/flowers.png.md")
    );
    assert_eq!(
        found[0].asset.as_deref(),
        Some(
            tree.dir()
                .join("knowledge/media/draft/flowers.png")
                .as_path()
        )
    );
    let found = locate(&tree.root(), &["flowers".into()], true, CacheMode::Off).unwrap();
    assert_eq!(
        found[0].asset,
        Some(tree.dir().join("knowledge/media/draft/flowers.png"))
    );
}
