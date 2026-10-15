//! `StoreView` answers the model's queries for the documents that a run reads
//! only from the store file, through its own read-only connection to that file.
//! It skips the files that the walk no longer lists.

use std::collections::BTreeSet;

use rusqlite::{Connection, OpenFlags, params};

use super::{Cache, conn::Conn};
use crate::{
    RepoPath,
    model::{Fallback, Fields, SchemaMeta},
};

/// `StoreView` is a read-only view of the store file, for the documents that
/// the run reads only from the store. Its queries add to the cache's query counter.
#[derive(Debug)]
pub struct StoreView {
    conn: Conn,
    /// `removed` holds the files that the walk no longer lists. Their rows in
    /// the store are stale until the cache commits.
    removed: BTreeSet<RepoPath>,
}

impl Cache {
    /// `view` returns the store as a fallback for the model, or `None` when
    /// the store isn't a file or can't be opened.
    pub fn view(&self) -> Option<Box<dyn Fallback>> {
        let file = self.file()?;
        let conn = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
        conn.busy_timeout(std::time::Duration::from_secs(5)).ok()?;
        Some(Box::new(StoreView {
            conn: Conn::new(conn, self.conn().counter()),
            removed: self.removed_set(),
        }))
    }
}

impl StoreView {
    fn live(&self, path: &str) -> Option<RepoPath> {
        let p = RepoPath::parse(path)?;
        (!self.removed.contains(&p)).then_some(p)
    }
}

impl Fallback for StoreView {
    fn ids(&self, knowledge: usize, id: &str) -> Vec<RepoPath> {
        self.conn
            .rows(
                "SELECT path FROM ids WHERE store = ?1 AND id = ?2 ORDER BY path",
                params![knowledge as i64, id],
                |r| r.get::<_, String>(0),
            )
            .iter()
            .filter_map(|p| self.live(p))
            .collect()
    }

    fn ids_of_type(&self, knowledge: usize, type_name: &str) -> Vec<(RepoPath, String)> {
        self.conn
            .rows(
                "SELECT path, id FROM documents WHERE store = ?1 AND type = ?2 AND id IS NOT NULL \
                 ORDER BY path",
                params![knowledge as i64, type_name],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .into_iter()
            .filter_map(|(p, id)| Some((self.live(&p)?, id)))
            .collect()
    }

    fn is_document(&self, path: &RepoPath) -> bool {
        if self.removed.contains(path) {
            return false;
        }
        self.conn
            .query_row(
                "SELECT 1 FROM documents WHERE path = ?1",
                [path.as_str()],
                |_| Ok(()),
            )
            .is_ok()
    }

    fn fields(&self, path: &RepoPath) -> Option<Fields> {
        if self.removed.contains(path) {
            return None;
        }
        self.conn
            .query_row(
                "SELECT id, title, description FROM documents WHERE path = ?1",
                [path.as_str()],
                |r| {
                    Ok(Fields {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        description: r.get(2)?,
                    })
                },
            )
            .ok()
    }

    fn broken(&self, path: &RepoPath) -> bool {
        if self.removed.contains(path) {
            return false;
        }
        self.conn
            .query_row(
                "SELECT broken FROM documents WHERE path = ?1",
                [path.as_str()],
                |r| r.get::<_, i64>(0),
            )
            .is_ok_and(|b| b != 0)
    }

    fn schema_meta(&self, path: &RepoPath) -> Option<SchemaMeta> {
        if self.removed.contains(path) {
            return None;
        }
        type Row = (String, String, String, String, i64, String);
        let (key, globs, asset_globs, uses, line, archived): Row = self
            .conn
            .query_row(
                "SELECT key, globs, asset_globs, uses, line, archived FROM schemas WHERE path = ?1",
                [path.as_str()],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .ok()?;
        let lines = |s: String| -> Vec<String> {
            s.lines()
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        };
        Some(SchemaMeta {
            id: self.fields(path).and_then(|f| f.id),
            type_const: (!key.starts_with('/')).then_some(key),
            globs: lines(globs),
            asset_globs: lines(asset_globs),
            uses: lines(uses),
            body_line: line as usize,
            archived: crate::model::Archived::decode(&archived),
        })
    }

    fn scalars_of_type(
        &self,
        knowledge: usize,
        type_name: &str,
    ) -> Vec<(RepoPath, Vec<(String, String)>)> {
        self.conn
            .rows(
                "SELECT path, scalars FROM documents WHERE store = ?1 AND type = ?2 ORDER BY path",
                params![knowledge as i64, type_name],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .into_iter()
            .filter_map(|(p, scalars)| {
                Some((
                    self.live(&p)?,
                    crate::model::archive::decode_scalars(&scalars),
                ))
            })
            .collect()
    }

    fn broken_in(&self, knowledge: usize) -> Vec<RepoPath> {
        self.conn
            .rows(
                "SELECT path FROM documents WHERE store = ?1 AND broken = 1 ORDER BY path",
                [knowledge as i64],
                |r| r.get::<_, String>(0),
            )
            .iter()
            .filter_map(|p| self.live(p))
            .collect()
    }
}
