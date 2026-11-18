//! These tests run `sokf harness` end to end. They check that `sokf harness`
//! prints the expected text lines and JSON object, that each refusal goes to
//! stderr with exit code 2, that `sokf harness` accepts `--no-cache`, that
//! `status` exits with code 0 whatever the state of each part, and that the
//! hook reads its input from stdin.

use std::fs;

use common::{Fixture, output, run, sokf, with_hooks};

use crate::common;

// sokf-tests: CLI-3_AC-9
// sokf-tests: CLI-3_AC-19
#[test]
fn install_prints_one_line_per_part_then_all_current() {
    let f = Fixture::new("harness");
    f.write(".sokf/config.toml", &with_hooks(&["claude"]));
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "claude:\n  \
         updated /AGENTS.md (instructions)\n  \
         created /.claude/skills (skills)\n  \
         updated /.claude/settings.json (hooks)\n  \
         updated /.claude/settings.json (permissions)\n"
    );
    assert_eq!(stderr, "");
    let (stdout, _, code) = f.run(&["--no-cache", "harness", "install", "claude"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "claude:\n  \
         current /AGENTS.md (instructions)\n  \
         current /.claude/skills (skills)\n  \
         current /.claude/settings.json (hooks)\n  \
         current /.claude/settings.json (permissions)\n"
    );
    let (stdout, _, code) = f.run(&["harness", "status", "claude"]);
    assert_eq!(code, 0);
    assert!(
        stdout.lines().skip(1).all(|l| l.starts_with("  current ")),
        "{stdout}"
    );
}

// sokf-tests: CLI-3_AC-9
// sokf-tests: CLI-3_AC-11
// sokf-tests: CLI-3_AC-20
#[test]
fn json_is_one_object_and_status_exits_0_whatever_the_states() {
    let f = Fixture::new("harness");
    f.write(".sokf/config.toml", &with_hooks(&["claude"]));
    f.write(".claude/skills/sokf/SKILL.md", "mine\n");
    let (stdout, _, code) = f.run(&["harness", "status", "claude", "--json"]);
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["scope"], "project");
    assert_eq!(doc["harnesses"][0]["harness"], "claude");
    let states: Vec<_> = doc["harnesses"][0]["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["state"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(states, ["absent", "edited", "absent", "absent"]);
    // The user declines the hooks in the configuration, which no install changes.
    let config = format!("{}without = [\"hooks\"]\n", with_hooks(&["claude"]));
    f.write(".sokf/config.toml", &config);
    let (stdout, _, code) = f.run(&["harness", "install", "claude", "--json"]);
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let parts = &doc["harnesses"][0]["parts"];
    assert_eq!(parts[1]["state"], "edited");
    assert_eq!(parts[2]["state"], "skipped");
    assert_eq!(parts[3]["path"], "/.claude/settings.json");
    let (stdout, _, code) = f.run(&["harness", "install", "claude", "--force", "skills"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("  rewrote /.claude/skills (skills)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  skipped /.claude/settings.json (hooks)\n"),
        "{stdout}"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".sokf/config.toml")).unwrap(),
        config
    );
}

// The scenario of the issue that found the problem: the instructions and the skill are
// edited, the hooks are declined, and `--force skills` refreshes the skill alone.
// sokf-tests: CLI-3_AC-4
// sokf-tests: CLI-3_AC-5
// sokf-tests: CLI-3_AC-7
// sokf-tests: CLI-3_AC-8
// sokf-tests: CLI-3_AC-75
// sokf-tests: CLI-3_AC-76
// sokf-tests: CLI-3_AC-77
// sokf-tests: CLI-3_AC-78
// sokf-tests: CLI-3_AC-79
// sokf-tests: CLI-3_AC-80
#[test]
fn force_names_a_part_and_diff_shows_each_edit() {
    let f = Fixture::new("harness");
    // `with` opts in to the hooks and `without` declines them, and `without` wins.
    let config = format!("{}without = [\"hooks\"]\n", with_hooks(&["claude"]));
    f.write(".sokf/config.toml", &config);
    let (_, stderr, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{stderr}");
    // In this fixture the instructions part is the block in `AGENTS.md`.
    let agents_md = fs::read_to_string(f.0.join("AGENTS.md")).unwrap();
    f.write(
        "AGENTS.md",
        &agents_md.replace(
            "<!-- sokf:harness -->\n",
            "<!-- sokf:harness -->\nMy own line.\n",
        ),
    );
    let skill = fs::read_to_string(f.0.join(".claude/skills/sokf/SKILL.md")).unwrap();
    f.write(".claude/skills/sokf/SKILL.md", &format!("{skill}mine\n"));
    let record = fs::read_to_string(f.0.join(".sokf/harness.toml")).unwrap();
    // status names the two next steps beside each edited part.
    let (stdout, _, code) = f.run(&["harness", "status", "claude"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("  edited  /.claude/skills (skills)\n    next: `sokf harness diff claude` shows the edit, and `sokf harness install claude --force skills` overwrites it\n"),
        "{stdout}"
    );
    // diff prints a patch for each edited file and writes nothing.
    let (stdout, stderr, code) = f.run(&["harness", "diff", "claude"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("diff --git a/AGENTS.md b/AGENTS.md\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\n+My own line.\n"), "{stdout}");
    assert!(
        stdout
            .contains("diff --git a/.claude/skills/sokf/SKILL.md b/.claude/skills/sokf/SKILL.md\n"),
        "{stdout}"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".sokf/harness.toml")).unwrap(),
        record
    );
    // `--force skills` writes the skill and keeps the instructions' edit and hash.
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude", "--force", "skills"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("  rewrote /.claude/skills (skills)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  edited  /AGENTS.md (instructions)\n"),
        "{stdout}"
    );
    assert!(
        fs::read_to_string(f.0.join("AGENTS.md"))
            .unwrap()
            .contains("My own line.")
    );
    let instructions = |r: &str| {
        r.lines()
            .find(|l| l.starts_with("instructions = "))
            .map(str::to_string)
    };
    let after = fs::read_to_string(f.0.join(".sokf/harness.toml")).unwrap();
    assert!(instructions(&after).is_some());
    assert_eq!(instructions(&after), instructions(&record));
    assert_eq!(
        fs::read_to_string(f.0.join(".sokf/config.toml")).unwrap(),
        config
    );
    // `--force` on a declined part refuses with exit code 2 and names the config.
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude", "--force", "hooks"]);
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert_eq!(
        stderr,
        "error: harness: harness `claude` declines `hooks` in /.sokf/config.toml; remove it from `without` there to force it\n"
    );
    // Once nothing is edited, diff prints nothing.
    let (_, stderr, code) = f.run(&["harness", "install", "claude", "--force", "all"]);
    assert_eq!(code, 0, "{stderr}");
    let (stdout, _, code) = f.run(&["harness", "diff", "claude"]);
    assert_eq!((code, stdout.as_str()), (0, ""));
}

// sokf-tests: CLI-3_AC-22
// sokf-tests: CLI-3_AC-23
// sokf-tests: CLI-9_AC-1
// sokf-tests: CLI-9_AC-2
#[test]
fn every_refusal_exits_2_on_stderr_and_writes_nothing() {
    let f = Fixture::new("harness");
    let before = fs::read_to_string(f.0.join(".claude/settings.json")).unwrap();
    for (args, message) in [
        (
            vec!["harness", "install", "cline"],
            "error: harness: no harness named `cline`\n",
        ),
        (
            vec!["harness", "install", "claude", "--force", "mcp"],
            "error: harness: harness `claude` has no part named `mcp`\n",
        ),
        (
            vec!["harness", "status", "cline"],
            "error: harness: no harness named `cline`\n",
        ),
        (
            vec!["harness", "install", "claude", "cursor", "--scope", "local"],
            "error: harness: harness `cursor` has no local scope\n",
        ),
    ] {
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!(code, 2, "{stderr}");
        assert_eq!(stdout, "");
        assert_eq!(stderr, message);
    }
    f.write(".claude/settings.json", "{ nope");
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert!(
        stderr.starts_with("error: harness: /.claude/settings.json: does not parse"),
        "{stderr}"
    );
    assert!(!f.0.join(".claude/skills").exists());
    assert!(!f.0.join(".sokf/harness.toml").exists());
    f.write(".claude/settings.json", &before);
    // `sokf harness status` also refuses a repository root that doesn't
    // exist, with exit code 2.
    let (_, stderr, code) = run(&[
        "--repo-root",
        "/definitely/not/a/dir",
        "harness",
        "status",
        "claude",
    ]);
    assert_eq!(code, 2);
    assert!(stderr.starts_with("error: "), "{stderr}");
}

// sokf-tests: CLI-3_AC-24
#[test]
fn a_record_that_does_not_parse_exits_2() {
    let f = Fixture::new("harness");
    f.write(".sokf/harness.toml", "[claude\n");
    let before = f.files();
    for sub in ["install", "status"] {
        let (stdout, stderr, code) = f.run(&["harness", sub, "claude"]);
        assert_eq!(code, 2, "{sub}: {stderr}");
        assert_eq!(stdout, "", "{sub}");
        assert!(
            stderr.starts_with("error: harness: /.sokf/harness.toml: "),
            "{sub}: {stderr}"
        );
    }
    assert_eq!(f.files(), before);
    assert_eq!(
        fs::read_to_string(f.0.join(".sokf/harness.toml")).unwrap(),
        "[claude\n"
    );
}

fn hook(f: &Fixture, extra: &[&str], stdin: &str) -> (String, String, i32) {
    hook_of(f, "claude", extra, stdin)
}

fn hook_of(f: &Fixture, harness: &str, extra: &[&str], stdin: &str) -> (String, String, i32) {
    let mut args = vec!["--repo-root", f.root()];
    args.extend(extra);
    args.extend(["harness", "hook", harness, "prompt-submit"]);
    output(sokf().args(&args).write_stdin(stdin))
}

// sokf-tests: CLI-3_AC-25
// sokf-tests: CLI-3_AC-28
// sokf-tests: CLI-3_AC-82
// sokf-tests: CLI-3_AC-85
#[test]
fn the_hook_answers_json_and_exits_0() {
    let f = Fixture::new("clean");
    let (stdout, stderr, code) = hook(&f, &[], "{\"hook_event_name\":\"UserPromptSubmit\"}");
    assert_eq!((stdout.as_str(), stderr.as_str(), code), ("{}\n", "", 0));

    let f = Fixture::new("broken");
    let before = crate::common::tree(&f);
    let (stdout, stderr, code) = hook(&f, &[], "{}");
    assert_eq!((stderr.as_str(), code), ("", 0));
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        doc["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    let note = doc["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(note.starts_with("sokf: "), "{note}");
    assert!(
        note.ends_with(" errors. Fix before finishing the work."),
        "{note}"
    );
    // The hook changes no file.
    assert_eq!(crate::common::tree(&f), before);
    // The hook has already noted these errors, so it adds nothing.
    let (stdout, stderr, code) = hook(&f, &[], "{}");
    assert_eq!((stdout.as_str(), stderr.as_str(), code), ("{}\n", "", 0));
    // With the cache off, the hook has no record of the earlier note, so it
    // notes the errors again.
    let (stdout, _, code) = hook(&f, &["--no-cache"], "{}");
    assert_eq!(code, 0);
    assert!(
        stdout.contains("\"additionalContext\":\"sokf: "),
        "{stdout}"
    );
}

// sokf-tests: CLI-3_AC-69
// sokf-tests: CLI-3_AC-82
// sokf-tests: CLI-3_AC-83
#[test]
fn the_hook_answers_each_harness_in_its_own_form() {
    let f = Fixture::new("broken");
    for (harness, note) in [
        ("claude", "/hookSpecificOutput/additionalContext"),
        ("codex", "/hookSpecificOutput/additionalContext"),
        ("factory", "/hookSpecificOutput/additionalContext"),
        ("gemini", "/hookSpecificOutput/additionalContext"),
        ("pi", "/text"),
        ("opencode", "/text"),
        ("kilo", "/text"),
        ("qwen", "/hookSpecificOutput/additionalContext"),
        ("devin", "/hookSpecificOutput/additionalContext"),
    ] {
        let (stdout, stderr, code) = hook_of(&f, harness, &[], "{}");
        assert_eq!(code, 0, "{harness}: {stderr}");
        let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let text = doc
            .pointer(note)
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(text.starts_with("sokf: "), "{harness}: {stdout}");
        // Each harness has a guard of its own, so the second prompt gets nothing.
        let (stdout, stderr, code) = hook_of(&f, harness, &[], "{}");
        assert_eq!((stderr.as_str(), code), ("", 0), "{harness}");
        assert!(!stdout.contains("sokf: "), "{harness}: {stdout}");
    }
    // Copilot and Cursor have no answer that adds text to the agent's context
    // at the prompt, so the hook adds nothing.
    for harness in ["copilot", "cursor"] {
        let (stdout, stderr, code) = hook_of(&f, harness, &[], "{}");
        assert_eq!((stderr.as_str(), code), ("", 0), "{harness}");
        assert!(!stdout.contains("sokf: "), "{harness}: {stdout}");
    }
}

// sokf-tests: CLI-3_AC-84
#[test]
fn the_stop_hook_of_an_earlier_sokf_lets_the_agent_stop() {
    let f = Fixture::new("broken");
    let (stdout, stderr, code) = output(
        sokf()
            .args(["--repo-root", f.root(), "harness", "hook", "claude", "stop"])
            .write_stdin("{}"),
    );
    assert_eq!((stdout.as_str(), stderr.as_str(), code), ("{}\n", "", 0));
}

// sokf-tests: CLI-3_AC-36
#[test]
fn an_install_moves_the_hook_of_an_earlier_sokf_to_the_prompt() {
    let f = Fixture::new("clean");
    f.write(".sokf/config.toml", &with_hooks(&["claude"]));
    f.write(
        ".claude/settings.json",
        "{\"hooks\":{\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"echo mine\"},{\"type\":\"command\",\"command\":\"sokf harness hook claude stop\"}]}]}}\n",
    );
    let (out, err, code) = f.run(&["harness", "install", "claude", "--force", "hooks"]);
    assert_eq!(code, 0, "{out}{err}");
    let settings = fs::read_to_string(f.path(".claude/settings.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&settings).unwrap();
    assert_eq!(
        doc["hooks"]["Stop"],
        serde_json::json!([{ "hooks": [{ "type": "command", "command": "echo mine" }] }]),
        "{settings}"
    );
    assert_eq!(
        doc["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
        "sokf harness hook claude prompt-submit",
        "{settings}"
    );
}

// sokf-tests: CLI-3_AC-30
// sokf-tests: CLI-3_AC-31
// sokf-tests: CLI-3_AC-68
#[test]
fn the_hook_never_blocks_on_a_failure_of_the_tool() {
    let f = Fixture::new("clean");
    // An unknown event, an unknown harness, or a harness that runs no hooks is a
    // usage error, which exits with code 1, because a harness reads code 2 at the
    // prompt as a refusal of the prompt.
    for (harness, event, message) in [
        (
            "claude",
            "start",
            "error: harness: sokf has no hook for the event `start`\n",
        ),
        (
            "cline",
            "prompt-submit",
            "error: harness: no harness named `cline`\n",
        ),
        (
            "agents",
            "prompt-submit",
            "error: harness: harness `agents` runs no hooks\n",
        ),
    ] {
        let (stdout, stderr, code) = output(
            sokf()
                .args(["--repo-root", f.root(), "harness", "hook", harness, event])
                .write_stdin("{}"),
        );
        assert_eq!((stdout.as_str(), code), ("", 1), "{harness} {event}");
        assert_eq!(stderr, message);
    }
    // A broken configuration makes the hook exit with code 1, with the message
    // on stderr and nothing on stdout.
    f.write(".sokf/config.toml", "[knowledge]\nroots = [\"/missing\"]\n");
    let (stdout, stderr, code) = hook(&f, &[], "{}");
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(stdout, "");
    assert!(
        stderr.starts_with("error: /.sokf/config.toml: "),
        "{stderr}"
    );
    // A missing repository root also makes the hook exit with code 1, because a
    // failure of the tool must not block the agent.
    let out = sokf()
        .args([
            "--repo-root",
            "/definitely/not/a/dir",
            "harness",
            "hook",
            "claude",
            "prompt-submit",
        ])
        .write_stdin("{}")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "");
    // Outside any repository, as a hook installed for the user runs, the hook
    // adds nothing.
    let outside = std::env::temp_dir().join(format!("sokf-cli-outside-{}", std::process::id()));
    fs::create_dir_all(&outside).unwrap();
    let (stdout, stderr, code) = output(
        sokf()
            .args(["harness", "hook", "claude", "prompt-submit"])
            .current_dir(&outside)
            .write_stdin("{}"),
    );
    assert_eq!((stdout.as_str(), stderr.as_str(), code), ("{}\n", "", 0));
    fs::remove_dir_all(&outside).unwrap();
}

/// `installed_block` installs `args` into a repository without harness files,
/// whose `/.sokf/config.toml` is `config` when it is given, and returns the
/// instructions block of `file`.
fn installed_block(args: &[&str], file: &str, config: Option<&str>) -> String {
    let f = Fixture::new("clean");
    if let Some(config) = config {
        f.write(".sokf/config.toml", config);
    }
    let mut all = vec!["harness", "install"];
    all.extend(args);
    let (_, stderr, code) = f.run(&all);
    assert_eq!(code, 0, "{args:?}: {stderr}");
    let text = fs::read_to_string(f.path(file)).unwrap();
    let start = text.find("<!-- sokf:harness -->").expect("the block opens");
    let end = text
        .find("<!-- /sokf:harness -->")
        .expect("the block closes");
    text[start..end].to_string()
}

// sokf-tests: CLI-3_AC-35
// sokf-tests: CLI-3_AC-70
// sokf-tests: CLI-3_AC-112
#[test]
fn every_harness_writes_the_same_block_with_or_without_the_hook() {
    let claude = installed_block(&["claude"], "CLAUDE.md", Some(&with_hooks(&["claude"])));
    let without = installed_block(
        &["claude"],
        "CLAUDE.md",
        Some("[harness.claude]\nwithout = [\"hooks\"]\n"),
    );
    let agents = installed_block(&["agents"], "AGENTS.md", None);
    assert_eq!(without, claude);
    assert_eq!(agents, claude);
    for words in [
        "`/.sokf/config.toml`",
        "read the `sokf` skill",
        "Before you end a turn, run `sokf validate --fix` and fix every error it reports.",
        "A hook may tell you that errors remain; fix them before you finish the work.",
        "Before modifying a knowledge store document, use `sokf guide <PATH>` to read the update guidance to follow when writing.",
        "Add `--state <KEY>=<STATE>` when changing document state.",
    ] {
        assert!(claude.contains(words), "{words}: {claude}");
    }
    // The line about the guidance follows the line about the skill, and
    // comes before the line about `sokf validate --fix`.
    assert!(
        claude.contains("read the `sokf` skill.\nBefore modifying a knowledge store document,"),
        "{claude}"
    );
    assert!(
        claude.contains("when changing document state.\nBefore you end a turn,"),
        "{claude}"
    );
    assert!(!claude.contains("a hook runs `sokf"), "{claude}");
}

// sokf-tests: CLI-4_AC-40
// sokf-tests: CLI-4_AC-43
#[test]
fn validate_warns_at_a_stale_part_and_not_at_an_edited_one() {
    let f = crate::common::project();
    let config = fs::read_to_string(f.path(".sokf/config.toml")).unwrap_or_default();
    let config = format!("{config}\n{}", with_hooks(&["claude"]));
    f.write(".sokf/config.toml", &config);
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    let (out, _, code) = f.run(&["validate", "--warnings"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("is stale"), "{out}");
    // A new program for the hook lines makes the installed hooks stale,
    // though nobody edited them.
    f.write(
        ".sokf/config.toml",
        &format!("{config}command = \"other-sokf\"\n"),
    );
    let (out, _, code) = f.run(&["validate", "--warnings"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(
            "/.claude/settings.json: warning: the hooks part of the claude harness is stale"
        ),
        "{out}"
    );
    assert!(out.contains("run `sokf harness install claude`"), "{out}");
    // The agent sees the warning without asking for the warnings.
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(
            "/.claude/settings.json: warning: the hooks part of the claude harness is stale"
        ),
        "{out}"
    );
    assert!(out.ends_with("0 errors, 1 of 1 warnings listed\n"), "{out}");
    // Once the part is installed again, the warning goes.
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    let (out, _, _) = f.run(&["validate", "--warnings"]);
    assert!(!out.contains("is stale"), "{out}");
    // An edited part is the user's choice, so it gets no warning.
    let skill = f.path(".claude/skills/sokf/SKILL.md");
    let text = fs::read_to_string(&skill).unwrap();
    fs::write(&skill, format!("{text}\nA note of the project.\n")).unwrap();
    let (status, _, _) = f.run(&["harness", "status", "claude"]);
    assert!(status.contains("edited"), "{status}");
    let (out, _, _) = f.run(&["validate", "--warnings"]);
    assert!(!out.contains("is stale"), "{out}");
}

// sokf-tests: CLI-4_AC-42
// sokf-tests: CLI-4_AC-43
#[test]
fn validate_warns_at_an_absent_part_unless_the_configuration_declines_it() {
    let f = crate::common::project();
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    // A deleted skill reads as absent, as does a part that a newer sokf adds.
    fs::remove_dir_all(f.path(".claude/skills")).unwrap();
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(
            "/.claude/skills: warning: the skills part of the claude harness is absent; run `sokf harness install claude` to write it (CLI-4_AC-42)"
        ),
        "{out}"
    );
    // A part that the configuration declines is skipped, not absent.
    f.write(
        ".sokf/config.toml",
        &format!(
            "{}\n[harness.claude]\nwithout = [\"skills\"]\n",
            fs::read_to_string(f.path(".sokf/config.toml")).unwrap_or_default()
        ),
    );
    let (status, _, code) = f.run(&["harness", "status", "claude"]);
    assert_eq!(code, 0, "{status}");
    assert!(
        status.contains("skipped /.claude/skills (skills)"),
        "{status}"
    );
    let (out, _, code) = f.run(&["validate", "--warnings"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("is absent"), "{out}");
}

// sokf-tests: CONF-4_AC-4
// sokf-tests: CONF-4_AC-5
// sokf-tests: CLI-3_AC-86
#[test]
fn the_hooks_part_is_off_until_with_lists_it() {
    let f = crate::common::project();
    let config = fs::read_to_string(f.path(".sokf/config.toml")).unwrap_or_default();
    // Without `with`, the install writes no hook, and status reads the part as skipped.
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    let settings = fs::read_to_string(f.path(".claude/settings.json")).unwrap();
    assert!(!settings.contains("harness hook"), "{settings}");
    let (status, _, code) = f.run(&["harness", "status", "claude"]);
    assert_eq!(code, 0, "{status}");
    assert!(
        status.contains("  skipped /.claude/settings.json (hooks)\n"),
        "{status}"
    );
    // `--force hooks` refuses with exit code 2, says to add the part to `with`, and writes
    // nothing.
    let before = crate::common::tree(&f);
    let (stdout, stderr, code) = f.run(&["harness", "install", "claude", "--force", "hooks"]);
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert_eq!(
        stderr,
        "error: harness: harness `claude` installs `hooks` only when `with` lists it; add `with = [\"hooks\"]` under `[harness.claude]` in /.sokf/config.toml to install it\n"
    );
    assert_eq!(crate::common::tree(&f), before);
    // Where `without` declines the hooks too, the refusal says to remove them
    // from `without`, because adding them to `with` alone would not help.
    f.write(
        ".sokf/config.toml",
        &format!("{config}\n[harness.claude]\nwithout = [\"hooks\"]\n"),
    );
    let (_, err, code) = f.run(&["harness", "install", "claude", "--force", "hooks"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("remove it from `without`"), "{err}");
    // Once `with` lists the hooks, the install writes the hook.
    f.write(
        ".sokf/config.toml",
        &format!("{config}\n{}", with_hooks(&["claude"])),
    );
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    let settings = fs::read_to_string(f.path(".claude/settings.json")).unwrap();
    assert!(
        settings.contains("sokf harness hook claude prompt-submit"),
        "{settings}"
    );
}

// sokf-tests: CONF-4_AC-6
#[test]
fn a_with_list_of_a_part_that_is_on_by_default_is_refused() {
    let f = crate::common::project();
    let config = fs::read_to_string(f.path(".sokf/config.toml")).unwrap_or_default();
    f.write(
        ".sokf/config.toml",
        &format!("{config}\n[harness.claude]\nwith = [\"skills\"]\n"),
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_ne!(code, 0, "{out}{err}");
    assert!(err.contains("harness.claude.with"), "{out}{err}");
}
