//! This module implements `sokf pack status`, `list`, `remove`, `diff`, and
//! `export` (CLI-13). None of them uses the network.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::pack::{edited_files, harness_dirs, remove, write};
use crate::{
    Error, RepoPath, RepoRoot, Result, check,
    pack::{self, EMBEDDED, EmbeddedPack},
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
}

impl PackFileState {
    /// `label` returns the word that the report prints.
    pub fn label(self) -> &'static str {
        match self {
            PackFileState::Edited => "edited",
            PackFileState::Conflicted => "conflicted",
            PackFileState::Deleted => "deleted",
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
    pub files: Vec<(String, PackFileState)>,
}

/// A `PackStatus` is the report of `pack status`.
#[derive(Debug, Clone, Serialize)]
pub struct PackStatus {
    /// Each pack in the lock, by name.
    pub packs: Vec<PackState>,
}

impl PackStatus {
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
            for (path, state) in &p.files {
                out.push_str(&format!("{} {path}\n", state.label()));
            }
        }
        out
    }
}

/// `pack_status` reports each pack in the lock and each of its files that is
/// edited, conflicted, or deleted. It reads the working tree and the lock
/// only, and writes no file.
// sokf-implements: CLI-13_AC-22
// sokf-implements: CLI-13_AC-23
// sokf-implements: CLI-13_AC-24
pub fn pack_status(root: &RepoRoot) -> Result<PackStatus> {
    let lock = pack::read_lock(root)?;
    let tool = pack::tool_version();
    let packs = lock
        .packs
        .iter()
        .map(|(name, p)| {
            let edited = edited_files(root, p);
            let files = edited
                .into_iter()
                .map(|path| {
                    let text = RepoPath::parse(&path)
                        .and_then(|rp| fs::read_to_string(rp.to_fs(root)).ok().map(|t| (rp, t)));
                    let state = match text {
                        None => PackFileState::Deleted,
                        Some((rp, t)) if !check::conflict_lines(&rp, &t).is_empty() => {
                            PackFileState::Conflicted
                        }
                        Some(_) => PackFileState::Edited,
                    };
                    (path, state)
                })
                .collect();
            let builtin = p.source.starts_with(pack::BUILTIN);
            let stale = (builtin && p.version != tool).then(|| {
                format!(
                    "this is sokf {tool}; run `sokf pack sync` to sync the pack of this release"
                )
            });
            PackState {
                name: name.clone(),
                version: p.version.clone(),
                source: p.source.clone(),
                git_ref: p.git_ref.clone(),
                stale,
                files,
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
    /// Each file that the command deleted.
    pub removed: Vec<String>,
    /// Each edited file that the command kept.
    pub kept: Vec<String>,
    /// A line for each `instructions` part of a harness that the command rewrote.
    pub instructions: Vec<String>,
}

impl PackRemoved {
    /// `to_text` returns a line for each file and one for the lock.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for p in &self.removed {
            out.push_str(&format!("removed {p}\n"));
        }
        for p in &self.kept {
            out.push_str(&format!("kept {p}: edited\n"));
        }
        out.push_str(&format!("removed the pack {} from the lock\n", self.name));
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

/// `pack_remove` deletes each file of the pack `name` whose hash equals its
/// base hash, keeps each other file, deletes the pack's base copies, and
/// removes the pack from the lock. It deletes the lock when no pack remains.
/// The copy of the pack's instructions fragment goes whatever its hash, and
/// the `instructions` part of each installed harness then follows the lock.
/// `home` is the home directory, where a harness can read a setting.
// sokf-implements: CLI-13_AC-25
// sokf-implements: CLI-13_AC-37
pub fn pack_remove(root: &RepoRoot, name: &str, home: Option<PathBuf>) -> Result<PackRemoved> {
    let (mut lock, entry) = locked(root, name)?;
    let edited = edited_files(root, &entry);
    let mut report = PackRemoved {
        name: name.to_string(),
        removed: Vec::new(),
        kept: Vec::new(),
        instructions: Vec::new(),
    };
    let fragment = pack::instructions_of(name);
    for (rel, f) in &entry.files {
        if edited.contains(&f.path) && f.path != fragment.as_str() {
            if RepoPath::parse(&f.path).is_some_and(|p| p.to_fs(root).exists()) {
                report.kept.push(f.path.clone());
            }
        } else if let Some(p) = RepoPath::parse(&f.path) {
            let existed = p.to_fs(root).exists();
            remove(root, &p)?;
            if existed {
                report.removed.push(f.path.clone());
            }
        }
        remove(root, &pack::base_path(name, rel))?;
    }
    lock.packs.remove(name);
    if lock.packs.is_empty() {
        remove(root, &pack::lock_path())?;
        // The directory of the packs goes too, when nothing is left in it.
        let _ = fs::remove_dir(root.as_path().join(pack::PACKS_DIR));
    } else {
        write(root, &pack::lock_path(), pack::lock_text(&lock)?.as_bytes())?;
    }
    report.instructions = super::harness::refresh_instructions(&harness_dirs(root, &home))?;
    Ok(report)
}

/// `pack_diff` returns the edits of the pack `name` as one unified diff: for
/// each edited file, the difference between its base copy and the store's
/// copy, with paths relative to the pack's directory, and with each CRLF of
/// either read as LF.
// sokf-implements: CLI-13_AC-30
// sokf-implements: CLI-13_AC-31
pub fn pack_diff(root: &RepoRoot, name: &str) -> Result<String> {
    let (_, entry) = locked(root, name)?;
    let edited = edited_files(root, &entry);
    let mut out = String::new();
    for (rel, f) in &entry.files {
        if !edited.contains(&f.path) {
            continue;
        }
        let base_path = pack::base_path(name, rel).to_fs(root);
        let base = fs::read(&base_path).map_err(|e| Error::io(&base_path, e))?;
        let current = RepoPath::parse(&f.path).and_then(|p| fs::read(p.to_fs(root)).ok());
        // The pack's repository holds LF, so the patch compares the texts
        // with each CRLF read as LF.
        let current = current.as_deref().map(pack::normalise);
        out.push_str(&pack::diff(
            rel,
            &pack::normalise(&base),
            current.as_deref(),
        )?);
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
