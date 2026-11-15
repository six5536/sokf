//! These tests check the properties of the files that connect the tool to an
//! agent harness: CLI_P-2, CLI_P-17 to CLI_P-20, and CLI-9_AC-1. The part
//! kinds and the states are agent-harness-adapter's, so the tests reach them
//! through `harness install` and `harness status`, through the adapter's region
//! markers, and through an adapter tool with the parts that a test chooses.

use agent_harness_adapter_core::{Part, State, harness::Markers};
use common::{
    TempTree,
    adapter::{install_parts, status_parts},
};
use proptest::prelude::*;
use serde::Serialize;
use sokf_core::{
    CacheMode, HarnessDirs, HarnessOptions, HarnessReport, HarnessScope, harness, harness_install,
    harness_status, report_text,
};

use crate::common;

/// `PARTS` lists the parts of Claude Code.
const PARTS: [&str; 4] = ["instructions", "skills", "hooks", "permissions"];

fn claude_opts() -> HarnessOptions {
    HarnessOptions::new(["claude"], HarnessScope::Project)
}

/// `decline` writes `/.sokf/config.toml` of `tree` so that it opts in to the hooks of
/// Claude Code, which are off by default, and declines `parts` of it, as the user does.
fn decline<S: AsRef<str>>(tree: &TempTree, parts: &[S]) {
    let list: Vec<String> = parts
        .iter()
        .map(|p| format!("\"{}\"", p.as_ref()))
        .collect();
    tree.write(
        ".sokf/config.toml",
        &format!(
            "{}without = [{}]\n",
            common::with_hooks(&["claude"]),
            list.join(", ")
        ),
    );
}

/// `verbs` returns the verb of each part of the first harness in `report`.
fn verbs(report: &HarnessReport) -> Vec<&str> {
    report.harnesses[0]
        .parts
        .iter()
        .map(sokf_core::PartResult::verb)
        .collect()
}

/// `json_text` serialises `doc` as JSON with the given indent and, when
/// `trailing_newline` is true, a trailing newline.
fn json_text(doc: &serde_json::Value, indent: &str, trailing_newline: bool) -> String {
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(
        &mut buf,
        serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes()),
    );
    doc.serialize(&mut ser).unwrap();
    let mut text = String::from_utf8(buf).unwrap();
    if trailing_newline {
        text.push('\n');
    }
    text
}

/// `indent_of` returns the indent of a file: the leading whitespace of its
/// first line that starts with a space or a tab, or two spaces when no line
/// does.
fn indent_of(text: &str) -> String {
    text.lines()
        .find(|l| l.starts_with(' ') || l.starts_with('\t'))
        .map(|l| l[..l.len() - l.trim_start().len()].to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "  ".to_string())
}

/// `arb_key` generates a top-level key that the tool doesn't write.
fn arb_key() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "model",
        "env",
        "theme",
        "cleanupPeriodDays",
        "statusLine",
    ])
    .prop_map(str::to_string)
}

fn arb_scalar() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        any::<bool>().prop_map(serde_json::Value::from),
        (0i64..1000).prop_map(serde_json::Value::from),
        "[a-z]{1,8}".prop_map(serde_json::Value::from),
    ]
}

/// `arb_settings_object` generates a `settings.json` object with other keys
/// before and after the tool's containers, an `allow` list with or without
/// the tool's entry, a `UserPromptSubmit` list with or without the tool's group and with
/// other groups, and a nested object whose key order must survive a merge.
fn arb_settings_object() -> impl Strategy<Value = serde_json::Value> {
    (
        prop::collection::vec((arb_key(), arb_scalar()), 0..3),
        prop::option::of(prop::collection::vec(
            prop::sample::select(vec!["Bash(npm run *)", "Bash(sokf *)", "Read(./src/**)"]),
            0..3,
        )),
        prop::option::of(prop::collection::vec(
            prop::sample::select(vec![
                "echo hi",
                "sokf harness hook claude prompt-submit",
                "make test",
            ]),
            0..3,
        )),
        prop::collection::vec((arb_key(), arb_scalar()), 0..3),
    )
        .prop_map(|(before, allow, stop, after)| {
            let mut map = serde_json::Map::new();
            for (k, v) in before {
                map.insert(k, v);
            }
            if let Some(allow) = allow {
                map.insert(
                    "permissions".into(),
                    serde_json::json!({ "allow": allow, "deny": ["Bash(rm *)"] }),
                );
            }
            if let Some(stop) = stop {
                let groups: Vec<_> = stop
                    .into_iter()
                    .map(|c| serde_json::json!({ "hooks": [{ "type": "command", "command": c }] }))
                    .collect();
                map.insert(
                    "hooks".into(),
                    serde_json::json!({ "PreToolUse": [], "UserPromptSubmit": groups }),
                );
            }
            for (k, v) in after {
                map.insert(k, v);
            }
            map.insert(
                "nested".into(),
                serde_json::json!({ "z": 1, "a": { "y": 2, "b": 3 } }),
            );
            serde_json::Value::Object(map)
        })
}

fn arb_indent() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["  ", "    ", "\t"]).prop_map(str::to_string)
}

/// `strip_entries` returns a copy of the value with the tool's entries
/// removed and nothing else changed.
fn strip_entries(v: &serde_json::Value) -> serde_json::Value {
    let mut v = v.clone();
    if let Some(allow) = v
        .get_mut("permissions")
        .and_then(|p| p.get_mut("allow"))
        .and_then(|a| a.as_array_mut())
    {
        allow.retain(|e| e != "Bash(sokf *)");
    }
    if let Some(stop) = v
        .get_mut("hooks")
        .and_then(|h| h.get_mut("UserPromptSubmit"))
        .and_then(|s| s.as_array_mut())
    {
        stop.retain(|g| {
            !g["hooks"].as_array().is_some_and(|h| {
                h.iter().any(|c| {
                    c["command"]
                        .as_str()
                        .is_some_and(|c| c.starts_with("sokf harness hook "))
                })
            })
        });
    }
    v
}

fn keys_in_order(v: &serde_json::Value) -> Vec<String> {
    v.as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

fn arb_text() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z #@-]{0,12}", 0..6).prop_map(|lines| lines.join("\n"))
}

fn arb_block() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-zA-Z .]{1,20}", 1..4)
        .prop_map(|lines| format!("{}\n", lines.join("\n")))
}

/// `install_only` installs the part `part` of the `claude` profile, declining
/// the others.
fn install_only(tree: &TempTree, part: &str) {
    let without: Vec<&str> = PARTS.iter().copied().filter(|p| *p != part).collect();
    decline(tree, &without);
    harness_install(&HarnessDirs::repo(tree.root()), &claude_opts()).unwrap();
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    // sokf-tests: CLI_P-17
    #[test]
    fn region_touches_only_its_block(text in arb_text(), block in arb_block(), crlf in any::<bool>()) {
        let markers = Markers::new("sokf");
        let text = if crlf { text.replace('\n', "\r\n") } else { text };
        let Some(after) = markers.render(Some(&text), &block) else {
            // `render` returns `None` when the text already holds the block.
            let found = markers.find(&text);
            prop_assert_eq!(found.as_deref(), Some(block.as_str()));
            return Ok(());
        };
        let lf = after.replace("\r\n", "\n");
        // The output contains exactly one block, and its content is the given
        // block.
        prop_assert_eq!(lf.matches("<!-- sokf:harness -->").count(), 1);
        let found = markers.find(&after);
        prop_assert_eq!(found.as_deref(), Some(block.as_str()));
        // Outside the markers the input survives: the output without its
        // block starts with the input without its old block.
        let strip = |t: &str| -> String {
            let lines: Vec<&str> = t.split('\n').collect();
            match (
                lines.iter().position(|l| *l == "<!-- sokf:harness -->"),
                lines.iter().position(|l| *l == "<!-- /sokf:harness -->"),
            ) {
                (Some(o), Some(c)) if c > o => [&lines[..o], &lines[c + 1..]].concat().join("\n"),
                _ => t.to_string(),
            }
        };
        let before_lf = text.replace("\r\n", "\n");
        let kept = strip(&lf);
        let original = strip(&before_lf);
        prop_assert!(kept.starts_with(original.trim_end_matches('\n')), "{kept:?} vs {original:?}");
        // A second render changes nothing, and the render keeps the line
        // endings of the input.
        prop_assert_eq!(markers.render(Some(&after), &block), None);
        prop_assert_eq!(after.contains("\r\n"), text.contains("\r\n"));
    }
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    // sokf-tests: CLI_P-18
    #[test]
    fn merge_keeps_the_rest(object in arb_settings_object(), indent in arb_indent(), newline in any::<bool>(), part in prop::sample::select(vec!["hooks", "permissions"])) {
        let tree = TempTree::empty("harness-p18");
        let text = json_text(&object, &indent, newline);
        tree.write(".claude/settings.json", &text);
        install_only(&tree, part);
        let after = tree.read(".claude/settings.json");
        if after == text {
            return Ok(());
        }
        let parsed: serde_json::Value = serde_json::from_str(&after).unwrap();
        let container = if part == "hooks" { "hooks" } else { "permissions" };
        // Everything except the tool's entries survives the merge. When the
        // input lacks the container, the merge creates it, and the container holds only
        // the tool's entries.
        let mut stripped = strip_entries(&parsed);
        if object.get(container).is_none() {
            let created = stripped.as_object_mut().unwrap().remove(container);
            let empty = if part == "hooks" { serde_json::json!({ "UserPromptSubmit": [] }) } else { serde_json::json!({ "allow": [] }) };
            prop_assert_eq!(created, Some(empty));
        }
        prop_assert_eq!(stripped, strip_entries(&object));
        // The keys keep the input's order, and the merge appends the tool's
        // container at the end when the input lacked it.
        let mut expected = keys_in_order(&object);
        if !expected.iter().any(|k| k == container) {
            expected.push(container.to_string());
        }
        prop_assert_eq!(keys_in_order(&parsed), expected);
        prop_assert_eq!(indent_of(&after), indent);
        prop_assert_eq!(after.ends_with('\n'), newline);
        // A second install writes nothing.
        install_only(&tree, part);
        prop_assert_eq!(tree.read(".claude/settings.json"), after);
    }

    // sokf-tests: CLI_P-19
    #[test]
    fn states_partition(found in prop::option::of(arb_block()), recorded_of_found in any::<bool>(), declined in any::<bool>()) {
        let tree = TempTree::empty("harness-p19");
        let part = |block: &str| vec![Part::region("block", "CLAUDE.md", block)];
        match &found {
            // The tool wrote the block and recorded its hash, as an older sokf
            // that embedded the block would have.
            Some(b) if recorded_of_found => {
                install_parts(tree.dir(), part(b));
            }
            other => {
                if let Some(b) = other {
                    tree.write("CLAUDE.md", &format!("<!-- sokf:harness -->\n\n{b}<!-- /sokf:harness -->\n"));
                }
                tree.write(".sokf/harness.toml", "[claude]\nblock = \"fnv1a64:0000000000000000\"\n");
            }
        }
        if declined {
            tree.write(".sokf/config.toml", "[harness.claude]\nwithout = [\"block\"]\n");
        }
        let embedded = "embedded\n";
        let state = status_parts(tree.dir(), part(embedded)).harnesses[0].parts[0].state;
        // A block is compared word by word (CLI-3_AC-16).
        let expected = if declined {
            State::Skipped
        } else if found.is_none() {
            State::Absent
        } else if found.as_deref().unwrap().split_whitespace().eq(embedded.split_whitespace()) {
            State::Current
        } else if recorded_of_found {
            State::Stale
        } else {
            State::Edited
        };
        prop_assert_eq!(state, expected);
        // After the tool writes the embedded content and records its hash,
        // the state is current.
        let _ = std::fs::remove_file(tree.dir().join(".sokf/config.toml"));
        install_parts(tree.dir(), part(embedded));
        prop_assert_eq!(status_parts(tree.dir(), part(embedded)).harnesses[0].parts[0].state, State::Current);
    }
}

/// A `Scenario` describes a tree with an optional `AGENTS.md`, an optional
/// `CLAUDE.md`, an optional settings file in any style, and the parts that
/// the install skips. `install_is_idempotent` runs `install` on it twice.
#[derive(Debug, Clone)]
struct Scenario {
    agents: Option<String>,
    claude: Option<String>,
    settings: Option<(serde_json::Value, String, bool)>,
    without: Vec<&'static str>,
}

fn arb_scenario() -> impl Strategy<Value = Scenario> {
    (
        prop::option::of(arb_text()),
        prop::option::of(prop_oneof![Just("@AGENTS.md\n".to_string()), arb_text()]),
        prop::option::of((arb_settings_object(), arb_indent(), any::<bool>())),
        prop::collection::btree_set(prop::sample::select(PARTS.to_vec()), 0..3),
    )
        .prop_map(|(agents, claude, settings, without)| Scenario {
            agents,
            claude,
            settings,
            without: without.into_iter().collect(),
        })
}

fn build(name: &str, s: &Scenario) -> TempTree {
    let tree = TempTree::empty(name);
    decline(&tree, &s.without);
    if let Some(t) = &s.agents {
        tree.write("AGENTS.md", t);
    }
    if let Some(t) = &s.claude {
        tree.write("CLAUDE.md", t);
    }
    if let Some((object, indent, newline)) = &s.settings {
        tree.write(
            ".claude/settings.json",
            &json_text(object, indent, *newline),
        );
    }
    tree
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    // sokf-tests: CLI_P-2
    #[test]
    fn install_is_idempotent(s in arb_scenario()) {
        let tree = build("harness-p1", &s);
        let opts = claude_opts();
        let dirs = HarnessDirs::repo(tree.root());
        let first = harness_install(&dirs, &opts).unwrap();
        let after_first = tree.files();
        let second = harness_install(&dirs, &opts).unwrap();
        // A part that the first install found edited, such as a settings file that holds
        // the tool's hook twice, stays edited; every other part is current or skipped.
        for (p, before) in second.harnesses[0].parts.iter().zip(&first.harnesses[0].parts) {
            let edited = before.verb() == "edited";
            prop_assert!(p.verb() == "current" || p.verb() == "skipped" || (edited && p.verb() == "edited"), "{second:?}");
            prop_assert_eq!(p.verb() == "skipped", s.without.contains(&p.part.as_str()));
        }
        prop_assert_eq!(tree.files(), after_first);
        let status = harness_status(&dirs, &["claude".to_string()], HarnessScope::Project).unwrap();
        prop_assert_eq!(verbs(&status), verbs(&second));
    }

    // sokf-tests: CLI-9_AC-1
    // sokf-tests: CLI-3_AC-62
    #[test]
    fn nothing_on_refusal(s in arb_scenario(), which in 0u8..4) {
        let tree = build("harness-p5", &s);
        let without: Vec<String> = s.without.iter().map(ToString::to_string).collect();
        let mut name = "claude".to_string();
        let mut force: Vec<String> = Vec::new();
        match which {
            0 => name = "cline".into(),
            1 => force.push("mcp".into()),
            2 => tree.write(".claude/settings.json", "{ not json"),
            _ => tree.write(".sokf/harness.toml", "[claude\nx = "),
        }
        let before = tree.files();
        let result = harness_install(
            &HarnessDirs::repo(tree.root()),
            &HarnessOptions::new([name], HarnessScope::Project).force(force),
        );
        // The tool never reads a declined part, so a settings file that both
        // of its parts decline is no refusal, and the install leaves it as it
        // is.
        let settings_declined = ["hooks", "permissions"].iter().all(|p| without.iter().any(|w| w == p));
        if which == 2 && settings_declined {
            prop_assert!(result.is_ok(), "{result:?}");
            prop_assert_eq!(tree.read(".claude/settings.json"), "{ not json");
            return Ok(());
        }
        let e = result.unwrap_err();
        prop_assert!(matches!(e, sokf_core::Error::Harness(_)), "{e}");
        prop_assert_eq!(tree.files(), before);
    }
}

/// `EDITS` holds the contents that the test writes to `knowledge/a.md` in
/// turn: an empty entry and two documents with distinct errors. The empty
/// entry removes the file and leaves the knowledge valid.
const EDITS: &[&str] = &[
    "",
    "---\ntype: Nope\nid: a\n---\n\n# A\n",
    "---\ntype: Other\nid: a\n---\n\n# A\n",
];

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(24))]

    // sokf-tests: CLI_P-20
    #[test]
    fn the_hook_notes_once_per_set_of_errors(edits in prop::collection::vec(0usize..3, 1..6)) {
        let tree = TempTree::empty("harness-p6");
        sokf_core::init(
            &tree.root(),
            &sokf_core::InitOptions { dir: None, cwd: tree.dir().to_path_buf(), name: None, scope: Some("test".into()), description: None },
        )
        .unwrap();
        let call = harness::hook_call("claude", "prompt-submit").unwrap();
        let mut last_noted: Option<String> = None;
        for edit in edits {
            let text = EDITS[edit];
            if text.is_empty() {
                let _ = std::fs::remove_file(tree.dir().join("knowledge/a.md"));
            } else {
                tree.write("knowledge/a.md", text);
            }
            let answer = harness::hook(&tree.root(), &call, "{}", CacheMode::On).unwrap();
            let report = tree.validate_all(false, CacheMode::Off);
            let errors = sokf_core::Report { errors: report.errors.clone(), ..Default::default() };
            let current = report_text(&errors, false);
            if report.errors.is_empty() {
                prop_assert_eq!(answer, harness::Answer::Allow { stderr: None });
                // A clean run clears the guard.
                last_noted = None;
            } else if last_noted.as_deref() == Some(current.as_str()) {
                prop_assert_eq!(answer, harness::Answer::Allow { stderr: None });
            } else {
                prop_assert!(matches!(answer, harness::Answer::Context { .. }), "{answer:?}");
                last_noted = Some(current);
            }
        }
    }
}
