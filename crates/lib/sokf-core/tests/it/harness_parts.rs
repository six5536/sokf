//! These tests check the part kinds, the states, the hashes, the record, and
//! the declined parts of `harness install`, which agent-harness-adapter carries
//! out for sokf. They reach the adapter through `harness install` and `harness
//! status`, through its region markers, and through an adapter tool with the parts
//! that a test chooses.

use std::fs;

use agent_harness_adapter_core::{Part, State, harness::Markers};
use common::{
    TempTree,
    adapter::{install_parts, recorded_value, status_parts},
};
use sokf_core::{
    Error, HarnessDirs, HarnessOptions, HarnessReport, HarnessScope, PartResult, harness_install,
    harness_status,
};

use crate::common;

const BLOCK: &str = "Read the skill.\n";

fn markers() -> Markers {
    Markers::new("sokf")
}

/// `install` installs Claude Code. When `without` names parts, it first writes the
/// configuration so that it opts in to the hooks and declines those parts, as the user does;
/// with `force`, it passes `--force all`.
fn install(tree: &TempTree, without: &[&str], force: bool) -> sokf_core::Result<HarnessReport> {
    if !without.is_empty() {
        let list: Vec<String> = without.iter().map(|p| format!("\"{p}\"")).collect();
        tree.write(
            ".sokf/config.toml",
            &format!(
                "{}without = [{}]\n",
                common::with_hooks(&["claude"]),
                list.join(", ")
            ),
        );
    }
    let force: &[&str] = if force { &["all"] } else { &[] };
    harness_install(
        &HarnessDirs::repo(tree.root()),
        &HarnessOptions::new(["claude"], HarnessScope::Project).force(force.iter().copied()),
    )
}

fn status(tree: &TempTree) -> HarnessReport {
    harness_status(
        &HarnessDirs::repo(tree.root()),
        &["claude".to_string()],
        HarnessScope::Project,
    )
    .unwrap()
}

/// `parts` returns the parts of Claude Code, the only harness named.
fn parts(report: &HarnessReport) -> &[PartResult] {
    &report.harnesses[0].parts
}

/// `install_only` installs the part `part` of the `claude` profile, declining
/// the others.
fn install_only(tree: &TempTree, part: &str, force: bool) -> sokf_core::Result<HarnessReport> {
    let without: Vec<&str> = ["instructions", "skills", "hooks", "permissions"]
        .into_iter()
        .filter(|p| *p != part)
        .collect();
    install(tree, &without, force)
}

// sokf-tests: CLI_P-17
#[test]
fn a_new_file_is_the_block_alone() {
    let out = markers().render(None, BLOCK).unwrap();
    assert_eq!(
        out,
        "<!-- sokf:harness -->\n\nRead the skill.\n<!-- /sokf:harness -->\n"
    );
    assert_eq!(markers().find(&out).as_deref(), Some(BLOCK));
    assert_eq!(markers().render(Some(&out), BLOCK), None);
}

// sokf-tests: CLI-3_AC-40
#[test]
fn markers_absent_appends_after_one_blank_line() {
    let framed = "<!-- sokf:harness -->\n\nRead the skill.\n<!-- /sokf:harness -->\n";
    let out = markers().render(Some("# Title\n\nText."), BLOCK).unwrap();
    assert_eq!(out, format!("# Title\n\nText.\n\n{framed}"));
    // An empty file receives the framed block alone, and a file that ends in
    // a newline receives exactly one blank line before the framed block.
    assert_eq!(markers().render(Some(""), BLOCK).unwrap(), framed);
    assert_eq!(
        markers().render(Some("x\n"), BLOCK).unwrap(),
        format!("x\n\n{framed}")
    );
}

// sokf-tests: CLI-3_AC-15
// sokf-tests: CLI_P-17
#[test]
fn markers_present_rewrites_between_them_only() {
    let before =
        "# Title\n\n<!-- sokf:harness -->\nold\n<!-- /sokf:harness -->\n\n## After\n\nkept\n";
    let out = markers().render(Some(before), BLOCK).unwrap();
    assert_eq!(
        out,
        "# Title\n\n<!-- sokf:harness -->\n\nRead the skill.\n<!-- /sokf:harness -->\n\n## After\n\nkept\n"
    );
    assert_eq!(markers().render(Some(&out), BLOCK), None);
    // The markers start the file, and the block between them is empty.
    let before = "<!-- sokf:harness -->\n<!-- /sokf:harness -->\ntail\n";
    assert_eq!(markers().find(before).as_deref(), Some(""));
    assert_eq!(
        markers().render(Some(before), BLOCK).unwrap(),
        "<!-- sokf:harness -->\n\nRead the skill.\n<!-- /sokf:harness -->\ntail\n"
    );
    // A stray opening marker, whose closing marker the user deleted, is the
    // user's text: the block goes after it and the text stays.
    let stray = "<!-- sokf:harness -->\nmine\n";
    assert_eq!(markers().find(stray), None);
    let out = markers().render(Some(stray), BLOCK).unwrap();
    assert!(out.starts_with(stray), "{out}");
    assert_eq!(markers().find(&out).as_deref(), Some(BLOCK));
}

// sokf-tests: CLI-12_AC-2
#[test]
fn keeps_crlf_line_endings() {
    let before = "a\r\n\r\n<!-- sokf:harness -->\r\nold\r\n<!-- /sokf:harness -->\r\n";
    let out = markers().render(Some(before), BLOCK).unwrap();
    assert_eq!(
        out,
        "a\r\n\r\n<!-- sokf:harness -->\r\n\r\nRead the skill.\r\n<!-- /sokf:harness -->\r\n"
    );
    assert_eq!(markers().find(&out).as_deref(), Some(BLOCK));
    assert_eq!(markers().render(Some(&out), BLOCK), None);
    let out = markers().render(Some("a\r\n"), BLOCK).unwrap();
    assert!(out.contains("\r\n<!-- sokf:harness -->\r\n"), "{out:?}");
}

#[test]
fn a_closing_marker_before_the_opening_one_is_no_block() {
    assert_eq!(
        markers().find("<!-- /sokf:harness -->\n<!-- sokf:harness -->\n"),
        None
    );
    assert_eq!(markers().find("no markers"), None);
}

// sokf-tests: CLI-3_AC-39
#[test]
fn a_file_part_is_written_with_lf() {
    let tree = TempTree::empty("parts-lf");
    install_parts(
        tree.dir(),
        vec![Part::files(
            "files",
            ".claude/skills/x",
            vec![("a.md".into(), "a\r\nb\r\n".into())],
        )],
    );
    assert_eq!(tree.read(".claude/skills/x/a.md"), "a\nb\n");
}

// sokf-tests: CLI-3_AC-18
// sokf-tests: CLI-3_AC-44
#[test]
fn the_indent_of_the_settings_file_is_kept_or_two_spaces() {
    for (indent, existing) in [
        ("    ", "{\n    \"a\": 1\n}\n"),
        ("\t", "{\n\t\"a\": 1\n}\n"),
        ("  ", "{}\n"),
    ] {
        let tree = TempTree::empty("parts-indent");
        tree.write(".claude/settings.json", existing);
        install_only(&tree, "permissions", false).unwrap();
        let after = tree.read(".claude/settings.json");
        assert!(after.starts_with(&format!("{{\n{indent}\"")), "{after:?}");
    }
}

// sokf-tests: CLI-3_AC-41
// sokf-tests: CLI_P-18
#[test]
fn permissions_are_added_once_and_found() {
    let tree = TempTree::empty("parts-permissions");
    install_only(&tree, "permissions", false).unwrap();
    assert_eq!(
        tree.read(".claude/settings.json"),
        "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(sokf *)\"\n    ]\n  }\n}\n"
    );
    assert_eq!(parts(&status(&tree))[3].state, State::Current);
    // When the permission is there already, nothing changes, and the other
    // entries stay.
    let existing = "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(npm *)\",\n      \"Bash(sokf *)\"\n    ]\n  }\n}\n";
    tree.write(".claude/settings.json", existing);
    install_only(&tree, "permissions", false).unwrap();
    assert_eq!(tree.read(".claude/settings.json"), existing);
    // Without the permission, the part is absent.
    tree.write(
        ".claude/settings.json",
        "{\"permissions\":{\"allow\":[\"Bash(npm *)\"]}}",
    );
    assert_eq!(parts(&status(&tree))[3].state, State::Absent);
}

// sokf-tests: CLI-3_AC-18
// sokf-tests: CLI-3_AC-42
// sokf-tests: CLI-3_AC-43
// sokf-tests: CLI_P-18
#[test]
fn the_hook_replaces_the_tools_entry_and_keeps_others() {
    let tree = TempTree::empty("parts-hooks");
    let existing = r#"{
    "other": true,
    "hooks": {
        "PreToolUse": [],
        "UserPromptSubmit": [
            { "hooks": [ { "type": "command", "command": "echo hi" } ] },
            { "hooks": [ { "type": "command", "command": "sokf harness hook claude old" } ] }
        ]
    },
    "z": [1, 2]
}"#;
    tree.write(".claude/settings.json", existing);
    // The tool's hook command differs from the embedded one, and no hash is
    // recorded, so the part is edited and only `--force` replaces it.
    let out = install_only(&tree, "hooks", false).unwrap();
    assert_eq!(parts(&out)[2].state, State::Edited);
    assert_eq!(tree.read(".claude/settings.json"), existing);
    install_only(&tree, "hooks", true).unwrap();
    let out = tree.read(".claude/settings.json");
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    // The key order and the other entries stay, and the indent stays at four
    // spaces. The file had no trailing newline, so none is added.
    let keys: Vec<_> = doc.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["other", "hooks", "z"]);
    assert_eq!(doc["hooks"]["PreToolUse"], serde_json::json!([]));
    assert_eq!(
        doc["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
        "echo hi"
    );
    assert_eq!(
        doc["hooks"]["UserPromptSubmit"][1]["hooks"][0]["command"],
        "sokf harness hook claude prompt-submit"
    );
    assert_eq!(
        doc["hooks"]["UserPromptSubmit"].as_array().unwrap().len(),
        2
    );
    assert!(out.starts_with("{\n    \"other\": true,"), "{out}");
    assert!(!out.ends_with('\n'));
    assert_eq!(parts(&status(&tree))[2].state, State::Current);
    install_only(&tree, "hooks", false).unwrap();
    assert_eq!(tree.read(".claude/settings.json"), out);
    // The tool owns its entries, not the group: the user's entry in the same
    // group stays, and the part stays current.
    let shared = r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"echo mine"},{"type":"command","command":"sokf harness hook claude prompt-submit"}]}]}}
"#;
    tree.write(".claude/settings.json", shared);
    assert_eq!(parts(&status(&tree))[2].state, State::Current);
    // When the file has no hooks, the merge creates the objects and arrays
    // that hold them.
    tree.write(".claude/settings.json", "{}\n");
    install_only(&tree, "hooks", false).unwrap();
    let out = tree.read(".claude/settings.json");
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        doc["hooks"]["UserPromptSubmit"].as_array().unwrap().len(),
        1
    );
    assert!(out.ends_with('\n'));
}

// sokf-tests: CLI-9_AC-2
#[test]
fn a_settings_file_that_does_not_parse_is_refused_and_named() {
    let tree = TempTree::empty("parts-refused");
    for (text, part, why) in [
        (
            "{ nope",
            "hooks",
            "harness: /.claude/settings.json: does not parse",
        ),
        (
            "[]",
            "hooks",
            "harness: /.claude/settings.json: is not a JSON object",
        ),
        ("{\"permissions\": []}", "permissions", "not an object"),
        (
            "{\"permissions\": {\"allow\": {}}}",
            "permissions",
            "not an array",
        ),
    ] {
        tree.write(".claude/settings.json", text);
        let e = install_only(&tree, part, false).unwrap_err();
        assert!(matches!(e, Error::Harness(_)), "{e}");
        assert!(e.to_string().contains(why), "{e}");
        assert_eq!(tree.read(".claude/settings.json"), text);
    }
}

// sokf-tests: CLI-3_AC-62
// sokf-tests: CLI-9_AC-2
#[test]
fn a_declined_part_is_never_read() {
    let tree = TempTree::empty("parts-declined-unread");
    tree.write(".claude/settings.json", "{ nope");
    let out = install(&tree, &["hooks", "permissions"], false).unwrap();
    assert_eq!(parts(&out)[2].state, State::Skipped);
    assert_eq!(parts(&out)[3].state, State::Skipped);
    assert_eq!(tree.read(".claude/settings.json"), "{ nope");
    status(&tree);
    // Once one of its parts is no longer declined, the file is read and
    // refused.
    assert!(install(&tree, &["hooks"], false).is_err());
    assert_eq!(tree.read(".claude/settings.json"), "{ nope");
}

// sokf-tests: CLI-3_AC-24
// sokf-tests: CLI-3_AC-52
// sokf-tests: CLI-3_AC-58
// sokf-tests: CLI-3_AC-59
#[test]
fn reads_and_renders_the_record() {
    let tree = TempTree::empty("parts-record");
    // Another harness's table stays, and the tables and keys are in name
    // order.
    tree.write(".sokf/harness.toml", "[aider]\nx = \"kept\"\n");
    tree.write(".sokf/config.toml", &common::with_hooks(&["claude"]));
    install(&tree, &[], false).unwrap();
    let record = tree.read(".sokf/harness.toml");
    let header = "# Written by sokf harness install. Do not edit it: sokf compares the parts with these hashes to tell a part you edited from a stale one.\n";
    assert!(
        record.starts_with(&format!(
            "{header}\n[aider]\nx = \"kept\"\n\n[claude]\nhooks = \""
        )),
        "{record}"
    );
    let keys: Vec<_> = record
        .lines()
        .skip_while(|l| *l != "[claude]")
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(|l| l.split(" = ").next().unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "hooks",
            "instructions",
            "permissions",
            "[claude.skills]",
            "\"sokf/SKILL.md\""
        ]
    );
    // The record reads back: a second install writes nothing.
    let before = tree.files();
    install(&tree, &[], false).unwrap();
    assert_eq!(tree.read(".sokf/harness.toml"), record);
    assert_eq!(tree.files(), before);
    // A harness with every part declined keeps an empty table: it is
    // installed, and stays in the set of harnesses later runs consider.
    tree.write(".sokf/harness.toml", "");
    install(
        &tree,
        &["instructions", "skills", "hooks", "permissions"],
        false,
    )
    .unwrap();
    assert_eq!(
        tree.read(".sokf/harness.toml"),
        format!("{header}\n[claude]\n")
    );
    // A record that does not parse, or holds anything but tables of strings,
    // is an error that names it.
    for bad in ["[claude]\nskills = 1\n", "claude = 1\n", "[claude\n"] {
        tree.write(".sokf/harness.toml", bad);
        let e = harness_status(
            &HarnessDirs::repo(tree.root()),
            &["claude".to_string()],
            HarnessScope::Project,
        )
        .unwrap_err();
        assert!(matches!(e, Error::Harness(_)), "{e}");
        assert!(
            e.to_string().starts_with("harness: /.sokf/harness.toml: "),
            "{e}"
        );
    }
}

// sokf-tests: CLI-3_AC-8
// sokf-tests: CONF-4_AC-1
// sokf-tests: CONF-5_AC-1
#[test]
fn no_install_changes_the_config() {
    let tree = TempTree::empty("parts-declined");
    let path = tree.dir().join(".sokf/config.toml");
    // Each config declines the hooks, leaves them off by default, or opts in to them and
    // declines nothing; the install reads it, skips what it declines, and leaves its bytes
    // and its time as they were.
    for (text, declined) in [
        (
            "[harness.claude]\nwith = [\"hooks\"]\nwithout = [\"hooks\"]\n",
            true,
        ),
        (
            "# The stores.\n[knowledge]\nroots = [\"/knowledge\"] # kept\n",
            true,
        ),
        (
            "[harness.claude] # mine\nwith = [\"hooks\"]\nwithout = [\"hooks\"]\n",
            true,
        ),
        (
            "harness = { claude = { with = [\"hooks\"], without = [\"hooks\"] } }\n",
            true,
        ),
        (
            "harness = { claude = { with = [\"hooks\"], without = [] } }\n",
            false,
        ),
        ("[harness.claude] # mine\nwith = [\"hooks\"]\n", false),
    ] {
        tree.write(".sokf/config.toml", text);
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        let out = install(&tree, &[], false).unwrap();
        assert_eq!(parts(&out)[2].state == State::Skipped, declined, "{text}");
        assert_eq!(tree.read(".sokf/config.toml"), text);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        install(&tree, &[], true).unwrap();
        assert_eq!(tree.read(".sokf/config.toml"), text);
    }
    // A config that doesn't parse is an error that names it.
    tree.write(".sokf/config.toml", "[harness\n");
    let e = install(&tree, &[], false).unwrap_err();
    assert!(matches!(e, Error::Harness(_)), "{e}");
    assert!(
        e.to_string().starts_with("harness: /.sokf/config.toml: "),
        "{e}"
    );
}

// sokf-tests: CLI-3_AC-53
// sokf-tests: CLI-3_AC-56
// sokf-tests: CLI-3_AC-13
#[test]
fn the_record_holds_a_fingerprint_of_the_meaning() {
    let block = |b: &str| recorded_value(Part::region("block", "CLAUDE.md", b));
    // Line endings and a re-wrapped paragraph don't change a block's
    // fingerprint; another word does.
    assert_eq!(block("x\r\ny\r\n"), block("x\ny\n"));
    assert_eq!(block("one two\nthree\n"), block("one\ntwo three\n"));
    assert_ne!(block("x\n"), block("y\n"));
    assert!(block("x\n").is_str());
    // A part of files records each file's fingerprint by its path in the part,
    // so a rename changes the record.
    let files = |name: &str, text: &str| {
        recorded_value(Part::files(
            "files",
            ".claude/skills/x",
            vec![(name.into(), text.into())],
        ))
    };
    assert_ne!(files("A.md", "t\n"), files("B.md", "t\n"));
    assert!(files("A.md", "t\n")["A.md"].is_str());
    // Each file is compared in the format that its name gives: a JSON file
    // whose keys a formatter re-laid and reordered is current, and a text of
    // no known format is compared by its text.
    let part = |name: &str, text: &str| {
        vec![Part::files(
            "files",
            ".claude/skills/x",
            vec![(name.into(), text.into())],
        )]
    };
    let state = |tree: &TempTree, name: &str, text: &str| {
        status_parts(tree.dir(), part(name, text)).harnesses[0].parts[0].state
    };
    let tree = TempTree::empty("parts-meaning-of-files");
    install_parts(tree.dir(), part("a.json", "{\"a\": 1, \"b\": 2}\n"));
    tree.write(".claude/skills/x/a.json", "{\n  \"b\": 2,\n  \"a\": 1\n}\n");
    assert_eq!(
        state(&tree, "a.json", "{\"a\": 1, \"b\": 2}\n"),
        State::Current
    );
    // Line endings are no difference.
    tree.write(
        ".claude/skills/x/a.json",
        "{\r\n  \"a\": 1,\r\n  \"b\": 2\r\n}\r\n",
    );
    assert_eq!(
        state(&tree, "a.json", "{\"a\": 1, \"b\": 2}\n"),
        State::Current
    );
    let tree = TempTree::empty("parts-text-of-files");
    install_parts(tree.dir(), part("a.txt", "{\"a\": 1, \"b\": 2}\n"));
    tree.write(".claude/skills/x/a.txt", "{\n  \"b\": 2,\n  \"a\": 1\n}\n");
    assert_eq!(
        state(&tree, "a.txt", "{\"a\": 1, \"b\": 2}\n"),
        State::Edited
    );
}

// sokf-tests: CLI-3_AC-45
// sokf-tests: CLI-3_AC-46
// sokf-tests: CLI-3_AC-47
// sokf-tests: CLI-3_AC-48
// sokf-tests: CLI-3_AC-49
// sokf-tests: CLI-3_AC-12
// sokf-tests: CLI-3_AC-13
#[test]
fn each_row_of_the_state_table() {
    let part = |b: &str| vec![Part::region("block", "CLAUDE.md", b)];
    let state =
        |tree: &TempTree| status_parts(tree.dir(), part("new\n")).harnesses[0].parts[0].state;
    // A part that the config turns off is skipped, whatever the tree holds.
    let tree = TempTree::empty("parts-states");
    install_parts(tree.dir(), part("new\n"));
    tree.write(
        ".sokf/config.toml",
        "[harness.claude]\nwithout = [\"block\"]\n",
    );
    assert_eq!(state(&tree), State::Skipped);
    fs::remove_file(tree.dir().join(".sokf/config.toml")).unwrap();
    assert_eq!(state(&tree), State::Current);
    // The same block with other line endings is current.
    tree.write(
        "CLAUDE.md",
        "<!-- sokf:harness -->\r\n\r\nnew\r\n<!-- /sokf:harness -->\r\n",
    );
    assert_eq!(state(&tree), State::Current);
    // No block is absent.
    let tree = TempTree::empty("parts-states-absent");
    assert_eq!(state(&tree), State::Absent);
    // An older block with its hash recorded is stale.
    install_parts(tree.dir(), part("old\n"));
    assert_eq!(state(&tree), State::Stale);
    // Another block is edited, and so is the older block with no hash.
    tree.write(
        "CLAUDE.md",
        "<!-- sokf:harness -->\n\nother\n<!-- /sokf:harness -->\n",
    );
    assert_eq!(state(&tree), State::Edited);
    tree.write(
        "CLAUDE.md",
        "<!-- sokf:harness -->\n\nold\n<!-- /sokf:harness -->\n",
    );
    fs::remove_file(tree.dir().join(".sokf/harness.toml")).unwrap();
    assert_eq!(state(&tree), State::Edited);
    assert_eq!(State::Stale.as_str(), "stale");
}
