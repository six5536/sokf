//! This module defines sokf's integration with agent harnesses (CLI-3_AC-14): the
//! instructions block, the `sokf` skill, the stop hook, and the allowed command, declared
//! once as an integration of agent-harness-adapter. Each harness of the adapter renders it into its
//! own files.

use std::collections::BTreeMap;

use agent_harness_adapter_core::{
    EntryMatch, Integration,
    hook::Event,
    integration::{Hook, Skill},
};

const SKILL: &str = include_str!(concat!(env!("OUT_DIR"), "/harness/skills/sokf/SKILL.md"));
// sokf-implements: CLI-3_AC-35
const INSTRUCTIONS: &str = include_str!(concat!(env!("OUT_DIR"), "/harness/instructions.md"));

/// `EMBEDDED` lists every embedded file, relative to `harness/` at the
/// repository root. The test `every_file_under_harness_is_listed` walks the
/// directory and fails when it finds a file that this list lacks.
#[cfg(test)]
const EMBEDDED: &[&str] = &["skills/sokf/SKILL.md", "instructions.md"];

/// `HOOK_COMMAND_MARK` is the text that identifies the tool's hook command,
/// whichever binary runs it. A command is the tool's when it contains this
/// text. The tool can't identify the command by the binary's name, because a
/// repository can point its hook at its own build of the tool (CLI-3_AC-42,
/// CONF-4_AC-2).
pub const HOOK_COMMAND_MARK: &str = " harness hook ";

/// `ALLOWED_COMMAND` is the command that the agent may run without asking
/// (CLI-3_AC-37).
pub const ALLOWED_COMMAND: &str = "sokf";

/// `HOOK_EVENTS` lists the events that `sokf harness hook` answers.
pub const HOOK_EVENTS: &[Event] = &[Event::Stop];

/// `version_line` returns the comment line that `sokf harness install` puts
/// after the frontmatter of each skill it writes.
fn version_line() -> String {
    format!(
        "<!-- Written by sokf {} with sokf harness install. Do not edit it: sokf harness install overwrites it. -->",
        env!("CARGO_PKG_VERSION")
    )
}

/// `frontmatter` splits the embedded skill into the values of its frontmatter
/// keys `name` and `description`, and its body.
fn frontmatter(skill: &str) -> (&str, &str, &str) {
    let rest = skill
        .strip_prefix("---\n")
        .expect("the embedded skill has frontmatter");
    let close = rest
        .find("\n---\n")
        .expect("the embedded skill closes its frontmatter");
    let (head, body) = (&rest[..close], &rest[close + 5..]);
    let value = |key: &str| {
        head.lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix(": "))
            .unwrap_or_else(|| panic!("the embedded skill has no `{key}`"))
    };
    (value("name"), value("description"), body)
}

/// `skill` returns the `sokf` skill: the embedded skill's name and
/// description, and its body after the version line.
// sokf-implements: CLI-3_AC-38
pub fn skill() -> Skill {
    let (name, description, body) = frontmatter(SKILL);
    let body = body.strip_prefix('\n').unwrap_or(body);
    Skill::new(name, description, format!("{}\n\n{body}", version_line()))
}

/// `hook_command` returns the command template of the stop hook: `program`, then
/// ` harness hook {harness} {event}`, with any brace in `program` escaped so that the
/// adapter leaves it as written (CONF-4_AC-2).
pub fn hook_command(program: &str) -> String {
    let program = program.replace('{', "{{").replace('}', "}}");
    format!("{program}{HOOK_COMMAND_MARK}{{harness}} {{event}}")
}

/// `integration` returns sokf's integration, whose hook command runs `sokf`, or for a
/// harness that `programs` names, the program that it gives (CONF-4_AC-2). Each hook entry whose command contains `HOOK_COMMAND_MARK` is sokf's, so the
/// hooks part replaces it, whatever program it runs, and leaves the user's
/// entries alone.
// The adapter's Claude Code harness writes the skill to `/.claude/skills/sokf/SKILL.md` and
// the instructions to the file that `claude::instructions_file` chooses.
// sokf-implements: CLI-3_AC-32
// sokf-implements: CLI-3_AC-33
// sokf-implements: CLI-3_AC-34
// sokf-implements: CLI-3_AC-14
// sokf-implements: CLI-3_AC-36
// sokf-implements: CLI-3_AC-37
// sokf-implements: CLI-3_AC-42
// sokf-implements: CONF-4_AC-2
pub fn integration(programs: &BTreeMap<String, String>) -> Integration {
    let mut i = Integration::new()
        .instructions(INSTRUCTIONS)
        .skill(skill())
        .hook_match(EntryMatch::Contains(HOOK_COMMAND_MARK.into()))
        .allow_command(ALLOWED_COMMAND);
    for event in HOOK_EVENTS {
        let hook = programs.iter().fold(
            Hook::new(*event, hook_command(super::DEFAULT_HOOK_PROGRAM)),
            |hook, (harness, program)| hook.command_for(harness, hook_command(program)),
        );
        i = i.hook(hook);
    }
    i
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use agent_harness_adapter_core::Item;

    use super::*;

    // sokf-tests: CLI-3_AC-14
    // sokf-tests: CLI-3_AC-36
    // sokf-tests: CLI-3_AC-37
    #[test]
    fn the_integration_has_four_items_and_the_stop_hook() {
        let i = integration(&BTreeMap::new());
        assert_eq!(
            i.items(),
            [
                Item::Instructions,
                Item::Skills,
                Item::Hooks,
                Item::Permissions
            ]
        );
        assert_eq!(i.allowed_commands(), ["sokf"]);
        assert_eq!(i.hooks().len(), 1);
        assert_eq!(i.hooks()[0].event(), Event::Stop);
        assert_eq!(
            i.hooks()[0].command("claude"),
            "sokf harness hook claude stop"
        );
        assert_eq!(
            i.hook_owner(&i.hooks()[0]).unwrap(),
            EntryMatch::Contains(" harness hook ".into())
        );
        // A harness's own program replaces `sokf` at the start of its command only, and a
        // brace in it stays as written.
        let i = integration(&[("codex".to_string(), "./target/debug/sokf".to_string())].into());
        assert_eq!(
            i.hooks()[0].command("codex"),
            "./target/debug/sokf harness hook codex stop"
        );
        assert_eq!(
            i.hooks()[0].command("claude"),
            "sokf harness hook claude stop"
        );
        assert_eq!(
            Hook::new(Event::Stop, hook_command("${DIR}/sokf")).command("pi"),
            "${DIR}/sokf harness hook pi stop"
        );
    }

    // sokf-tests: CLI-3_AC-38
    #[test]
    fn the_skill_carries_the_version_line_after_its_frontmatter() {
        let files = skill().dir_files();
        assert_eq!(files.len(), 1);
        let (path, text) = &files[0];
        assert_eq!(path, "sokf/SKILL.md");
        assert!(text.starts_with("---\nname: sokf\ndescription: "), "{text}");
        let expected = format!("---\n\n{}\n\n# SOKF", version_line());
        assert!(text.contains(&expected), "{text}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
        // The body of the skill appears once, unchanged, after the version line.
        assert_eq!(text.matches("# SOKF v0.6").count(), 1);
    }

    /// `every_file_under_harness_is_listed` checks that the binary embeds
    /// every file under `harness/`. The test passes without checking when
    /// the workspace directory is absent, as it is in a packaged crate.
    #[test]
    fn every_file_under_harness_is_listed() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../harness");
        if !dir.is_dir() {
            return;
        }
        let mut found = Vec::new();
        walk(&dir, &dir, &mut found);
        found.sort();
        let mut listed: Vec<String> = EMBEDDED.iter().map(|s| s.to_string()).collect();
        listed.sort();
        assert_eq!(found, listed, "every file under harness/ must be embedded");
    }

    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path: PathBuf = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
}
