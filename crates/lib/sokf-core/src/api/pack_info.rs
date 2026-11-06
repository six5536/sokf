//! This module implements `sokf pack status`, `list`, `remove`, `diff`, and
//! `export` (CLI-13). None of them uses the network.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::pack::{SyncedPack, edited_files, remove};
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
    /// Each file that a higher pack owns, with that pack, and whether the
    /// last sync changed this pack's version of it.
    pub shadowed: Vec<ShadowedFile>,
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
    /// Each pack in the lock, by name.
    pub packs: Vec<PackState>,
}

impl PackStatus {
    /// `drifted` returns whether a file of a pack is edited, conflicted, or
    /// deleted.
    pub fn drifted(&self) -> bool {
        self.packs.iter().any(|p| !p.files.is_empty())
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
            for (path, state) in &p.files {
                out.push_str(&format!("{} {path}\n", state.label()));
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
                name: name.clone(),
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
/// deletes the base copies. The copy of the pack's instructions fragment
/// goes whatever its hash. A file that another pack owns, or that a pack of
/// `supplied` takes over, stays for that pack. It writes nothing.
// sokf-implements: CLI-13_AC-25
// sokf-implements: CLI-13_AC-37
pub(super) fn plan_removal(
    root: &RepoRoot,
    name: &str,
    entry: &pack::LockedPack,
    supplied: &BTreeSet<RepoPath>,
) -> Removal {
    let edited = edited_files(root, entry);
    let fragment = pack::instructions_of(name);
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
        if edited.contains(&f.path) && f.path != fragment.as_str() {
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
