//! This module carries out `sokf harness install` and `sokf harness status` (CLI-3), through
//! `install` and `status` of agent-harness-adapter. It reads the config, presents sokf to the adapter
//! as a `Tool`, and turns the adapter's result and refusals into sokf's report and errors, with
//! paths from the repository root, or from the home directory at the user scope.

use std::path::PathBuf;

use agent_harness_adapter_core::{Action, InstallOptions, Item, Scope, State, harness};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{Error, RepoRoot, Result, config, harness::SokfTool};

/// `ALL_HARNESSES` is the harness name that stands for every harness that sokf supports.
pub const ALL_HARNESSES: &str = "all";

/// The directories that `install` and `status` work in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarnessDirs {
    /// The repository: the root of the project and local scopes.
    pub repo: Option<RepoRoot>,
    /// The home directory: the root of the user scope. At the project scope, a harness
    /// can read a setting there.
    pub home: Option<PathBuf>,
}

impl HarnessDirs {
    /// `repo` returns the directories of the repository `root`, without a home directory.
    pub fn repo(root: RepoRoot) -> Self {
        HarnessDirs {
            repo: Some(root),
            home: None,
        }
    }

    /// `home` returns the directories with the home directory `home`.
    #[must_use]
    pub fn home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }
}

/// The options of `install`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessOptions {
    /// The harness names, for example `claude`, or `all`.
    pub harnesses: Vec<String>,
    /// The scope: `project`, `user`, or `local`.
    pub scope: Scope,
    /// The parts listed after `--without`, or `None` when the option isn't given.
    pub without: Option<Vec<String>>,
    /// Whether `--force` is given.
    pub force: bool,
}

impl HarnessOptions {
    /// `new` returns the options that install `harnesses` at `scope`, with the declined parts
    /// in the configuration and without `--force`.
    pub fn new<I: IntoIterator<Item = S>, S: Into<String>>(harnesses: I, scope: Scope) -> Self {
        HarnessOptions {
            harnesses: harnesses.into_iter().map(Into::into).collect(),
            scope,
            without: None,
            force: false,
        }
    }

    /// `without` declines `parts`, replacing the declined parts in the configuration.
    #[must_use]
    pub fn without<I: IntoIterator<Item = S>, S: Into<String>>(mut self, parts: I) -> Self {
        self.without = Some(parts.into_iter().map(Into::into).collect());
        self
    }

    /// `force` writes the parts that the user edited too.
    #[must_use]
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

// sokf:begin part-result
/// One part of a harness.
// sokf-implements: CLI-3_AC-61
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(rename = "Part")]
pub struct PartResult {
    /// The name of the part, for example `skills` or `hooks`.
    pub part: String,
    /// The state in which the command found the part, before it wrote anything: `skipped`,
    /// `shared`, `absent`, `current`, `stale`, or `edited`.
    pub state: State,
    /// For `install`, what the command did to the part: `created`, `rewrote`, or `updated`.
    /// The key is absent when the command left the part as it found it, and always for
    /// `status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Action")]
    pub action: Option<Action>,
    /// The file or directory to which the command writes the part: a path from the
    /// repository root, which starts with `/`, or at the user scope a path from the home
    /// directory, which starts with `~/`. For a shared part, the location that another
    /// harness writes.
    pub path: String,
    /// For a shared part, the harness whose part writes it. The key is absent for every
    /// other part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub by: Option<String>,
}
// sokf:end part-result

impl PartResult {
    /// `verb` returns the word that the text report shows for the part: the action, or else
    /// the state.
    pub fn verb(&self) -> &'static str {
        self.action
            .map_or_else(|| self.state.as_str(), Action::as_str)
    }

    /// `to_line` returns the part's line of the text report: the verb in a column seven
    /// characters wide, the path, and the name of the part in parentheses, with the harness
    /// that writes a shared part.
    pub fn to_line(&self) -> String {
        match &self.by {
            Some(by) => format!("{:<7} {} ({}, by {by})", self.verb(), self.path, self.part),
            None => format!("{:<7} {} ({})", self.verb(), self.path, self.part),
        }
    }
}

/// The parts of one harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct HarnessResult {
    /// The name of the harness, for example `claude`.
    pub harness: String,
    /// The parts, in the harness's order.
    pub parts: Vec<PartResult>,
    /// The items of sokf's integration that the harness has no place for at the scope, for
    /// example `permissions`.
    pub unsupported: Vec<Item>,
    /// What the user still has to do in the agent, for example trust the folder.
    pub notes: Vec<String>,
}

/// The report of `sokf harness install --json` and `sokf harness status --json`: one entry
/// for each harness named, in sokf's order of the harnesses, with one entry for each part
/// (CLI-3_AC-20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(title = "sokf harness report")]
pub struct HarnessReport {
    /// The scope: `project`, `user`, or `local`.
    pub scope: Scope,
    /// The harnesses, in sokf's order.
    pub harnesses: Vec<HarnessResult>,
    /// The content that an agent may load twice.
    pub warnings: Vec<String>,
}

impl HarnessReport {
    /// `harness` returns the result of the harness `id`.
    pub fn harness(&self, id: &str) -> Option<&HarnessResult> {
        self.harnesses.iter().find(|h| h.harness == id)
    }

    /// `to_text` returns the text report: for each harness, `<name>:`, one indented line for
    /// each part, then `unsupported:` and `note:` lines; then a `warning:` line for each
    /// warning.
    // sokf-implements: CLI-3_AC-19
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for h in &self.harnesses {
            out.push_str(&format!("{}:\n", h.harness));
            for p in &h.parts {
                out.push_str(&format!("  {}\n", p.to_line()));
            }
            if !h.unsupported.is_empty() {
                let items: Vec<&str> = h.unsupported.iter().map(|i| i.as_str()).collect();
                out.push_str(&format!("  unsupported: {}\n", items.join(", ")));
            }
            for n in &h.notes {
                out.push_str(&format!("  note: {n}\n"));
            }
        }
        for w in &self.warnings {
            out.push_str(&format!("warning: {w}\n"));
        }
        out
    }
}

/// `harness_ids` returns the harnesses that `names` names, in the order given, with `all`
/// in place of every harness that sokf supports, each once.
// sokf-implements: CLI-3_AC-65
pub fn harness_ids(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in names {
        let ids = if name == ALL_HARNESSES {
            harness::builtin()
                .iter()
                .map(|h| h.id().to_string())
                .collect()
        } else {
            vec![name.clone()]
        };
        for id in ids {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

/// `shown` returns `path`, which the adapter gives relative to the root of `scope`, as the
/// report shows it: from the repository root with `/`, or from the home directory with
/// `~/`. A path that already starts with either stays as it is.
fn shown(scope: Scope, path: &str) -> String {
    if path.starts_with('/') || path.starts_with('~') {
        return path.to_string();
    }
    match scope {
        Scope::User => format!("~/{path}"),
        _ => format!("/{path}"),
    }
}

/// `tool` reads the configuration of the scope and returns sokf as the adapter's `Tool`, with
/// the program that the repository, or at the user scope the home directory, configures
/// for the hook line of each harness (CONF-4_AC-2, CONF-4_AC-3).
// sokf-implements: CONF-4_AC-2
// sokf-implements: CONF-4_AC-3
fn tool(dirs: &HarnessDirs, scope: Scope) -> Result<SokfTool> {
    let (config_root, prefix) =
        match scope {
            Scope::User => {
                let home = dirs.home.clone().ok_or_else(|| {
                    Error::Harness("the user scope needs the home directory".into())
                })?;
                (RepoRoot::new(home), "~")
            }
            _ => {
                let repo = dirs.repo.clone().ok_or_else(|| {
                    Error::Harness(format!("the {scope} scope needs a repository"))
                })?;
                (repo, "")
            }
        };
    let config = config::read_lenient(&config_root).map_err(|e| match e {
        Error::Init(m) => Error::Harness(format!("{prefix}{m}")),
        other => other,
    })?;
    Ok(SokfTool::new(
        dirs.repo.as_ref().map(|r| r.as_path().to_path_buf()),
        dirs.home.clone(),
        config.hook_commands,
    ))
}

/// `refusal` turns an error of the adapter into a sokf error, naming a file by its path from the
/// root of `scope` (CLI-9_AC-2).
fn refusal(scope: Scope) -> impl Fn(agent_harness_adapter_core::Error) -> Error {
    move |e| match e {
        agent_harness_adapter_core::Error::File { file, message } => {
            Error::Harness(format!("{}: {message}", shown(scope, &file)))
        }
        other => other.into(),
    }
}

/// `report` turns the adapter's result into sokf's report.
fn report(result: agent_harness_adapter_core::InstallResult) -> HarnessReport {
    let scope = result.scope;
    HarnessReport {
        scope,
        harnesses: result
            .harnesses
            .into_iter()
            .map(|h| HarnessResult {
                harness: h.harness,
                parts: h
                    .parts
                    .into_iter()
                    .map(|p| PartResult {
                        path: shown(scope, &p.path),
                        part: p.part,
                        state: p.state,
                        action: p.action,
                        by: p.by,
                    })
                    .collect(),
                unsupported: h.unsupported,
                notes: h.notes,
            })
            .collect(),
        warnings: result.warnings,
    }
}

/// `install` makes the tree match sokf's integration for each harness named, as CLI-3
/// describes. The adapter writes nothing until every check has passed, writes the parts, then
/// the record, then the declined parts. The adapter carries out the part kinds, the states, the
/// hashes, the shared locations, and the record, so this call implements their criteria.
// sokf-implements: CLI-3_AC-12
// sokf-implements: CLI-3_AC-13
// sokf-implements: CLI-3_AC-15
// sokf-implements: CLI-3_AC-16
// sokf-implements: CLI-3_AC-17
// sokf-implements: CLI-3_AC-18
// sokf-implements: CLI-3_AC-39
// sokf-implements: CLI-3_AC-40
// sokf-implements: CLI-3_AC-41
// sokf-implements: CLI-3_AC-43
// sokf-implements: CLI-3_AC-44
// sokf-implements: CLI-3_AC-45
// sokf-implements: CLI-3_AC-46
// sokf-implements: CLI-3_AC-47
// sokf-implements: CLI-3_AC-48
// sokf-implements: CLI-3_AC-49
// sokf-implements: CLI-3_AC-53
// sokf-implements: CLI-3_AC-56
// sokf-implements: CLI-3_AC-59
// sokf-implements: CLI-3_AC-60
// sokf-implements: CLI-3_AC-62
// sokf-implements: CLI-3_AC-66
// sokf-implements: CLI-3_AC-67
// sokf-implements: CONF-4_AC-4
// sokf-implements: CLI-3_AC-1
// sokf-implements: CLI-3_AC-2
// sokf-implements: CLI-3_AC-3
// sokf-implements: CLI-3_AC-4
// sokf-implements: CLI-3_AC-5
// sokf-implements: CLI-3_AC-7
// sokf-implements: CLI-3_AC-22
// sokf-implements: CLI-3_AC-23
// sokf-implements: CLI-3_AC-24
// sokf-implements: CLI-9_AC-1
// sokf-implements: CLI-9_AC-2
// sokf-implements: CLI-3_AC-54
// sokf-implements: CLI-3_AC-55
// sokf-implements: CLI-3_AC-21
// sokf-implements: CLI-3_AC-6
pub fn install(dirs: &HarnessDirs, opts: &HarnessOptions) -> Result<HarnessReport> {
    let tool = tool(dirs, opts.scope)?;
    let mut kit_opts =
        InstallOptions::new(harness_ids(&opts.harnesses), opts.scope).force(opts.force);
    if let Some(without) = &opts.without {
        kit_opts = kit_opts.without(without.iter().cloned());
    }
    Ok(report(
        agent_harness_adapter_core::install(&tool, &kit_opts).map_err(refusal(opts.scope))?,
    ))
}

/// `status` returns the state of every part of each harness named, and writes nothing.
// sokf-implements: CLI-3_AC-11
// sokf-implements: CLI-3_AC-9
// sokf-implements: CLI-3_AC-10
// sokf-implements: CLI-3_AC-21
pub fn status(dirs: &HarnessDirs, harnesses: &[String], scope: Scope) -> Result<HarnessReport> {
    let tool = tool(dirs, scope)?;
    Ok(report(
        agent_harness_adapter_core::status(&tool, harness_ids(harnesses), scope)
            .map_err(refusal(scope))?,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        crate::test_support::fresh_dir(&format!("harness-api-{name}"))
    }

    fn opts(without: Option<&[&str]>, force: bool) -> HarnessOptions {
        let o = HarnessOptions::new(["claude"], Scope::Project).force(force);
        match without {
            Some(w) => o.without(w.iter().copied()),
            None => o,
        }
    }

    fn verbs(r: &HarnessReport) -> Vec<&str> {
        r.harnesses[0].parts.iter().map(PartResult::verb).collect()
    }

    #[test]
    fn all_names_every_harness_once() {
        let names = |n: &[&str]| harness_ids(&n.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(names(&["codex", "claude", "codex"]), ["codex", "claude"]);
        let all = names(&["pi", "all"]);
        assert_eq!(all[0], "pi");
        assert_eq!(all.len(), harness::builtin().len());
        assert_eq!(
            shown(Scope::User, ".claude/settings.json"),
            "~/.claude/settings.json"
        );
        assert_eq!(
            shown(Scope::Local, ".claude/settings.local.json"),
            "/.claude/settings.local.json"
        );
        assert_eq!(
            shown(Scope::User, "~/.sokf/config.toml"),
            "~/.sokf/config.toml"
        );
    }

    // sokf-tests: CLI-3_AC-22
    // sokf-tests: CLI-3_AC-23
    // sokf-tests: CLI-3_AC-24
    // sokf-tests: CONF-1_AC-3
    // sokf-tests: CLI-9_AC-1
    // sokf-tests: CLI-9_AC-2
    #[test]
    fn refuses_before_any_write() {
        let dir = temp("refuse");
        let dirs = HarnessDirs::repo(RepoRoot::new(dir.clone()));
        let e = install(&dirs, &HarnessOptions::new(["cline"], Scope::Project)).unwrap_err();
        assert_eq!(e.to_string(), "harness: no harness named `cline`");
        let e = install(&dirs, &opts(Some(&["mcp"]), false)).unwrap_err();
        assert_eq!(
            e.to_string(),
            "harness: harness `claude` has no part named `mcp`"
        );
        let e = install(&dirs, &HarnessOptions::new(["cursor"], Scope::Local)).unwrap_err();
        assert_eq!(
            e.to_string(),
            "harness: harness `cursor` has no local scope"
        );
        let e = status(&dirs, &["claude".into()], Scope::User).unwrap_err();
        assert_eq!(
            e.to_string(),
            "harness: the user scope needs the home directory"
        );
        assert!(status(&dirs, &["cline".into()], Scope::Project).is_err());
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        fs::write(dir.join(".sokf/config.toml"), "[knowledge\n").unwrap();
        let e = install(&dirs, &opts(None, false)).unwrap_err();
        assert!(
            e.to_string().starts_with("harness: /.sokf/config.toml: "),
            "{e}"
        );
        fs::remove_file(dir.join(".sokf/config.toml")).unwrap();
        fs::write(dir.join(".sokf/harness.toml"), "x = [\n").unwrap();
        let e = install(&dirs, &opts(None, false)).unwrap_err();
        assert!(
            e.to_string().starts_with("harness: /.sokf/harness.toml: "),
            "{e}"
        );
        fs::remove_file(dir.join(".sokf/harness.toml")).unwrap();
        fs::create_dir_all(dir.join(".claude")).unwrap();
        fs::write(dir.join(".claude/settings.json"), "{ nope").unwrap();
        let e = install(&dirs, &opts(None, false)).unwrap_err();
        assert!(e.to_string().contains("does not parse"), "{e}");
        assert!(
            e.to_string()
                .starts_with("harness: /.claude/settings.json: "),
            "{e}"
        );
        // No refusal wrote anything.
        assert!(!dir.join(".claude/skills").exists());
        assert!(!dir.join("CLAUDE.md").exists());
        assert!(!dir.join(".sokf/harness.toml").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-3_AC-1
    // sokf-tests: CLI-3_AC-3
    // sokf-tests: CLI-3_AC-4
    // sokf-tests: CLI-3_AC-5
    // sokf-tests: CLI-3_AC-7
    // sokf-tests: CLI-3_AC-8
    // sokf-tests: CLI-3_AC-9
    // sokf-tests: CLI-3_AC-54
    // sokf-tests: CLI-3_AC-55
    #[test]
    fn the_verbs_follow_the_states() {
        let dir = temp("verbs");
        let dirs = HarnessDirs::repo(RepoRoot::new(dir.clone()));
        let first = install(&dirs, &opts(Some(&["hooks"]), false)).unwrap();
        assert_eq!(verbs(&first), ["created", "created", "skipped", "created"]);
        let parts = &first.harnesses[0].parts;
        assert_eq!(parts[0].state, State::Absent);
        assert_eq!(parts[0].action, Some(Action::Created));
        assert_eq!(parts[0].path, "/CLAUDE.md");
        assert_eq!(parts[1].path, "/.claude/skills");
        // A second run reports `current` and `skipped`, and the record has no hooks key.
        let second = install(&dirs, &opts(None, false)).unwrap();
        assert_eq!(verbs(&second), ["current", "current", "skipped", "current"]);
        assert!(second.harnesses[0].parts.iter().all(|p| p.action.is_none()));
        let record = fs::read_to_string(dir.join(".sokf/harness.toml")).unwrap();
        assert!(record.contains("skills = \"fnv1a64:"), "{record}");
        assert!(!record.contains("hooks"), "{record}");
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/config.toml")).unwrap(),
            "[harness.claude]\nwithout = [\"hooks\"]\n"
        );
        // Edit the block and the skill: `install` leaves them unchanged, and `--force` then
        // rewrites them.
        let claude_md = fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
        fs::write(
            dir.join("CLAUDE.md"),
            claude_md.replace("read the", "ignore the"),
        )
        .unwrap();
        fs::write(dir.join(".claude/skills/sokf/SKILL.md"), "mine\n").unwrap();
        let third = install(&dirs, &opts(None, false)).unwrap();
        assert_eq!(verbs(&third), ["edited", "edited", "skipped", "current"]);
        assert_eq!(
            fs::read_to_string(dir.join(".claude/skills/sokf/SKILL.md")).unwrap(),
            "mine\n"
        );
        let forced = install(&dirs, &opts(None, true)).unwrap();
        assert_eq!(verbs(&forced), ["updated", "rewrote", "skipped", "current"]);
        assert_eq!(forced.harnesses[0].parts[1].state, State::Edited);
        let st = status(&dirs, &["claude".into()], Scope::Project).unwrap();
        assert_eq!(verbs(&st), ["current", "current", "skipped", "current"]);
        // Turn the hooks back on by editing the config: `install` writes them into the
        // settings file, which exists, so it reports `updated`.
        fs::write(
            dir.join(".sokf/config.toml"),
            "[harness.claude]\nwithout = []\n",
        )
        .unwrap();
        let st = status(&dirs, &["claude".into()], Scope::Project).unwrap();
        assert_eq!(st.harnesses[0].parts[2].verb(), "absent");
        let fourth = install(&dirs, &opts(None, false)).unwrap();
        assert_eq!(fourth.harnesses[0].parts[2].verb(), "updated");
        let record = fs::read_to_string(dir.join(".sokf/harness.toml")).unwrap();
        assert!(record.contains("hooks = \"fnv1a64:"), "{record}");
        fs::remove_dir_all(&dir).unwrap();
    }
}
