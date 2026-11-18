//! End-to-end tests of `sokf info`: it prints the grammar verbatim, shows
//! each layer of the format registry, and shows the registry as one schema
//! sees it.

use std::path::Path;

use common::{Fixture, run, run_in};

use crate::common;

/// `run_outside` runs `sokf` with `args` from a directory outside every
/// repository, and returns its standard output, standard error, and exit code.
fn run_outside(args: &[&str]) -> (String, String, i32) {
    run_in(&std::env::temp_dir(), args)
}

// sokf-tests: CLI-5_AC-1
// sokf-tests: CLI-5_AC-2
#[test]
fn info_grammar_prints_the_three_files_verbatim_and_reads_no_repository() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let extension = std::fs::read_to_string(spec.join("sokf-spec/sokf.tramlines.yaml")).unwrap();
    let expected = sokf_core::grammar_text();
    let (stdout, stderr, code) = run_outside(&["info", "grammar"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, expected);
    // The output holds three files, each separated by `---`: the tramlines core
    // grammar, the tramlines-md grammar, and SOKF's extension, which is the file
    // in `sokf-spec/` verbatim.
    let parts: Vec<&str> = stdout.split("\n---\n").collect();
    assert_eq!(parts.len(), 3, "{stdout}");
    assert!(parts[0].starts_with("grammar: tramlines\n"), "{stdout}");
    assert!(parts[1].starts_with("grammar: tramlines-md\n"), "{stdout}");
    assert_eq!(parts[2], extension);
    // `info grammar` reads no repository, so a root that does not exist
    // changes nothing.
    let (stdout, stderr, code) =
        run_outside(&["--repo-root", "/definitely/not/a/dir", "info", "grammar"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, expected);
}

// sokf-tests: CLI-5_AC-5
// sokf-tests: CLI-5_AC-6
#[test]
fn the_embedded_layer_reads_no_repository() {
    let (inside, _, code) = run(&["info", "formats", "--layer", "embedded"]);
    assert_eq!(code, 0);
    let (stdout, stderr, code) = run_outside(&[
        "--repo-root",
        "/definitely/not/a/dir",
        "info",
        "formats",
        "--layer",
        "embedded",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, inside);
    assert!(!stdout.contains("source: repo"), "{stdout}");
    // The effective layer reads the repository: with no root, it fails.
    let (_, _, code) = run_outside(&["info", "formats"]);
    assert_eq!(code, 2);
}

// sokf-tests: CLI-5_AC-3
// sokf-tests: CLI-5_AC-8
#[test]
fn a_schema_view_merges_the_formats_of_the_schema_on_top() {
    let f = Fixture::new("clean");
    // The repository layer declares `requirement-ears`, which the
    // requirement schema declares again in its own formats.
    f.write(
        ".sokf/formats.yaml",
        "types:\n  criterion-id: '[A-Z]{1,10}-\\d+_AC-\\d+'\nregistry:\n  requirement-ears:\n    key: id\n    template: '${id:criterion-id} ${}'\n    example: CLI-1_AC-1 x\n",
    );
    f.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nformats:\n  word:\n    pattern: '^w$'\n    example: w\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: Note\n    level: 1\n    blocks:\n      - kind: prose\n        example: Milk, bread, and the good coffee.\n````\n",
    );
    let (stdout, stderr, code) = f.run(&["info", "formats", "--schema", "schema-note"]);
    assert_eq!(code, 0, "{stderr}");
    // The output holds one registry, which merges the tool's types, the
    // repository's layer, and then the schema's layer.
    assert_eq!(stdout.matches("\nregistry:\n").count(), 1, "{stdout}");
    assert!(stdout.contains("source: built-in"), "{stdout}");
    assert!(stdout.contains("source: repo"), "{stdout}");
    assert!(stdout.contains("source: schema-note"), "{stdout}");
    assert!(stdout.contains("\n  word:\n"), "{stdout}");
    // The view of another schema doesn't include the formats of `schema-note`.
    let (stdout, _, code) = f.run(&["info", "formats", "--schema", "schema-requirement"]);
    assert_eq!(code, 0);
    assert!(!stdout.contains("source: schema-note"), "{stdout}");
}

/// `names_and_sources` returns each name of the map `section` of the YAML
/// registry `yaml`, with its `source`, in the order of the text.
fn names_and_sources(yaml: &str, section: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut inside = false;
    for line in yaml.lines() {
        if !line.starts_with(' ') {
            inside = line == format!("{section}:");
        } else if inside {
            if let Some(name) = line.strip_prefix("  ").and_then(|l| l.strip_suffix(':'))
                && !name.starts_with(' ')
            {
                out.push((name.trim_matches('"').to_string(), String::new()));
            } else if let Some(source) = line.strip_prefix("    source: ") {
                out.last_mut().unwrap().1 = source.to_string();
            }
        }
    }
    out
}

/// `json_names_and_sources` returns each name of the map `section` of the
/// JSON registry `json`, with its `source`.
fn json_names_and_sources(json: &serde_json::Value, section: &str) -> Vec<(String, String)> {
    json[section]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, e)| (name.clone(), e["source"].as_str().unwrap().to_string()))
        .collect()
}

// sokf-tests: CLI-5_AC-38
// sokf-tests: CLI-5_AC-40
#[test]
fn info_formats_prints_the_registry_of_its_yaml_as_one_json_object() {
    let f = Fixture::new("clean");
    f.write(
        ".sokf/formats.yaml",
        "types:\n  colour: [red, green]\nregistry:\n  shade:\n    template: '${c:colour} shade'\n    example: red shade\n",
    );
    for args in [
        &["info", "formats", "--layer", "embedded"][..],
        &["info", "formats"],
        &["info", "formats", "--schema", "schema-requirement"],
    ] {
        let (yaml, stderr, code) = f.run(args);
        assert_eq!(code, 0, "{stderr}");
        let mut with_json = args.to_vec();
        with_json.push("--json");
        let (out, stderr, code) = f.run(&with_json);
        assert_eq!(code, 0, "{stderr}");
        let json = crate::json_schemas::valid("info-formats-registry.schema.json", &out);
        for section in ["types", "registry"] {
            let names = names_and_sources(&yaml, section);
            // The embedded layer holds the tool's types and no format.
            assert!(!names.is_empty() || section == "registry", "{yaml}");
            assert_eq!(json_names_and_sources(&json, section), names, "{args:?}");
        }
    }
    let (out, stderr, code) = f.run(&["info", "formats", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        json["types"]["colour"],
        serde_json::json!({"match": ["red", "green"], "source": "repo"})
    );
    assert_eq!(
        json["registry"]["shade"],
        serde_json::json!({"template": "${c:colour} shade", "example": "red shade", "source": "repo"})
    );
    let (out, stderr, code) = f.run(&["info", "formats", "--json", "--query", "types.slug.source"]);
    assert_eq!((out.as_str(), code), ("\"built-in\"\n", 0), "{stderr}");
}

// sokf-tests: CLI-5_AC-39
#[test]
fn a_registry_without_types_prints_an_empty_map_of_types() {
    let f = Fixture::new("clean");
    f.write(
        ".sokf/formats.yaml",
        "registry:\n  shade:\n    text: Red.\n    example: Red.\n",
    );
    let (yaml, _, code) = f.run(&["info", "formats", "--layer", "repo"]);
    assert_eq!(code, 0);
    assert!(!yaml.contains("types:"), "{yaml}");
    let (out, stderr, code) = f.run(&["info", "formats", "--layer", "repo", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let json = crate::json_schemas::valid("info-formats-registry.schema.json", &out);
    assert_eq!(json["types"], serde_json::json!({}));
    assert_eq!(
        json["registry"],
        serde_json::json!({"shade": {"text": "Red.", "example": "Red.", "source": "repo"}})
    );
}

// sokf-tests: CLI-5_AC-41
#[test]
fn query_without_json_is_a_usage_error() {
    let (out, err, code) = run_outside(&["info", "formats", "--query", "types"]);
    assert_eq!((out.as_str(), code), ("", 2), "{err}");
}
