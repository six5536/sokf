//! These tests run `sokf harness install` and `sokf harness status` end to end
//! for several harnesses at once and at each scope. They check that a location
//! that several harnesses read is written once, that `all` names every
//! harness, and that the user and local scopes write where they should.

use std::{fs, path::Path};

use common::{Fixture, output, sokf, with_hooks};

use crate::common;

/// `run_home` runs the binary on the fixture with the home directory `home`.
fn run_home(f: &Fixture, home: &Path, args: &[&str]) -> (String, String, i32) {
    output(
        sokf()
            .env("HOME", home)
            .env("USERPROFILE", home)
            .arg("--repo-root")
            .arg(f.root())
            .args(args),
    )
}

// sokf-tests: CLI-3_AC-65
// sokf-tests: CLI-3_AC-67
// sokf-tests: CLI_P-2
#[test]
fn several_harnesses_share_what_they_read_in_one_place() {
    let f = Fixture::new("harness");
    f.write(
        ".sokf/config.toml",
        &with_hooks(&["claude", "codex", "cursor"]),
    );
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude", "codex", "cursor"]);
    assert_eq!(code, 0, "{stderr}");
    for line in [
        "claude:\n  updated /AGENTS.md (instructions)\n  created /.claude/skills (skills)\n",
        "codex:\n  shared  /AGENTS.md (instructions, by claude)\n  created /.agents/skills (skills)\n  created /.codex/hooks.json (hooks)\n  unsupported: permissions\n",
        "cursor:\n  shared  /AGENTS.md (instructions, by claude)\n  shared  /.claude/skills (skills, by claude)\n  created /.cursor/hooks.json (hooks)\n  created /.cursor/cli.json (permissions)\n",
        "warning: cursor may load the skills twice: .claude/skills, .agents/skills\n",
    ] {
        assert!(stdout.contains(line), "{line}\nin\n{stdout}");
    }
    // The instructions are written once, into the file that every agent reads.
    let agents = fs::read_to_string(f.0.join("AGENTS.md")).unwrap();
    assert_eq!(
        agents.matches("<!-- sokf:harness -->").count(),
        1,
        "{agents}"
    );
    assert_eq!(
        fs::read_to_string(f.0.join("CLAUDE.md")).unwrap(),
        "@AGENTS.md\n"
    );
    // Each harness's hook runs the hook command with its own name.
    let codex = fs::read_to_string(f.0.join(".codex/hooks.json")).unwrap();
    assert!(
        codex.contains("\"sokf harness hook codex prompt-submit\""),
        "{codex}"
    );
    let cursor = fs::read_to_string(f.0.join(".cursor/hooks.json")).unwrap();
    assert!(
        cursor.contains("\"sokf harness hook cursor prompt-submit\""),
        "{cursor}"
    );
    let record = fs::read_to_string(f.0.join(".sokf/harness.toml")).unwrap();
    for table in ["[claude]", "[codex]", "[cursor]"] {
        assert!(record.contains(table), "{record}");
    }
    // A second install writes nothing, and every part is current or shared.
    let before = f.files();
    let (stdout, _, code) = f.run(&["harness", "status", "claude", "codex", "cursor", "--json"]);
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    for h in doc["harnesses"].as_array().unwrap() {
        for p in h["parts"].as_array().unwrap() {
            assert!(p["state"] == "current" || p["state"] == "shared", "{h}");
            assert_eq!(p.get("by").is_some(), p["state"] == "shared", "{p}");
        }
    }
    f.run(&["harness", "install", "claude", "codex", "cursor"]);
    assert_eq!(f.files(), before);
}

// sokf-tests: CLI-3_AC-65
#[test]
fn all_names_every_harness() {
    let f = Fixture::new("harness");
    // Each harness that runs hooks opts in to them.
    f.write(
        ".sokf/config.toml",
        &with_hooks(&[
            "claude", "codex", "factory", "gemini", "copilot", "cursor", "pi", "opencode", "kilo",
            "qwen", "devin",
        ]),
    );
    let (stdout, stderr, code) = f.run(&["harness", "install", "all", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let names: Vec<_> = doc["harnesses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["harness"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        names,
        [
            "claude", "codex", "factory", "gemini", "copilot", "cursor", "pi", "opencode", "kilo",
            "qwen", "devin", "agents"
        ]
    );
    for file in [
        ".factory/hooks.json",
        ".gemini/settings.json",
        "GEMINI.md",
        ".github/hooks/sokf.json",
        ".pi/extensions/sokf.ts",
        ".agents/skills/sokf/SKILL.md",
    ] {
        assert!(f.0.join(file).is_file(), "{file}");
    }
    let (stdout, _, code) = f.run(&["harness", "status", "all"]);
    assert_eq!(code, 0);
    assert!(
        stdout
            .lines()
            .filter(|l| l.starts_with("  ") && !l.contains(':'))
            .all(|l| l.starts_with("  current ") || l.starts_with("  shared  ")),
        "{stdout}"
    );
    // The configuration declines a part of each harness, and no install changes it.
    let config = "[harness.codex]\nwith = [\"hooks\"]\nwithout = [\"hooks\"]\n\n[harness.pi]\nwith = [\"hooks\"]\nwithout = [\"hooks\"]\n";
    f.write(".sokf/config.toml", config);
    let (stdout, _, code) = f.run(&["harness", "install", "codex", "pi"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("  skipped /.codex/hooks.json (hooks)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  skipped /.pi/extensions (hooks)\n"),
        "{stdout}"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".sokf/config.toml")).unwrap(),
        config
    );
}

// sokf-tests: CLI-3_AC-66
#[test]
fn the_user_scope_writes_under_the_home_directory() {
    let f = Fixture::new("harness");
    let home = f.0.join("home");
    fs::create_dir_all(home.join(".sokf")).unwrap();
    fs::write(home.join(".sokf/config.toml"), with_hooks(&["claude"])).unwrap();
    let before = f.files();
    let (stdout, stderr, code) = run_home(
        &f,
        &home,
        &["harness", "install", "claude", "--scope", "user"],
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "claude:\n  \
         created ~/.claude/CLAUDE.md (instructions)\n  \
         created ~/.claude/skills (skills)\n  \
         created ~/.claude/settings.json (hooks)\n  \
         created ~/.claude/settings.json (permissions)\n"
    );
    assert!(home.join(".claude/skills/sokf/SKILL.md").is_file());
    assert!(home.join(".sokf/harness.toml").is_file());
    // The repository's own files stay as they were.
    let after: Vec<_> = f
        .files()
        .into_iter()
        .filter(|(p, _)| !p.starts_with("home/"))
        .collect();
    assert_eq!(after, before);
    // At the user scope, the home directory's configuration declines a part, and a
    // `--force` on that part names that file.
    fs::write(
        home.join(".sokf/config.toml"),
        format!("{}without = [\"hooks\"]\n", with_hooks(&["claude"])),
    )
    .unwrap();
    let (_, stderr, code) = run_home(
        &f,
        &home,
        &["harness", "install", "claude", "--scope", "user"],
    );
    assert_eq!(code, 0, "{stderr}");
    let (_, stderr, code) = run_home(
        &f,
        &home,
        &[
            "harness", "install", "claude", "--scope", "user", "--force", "hooks",
        ],
    );
    assert_eq!(code, 2);
    assert!(
        stderr.contains("declines `hooks` in ~/.sokf/config.toml"),
        "{stderr}"
    );
    assert!(!f.0.join(".sokf/config.toml").exists());
    let (stdout, _, code) = run_home(
        &f,
        &home,
        &["harness", "status", "claude", "--scope", "user", "--json"],
    );
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["scope"], "user");
    assert_eq!(doc["harnesses"][0]["parts"][2]["state"], "skipped");
}

// sokf-tests: CLI-3_AC-66
#[test]
fn the_local_scope_writes_the_files_that_stay_out_of_git() {
    let f = Fixture::new("harness");
    f.write(".sokf/config.local.toml", &with_hooks(&["claude"]));
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude", "--scope", "local"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "claude:\n  \
         created /.claude/settings.local.json (hooks)\n  \
         created /.claude/settings.local.json (permissions)\n  \
         unsupported: instructions, skills\n"
    );
    assert!(f.0.join(".sokf/harness.local.toml").is_file());
    assert!(!f.0.join(".sokf/harness.toml").exists());
    let settings = fs::read_to_string(f.0.join(".claude/settings.local.json")).unwrap();
    assert!(
        settings.contains("sokf harness hook claude prompt-submit"),
        "{settings}"
    );
}

// sokf-tests: CONF-4_AC-2
// sokf-tests: CONF-4_AC-3
#[test]
fn a_configured_command_is_the_hook_program_of_its_harness_only() {
    let f = Fixture::new("harness");
    f.write(
        ".sokf/config.toml",
        &format!(
            "{}command = '\"$CLAUDE_PROJECT_DIR/sokf\"'\n\n{}",
            with_hooks(&["claude"]),
            with_hooks(&["codex"])
        ),
    );
    let (_, stderr, code) = f.run(&["harness", "install", "claude", "codex"]);
    assert_eq!(code, 0, "{stderr}");
    let claude = fs::read_to_string(f.0.join(".claude/settings.json")).unwrap();
    assert!(
        claude.contains("\\\"$CLAUDE_PROJECT_DIR/sokf\\\" harness hook claude prompt-submit"),
        "{claude}"
    );
    let codex = fs::read_to_string(f.0.join(".codex/hooks.json")).unwrap();
    assert!(
        codex.contains("\"sokf harness hook codex prompt-submit\""),
        "{codex}"
    );
    let (stdout, _, _) = f.run(&["harness", "status", "claude", "codex"]);
    assert!(
        stdout.contains("  current /.claude/settings.json (hooks)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  current /.codex/hooks.json (hooks)\n"),
        "{stdout}"
    );
    // Status names no harness that it was not asked about.
    assert!(!stdout.contains("cursor"), "{stdout}");
}
