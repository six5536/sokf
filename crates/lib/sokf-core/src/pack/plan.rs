//! This module plans what a sync of one pack writes: it decides the action
//! for each file of the pack and refuses a sync that would break a rule of
//! design-pack, before any file is written (D-10, D-11, D-12, D-27).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use super::{
    eol::{is_base, keep_ending, normalise, same_text},
    git,
    lock::{Lock, LockedFile, LockedPack, base_path, hash_of},
    manifest::{requires_newer, tool_version},
    source::{PackFiles, Source, destination},
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
    Adopted,
    /// The sync deleted an unedited file that the pack no longer holds.
    Removed,
    /// The sync kept an edited file that the pack no longer holds.
    Kept,
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
            FileAction::Adopted => "adopted",
            FileAction::Removed => "removed",
            FileAction::Kept => "kept",
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
    /// The types of the pack's schemas.
    pub types: BTreeSet<String>,
}

/// A `Request` holds the options of a sync that apply to one pack.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The store that `--store` names.
    pub store: Option<RepoPath>,
    /// Whether `--adopt` is given.
    pub adopt: bool,
    /// Whether `--replace` is given.
    pub replace: bool,
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

/// `plan` plans the sync of `fetched` into the repository. `types` holds the
/// types of the schemas of every other pack. It refuses the sync, with no
/// file written, when the pack needs a newer sokf, when the lock holds a pack
/// of the same name from another source or store, when `--replace` names a
/// pack that the lock doesn't hold, when another pack supplies one of its
/// types or files, or when a file lands on an existing file that the lock
/// doesn't record and `--adopt` is absent, or when an edited file needs a
/// merge and its base copy is missing.
// sokf-implements: CLI-13_AC-14
// sokf-implements: CLI-13_AC-15
// sokf-implements: CLI-13_AC-16
// sokf-implements: CLI-13_AC-17
// sokf-implements: CLI-13_AC-28
// sokf-implements: CLI-13_AC-29
// sokf-implements: CLI-13_AC-32
// sokf-implements: CLI-13_AC-44
pub fn plan(
    root: &RepoRoot,
    lock: &Lock,
    fetched: &Fetched,
    request: &Request,
    types: &BTreeMap<String, BTreeSet<String>>,
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
    match existing {
        None if request.replace => {
            return Err(fail(
                "--replace moves a pack that the lock holds, and the lock holds no pack of this name"
                    .into(),
            ));
        }
        Some(e) if !request.replace && (e.source != label || e.path != fetched.path) => {
            return Err(fail(format!(
                "the lock holds this pack from {}{}; pass --replace to move it to {label}",
                e.source,
                e.path
                    .as_ref()
                    .map(|p| format!(" at {p}"))
                    .unwrap_or_default()
            )));
        }
        _ => {}
    }
    let store = match (&request.store, existing) {
        (Some(s), Some(e)) if s.as_str() != e.store => {
            return Err(fail(format!(
                "the lock holds this pack in the store {}, not {s}",
                e.store
            )));
        }
        (Some(s), _) => s.clone(),
        (None, Some(e)) => RepoPath::parse(&e.store)
            .ok_or_else(|| fail(format!("the lock's store `{}` is not a path", e.store)))?,
        (None, None) => RepoPath::parse("/knowledge").expect("literal"),
    };
    let has_manifest = store
        .join("manifest.sokf.yaml")
        .is_some_and(|m| m.to_fs(root).is_file());
    if !has_manifest {
        return Err(fail(format!(
            "{store} is not a knowledge store; create it with `sokf init`"
        )));
    }
    let mut out = PackPlan {
        name: name.clone(),
        entry: LockedPack {
            source: label,
            git_ref: fetched.git_ref.clone(),
            path: fetched.path.clone(),
            store: store.as_str().to_string(),
            commit: fetched.commit.clone(),
            // An embedded pack changes only with a release, so its version is
            // the tool's version (D-14 of design-pack).
            version: match fetched.source {
                Source::Builtin(_) => tool_version().to_string(),
                Source::Git(_) => manifest.version.clone(),
            },
            files: BTreeMap::new(),
        },
        actions: Vec::new(),
        writes: Vec::new(),
        deletes: Vec::new(),
        types: BTreeSet::new(),
    };
    let mut unrecorded = Vec::new();
    let mut conflicted = Vec::new();
    // A store holds one schema for each type, so no two packs supply one.
    for (rel, upstream) in &fetched.pack.files {
        let Some(t) = rel
            .strip_prefix("schemas/")
            .and_then(|r| store.join("schemas")?.join(r))
            .and_then(|dest| schema_type(&dest, upstream))
        else {
            continue;
        };
        let other = types
            .iter()
            .find(|(n, ts)| **n != name && ts.contains(&t))
            .map(|(n, _)| n.clone());
        if let Some(other) = other {
            return Err(fail(format!(
                "the pack supplies a schema of the type `{t}`, and so does the pack {other}"
            )));
        }
        out.types.insert(t);
    }
    for (rel, upstream) in &fetched.pack.files {
        let Some(dest) = destination(&name, store.as_str(), rel) else {
            continue;
        };
        let dest = RepoPath::parse(&dest).ok_or_else(|| fail(format!("{rel}: not a path")))?;
        if let Some((owner, _)) = lock.owner_of(dest.as_str())
            && owner != name
        {
            return Err(fail(format!(
                "{dest}: the file belongs to the pack {owner}"
            )));
        }
        let current = fs::read(dest.to_fs(root)).ok();
        let recorded = existing.and_then(|e| e.files.get(rel));
        if recorded.is_some()
            && current
                .as_ref()
                .is_some_and(|c| !conflict_lines(&dest, &String::from_utf8_lossy(c)).is_empty())
        {
            conflicted.push(dest.to_string());
        }
        let action = match (recorded, current) {
            // sokf-implements: CLI-13_AC-43
            (Some(_), None) => Some(FileAction::Deleted),
            (None, None) => {
                out.writes.push((dest.clone(), upstream.clone()));
                Some(FileAction::Added)
            }
            (None, Some(cur)) => {
                if !request.adopt {
                    unrecorded.push(dest.to_string());
                }
                (!same_text(&cur, upstream)).then_some(FileAction::Adopted)
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
                            "{dest}: the base copy {base_copy} is missing, so the sync can't merge the pack's change into the edited file; restore the base copy from git, or run `sokf pack remove {name}` and then sync the pack again with --adopt, which records the edited file as an edit of the pack's version"
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
        if let Some(action) = action {
            out.actions.push((dest.clone(), action));
        }
        out.entry.files.insert(
            rel.clone(),
            LockedFile {
                path: dest.as_str().to_string(),
                base: hash_of(upstream),
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
            "the pack's files land on files that the lock doesn't record: {}; pass --adopt to keep each as an edit of the pack's version",
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
