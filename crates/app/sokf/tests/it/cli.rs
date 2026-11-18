//! These end-to-end tests run the real `sokf` binary on fixtures and check
//! its text output, JSON output, standard error, and exit code. The release
//! smoke script mirrors them, so the packaged binary must meet the same checks.

use common::{Fixture, run, run_in, sokf};

use crate::common;

#[test]
fn version_prints_name_and_semver() {
    for flag in ["-V", "--version"] {
        let (stdout, _, code) = run(&[flag]);
        assert_eq!(code, 0);
        assert_eq!(
            stdout.trim(),
            format!("sokf {}", env!("CARGO_PKG_VERSION")),
            "`{flag}` output"
        );
    }
}

#[test]
fn bare_invocation_prints_help_and_exits_clean() {
    let (stdout, _, code) = run(&[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage: sokf"), "{stdout}");
    assert!(
        stdout.contains("Exit codes: 0 if there are no errors, 1 if there are errors, 2 on a usage or internal error"),
        "{stdout}"
    );
}

#[test]
fn help_lists_the_visible_commands_and_hides_man() {
    let (stdout, _, code) = run(&["--help"]);
    assert_eq!(code, 0);
    for verb in [
        "init",
        "harness",
        "validate",
        "info",
        "new",
        "graph",
        "locate",
        "guide",
        "completions",
    ] {
        assert!(stdout.contains(verb), "{verb} missing from {stdout}");
    }
    assert!(
        !stdout.contains("\n  man"),
        "man is hidden from help: {stdout}"
    );
}

// sokf-tests: CLI-1_AC-1
// sokf-tests: CLI-1_AC-9
#[test]
fn a_clean_store_validates_with_exit_0() {
    let f = Fixture::new("clean");
    let (stdout, stderr, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(stdout, "0 errors, 0 warnings\n");
    assert_eq!(stderr, "");
    // `--repo-root` after the command works too, and `--fix` changes nothing.
    let (stdout, _, code) = run(&["validate", "--fix", "--repo-root", f.root()]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "0 errors, 0 warnings\n");
}

// sokf-tests: CLI-1_AC-6
// sokf-tests: CLI-1_AC-10
// sokf-tests: CLI-4_AC-7
// sokf-tests: CLI-4_AC-8
#[test]
fn errors_exit_1_and_render_per_the_tool_spec() {
    let f = Fixture::new("clean");
    f.write(
        "knowledge/broken.md",
        "---\ntype: Decision\nid: broken\ntitle: Broken\n---\n\n# Broken\n\n## Context\n\nSee [nothing][sokf:planer].\n\n## Decision\n\ny\n\n## Consequences\n\n- z\n",
    );
    let (stdout, stderr, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert!(
        stdout.contains(
            "/knowledge/broken.md:11: error: `[sokf:planer]` resolves to no document (SOKF_FORMAT_61)"
        ),
        "{stdout}"
    );
    // The new document is missing from its directory's index, which is a
    // second error, reported at the index's path with no line number
    // (CLI-4_AC-7, CLI-4_AC-27).
    assert!(
        stdout.contains(
            "/knowledge/index.md: error: the index is stale: it differs from the generated content (SOKF_FORMAT_97)"
        ),
        "{stdout}"
    );
    // The run reports these two errors and no others.
    assert!(stdout.ends_with("2 errors, 0 warnings\n"), "{stdout}");
    assert_eq!(stderr, "");
}

// sokf-tests: CLI-1_AC-6
// sokf-tests: CLI-4_AC-12
// sokf-tests: CLI-4_AC-31
// sokf-tests: CLI-4_AC-14
// sokf-tests: CLI-4_AC-16
// sokf-tests: CLI-4_AC-17
#[test]
fn json_is_one_object_with_the_spec_shape() {
    let f = Fixture::new("clean");
    f.write("knowledge/req-cli.md", "---\ntype: Requirement\nid: req-cli\ntitle: CLI requirement\n---\n\n# CLI requirement\n\n## User Stories\n\nCLI-1: AS A developer, I WANT it, SO THAT I win.\n\n> Rationale.\n\n## Acceptance Criteria\n\n- CLI-1_AC-1 [ubiquitous]: the system SHALL work.\n");
    let (stdout, _, code) = f.run(&["validate", "--json", "--fix"]);
    assert_eq!(code, 0, "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["errors"], serde_json::json!([]));
    assert_eq!(doc["warnings"], serde_json::json!([]));
    assert_eq!(
        doc["fixes"],
        serde_json::json!([
            {"op": "rewrote", "path": "/knowledge/index.md"},
            {"op": "rewrote", "path": "/knowledge/overview.md"},
            {"op": "rewrote", "path": "/knowledge/req-cli.md"}
        ])
    );
    assert!(stdout.ends_with("}\n"));
    // `--query` selects part of the JSON object (CLI-4_AC-14); without `--json`,
    // `--query` is a usage error (CLI-4_AC-16).
    let (stdout, _, code) = f.run(&["validate", "--json", "--query", "keys(@)"]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(stdout, "[\"errors\",\"warnings\",\"fixes\",\"summary\"]\n");
    let (stdout, _, _) = f.run(&["validate", "--json", "--query", "nope"]);
    assert_eq!(stdout, "null\n");
    let (_, stderr, code) = f.run(&["validate", "--json", "--query", "fixes[?"]);
    assert_eq!(code, 2);
    assert!(stderr.starts_with("error: query: "), "{stderr}");
    let (_, _, code) = f.run(&["validate", "--query", "fixes"]);
    assert_eq!(code, 2);
}

// sokf-tests: CLI-1_AC-3
// sokf-tests: CLI-1_AC-7
// sokf-tests: CLI-1_AC-8
// sokf-tests: CLI-4_AC-2
// sokf-tests: CLI-4_AC-3
// sokf-tests: CLI-4_AC-15
#[test]
fn paths_narrow_the_report_and_exit_code() {
    let f = Fixture::new("clean");
    f.write(
        "knowledge/broken.md",
        "---\ntype: Nope\nid: broken\n---\n\n# Broken\n",
    );
    // `PATHS` resolve from the current directory (CLI-1_AC-8).
    let (stdout, _, code) = f.run_here(&["validate", "README.md"]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(stdout, "0 errors, 0 warnings\n");
    let (stdout, _, code) = run_in(
        &f.0.join("knowledge"),
        &["--repo-root", f.root(), "validate", "."],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("/knowledge/broken.md"), "{stdout}");
    // A path that starts with `/` resolves from the repository root, whatever
    // the current directory is (CLI-1_AC-7).
    let (stdout, _, code) = run_in(
        &f.0.join("knowledge"),
        &["--repo-root", f.root(), "validate", "/README.md"],
    );
    assert_eq!(code, 0, "{stdout}");
    let (_, stderr, code) = f.run_here(&["validate", "missing.md"]);
    assert_eq!(code, 2);
    assert!(
        stderr.starts_with("error: missing.md: no such path"),
        "{stderr}"
    );
}

// sokf-tests: CLI-4_AC-4
// sokf-tests: CLI-4_AC-6
#[test]
fn fix_moves_a_lifecycle_document_and_reports_it() {
    let f = Fixture::new("clean");
    f.write(
        "knowledge/issues/open/issue-002-done.md",
        "---\ntype: Issue\nid: issue-002-done\ntitle: Done\nstate: closed\nkind: feature\n---\n\n# Done\n\nShipped.\n\n## Conclusion\n\nDone.\n",
    );
    let (stdout, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("/knowledge/issues/open/issue-002-done.md: error: the document sits in `/knowledge/issues/open`, but its lifecycle states place it in `/knowledge/issues/closed` (SOKF_SCHEMA_26)"), "{stdout}");
    let (stdout, _, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 0, "{stdout}");
    // `validate --fix` rewrites the index of the directory the document
    // enters. The index of the directory it leaves already matches, because
    // that index never listed the document.
    assert_eq!(
        stdout,
        "moved   /knowledge/issues/open/issue-002-done.md -> /knowledge/issues/closed/issue-002-done.md\nrewrote /knowledge/issues/closed/index.md\n\n0 errors, 0 warnings\n"
    );
    assert!(
        f.0.join("knowledge/issues/closed/issue-002-done.md")
            .exists()
    );
}

// sokf-tests: CLI-4_AC-11
// sokf-tests: CLI-4_AC-30
#[test]
fn warnings_are_counted_by_default_and_listed_on_request() {
    let f = Fixture::new("clean");
    f.write(
        "knowledge/issues/open/issue-001-retry.md",
        "---\ntype: Issue\nid: issue-001-retry\ntitle: Retry fails twice\nstate: open\nkind: bug\nlinks:\n  - rel: implements\n    to: decision-single-binary\n  - rel: likes\n    to: decision-single-binary\n---\n\n# Retry fails twice\n\nThe retry of [the one binary][sokf:decision-single-binary] runs twice.\n\n## Reproduction\n\n1. Run the daemon.\n\n<!-- sokf:links -->\n\n[sokf:decision-single-binary]: /knowledge/decisions/decision-single-binary.md\n",
    );
    let (stdout, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(
        stdout.contains("the schema declares no `links` rule for `rel: likes`"),
        "{stdout}"
    );
    assert!(
        stdout.ends_with("\n\n1 error, 0 of 1 warnings listed\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("warning:"), "{stdout}");
    let (stdout, _, _) = f.run(&["validate", "--warnings"]);
    assert!(
        stdout.ends_with("1 error, 1 of 1 warnings listed\n"),
        "{stdout}"
    );
    assert!(stdout.contains("/knowledge/issues/open/issue-001-retry.md:10: warning: `rel: likes` is outside the core set"), "{stdout}");
}

// sokf-tests: CLI-5_AC-2
#[test]
fn info_grammar_prints_the_embedded_grammar_without_a_repository() {
    let (stdout, (), code) = sokf()
        .args(["info", "grammar"])
        .current_dir(std::env::temp_dir())
        .output()
        .map(|o| {
            (
                String::from_utf8(o.stdout).unwrap(),
                (),
                o.status.code().unwrap(),
            )
        })
        .unwrap();
    assert_eq!(code, 0);
    assert!(stdout.starts_with("grammar: tramlines\n"), "{stdout}");
    assert!(
        stdout.contains("\n---\ngrammar: tramlines-md\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\n---\nextension: sokf\n"), "{stdout}");
}

// sokf-tests: CLI-5_AC-3
// sokf-tests: CLI-5_AC-4
// sokf-tests: CLI-5_AC-5
// sokf-tests: CLI-5_AC-7
// sokf-tests: CLI-5_AC-10
// sokf-tests: CLI-5_AC-11
#[test]
fn info_formats_prints_each_layer_and_a_schema_view() {
    let f = Fixture::new("clean");
    let (stdout, _, code) = run(&["info", "formats", "--layer", "embedded"]);
    assert_eq!(code, 0);
    // The types come first, the built-ins among them, then the formats, of
    // which the tool embeds none.
    assert!(stdout.starts_with("types:\n"), "{stdout}");
    assert!(stdout.contains("source: built-in"), "{stdout}");
    assert!(stdout.ends_with("\nregistry: {}\n"), "{stdout}");
    let (_, stderr, code) = f.run(&["info", "formats", "--layer", "repo"]);
    assert_eq!(code, 2);
    assert!(
        stderr.starts_with("error: info: the repository has no /.sokf/formats.yaml"),
        "{stderr}"
    );
    f.write(
        ".sokf/formats.yaml",
        "registry:\n  shade:\n    text: Red.\n    example: Red.\n",
    );
    let (stdout, _, code) = f.run(&["info", "formats", "--layer", "repo"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("source: repo"), "{stdout}");
    assert!(!stdout.contains("user-story"), "{stdout}");
    // The repository layer declares no types, so the command prints none.
    assert!(stdout.starts_with("registry:\n"), "{stdout}");
    let (stdout, _, code) = f.run(&["info", "formats"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("source: repo") && stdout.contains("shade"),
        "{stdout}"
    );
    // The view of a schema adds the schema's own formats.
    let (stdout, _, code) = f.run(&["info", "formats", "--schema", "schema-requirement"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("source: repo"), "{stdout}");
    assert!(stdout.contains("source: schema-requirement"), "{stdout}");
    let (_, stderr, code) = f.run(&["info", "formats", "--schema", "nope"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: info: schema `nope` is not in any root. The roots are /knowledge\n"
    );
}

// sokf-tests: CLI-6_AC-13
// sokf-tests: CLI-6_AC-17
// sokf-tests: CLI-6_AC-18
// sokf-tests: CLI-6_AC-48
// sokf-tests: CLI-6_AC-49
// sokf-tests: CLI-6_AC-50
#[test]
fn the_id_alone_per_the_tool_spec() {
    let f = Fixture::new("clean");
    let (stdout, _, code) = f.run(&["new", "Issue", "--slug", "retry-logic", "--id-only"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "issue-002-retry-logic\n");
    for (args, needle) in [
        (vec!["new", "Nope", "--id-only"], "has no schema"),
        (vec!["new", "Decision", "--id-only"], "no id template"),
        (vec!["new", "Issue", "--id-only"], "`--slug` is required"),
        (
            vec!["new", "Issue", "--slug", "Not Slug", "--id-only"],
            "is not a valid slug",
        ),
    ] {
        let (_, stderr, code) = f.run(&args);
        assert_eq!(code, 2, "{stderr}");
        assert!(stderr.starts_with("error: new: "), "{stderr}");
        assert!(stderr.contains(needle), "{stderr}");
    }
    f.write("knowledge/issues/open/issue-broken.md", "---\ntype: Issue\nid: broken\ntitle: B\nstate: open\nkind: bug\n---\n\n# B\n\nx\n\n## Reproduction\n\n1. y\n");
    let (_, stderr, code) = f.run(&["new", "Issue", "--slug", "x", "--id-only"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("does not match the id template"),
        "{stderr}"
    );
}

// sokf-tests: CLI-1_AC-5
// sokf-tests: CLI-1_AC-11
// sokf-tests: CLI-1_AC-12
// sokf-tests: CLI-7_AC-23
// sokf-tests: CLI-7_AC-24
// sokf-tests: CLI-5_AC-9
#[test]
fn no_repository_and_usage_errors_exit_2() {
    let out = sokf()
        .args(["validate"])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(stderr, "error: not in a Git repository; pass --repo-root\n");
    for args in [
        vec!["--definitely-not-a-flag"],
        vec!["frobnicate"],
        vec!["completions"],
        vec!["completions", "cmd.exe"],
        vec!["info", "formats", "--layer", "repo", "--schema", "x"],
        vec!["--repo-root", "/definitely/not/a/dir", "validate"],
        vec!["graph", "--json", "--mermaid"],
        vec!["graph", "--format", "{id}", "--json"],
        vec!["graph", "--format", "{id}", "--mermaid"],
        vec!["graph", "--query", "nodes"],
        vec!["graph", "--query", "nodes", "--mermaid"],
        vec!["graph", "--query", "nodes", "--format", "{id}"],
        vec!["validate", "--query", "errors"],
        vec!["locate"],
    ] {
        let (_, stderr, code) = run(&args);
        assert_eq!(code, 2, "`sokf {}` -> {stderr}", args.join(" "));
    }
}

// sokf-tests: CONF-1_AC-4
#[test]
fn a_broken_config_is_a_usage_error() {
    let f = Fixture::new("clean");
    f.write(".sokf/config.toml", "[knowledge]\nrootz = []\n");
    let (stdout, stderr, code) = f.run(&["validate"]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert!(
        stderr.starts_with("error: /.sokf/config.toml: "),
        "{stderr}"
    );
}

#[test]
fn completions_render_for_every_supported_shell() {
    for (shell, marker) in [
        ("bash", "_sokf()"),
        ("zsh", "#compdef sokf"),
        ("fish", "complete -c sokf"),
        ("powershell", "Register-ArgumentCompleter"),
        ("elvish", "set edit:completion:arg-completer[sokf]"),
    ] {
        let (stdout, _, code) = run(&["completions", shell]);
        assert_eq!(code, 0, "{shell} exited {code}");
        assert!(stdout.contains(marker), "{shell} script lacks {marker}");
    }
}

#[test]
fn man_renders_the_hand_written_sections() {
    let (stdout, _, code) = run(&["man"]);
    assert_eq!(code, 0);
    assert!(stdout.contains(".TH sokf 1"), "{stdout}");
    assert!(stdout.contains(".SH SYNOPSIS"), "{stdout}");
    assert!(stdout.contains(".SH COMMANDS"), "{stdout}");
    assert!(
        stdout.contains(".SS \"sokf validate [PATHS] [OPTIONS]\""),
        "{stdout}"
    );
    assert!(
        stdout.contains(".SS \"sokf new <TYPE> [OPTIONS]\""),
        "{stdout}"
    );
    assert!(
        stdout.contains(".SS \"sokf graph [START] [OPTIONS]\""),
        "{stdout}"
    );
    assert!(
        stdout.contains(".SS \"sokf locate <ID> [OPTIONS]\""),
        "{stdout}"
    );
    assert!(stdout.contains("no\\-cache"), "{stdout}");
    assert!(stdout.contains(".SH EXIT STATUS"), "{stdout}");
    assert!(stdout.contains(".SH FILES"), "{stdout}");
    assert!(!stdout.contains(".SS \"sokf man\""), "{stdout}");
}

// sokf-tests: CLI-5_AC-11
#[test]
fn info_formats_refuses_a_schema_two_roots_declare() {
    let two = Fixture::new("two-roots");
    let (_, stderr, code) = two.run(&["info", "formats", "--schema", "schema-note"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: info: schema `schema-note` is declared in more than one root: /knowledge, /other\n"
    );
}

// sokf-tests: CLI_P-1
#[test]
fn a_warm_cache_prints_exactly_what_a_cold_run_prints() {
    let f = Fixture::new("graph");
    let cold = f.run(&[
        "--no-cache",
        "graph",
        "planner",
        "knowledge",
        "--depth",
        "2",
    ]);
    let args = [
        "--repo-root",
        f.root(),
        "graph",
        "planner",
        "knowledge",
        "--depth",
        "2",
    ];
    let first = run(&args);
    assert!(f.0.join(".sokf/cache/.gitignore").exists());
    let warm = run(&args);
    assert_eq!(cold, first);
    assert_eq!(cold, warm);
    let clean = Fixture::new("clean");
    let first = clean.run(&["new", "Issue", "--slug", "x", "--id-only"]);
    let warm = clean.run(&["new", "Issue", "--slug", "x", "--id-only", "--no-cache"]);
    assert_eq!(first, warm);
    assert_eq!(first.0, "issue-002-x\n");
    let first = clean.run(&["info", "formats", "--schema", "schema-requirement"]);
    let warm = clean.run(&["info", "formats", "--schema", "schema-requirement"]);
    assert_eq!(first, warm);
}

// sokf-tests: CLI-4_AC-21
// sokf-tests: CLI-4_AC-22
#[test]
fn changed_stats_the_named_files_and_refuses_no_cache_and_an_outside_path() {
    let f = Fixture::new("clean");
    let (_, _, code) = f.run_here(&["validate"]);
    assert_eq!(code, 0);
    let (stdout, _, code) = f.run_here(&[
        "validate",
        "--changed",
        "knowledge/overview.md",
        "--changed",
        "knowledge/gone.md",
    ]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("0 errors"), "{stdout}");
    let (_, stderr, code) = f.run_here(&[
        "--no-cache",
        "validate",
        "--changed",
        "knowledge/overview.md",
    ]);
    assert_eq!(code, 2, "{stderr}");
    let (_, stderr, code) = f.run_here(&["validate", "--changed", "/../outside.md"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("outside.md"), "{stderr}");
}

// On macOS the temporary directory is reached through a symbolic link, so
// `--repo-root` names a path that the current directory, which the operating
// system reports in canonical form, doesn't start with. This test builds the
// same situation on every platform with a link to the fixture.
// sokf-tests: CLI-1_AC-18
#[cfg(unix)]
#[test]
fn a_repo_root_through_a_symbolic_link_resolves_a_path_from_the_current_directory() {
    let f = Fixture::new("clean");
    let link = f.0.parent().unwrap().join(format!(
        "sokf-cli-link-{}-{}",
        std::process::id(),
        f.0.file_name().unwrap().to_string_lossy()
    ));
    std::os::unix::fs::symlink(&f.0, &link).unwrap();
    let (stdout, stderr, code) = common::output(
        sokf()
            .arg("--repo-root")
            .arg(&link)
            .args([
                "validate",
                "--changed",
                "knowledge/overview.md",
                "--changed",
                "knowledge/gone.md",
            ])
            .current_dir(&f.0),
    );
    std::fs::remove_file(&link).unwrap();
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains("0 errors"), "{stdout}");
}

// sokf-tests: CLI-7_AC-65
// sokf-tests: CLI-7_AC-66
// sokf-tests: CLI-7_AC-77
#[test]
fn the_layout_names_each_store_and_ends_with_the_counts() {
    let f = Fixture::new("graph");
    let (stdout, _, code) = f.run(&["graph"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(!stdout.contains('→'), "{stdout}");
    assert!(
        stdout.ends_with("    └── subsystem\n\n4 / 4 docs\n"),
        "{stdout}"
    );
    // With a depth, the layout prints the edges of its documents, and
    // counts them.
    let (stdout, _, code) = f.run(&["graph", "--depth", "1"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("│   └── depended-on-by → planner (*)\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("\n4 / 4 docs\n"), "{stdout}");
}
