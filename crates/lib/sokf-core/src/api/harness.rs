//! This module carries out `sokf harness install`, `sokf harness status`, and `sokf harness diff`
//! (CLI-3), through `install`, `status`, and `edits` of agent-harness-adapter. It reads the config, presents sokf to the adapter
//! as a `Tool`, and turns the adapter's result and refusals into sokf's report and errors, with
//! paths from the repository root, or from the home directory at the user scope.

use std::path::PathBuf;

use agent_harness_adapter_core::{Action, InstallOptions, Item, Scope, State, harness};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{Error, RepoRoot, Result, config, harness::SokfTool, pack};

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
    /// The parts that `--force` names, which `install` writes whatever their state, or `all`
    /// for every part that the configuration doesn't decline. Empty without `--force`.
    pub force: Vec<String>,
}

impl HarnessOptions {
    /// `new` returns the options that install `harnesses` at `scope`, with the declined parts
    /// in the configuration and without `--force`.
    pub fn new<I: IntoIterator<Item = S>, S: Into<String>>(harnesses: I, scope: Scope) -> Self {
        HarnessOptions {
            harnesses: harnesses.into_iter().map(Into::into).collect(),
            scope,
            force: Vec::new(),
        }
    }

    /// `force` writes `parts` whatever their state, or every part that the configuration
    /// doesn't decline when `parts` holds `all`.
    #[must_use]
    pub fn force<I: IntoIterator<Item = S>, S: Into<String>>(mut self, parts: I) -> Self {
        self.force = parts.into_iter().map(Into::into).collect();
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
    /// each part, with a `next:` line after each part that is edited and stays as it is,
    /// then `unsupported:` and `note:` lines; then a `warning:` line for each warning.
    // sokf-implements: CLI-3_AC-19
    // sokf-implements: CLI-3_AC-77
    pub fn to_text(&self) -> String {
        let scope = match self.scope {
            Scope::Project => String::new(),
            other => format!(" --scope {other}"),
        };
        let mut out = String::new();
        for h in &self.harnesses {
            out.push_str(&format!("{}:\n", h.harness));
            for p in &h.parts {
                out.push_str(&format!("  {}\n", p.to_line()));
                if p.state == State::Edited && p.action.is_none() {
                    out.push_str(&format!(
                        "    next: `sokf harness diff {h}{scope}` shows the edit, and `sokf harness install {h}{scope} --force {p}` overwrites it\n",
                        h = h.harness,
                        p = p.part
                    ));
                }
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
/// for the hook line of each harness (CONF-4_AC-2, CONF-4_AC-3). At the project and local
/// scopes, the instructions block holds the fragment of each pack in the lock, which it
/// reads from the working tree.
// sokf-implements: CONF-4_AC-2
// sokf-implements: CONF-4_AC-3
// sokf-implements: CLI-3_AC-73
// sokf-implements: CLI-3_AC-74
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
    // The fragments of the packs belong to the repository, so the block of the user scope
    // holds none.
    let fragments = match (scope, &dirs.repo) {
        (Scope::User, _) | (_, None) => Vec::new(),
        (_, Some(repo)) => pack::fragments(repo, &pack::read_lock(repo)?)?,
    };
    Ok(SokfTool::new(
        dirs.repo.as_ref().map(|r| r.as_path().to_path_buf()),
        dirs.home.clone(),
        config.hook_commands,
    )
    .fragments(fragments))
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
/// the record, and never the configuration. The adapter carries out the part kinds, the states, the
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
// sokf-implements: CLI-3_AC-75
// sokf-implements: CLI-3_AC-76
pub fn install(dirs: &HarnessDirs, opts: &HarnessOptions) -> Result<HarnessReport> {
    let tool = tool(dirs, opts.scope)?;
    let kit_opts = InstallOptions::new(harness_ids(&opts.harnesses), opts.scope)
        .force_parts(opts.force.iter().cloned());
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

/// `diff` returns a patch for each file of each part in the state `edited` of each harness
/// named, from the text that `install` would write to the text on disk, with the paths
/// `a/<path>` and `b/<path>` from the root of `scope`, in the order of `status`. A `file` part
/// gives each file whole; a `region` part gives the block between its markers, and a `merge`
/// part its entries as JSON, so their line numbers count from the block or the entries. It returns
/// an empty text when no part is edited, and writes nothing.
// sokf-implements: CLI-3_AC-78
// sokf-implements: CLI-3_AC-79
// sokf-implements: CLI-3_AC-80
pub fn diff(dirs: &HarnessDirs, harnesses: &[String], scope: Scope) -> Result<String> {
    let tool = tool(dirs, scope)?;
    let edits = agent_harness_adapter_core::edits(&tool, harness_ids(harnesses), scope)
        .map_err(refusal(scope))?;
    let mut out = String::new();
    for e in &edits {
        out.push_str(&pack::diff(
            &e.path,
            e.expected.as_bytes(),
            Some(e.found.as_bytes()),
        )?);
    }
    Ok(out)
}

/// `refresh_instructions` rewrites the `instructions` part of each harness installed at the
/// project and local scopes of `dirs` whose part is stale, so that the block follows the
/// packs in the lock. It leaves every other part, and an instructions part that is edited,
/// skipped, shared, or absent, unchanged, and writes nothing where no harness is installed.
/// It returns a line for each part that it rewrote.
///
/// While the copy of a pack's fragment holds a conflict that a merge left,
/// it rewrites no part, because the block would hold the conflict markers,
/// and it returns a line for each stale part that it kept, which names the
/// conflicted copy.
// sokf-implements: CLI-13_AC-38
// sokf-implements: CLI-13_AC-39
// sokf-implements: CLI-13_AC-40
pub(crate) fn refresh_instructions(dirs: &HarnessDirs) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let conflicted: Vec<String> = match &dirs.repo {
        Some(repo) => pack::fragments(repo, &pack::read_lock(repo)?)?
            .into_iter()
            .map(|(name, text)| (pack::instructions_of(&name), text))
            .filter(|(path, text)| !crate::check::conflict_lines(path, text).is_empty())
            .map(|(path, _)| path.as_str().to_string())
            .collect(),
        None => Vec::new(),
    };
    for scope in [Scope::Project, Scope::Local] {
        let tool = tool(dirs, scope)?.instructions_only();
        let installed =
            agent_harness_adapter_core::installed(&tool, scope).map_err(refusal(scope))?;
        if installed.is_empty() {
            continue;
        }
        let found =
            agent_harness_adapter_core::status(&tool, &installed, scope).map_err(refusal(scope))?;
        let state = |h: &agent_harness_adapter_core::HarnessResult| {
            h.parts
                .iter()
                .find(|p| p.part == Item::Instructions.as_str())
                .map(|p| p.state)
        };
        if !found
            .harnesses
            .iter()
            .any(|h| state(h) == Some(State::Stale))
        {
            continue;
        }
        // sokf-implements: CLI-13_AC-42
        if !conflicted.is_empty() {
            for h in found
                .harnesses
                .iter()
                .filter(|h| state(h) == Some(State::Stale))
            {
                for p in h
                    .parts
                    .iter()
                    .filter(|p| p.part == Item::Instructions.as_str())
                {
                    lines.push(format!(
                        "kept {} ({} {}): {} holds a conflict; resolve it, and the next `sokf pack sync` rewrites the part",
                        shown(scope, &p.path),
                        h.harness,
                        p.part,
                        conflicted.join(", ")
                    ));
                }
            }
            continue;
        }
        // An install creates an absent part, so a harness whose block is absent stays out.
        let named: Vec<String> = found
            .harnesses
            .iter()
            .filter(|h| state(h).is_some_and(|s| s != State::Absent))
            .map(|h| h.harness.clone())
            .collect();
        let written = report(
            agent_harness_adapter_core::install(&tool, &InstallOptions::new(named, scope))
                .map_err(refusal(scope))?,
        );
        for h in &written.harnesses {
            for p in h.parts.iter().filter(|p| p.action.is_some()) {
                lines.push(format!(
                    "{} {} ({} {})",
                    p.verb(),
                    p.path,
                    h.harness,
                    p.part
                ));
            }
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        crate::test_support::fresh_dir(&format!("harness-api-{name}"))
    }

    fn opts(force: &[&str]) -> HarnessOptions {
        HarnessOptions::new(["claude"], Scope::Project).force(force.iter().copied())
    }

    /// `decline` writes the configuration of `dir` so that it declines `parts` of `claude`.
    fn decline(dir: &std::path::Path, parts: &str) {
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        fs::write(
            dir.join(".sokf/config.toml"),
            format!("[harness.claude]\nwithout = [{parts}]\n"),
        )
        .unwrap();
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
        let e = install(&dirs, &opts(&["mcp"])).unwrap_err();
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
        let e = install(&dirs, &opts(&[])).unwrap_err();
        assert!(
            e.to_string().starts_with("harness: /.sokf/config.toml: "),
            "{e}"
        );
        fs::remove_file(dir.join(".sokf/config.toml")).unwrap();
        fs::write(dir.join(".sokf/harness.toml"), "x = [\n").unwrap();
        let e = install(&dirs, &opts(&[])).unwrap_err();
        assert!(
            e.to_string().starts_with("harness: /.sokf/harness.toml: "),
            "{e}"
        );
        fs::remove_file(dir.join(".sokf/harness.toml")).unwrap();
        fs::create_dir_all(dir.join(".claude")).unwrap();
        fs::write(dir.join(".claude/settings.json"), "{ nope").unwrap();
        let e = install(&dirs, &opts(&[])).unwrap_err();
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
    // sokf-tests: CLI-3_AC-75
    // sokf-tests: CLI-3_AC-76
    #[test]
    fn the_verbs_follow_the_states() {
        let dir = temp("verbs");
        let dirs = HarnessDirs::repo(RepoRoot::new(dir.clone()));
        decline(&dir, "\"hooks\"");
        let first = install(&dirs, &opts(&[])).unwrap();
        assert_eq!(verbs(&first), ["created", "created", "skipped", "created"]);
        let parts = &first.harnesses[0].parts;
        assert_eq!(parts[0].state, State::Absent);
        assert_eq!(parts[0].action, Some(Action::Created));
        assert_eq!(parts[0].path, "/CLAUDE.md");
        assert_eq!(parts[1].path, "/.claude/skills");
        // A second run reports `current` and `skipped`, and the record has no hooks key.
        let second = install(&dirs, &opts(&[])).unwrap();
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
        let third = install(&dirs, &opts(&[])).unwrap();
        assert_eq!(verbs(&third), ["edited", "edited", "skipped", "current"]);
        assert_eq!(
            fs::read_to_string(dir.join(".claude/skills/sokf/SKILL.md")).unwrap(),
            "mine\n"
        );
        // `--force skills` rewrites the skill alone, and the block keeps its edit.
        let skill = install(&dirs, &opts(&["skills"])).unwrap();
        assert_eq!(verbs(&skill), ["edited", "rewrote", "skipped", "current"]);
        assert!(
            fs::read_to_string(dir.join("CLAUDE.md"))
                .unwrap()
                .contains("ignore the")
        );
        // `--force` on a declined part refuses, names the config, and writes nothing.
        let before = fs::read_to_string(dir.join(".sokf/harness.toml")).unwrap();
        let e = install(&dirs, &opts(&["hooks"])).unwrap_err();
        assert_eq!(
            e.to_string(),
            "harness: harness `claude` declines `hooks` in /.sokf/config.toml; remove it from `without` there to force it"
        );
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/harness.toml")).unwrap(),
            before
        );
        // `--force all` covers every part that the config doesn't decline.
        let forced = install(&dirs, &opts(&["all"])).unwrap();
        assert_eq!(verbs(&forced), ["updated", "current", "skipped", "current"]);
        assert_eq!(forced.harnesses[0].parts[0].state, State::Edited);
        let st = status(&dirs, &["claude".into()], Scope::Project).unwrap();
        assert_eq!(verbs(&st), ["current", "current", "skipped", "current"]);
        // No install changed the config.
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/config.toml")).unwrap(),
            "[harness.claude]\nwithout = [\"hooks\"]\n"
        );
        // Turn the hooks back on by editing the config: `install` writes them into the
        // settings file, which exists, so it reports `updated`.
        fs::write(
            dir.join(".sokf/config.toml"),
            "[harness.claude]\nwithout = []\n",
        )
        .unwrap();
        let st = status(&dirs, &["claude".into()], Scope::Project).unwrap();
        assert_eq!(st.harnesses[0].parts[2].verb(), "absent");
        let fourth = install(&dirs, &opts(&[])).unwrap();
        assert_eq!(fourth.harnesses[0].parts[2].verb(), "updated");
        let record = fs::read_to_string(dir.join(".sokf/harness.toml")).unwrap();
        assert!(record.contains("hooks = \"fnv1a64:"), "{record}");
        // Declining the hooks again skips them and keeps their hash.
        decline(&dir, "\"hooks\"");
        let fifth = install(&dirs, &opts(&[])).unwrap();
        assert_eq!(fifth.harnesses[0].parts[2].verb(), "skipped");
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/harness.toml")).unwrap(),
            record
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-3_AC-77
    // sokf-tests: CLI-3_AC-78
    // sokf-tests: CLI-3_AC-79
    // sokf-tests: CLI-3_AC-80
    #[test]
    fn diff_gives_a_patch_for_each_edited_file_and_writes_nothing() {
        let dir = temp("diff");
        let dirs = HarnessDirs::repo(RepoRoot::new(dir.clone()));
        let claude = ["claude".to_string()];
        install(&dirs, &opts(&[])).unwrap();
        assert_eq!(diff(&dirs, &claude, Scope::Project).unwrap(), "");
        let skill = dir.join(".claude/skills/sokf/SKILL.md");
        let text = fs::read_to_string(&skill).unwrap();
        fs::write(&skill, format!("{text}mine\n")).unwrap();
        let patch = diff(&dirs, &claude, Scope::Project).unwrap();
        assert!(
            patch.starts_with(
                "diff --git a/.claude/skills/sokf/SKILL.md b/.claude/skills/sokf/SKILL.md\n"
            ),
            "{patch}"
        );
        assert!(patch.contains("\n+mine\n"), "{patch}");
        assert_eq!(fs::read_to_string(&skill).unwrap(), format!("{text}mine\n"));
        // The text report names both next steps beside the edited part.
        let st = status(&dirs, &claude, Scope::Project).unwrap().to_text();
        assert!(
            st.contains("    next: `sokf harness diff claude` shows the edit, and `sokf harness install claude --force skills` overwrites it\n"),
            "{st}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
