//! This module keeps the formatted texts in `/.sokf/cache/formatted.sqlite`,
//! a file of its own beside the store of the cache, so that a run reuses
//! the form of each text that an earlier run formatted (CACHE-2_AC-6).

use std::{collections::HashMap, time::Duration};

use rusqlite::{Connection, OptionalExtension as _, TransactionBehavior, params};

use crate::RepoRoot;

/// `FORMATTED_FILE` is the name of the file that holds the formatted texts,
/// in the cache directory.
pub const FORMATTED_FILE: &str = "formatted.sqlite";

/// `FORMS` names the meaning of a row of the file. It changed when a
/// failure of the environment stopped counting as a failure of the text
/// (issue-175), and when a hang began to record its limit (issue-180), so a
/// file whose rows have another meaning, which can hold such a failure or a
/// hang without its limit, loses them once.
const FORMS: &str = "3";

/// `TABLE` creates the table of the formatted texts: the form of each key,
/// or no form where the formatter failed, with the limit in milliseconds
/// where the failure was a hang.
const TABLE: &str =
    "CREATE TABLE IF NOT EXISTS formatted (key TEXT PRIMARY KEY, form TEXT, hang INTEGER)";

/// A `Row` is what the file records of one key: the form of its text, or
/// `None` where the formatter failed on it, and the limit that a hang
/// reached.
pub type Row = (String, Option<String>, Option<Duration>);

/// A `Store` holds the formatted texts in a SQLite file.
pub struct Store {
    /// The connection to the file.
    conn: Connection,
}

impl Store {
    /// `open` opens the file of formatted texts under `root`, and creates it
    /// with the cache directory when it is absent. It returns `None` when
    /// the file doesn't open, and the run then keeps the texts in memory.
    pub fn open(root: &RepoRoot) -> Option<Store> {
        let dir = crate::cache::ensure_dir(root).ok()?;
        let mut conn = Connection::open(dir.join(FORMATTED_FILE)).ok()?;
        conn.busy_timeout(std::time::Duration::from_secs(5)).ok()?;
        conn.pragma_update(None, "journal_mode", "WAL").ok()?;
        conn.execute_batch(&format!(
            "{TABLE}; CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)"
        ))
        .ok()?;
        if !current(&conn)? {
            // Another run can change the meaning between the read and the
            // lock, so the transaction reads it again, and a file that such
            // a run already moved to this meaning keeps its new rows.
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .ok()?;
            if !current(&tx)? {
                tx.execute_batch(&format!(
                    "DROP TABLE formatted; {TABLE}; \
                     INSERT OR REPLACE INTO meta (key, value) VALUES ('forms', '{FORMS}')"
                ))
                .ok()?;
            }
            tx.commit().ok()?;
        }
        Some(Store { conn })
    }

    /// `get` returns the form of each of `keys` that the file holds: the
    /// text, or `None` where the formatter failed on it. It leaves out a
    /// hang that reached a limit shorter than `limit`, which a run with
    /// `limit` formats again (CLI-4_AC-90, CLI-4_AC-91).
    // sokf-implements: CLI-4_AC-90
    // sokf-implements: CLI-4_AC-91
    pub fn get(&self, keys: &[String], limit: Duration) -> HashMap<String, Option<String>> {
        let mut out = HashMap::new();
        let Ok(mut stmt) = self.conn.prepare_cached(
            "SELECT form FROM formatted WHERE key = ?1 AND (hang IS NULL OR hang >= ?2)",
        ) else {
            return out;
        };
        let limit = millis(limit);
        for key in keys {
            if let Ok(Some(form)) = stmt
                .query_row(params![key, limit], |r| r.get::<_, Option<String>>(0))
                .optional()
            {
                out.insert(key.clone(), form);
            }
        }
        out
    }

    /// `put` records each row, in one transaction, with the limit of each
    /// hang (CLI-4_AC-73). A failure leaves the file as it was, and the next
    /// run formats the texts again.
    // sokf-implements: CLI-4_AC-73
    pub fn put(&mut self, rows: &[Row]) {
        let Ok(tx) = self.conn.transaction() else {
            return;
        };
        for (key, form, hang) in rows {
            let _ = tx.execute(
                "INSERT OR REPLACE INTO formatted (key, form, hang) VALUES (?1, ?2, ?3)",
                params![key, form, hang.map(millis)],
            );
        }
        let _ = tx.commit();
    }
}

/// `current` returns whether the file of `conn` records the meaning
/// `FORMS` of its rows, and `None` where the read fails.
fn current(conn: &Connection) -> Option<bool> {
    let forms: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'forms'", [], |r| {
            r.get(0)
        })
        .optional()
        .ok()?;
    Some(forms.as_deref() == Some(FORMS))
}

/// `millis` returns `limit` in whole milliseconds, and the largest number
/// that SQLite holds for a longer one.
fn millis(limit: Duration) -> i64 {
    i64::try_from(limit.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A file whose rows have an older meaning, which can hold a failure of
    // the environment (no meaning, before issue-175) or a hang without its
    // limit (meaning 2, before issue-180), loses them once, and keeps the
    // rows of this meaning with the limit of each hang.
    #[test]
    fn rows_of_another_meaning_go_once() {
        let table = "CREATE TABLE formatted (key TEXT PRIMARY KEY, form TEXT); \
             INSERT INTO formatted (key, form) VALUES ('old', NULL)";
        let meaning_2 = format!(
            "{table}; CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL); \
             INSERT INTO meta (key, value) VALUES ('forms', '2')"
        );
        for (tag, old) in [("none", table.to_string()), ("2", meaning_2)] {
            let dir = crate::test_support::fresh_dir(&format!("formatted-meaning-{tag}"));
            let root = RepoRoot::new(dir.clone());
            let path = crate::cache::ensure_dir(&root)
                .unwrap()
                .join(FORMATTED_FILE);
            Connection::open(&path)
                .unwrap()
                .execute_batch(&old)
                .unwrap();
            let key = |k: &str| vec![k.to_string()];
            let limit = Duration::from_secs(1);
            let mut store = Store::open(&root).unwrap();
            assert!(store.get(&key("old"), limit).is_empty(), "{tag}");
            store.put(&[
                ("new".into(), Some("form".into()), None),
                ("hang".into(), None, Some(limit)),
            ]);
            drop(store);
            let store = Store::open(&root).unwrap();
            assert_eq!(
                store.get(&key("new"), limit)["new"].as_deref(),
                Some("form")
            );
            assert!(store.get(&key("hang"), limit).contains_key("hang"));
            assert!(store.get(&key("hang"), limit * 2).is_empty(), "{tag}");
        }
    }

    // A hang serves a limit no longer than the one that it reached, and a
    // form or another failure serves every limit.
    // sokf-tests: CLI-4_AC-90
    // sokf-tests: CLI-4_AC-91
    #[test]
    fn a_hang_serves_only_a_limit_no_longer_than_its_own() {
        let dir = crate::test_support::fresh_dir("formatted-hang");
        let root = RepoRoot::new(dir);
        let mut store = Store::open(&root).unwrap();
        let secs = Duration::from_secs;
        store.put(&[
            ("hang".into(), None, Some(secs(10))),
            ("failed".into(), None, None),
            ("form".into(), Some("a".into()), None),
            ("huge".into(), None, Some(Duration::MAX)),
        ]);
        let keys: Vec<String> = ["hang", "failed", "form", "huge"].map(String::from).into();
        for limit in [secs(1), secs(10)] {
            assert_eq!(store.get(&keys, limit).len(), 4, "{limit:?}");
        }
        let held = store.get(&keys, Duration::from_millis(10_001));
        assert!(!held.contains_key("hang"));
        assert_eq!(held.len(), 3);
        assert_eq!(store.get(&keys, Duration::MAX).len(), 3);
    }
}
