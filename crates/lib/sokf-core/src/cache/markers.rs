//! This module finds the markers in files outside the knowledge roots and
//! keeps them in the store. The store holds one row for each marker or
//! fault that a scan found, so that a later run can check the file again
//! without reading it.

use rusqlite::params;

use super::{Cache, conn::Conn, sync::SyncState};
use crate::{
    FileKind, RepoPath,
    markers::{IgnoreFault, MarkerRow, RowKind},
};

/// `encode` returns the code that the store keeps for a row's kind and
/// the text of the row's two text columns.
fn encode(kind: &RowKind) -> (i64, &str, &str) {
    match kind {
        RowKind::Marker { rel, target } => (0, rel, target),
        RowKind::Malformed(tag) => (1, tag, ""),
        RowKind::Case(tag) => (2, tag, ""),
        RowKind::Ignore(IgnoreFault::BeginInside) => (3, "", ""),
        RowKind::Ignore(IgnoreFault::EndOutside) => (4, "", ""),
        RowKind::Ignore(IgnoreFault::NeverEnds) => (5, "", ""),
    }
}

/// `decode` returns the kind of a stored row, or `None` for a code that no
/// release writes.
fn decode(code: i64, rel: String, target: String) -> Option<RowKind> {
    Some(match code {
        0 => RowKind::Marker { rel, target },
        1 => RowKind::Malformed(rel),
        2 => RowKind::Case(rel),
        3 => RowKind::Ignore(IgnoreFault::BeginInside),
        4 => RowKind::Ignore(IgnoreFault::EndOutside),
        5 => RowKind::Ignore(IgnoreFault::NeverEnds),
        _ => return None,
    })
}

impl Cache {
    /// `outside_to_read` returns the files outside the knowledge roots that
    /// this run must read for markers. A file is one of them when:
    ///
    /// - it is new
    /// - its stat changed
    /// - its modification time is so close to the last sync that a change
    ///   might not show in the stat
    // sokf-implements: CACHE-1_AC-32
    pub fn outside_to_read(&self) -> Vec<RepoPath> {
        let state = self.state.borrow();
        let mut paths: Vec<RepoPath> = state
            .known
            .keys()
            .filter(|p| {
                Self::listed(&state, p).is_some_and(|(kind, root)| {
                    root.is_none() && super::sync::kind_of(kind) == FileKind::Candidate
                })
            })
            .cloned()
            .collect();
        paths.sort();
        paths
    }

    /// `unread` records a file that the scan could not read. The store keeps
    /// the file's old row, so that the next run reads the file again, for
    /// example when its permissions change and its stat does not.
    pub fn unread(&self, path: &RepoPath) {
        self.state.borrow_mut().candidates.remove(path);
    }

    /// `record_markers` records the markers that a scan of `path` found. The
    /// commit writes them in place of the rows that the store holds for the
    /// file.
    pub fn record_markers(&self, path: &RepoPath, rows: Vec<MarkerRow>) {
        self.state.borrow_mut().markers.insert(path.clone(), rows);
    }

    /// `stored_markers` returns the markers of `path` that the store keeps,
    /// in line order.
    // sokf-implements: CACHE-1_AC-33
    pub fn stored_markers(&self, path: &RepoPath) -> Vec<MarkerRow> {
        let mut out: Vec<MarkerRow> = self
            .conn()
            .rows(
                "SELECT line, kind, rel, target FROM markers WHERE path = ?1",
                [path.as_str()],
                |r| {
                    let line = r.get::<_, i64>(0)? as usize;
                    let kind = decode(r.get(1)?, r.get(2)?, r.get(3)?);
                    Ok(kind.map(|kind| MarkerRow { line, kind }))
                },
            )
            .into_iter()
            .flatten()
            .collect();
        out.sort();
        out
    }

    /// `has_markers` reports whether the store holds a row for any marker or
    /// fault in `path`.
    pub fn has_markers(&self, path: &RepoPath) -> bool {
        self.conn()
            .query_row(
                "SELECT 1 FROM markers WHERE path = ?1 LIMIT 1",
                [path.as_str()],
                |_| Ok(()),
            )
            .is_ok()
    }

    /// `commit_markers` writes the markers inside the commit's transaction.
    /// It replaces all the rows of each file that the run scanned, and it
    /// deletes the rows of each removed file.
    pub(super) fn commit_markers(&self, conn: &Conn, state: &SyncState) -> rusqlite::Result<()> {
        let mut delete = conn.prepare("DELETE FROM markers WHERE path = ?1")?;
        for path in &state.removed {
            delete.execute([path.as_str()])?;
        }
        let mut put = conn.prepare(
            "INSERT INTO markers (path, line, kind, rel, target) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (path, rows) in &state.markers {
            delete.execute([path.as_str()])?;
            for row in rows {
                let (code, rel, target) = encode(&row.kind);
                put.execute(params![path.as_str(), row.line as i64, code, rel, target])?;
            }
        }
        Ok(())
    }
}
