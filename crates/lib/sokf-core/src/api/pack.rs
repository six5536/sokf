//! This module implements `sokf pack sync` and `sokf pack add` (CLI-13): it
//! reads the packs that `/.sokf/config.toml` declares, fetches each one,
//! plans the sync of all of them and the removal of each locked pack that
//! no entry declares, and writes the plans only when every one succeeds.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::pack_run::{Fetching, Run, run};
use crate::{
    Error, PathArg, RepoPath, RepoRoot, Result,
    config::{self, PackEntry},
    pack::{self, EMBEDDED, EmbeddedPack, Fetched, FileAction, Lock, PackPlan, Source},
    source::read_optional,
};

/// `PackSyncOptions` holds the options of `pack sync`.
#[derive(Debug, Clone, Default)]
pub struct PackSyncOptions {
    /// The name of the one declared pack to sync, or `None` for every
    /// declared pack.
    pub name: Option<String>,
    /// Keep each existing file that a file of a pack lands on as an edit.
    pub keep_existing: bool,
    /// The home directory, where a harness of the project scope can read a setting.
    pub home: Option<PathBuf>,
}

/// `PackAddOptions` holds the options of `pack add`.
#[derive(Debug, Clone, Default)]
pub struct PackAddOptions {
    /// The source: a git URL or path, `builtin:<NAME>`, or `NAME` alone.
    pub source: String,
    /// The branch, tag, or commit of a git source.
    pub git_ref: Option<String>,
    /// The pack's directory in a git source.
    pub path: Option<String>,
    /// The knowledge root that receives the pack's schemas and documents.
    pub store: Option<PathBuf>,
    /// Keep each existing file that a file of the pack lands on as an edit.
    pub keep_existing: bool,
    /// The current directory, from which a relative `store` resolves.
    pub cwd: PathBuf,
    /// The home directory, where a harness of the project scope can read a setting.
    pub home: Option<PathBuf>,
}

/// A `SyncedPack` is the report of the sync of one pack.
#[derive(Debug, Clone, Serialize)]
pub struct SyncedPack {
    /// The pack's name.
    pub name: String,
    /// The pack's version.
    pub version: String,
    /// The pack's source, as the lock records it.
    pub source: String,
    /// The store that holds the pack's schemas and documents, when it has
    /// any.
    pub store: Option<String>,
    /// Each file that the sync changed or kept, with what it did.
    pub files: Vec<(String, FileAction)>,
    /// A note for each file whose shadowed version the pack changed.
    pub notes: Vec<String>,
    /// Whether the sync fetched the pack, or read it from its base copies
    /// because only the owners of its files changed.
    pub fetched: bool,
}

/// An `UndeclaredPack` is the report of the removal of a locked pack that
/// no `[[pack]]` entry declares.
#[derive(Debug, Clone, Serialize)]
pub struct UndeclaredPack {
    /// The pack's name.
    pub name: String,
    /// Each file that the sync deleted.
    pub removed: Vec<String>,
    /// Each edited file that the sync kept.
    pub kept: Vec<String>,
}

/// A `PackSyncReport` is the report of `pack sync` and `pack add`.
#[derive(Debug, Clone, Serialize)]
pub struct PackSyncReport {
    /// The pack that `pack add` declared in the configuration.
    pub declared: Option<String>,
    /// Each locked pack that no entry declares, which the sync removed.
    pub undeclared: Vec<UndeclaredPack>,
    /// Each pack that the sync wrote.
    pub packs: Vec<SyncedPack>,
    /// A line for each `instructions` part of a harness that the sync rewrote.
    pub instructions: Vec<String>,
}

impl PackSyncReport {
    /// `conflicted` returns whether a merge of the sync conflicts.
    pub fn conflicted(&self) -> bool {
        self.packs
            .iter()
            .any(|p| p.files.iter().any(|(_, a)| *a == FileAction::Conflicted))
    }

    /// `to_text` returns the report as text lines: a line for the pack that
    /// `pack add` declared, a line for each pack that the sync removed or
    /// wrote, a line for each file, and the note to run `validate --fix`.
    // sokf-implements: CLI-13_AC-10
    // sokf-implements: CLI-13_AC-12
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        if let Some(name) = &self.declared {
            out.push_str(&format!("declared the pack {name} in /.sokf/config.toml\n"));
        }
        for u in &self.undeclared {
            out.push_str(&format!(
                "removed the pack {}, which no [[pack]] entry declares\n",
                u.name
            ));
            for p in &u.removed {
                out.push_str(&format!("removed {p}\n"));
            }
            for p in &u.kept {
                out.push_str(&format!("kept {p}: edited\n"));
            }
        }
        for p in &self.packs {
            let into = p
                .store
                .as_ref()
                .map(|s| format!(" into {s}"))
                .unwrap_or_default();
            if p.fetched {
                out.push_str(&format!(
                    "synced {} {} from {}{into}\n",
                    p.name, p.version, p.source
                ));
            } else {
                out.push_str(&format!(
                    "changed the owners of the files of {} {}\n",
                    p.name, p.version
                ));
            }
            for (path, action) in &p.files {
                out.push_str(&format!("{} {path}\n", action.label()));
            }
        }
        for line in &self.instructions {
            out.push_str(&format!("{line}\n"));
        }
        for p in &self.packs {
            for note in &p.notes {
                out.push_str(&format!("{note}\n"));
            }
        }
        if self.conflicted() {
            out.push_str(
                "error: a merge conflicts: resolve the conflict markers in each conflicted file\n",
            );
        }
        // sokf-implements: CLI-13_AC-43
        let deleted = self
            .packs
            .iter()
            .any(|p| p.files.iter().any(|(_, a)| *a == FileAction::Deleted));
        if deleted {
            out.push_str(
                "note: a deleted file is your edit, so the sync keeps it deleted; to restore it, copy its base copy from /.sokf/packs/<NAME>/base/ back\n",
            );
        }
        out.push_str("note: run `sokf validate --fix` to bring the store's indexes up to date\n");
        out
    }
}

/// `pack_sync` syncs the packs that `/.sokf/config.toml` declares, with the
/// packs that the binary embeds.
pub fn pack_sync(root: &RepoRoot, opts: &PackSyncOptions) -> Result<PackSyncReport> {
    pack_sync_with(root, opts, EMBEDDED)
}

/// `pack_sync_with` is `pack_sync` with `embedded` as the embedded packs.
/// It syncs each declared pack, or only the pack that `opts` names, from
/// the source, ref, and path of its entry, and removes each locked pack that
/// no entry declares. It writes no file unless the plan of every pack
/// succeeds.
// sokf-implements: CLI-13_AC-8
// sokf-implements: CLI-13_AC-11
// sokf-implements: CLI-13_AC-61
pub fn pack_sync_with(
    root: &RepoRoot,
    opts: &PackSyncOptions,
    embedded: &[EmbeddedPack],
) -> Result<PackSyncReport> {
    let entries = config::read_packs(root)?;
    refuse_undeclared_lock(root, &entries)?;
    run(
        root,
        &Run {
            entries: &entries,
            fetching: match opts.name.as_deref() {
                Some(n) => Fetching::One(n),
                None => Fetching::All,
            },
            remove_only: None,
            keep_existing: opts.keep_existing,
            home: &opts.home,
            config: None,
        },
        None,
        embedded,
    )
}

/// `pack_add` declares the pack of a source in `/.sokf/config.toml` and
/// syncs it, with the packs that the binary embeds.
pub fn pack_add(root: &RepoRoot, opts: &PackAddOptions) -> Result<PackSyncReport> {
    pack_add_with(root, opts, EMBEDDED)
}

/// `pack_add_with` is `pack_add` with `embedded` as the embedded packs. It
/// fetches the pack of the source, appends a `[[pack]]` entry for it at the
/// end of the list, and syncs it. It refuses a pack whose name an entry
/// already declares, and writes the configuration only with the sync.
// sokf-implements: CLI-13_AC-63
// sokf-implements: CLI-13_AC-64
pub fn pack_add_with(
    root: &RepoRoot,
    opts: &PackAddOptions,
    embedded: &[EmbeddedPack],
) -> Result<PackSyncReport> {
    let source = Source::parse(&opts.source);
    let fetched = fetch(
        &source,
        opts.git_ref.as_deref(),
        opts.path.as_deref(),
        embedded,
    )?;
    let name = fetched.pack.manifest.name.clone();
    let mut entries = config::read_packs(root)?;
    refuse_undeclared_lock(root, &entries)?;
    if entries.iter().any(|e| e.name == name) {
        return Err(Error::Pack(format!(
            "{name}: /.sokf/config.toml already declares the pack {name}; edit its [[pack]] entry to change its source"
        )));
    }
    let store = match &opts.store {
        Some(s) => Some(
            PathArg(&s.to_string_lossy())
                .resolve(root, &opts.cwd)
                .ok_or_else(|| Error::Pack(format!("{}: no such directory", s.display())))?
                .as_str()
                .to_string(),
        ),
        None => None,
    };
    let entry = PackEntry {
        name: name.clone(),
        source: source.label(),
        git_ref: opts.git_ref.clone(),
        path: fetched.path.clone(),
        store,
    };
    let text = read_optional(&config::config_path().to_fs(root))?;
    let config = config::add_entry(text.as_deref(), &entry)?;
    entries.push(entry);
    let mut report = run(
        root,
        &Run {
            entries: &entries,
            fetching: Fetching::One(&name),
            remove_only: None,
            keep_existing: opts.keep_existing,
            home: &opts.home,
            config: Some(config),
        },
        Some(fetched),
        embedded,
    )?;
    report.declared = Some(name);
    Ok(report)
}

/// `refuse_undeclared_lock` refuses a sync while no `[[pack]]` entry
/// declares a pack and the lock holds one, as a lock that an earlier release
/// wrote does, because the sync would remove every locked pack. The error
/// gives an entry for each locked pack to add to the configuration.
// sokf-implements: CLI-13_AC-81
fn refuse_undeclared_lock(root: &RepoRoot, entries: &[PackEntry]) -> Result<()> {
    let lock = pack::read_lock(root)?;
    if !entries.is_empty() || lock.packs.is_empty() {
        return Ok(());
    }
    let mut text = String::new();
    for (name, p) in &lock.packs {
        text = config::add_entry(
            Some(&text),
            &PackEntry {
                name: name.clone(),
                source: p.source.clone(),
                // An embedded pack has no ref, and a git pack follows the
                // branch that the lock records.
                git_ref: p.git_ref.clone(),
                path: p.path.clone(),
                store: p.store.clone(),
            },
        )?;
    }
    Err(Error::Pack(format!(
        "no [[pack]] entry of /.sokf/config.toml declares a pack, and the lock holds {}; add these entries to /.sokf/config.toml to keep them, or remove each with `sokf pack remove <NAME>`:\n\n{text}",
        lock.packs.keys().cloned().collect::<Vec<_>>().join(", ")
    )))
}

/// `harness_dirs` returns the directories in which `pack sync` and `pack remove` find the
/// installed harnesses.
pub(super) fn harness_dirs(root: &RepoRoot, home: &Option<PathBuf>) -> super::HarnessDirs {
    super::HarnessDirs {
        repo: Some(root.clone()),
        home: home.clone(),
    }
}

/// `fetch` reads the pack of `source`. An embedded pack takes the tool's
/// version as its commit. A git source is cloned at `git_ref` into a
/// temporary directory, and the pack is read from its directory `path`.
// sokf-implements: CLI-13_AC-18
pub(super) fn fetch(
    source: &Source,
    git_ref: Option<&str>,
    path: Option<&str>,
    embedded: &[EmbeddedPack],
) -> Result<Fetched> {
    match source {
        Source::Builtin(name) => {
            if git_ref.is_some() || path.is_some() {
                return Err(Error::Pack(format!(
                    "{}: an embedded pack takes no --ref and no --path",
                    source.label()
                )));
            }
            Ok(Fetched {
                pack: pack::find_embedded(embedded, name)?,
                source: source.clone(),
                git_ref: None,
                path: None,
                commit: pack::tool_version().to_string(),
                offline: false,
            })
        }
        Source::Git(url) => {
            let path = path
                .map(|p| p.trim_matches('/'))
                .filter(|p| !p.is_empty() && *p != ".");
            if let Some(p) = path
                && p.split('/').any(|s| s == "..")
            {
                return Err(Error::Pack(format!(
                    "--path {p}: the path leaves the repository"
                )));
            }
            let clone = pack::shallow_clone(url, git_ref)?;
            let dir = match path {
                Some(p) => pack::clone_dir(&clone.worktree(), p)?,
                None => clone.worktree(),
            };
            let label = match path {
                Some(p) => format!("{url} at {p}"),
                None => url.clone(),
            };
            let mut files = pack::read_dir(&dir, &label)?;
            // git names each file from the clone's root without `.` or an
            // empty segment, which `clone_dir` also skips in `path`.
            let prefix: String = path
                .unwrap_or_default()
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .map(|s| format!("{s}/"))
                .collect();
            files.executable = clone
                .executables()?
                .into_iter()
                .filter_map(|p| p.strip_prefix(&prefix).map(str::to_string))
                .filter(|rel| rel.starts_with(pack::FILES) && files.files.contains_key(rel))
                .collect();
            Ok(Fetched {
                pack: files,
                source: source.clone(),
                git_ref: Some(clone.git_ref.clone()),
                path: path.map(str::to_string),
                commit: clone.commit.clone(),
                offline: false,
            })
        }
    }
}

/// `apply` writes the plans and the lock.
// sokf-implements: CLI-13_AC-6
pub(super) fn apply(root: &RepoRoot, plans: &[PackPlan], lock: &Lock) -> Result<()> {
    for plan in plans {
        for (path, bytes) in &plan.writes {
            write(root, path, bytes)?;
        }
        for (path, executable) in &plan.modes {
            let fs_path = path.to_fs(root);
            pack::set_executable(&fs_path, *executable).map_err(|e| Error::io(&fs_path, e))?;
        }
        for path in &plan.deletes {
            remove(root, path)?;
        }
    }
    write_lock(root, lock)
}

/// `write_lock` writes `lock`, or deletes the lock and the directory of the
/// packs when the lock holds no pack and nothing else is left in it.
pub(super) fn write_lock(root: &RepoRoot, lock: &Lock) -> Result<()> {
    if lock.packs.is_empty() {
        remove(root, &pack::lock_path())?;
        let _ = std::fs::remove_dir(root.as_path().join(pack::PACKS_DIR));
        return Ok(());
    }
    write(root, &pack::lock_path(), pack::lock_text(lock)?.as_bytes())
}

/// `write` writes `bytes` to the repository file `path`, through a temporary
/// file and a rename.
pub(super) fn write(root: &RepoRoot, path: &RepoPath, bytes: &[u8]) -> Result<()> {
    agent_harness_adapter_core::fs::write_atomic(&path.to_fs(root), bytes).map_err(Error::from)
}

/// `remove` deletes the repository file `path`, when it exists, and each
/// directory that it leaves empty: up to `/.sokf/packs/` for a file under
/// it, up to `/.sokf/` for any other file of sokf, which stays, and up to
/// the repository root for any other file.
// sokf-implements: CLI-13_AC-60
pub(super) fn remove(root: &RepoRoot, path: &RepoPath) -> Result<()> {
    let fs_path = path.to_fs(root);
    match std::fs::remove_file(&fs_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(&fs_path, e)),
    }
    let packs = root.as_path().join(pack::PACKS_DIR);
    let sokf = root.as_path().join(".sokf");
    let stop = if fs_path.starts_with(&packs) {
        packs
    } else if fs_path.starts_with(&sokf) {
        sokf
    } else {
        root.as_path().to_path_buf()
    };
    let mut dir = fs_path.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        // A directory that isn't empty stays, and stops the climb.
        if d == stop || !d.starts_with(&stop) || std::fs::remove_dir(&d).is_err() {
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    Ok(())
}

/// `edited_files` returns the destination of each file of a locked pack
/// that is missing, whose content differs from its base, with each CRLF
/// read as LF, or, for a file of `files/`, whose executable bit differs from
/// the bit that the lock records. A shadowed file holds another pack's
/// version, so it is never this pack's edit.
// sokf-implements: CLI-13_AC-58
// sokf-implements: CLI-13_AC-80
pub(super) fn edited_files(root: &RepoRoot, pack: &pack::LockedPack) -> BTreeSet<String> {
    pack.files
        .iter()
        .filter(|(rel, f)| is_edited(root, rel, f))
        .map(|(_, f)| f.path.clone())
        .collect()
}

/// `is_edited` returns whether the file `rel` of a locked pack, whose record
/// is `f`, is one of the pack's edited files, as `edited_files` says.
pub(super) fn is_edited(root: &RepoRoot, rel: &str, f: &pack::LockedFile) -> bool {
    if f.shadowed_by.is_some() {
        return false;
    }
    let Some(path) = RepoPath::parse(&f.path).map(|p| p.to_fs(root)) else {
        return true;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return true;
    };
    !pack::is_base(&bytes, &f.base)
        || (pack::carries_mode(rel) && pack::file_is_executable(&path) != f.executable)
}

/// `holds_accepted` returns whether the file `rel` of a locked pack, whose
/// record is `f`, holds the version that its acceptance records: the
/// accepted text, with each CRLF read as LF, and the accepted executable bit
/// for a file of `files/`, or no file for an accepted deletion.
pub(super) fn holds_accepted(root: &RepoRoot, rel: &str, f: &pack::LockedFile) -> bool {
    let Some(accepted) = &f.accepted else {
        return false;
    };
    let path = RepoPath::parse(&f.path).map(|p| p.to_fs(root));
    let bytes = path.as_ref().and_then(|p| std::fs::read(p).ok());
    let bit = path
        .as_deref()
        .filter(|_| pack::carries_mode(rel) && bytes.is_some())
        .map(pack::file_is_executable);
    accepted.holds(bytes.as_deref(), bit)
}
