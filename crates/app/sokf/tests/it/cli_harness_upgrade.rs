//! These tests run `sokf harness` and `sokf validate` end to end in a
//! repository whose record sokf 0.8 wrote, and check each line that a user
//! sees there: a part that means the same as sokf's content converts its
//! record at the first install, and a part that differs reads as edited, with
//! a warning in sokf's words, until a forced install writes it.

use std::fs;

use common::with_hooks;

use crate::common;

/// `RECORD_OF_0_8` is the record that sokf 0.8.0 wrote for Claude Code with
/// its hooks, in a repository whose instructions are in `/CLAUDE.md`. The
/// adapter of sokf 0.8.0 recorded a `fnv1a64:` hash of each part, which the
/// adapter of this sokf can't use.
const RECORD_OF_0_8: &str = "# Written by sokf harness install. Do not edit it: sokf compares the parts with these hashes to tell a part you edited from a stale one.

[claude]
hooks = \"fnv1a64:ad06d5319ebb82da\"
instructions = \"fnv1a64:9332df6d49be414d\"
permissions = \"fnv1a64:5a8e1198910bedeb\"
skills = \"fnv1a64:0e67475aa679f455\"
";

/// `WARNING` is the warning of the report at the skill whose record sokf 0.8
/// wrote.
const WARNING: &str =
    "warning: claude: skills in /.claude/skills may not be an edit, as an older sokf recorded it\n";

// sokf-tests: CLI-3_AC-113
// sokf-tests: CLI-4_AC-118
// sokf-tests: CLI-4_AC-43
// sokf-tests: CLI-3_AC-6
#[test]
fn a_record_of_sokf_0_8_converts_or_warns_until_a_forced_install() {
    let f = common::project();
    f.write(".sokf/config.toml", &with_hooks(&["claude"]));
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    // The skill of an older sokf, with the record that sokf 0.8 wrote.
    let skill = f.path(".claude/skills/sokf/SKILL.md");
    let text = fs::read_to_string(&skill).unwrap();
    let older = text.replace("# SOKF v0.7 agent reference", "# SOKF v0.6 agent reference");
    assert_ne!(older, text);
    fs::write(&skill, &older).unwrap();
    f.write(".sokf/harness.toml", RECORD_OF_0_8);
    let next = "    next: `sokf harness diff claude` shows the edit, and `sokf harness install claude --force skills` overwrites it\n";
    // Status and install report the skill edited, with the next step and the
    // warning; the parts that mean the same as sokf's content are current.
    let (status, _, code) = f.run(&["harness", "status", "claude"]);
    assert_eq!(code, 0, "{status}");
    let expected = format!(
        "claude:\n  current /CLAUDE.md (instructions)\n  edited  /.claude/skills (skills)\n{next}  current /.claude/settings.json (hooks)\n  current /.claude/settings.json (permissions)\n{WARNING}"
    );
    assert_eq!(status, expected);
    let (json, _, code) = f.run(&["harness", "status", "claude", "--json"]);
    assert_eq!(code, 0, "{json}");
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        json["warnings"],
        serde_json::json!([WARNING.trim_start_matches("warning: ").trim_end()])
    );
    // `validate` warns at the skill, without `--warnings`.
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out,
        "/.claude/skills: warning: the skills part of the claude harness may not be an edit, as an older sokf recorded it; check `sokf harness diff claude`, then run `sokf harness install claude --force skills` unless the difference holds an edit of the project's (CLI-4_AC-118)\n\n0 errors, 1 of 1 warnings listed\n"
    );
    // `diff` shows the skill's difference from sokf's.
    let (patch, _, code) = f.run(&["harness", "diff", "claude"]);
    assert_eq!(code, 0, "{patch}");
    assert!(
        patch.starts_with(
            "diff --git a/.claude/skills/sokf/SKILL.md b/.claude/skills/sokf/SKILL.md\n"
        ),
        "{patch}"
    );
    assert!(
        patch.contains("\n-# SOKF v0.7 agent reference\n+# SOKF v0.6 agent reference\n"),
        "{patch}"
    );
    // The first install converts the record of the current parts, and leaves
    // the skill and its record.
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, expected);
    assert_eq!(fs::read_to_string(&skill).unwrap(), older);
    let record = fs::read_to_string(f.path(".sokf/harness.toml")).unwrap();
    assert_eq!(record.matches("fnv1a64:").count(), 1, "{record}");
    assert!(
        record.contains("skills = \"fnv1a64:0e67475aa679f455\""),
        "{record}"
    );
    // A forced install of the skill writes it, and the warnings go.
    let (out, err, code) = f.run(&["harness", "install", "claude", "--force", "skills"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out,
        "claude:\n  current /CLAUDE.md (instructions)\n  rewrote /.claude/skills (skills)\n  current /.claude/settings.json (hooks)\n  current /.claude/settings.json (permissions)\n"
    );
    assert_eq!(fs::read_to_string(&skill).unwrap(), text);
    let record = fs::read_to_string(f.path(".sokf/harness.toml")).unwrap();
    assert!(!record.contains("fnv1a64:"), "{record}");
    let (status, _, _) = f.run(&["harness", "status", "claude"]);
    assert!(!status.contains("warning:"), "{status}");
    let (out, _, _) = f.run(&["validate"]);
    assert_eq!(out, "0 errors, 0 warnings\n");
    // An edit made again after the forced install is an edit, with no warning.
    fs::write(&skill, &older).unwrap();
    let (status, _, _) = f.run(&["harness", "status", "claude"]);
    assert!(
        status.contains("  edited  /.claude/skills (skills)\n"),
        "{status}"
    );
    assert!(!status.contains("warning:"), "{status}");
    let (out, _, _) = f.run(&["validate"]);
    assert_eq!(out, "0 errors, 0 warnings\n");
}

// sokf-tests: CLI-3_AC-47
#[test]
fn a_part_of_a_record_of_sokf_0_8_that_holds_sokf_s_content_is_current() {
    let f = common::project();
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    let record = RECORD_OF_0_8.replace("hooks = \"fnv1a64:ad06d5319ebb82da\"\n", "");
    f.write(".sokf/harness.toml", &record);
    let (status, _, code) = f.run(&["harness", "status", "claude"]);
    assert_eq!(code, 0, "{status}");
    assert!(!status.contains("edited"), "{status}");
    assert!(!status.contains("warning:"), "{status}");
    let (out, _, _) = f.run(&["validate"]);
    assert_eq!(out, "0 errors, 0 warnings\n");
    // The next install records each part's fingerprint, and writes no part.
    let (out, err, code) = f.run(&["harness", "install", "claude"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.lines()
            .skip(1)
            .all(|l| l.starts_with("  current ") || l.starts_with("  skipped ")),
        "{out}"
    );
    let after = fs::read_to_string(f.path(".sokf/harness.toml")).unwrap();
    assert!(!after.contains("fnv1a64:"), "{after}");
    assert!(after.contains("\n[claude.skills]\n"), "{after}");
}
