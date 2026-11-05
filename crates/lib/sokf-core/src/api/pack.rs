//! This module implements `sokf pack sync` (CLI-13): it fetches each pack,
//! plans the sync of all of them, and writes the plans only when every one
//! succeeds.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    Error, PathArg, RepoPath, RepoRoot, Result,
    pack::{self, EMBEDDED, EmbeddedPack, Fetched, FileAction, Lock, PackPlan, Request, Source},
};

/// `PackSyncOptions` holds the options of `pack sync`.
#[derive(Debug, Clone, Default)]
pub struct PackSyncOptions {
    /// The source, or `None` to sync each pack in the lock.
    pub source: Option<String>,
    /// The branch, tag, or commit of a git source.
    pub git_ref: Option<String>,
    /// The pack's directory in a git source.
    pub path: Option<String>,
    /// The knowledge root that receives the pack.
    pub store: Option<PathBuf>,
    /// Keep each existing file that a file of the pack lands on as an edit.
    pub adopt: bool,
    /// Move a locked pack to the source.
    pub replace: bool,
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
    /// The store that holds the pack.
    pub store: String,
    /// Each file that the sync changed or kept, with what it did.
    pub files: Vec<(String, FileAction)>,
}

/// A `PackSyncReport` is the report of `pack sync`.
#[derive(Debug, Clone, Serialize)]
pub struct PackSyncReport {
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

    /// `to_text` returns the report as text lines: a line for each pack, a
    /// line for each file, and the note to run `validate --fix`.
    // sokf-implements: CLI-13_AC-10
    // sokf-implements: CLI-13_AC-12
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for p in &self.packs {
            out.push_str(&format!(
                "synced {} {} from {} into {}\n",
                p.name, p.version, p.source, p.store
            ));
            for (path, action) in &p.files {
                out.push_str(&format!("{} {path}\n", action.label()));
            }
        }
        for line in &self.instructions {
            out.push_str(&format!("{line}\n"));
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

/// `pack_sync` syncs the packs that `opts` names from their sources, with
/// the packs that the binary embeds.
pub fn pack_sync(root: &RepoRoot, opts: &PackSyncOptions) -> Result<PackSyncReport> {
    pack_sync_with(root, opts, EMBEDDED)
}

/// `pack_sync_with` is `pack_sync` with `embedded` as the embedded packs.
/// With a source, it syncs that source's pack. Without one, it syncs each
/// pack in the lock from its recorded source, ref, and path. It writes no
/// file unless the plan of every pack succeeds.
// sokf-implements: CLI-13_AC-8
// sokf-implements: CLI-13_AC-11
pub fn pack_sync_with(
    root: &RepoRoot,
    opts: &PackSyncOptions,
    embedded: &[EmbeddedPack],
) -> Result<PackSyncReport> {
    let lock = pack::read_lock(root)?;
    let store = match &opts.store {
        Some(s) => Some(
            PathArg(&s.to_string_lossy())
                .resolve(root, &opts.cwd)
                .ok_or_else(|| Error::Pack(format!("{}: no such directory", s.display())))?,
        ),
        None => None,
    };
    let request = Request {
        store,
        adopt: opts.adopt,
        replace: opts.replace,
    };
    // The clones live until the plans are written, because a plan holds
    // only the bytes that it read from them.
    let mut fetched = Vec::new();
    match &opts.source {
        Some(source) => fetched.push((
            None,
            fetch(
                &Source::parse(source),
                opts.git_ref.as_deref(),
                opts.path.as_deref(),
                embedded,
            )?,
        )),
        None => {
            if lock.packs.is_empty() {
                return Err(Error::Pack(
                    "the lock holds no pack; name a SOURCE to sync".into(),
                ));
            }
            for (name, locked) in &lock.packs {
                let source = Source::parse(&locked.source);
                let f = fetch(
                    &source,
                    locked.git_ref.as_deref(),
                    locked.path.as_deref(),
                    embedded,
                )?;
                fetched.push((Some(name.clone()), f));
            }
        }
    }
    let mut types = pack::locked_types(root, &lock);
    let mut working = lock.clone();
    let mut plans: Vec<PackPlan> = Vec::new();
    for (locked_name, f) in &fetched {
        if let Some(n) = locked_name
            && *n != f.pack.manifest.name
        {
            return Err(Error::Pack(format!(
                "{n}: the source {} now holds the pack {}",
                f.source.label(),
                f.pack.manifest.name
            )));
        }
        let request = match locked_name {
            Some(_) => Request {
                store: None,
                ..request.clone()
            },
            None => request.clone(),
        };
        let plan = pack::plan(root, &working, f, &request, &types)?;
        types.insert(plan.name.clone(), plan.types.clone());
        working.packs.insert(plan.name.clone(), plan.entry.clone());
        plans.push(plan);
    }
    apply(root, &plans, &working)?;
    // The block follows the lock, so it is rewritten after the lock is written.
    let instructions = super::harness::refresh_instructions(&harness_dirs(root, &opts.home))?;
    Ok(PackSyncReport {
        instructions,
        packs: plans
            .into_iter()
            .map(|p| SyncedPack {
                version: p.entry.version.clone(),
                source: p.entry.source.clone(),
                store: p.entry.store.clone(),
                files: p
                    .actions
                    .into_iter()
                    .map(|(d, a)| (d.as_str().to_string(), a))
                    .collect(),
                name: p.name,
            })
            .collect(),
    })
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
fn fetch(
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
            Ok(Fetched {
                pack: pack::read_dir(&dir, &label)?,
                source: source.clone(),
                git_ref: Some(clone.git_ref.clone()),
                path: path.map(str::to_string),
                commit: clone.commit.clone(),
            })
        }
    }
}

/// `apply` writes the plans and the lock.
// sokf-implements: CLI-13_AC-6
fn apply(root: &RepoRoot, plans: &[PackPlan], lock: &Lock) -> Result<()> {
    for plan in plans {
        for (path, bytes) in &plan.writes {
            write(root, path, bytes)?;
        }
        for path in &plan.deletes {
            remove(root, path)?;
        }
    }
    write(root, &pack::lock_path(), pack::lock_text(lock)?.as_bytes())
}

/// `write` writes `bytes` to the repository file `path`, through a temporary
/// file and a rename.
pub(super) fn write(root: &RepoRoot, path: &RepoPath, bytes: &[u8]) -> Result<()> {
    agent_harness_adapter_core::fs::write_atomic(&path.to_fs(root), bytes).map_err(Error::from)
}

/// `remove` deletes the repository file `path`, when it exists, and each
/// directory under `/.sokf/packs/` that it leaves empty.
pub(super) fn remove(root: &RepoRoot, path: &RepoPath) -> Result<()> {
    let fs_path = path.to_fs(root);
    match std::fs::remove_file(&fs_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(&fs_path, e)),
    }
    let packs = root.as_path().join(pack::PACKS_DIR);
    let mut dir = fs_path.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if d == packs || !d.starts_with(&packs) || std::fs::remove_dir(&d).is_err() {
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    Ok(())
}

/// `edited_files` returns the destination of each file of a locked pack
/// that is missing or whose content differs from its base, with each CRLF
/// read as LF.
pub(super) fn edited_files(root: &RepoRoot, pack: &pack::LockedPack) -> BTreeSet<String> {
    pack.files
        .values()
        .filter(|f| {
            RepoPath::parse(&f.path)
                .and_then(|p| std::fs::read(p.to_fs(root)).ok())
                .is_none_or(|bytes| !pack::is_base(&bytes, &f.base))
        })
        .map(|f| f.path.clone())
        .collect()
}
