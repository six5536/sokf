//! This module wraps the store's connection so that it counts each statement
//! it runs. A test can therefore check that a run does no more than a set
//! amount of work, rather than timing it (contract-cache CACHE-3_AC-4). A view
//! of the store shares the count of the cache that it was opened from.

use std::{cell::Cell, rc::Rc};

use rusqlite::{Connection, MappedRows, Params, Row, Statement};

/// `Conn` is a connection that counts the statements it runs.
pub(super) struct Conn {
    inner: Connection,
    count: Rc<Cell<u64>>,
}

impl Conn {
    /// `new` wraps `inner` in a `Conn` that adds each statement it runs to `count`.
    pub(super) fn new(inner: Connection, count: Rc<Cell<u64>>) -> Self {
        Conn { inner, count }
    }

    /// `counter` returns the shared counter, so that a view of the store can add
    /// its statements to the same count.
    pub(super) fn counter(&self) -> Rc<Cell<u64>> {
        Rc::clone(&self.count)
    }

    /// `count` returns the number of statements run so far, by this
    /// connection and by every view that shares its counter.
    pub(super) fn count(&self) -> u64 {
        self.count.get()
    }

    /// `reset` sets the count to zero, for this connection and for every
    /// view that shares its counter.
    pub(super) fn reset(&self) {
        self.count.set(0);
    }

    fn tick(&self) {
        self.count.set(self.count.get() + 1);
    }

    /// `prepare` prepares a statement that counts each of its runs.
    pub(super) fn prepare(&self, sql: &str) -> rusqlite::Result<Stmt<'_>> {
        Ok(Stmt {
            inner: self.inner.prepare(sql)?,
            count: &self.count,
        })
    }

    /// `query_row` runs a query that yields one row, and counts the query as one
    /// statement.
    pub(super) fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> rusqlite::Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.tick();
        self.inner.query_row(sql, params, f)
    }

    /// `rows` runs a query and returns each row that `f` maps. It skips a row
    /// that fails to map, and returns nothing when the query fails. It counts
    /// the query as one statement.
    pub(super) fn rows<T, P, F>(&self, sql: &str, params: P, f: F) -> Vec<T>
    where
        P: Params,
        F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
    {
        let Ok(mut stmt) = self.prepare(sql) else {
            return Vec::new();
        };
        stmt.query_map(params, f)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    /// `meta` returns the value of `key` in the `meta` table, parsed, or
    /// `None` when the table has no value for the key or the value doesn't
    /// parse.
    pub(super) fn meta<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .ok()
        .and_then(|v| v.parse().ok())
    }

    /// `put_meta` sets the value of `key` in the `meta` table.
    pub(super) fn put_meta(&self, key: &str, value: impl ToString) -> rusqlite::Result<usize> {
        self.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            [key, &value.to_string()],
        )
    }

    /// `execute` runs one statement and counts it.
    pub(super) fn execute<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.tick();
        self.inner.execute(sql, params)
    }

    /// `execute_batch` runs a batch of statements and counts the batch as one.
    pub(super) fn execute_batch(&self, sql: &str) -> rusqlite::Result<()> {
        self.tick();
        self.inner.execute_batch(sql)
    }
}

impl std::fmt::Debug for Conn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conn")
            .field("count", &self.count.get())
            .finish_non_exhaustive()
    }
}

/// `Stmt` is a prepared statement that counts each of its runs.
pub(super) struct Stmt<'c> {
    inner: Statement<'c>,
    count: &'c Cell<u64>,
}

impl Stmt<'_> {
    fn tick(&self) {
        self.count.set(self.count.get() + 1);
    }

    /// `query_map` runs the query and maps each row through `f`.
    pub(super) fn query_map<T, P, F>(
        &mut self,
        params: P,
        f: F,
    ) -> rusqlite::Result<MappedRows<'_, F>>
    where
        P: Params,
        F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.tick();
        self.inner.query_map(params, f)
    }

    /// `query` runs the query and returns its rows, which the caller reads one at a time.
    pub(super) fn query<P: Params>(&mut self, params: P) -> rusqlite::Result<rusqlite::Rows<'_>> {
        self.tick();
        self.inner.query(params)
    }

    /// `execute` runs the statement.
    pub(super) fn execute<P: Params>(&mut self, params: P) -> rusqlite::Result<usize> {
        self.tick();
        self.inner.execute(params)
    }

    /// `query_row` runs the query and maps its one row through `f`.
    pub(super) fn query_row<T, P, F>(&mut self, params: P, f: F) -> rusqlite::Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.tick();
        self.inner.query_row(params, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: CACHE-3_AC-4
    #[test]
    fn every_statement_counts_and_a_view_shares_the_count() {
        let conn = Conn::new(Connection::open_in_memory().unwrap(), Rc::new(Cell::new(0)));
        conn.execute_batch("CREATE TABLE t (n INTEGER)").unwrap();
        let mut put = conn.prepare("INSERT INTO t (n) VALUES (?1)").unwrap();
        for n in 0..3 {
            put.execute([n]).unwrap();
        }
        drop(put);
        let count: i64 = conn
            .query_row("SELECT count(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
        let mut get = conn.prepare("SELECT n FROM t WHERE n = ?1").unwrap();
        let rows: Vec<i64> = get
            .query_map([1], |r| r.get(0))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(rows, vec![1]);
        drop(get);
        // The six statements are the batch, the three inserts, the count, and the select.
        assert_eq!(conn.count(), 6);
        let view = Conn::new(Connection::open_in_memory().unwrap(), conn.counter());
        view.execute_batch("CREATE TABLE v (n INTEGER)").unwrap();
        assert_eq!(conn.count(), 7);
        assert!(format!("{view:?}").starts_with("Conn"));
    }
}
