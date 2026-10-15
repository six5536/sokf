//! This module scans the files outside the knowledge roots for markers
//! (sokf-format-spec §9.5). It reads the files in parallel, records each
//! read in the cache, and adds to the model every file that holds a marker
//! or used to hold one.

use std::{sync::Mutex, thread};

use crate::{
    Cache, FileKey, FileKind, MarkerCheck, Model, RepoPath, RepoRoot, Stat,
    graph::{EdgeKind, NodeKey, RawEdge},
    markers::{self, MarkerRow, RowKind},
    model::KeyIndex,
    source,
};

/// `Read` holds the result of reading one file: its key and its marker
/// rows, which are empty for a binary file.
type Read = (FileKey, Vec<MarkerRow>);

/// `scan_outside` scans the files outside the knowledge roots: every one
/// in a full run, and otherwise the ones that the sync must read. When the
/// scan of a changed file finds rows, or finds none where the cache
/// stored some, `scan_outside` adds the file to the model and records its rows.
/// In a full run, an unchanged file counts as changed. `scan_outside`
/// returns the paths it added, and the check runs on those files.
// sokf-implements: SOKF_VALIDATION_50
// sokf-implements: CACHE-1_AC-32
pub(super) fn scan_outside(
    model: &mut Model,
    cache: &Cache,
    root: &RepoRoot,
    full: bool,
) -> Vec<RepoPath> {
    // sokf-implements: CONF-7_AC-3
    if model.marker_check == MarkerCheck::Off {
        return Vec::new();
    }
    let paths: Vec<RepoPath> = if full {
        outside_files(model)
    } else {
        cache.outside_to_read()
    };
    let fresh = cache.is_fresh();
    let mut added = Vec::new();
    for (path, read) in read_all(root, paths) {
        let Some((key, rows)) = read else {
            cache.unread(&path);
            continue;
        };
        let state = cache.observe(&path, &key);
        // An unchanged file keeps its rows and its output, but a full run
        // still checks it against the rows that this scan found.
        if !full && state != crate::cache::FileState::Changed {
            continue;
        }
        if rows.is_empty() && (fresh || !cache.has_markers(&path)) {
            continue;
        }
        cache.record_markers(&path, rows.clone());
        model.markers.insert(path.clone(), rows);
        added.push(path);
    }
    added
}

/// `outside_files` returns the walked files outside the knowledge roots,
/// the files that hold markers.
fn outside_files(model: &Model) -> Vec<RepoPath> {
    model
        .tree
        .files
        .iter()
        .filter(|f| f.root.is_none() && f.kind == FileKind::Candidate)
        .map(|f| f.path.clone())
        .collect()
}

/// `marker_edges` reads every file outside the knowledge roots for
/// markers, and returns the edge of each marker that resolves: from the
/// file to the document of its reference, with the slug of the reference's
/// fragment as the region and the marker's line. `graph` calls it with
/// `--no-cache`, when no store holds the markers.
// sokf-implements: CLI-7_AC-90
// sokf-implements: CLI-7_AC-95
pub(super) fn marker_edges(model: &Model, keys: &KeyIndex, root: &RepoRoot) -> Vec<RawEdge> {
    if model.marker_check == MarkerCheck::Off {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (path, read) in read_all(root, outside_files(model)) {
        let Some((_, rows)) = read else {
            continue;
        };
        for row in rows {
            let RowKind::Marker { rel, target } = row.kind else {
                continue;
            };
            if let Ok(r) = markers::resolve(model, keys, &target) {
                out.push(RawEdge {
                    from: path.clone(),
                    kind: EdgeKind::Marker,
                    written: r.path.as_str().to_string(),
                    to: NodeKey::Path(r.path),
                    rel,
                    region: r.slug,
                    line: Some(row.line),
                    note: None,
                });
            }
        }
    }
    out
}

/// `is_checked_unit` returns whether `path` is a unit for which a check
/// produces output: a document, a schema document, an additional file, or
/// a file outside the roots whose markers the cache keeps (CACHE-1_AC-33).
pub(super) fn is_checked_unit(model: &Model, cache: &Cache, path: &RepoPath) -> bool {
    model.is_unit(path)
        || model.markers.contains_key(path)
        || (model.is_marker_file(path) && cache.has_markers(path))
}

/// `read_all` reads and scans `paths` on as many threads as the machine
/// can run in parallel, but no more than one thread for every 64 paths. It returns the
/// results in path order, with `None` for a file that it can't read.
fn read_all(root: &RepoRoot, paths: Vec<RepoPath>) -> Vec<(RepoPath, Option<Read>)> {
    let threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(paths.len().div_ceil(64).max(1));
    let chunk = paths.len().div_ceil(threads).max(1);
    let results: Mutex<Vec<(RepoPath, Option<Read>)>> = Mutex::new(Vec::with_capacity(paths.len()));
    thread::scope(|scope| {
        for part in paths.chunks(chunk) {
            let results = &results;
            scope.spawn(move || {
                let found: Vec<(RepoPath, Option<Read>)> = part
                    .iter()
                    .map(|p| (p.clone(), read_one(root, p)))
                    .collect();
                results
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .extend(found);
            });
        }
    });
    let mut results = results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// `read_one` reads one file, builds its key the same way as the key of a
/// source, and scans it for marker rows. It returns `None` when it can't
/// read the file.
fn read_one(root: &RepoRoot, path: &RepoPath) -> Option<Read> {
    let fs_path = path.to_fs(root);
    let bytes = std::fs::read(&fs_path).ok()?;
    let mtime = std::fs::metadata(&fs_path)
        .ok()
        .and_then(|m| Stat::of(&m).mtime);
    let key = FileKey {
        len: bytes.len() as u64,
        mtime,
        hash: source::fnv1a64(&bytes),
    };
    let rows = markers::scan(&bytes).unwrap_or_default();
    Some((key, rows))
}
