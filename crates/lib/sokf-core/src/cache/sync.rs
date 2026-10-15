//! This module compares the walk of a run with the store and writes the
//! results of the run back. It compares the stat that the walk took of
//! each file with the `files` table of the store, decides which files the
//! run must read, and writes the rows of the run in one transaction
//! (CACHE-1_AC-11).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::params;

use super::{Cache, conn::Conn, hint, rows::DocRow};
use crate::{FileKey, FileKind, RepoPath, Stat, Tree};

/// `RACY_MARGIN` is the time, in nanoseconds, before the last sync within
/// which the run reads a file again whatever its stat says. The run reads
/// a file again when its modification time lies within this time of the
/// last sync, because a filesystem might truncate the time to whole seconds
/// (CACHE-1_AC-1).
pub const RACY_MARGIN: i64 = 2_000_000_000;

/// `FileState` says what the sync found a file to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    /// The stat of the file equals the stat in the store, and its
    /// modification time is earlier than the last sync by at least
    /// `RACY_MARGIN`, so the run doesn't read the file.
    Unchanged,
    /// The run read the file, and its hash equals the hash in the store.
    Kept,
    /// The run read the file, and the file is new or its hash differs from
    /// the hash in the store.
    Changed,
}

/// `SyncStats` holds the counts of a run, for tests and diagnostics: what
/// the sync found, and the work that the run did against the store
/// (contract-cache CACHE-3_AC-4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncStats {
    /// The number of files that the sync judged unchanged by their stat,
    /// without reading them. With a hint, the sync judges only the files at
    /// or under the hinted paths.
    pub unchanged: usize,
    /// The number of files that the run read.
    pub read: usize,
    /// The number of files that the run read and found new, or whose hash
    /// differs from the hash in the store.
    pub changed: usize,
    /// The number of files in the store that the walk no longer lists.
    pub removed: usize,
    /// The number of statements that the run ran against the store: each
    /// run of a statement or of a batch counts as one, whatever number of
    /// rows it reads or writes. The count
    /// includes the statements of each `StoreView`, the view of the store
    /// that `Cache::view` opens for the documents that the run doesn't
    /// hold.
    pub queries: u64,
    /// The number of file rows that the sync holds in memory: one for each
    /// file that the run must read. The files that the run has already read
    /// are among these (CACHE-3_AC-2).
    pub held: usize,
}

/// `Row` is one row of the `files` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    kind: u8,
    root: Option<usize>,
    stat: Stat,
    hash: Option<u64>,
    /// The hash of the stored row of the document, when the file is a
    /// document that has one.
    pub(super) row_hash: Option<u64>,
}

/// `Listed` is one walked file as the sync keeps it: its path, its kind,
/// and its root.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Listed {
    path: RepoPath,
    kind: u8,
    root: Option<usize>,
}

/// `SyncState` holds the state of the sync of this run.
#[derive(Debug, Default)]
pub(super) struct SyncState {
    /// The generation of the store when the run synced. The store counts its
    /// commits, and a commit writes only while the count is unchanged.
    generation: i64,
    /// Every walked file, sorted by path.
    listed: Vec<Listed>,
    /// The files that the run must read, each with its row in the store
    /// when the store has one. The map holds only the rows of these files.
    pub(super) known: HashMap<RepoPath, Option<Row>>,
    /// The files that the run read, by path, each with the row that the
    /// commit writes for it.
    walked: HashMap<RepoPath, Row>,
    /// The candidate files that the walk lists. The store doesn't have
    /// their stat, because they are new or their stat changed. A run
    /// reads a candidate only when a check or a scan for markers needs it,
    /// so the store keeps only the stat of a candidate, with no hash. When the
    /// stat of a candidate differs from the stat in the store, the
    /// documents that include or link it count as changed.
    pub(super) candidates: BTreeMap<RepoPath, Stat>,
    /// The number of files that the sync judged unchanged by their stat,
    /// without reading them.
    unchanged: usize,
    /// The files that the run read, each with what the read found.
    observed: BTreeMap<RepoPath, FileState>,
    /// The rows to delete: those of the files that the walk no longer
    /// lists.
    pub(super) removed: Vec<RepoPath>,
    /// The directories that the walk lists and the store didn't, each with
    /// its root.
    pub(super) dirs_added: Vec<(RepoPath, usize)>,
    /// The directories that the store lists and the walk no longer does.
    pub(super) dirs_removed: Vec<RepoPath>,
    /// The rows of the documents that the run read.
    pub(super) rows: BTreeMap<RepoPath, DocRow>,
    /// The keys of each document that the run checked.
    pub(super) keys_of: BTreeMap<RepoPath, Vec<crate::model::KeyEntry>>,
    /// The paths that each checked document plans, which the commit writes
    /// in place of the document's rows in `plans`.
    pub(super) plans_of: BTreeMap<RepoPath, BTreeSet<RepoPath>>,
    /// The output of each check that the run ran.
    pub(super) outputs: BTreeMap<RepoPath, Vec<crate::Finding>>,
    /// The queries of each check that the run ran.
    pub(super) deps: BTreeMap<RepoPath, BTreeSet<String>>,
    /// The citations of each document that the run checked.
    pub(super) citations: BTreeMap<RepoPath, Vec<super::CitationRow>>,
    /// The edges of each document that the run checked.
    pub(super) edges: BTreeMap<RepoPath, Vec<crate::graph::RawEdge>>,
    /// Whether each path that the run probed outside the walk exists.
    pub(super) probes: BTreeMap<RepoPath, bool>,
    /// The asset pairings that the run found.
    pub(super) assets: Option<Vec<super::AssetRow>>,
    /// The schema files that the run found, by key.
    pub(super) schemas: Option<Vec<super::SchemaRow>>,
    /// The additional files that the run found, each with the schema that
    /// governs it.
    pub(super) additional: Option<Vec<super::AdditionalRow>>,
    /// The hash of the formats file, when the run compared it.
    pub(super) formats_hash: Option<u64>,
    /// The fingerprint of the registry, one hash for each format and type,
    /// when the formats file changed.
    pub(super) formats: Option<Vec<(String, u64)>>,
    /// The identities of every store, when the run compared them.
    pub(super) identities: Option<Vec<crate::model::IdentityRow>>,
    /// The markers of each file that the run scanned. They replace the rows
    /// of the file in the store.
    pub(super) markers: BTreeMap<RepoPath, Vec<crate::markers::MarkerRow>>,
}

impl Cache {
    /// `sync` compares the walk with the store to decide which files this
    /// run must read (CACHE-1_AC-1, CACHE-1_AC-24). The sync treats a file
    /// as unchanged, and doesn't read it, when two conditions hold. Its
    /// length and modification time equal those in the store. Its
    /// modification time is earlier than the last sync by at least
    /// `RACY_MARGIN`. The run reads any other file when a command needs it,
    /// and the hash of the file decides whether it changed. The walk is
    /// sorted by path, and the sync reads the rows of the store in the same
    /// order. So one pass yields the unchanged, changed, added, and removed
    /// files. The pass holds only the rows of the files that the run must
    /// read. `sync`
    /// also finds the directories that were added or removed.
    // sokf-implements: CACHE-1_AC-1
    // sokf-implements: CACHE-1_AC-24
    pub fn sync(&self, tree: &Tree) {
        let mut state = self.state.borrow_mut();
        // Each run starts from an empty state, so that nothing of an earlier
        // run on this cache leaks into it.
        *state = SyncState {
            generation: generation(self.conn()),
            listed: tree
                .files
                .iter()
                .map(|f| Listed {
                    path: f.path.clone(),
                    kind: kind_code(f.kind),
                    root: f.root,
                })
                .collect(),
            ..SyncState::default()
        };
        self.merge(tree, &mut state);
        // The sync compares the directories too, so that the run treats the
        // index of a directory that was added or removed as touched
        // (CACHE-1_AC-18).
        let known_dirs = self.stored_dirs();
        let walked_dirs: BTreeSet<&RepoPath> = tree.dirs.iter().map(|d| &d.path).collect();
        state.dirs_added = tree
            .dirs
            .iter()
            .filter(|d| !known_dirs.contains_key(&d.path))
            .map(|d| (d.path.clone(), d.root))
            .collect();
        state.dirs_removed = known_dirs
            .into_keys()
            .filter(|d| !walked_dirs.contains(d))
            .collect();
    }

    /// `merge` compares the walk with the rows of the store, both in path
    /// order. A file that is in both is unchanged when its stat equals the
    /// stat in the store, its modification time is earlier than the last
    /// sync by at least `RACY_MARGIN`, and the store has a hash for it or
    /// it is a candidate. `merge` holds no row for an unchanged file. Any
    /// other file in the walk is one that the run must read, and `merge`
    /// holds it with its row from the store. `merge` records the path of a
    /// row that the walk lacks as removed. `merge` also records each candidate that it holds, as the
    /// field `candidates` of `SyncState` describes. When a caller gave the
    /// run a hint, the list of the paths that changed, `merge` leaves the
    /// work to `merge_hinted`.
    // sokf-implements: CACHE-1_AC-26
    // sokf-implements: CACHE-1_AC-29
    fn merge(&self, tree: &Tree, state: &mut SyncState) {
        // A hint that holds a whole root gets the walk of the repository,
        // so the full merge compares it.
        if let Some(hinted) = self.hint.borrow().as_deref()
            && !self.is_fresh()
        {
            let scopes = hint::scopes(hinted);
            if !hint::covers_root(&scopes, &tree.roots) {
                self.merge_hinted(tree, scopes, state);
                return;
            }
        }
        let Ok(mut stmt) = self.conn().prepare(
            "SELECT path, kind, root, len, mtime, hash, row_hash FROM files ORDER BY path",
        ) else {
            for f in &tree.files {
                self.judge(state, f, None);
            }
            return;
        };
        let mut files = tree.files.iter().peekable();
        // The sync streams the rows of the store and compares each in
        // place: it copies a row only when the run must read its file
        // (CACHE-3_AC-2).
        if let Ok(mut rows) = stmt.query([]) {
            while let Ok(Some(r)) = rows.next() {
                let Ok(path) = r.get_ref(0).and_then(|v| Ok(v.as_str()?)) else {
                    continue;
                };
                while let Some(f) = files.next_if(|f| f.path.as_str() < path) {
                    self.judge(state, f, None);
                }
                let Some(f) = files.next_if(|f| f.path.as_str() == path) else {
                    if let Some(p) = RepoPath::parse(path) {
                        state.removed.push(p);
                    }
                    continue;
                };
                self.judge(state, f, row_of(r).ok());
            }
        }
        for f in files {
            self.judge(state, f, None);
        }
    }

    /// `judge` decides whether the run must read a walked file, given its
    /// row in the store. The file is unchanged when its stat equals the
    /// stat in the store, its modification time is earlier than the last
    /// sync by at least `RACY_MARGIN`, and the store has a hash for it or it
    /// is a candidate. `judge` counts an unchanged file, and holds any other
    /// file with its row, among the candidates too when it is one.
    fn judge(&self, state: &mut SyncState, f: &crate::WalkedFile, row: Option<Row>) {
        let unchanged = row.as_ref().is_some_and(|row| {
            (row.hash.is_some() || f.kind == FileKind::Candidate)
                && row.stat == f.stat
                && self.outside_margin(f.stat)
        });
        if unchanged {
            state.unchanged += 1;
            return;
        }
        if f.kind == FileKind::Candidate {
            state.candidates.insert(f.path.clone(), f.stat);
        }
        state.known.insert(f.path.clone(), row);
    }

    /// `merge_hinted` merges the walk with the store for a run that has a
    /// hint. The walk of such a run builds its tree from the rows of the
    /// store and lists again only each hinted path and the paths under it.
    /// Only those paths can therefore differ from the store, so
    /// `merge_hinted` compares only them (CACHE-1_AC-23).
    // sokf-implements: CACHE-1_AC-47
    // sokf-implements: CACHE-3_AC-12
    fn merge_hinted(&self, tree: &Tree, scopes: BTreeSet<RepoPath>, state: &mut SyncState) {
        let mut paths: BTreeSet<RepoPath> = BTreeSet::new();
        for scope in scopes {
            let (low, high) = hint::under(&scope);
            let start = tree
                .files
                .partition_point(|f| f.path.as_str() < low.as_str());
            paths.extend(
                tree.files[start..]
                    .iter()
                    .take_while(|f| f.path.as_str() < high.as_str())
                    .map(|f| f.path.clone()),
            );
            paths.extend(self.stored_under(&low, &high));
            paths.insert(scope);
        }
        for path in &paths {
            match (tree.file(path), self.file_row(path)) {
                (Some(f), row) => self.judge(state, f, row),
                (None, Some(_)) => state.removed.push(path.clone()),
                (None, None) => {}
            }
        }
    }

    /// `file_row` returns the row of one file in the store.
    fn file_row(&self, path: &RepoPath) -> Option<Row> {
        self.conn()
            .query_row(
                "SELECT path, kind, root, len, mtime, hash, row_hash FROM files WHERE path = ?1",
                [path.as_str()],
                row_of,
            )
            .ok()
    }

    /// `stored_under` returns the paths of the files in the store that lie
    /// between the bounds that `hint::under` returns for a directory.
    fn stored_under(&self, low: &str, high: &str) -> Vec<RepoPath> {
        self.conn()
            .rows(
                "SELECT path FROM files WHERE path >= ?1 AND path < ?2",
                [low, high],
                |r| r.get::<_, String>(0),
            )
            .iter()
            .filter_map(|p| RepoPath::parse(p))
            .collect()
    }

    /// `listed` returns the kind and the root of the walked file at `path`,
    /// found by a binary search of the sorted list.
    pub(super) fn listed(state: &SyncState, path: &RepoPath) -> Option<(u8, Option<usize>)> {
        state
            .listed
            .binary_search_by(|l| l.path.cmp(path))
            .ok()
            .map(|i| (state.listed[i].kind, state.listed[i].root))
    }

    /// `dirs_changed` returns the directories that the walk lists and the
    /// store didn't, and the directories that the store lists and the walk
    /// no longer does.
    pub fn dirs_changed(&self) -> (Vec<RepoPath>, Vec<RepoPath>) {
        let state = self.state.borrow();
        (
            state.dirs_added.iter().map(|(p, _)| p.clone()).collect(),
            state.dirs_removed.clone(),
        )
    }

    /// `must_read` says whether a run must read `path` before it can trust
    /// the hash in the store. The run must read the file when the file is
    /// new, when its stat differs from the stat in the store, when its
    /// modification time lies within `RACY_MARGIN` of the last sync, or
    /// when the walk doesn't list it.
    pub fn must_read(&self, path: &RepoPath) -> bool {
        let state = self.state.borrow();
        state.known.contains_key(path) || Self::listed(&state, path).is_none()
    }

    /// `observe` records that the run read a file. The hash in `key` decides
    /// whether the file changed, and the commit writes the row of the file.
    // sokf-implements: CACHE-1_AC-2
    // sokf-implements: CACHE-1_AC-3
    pub fn observe(&self, path: &RepoPath, key: &FileKey) -> FileState {
        // The run can read a file that the sync didn't mark as one to read,
        // such as a related document or the index of a touched directory.
        // For such a file, `observe` fetches its row from the store, for two
        // reasons. First, it can compare the hash that the run read with the
        // row. Second, the commit then writes nothing that the store already
        // has.
        let held = self.state.borrow().known.get(path).cloned();
        let known = match held {
            Some(known) => known,
            None => {
                let row = if self.is_walked(path) {
                    self.file_row(path)
                } else {
                    None
                };
                self.state
                    .borrow_mut()
                    .known
                    .insert(path.clone(), row.clone());
                row
            }
        };
        let mut state = self.state.borrow_mut();
        let found = match &known {
            Some(known) if known.hash == Some(key.hash) => FileState::Kept,
            _ => FileState::Changed,
        };
        let (kind, root) =
            Self::listed(&state, path).unwrap_or((kind_code(FileKind::Candidate), None));
        state.walked.insert(
            path.clone(),
            Row {
                kind,
                root,
                stat: key.stat(),
                hash: Some(key.hash),
                row_hash: known.and_then(|k| k.row_hash),
            },
        );
        state.observed.insert(path.clone(), found);
        found
    }

    /// `knowledge_of` returns the knowledge root that a walked file lies
    /// under, if any.
    pub fn knowledge_of(&self, path: &RepoPath) -> Option<usize> {
        let state = self.state.borrow();
        Self::listed(&state, path).and_then(|(_, root)| root)
    }

    /// `hash_of` returns the hash of a walked file: the hash that this run
    /// read, the hash in the row of the store that the run holds, or, for a
    /// file that the run didn't read, the hash that `hash_of` reads from
    /// the store.
    pub fn hash_of(&self, path: &RepoPath) -> Option<u64> {
        let state = self.state.borrow();
        if let Some(w) = state.walked.get(path) {
            return w.hash;
        }
        if let Some(known) = state.known.get(path) {
            return known.as_ref().and_then(|k| k.hash);
        }
        drop(state);
        self.stored_hash(path)
    }

    /// `is_document` says whether the walk lists `path` as a document.
    pub fn is_document(&self, path: &RepoPath) -> bool {
        let state = self.state.borrow();
        Self::listed(&state, path).is_some_and(|(kind, _)| kind == kind_code(FileKind::Document))
    }

    /// `is_schema` says whether the walk lists `path` as a schema document.
    pub fn is_schema(&self, path: &RepoPath) -> bool {
        let state = self.state.borrow();
        Self::listed(&state, path).is_some_and(|(kind, _)| kind == kind_code(FileKind::Schema))
    }

    /// `removed` returns the files that the walk no longer lists.
    pub fn removed(&self) -> Vec<RepoPath> {
        self.state.borrow().removed.clone()
    }

    /// `removed_set` returns the files that the walk no longer lists, as a
    /// set.
    pub(super) fn removed_set(&self) -> BTreeSet<RepoPath> {
        self.state.borrow().removed.iter().cloned().collect()
    }

    /// `is_new` says whether the store had no row for `path` before this
    /// run.
    pub fn is_new(&self, path: &RepoPath) -> bool {
        let state = self.state.borrow();
        match state.known.get(path) {
            Some(known) => known.is_none(),
            None => Self::listed(&state, path).is_none(),
        }
    }

    /// `pending_candidates` returns the candidate files that the walk lists.
    /// The store doesn't have their stat. Each is paired with a flag that
    /// says whether the store has no row for it yet. The documents that
    /// include or link such a file count as changed, although the run
    /// doesn't read the file (CACHE-1_AC-26). For a candidate that the run
    /// read, such as an additional file, the hash decides instead whether
    /// the file changed.
    // sokf-implements: CACHE-1_AC-26
    // sokf-implements: CACHE-1_AC-43
    pub fn pending_candidates(&self) -> Vec<(RepoPath, bool)> {
        let state = self.state.borrow();
        state
            .candidates
            .keys()
            .filter(|p| !state.observed.contains_key(*p))
            .map(|p| (p.clone(), state.known.get(p).is_some_and(Option::is_none)))
            .collect()
    }

    /// `is_walked` says whether the walk lists `path`.
    pub fn is_walked(&self, path: &RepoPath) -> bool {
        Self::listed(&self.state.borrow(), path).is_some()
    }

    /// `changed_set` returns the files that this run read and found
    /// changed, as a set.
    pub fn changed_set(&self) -> BTreeSet<RepoPath> {
        self.changed().into_iter().collect()
    }

    /// `changed` returns the files that this run read and found changed, in
    /// path order.
    pub fn changed(&self) -> Vec<RepoPath> {
        self.state
            .borrow()
            .observed
            .iter()
            .filter(|(_, s)| **s == FileState::Changed)
            .map(|(p, _)| p.clone())
            .collect()
    }

    /// `stats` returns the counts of the sync of this run, and of its work
    /// against the store.
    // sokf-implements: CACHE-3_AC-4
    pub fn stats(&self) -> SyncStats {
        let state = self.state.borrow();
        SyncStats {
            unchanged: state.unchanged,
            read: state.observed.len(),
            changed: state
                .observed
                .values()
                .filter(|s| **s == FileState::Changed)
                .count(),
            removed: state.removed.len(),
            queries: self.conn().count(),
            // Every file that the run read is in `known` too, so the rows
            // that the sync holds are the rows of `known`.
            held: state.known.len(),
        }
    }

    /// `commit` writes the result of the run in one transaction: the rows
    /// of every walked file and of the directories, the deletion of the
    /// rows of the removed files, the rows, outputs, and markers that the
    /// run produced, and the sync time. A failure is silent, and the next
    /// run reads the files again (CACHE-1_AC-9).
    // sokf-implements: CACHE-1_AC-9
    // sokf-implements: CACHE-1_AC-11
    // sokf-implements: CACHE-4_AC-1
    // sokf-implements: CACHE-4_AC-2
    pub fn commit(&self) {
        let _ = self.try_commit();
    }

    fn try_commit(&self) -> rusqlite::Result<()> {
        let mut state = self.state.borrow_mut();
        // The `files` row of a document records the hash of the stored row
        // of that document. Before it writes, the commit copies the hash of
        // each new document row into the row that the run built for the
        // file of that document.
        let SyncState { rows, walked, .. } = &mut *state;
        for (path, row) in rows.iter() {
            if let Some(w) = walked.get_mut(path) {
                w.row_hash = Some(row.hash());
            }
        }
        let conn = self.conn();
        conn.execute_batch("BEGIN IMMEDIATE")?;
        // Another run committed after this run synced, so this run's rows
        // rest on a store that no longer exists. Writing them would pair a
        // file's new stat with rows that another run derived, and no later
        // stat would show the mismatch. The run writes nothing, and the
        // next run reads what changed.
        // sokf-implements: CACHE-4_AC-3
        let next = state.generation + 1;
        if generation(conn) != state.generation {
            return conn.execute_batch("ROLLBACK");
        }
        state.generation = next;
        let state = &*state;
        let result = (|| {
            let mut delete = conn.prepare("DELETE FROM files WHERE path = ?1")?;
            for path in &state.removed {
                delete.execute([path.as_str()])?;
            }
            let mut delete_dir = conn.prepare("DELETE FROM dirs WHERE path = ?1")?;
            for path in &state.dirs_removed {
                delete_dir.execute([path.as_str()])?;
            }
            let mut put_dir =
                conn.prepare("INSERT OR REPLACE INTO dirs (path, root) VALUES (?1, ?2)")?;
            for (path, root) in &state.dirs_added {
                put_dir.execute(params![path.as_str(), *root as i64])?;
            }
            let mut upsert = conn.prepare(
                "INSERT OR REPLACE INTO files (path, kind, root, len, mtime, hash, row_hash) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            let mut put = |path: &RepoPath, row: &Row| {
                upsert.execute(params![
                    path.as_str(),
                    row.kind,
                    row.root.map(|r| r as i64),
                    row.stat.len as i64,
                    row.stat.mtime,
                    row.hash.map(|h| h as i64),
                    row.row_hash.map(|h| h as i64),
                ])
            };
            for (path, row) in &state.walked {
                if state
                    .known
                    .get(path)
                    .is_some_and(|k| k.as_ref() == Some(row))
                {
                    continue;
                }
                put(path, row)?;
            }
            // The row of a candidate that the run didn't read is its stat
            // alone.
            for (path, stat) in &state.candidates {
                if state.walked.contains_key(path) {
                    continue;
                }
                let row = Row {
                    kind: kind_code(FileKind::Candidate),
                    root: Self::listed(state, path).and_then(|(_, root)| root),
                    stat: *stat,
                    hash: None,
                    row_hash: None,
                };
                put(path, &row)?;
            }
            self.commit_rows(conn, state)?;
            self.commit_outputs(conn, state)?;
            self.commit_markers(conn, state)?;
            self.commit_plans(conn, state)?;
            conn.put_meta("synced_at", self.now())?;
            conn.put_meta("generation", next)?;
            Ok::<(), rusqlite::Error>(())
        })();
        match result {
            // The next run on this cache, such as the check after a fix,
            // compares its files with this sync. A store in memory stays
            // fresh, so that each run with `--no-cache` checks everything.
            // sokf-implements: CACHE-1_AC-51
            Ok(()) => conn.execute_batch("COMMIT").map(|()| {
                if self.file().is_some() {
                    self.set_synced_at(self.now());
                }
            }),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// `outside_margin` says whether the modification time of a stat is
    /// earlier than the last sync by at least `RACY_MARGIN`. A stat without
    /// a modification time, or a store without a sync time, is never
    /// outside the margin.
    fn outside_margin(&self, stat: Stat) -> bool {
        match (stat.mtime, self.synced_at()) {
            (Some(mtime), Some(synced)) => mtime.saturating_add(RACY_MARGIN) <= synced,
            _ => false,
        }
    }

    /// `stored_hash` returns the hash of `path` in the store.
    fn stored_hash(&self, path: &RepoPath) -> Option<u64> {
        self.conn()
            .query_row(
                "SELECT hash FROM files WHERE path = ?1",
                [path.as_str()],
                |r| r.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten()
            .map(|h| h as u64)
    }
}

/// `row_of` reads a row of the `files` table from the columns `path, kind,
/// root, len, mtime, hash, row_hash`.
fn row_of(r: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok(Row {
        kind: r.get::<_, u8>(1)?,
        root: r.get::<_, Option<i64>>(2)?.map(|v| v as usize),
        stat: Stat {
            len: r.get::<_, i64>(3)? as u64,
            mtime: r.get::<_, Option<i64>>(4)?,
        },
        hash: r.get::<_, Option<i64>>(5)?.map(|h| h as u64),
        row_hash: r.get::<_, Option<i64>>(6)?.map(|h| h as u64),
    })
}

/// `kind_code` returns the code that the store keeps for a `FileKind`.
fn kind_code(kind: FileKind) -> u8 {
    match kind {
        FileKind::Manifest => 0,
        FileKind::Index => 1,
        FileKind::Schema => 2,
        FileKind::Document => 3,
        FileKind::Candidate => 4,
    }
}

/// `kind_of` returns the `FileKind` of a code in the store. For an unknown
/// code, it returns `FileKind::Candidate`.
pub(super) fn kind_of(code: u8) -> FileKind {
    match code {
        0 => FileKind::Manifest,
        1 => FileKind::Index,
        2 => FileKind::Schema,
        3 => FileKind::Document,
        _ => FileKind::Candidate,
    }
}

/// `generation` returns the number of commits that the store records, or 0
/// for a store that records none.
fn generation(conn: &Conn) -> i64 {
    conn.meta("generation").unwrap_or(0)
}

#[cfg(test)]
mod tests;
