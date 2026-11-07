//! This module implements `sokf pack status`, `list`, `remove`, `diff`, and
//! `export` (CLI-13). None of them uses the network.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::pack::{SyncedPack, edited_files, holds_accepted, remove};
use crate::{
    Error, RepoPath, RepoRoot, Result, check, config,
    pack::{self, EMBEDDED, EmbeddedPack},
    source::read_optional,
};

/// A `PackFileState` is the state of one file of a locked pack that differs
/// from its base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackFileState {
    /// The user edited the file.
    Edited,
    /// The file holds a conflict that a merge left.
    Conflicted,
    /// The user deleted the file, and a sync keeps it deleted.
    Deleted,
    /// The file holds the edit or the deletion that a maintainer accepted.
    Accepted,
}

impl PackFileState {
    /// `label` returns the word that the report prints.
    pub fn label(self) -> &'static str {
        match self {
            PackFileState::Edited => "edited",
            PackFileState::Conflicted => "conflicted",
            PackFileState::Deleted => "deleted",
            PackFileState::Accepted => "accepted",
        }
    }
}

/// A `PackState` is the state of one pack in the lock.
#[derive(Debug, Clone, Serialize)]
pub struct PackState {
    /// The pack's name.
    pub name: String,
    /// The version that the lock records.
    pub version: String,
    /// The source.
    pub source: String,
    /// The ref of a git source.
    pub git_ref: Option<String>,
    /// Why the pack is stale, when it is.
    pub stale: Option<String>,
    /// Each file that differs from its base, with its state.
    pub files: Vec<PackFile>,
    /// Each file that a higher pack owns, with that pack, and whether the
    /// last sync changed this pack's version of it.
    pub shadowed: Vec<ShadowedFile>,
}

/// A `PackFile` is a file of a pack that differs from its base.
#[derive(Debug, Clone, Serialize)]
pub struct PackFile {
    /// The file's destination.
    pub path: String,
    /// The file's state.
    pub state: PackFileState,
    /// The note of the file's acceptance, when it has one.
    pub note: Option<String>,
    /// Whether the file has an acceptance and no longer holds the accepted
    /// version.
    pub changed_since_acceptance: bool,
}

/// A `ShadowedFile` is a file of a pack that a higher pack owns.
#[derive(Debug, Clone, Serialize)]
pub struct ShadowedFile {
    /// The file's destination.
    pub path: String,
    /// The pack that owns it.
    pub owner: String,
    /// Whether the last sync changed this pack's version of it.
    pub changed: bool,
}

/// A `PackStatus` is the report of `pack status`.
#[derive(Debug, Clone, Serialize)]
pub struct PackStatus {
    /// Each pack in the lock, in the order of the packs, from the lowest to
    /// the highest.
    pub packs: Vec<PackState>,
}

impl PackStatus {
    /// `drifted` returns whether a file of a pack is edited, conflicted, or
    /// deleted, and doesn't hold its accepted version.
    pub fn drifted(&self) -> bool {
        self.packs
            .iter()
            .any(|p| p.files.iter().any(|f| f.state != PackFileState::Accepted))
    }

    /// `to_text` returns a line for each pack and a line for each of its
    /// files that differs from its base.
    pub fn to_text(&self) -> String {
        if self.packs.is_empty() {
            return "the lock holds no pack\n".into();
        }
        let mut out = String::new();
        for p in &self.packs {
            let source = match &p.git_ref {
                Some(r) => format!("{}@{r}", p.source),
                None => p.source.clone(),
            };
            out.push_str(&format!("{} {} from {source}", p.name, p.version));
            match &p.stale {
                Some(why) => out.push_str(&format!(": stale, {why}\n")),
                None => out.push('\n'),
            }
            // sokf-implements: CLI-13_AC-91
            // sokf-implements: CLI-13_AC-92
            for f in &p.files {
                let tail = match (&f.note, f.state) {
                    (Some(note), PackFileState::Accepted) => format!(": {note}"),
                    _ if f.changed_since_acceptance => ", changed since its acceptance".into(),
                    _ => String::new(),
                };
                out.push_str(&format!("{} {}{tail}\n", f.state.label(), f.path));
            }
            for f in &p.shadowed {
                let changed = if f.changed {
                    ", changed in this pack at the last sync"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "shadowed {} by the pack {}{changed}\n",
                    f.path, f.owner
                ));
            }
        }
        out
    }
}

/// `pack_status` reports each pack in the lock, in the order of the packs,
/// from the lowest to the highest, and each of its files that is edited,
/// conflicted, deleted, or accepted, with the accepted files after the
/// others. It reads the working tree, the configuration, and the lock only,
/// and writes no file.
// sokf-implements: CLI-13_AC-22
// sokf-implements: CLI-13_AC-23
// sokf-implements: CLI-13_AC-24
// sokf-implements: CLI-13_AC-103
pub fn pack_status(root: &RepoRoot) -> Result<PackStatus> {
    let lock = pack::read_lock(root)?;
    let tool = pack::tool_version();
    let packs = pack::ordered(root, &lock)
        .into_iter()
        .filter_map(|name| lock.packs.get(&name).map(|p| (name, p)))
        .map(|(name, p)| {
            let edited = edited_files(root, p);
            let mut files: Vec<PackFile> = p
                .files
                .iter()
                .filter(|(_, f)| edited.contains(&f.path))
                .map(|(rel, f)| {
                    let text = RepoPath::parse(&f.path)
                        .and_then(|rp| fs::read_to_string(rp.to_fs(root)).ok().map(|t| (rp, t)));
                    let held = holds_accepted(root, rel, f);
                    let state = match text {
                        _ if held => PackFileState::Accepted,
                        None => PackFileState::Deleted,
                        Some((rp, t)) if !check::conflict_lines(&rp, &t).is_empty() => {
                            PackFileState::Conflicted
                        }
                        Some(_) => PackFileState::Edited,
                    };
                    PackFile {
                        path: f.path.clone(),
                        state,
                        note: f.accepted.as_ref().map(|a| a.note.clone()),
                        changed_since_acceptance: f.accepted.is_some() && !held,
                    }
                })
                .collect();
            files.sort_by(|a, b| {
                let accepted = |f: &PackFile| f.state == PackFileState::Accepted;
                (accepted(a), &a.path).cmp(&(accepted(b), &b.path))
            });
            let builtin = p.source.starts_with(pack::BUILTIN);
            let stale = (builtin && p.version != tool).then(|| {
                format!(
                    "this is sokf {tool}; run `sokf pack sync` to sync the pack of this release"
                )
            });
            // sokf-implements: CLI-13_AC-79
            let shadowed = p
                .files
                .values()
                .filter_map(|f| {
                    let owner = f.shadowed_by.clone()?;
                    Some((f, owner))
                })
                .map(|(f, owner)| ShadowedFile {
                    owner,
                    path: f.path.clone(),
                    changed: f.changed,
                })
                .collect();
            PackState {
                name,
                version: p.version.clone(),
                source: p.source.clone(),
                git_ref: p.git_ref.clone(),
                stale,
                files,
                shadowed,
            }
        })
        .collect();
    Ok(PackStatus { packs })
}

/// A `ListedPack` is one embedded pack in the report of `pack list`.
#[derive(Debug, Clone, Serialize)]
pub struct ListedPack {
    /// The pack's name.
    pub name: String,
    /// The pack's version.
    pub version: String,
    /// The pack's description.
    pub description: String,
    /// Whether the lock holds the pack from its embedded source.
    pub locked: bool,
}

/// `list_text` returns a line for each embedded pack.
pub fn list_text(packs: &[ListedPack]) -> String {
    if packs.is_empty() {
        return "the binary embeds no pack\n".into();
    }
    packs
        .iter()
        .map(|p| {
            let mark = if p.locked { " (synced)" } else { "" };
            format!("{} {}{mark}: {}\n", p.name, p.version, p.description)
        })
        .collect()
}

/// `pack_list` returns each embedded pack, marked when the lock of `root`
/// holds it. It reads only the lock.
pub fn pack_list(root: Option<&RepoRoot>) -> Result<Vec<ListedPack>> {
    pack_list_with(root, EMBEDDED)
}

/// `pack_list_with` is `pack_list` with `embedded` as the embedded packs.
// sokf-implements: CLI-13_AC-21
pub fn pack_list_with(
    root: Option<&RepoRoot>,
    embedded: &[EmbeddedPack],
) -> Result<Vec<ListedPack>> {
    let lock = match root {
        Some(r) => pack::read_lock(r)?,
        None => pack::Lock::default(),
    };
    Ok(pack::embedded_packs(embedded)?
        .into_iter()
        .map(|m| ListedPack {
            locked: lock
                .packs
                .get(&m.name)
                .is_some_and(|p| p.source == format!("{}{}", pack::BUILTIN, m.name)),
            name: m.name,
            version: m.version,
            description: m.description,
        })
        .collect())
}

/// A `PackRemoved` is the report of `pack remove`.
#[derive(Debug, Clone, Serialize)]
pub struct PackRemoved {
    /// The pack's name.
    pub name: String,
    /// Whether the command deleted the pack's `[[pack]]` entry.
    pub undeclared: bool,
    /// Whether the lock held the pack.
    pub locked: bool,
    /// Each file that the command deleted.
    pub removed: Vec<String>,
    /// Each edited file that the command kept.
    pub kept: Vec<String>,
    /// Each pack that took over a file of the removed pack, with what the
    /// command did to each file.
    pub handed: Vec<SyncedPack>,
    /// A line for each `instructions` part of a harness that the command rewrote.
    pub instructions: Vec<String>,
}

impl PackRemoved {
    /// `conflicted` returns whether the merge of a file that the command
    /// handed to another pack conflicts.
    pub fn conflicted(&self) -> bool {
        self.handed.iter().any(|p| {
            p.files
                .iter()
                .any(|(_, a)| *a == pack::FileAction::Conflicted)
        })
    }

    /// `to_text` returns a line for each file, one for the configuration,
    /// and one for the lock.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for p in &self.removed {
            out.push_str(&format!("removed {p}\n"));
        }
        for p in &self.kept {
            out.push_str(&format!("kept {p}: edited\n"));
        }
        if self.undeclared {
            out.push_str(&format!(
                "removed the pack {} from /.sokf/config.toml\n",
                self.name
            ));
        }
        if self.locked {
            out.push_str(&format!("removed the pack {} from the lock\n", self.name));
        }
        for p in &self.handed {
            for (path, action) in &p.files {
                out.push_str(&format!(
                    "{} {path}: from the pack {}\n",
                    action.label(),
                    p.name
                ));
            }
            for note in &p.notes {
                out.push_str(&format!("{note}\n"));
            }
        }
        for line in &self.instructions {
            out.push_str(&format!("{line}\n"));
        }
        out.push_str("note: run `sokf validate --fix` to bring the store's indexes up to date\n");
        out
    }
}

/// `locked` returns the lock and the entry of the pack `name`, and refuses a
/// name that the lock doesn't hold.
fn locked(root: &RepoRoot, name: &str) -> Result<(pack::Lock, pack::LockedPack)> {
    let lock = pack::read_lock(root)?;
    let entry = lock.packs.get(name).cloned().ok_or_else(|| {
        let names: Vec<&str> = lock.packs.keys().map(String::as_str).collect();
        Error::Pack(format!(
            "the lock holds no pack named `{name}`; it holds {}",
            if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            }
        ))
    })?;
    Ok((lock, entry))
}

/// A `Removal` is what the removal of one locked pack deletes and keeps.
#[derive(Debug, Clone, Default)]
pub(super) struct Removal {
    /// The files and the base copies to delete.
    pub deletes: Vec<RepoPath>,
    /// Each file of the pack that the removal deletes and that exists.
    pub removed: Vec<String>,
    /// Each edited file that the removal keeps.
    pub kept: Vec<String>,
}

/// `plan_removal` plans the removal of the locked pack `name`: it deletes
/// each file whose hash equals its base hash, keeps each other file, and
/// deletes the base copies. The copy of the pack's instructions fragment is
/// a file of the pack like any other, so an edited copy stays. A file that
/// another pack owns, or that a pack of `supplied` takes over, stays for
/// that pack. It writes nothing.
// sokf-implements: CLI-13_AC-25
// sokf-implements: CLI-13_AC-37
// sokf-implements: CLI-13_AC-102
pub(super) fn plan_removal(
    root: &RepoRoot,
    name: &str,
    entry: &pack::LockedPack,
    supplied: &BTreeSet<RepoPath>,
) -> Removal {
    let edited = edited_files(root, entry);
    let mut out = Removal::default();
    for (rel, f) in &entry.files {
        let Some(p) = RepoPath::parse(&f.path) else {
            continue;
        };
        if f.shadowed_by.is_some() || supplied.contains(&p) {
            out.deletes.push(pack::base_path(name, rel));
            continue;
        }
        let exists = p.to_fs(root).exists();
        if edited.contains(&f.path) {
            if exists {
                out.kept.push(f.path.clone());
            }
        } else {
            if exists {
                out.removed.push(f.path.clone());
            }
            out.deletes.push(p);
        }
        out.deletes.push(pack::base_path(name, rel));
    }
    out
}

/// `apply_removal` deletes the files of `removal`, with each directory that
/// a deletion leaves empty.
pub(super) fn apply_removal(root: &RepoRoot, removal: &Removal) -> Result<()> {
    for p in &removal.deletes {
        remove(root, p)?;
    }
    Ok(())
}

/// `pack_remove` deletes the `[[pack]]` entry of the pack `name` from
/// `/.sokf/config.toml`, removes the pack as `plan_removal` says when the
/// lock holds it, hands each of its files that another declared pack
/// supplies to that pack by a merge, and drops it from the lock. It deletes
/// the lock when no pack remains. The `instructions` part of each installed
/// harness then follows the lock. It refuses a name that neither the
/// configuration nor the lock holds, and a pack that another pack of the
/// lock depends on. It uses no network. `home` is the home directory,
/// where a harness can read a setting.
// sokf-implements: CLI-13_AC-66
// sokf-implements: CLI-13_AC-72
pub fn pack_remove(root: &RepoRoot, name: &str, home: Option<PathBuf>) -> Result<PackRemoved> {
    let config_path = config::config_path();
    let text = read_optional(&config_path.to_fs(root))?;
    let config = match &text {
        Some(t) => config::remove_entry(t, name)?,
        None => None,
    };
    let lock = pack::read_lock(root)?;
    let locked = lock.packs.contains_key(name);
    if config.is_none() && !locked {
        // `locked` gives the error that lists the packs of the lock.
        self::locked(root, name)?;
    }
    let dependents = super::pack_run::dependents(&lock, name);
    if !dependents.is_empty() {
        return Err(Error::Pack(format!(
            "{name}: the pack {} depends on it; remove that pack first",
            dependents.join(", ")
        )));
    }
    let entries: Vec<config::PackEntry> = config::read_packs(root)?
        .into_iter()
        .filter(|e| e.name != name)
        .collect();
    let report = super::pack_run::run(
        root,
        &super::pack_run::Run {
            entries: &entries,
            fetching: super::pack_run::Fetching::Nothing,
            remove_only: Some(name),
            keep_existing: false,
            home: &home,
            config: config.clone(),
        },
        None,
        &[],
    )?;
    let (removed, kept) = report
        .undeclared
        .into_iter()
        .find(|u| u.name == name)
        .map(|u| (u.removed, u.kept))
        .unwrap_or_default();
    Ok(PackRemoved {
        name: name.to_string(),
        undeclared: config.is_some(),
        locked,
        removed,
        kept,
        handed: report.packs,
        instructions: report.instructions,
    })
}

/// `PackDiffOptions` holds the options of `pack diff`.
#[derive(Debug, Clone, Default)]
pub struct PackDiffOptions {
    /// The pack's name.
    pub name: String,
    /// Whether to print the edits that the project accepted too.
    pub accepted: bool,
    /// The project's files to add to the pack, as the user wrote them.
    pub new: Vec<String>,
    /// The directory from which a relative path of `new` is read.
    pub cwd: PathBuf,
    /// The home directory, where a harness can read a setting.
    pub home: Option<PathBuf>,
}

/// `pack_diff` returns the edits of the pack `options.name` as one unified
/// diff: for each edited file, the difference between its base copy and the
/// store's copy, with paths relative to the pack's directory, and with each
/// CRLF of either read as LF. The diff of a file of `files/` carries the
/// recorded and the current executable bit. A file that holds its accepted
/// version is in the diff only with `accepted`, because the patch carries
/// what goes back to the pack. After the edits come the files of `new`, each
/// as a new file at its place in the pack, as `new_files` finds it.
// sokf-implements: CLI-13_AC-30
// sokf-implements: CLI-13_AC-31
// sokf-implements: CLI-13_AC-99
// sokf-implements: CLI-13_AC-100
pub fn pack_diff(root: &RepoRoot, options: &PackDiffOptions) -> Result<String> {
    let name = options.name.as_str();
    let (lock, entry) = locked(root, name)?;
    // The new files refuse before any diff runs, so a refusal prints no
    // patch.
    let new = new_files(root, &lock, &entry, options)?;
    let edited = edited_files(root, &entry);
    let mut out = String::new();
    for (rel, f) in &entry.files {
        if !edited.contains(&f.path) || (!options.accepted && holds_accepted(root, rel, f)) {
            continue;
        }
        let base_path = pack::base_path(name, rel).to_fs(root);
        let base = fs::read(&base_path).map_err(|e| Error::io(&base_path, e))?;
        let dest = RepoPath::parse(&f.path).map(|p| p.to_fs(root));
        let current = dest.as_ref().and_then(|p| fs::read(p).ok());
        // The pack's repository holds LF, so the patch compares the texts
        // with each CRLF read as LF.
        let current = current.as_deref().map(pack::normalise);
        // A file of `files/` carries its executable bit, which the lock
        // records, so the patch carries a change of the bit too.
        // sokf-implements: CLI-13_AC-82
        let modes = dest
            .as_deref()
            .filter(|_| pack::carries_mode(rel))
            .map(|p| (f.executable, pack::file_is_executable(p)));
        out.push_str(&pack::diff_modes(
            rel,
            Some(&pack::normalise(&base)),
            current.as_deref(),
            modes,
        )?);
    }
    // sokf-implements: CLI-13_AC-104
    // sokf-implements: CLI-13_AC-105
    for n in &new {
        let modes = pack::carries_mode(&n.rel).then_some((false, n.executable));
        out.push_str(&pack::diff_modes(
            &n.rel,
            None,
            Some(&pack::normalise(&n.bytes)),
            modes,
        )?);
    }
    Ok(out)
}

/// A `NewFile` is a file of the project that `pack diff` adds to the pack.
struct NewFile {
    /// The file's path in the pack.
    rel: String,
    /// The file's bytes.
    bytes: Vec<u8>,
    /// Whether the file is executable, which only a file of `files/` keeps.
    executable: bool,
}

/// `new_files` finds the place in the pack of each file of `options.new`,
/// through the inverse of the pack's layout with the store that the lock
/// records, in the order of the arguments and once each. It refuses, with
/// every reason at once, a path outside the repository, a file that doesn't
/// exist, a file that a pack in the lock records, an `index.md` of a store,
/// a file of a store other than the pack's, and a file that sync would
/// refuse to land.
// sokf-implements: CLI-13_AC-106
fn new_files(
    root: &RepoRoot,
    lock: &pack::Lock,
    entry: &pack::LockedPack,
    options: &PackDiffOptions,
) -> Result<Vec<NewFile>> {
    if options.new.is_empty() {
        return Ok(Vec::new());
    }
    let config_roots = config::read_lenient(root)?.roots;
    let roots: Vec<&str> = config_roots.iter().map(RepoPath::as_str).collect();
    let dirs = super::pack::harness_dirs(root, &options.home);
    let protected = super::harness_packs::written_paths(&dirs)?;
    let mut refused = Vec::new();
    let mut out: Vec<NewFile> = Vec::new();
    let mut seen = BTreeSet::new();
    for arg in &options.new {
        let Some(dest) = config::PathArg(arg).resolve_new(root, &options.cwd) else {
            refused.push(format!("{arg}: not a path in the repository"));
            continue;
        };
        if !seen.insert(dest.clone()) {
            continue;
        }
        let recorded = lock.packs.iter().find_map(|(owner, p)| {
            p.files
                .values()
                .any(|f| f.path == dest.as_str())
                .then_some(owner)
        });
        if let Some(owner) = recorded {
            refused.push(format!(
                "{dest}: the pack {owner} records the file already, so the patch carries its edits"
            ));
            continue;
        }
        let rel = match pack::origin(&options.name, entry.store.as_deref(), &roots, dest.as_str()) {
            Ok(rel) => rel,
            Err(why) => {
                refused.push(format!("{dest}: {why}"));
                continue;
            }
        };
        let path = dest.to_fs(root);
        match fs::symlink_metadata(&path).map(|m| m.file_type()) {
            Ok(t) if t.is_file() => {}
            Ok(t) if t.is_symlink() => {
                refused.push(format!(
                    "{dest}: the file is a symbolic link, and a pack holds only files"
                ));
                continue;
            }
            _ => {
                refused.push(format!("{dest}: no such file"));
                continue;
            }
        }
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                refused.push(format!("{dest}: the file can't be read: {e}"));
                continue;
            }
        };
        if let Some(why) = pack::refusal(&rel, &bytes, &config_roots, &protected) {
            refused.push(format!("{dest}: sync would refuse the file: {why}"));
            continue;
        }
        let executable = pack::carries_mode(&rel) && pack::file_is_executable(&path);
        out.push(NewFile {
            rel,
            bytes,
            executable,
        });
    }
    if !refused.is_empty() {
        return Err(Error::Pack(refused.join("; ")));
    }
    Ok(out)
}

/// `pack_export` writes the embedded pack `name` to `dir` in the layout of a
/// pack, with its manifest, and returns the paths it wrote. It refuses a
/// `dir` that exists and isn't empty.
pub fn pack_export(name: &str, dir: &Path) -> Result<Vec<String>> {
    pack_export_with(name, dir, EMBEDDED)
}

/// `pack_export_with` is `pack_export` with `embedded` as the embedded
/// packs.
// sokf-implements: CLI-13_AC-26
// sokf-implements: CLI-13_AC-27
pub fn pack_export_with(name: &str, dir: &Path, embedded: &[EmbeddedPack]) -> Result<Vec<String>> {
    let found = embedded
        .iter()
        .find(|p| p.read().is_ok_and(|f| f.manifest.name == name));
    let Some(found) = found else {
        // `find_embedded` gives the error that lists the embedded packs.
        pack::find_embedded(embedded, name)?;
        return Err(Error::Internal(format!("no embedded pack {name}")));
    };
    if let Ok(mut entries) = fs::read_dir(dir)
        && entries.next().is_some()
    {
        return Err(Error::Pack(format!(
            "{}: the directory is not empty",
            dir.display()
        )));
    }
    let mut written = Vec::new();
    for (rel, text) in found.files {
        let path = dir.join(rel);
        agent_harness_adapter_core::fs::write_atomic(&path, text.as_bytes())?;
        written.push(rel.to_string());
    }
    Ok(written)
}
