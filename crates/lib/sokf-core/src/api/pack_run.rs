//! This module runs one sync of the packs that `/.sokf/config.toml`
//! declares, for `pack sync`, `pack add`, and `pack remove` (CLI-13). A sync
//! plans every declared pack together, because the owner of a file depends
//! on every pack: it fetches the packs that it syncs, and reads each other
//! pack from its base copies and its lock entry, without the network.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use super::{
    pack::{PackSyncReport, SyncedPack, UndeclaredPack, apply, fetch, harness_dirs, write},
    pack_info::{Removal, apply_removal, plan_removal},
};
use crate::{
    Error, RepoPath, RepoRoot, Result,
    config::{self, PackEntry},
    pack::{
        self, EmbeddedPack, Fetched, Lock, LockedPack, PackFiles, PackManifest, PackPlan, Request,
        Source,
    },
};

/// `Fetching` says which declared packs a sync fetches from their sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fetching<'a> {
    /// Every declared pack.
    All,
    /// The one declared pack of this name.
    One(&'a str),
    /// No pack: every pack comes from its base copies.
    Nothing,
}

/// A `Run` holds what one sync reads and writes beside the lock.
pub(super) struct Run<'a> {
    /// The declared packs, in the order of the configuration.
    pub entries: &'a [PackEntry],
    /// The packs that the sync fetches.
    pub fetching: Fetching<'a>,
    /// The one locked pack that no entry declares that the sync removes, or
    /// `None` for each such pack.
    pub remove_only: Option<&'a str>,
    /// Whether `--keep-existing` is given.
    pub keep_existing: bool,
    /// The home directory.
    pub home: &'a Option<PathBuf>,
    /// The text of the configuration to write with the sync, if it changes.
    pub config: Option<String>,
}

/// `run` syncs the packs of `r`. `prefetched` holds a pack that the caller
/// already fetched. It removes each locked pack that no entry declares, and
/// hands each of its files that a declared pack supplies to that pack. It
/// refuses a missing or an older dependency and a cycle of dependencies,
/// orders the packs, decides the owner of each destination and each schema
/// type, and plans every declared pack. It writes the configuration, the
/// files, and the lock only after every plan succeeds.
// sokf-implements: CLI-13_AC-62
// sokf-implements: CLI-13_AC-78
pub(super) fn run(
    root: &RepoRoot,
    r: &Run,
    mut prefetched: Option<Fetched>,
    embedded: &[EmbeddedPack],
) -> Result<PackSyncReport> {
    let lock = pack::read_lock(root)?;
    if let Fetching::One(n) = r.fetching
        && !r.entries.iter().any(|e| e.name == n)
    {
        return Err(Error::Pack(format!(
            "no [[pack]] entry of /.sokf/config.toml declares the pack {n}; add it with `sokf pack add <SOURCE>`"
        )));
    }
    if r.fetching != Fetching::Nothing && r.entries.is_empty() && lock.packs.is_empty() {
        return Err(Error::Pack(
            "no [[pack]] entry of /.sokf/config.toml declares a pack; add one with `sokf pack add <SOURCE>`"
                .into(),
        ));
    }
    let declared: BTreeSet<&str> = r.entries.iter().map(|e| e.name.as_str()).collect();
    let undeclared: Vec<String> = lock
        .packs
        .keys()
        .filter(|n| !declared.contains(n.as_str()) && r.remove_only.is_none_or(|o| o == *n))
        .cloned()
        .collect();
    let mut working = lock.clone();
    for name in &undeclared {
        working.packs.remove(name);
    }
    // The clones live until the plans are written, because a plan holds
    // only the bytes that it read from them.
    let mut packs: Vec<(&PackEntry, Fetched)> = Vec::new();
    for entry in r.entries {
        let fetch_it = match r.fetching {
            Fetching::All => true,
            Fetching::One(n) => entry.name == n,
            Fetching::Nothing => false,
        };
        let f = if fetch_it {
            match prefetched.take() {
                Some(f) if f.pack.manifest.name == entry.name => f,
                other => {
                    prefetched = other;
                    fetch(
                        &Source::parse(&entry.source),
                        entry.git_ref.as_deref(),
                        entry.path.as_deref(),
                        embedded,
                    )?
                }
            }
        } else if let Some(locked) = lock.packs.get(&entry.name) {
            from_lock(root, &entry.name, locked)?
        } else {
            // A declared pack that no sync has written yet waits for a
            // sync that fetches it.
            continue;
        };
        if f.pack.manifest.name != entry.name {
            return Err(Error::Pack(format!(
                "{}: the source {} holds the pack {}",
                entry.name,
                f.source.label(),
                f.pack.manifest.name
            )));
        }
        packs.push((entry, f));
    }
    check_depends(r.entries, &packs)?;
    let list: Vec<String> = packs.iter().map(|(e, _)| e.name.clone()).collect();
    let depends: BTreeMap<String, Vec<String>> = packs
        .iter()
        .map(|(e, f)| {
            (
                e.name.clone(),
                f.pack.manifest.depends.keys().cloned().collect(),
            )
        })
        .collect();
    let order = pack::precedence(&list, &depends)?;
    let dirs = harness_dirs(root, r.home);
    let base = Request {
        store: None,
        keep_existing: r.keep_existing,
        // A pack without a store syncs into a repository whose roots
        // don't exist yet, so the roots are read as written.
        roots: config::read_lenient(root)?.roots,
        protected: super::harness_packs::written_paths(&dirs)?,
        vacated: BTreeSet::new(),
    };
    let mut requests = BTreeMap::new();
    let mut supplies = BTreeMap::new();
    for (entry, f) in &packs {
        let store = match &entry.store {
            Some(s) => Some(RepoPath::parse(s).ok_or_else(|| {
                Error::Pack(format!("{}: the store `{s}` is not a path", entry.name))
            })?),
            None => None,
        };
        let request = Request {
            store,
            ..base.clone()
        };
        let store = pack::store_of(&working, f, &request)?;
        supplies.insert(entry.name.clone(), pack::supply(f, store.as_ref()));
        requests.insert(entry.name.clone(), request);
    }
    let owners = pack::owners(&order, &supplies, &lock);
    // A file of a removed pack that a declared pack supplies passes to that
    // pack, so the removal keeps it.
    let supplied: BTreeSet<RepoPath> = supplies
        .values()
        .flat_map(|s| s.files.iter().map(|(d, _)| d.clone()))
        .collect();
    let removals: Vec<(String, Removal)> = undeclared
        .iter()
        .map(|name| {
            let r = plan_removal(root, name, &lock.packs[name], &supplied);
            (name.clone(), r)
        })
        .collect();
    let vacated: BTreeSet<RepoPath> = removals
        .iter()
        .flat_map(|(_, r)| r.deletes.iter().cloned())
        .collect();
    let mut plans: Vec<(bool, PackPlan)> = Vec::new();
    for (entry, f) in &packs {
        let request = Request {
            vacated: vacated.clone(),
            ..requests[&entry.name].clone()
        };
        let plan = pack::plan(root, &working, f, &request, &owners[&entry.name])?;
        working.packs.insert(plan.name.clone(), plan.entry.clone());
        plans.push((!f.offline, plan));
    }
    if let Some(text) = &r.config {
        write(root, &config::config_path(), text.as_bytes())?;
    }
    for (_, removal) in &removals {
        apply_removal(root, removal)?;
    }
    let written: Vec<PackPlan> = plans.iter().map(|(_, p)| p.clone()).collect();
    apply(root, &written, &working)?;
    // The block follows the lock, so it is rewritten after the lock is written.
    let instructions = super::harness::refresh_instructions(&dirs)?;
    Ok(PackSyncReport {
        declared: None,
        undeclared: removals
            .into_iter()
            .map(|(name, r)| UndeclaredPack {
                name,
                removed: r.removed,
                kept: r.kept,
            })
            .collect(),
        instructions,
        packs: plans
            .into_iter()
            .filter(|(fetched, p)| *fetched || !p.actions.is_empty() || !p.notes.is_empty())
            .map(|(fetched, p)| SyncedPack {
                fetched,
                version: p.entry.version.clone(),
                source: p.entry.source.clone(),
                store: p.entry.store.clone(),
                files: p
                    .actions
                    .into_iter()
                    .map(|(d, a)| (d.as_str().to_string(), a))
                    .collect(),
                notes: p.notes,
                name: p.name,
            })
            .collect(),
    })
}

/// `check_depends` refuses a pack that depends on a pack that no entry of
/// `entries` declares, or on a pack whose version is older than the
/// minimum that it names. `packs` holds each pack that the sync plans.
// sokf-implements: CLI-13_AC-68
// sokf-implements: CLI-13_AC-69
fn check_depends(entries: &[PackEntry], packs: &[(&PackEntry, Fetched)]) -> Result<()> {
    for (entry, f) in packs {
        for (dep, min) in &f.pack.manifest.depends {
            if !entries.iter().any(|e| e.name == *dep) {
                return Err(Error::Pack(format!(
                    "{}: the pack depends on the pack {dep}, which no [[pack]] entry of /.sokf/config.toml declares; add it with `sokf pack add <SOURCE>`",
                    entry.name
                )));
            }
            let Some((_, d)) = packs.iter().find(|(e, _)| e.name == *dep) else {
                return Err(Error::Pack(format!(
                    "{}: the pack depends on the pack {dep}, which no sync has written yet; run `sokf pack sync`",
                    entry.name
                )));
            };
            let found = d.version();
            if pack::requires_newer(min, &found) {
                return Err(Error::Pack(format!(
                    "{}: the pack depends on {dep} {min} or newer, and {dep} is at {found}; sync a newer {dep}",
                    entry.name
                )));
            }
        }
    }
    Ok(())
}

/// `from_lock` reads the locked pack `name` from its base copies and its
/// lock entry, as the last sync fetched it, without its source.
fn from_lock(root: &RepoRoot, name: &str, locked: &LockedPack) -> Result<Fetched> {
    let mut files = BTreeMap::new();
    for rel in locked.files.keys() {
        let base = pack::base_path(name, rel);
        let bytes = fs::read(base.to_fs(root)).map_err(|_| {
            Error::Pack(format!(
                "{name}: the base copy {base} is missing; restore it from git, or sync the pack with `sokf pack sync {name}`"
            ))
        })?;
        files.insert(rel.clone(), bytes);
    }
    Ok(Fetched {
        pack: PackFiles {
            manifest: PackManifest {
                name: name.to_string(),
                version: locked.version.clone(),
                requires_sokf: "0.0.0".into(),
                description: String::new(),
                depends: locked.depends.clone(),
            },
            files,
            executable: locked
                .files
                .iter()
                .filter(|(_, f)| f.executable)
                .map(|(rel, _)| rel.clone())
                .collect(),
        },
        source: Source::parse(&locked.source),
        git_ref: locked.git_ref.clone(),
        path: locked.path.clone(),
        commit: locked.commit.clone(),
        offline: true,
    })
}

/// `dependents` returns each pack of `lock`, other than `name`, that
/// depends on the pack `name`.
pub(super) fn dependents<'a>(lock: &'a Lock, name: &str) -> Vec<&'a str> {
    lock.packs
        .iter()
        .filter(|(n, p)| *n != name && p.depends.contains_key(name))
        .map(|(n, _)| n.as_str())
        .collect()
}
