//! These tests validate the JSON outputs of the real binary against the published
//! schemas in `/knowledge/json/`. `schemars` generates the schemas from the wire types, so the
//! tests catch a gap between `schemars` and serde: an output that the schema of its own type
//! refuses. Between them, the fixtures produce every kind of fix, node, and edge, and
//! the tests check that they do.

use std::{collections::BTreeSet, fs, path::Path};

use common::{Fixture, run, sokf};
use serde_json::Value;

use crate::common;

/// `check` returns `Ok` when the schema `name` accepts `value`, and otherwise the reason
/// that the schema refuses it.
pub fn check(name: &str, value: &Value) -> Result<(), String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../knowledge/json")
        .join(name);
    let schema: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let url = format!("file:///knowledge/json/{name}");
    let mut compiler = boon::Compiler::new();
    compiler.add_resource(&url, schema).unwrap();
    let mut schemas = boon::Schemas::new();
    let index = compiler.compile(&url, &mut schemas).unwrap();
    schemas.validate(value, index).map_err(|e| format!("{e:#}"))
}

/// `valid` parses `stdout` as JSON, panics unless the schema `name` accepts it, and
/// returns the value.
pub fn valid(name: &str, stdout: &str) -> Value {
    let value: Value = serde_json::from_str(stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    if let Err(e) = check(name, &value) {
        panic!("{name}: {e}\n{stdout}");
    }
    value
}

// Each schema refuses a wrong shape, so when the other tests in this file pass, the outputs
// have the right shape.
#[test]
fn the_schemas_refuse_a_wrong_shape() {
    for (name, wrong) in [
        (
            "validate-report.schema.json",
            r#"{"errors":[{"path":1,"message":"m"}],"warnings":[],"fixes":[]}"#,
        ),
        (
            "validate-report.schema.json",
            r#"{"errors":[],"warnings":[],"fixes":[{"op":"deleted","path":"/a"}]}"#,
        ),
        (
            "graph.schema.json",
            r#"{"start":[],"nodes":[{"kind":"document","path":"/a.md"}],"edges":[]}"#,
        ),
        (
            "graph.schema.json",
            r#"{"start":[],"nodes":[],"edges":[{"kind":"links","from":"/a","to":"/b","rel":"r","synthesised":false}]}"#,
        ),
        (
            "harness-hook-answer.schema.json",
            r#"{"decision":"allow","reason":"x"}"#,
        ),
        ("locate-report.schema.json", r#"[{"id":"a"}]"#),
    ] {
        let value: Value = serde_json::from_str(wrong).unwrap();
        assert!(check(name, &value).is_err(), "{name} accepts {wrong}");
    }
}

fn kinds<'a>(values: impl IntoIterator<Item = &'a Value>, key: &str) -> BTreeSet<String> {
    values
        .into_iter()
        .filter_map(|v| v[key].as_str().map(str::to_string))
        .collect()
}

// sokf-tests: CLI-4_AC-12
#[test]
fn validate_reports_match_their_schema() {
    let mut ops = BTreeSet::new();
    for (fixture, fix) in [
        ("clean", false),
        ("broken", false),
        ("fixable", true),
        ("assets", true),
    ] {
        let f = Fixture::new(fixture);
        let mut args = vec!["--repo-root", f.root(), "validate", "--json"];
        if fix {
            args.push("--fix");
        }
        let (stdout, _, _) = run(&args);
        let report = valid("validate-report.schema.json", &stdout);
        ops.extend(kinds(report["fixes"].as_array().unwrap(), "op"));
    }
    assert_eq!(
        ops,
        ["created", "moved", "rewrote"].map(String::from).into(),
        "the fixtures don't produce every kind of fix"
    );
}

// sokf-tests: CLI-7_AC-12
#[test]
fn graphs_match_their_schema() {
    let mut nodes = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut broken = false;
    // The test runs each fixture from the start points that, together, reach
    // every kind of node and edge.
    for (fixture, starts) in [
        ("graph", &["/", "planner"][..]),
        ("clean", &["/", "sourced", "#cli-1_ac-2"]),
        ("broken", &["/"]),
        ("assets", &["/", "rates"]),
    ] {
        let f = Fixture::new(fixture);
        // No fixture cites a repository file as a source, so the test writes a document that does.
        f.write(
            "knowledge/sourced.md",
            "---\ntype: Note\nid: sourced\nsources:\n  - resource: /src/main.rs\n---\n\n# Sourced\n",
        );
        for start in starts {
            let (stdout, stderr, code) = f.run(&["graph", "--json", start]);
            assert_eq!(code, 0, "{stderr}");
            let graph = valid("graph.schema.json", &stdout);
            nodes.extend(kinds(graph["nodes"].as_array().unwrap(), "kind"));
            edges.extend(kinds(graph["edges"].as_array().unwrap(), "kind"));
            broken |= graph["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["broken"] == true);
        }
    }
    let all_nodes = [
        "additional",
        "asset",
        "directory",
        "document",
        "file",
        "key",
        "missing",
        "root",
    ];
    assert_eq!(nodes, all_nodes.map(String::from).into());
    let all_edges = ["includes", "keys", "links", "resource", "sources"];
    assert_eq!(edges, all_edges.map(String::from).into());
    assert!(broken, "no fixture reaches a broken document");
}

// sokf-tests: CLI-6_AC-12
// sokf-tests: CLI-6_AC-72
// sokf-tests: CLI-2_AC-12
// sokf-tests: CLI-8_AC-5
#[test]
fn init_new_and_locate_match_their_schemas() {
    let f = Fixture::new("manifest");
    fs::remove_dir_all(f.0.join("knowledge")).unwrap();
    let (stdout, stderr, code) = f.run(&["init", "/docs/kb", "--scope", "test", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    valid("init-report.schema.json", &stdout);

    let f = Fixture::new("new");
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "x", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    valid("new-report.schema.json", &stdout);
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "x", "--write", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    valid("new-report.schema.json", &stdout);

    let f = Fixture::new("assets");
    let (stdout, stderr, code) = f.run(&["locate", "--json", "rates", "album"]);
    assert_eq!(code, 0, "{stderr}");
    let located = valid("locate-report.schema.json", &stdout);
    assert!(located[0]["asset"].is_string() && located[1].get("asset").is_none());
}

// sokf-tests: CLI-3_AC-20
// sokf-tests: CLI-3_AC-82
#[test]
fn harness_reports_and_hook_answers_match_their_schemas() {
    let f = Fixture::new("harness");
    for sub in ["install", "status"] {
        let (stdout, stderr, code) = f.run(&["harness", sub, "claude", "--json"]);
        assert_eq!(code, 0, "{stderr}");
        valid("harness-report.schema.json", &stdout);
    }
    for (fixture, noted) in [("clean", false), ("broken", true)] {
        let f = Fixture::new(fixture);
        let out = sokf()
            .args([
                "--repo-root",
                f.root(),
                "harness",
                "hook",
                "claude",
                "prompt-submit",
            ])
            .write_stdin("{}")
            .output()
            .unwrap();
        let answer = valid(
            "harness-hook-answer.schema.json",
            &String::from_utf8(out.stdout).unwrap(),
        );
        assert_eq!(answer.get("hookSpecificOutput").is_some(), noted);
    }
}

/// Each command that takes `--json`, with the output whose schema `--json-schema` prints.
const SCHEMA_COMMANDS: [(&[&str], sokf_core::Output); 8] = [
    (&["init"], sokf_core::Output::Init),
    (&["harness", "install"], sokf_core::Output::Harness),
    (&["harness", "status"], sokf_core::Output::Harness),
    (&["validate"], sokf_core::Output::Validate),
    (&["graph"], sokf_core::Output::Graph),
    (&["locate"], sokf_core::Output::Locate),
    (&["new"], sokf_core::Output::New),
    (&["info", "formats"], sokf_core::Output::InfoFormats),
];

// Each command prints the schema of its `--json` output and nothing else, outside any
// repository, with no required argument, and exits with code 0.
// sokf-tests: CLI-1_AC-14
// sokf-tests: CLI-1_AC-15
#[test]
fn json_schema_prints_the_schema_of_the_command_outside_a_repository() {
    let outside = std::env::temp_dir();
    for (command, output) in SCHEMA_COMMANDS {
        let args: Vec<&str> = command.iter().copied().chain(["--json-schema"]).collect();
        let (stdout, stderr, code) = common::run_in(&outside, &args);
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), code),
            (output.schema_text().as_str(), "", 0),
            "{args:?}"
        );
    }
}

// Any other argument beside `--json-schema` is a usage error: an argument of the command, and
// a global option before or after the command.
// sokf-tests: CLI-1_AC-16
#[test]
fn json_schema_refuses_every_other_argument() {
    for args in [
        &["graph", "x", "--json-schema"][..],
        &["locate", "--json-schema", "glossary"],
        &["validate", "--fix", "--json-schema"],
        &["harness", "install", "claude", "--json-schema"],
        &["graph", "--json-schema", "--no-cache"],
        &["--no-cache", "graph", "--json-schema"],
        &["--repo-root", "/nowhere", "new", "--json-schema"],
        &["info", "formats", "--layer", "embedded", "--json-schema"],
    ] {
        let (stdout, stderr, code) = run(args);
        assert_eq!((stdout.as_str(), code), ("", 2), "{args:?}: {stderr}");
        assert!(
            stderr.contains("'--json-schema' cannot be used"),
            "{args:?}: {stderr}"
        );
    }
}

// The help of each `--json` flag names `--json-schema`.
// sokf-tests: CLI-1_AC-17
#[test]
fn the_help_of_json_names_json_schema() {
    for (command, _) in SCHEMA_COMMANDS {
        let args: Vec<&str> = command.iter().copied().chain(["--help"]).collect();
        let (stdout, _, code) = run(&args);
        assert_eq!(code, 0, "{args:?}");
        // clap puts the help beside the flag, or below it in the long form, and the
        // help ends where the next option begins.
        let at = ["\n      --json ", "\n      --json\n"]
            .iter()
            .find_map(|flag| stdout.find(flag))
            .unwrap_or_else(|| panic!("{args:?} has no --json: {stdout}"));
        let rest = &stdout[at + 1..];
        let end = ["\n  -", "\n      -"]
            .iter()
            .filter_map(|next| rest.find(next))
            .min()
            .unwrap_or(rest.len());
        let json = &rest[..end];
        assert!(json.contains("`--json-schema`"), "{args:?}: {json}");
    }
}
