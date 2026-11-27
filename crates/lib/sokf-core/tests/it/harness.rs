//! These tests run `harness install` and `harness status` on temporary
//! repositories. Each test runs one scenario of CLI-3 through the core
//! library and checks the tree, the record, and the result. Every refusal
//! must leave the tree byte-identical.

// sokf-tests: CLI-9_AC-1

use common::TempTree;
use sokf_core::{
    Error, HarnessDirs, HarnessOptions, HarnessReport, HarnessScope, PartResult, harness_install,
    harness_status,
};

use crate::common;

/// `install` installs Claude Code. With `without`, it first writes the configuration so that
/// it declines those parts, as the user does; with `force`, it passes `--force all`.
fn install(
    tree: &TempTree,
    without: Option<&[&str]>,
    force: bool,
) -> sokf_core::Result<HarnessReport> {
    if let Some(w) = without {
        decline(tree, w);
    }
    let force: &[&str] = if force { &["all"] } else { &[] };
    let opts = HarnessOptions::new(["claude"], HarnessScope::Project).force(force.iter().copied());
    harness_install(&HarnessDirs::repo(tree.root()), &opts)
}

/// `decline` writes `/.sokf/config.toml` so that it opts in to the hooks of Claude Code and
/// declines `parts` of it.
fn decline(tree: &TempTree, parts: &[&str]) {
    let list: Vec<String> = parts.iter().map(|p| format!("\"{p}\"")).collect();
    tree.write(
        ".sokf/config.toml",
        &format!("{}without = [{}]\n", with_hooks(), list.join(", ")),
    );
}

/// `with_hooks` returns the configuration that opts Claude Code in to its hooks.
fn with_hooks() -> String {
    common::with_hooks(&["claude"])
}

/// `opt_in` writes `/.sokf/config.toml` so that it opts Claude Code in to its hooks.
fn opt_in(tree: &TempTree) {
    tree.write(".sokf/config.toml", &with_hooks());
}

fn status(tree: &TempTree) -> sokf_core::Result<HarnessReport> {
    harness_status(
        &HarnessDirs::repo(tree.root()),
        &["claude".to_string()],
        HarnessScope::Project,
    )
}

/// `parts` returns the parts of Claude Code, the only harness named.
fn parts(report: &HarnessReport) -> &[PartResult] {
    &report.harness("claude").expect("claude").parts
}

fn verbs(report: &HarnessReport) -> Vec<&str> {
    parts(report).iter().map(PartResult::verb).collect()
}

fn states(result: &HarnessReport) -> Vec<(&str, &str, &str)> {
    parts(result)
        .iter()
        .map(|p| (p.part.as_str(), p.verb(), p.path.as_str()))
        .collect()
}

// sokf-tests: CLI-3_AC-1
// sokf-tests: CLI-3_AC-3
// sokf-tests: CLI-3_AC-6
// sokf-tests: CONF-4_AC-3
// sokf-tests: CLI-3_AC-14
// sokf-tests: CLI-3_AC-32
// sokf-tests: CLI-3_AC-34
// sokf-tests: CLI-3_AC-36
// sokf-tests: CLI-3_AC-37
// sokf-tests: CLI-3_AC-38
// sokf-tests: CLI-3_AC-47
// sokf-tests: CLI-3_AC-52
// sokf-tests: CLI-3_AC-54
// sokf-tests: CLI_P-2
#[test]
fn an_empty_repository_gets_every_part_and_the_record() {
    let tree = TempTree::empty("harness-empty");
    opt_in(&tree);
    let out = install(&tree, None, false).unwrap();
    assert_eq!(
        states(&out),
        [
            ("instructions", "created", "/CLAUDE.md"),
            ("skills", "created", "/.claude/skills"),
            ("hooks", "created", "/.claude/settings.json"),
            ("permissions", "created", "/.claude/settings.json"),
        ]
    );
    let skill = tree.read(".claude/skills/sokf/SKILL.md");
    assert!(skill.starts_with("---\nname: sokf\n"), "{skill}");
    assert!(skill.contains("Written by sokf "), "{skill}");
    assert!(skill.contains("# SOKF v0.7"), "{skill}");
    assert_eq!(
        tree.read("CLAUDE.md"),
        format!(
            "<!-- sokf:harness -->\n\n{}<!-- /sokf:harness -->\n",
            include_str!("../../../../../harness/instructions.md")
        )
    );
    let settings: serde_json::Value =
        serde_json::from_str(&tree.read(".claude/settings.json")).unwrap();
    assert_eq!(settings["permissions"]["allow"][0], "Bash(sokf *)");
    assert_eq!(
        settings["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
        "sokf harness hook claude prompt-submit"
    );
    let record = tree.read(".sokf/harness.toml");
    assert!(
        record.starts_with("# Written by sokf harness install. Do not edit it: sokf compares the parts with these hashes to tell a part you edited from a stale one.\n\n[claude]\n"),
        "{record}"
    );
    // Each part has its fingerprint, and the skills part a table of its file's.
    let table: toml::Table = record.parse().unwrap();
    let claude = table["claude"].as_table().unwrap();
    for part in ["hooks", "instructions", "permissions"] {
        assert!(claude[part].is_str(), "{record}");
    }
    assert!(claude["skills"]["sokf/SKILL.md"].is_str(), "{record}");
    assert!(!record.contains("fnv1a64:"), "{record}");
    assert_eq!(tree.read(".sokf/config.toml"), with_hooks());
    // Install is idempotent: the second run writes nothing and reports every
    // part current.
    let before = tree.files();
    let again = install(&tree, None, false).unwrap();
    assert!(verbs(&again).iter().all(|v| *v == "current"), "{again:?}");
    assert_eq!(tree.files(), before);
    let st = status(&tree).unwrap();
    assert!(verbs(&st).iter().all(|v| *v == "current"), "{st:?}");
}

// sokf-tests: CLI-3_AC-33
// sokf-tests: CLI-3_AC-34
// sokf-tests: CLI-3_AC-40
#[test]
fn the_instructions_target_follows_the_tree() {
    // With only AGENTS.md, which the agent reads directly, the block goes into
    // AGENTS.md.
    let tree = TempTree::empty("harness-agents");
    tree.write("AGENTS.md", "# Agents\n");
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[0].path.as_str(), "/AGENTS.md");
    assert_eq!(parts(&out)[0].verb(), "updated");
    assert!(
        tree.read("AGENTS.md")
            .starts_with("# Agents\n\n<!-- sokf:harness -->\n")
    );
    assert!(!tree.dir().join("CLAUDE.md").exists());
    // When both files exist and CLAUDE.md imports AGENTS.md, the block goes
    // into AGENTS.md and CLAUDE.md stays untouched.
    let tree = TempTree::fixture("harness");
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[0].path.as_str(), "/AGENTS.md");
    assert_eq!(tree.read("CLAUDE.md"), "@AGENTS.md\n");
    // When both files exist and CLAUDE.md doesn't import AGENTS.md, the block
    // goes into CLAUDE.md.
    let tree = TempTree::empty("harness-both");
    tree.write("AGENTS.md", "# Agents\n");
    tree.write("CLAUDE.md", "# Claude\n");
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[0].path.as_str(), "/CLAUDE.md");
    assert_eq!(tree.read("AGENTS.md"), "# Agents\n");
    assert!(
        tree.read("CLAUDE.md")
            .starts_with("# Claude\n\n<!-- sokf:harness -->\n")
    );
}

// sokf-tests: CLI-3_AC-17
// sokf-tests: CLI-3_AC-18
#[test]
fn a_settings_file_of_another_style_keeps_its_entries_order_and_indent() {
    let tree = TempTree::fixture("harness");
    opt_in(&tree);
    let before = tree.read(".claude/settings.json");
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[2].verb(), "updated");
    assert_eq!(parts(&out)[3].verb(), "updated");
    let after = tree.read(".claude/settings.json");
    let doc: serde_json::Value = serde_json::from_str(&after).unwrap();
    let keys: Vec<_> = doc.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["permissions", "hooks", "model"]);
    assert_eq!(
        doc["permissions"]["allow"],
        serde_json::json!(["Bash(npm run *)", "Bash(sokf *)"])
    );
    // The user's hook at the stop stays, and sokf's hook goes to the prompt.
    assert_eq!(doc["hooks"]["Stop"][0]["hooks"][0]["command"], "echo done");
    assert_eq!(
        doc["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
        "sokf harness hook claude prompt-submit"
    );
    assert_eq!(doc["model"], "opus");
    assert!(
        after.starts_with("{\n    \"permissions\": {\n        \"allow\": ["),
        "{after}"
    );
    assert!(
        !after.ends_with('\n'),
        "the original file had no trailing newline"
    );
    assert_ne!(before, after);
    let st = status(&tree).unwrap();
    assert!(verbs(&st).iter().all(|v| *v == "current"), "{st:?}");
}

// sokf-tests: CLI-3_AC-4
// sokf-tests: CLI-3_AC-5
#[test]
fn a_skill_the_tool_never_wrote_is_edited_until_forced() {
    let tree = TempTree::empty("harness-foreign");
    tree.write(
        ".claude/skills/sokf/SKILL.md",
        "---\nname: sokf\n---\nmine\n",
    );
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[1].verb(), "edited");
    assert_eq!(
        tree.read(".claude/skills/sokf/SKILL.md"),
        "---\nname: sokf\n---\nmine\n"
    );
    let record = tree.read(".sokf/harness.toml");
    assert!(
        !record.contains("skills"),
        "an edited part is not recorded: {record}"
    );
    let st = status(&tree).unwrap();
    assert_eq!(parts(&st)[1].verb(), "edited");
    let out = install(&tree, None, true).unwrap();
    assert_eq!(parts(&out)[1].verb(), "rewrote");
    assert!(
        tree.read(".claude/skills/sokf/SKILL.md")
            .contains("# SOKF v0.7")
    );
    assert!(
        tree.read(".sokf/harness.toml")
            .contains("\n[claude.skills]\n")
    );
}

// sokf-tests: CLI-3_AC-4
// sokf-tests: CLI-3_AC-5
// sokf-tests: CLI-3_AC-42
// sokf-tests: CLI-3_AC-46
// sokf-tests: CLI-3_AC-49
#[test]
fn every_part_edited_then_install_then_force() {
    let tree = TempTree::empty("harness-edited");
    opt_in(&tree);
    install(&tree, None, false).unwrap();
    tree.write(".claude/skills/sokf/SKILL.md", "changed\n");
    let claude = tree.read("CLAUDE.md").replace("read the", "skip the");
    tree.write("CLAUDE.md", &claude);
    let settings = tree
        .read(".claude/settings.json")
        .replace(
            "sokf harness hook claude prompt-submit",
            "sokf harness hook claude prompt-submit --quiet",
        )
        .replace("Bash(sokf *)", "Bash(sokf validate *)");
    tree.write(".claude/settings.json", &settings);
    let st = status(&tree).unwrap();
    // The permission entry is gone, so the permissions part is absent. The
    // hook group still holds the tool's hook command, so the hooks part is
    // edited.
    assert_eq!(verbs(&st), ["edited", "edited", "edited", "absent"]);
    let out = install(&tree, None, false).unwrap();
    assert_eq!(verbs(&out), ["edited", "edited", "edited", "updated"]);
    assert_eq!(tree.read(".claude/skills/sokf/SKILL.md"), "changed\n");
    assert!(tree.read("CLAUDE.md").contains("skip the"));
    assert!(tree.read(".claude/settings.json").contains("--quiet"));
    let out = install(&tree, None, true).unwrap();
    assert_eq!(verbs(&out), ["updated", "rewrote", "updated", "current"]);
    let st = status(&tree).unwrap();
    assert!(verbs(&st).iter().all(|v| *v == "current"), "{st:?}");
}

// sokf-tests: CLI-3_AC-3
// sokf-tests: CLI-3_AC-7
// sokf-tests: CLI-3_AC-8
// sokf-tests: CONF-4_AC-1
#[test]
fn a_declined_part_stays_declined_until_the_config_line_goes() {
    let tree = TempTree::empty("harness-declined");
    let out = install(&tree, Some(&["hooks"]), false).unwrap();
    assert_eq!(parts(&out)[2].verb(), "skipped");
    // `without` wins over `with`.
    let declined = format!("{}without = [\"hooks\"]\n", with_hooks());
    assert_eq!(tree.read(".sokf/config.toml"), declined);
    let settings = tree.read(".claude/settings.json");
    assert!(!settings.contains("hooks"), "{settings}");
    // A second run keeps the hooks declined, and leaves the config as it is.
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[2].verb(), "skipped");
    assert_eq!(parts(&status(&tree).unwrap())[2].verb(), "skipped");
    assert_eq!(tree.read(".sokf/config.toml"), declined);
    // The user changes the list, so the hooks come back and the skill is declined.
    let skill_hash = |tree: &TempTree| {
        let record: toml::Table = tree.read(".sokf/harness.toml").parse().unwrap();
        record["claude"].get("skills").map(ToString::to_string)
    };
    let before = skill_hash(&tree);
    assert!(before.is_some());
    let out = install(&tree, Some(&["skills"]), false).unwrap();
    assert_eq!(verbs(&out), ["current", "skipped", "updated", "current"]);
    // The skill file stays as it was written, and the record keeps its hash.
    assert_eq!(skill_hash(&tree), before);
    // The user removes the `without` line by hand, and `install` then installs every
    // part again.
    tree.write(
        ".sokf/config.toml",
        &format!("# nothing declined\n{}", with_hooks()),
    );
    let out = install(&tree, None, false).unwrap();
    assert!(verbs(&out).iter().all(|v| *v == "current"), "{out:?}");
    assert!(tree.read(".sokf/harness.toml").contains("[claude.skills]"));
}

// sokf-tests: CLI-3_AC-20
// sokf-tests: CLI-3_AC-45
// sokf-tests: CLI-3_AC-61
#[test]
fn json_carries_the_same_parts() {
    let tree = TempTree::empty("harness-json");
    let out = install(&tree, Some(&["permissions"]), false).unwrap();
    let json = serde_json::to_value(&out).unwrap();
    let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["scope", "harnesses", "warnings"]);
    assert_eq!(json["scope"], "project");
    let claude = &json["harnesses"][0];
    assert_eq!(claude["harness"], "claude");
    assert_eq!(claude["unsupported"], serde_json::json!([]));
    assert_eq!(claude["parts"].as_array().unwrap().len(), 4);
    assert_eq!(
        claude["parts"][3],
        serde_json::json!({ "part": "permissions", "state": "skipped", "path": "/.claude/settings.json" })
    );
    // A part that the command wrote carries the state it found and the action;
    // a part it left as it found it carries no action.
    assert_eq!(
        claude["parts"][1],
        serde_json::json!({ "part": "skills", "state": "absent", "action": "created", "path": "/.claude/skills" })
    );
    let keys: Vec<_> = claude["parts"][1]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(keys, ["part", "state", "action", "path"]);
    let keys: Vec<_> = claude["parts"][3]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(keys, ["part", "state", "path"]);
    let st = serde_json::to_value(status(&tree).unwrap()).unwrap();
    assert_eq!(st["harnesses"][0]["parts"][1]["state"], "current");
}

// sokf-tests: CLI-3_AC-22
// sokf-tests: CLI-3_AC-23
// sokf-tests: CLI-3_AC-24
// sokf-tests: CLI-9_AC-1
// sokf-tests: CLI-9_AC-2
#[test]
fn every_refusal_leaves_the_tree_byte_identical() {
    let tree = TempTree::empty("harness-refusals");
    opt_in(&tree);
    tree.write("CLAUDE.md", "# Mine\n");
    let pristine = tree.files();
    let refused = |tree: &TempTree, force: Option<&[&str]>, name: &str| {
        let opts = HarnessOptions::new([name], HarnessScope::Project)
            .force(force.unwrap_or_default().iter().copied());
        let e = harness_install(&HarnessDirs::repo(tree.root()), &opts).unwrap_err();
        assert!(matches!(e, Error::Harness(_)), "{e}");
        assert_eq!(tree.files(), pristine, "{e}");
        e.to_string()
    };
    assert_eq!(
        refused(&tree, None, "cline"),
        "harness: no harness named `cline`"
    );
    assert_eq!(
        refused(&tree, Some(&["hooks", "mcp"]), "claude"),
        "harness: harness `claude` has no part named `mcp`"
    );
    // After the snapshot, the test writes each file that causes one of the
    // following refusals, and removes the file after it checks its refusal.
    tree.write(".claude/settings.json", "{ \"permissions\": [ }");
    let pristine_with = tree.files();
    let e = install(&tree, None, false).unwrap_err();
    assert!(
        e.to_string()
            .contains("/.claude/settings.json: does not parse"),
        "{e}"
    );
    assert_eq!(tree.files(), pristine_with);
    std::fs::remove_file(tree.dir().join(".claude/settings.json")).unwrap();
    tree.write(".sokf/harness.toml", "[claude\n");
    let e = install(&tree, None, false).unwrap_err();
    assert!(
        e.to_string().starts_with("harness: /.sokf/harness.toml: "),
        "{e}"
    );
    std::fs::remove_file(tree.dir().join(".sokf/harness.toml")).unwrap();
    tree.write(".sokf/config.toml", "[harness.claude]\nwith = 1\n");
    let e = install(&tree, None, false).unwrap_err();
    assert!(
        e.to_string().starts_with("harness: /.sokf/config.toml: "),
        "{e}"
    );
    assert!(status(&tree).is_err());
    std::fs::remove_file(tree.dir().join(".sokf/config.toml")).unwrap();
    assert_eq!(tree.files(), pristine);
}

// sokf-tests: CONF-4_AC-2
// sokf-tests: CLI-3_AC-36
// sokf-tests: CLI-3_AC-42
#[test]
fn a_configured_command_is_what_the_hook_runs_and_what_the_tool_knows() {
    let tree = TempTree::empty("harness-command");
    tree.write(
        ".sokf/config.toml",
        &format!(
            "[knowledge]\nroots = [\"/knowledge\"]\n\n{}command = '\"$DIR/target/debug/sokf\"'\n",
            with_hooks()
        ),
    );
    install(&tree, None, false).unwrap();
    let settings = tree.read(".claude/settings.json");
    assert!(
        settings.contains("\\\"$DIR/target/debug/sokf\\\" harness hook claude prompt-submit"),
        "{settings}"
    );
    assert!(!settings.contains("\"sokf harness hook"), "{settings}");
    // The hook that install writes is the hook that status expects, so the
    // hooks part is current, not stale.
    let st = status(&tree).unwrap();
    assert_eq!(
        states(&st)
            .iter()
            .find(|(p, _, _)| *p == "hooks")
            .map(|(_, s, _)| *s),
        Some("current")
    );
    // A second install leaves one hook group, not two, because the tool
    // recognises its hook by the command it runs.
    install(&tree, None, false).unwrap();
    let again = tree.read(".claude/settings.json");
    assert_eq!(
        again.matches("harness hook claude prompt-submit").count(),
        1,
        "{again}"
    );
}

// sokf-tests: CLI-3_AC-4
// sokf-tests: CLI-3_AC-42
// sokf-tests: CLI-3_AC-49
#[test]
fn a_hook_line_edited_to_another_binary_is_edited_not_absent() {
    let tree = TempTree::empty("harness-edited-hook");
    opt_in(&tree);
    install(&tree, None, false).unwrap();
    let settings = tree.read(".claude/settings.json");
    tree.write(
        ".claude/settings.json",
        &settings.replace("sokf harness hook", "./other/sokf harness hook"),
    );
    let st = status(&tree).unwrap();
    assert_eq!(
        states(&st)
            .iter()
            .find(|(p, _, _)| *p == "hooks")
            .map(|(_, s, _)| *s),
        Some("edited")
    );
    // Install leaves an edited part alone and doesn't add a second hook.
    install(&tree, None, false).unwrap();
    let after = tree.read(".claude/settings.json");
    assert_eq!(
        after.matches("harness hook claude prompt-submit").count(),
        1,
        "{after}"
    );
    assert!(after.contains("./other/sokf harness hook"), "{after}");
}

/// `state_of` returns the state of the part `part` of Claude Code.
fn state_of<'a>(report: &'a HarnessReport, part: &str) -> Option<&'a str> {
    states(report)
        .into_iter()
        .find(|(p, _, _)| *p == part)
        .map(|(_, s, _)| s)
}

// sokf-tests: CLI-3_AC-16
#[test]
fn a_formatter_may_relay_out_the_block_without_making_it_edited() {
    let tree = TempTree::empty("harness-formatted");
    install(&tree, None, false).unwrap();
    let before = tree.read("CLAUDE.md");
    // Lay the block out again as a Markdown formatter can: put blank lines
    // around the markers and before the list, and wrap each line, the items'
    // lines under their text, without changing a word or the Markdown.
    let block_start = before.find("<!-- sokf:harness -->").unwrap();
    let block_end = before.find("<!-- /sokf:harness -->").unwrap();
    let block = &before[block_start + "<!-- sokf:harness -->".len()..block_end];
    let wrap = |line: &str, indent: &str| {
        line.split_whitespace()
            .collect::<Vec<_>>()
            .chunks(8)
            .map(|c| c.join(" "))
            .collect::<Vec<_>>()
            .join(&format!("\n{indent}"))
    };
    let relaid: Vec<String> = block
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| match l.strip_prefix("- ") {
            Some(item) => format!("- {}", wrap(item, "  ")),
            None => format!("{}\n", wrap(l, "")),
        })
        .collect();
    tree.write(
        "CLAUDE.md",
        &format!(
            "<!-- sokf:harness -->\n\n{}\n\n<!-- /sokf:harness -->\n",
            relaid.join("\n")
        ),
    );
    let st = status(&tree).unwrap();
    assert_eq!(
        state_of(&st, "instructions"),
        Some("current"),
        "a block laid out again is the same block: {}",
        tree.read("CLAUDE.md")
    );
    // The same words in one paragraph are another block, and an edit.
    let words = block.split_whitespace().collect::<Vec<_>>().join(" ");
    tree.write(
        "CLAUDE.md",
        &format!("<!-- sokf:harness -->\n\n{words}\n<!-- /sokf:harness -->\n"),
    );
    assert_eq!(
        state_of(&status(&tree).unwrap(), "instructions"),
        Some("edited")
    );
    // A change to the words of the block is still an edit.
    tree.write(
        "CLAUDE.md",
        "<!-- sokf:harness -->\n\nSomething else entirely.\n<!-- /sokf:harness -->\n",
    );
    assert_eq!(
        state_of(&status(&tree).unwrap(), "instructions"),
        Some("edited")
    );
}

// sokf-tests: CLI-3_AC-13
// sokf-tests: CLI-3_AC-56
#[test]
fn a_part_that_oxfmt_re_laid_is_current() {
    let tree = TempTree::empty("harness-oxfmt");
    opt_in(&tree);
    tree.write("AGENTS.md", "# Demo\n");
    install(&tree, None, false).unwrap();
    // The changes that oxfmt 0.77.0 makes to the files that sokf wrote: a
    // blank line before a list and before the closing marker, a blank line
    // before the item that follows an item with a second paragraph, in a list
    // that is loose already, and a short array on one line.
    let written = tree.read("AGENTS.md");
    let agents = written
        .replace("stores:\n- ", "stores:\n\n- ")
        .replace(".\n<!-- /sokf:harness -->", ".\n\n<!-- /sokf:harness -->");
    assert_eq!(agents.len(), written.len() + 2, "{written}");
    tree.write("AGENTS.md", &agents);
    let skill = tree.read(".claude/skills/sokf/SKILL.md");
    let second = skill
        .find("\n\n  ")
        .expect("an item with a second paragraph");
    let next = second + skill[second..].find("\n- ").expect("an item after it");
    let relaid = format!("{}\n{}", &skill[..next], &skill[next..]);
    tree.write(".claude/skills/sokf/SKILL.md", &relaid);
    let settings = tree.read(".claude/settings.json");
    let one_line = settings.replace("[\n      \"Bash(sokf *)\"\n    ]", "[\"Bash(sokf *)\"]");
    assert_ne!(one_line, settings);
    tree.write(".claude/settings.json", &one_line);
    let st = status(&tree).unwrap();
    assert!(verbs(&st).iter().all(|v| *v == "current"), "{st:?}");
    assert!(st.warnings.is_empty(), "{st:?}");
    // An install writes nothing, and the files keep their layout.
    let before = tree.files();
    let again = install(&tree, None, false).unwrap();
    assert!(verbs(&again).iter().all(|v| *v == "current"), "{again:?}");
    assert_eq!(tree.files(), before);
    // A file of the skill that changes its meaning is an edit.
    let other = format!("{relaid}\nA paragraph of the project's.\n");
    tree.write(".claude/skills/sokf/SKILL.md", &other);
    assert_eq!(state_of(&status(&tree).unwrap(), "skills"), Some("edited"));
}

// sokf-tests: CLI-3_AC-113
// sokf-tests: CLI-4_AC-118
#[test]
fn a_record_of_a_later_sokf_names_no_forced_install() {
    let tree = TempTree::fixture("clean");
    install(&tree, None, false).unwrap();
    // Another skill, with a record of it that holds a named thing of a later
    // release of same-same, as a later sokf would write it.
    let other = "---\nname: sokf\n---\nlater\n";
    tree.write(".claude/skills/sokf/SKILL.md", other);
    let value = common::adapter::recorded_value(agent_harness_adapter_core::Part::files(
        "other-skills",
        ".claude/skills",
        vec![("sokf/SKILL.md".into(), other.into())],
    ));
    let later = format!(
        "{}:zz-later=0011223344556677",
        value["sokf/SKILL.md"].as_str().unwrap()
    );
    let mut record: toml::Table = tree.read(".sokf/harness.toml").parse().unwrap();
    let mut skills = toml::Table::new();
    skills.insert("sokf/SKILL.md".into(), later.into());
    record["claude"]
        .as_table_mut()
        .unwrap()
        .insert("skills".into(), skills.into());
    tree.write(".sokf/harness.toml", &record.to_string());
    let st = status(&tree).unwrap();
    assert_eq!(state_of(&st, "skills"), Some("edited"));
    assert_eq!(
        st.warnings,
        ["claude: skills in /.claude/skills may not be an edit, as a later sokf recorded it"]
    );
    let report = tree.validate_all(false, sokf_core::CacheMode::Off);
    let messages: Vec<&str> = report
        .warnings
        .iter()
        .filter(|f| f.criterion == "CLI-4_AC-118")
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "the skills part of the claude harness may not be an edit, as a later sokf recorded it; check `sokf harness diff claude`"
        ]
    );
}

// sokf-tests: CLI-3_AC-2
#[test]
fn a_stale_part_is_rewritten_and_recorded_again() {
    let tree = TempTree::empty("harness-stale");
    opt_in(&tree);
    install(&tree, None, false).unwrap();
    // Simulate an older sokf: a different skill text, with its hash in the
    // record.
    let old = "---\nname: sokf\n---\nold\n";
    tree.write(".claude/skills/sokf/SKILL.md", old);
    // The adapter's raw part can't be named `skills`, but its value covers the
    // content only.
    let old_value = common::adapter::recorded_value(agent_harness_adapter_core::Part::files(
        "old-skills",
        ".claude/skills",
        vec![("sokf/SKILL.md".into(), old.into())],
    ));
    let mut record: toml::Table = tree.read(".sokf/harness.toml").parse().unwrap();
    record["claude"]
        .as_table_mut()
        .unwrap()
        .insert("skills".into(), old_value.clone());
    tree.write(".sokf/harness.toml", &record.to_string());
    let st = status(&tree).unwrap();
    assert_eq!(parts(&st)[1].verb(), "stale");
    // Install rewrites a stale part with the embedded content without
    // `--force`.
    let out = install(&tree, None, false).unwrap();
    assert_eq!(parts(&out)[1].verb(), "rewrote");
    assert!(
        tree.read(".claude/skills/sokf/SKILL.md")
            .contains("# SOKF v0.7")
    );
    let record: toml::Table = tree.read(".sokf/harness.toml").parse().unwrap();
    assert_ne!(record["claude"]["skills"], old_value);
    let st = status(&tree).unwrap();
    assert!(verbs(&st).iter().all(|v| *v == "current"), "{st:?}");
}

/// `all_files` returns every file under the tree with its text, including
/// the record and the config under `/.sokf`.
fn all_files(tree: &TempTree) -> Vec<(String, String)> {
    let mut out = tree.files();
    for rel in [".sokf/harness.toml", ".sokf/config.toml"] {
        if let Ok(text) = std::fs::read_to_string(tree.dir().join(rel)) {
            out.push((rel.to_string(), text));
        }
    }
    out
}

// sokf-tests: CLI-3_AC-10
#[test]
fn status_writes_no_file_whatever_the_states() {
    // With nothing installed, status creates no file, not even the record: the
    // configuration that opts in to the hooks stays the only file.
    let tree = TempTree::empty("harness-status-still");
    opt_in(&tree);
    let st = status(&tree).unwrap();
    assert!(verbs(&st).iter().all(|v| *v == "absent"), "{st:?}");
    assert_eq!(
        tree.files(),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        all_files(&tree),
        [(".sokf/config.toml".into(), with_hooks())]
    );
    assert_eq!(
        std::fs::read_dir(tree.dir().join(".sokf")).unwrap().count(),
        1
    );
    // With one part skipped, one part edited, and the rest current, status
    // leaves the tree as it is.
    install(&tree, Some(&["hooks"]), false).unwrap();
    tree.write(".claude/skills/sokf/SKILL.md", "mine\n");
    let before = all_files(&tree);
    let st = status(&tree).unwrap();
    let states: Vec<_> = verbs(&st);
    assert_eq!(states, ["current", "edited", "skipped", "current"]);
    assert_eq!(all_files(&tree), before);
    assert!(!tree.dir().join(".sokf/cache").exists());
}

// sokf-tests: CLI-3_AC-21
#[test]
fn install_and_status_are_blind_to_the_knowledge_and_the_cache() {
    // Errors in the knowledge don't stop the commands, and the commands
    // neither create nor change the cache.
    let tree = TempTree::fixture("broken");
    opt_in(&tree);
    // The cache file holds text that isn't a cache. A command that read the
    // file would see garbage.
    tree.write(".sokf/cache/store", "not a cache\n");
    let before = tree.cache_files();
    let out = install(&tree, None, false).unwrap();
    assert!(
        verbs(&out)
            .iter()
            .all(|v| *v == "created" || *v == "updated"),
        "{out:?}"
    );
    status(&tree).unwrap();
    assert_eq!(tree.cache_files(), before);
}

// sokf-tests: CLI-3_AC-8
// sokf-tests: CONF-4_AC-5
#[test]
fn an_install_without_a_configuration_writes_none_and_skips_the_hook() {
    let tree = TempTree::empty("harness-no-config");
    let out = install(&tree, None, false).unwrap();
    assert_eq!(out.harnesses[0].parts[2].verb(), "skipped");
    assert!(!tree.dir().join(".sokf/config.toml").exists());
    assert!(!tree.read(".claude/settings.json").contains("harness hook"));
}
