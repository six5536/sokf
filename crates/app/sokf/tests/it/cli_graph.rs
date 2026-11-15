//! End-to-end tests for `sokf graph` and `sokf locate`. Each test runs the
//! real binary on a fixture and checks its text, JSON, and Mermaid output,
//! its standard error, and its exit code.

use std::{
    fs,
    path::{Path, PathBuf},
};

use common::{Fixture, run_in};

use crate::common;

fn design_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../knowledge/designs/design-graph.md")
}

fn design_example(marker: &str) -> String {
    let design = fs::read_to_string(design_path()).unwrap();
    let after = &design[design.find(marker).unwrap() + marker.len()..];
    let open = after.find("```").unwrap();
    let body = &after[open..];
    let body = &body[body.find('\n').unwrap() + 1..];
    body[..body.find("```").unwrap()].to_string()
}

// sokf-tests: CLI-7_AC-12
#[test]
fn graph_prints_the_three_examples_of_the_graph_design() {
    // The public mirror holds this test and not the design, so the test has
    // nothing to compare there.
    if !design_path().exists() {
        return;
    }
    let f = Fixture::new("graph");
    let (stdout, stderr, code) = f.run(&["graph"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, design_example("`sokf graph`:\n"));
    let (stdout, _, code) = f.run(&["graph", "planner", "--json"]);
    assert_eq!(code, 0);
    let expected: serde_json::Value =
        serde_json::from_str(&design_example("`sokf graph planner --json`")).unwrap();
    assert_eq!(
        stdout,
        format!("{}\n", serde_json::to_string(&expected).unwrap())
    );
    let (stdout, _, code) = f.run_here(&["graph", "planner", "--mermaid"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, design_example("`sokf graph planner --mermaid`:"));
    let (stdout, _, code) = f.run(&["graph", "--json", "--query", "nodes[?kind=='document'].id"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "[\"config\",\"executor\",\"planner\",\"subsystem\"]\n"
    );
}

// sokf-tests: CLI-7_AC-1
// sokf-tests: CLI-7_AC-2
// sokf-tests: CLI-7_AC-4
// sokf-tests: CLI-7_AC-6
// sokf-tests: CLI-7_AC-8
// sokf-tests: CLI-7_AC-18
#[test]
fn graph_starts_depths_and_predicates() {
    let f = Fixture::new("graph");
    let g = |args: &[&str]| {
        let mut full = vec!["--repo-root", f.root(), "graph"];
        full.extend_from_slice(args);
        run_in(&f.0, &full)
    };
    // With no start, the graph prints the layout: each root with its
    // description, and no edges. At depth 0 a document prints alone. At
    // depth 3 each target expands once.
    let (stdout, _, _) = g(&[]);
    assert_eq!(
        stdout,
        "/knowledge - The example knowledge of the graph design.\n├── config\n├── core/\n│   ├── executor\n│   └── planner\n└── schemas/\n    └── subsystem\n\n4 / 4 docs\n"
    );
    let (stdout, _, _) = g(&["planner", "--depth", "0"]);
    assert_eq!(stdout, "planner\n\n1 / 1 docs\n");
    let (deep, _, _) = g(&["planner", "--depth", "3"]);
    assert_eq!(
        deep,
        "planner\n├── includes → /crates/app/sokf/src/main.rs#cli\n│   └── included-by #cli → planner (*)\n├── depends-on → config\n│   └── depended-on-by → planner (*)\n├── depended-on-by → executor\n│   └── depends-on → planner (*)\n├── references → planer\n└── describes → /crates/lib/example-core/src/planner.rs\n    └── described-by → planner (*)\n\n3 / 3 docs\n"
    );
    // A start can be a document's id or its path, with or without the
    // leading slash, or a file.
    let (by_id, _, _) = g(&["planner"]);
    assert_eq!(
        by_id,
        "planner\n├── includes → /crates/app/sokf/src/main.rs#cli\n├── depends-on → config\n├── depended-on-by → executor\n├── references → planer\n└── describes → /crates/lib/example-core/src/planner.rs\n\n3 / 3 docs\n"
    );
    let (by_path, _, _) = g(&["knowledge/core/planner.md"]);
    assert_eq!(by_path, by_id);
    let (by_spec, _, _) = g(&["/knowledge/core/planner.md"]);
    assert_eq!(by_spec, by_id);
    let (stdout, _, _) = g(&["crates/lib/example-core/src/planner.rs", "--depth", "2"]);
    assert_eq!(
        stdout,
        "/crates/lib/example-core/src/planner.rs\n└── described-by → planner\n    ├── includes → /crates/app/sokf/src/main.rs#cli\n    ├── depends-on → config\n    ├── depended-on-by → executor\n    ├── references → planer\n    └── describes → /crates/lib/example-core/src/planner.rs (*)\n\n3 / 3 docs\n"
    );
    // A directory that is not inside a knowledge root prints its files as a
    // flat list, and prints each knowledge root inside it with that root's
    // layout.
    let (stdout, _, _) = g(&["crates"]);
    assert_eq!(
        stdout,
        "/crates\n├── /crates/app/sokf/src/main.rs\n└── /crates/lib/example-core/src/planner.rs\n\n0 / 0 docs\n"
    );
    let (stdout, _, _) = g(&["."]);
    assert!(stdout.starts_with("/\n├── /crates/app/sokf/src/main.rs\n├── /crates/lib/example-core/src/planner.rs\n└── /knowledge - The example knowledge of the graph design.\n    ├── config\n"), "{stdout}");
    // Two starts print one after the other, and each start's edges print
    // under it, not in the layout.
    let (stdout, _, _) = g(&["config", "planner", "--depth", "0"]);
    assert_eq!(stdout, "config\nplanner\n\n2 / 2 docs\n");
    let (stdout, _, _) = g(&["config", "knowledge"]);
    assert_eq!(
        stdout,
        "config\n└── depended-on-by → planner\n/knowledge - The example knowledge of the graph design.\n├── config\n├── core/\n│   ├── executor\n│   └── planner\n└── schemas/\n    └── subsystem\n\n4 / 4 docs\n"
    );
    // Each of `-l`, `--edge`, and `--to` takes a predicate, and a predicate
    // that matches nothing prints no documents.
    let (stdout, _, _) = g(&["-l", "type=='Subsystem'", "--format", "{id}  {title}"]);
    assert_eq!(
        stdout,
        "/knowledge - The example knowledge of the graph design.\n├── config  Configuration\n└── core/\n    ├── executor  Executor\n    └── planner  Planner\n\n3 / 4 docs\n"
    );
    let (stdout, _, _) = g(&["planner", "--edge", "rel=='depended-on-by'"]);
    assert_eq!(
        stdout,
        "planner\n└── depended-on-by → executor\n\n2 / 3 docs\n"
    );
    let (stdout, _, _) = g(&["planner", "--to", "id=='config'", "--to", "id=='executor'"]);
    assert_eq!(
        stdout,
        "planner\n├── depends-on → config\n└── depended-on-by → executor\n\n3 / 3 docs\n"
    );
    let (stdout, _, code) = g(&["-l", "type=='Nope'"]);
    assert_eq!((stdout.as_str(), code), ("0 / 4 docs\n", 0));
    // A broken document prints with its `[broken: …]` marker.
    f.write(
        "knowledge/bad.md",
        "---\ntype: Subsystem\ntitle: Bad\n---\n\n# Bad\n",
    );
    let (stdout, _, _) = g(&["knowledge/bad.md", "--format", "{id} {title}"]);
    assert_eq!(
        stdout,
        "/knowledge/bad.md Bad [broken: no id]\n\n1 / 1 docs\n"
    );
}

// sokf-tests: CLI-7_AC-19
// sokf-tests: CLI-7_AC-20
// sokf-tests: CLI-7_AC-21
#[test]
fn graph_refusals_exit_2_and_name_the_readings() {
    let f = Fixture::new("graph");
    let g = |args: &[&str]| {
        let mut full = vec!["--repo-root", f.root(), "graph"];
        full.extend_from_slice(args);
        run_in(&f.0, &full)
    };
    for (args, needle) in [
        (
            vec!["nothing"],
            "error: graph: `nothing` is neither an id nor a path in the repository",
        ),
        (
            vec!["planner", "--format", "{id"],
            "error: format: a `{` has no `}` after it",
        ),
        (vec!["-l", "type=="], "error: query: "),
        (vec!["--edge", "rel=="], "error: query: "),
        (vec!["--to", "id=="], "error: query: "),
        (vec!["--json", "--query", "nodes[?"], "error: query: "),
    ] {
        let (stdout, stderr, code) = g(&args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(stdout, "", "{args:?}");
        assert!(stderr.starts_with(needle), "{args:?}: {stderr}");
    }
    f.write("planner", "");
    let (_, stderr, code) = g(&["planner"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: graph: `planner` is both an id (/knowledge/core/planner.md) and a path (/planner)\n"
    );
    let two = Fixture::new("two-roots");
    let (_, stderr, code) = two.run(&["graph", "shared"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: graph: `shared` identifies more than one document: /knowledge/shared.md, /other/shared.md. Write `first:shared` or `second:shared`\n"
    );
}

// sokf-tests: CLI-8_AC-1
// sokf-tests: CLI-8_AC-6
// sokf-tests: CLI-8_AC-8
#[test]
fn locate_prints_absolute_paths_or_nothing() {
    let f = Fixture::new("graph");
    let (stdout, stderr, code) = f.run(&["locate", "planner"]);
    assert_eq!(code, 0, "{stderr}");
    let expected = f.path("knowledge/core/planner.md");
    assert_eq!(stdout, format!("{}\n", expected.display()));
    assert!(expected.is_absolute() && expected.exists());
    let (stdout, _, code) = f.run(&["locate", "planner", "config"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.lines().count(), 2);
    let expected = f.path("knowledge/config.md");
    assert!(
        stdout.ends_with(&format!("{}\n", expected.display())),
        "{stdout}"
    );
    let (stdout, stderr, code) = f.run(&["locate", "planner", "planer"]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert_eq!(stderr, "error: locate: `planer` identifies no document\n");
    let two = Fixture::new("two-roots");
    let (stdout, stderr, code) = two.run(&["locate", "shared"]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "error: locate: `shared` identifies more than one document: /knowledge/shared.md, /other/shared.md. Write `first:shared` or `second:shared`\n"
    );
    // A qualified id identifies a document in one named store.
    let (stdout, stderr, code) = two.run(&["locate", "second:shared"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        format!("{}\n", two.path("other/shared.md").display())
    );
}

// sokf-tests: CLI-8_AC-4
// sokf-tests: CLI-4_AC-24
// sokf-tests: CLI-8_AC-3
// sokf-tests: CLI-8_AC-4
// sokf-tests: CLI-8_AC-5
// sokf-tests: CLI-8_AC-7
// sokf-tests: CLI-8_AC-8
#[test]
fn locate_finds_a_sidecar_and_its_asset_and_fix_moves_the_pair() {
    let f = Fixture::new("assets");
    let sidecar = f.path("knowledge/media/draft/flowers.png.md");
    let asset = f.path("knowledge/media/draft/flowers.png");
    let (stdout, stderr, code) = f.run(&["locate", "flowers"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, format!("{}\n", sidecar.display()));
    let (stdout, _, code) = f.run(&["locate", "--asset", "flowers"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, format!("{}\n", asset.display()));
    let (stdout, _, code) = f.run(&["locate", "--json", "flowers", "album"]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value[0]["id"], "flowers");
    assert_eq!(value[0]["path"], sidecar.display().to_string());
    assert_eq!(value[0]["asset"], asset.display().to_string());
    assert_eq!(value[1]["id"], "album");
    assert!(value[1].get("asset").is_none(), "{stdout}");
    let (stdout, stderr, code) = f.run(&["locate", "--asset", "album"]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "error: locate: `album` identifies /knowledge/notes/album.md, which is not a sidecar and has no asset\n"
    );
    let (_, stderr, code) = f.run(&["locate", "--asset", "--json", "album"]);
    assert_eq!(code, 2, "{stderr}");
    // `--fix` moves an asset and its sidecar together, and prints one
    // `moved` line for each file.
    let (stdout, _, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(
        stdout.starts_with(
            "moved   /knowledge/media/draft/sunset.png -> /knowledge/media/published/sunset.png\n\
             moved   /knowledge/media/draft/sunset.png.md -> /knowledge/media/published/sunset.png.md\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("rewrote /knowledge/media/draft/stale.png.md\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("/knowledge/media/draft/orphan.png: error: the asset has no sidecar"),
        "{stdout}"
    );
}

// sokf-tests: CLI-7_AC-25
// sokf-tests: CLI-7_AC-26
#[test]
fn graph_prints_an_asset_once_by_its_sidecar_and_as_a_cylinder() {
    let f = Fixture::new("assets");
    let g = |args: &[&str]| {
        let mut full = vec!["--repo-root", f.root(), "graph"];
        full.extend_from_slice(args);
        run_in(&f.0, &full)
    };
    // The layout lists the sidecar, not its asset.
    let (stdout, stderr, code) = g(&["/knowledge/data"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "/knowledge/data\n└── rates\n\n1 / 1 docs\n");
    // An asset that is a target prints as the id of its sidecar. An asset
    // that is a start prints as its path.
    let (stdout, _, _) = g(&["rates"]);
    assert_eq!(
        stdout,
        "rates\n├── includes → rates\n├── includes → rates#q3\n├── relates-to → album\n└── describes → rates\n\n2 / 2 docs\n"
    );
    let (stdout, _, _) = g(&["knowledge/data/rates.csv"]);
    assert_eq!(
        stdout,
        "/knowledge/data/rates.csv\n├── included-by → rates\n├── included-by #q3 → rates\n├── included-by #q3 → album\n├── included-by → broken\n└── described-by → rates\n\n3 / 3 docs\n"
    );
    let (stdout, _, code) = g(&["rates", "--mermaid"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("n1[(\"rates\")]"), "{stdout}");
}

// sokf-tests: CLI-7_AC-9
// sokf-tests: CLI-7_AC-10
#[test]
fn edge_and_to_predicates_apply_at_every_hop() {
    let f = Fixture::new("graph");
    let g = |args: &[&str]| {
        let mut full = vec!["--repo-root", f.root(), "graph"];
        full.extend_from_slice(args);
        run_in(&f.0, &full)
    };
    // At hop 2 the planner has five edges. The predicate keeps one.
    let (stdout, stderr, code) = g(&["executor", "--depth", "2", "--edge", "rel=='depends-on'"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "executor\n└── depends-on → planner\n    └── depends-on → config\n\n3 / 3 docs\n"
    );
    // At hop 2 only the edges to a Subsystem document stay.
    let (stdout, stderr, code) = g(&["executor", "--depth", "2", "--to", "type=='Subsystem'"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "executor\n└── depends-on → planner\n    ├── depends-on → config\n    └── depended-on-by → executor (*)\n\n3 / 3 docs\n"
    );
}

// sokf-tests: CLI-7_AC-16
// sokf-tests: CLI-7_AC-17
#[test]
fn graph_compiles_no_schema_and_runs_no_check() {
    let f = Fixture::new("graph");
    // The fixture gets a schema that breaks the grammar. It also gets a
    // document that fails its schema.
    let schema = fs::read_to_string(f.0.join("knowledge/schemas/subsystem.md")).unwrap();
    f.write(
        "knowledge/schemas/subsystem.md",
        &schema.replace("line-limit: 0\n", "line-limit: 0\nnonsense-rule: 1\n"),
    );
    f.write(
        "knowledge/core/loose.md",
        "---\ntype: Subsystem\nid: loose\ntitle: Loose\n---\n\nNo heading.\n",
    );
    let (stdout, _, code) = f.run(&["--no-cache", "validate"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("breaks the grammar"), "{stdout}");
    let (stdout, stderr, code) = f.run(&["--no-cache", "graph"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stderr, "");
    assert_eq!(
        stdout,
        "/knowledge - The example knowledge of the graph design.\n├── config\n├── core/\n│   ├── executor\n│   ├── loose\n│   └── planner\n└── schemas/\n    └── subsystem\n\n5 / 5 docs\n"
    );
}

// sokf-tests: CLI-7_AC-22
#[test]
fn a_query_that_fails_to_evaluate_exits_2() {
    let f = Fixture::new("graph");
    let (stdout, stderr, code) = f.run(&["graph", "--json", "--query", "abs(start)"]);
    assert_eq!(code, 2, "{stderr}");
    assert_eq!(stdout, "");
    assert!(stderr.starts_with("error: query: "), "{stderr}");
}

// sokf-tests: CLI-7_AC-32
// sokf-tests: CLI-7_AC-74
// sokf-tests: CACHE-1_AC-38
#[test]
fn a_key_start_prints_its_document_and_line_then_who_cites_it() {
    let f = Fixture::new("clean");
    let warm = f.run(&["graph", "#cli-1"]);
    let cold = f.run(&["--no-cache", "graph", "#cli-1"]);
    assert_eq!(warm.2, 0, "{}", warm.1);
    assert_eq!(
        warm.0,
        "CLI-1\n├── defined-in → req-cli (line 12)\n└── cited-by → overview\n\n2 / 2 docs\n"
    );
    assert_eq!(warm, cold);
    // The key as written, `#CLI-1`, resolves to the same key as its slug,
    // `#cli-1`.
    let written = f.run(&["graph", "#CLI-1"]);
    assert_eq!(written, warm);
    // When the start is a key, the graph follows the edges of each document it
    // reaches past the first hop. A run with the cache prints the same edges
    // as a run without the cache (CACHE-1_AC-38).
    for depth in ["2", "3"] {
        let warm = f.run(&["graph", "#cli-1", "--depth", depth]);
        let cold = f.run(&["--no-cache", "graph", "#cli-1", "--depth", depth]);
        assert_eq!(warm, cold);
        assert!(warm.0.lines().count() > 4, "{}", warm.0);
    }
}

// sokf-tests: CLI-7_AC-79
#[test]
fn the_text_the_json_and_the_mermaid_show_one_view() {
    let f = Fixture::new("graph");
    for start in [
        "planner",
        "config",
        "crates/lib/example-core/src/planner.rs",
    ] {
        let (text, _, _) = f.run_here(&["graph", start]);
        let (json, _, _) = f.run_here(&["graph", start, "--json"]);
        let (mermaid, _, _) = f.run_here(&["graph", start, "--mermaid"]);
        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let lines = text.lines().filter(|l| l.contains(" → ")).count();
        let arrows = mermaid
            .lines()
            .filter(|l| l.contains("-->") || l.contains("-.->"))
            .count();
        assert_eq!(arrows, lines, "{start}: {mermaid}");
        let nodes = json["nodes"].as_array().unwrap().len();
        let declared = mermaid
            .lines()
            .filter(|l| l.trim_start().starts_with('n') && !l.contains("->"))
            .count();
        assert_eq!(declared, nodes, "{start}: {mermaid}");
    }
}

/// `scoped_keys` returns the `clean` fixture with three notes whose keys
/// have document scope: `n1` captures `XX-A1`, and `n2` and `n3` each
/// capture `YY-B1`. `n2` and a marker cite `n1#xx-a1`.
fn scoped_keys() -> Fixture {
    let f = Fixture::new("clean");
    f.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\ndescription: Notes with terms of their own.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        each:\n          pattern: '^(?P<t>[A-Z]{2}-[A-Z]\\d+): .+$'\n          key: t\n          example: 'XX-A1: a term'\n      - kind: prose\n        example: The terms come first.\n````\n",
    );
    for (id, key, prose) in [
        ("n1", "XX-A1", "The terms come first."),
        (
            "n2",
            "YY-B1",
            "The term [XX-A1][sokf:n1#xx-a1] comes first.",
        ),
        ("n3", "YY-B1", "The terms come first."),
    ] {
        f.write(
            &format!("knowledge/{id}.md"),
            &format!("---\ntype: Note\nid: {id}\ntitle: {id}\n---\n\n# {id}\n\n- {key}: a term\n\n{prose}\n"),
        );
    }
    // sokf:ignore-begin
    f.write(
        "src/term.rs",
        "// sokf-references: n1#xx-a1\nfn term() {}\n",
    );
    // sokf:ignore-end
    let (stdout, _, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 0, "{stdout}");
    f
}

// sokf-tests: CLI-7_AC-97
// sokf-tests: CLI-7_AC-98
// sokf-tests: CLI-7_AC-99
// sokf-tests: CLI-7_AC-100
#[test]
fn a_key_of_document_scope_starts_when_one_document_captures_it() {
    let f = scoped_keys();
    let expected = "XX-A1\n├── defined-in → n1 (line 9)\n├── cited-by → n2\n└── referenced-by → /src/term.rs (line 1)\n\n2 / 2 docs\n";
    // One document captures `XX-A1`, so `#xx-a1` starts at the key, with a
    // note that gives the reference to link.
    for cache in [&[][..], &["--no-cache"][..]] {
        let mut args = cache.to_vec();
        args.extend(["graph", "#xx-a1"]);
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(stdout, expected, "{args:?}");
        assert_eq!(
            stderr,
            "note: graph: `#xx-a1` is unique only within `n1`, so link it as `n1#xx-a1`\n"
        );
    }
    // `n1#xx-a1` starts at the same key, with no note.
    assert_eq!(
        f.run(&["graph", "n1#xx-a1"]),
        (expected.to_string(), String::new(), 0)
    );
    // Two documents capture `YY-B1`, so the error lists the reference of each.
    let (stdout, stderr, code) = f.run(&["graph", "#yy-b1"]);
    assert_eq!((stdout.as_str(), code), ("", 2));
    assert_eq!(
        stderr,
        "error: graph: `#yy-b1` is a key of document scope that more than one document captures. Write `n2#yy-b1` or `n3#yy-b1`\n"
    );
    // A key that no document captures keeps the error for a word that
    // matches nothing.
    let (_, stderr, code) = f.run(&["graph", "#zz-c1"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: graph: `#zz-c1` is neither an id nor a path in the repository\n"
    );
}

// sokf-tests: CLI-7_AC-99
#[test]
fn the_error_for_a_key_of_document_scope_lists_ten_references() {
    let f = scoped_keys();
    // Twelve more notes capture `ZZ-D1`, so the error lists the first ten,
    // in the order of their paths, and counts the other two.
    for i in 1..=12 {
        let id = format!("m{i:02}");
        f.write(
            &format!("knowledge/{id}.md"),
            &format!("---\ntype: Note\nid: {id}\ntitle: {id}\n---\n\n# {id}\n\n- ZZ-D1: a term\n\nThe terms come first.\n"),
        );
    }
    let (stdout, stderr, code) = f.run(&["graph", "#zz-d1"]);
    assert_eq!((stdout.as_str(), code), ("", 2));
    let listed = (1..=10)
        .map(|i| format!("`m{i:02}#zz-d1`"))
        .collect::<Vec<_>>()
        .join(" or ");
    assert_eq!(
        stderr,
        format!(
            "error: graph: `#zz-d1` is a key of document scope that more than one document captures. Write {listed}, and 2 more\n"
        )
    );
}
