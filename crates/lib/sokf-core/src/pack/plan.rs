//! This module plans what a sync of one pack writes: it decides the action
//! for each file of the pack and refuses a sync that would break a rule of
//! design-pack, before any file is written (D-10, D-11, D-12, D-27).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use super::{
    eol::{is_base, keep_ending, normalise, same_text},
    git, landing,
    lock::{Lock, LockedFile, LockedPack, base_path, hash_of},
    manifest::{requires_newer, tool_version},
    mode::file_is_executable,
    owner::{Ownership, Supply},
    source::{FILES, PackFiles, Source, destination},
};
use crate::{
    Error, RepoPath, RepoRoot, Result, Source as TextSource,
    check::conflict_lines,
    model::{Document, ParseMode, SchemaDocument, parse_source},
};

/// A `Fetched` is a pack read from its source, with what the lock records
/// of the source.
#[derive(Debug, Clone)]
pub struct Fetched {
    /// The pack's manifest and files.
    pub pack: PackFiles,
    /// The source.
    pub source: Source,
    /// The ref of a git source.
    pub git_ref: Option<String>,
    /// The pack's directory in a git source.
    pub path: Option<String>,
    /// The resolved commit, or the tool's version for an embedded pack.
    pub commit: String,
    /// Whether the pack was read from its base copies and its lock entry,
    /// without its source, because this sync doesn't fetch it.
    pub offline: bool,
}

impl Fetched {
    /// `version` returns the version that the lock records for the pack. An
    /// embedded pack changes only with a release, so a fetched one has the
    /// tool's version (D-14 of design-pack), and a pack read from the lock
    /// keeps the version that the lock records.
    pub fn version(&self) -> String {
        match self.source {
            Source::Builtin(_) if !self.offline => tool_version().to_string(),
            _ => self.pack.manifest.version.clone(),
        }
    }
}

/// A `FileAction` is what a sync does to one file of a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
    /// The sync wrote a file that didn't exist.
    Added,
    /// The sync replaced an unedited file with the pack's new version.
    Updated,
    /// The sync merged the pack's change into an edited file.
    Merged,
    /// The merge conflicts, and the file holds git's conflict markers.
    Conflicted,
    /// The sync kept an existing file as an edit of the pack's version.
    KeptExisting,
    /// The sync deleted an unedited file that the pack no longer holds.
    Removed,
    /// The sync kept an edited file that the pack no longer holds, or an
    /// edited schema whose type a higher pack now owns.
    Kept,
    /// The pack changed its version of a file that a higher pack owns, so
    /// the file holds the higher pack's version.
    Overridden,
    /// The user deleted a file that the lock records, and the sync keeps it
    /// deleted, because a deletion is an edit of the store.
    Deleted,
}

impl FileAction {
    /// `label` returns the word that the report prints.
    pub fn label(self) -> &'static str {
        match self {
            FileAction::Added => "added",
            FileAction::Updated => "updated",
            FileAction::Merged => "merged",
            FileAction::Conflicted => "conflicted",
            FileAction::KeptExisting => "kept-existing",
            FileAction::Removed => "removed",
            FileAction::Kept => "kept",
            FileAction::Overridden => "overridden",
            FileAction::Deleted => "deleted",
        }
    }
}

/// A `PackPlan` is everything that a sync of one pack writes.
#[derive(Debug, Clone)]
pub struct PackPlan {
    /// The pack's name.
    pub name: String,
    /// The pack's new lock entry.
    pub entry: LockedPack,
    /// The action on each file that changes, by destination.
    pub actions: Vec<(RepoPath, FileAction)>,
    /// The files to write, with their content.
    pub writes: Vec<(RepoPath, Vec<u8>)>,
    /// The files to delete.
    pub deletes: Vec<RepoPath>,
    /// The executable bit to give each file of `files/` that the sync
    /// writes, or whose bit the pack changes.
    pub modes: Vec<(RepoPath, bool)>,
    /// A note for each file whose shadowed version the pack changed, which
    /// names the pack that owns the file.
    pub notes: Vec<String>,
}

/// A `Request` holds the options of a sync that apply to one pack.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The store that `--store` names.
    pub store: Option<RepoPath>,
    /// Whether `--keep-existing` is given.
    pub keep_existing: bool,
    /// The knowledge roots of the repository, inside which no file of
    /// `files/` lands.
    pub roots: Vec<RepoPath>,
    /// The paths that an installed harness writes, each with what writes
    /// it, on which no file of `files/` lands.
    pub protected: landing::Protected,
    /// The files that the same sync deletes before it writes, because it
    /// removes a pack that no entry declares. The planner reads each as
    /// absent.
    pub vacated: BTreeSet<RepoPath>,
}

/// `schema_type` returns the `type` const of the schema in `bytes`.
pub fn schema_type(path: &RepoPath, bytes: &[u8]) -> Option<String> {
    let source = TextSource::from_text(path.clone(), &String::from_utf8_lossy(bytes));
    let parsed = parse_source(&source, ParseMode::Document);
    SchemaDocument::new(Document::new(source, parsed)).type_const
}

/// `locked_types` returns the types of the schemas of each pack in the lock,
/// read from the base copies.
pub fn locked_types(root: &RepoRoot, lock: &Lock) -> BTreeMap<String, BTreeSet<String>> {
    lock.packs
        .iter()
        .map(|(name, pack)| {
            let types = pack
                .files
                .keys()
                .filter(|rel| rel.starts_with("schemas/"))
                .filter_map(|rel| {
                    let base = base_path(name, rel);
                    let bytes = fs::read(base.to_fs(root)).ok()?;
                    schema_type(&base, &bytes)
                })
                .collect();
            (name.clone(), types)
        })
        .collect()
}

/// `store_of` returns the store of the pack `fetched`, or `None` when the
/// pack holds no `schemas/` and no `documents/`: the store that the request
/// names, or else the store that the lock records, or else `/knowledge`. It
/// refuses a store that differs from the store that the lock records.
// sokf-implements: CLI-13_AC-56
pub fn store_of(lock: &Lock, fetched: &Fetched, request: &Request) -> Result<Option<RepoPath>> {
    let name = &fetched.pack.manifest.name;
    let fail = |m: String| Error::Pack(format!("{name}: {m}"));
    let needs_store = fetched
        .pack
        .files
        .keys()
        .any(|rel| rel.starts_with("schemas/") || rel.starts_with("documents/"));
    if !needs_store {
        return Ok(None);
    }
    let locked_store = lock.packs.get(name).and_then(|e| e.store.as_deref());
    Ok(Some(match (&request.store, locked_store) {
        (Some(s), Some(e)) if s.as_str() != e => {
            return Err(fail(format!(
                "the lock holds this pack in the store {e}, not {s}"
            )));
        }
        (Some(s), _) => s.clone(),
        (None, Some(e)) => RepoPath::parse(e)
            .ok_or_else(|| fail(format!("the lock's store `{e}` is not a path")))?,
        (None, None) => RepoPath::parse("/knowledge").expect("literal"),
    }))
}

/// `supply` returns each destination of the pack `fetched` in the store
/// `store`, with the type of each schema.
pub fn supply(fetched: &Fetched, store: Option<&RepoPath>) -> Supply {
    let name = &fetched.pack.manifest.name;
    let files = fetched
        .pack
        .files
        .iter()
        .filter_map(|(rel, bytes)| {
            let dest = RepoPath::parse(&destination(name, store.map(RepoPath::as_str), rel)?)?;
            let ty = rel
                .starts_with("schemas/")
                .then(|| schema_type(&dest, bytes))
                .flatten();
            Some((dest, ty))
        })
        .collect();
    Supply {
        store: store.cloned(),
        files,
    }
}

/// `plan` plans the sync of `fetched` into the repository. `own` says which
/// of its destinations another pack owns, and which it takes over from
/// another pack. It refuses the sync, with no
/// file written, when the pack needs a newer sokf, when the lock holds the
/// pack in another store, when another pack supplies one of its types or
/// files, or when a file lands on an existing file that the lock doesn't
/// record and `--keep-existing` is absent, or when an edited file needs a
/// merge and its base copy is missing. A pack whose source or path differs
/// from the lock's record moves to it, and keeps its bases. It also refuses a pack with a file
/// that isn't text, or a file of `files/` that lands where no pack writes,
/// as the landing module says. A pack without `schemas/` and `documents/`
/// needs no store.
// sokf-implements: CLI-13_AC-14
// sokf-implements: CLI-13_AC-15
// sokf-implements: CLI-13_AC-28
// sokf-implements: CLI-13_AC-32
// sokf-implements: CLI-13_AC-44
pub fn plan(
    root: &RepoRoot,
    lock: &Lock,
    fetched: &Fetched,
    request: &Request,
    own: &Ownership,
) -> Result<PackPlan> {
    let manifest = &fetched.pack.manifest;
    let name = manifest.name.clone();
    let fail = |m: String| Error::Pack(format!("{name}: {m}"));
    if requires_newer(&manifest.requires_sokf, tool_version()) {
        return Err(fail(format!(
            "the pack requires sokf {}, and this is sokf {}; upgrade sokf",
            manifest.requires_sokf,
            tool_version()
        )));
    }
    let existing = lock.packs.get(&name);
    let label = fetched.source.label();
    let refused = landing::check(&fetched.pack, &request.roots, &request.protected);
    if !refused.is_empty() {
        return Err(fail(refused.join("; ")));
    }
    let store = store_of(lock, fetched, request)?;
    if let Some(store) = &store {
        let has_manifest = store
            .join("manifest.sokf.yaml")
            .is_some_and(|m| m.to_fs(root).is_file());
        if !has_manifest {
            return Err(fail(format!(
                "{store} is not a knowledge store; create it with `sokf init`"
            )));
        }
    }
    let mut out = PackPlan {
        name: name.clone(),
        entry: LockedPack {
            source: label,
            git_ref: fetched.git_ref.clone(),
            path: fetched.path.clone(),
            store: store.as_ref().map(|s| s.as_str().to_string()),
            commit: fetched.commit.clone(),
            version: fetched.version(),
            depends: manifest.depends.clone(),
            files: BTreeMap::new(),
        },
        actions: Vec::new(),
        writes: Vec::new(),
        deletes: Vec::new(),
        modes: Vec::new(),
        notes: Vec::new(),
    };
    let mut unrecorded = Vec::new();
    let mut conflicted = Vec::new();
    for (rel, upstream) in &fetched.pack.files {
        let Some(dest) = destination(&name, store.as_ref().map(RepoPath::as_str), rel) else {
            continue;
        };
        let dest = RepoPath::parse(&dest).ok_or_else(|| fail(format!("{rel}: not a path")))?;
        let current = if request.vacated.contains(&dest) {
            None
        } else {
            fs::read(dest.to_fs(root)).ok()
        };
        let old = existing.and_then(|e| e.files.get(rel));
        let executable = rel.starts_with(FILES) && fetched.pack.executable.contains(rel);
        // sokf-implements: CLI-13_AC-75
        if let Some(shadow) = own.shadowed.get(&dest) {
            let changed = old.is_some_and(|f| f.base != hash_of(upstream));
            let wrote = old.is_some_and(|f| f.shadowed_by.is_none());
            // A schema whose type a higher pack now owns at another path
            // goes, unless the user edited it.
            let action = match (&current, old) {
                (Some(cur), Some(f)) if shadow.by_type && wrote => {
                    if is_base(cur, &f.base) {
                        out.deletes.push(dest.clone());
                        Some(FileAction::Removed)
                    } else {
                        Some(FileAction::Kept)
                    }
                }
                // sokf-implements: CLI-13_AC-76
                _ => changed.then(|| {
                    out.notes.push(format!(
                        "note: the pack {name} changed {dest}, which the pack {} overrides; check whether its version needs the change too",
                        shadow.owner
                    ));
                    FileAction::Overridden
                }),
            };
            if let Some(action) = action {
                out.actions.push((dest.clone(), action));
            }
            out.entry.files.insert(
                rel.clone(),
                LockedFile {
                    path: dest.as_str().to_string(),
                    base: hash_of(upstream),
                    executable,
                    shadowed_by: Some(shadow.owner.clone()),
                    changed,
                },
            );
            let base = base_path(&name, rel);
            if !fs::read(base.to_fs(root)).is_ok_and(|b| same_text(&b, upstream)) {
                out.writes.push((base, upstream.clone()));
            }
            continue;
        }
        // A record of a version that another pack shadowed until now is no
        // record of the file, which holds another pack's version or none.
        let recorded = old.filter(|f| f.shadowed_by.is_none());
        let takeover = own.takeover.get(&dest);
        // A platform without the bit neither compares nor sets it.
        // sokf-implements: CLI-13_AC-59
        let mode = (cfg!(unix) && rel.starts_with(FILES))
            .then(|| match takeover {
                Some(t) if current.is_some() => {
                    let bit = file_is_executable(&dest.to_fs(root));
                    Some(if bit == t.executable { executable } else { bit })
                }
                _ => file_mode(recorded, current.is_some(), executable, &dest, root),
            })
            .flatten();
        if (recorded.is_some() || takeover.is_some())
            && current
                .as_ref()
                .is_some_and(|c| !conflict_lines(&dest, &String::from_utf8_lossy(c)).is_empty())
        {
            conflicted.push(dest.to_string());
        }
        let action = match (recorded, current) {
            // sokf-implements: CLI-13_AC-77
            (_, Some(cur)) if let Some(t) = takeover => {
                if is_base(&cur, &t.base) {
                    if same_text(&cur, upstream) {
                        None
                    } else {
                        out.writes
                            .push((dest.clone(), keep_ending(&cur, upstream.clone())));
                        Some(FileAction::Updated)
                    }
                } else {
                    let base_copy = base_path(&t.from, &t.rel);
                    let base = fs::read(base_copy.to_fs(root)).map_err(|_| {
                        fail(format!(
                            "{dest}: the base copy {base_copy} of the pack {} is missing, so the sync can't merge this pack's version into the edited file; restore the base copy from git",
                            t.from
                        ))
                    })?;
                    let merge = git::merge_file(
                        &normalise(&cur),
                        &normalise(&base),
                        &normalise(upstream),
                        dest.as_str(),
                    )?;
                    out.writes
                        .push((dest.clone(), keep_ending(&cur, merge.text)));
                    Some(if merge.conflicted {
                        FileAction::Conflicted
                    } else {
                        FileAction::Merged
                    })
                }
            }
            // A deletion is an edit, and it outlives a change of owner too.
            // sokf-implements: CLI-13_AC-43
            (Some(_), None) => Some(FileAction::Deleted),
            (None, None) if takeover.is_some() => Some(FileAction::Deleted),
            (None, None) => {
                out.writes.push((dest.clone(), upstream.clone()));
                Some(FileAction::Added)
            }
            (None, Some(cur)) => {
                if !request.keep_existing {
                    unrecorded.push(dest.to_string());
                }
                (!same_text(&cur, upstream)).then_some(FileAction::KeptExisting)
            }
            // A file and the pack's version that differ only in their line
            // endings hold the same text (the decision "The committed bytes"
            // of design-pack).
            (Some(f), Some(cur)) if is_base(&cur, &f.base) => {
                if same_text(&cur, upstream) {
                    None
                } else {
                    out.writes
                        .push((dest.clone(), keep_ending(&cur, upstream.clone())));
                    Some(FileAction::Updated)
                }
            }
            (Some(f), Some(cur)) => {
                if hash_of(upstream) == f.base || same_text(&cur, upstream) {
                    None
                } else {
                    // sokf-implements: CLI-13_AC-44
                    let base_copy = base_path(&name, rel);
                    let base = fs::read(base_copy.to_fs(root)).map_err(|_| {
                        fail(format!(
                            "{dest}: the base copy {base_copy} is missing, so the sync can't merge the pack's change into the edited file; restore the base copy from git, or run `sokf pack remove {name}` and then `sokf pack add` with --keep-existing, which records the edited file as an edit of the pack's version"
                        ))
                    })?;
                    let merge = git::merge_file(
                        &normalise(&cur),
                        &normalise(&base),
                        &normalise(upstream),
                        dest.as_str(),
                    )?;
                    out.writes
                        .push((dest.clone(), keep_ending(&cur, merge.text)));
                    Some(if merge.conflicted {
                        FileAction::Conflicted
                    } else {
                        FileAction::Merged
                    })
                }
            }
        };
        let written = out.writes.last().is_some_and(|(p, _)| *p == dest);
        // sokf-implements: CLI-13_AC-57
        let action = match mode {
            Some(m) if written || m != file_is_executable(&dest.to_fs(root)) => {
                out.modes.push((dest.clone(), m));
                action.or(Some(FileAction::Updated))
            }
            _ => action,
        };
        if let Some(action) = action {
            out.actions.push((dest.clone(), action));
        }
        out.entry.files.insert(
            rel.clone(),
            LockedFile {
                path: dest.as_str().to_string(),
                base: hash_of(upstream),
                executable,
                shadowed_by: None,
                changed: false,
            },
        );
        // sokf-implements: CLI-13_AC-7
        let base = base_path(&name, rel);
        if !fs::read(base.to_fs(root)).is_ok_and(|b| same_text(&b, upstream)) {
            out.writes.push((base, upstream.clone()));
        }
    }
    if !conflicted.is_empty() {
        return Err(fail(format!(
            "a file of the pack holds a conflict that an earlier sync left: {}; resolve it, then sync again",
            conflicted.join(", ")
        )));
    }
    if !unrecorded.is_empty() {
        return Err(fail(format!(
            "the pack's files land on files that the lock doesn't record: {}; delete a file to take the pack's version, or pass --keep-existing to keep each as an edit of the pack's version",
            unrecorded.join(", ")
        )));
    }
    // A file that the pack no longer holds goes, unless the user edited it.
    if let Some(e) = existing {
        for (rel, f) in &e.files {
            if out.entry.files.contains_key(rel) {
                continue;
            }
            out.deletes.push(base_path(&name, rel));
            let Some(dest) = RepoPath::parse(&f.path) else {
                continue;
            };
            match fs::read(dest.to_fs(root)) {
                Ok(cur) if is_base(&cur, &f.base) => {
                    out.deletes.push(dest.clone());
                    out.actions.push((dest, FileAction::Removed));
                }
                Ok(_) => out.actions.push((dest, FileAction::Kept)),
                Err(_) => {}
            }
        }
    }
    out.actions.sort();
    Ok(out)
}

/// `file_mode` returns the executable bit that the file `dest` of `files/`
/// has after the sync, or `None` when no file is there after it. A new file
/// takes the pack's bit, `executable`. A file whose bit the lock records
/// takes the pack's bit while its bit equals the recorded one, and keeps
/// its own bit otherwise, because a change of the bit is an edit. A file
/// that the lock doesn't record keeps its bit.
fn file_mode(
    recorded: Option<&LockedFile>,
    exists: bool,
    executable: bool,
    dest: &RepoPath,
    root: &RepoRoot,
) -> Option<bool> {
    let current = file_is_executable(&dest.to_fs(root));
    match (recorded, exists) {
        (Some(_), false) => None,
        (None, false) => Some(executable),
        (Some(f), true) if f.executable == current => Some(executable),
        (_, true) => Some(current),
    }
}
