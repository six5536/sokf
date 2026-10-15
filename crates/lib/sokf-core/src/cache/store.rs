//! This module opens the store. The store is one SQLite file under
//! `/.sokf/cache/`, and with `--no-cache` the module creates the same tables
//! in memory instead. When the store file does not open, the module uses a
//! store in memory. When the store is absent or has another version or
//! format, the module builds it again. Neither case prints a message
//! (CACHE-1_AC-7).

use std::{
    cell::{Cell, RefCell},
    fs,
    path::PathBuf,
    rc::Rc,
    time::UNIX_EPOCH,
};

use rusqlite::Connection;

use super::{conn::Conn, sync::SyncState};
use crate::{RepoPath, RepoRoot};

// sokf:begin cache-dir
/// `CACHE_DIR` is the cache directory, relative to the repository root.
pub const CACHE_DIR: &str = ".sokf/cache";
// sokf:end cache-dir

/// `STORE_FILE` is the name of the store file inside the cache directory.
pub const STORE_FILE: &str = "sokf.sqlite";

/// `STORE_FORMAT` is the version of the store's tables. Raise it whenever a
/// table changes. A change to what a check finds needs no raise, because the
/// fingerprint of the build in the version changes with the code
/// (CACHE-1_AC-7).
pub const STORE_FORMAT: u32 = 20;

// sokf:begin cache
/// `CacheMode` sets whether a run reads and writes the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// `On` reads and writes the cache.
    On,
    /// `Off` neither reads nor writes the cache (`--no-cache`): the run uses
    /// the same tables, in memory.
    Off,
}

/// `Cache` holds the store and the state of this run's sync.
pub struct Cache {
    /// `conn` is the store's connection. It counts the statements that the
    /// run makes.
    conn: Conn,
    /// `file` is the path of the store file, or `None` when the store is in
    /// memory.
    file: Option<PathBuf>,
    mode: CacheMode,
    /// `synced_at` is the start of the last run's sync, as the store recorded
    /// it, or `None` for a new store. A commit of this cache sets it, so that
    /// a second run on the same cache sees the first.
    synced_at: Cell<Option<i64>>,
    /// `now` is the start of this run, in nanoseconds since the Unix epoch.
    now: Cell<i64>,
    pub(super) state: RefCell<SyncState>,
    /// `hint` holds the paths that a caller reported as changed. The sync
    /// checks only these paths on disk.
    pub(super) hint: RefCell<Option<Vec<RepoPath>>>,
    /// `missed` records that this run, with a hint, read a file that it took
    /// from the store and found the file removed.
    pub(super) missed: Cell<bool>,
}

impl Cache {
    /// `open` opens the store under `root`, or opens it in memory for `Off`.
    /// When the store file does not open, the run uses a store in memory. When
    /// the store is of another version, format, or configuration, `open`
    /// replaces it with an empty one. Neither case prints a message.
    // sokf-implements: CACHE-2_AC-4
    // sokf-implements: CACHE-1_AC-6
    // sokf-implements: CACHE-1_AC-7
    // sokf-implements: CACHE-1_AC-8
    // sokf-implements: CACHE-1_AC-10
    // sokf-implements: CLI-1_AC-13
    pub fn open(root: &RepoRoot, mode: CacheMode) -> Cache {
        let now = now();
        let version = version(root);
        let (conn, file) = match mode {
            CacheMode::On => match open_file(root) {
                Some(conn) => (conn, Some(root.as_path().join(CACHE_DIR).join(STORE_FILE))),
                None => (memory(), None),
            },
            CacheMode::Off => (memory(), None),
        };
        let (conn, synced_at, file) = match ensure_schema(conn, &version) {
            Ok((conn, synced_at)) => (conn, synced_at, file),
            Err(_) => {
                let (conn, synced_at) =
                    ensure_schema(memory(), &version).unwrap_or_else(|_| (memory(), None));
                (conn, synced_at, None)
            }
        };
        Cache {
            conn: Conn::new(conn, Rc::new(Cell::new(0))),
            file,
            mode,
            synced_at: Cell::new(synced_at),
            now: Cell::new(now),
            state: RefCell::new(SyncState::default()),
            hint: RefCell::new(None),
            missed: Cell::new(false),
        }
    }

    /// `is_on` returns whether the cache is on.
    pub fn is_on(&self) -> bool {
        self.mode == CacheMode::On
    }

    /// `is_fresh` returns true when the store has no record of an earlier
    /// sync, because it is new, rebuilt, or in memory.
    pub fn is_fresh(&self) -> bool {
        self.synced_at.get().is_none()
    }

    /// `begin_run` starts a run on this cache: it takes the time of the run's
    /// start and sets the count of statements to zero, so that the counts
    /// of `stats` cover this run alone (CACHE-3_AC-4).
    // sokf-implements: CACHE-3_AC-11
    pub fn begin_run(&self) {
        self.now.set(now());
        self.conn.reset();
        self.missed.set(false);
    }
}
// sokf:end cache

impl Cache {
    pub(super) fn conn(&self) -> &Conn {
        &self.conn
    }

    /// `file` returns the path of the store file, or `None` when the store
    /// is in memory.
    pub(super) fn file(&self) -> Option<&std::path::Path> {
        self.file.as_deref()
    }

    pub(super) fn synced_at(&self) -> Option<i64> {
        self.synced_at.get()
    }

    /// `set_synced_at` records the start of the run that this cache just
    /// committed as the time of the last sync.
    pub(super) fn set_synced_at(&self, at: i64) {
        self.synced_at.set(Some(at));
    }

    pub(super) fn now(&self) -> i64 {
        self.now.get()
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("mode", &self.mode)
            .field("synced_at", &self.synced_at)
            .finish_non_exhaustive()
    }
}

/// `version` returns the version that a store must record for a run to read
/// it. The version combines the release version, the store format, the
/// fingerprint of the build, and the hash of the configuration. The
/// fingerprint covers the sources and the pinned dependencies, the engine
/// among them, so a build that checks differently never reads the results of
/// another. The configuration is part of the version because it determines
/// the order in which sokf finds the stores when it walks the repository.
/// That order sets the number each store gets. When any of the four differs,
/// the run rebuilds the store (CACHE-1_AC-7).
// sokf-implements: CACHE-1_AC-6
fn version(root: &RepoRoot) -> String {
    let config = fs::read(root.as_path().join(".sokf/config.toml"))
        .map(|bytes| crate::source::fnv1a64(&bytes))
        .unwrap_or(0);
    format!(
        "{}+store.{STORE_FORMAT}+build.{}+config.{config:016x}",
        env!("CARGO_PKG_VERSION"),
        env!("SOKF_BUILD_FINGERPRINT")
    )
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_nanos()).ok())
        .unwrap_or(0)
}

/// `ensure_dir` creates the cache directory and its `.gitignore`, which
/// keeps everything in the directory out of git, and returns the directory.
// sokf-implements: CLI-4_AC-18
pub(crate) fn ensure_dir(root: &RepoRoot) -> std::io::Result<PathBuf> {
    let dir: PathBuf = root.as_path().join(CACHE_DIR);
    fs::create_dir_all(&dir)?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        fs::write(&ignore, "*\n")?;
    }
    Ok(dir)
}

/// `open_file` creates the cache directory and its `.gitignore`, and opens
/// the store file in it. It returns `None` when any step fails, and in that
/// case the run uses a store in memory.
fn open_file(root: &RepoRoot) -> Option<Connection> {
    let dir = ensure_dir(root).ok()?;
    // An earlier cache kept one JSON file for each entry. This cache does not
    // read those files, so `open_file` removes them.
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let _ = fs::remove_file(path);
            }
        }
    }
    let conn = Connection::open(dir.join(STORE_FILE)).ok()?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).ok()?;
    conn.pragma_update(None, "journal_mode", "WAL").ok()?;
    conn.pragma_update(None, "synchronous", "NORMAL").ok()?;
    Some(conn)
}

fn memory() -> Connection {
    Connection::open_in_memory().expect("an in-memory SQLite database opens")
}

// sokf:begin tables
/// `TABLES` lists the tables in the order that `ensure_schema` creates them.
/// `ensure_schema` drops them in the reverse order.
// sokf-implements: CACHE-1_AC-30
const TABLES: &[(&str, &str)] = &[
    (
        "meta",
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
    ),
    (
        "files",
        "CREATE TABLE files (path TEXT PRIMARY KEY, kind INTEGER NOT NULL, root INTEGER, \
         len INTEGER NOT NULL, mtime INTEGER, hash INTEGER, row_hash INTEGER) WITHOUT ROWID",
    ),
    (
        "dirs",
        "CREATE TABLE dirs (path TEXT PRIMARY KEY, root INTEGER NOT NULL)",
    ),
    (
        "documents",
        "CREATE TABLE documents (path TEXT PRIMARY KEY, store INTEGER NOT NULL, id TEXT, \
         type TEXT, title TEXT, description TEXT, broken INTEGER NOT NULL, \
         visible_hash INTEGER NOT NULL, scalars TEXT NOT NULL)",
    ),
    (
        "outputs",
        "CREATE TABLE outputs (path TEXT PRIMARY KEY, findings BLOB NOT NULL, \
         has_findings INTEGER NOT NULL)",
    ),
    (
        "queries",
        "CREATE TABLE queries (id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE)",
    ),
    (
        "deps",
        "CREATE TABLE deps (reader INTEGER NOT NULL, query INTEGER NOT NULL, \
         PRIMARY KEY (reader, query))",
    ),
    (
        "citations",
        "CREATE TABLE citations (target TEXT NOT NULL, slug TEXT, source TEXT NOT NULL, \
         source_type TEXT NOT NULL, rel TEXT, line INTEGER NOT NULL)",
    ),
    (
        "probes",
        "CREATE TABLE probes (path TEXT PRIMARY KEY, exists_ INTEGER NOT NULL)",
    ),
    (
        "assets",
        "CREATE TABLE assets (store INTEGER NOT NULL, asset TEXT NOT NULL, sidecar TEXT)",
    ),
    (
        "ids",
        "CREATE TABLE ids (store INTEGER NOT NULL, id TEXT NOT NULL, path TEXT NOT NULL, \
         PRIMARY KEY (store, id, path))",
    ),
    (
        "schemas",
        "CREATE TABLE schemas (path TEXT PRIMARY KEY, store INTEGER NOT NULL, \
         key TEXT NOT NULL, hash INTEGER NOT NULL, globs TEXT NOT NULL, \
         asset_globs TEXT NOT NULL, uses TEXT NOT NULL, line INTEGER NOT NULL, \
         archived TEXT NOT NULL)",
    ),
    (
        "formats",
        "CREATE TABLE formats (name TEXT PRIMARY KEY, hash INTEGER NOT NULL)",
    ),
    (
        "stores",
        "CREATE TABLE stores (root INTEGER PRIMARY KEY, identity TEXT NOT NULL, \
         former TEXT NOT NULL)",
    ),
    (
        "additional",
        "CREATE TABLE additional (path TEXT PRIMARY KEY, schema TEXT NOT NULL)",
    ),
    (
        "edges",
        "CREATE TABLE edges (source TEXT NOT NULL, kind INTEGER NOT NULL, written TEXT NOT NULL, \
         target TEXT NOT NULL, missing INTEGER NOT NULL, rel TEXT NOT NULL, region TEXT, \
         note TEXT)",
    ),
    (
        "keys",
        "CREATE TABLE keys (path TEXT NOT NULL, key TEXT NOT NULL, slug TEXT NOT NULL, \
         line INTEGER NOT NULL, heading_slug TEXT, scope TEXT NOT NULL, \
         knowledge INTEGER NOT NULL, rule_id TEXT)",
    ),
    (
        "markers",
        "CREATE TABLE markers (path TEXT NOT NULL, line INTEGER NOT NULL, \
         kind INTEGER NOT NULL, rel TEXT NOT NULL, target TEXT NOT NULL)",
    ),
    (
        "plans",
        "CREATE TABLE plans (path TEXT NOT NULL, target TEXT NOT NULL)",
    ),
];

/// `INDEXES` lists the indexes, which `ensure_schema` creates after the
/// tables.
const INDEXES: &[&str] = &[
    "CREATE INDEX ids_by_path ON ids (path)",
    "CREATE INDEX keys_by_path ON keys (path)",
    "CREATE INDEX keys_by_slug ON keys (slug)",
    "CREATE INDEX documents_by_type ON documents (store, type)",
    "CREATE INDEX deps_by_query ON deps (query)",
    "CREATE INDEX citations_by_target ON citations (target)",
    "CREATE INDEX citations_by_source ON citations (source)",
    "CREATE INDEX edges_by_source ON edges (source)",
    "CREATE INDEX edges_by_target ON edges (target)",
    "CREATE INDEX outputs_with_findings ON outputs (has_findings)",
    "CREATE INDEX markers_by_path ON markers (path)",
    "CREATE INDEX markers_by_target ON markers (target)",
    "CREATE INDEX plans_by_path ON plans (path)",
];
// sokf:end tables

/// `ensure_schema` keeps the tables when the store records `version`, and
/// otherwise drops them and creates them again. It returns the time of the
/// last sync that the store recorded, or `None` for a rebuilt store.
// sokf-implements: CLI-4_AC-19
fn ensure_schema(conn: Connection, version: &str) -> rusqlite::Result<(Connection, Option<i64>)> {
    let stored: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'version'", [], |r| {
            r.get(0)
        })
        .ok();
    if stored.as_deref() == Some(version) {
        let synced_at: Option<i64> = conn
            .query_row("SELECT value FROM meta WHERE key = 'synced_at'", [], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .and_then(|s| s.parse().ok());
        return Ok((conn, synced_at));
    }
    conn.execute_batch("BEGIN IMMEDIATE")?;
    for (name, _) in TABLES.iter().rev() {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {name}"))?;
    }
    for (_, ddl) in TABLES {
        conn.execute_batch(ddl)?;
    }
    for ddl in INDEXES {
        conn.execute_batch(ddl)?;
    }
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('version', ?1)",
        [version],
    )?;
    conn.execute_batch("COMMIT")?;
    Ok((conn, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        crate::test_support::fresh_dir(&format!("store-{name}"))
    }

    // sokf-tests: CACHE-1_AC-8
    // sokf-tests: CACHE-1_AC-10
    // sokf-tests: CLI-4_AC-18
    #[test]
    fn on_opens_a_file_with_a_gitignore_and_off_writes_nothing() {
        let dir = temp("open");
        let root = RepoRoot::new(dir.clone());
        let cache = Cache::open(&root, CacheMode::On);
        assert!(cache.is_on());
        assert!(dir.join(CACHE_DIR).join(STORE_FILE).exists());
        assert_eq!(
            fs::read_to_string(dir.join(CACHE_DIR).join(".gitignore")).unwrap(),
            "*\n"
        );
        assert_eq!(cache.synced_at(), None);
        drop(cache);
        let off_dir = temp("off");
        let off = Cache::open(&RepoRoot::new(off_dir.clone()), CacheMode::Off);
        assert!(!off.is_on());
        assert!(!off_dir.join(CACHE_DIR).exists());
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_dir_all(&off_dir).unwrap();
    }

    // sokf-tests: CACHE-1_AC-6
    // sokf-tests: CACHE-1_AC-7
    // sokf-tests: CLI-4_AC-19
    #[test]
    fn a_store_of_another_version_is_rebuilt_silently() {
        let dir = temp("version");
        let root = RepoRoot::new(dir.clone());
        {
            let cache = Cache::open(&root, CacheMode::On);
            cache
                .conn()
                .execute_batch(
                    "UPDATE meta SET value = '0.0.0+store.0' WHERE key = 'version'; \
                     INSERT INTO meta (key, value) VALUES ('synced_at', '5'); \
                     INSERT INTO files (path, kind, len) VALUES ('/a.md', 3, 1)",
                )
                .unwrap();
        }
        let cache = Cache::open(&root, CacheMode::On);
        assert_eq!(cache.synced_at(), None);
        assert!(cache.is_fresh());
        let rows: i64 = cache
            .conn()
            .query_row("SELECT count(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
        let version: String = cache
            .conn()
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, super::version(&root));
        // The version names the build, so another build reads another store.
        assert!(
            version.contains(&format!("+build.{}+", env!("SOKF_BUILD_FINGERPRINT"))),
            "{version}"
        );
        // A run records the time of its sync in the store. When the
        // configuration changes, the next run rebuilds the store, which
        // removes that time.
        cache
            .conn()
            .execute_batch("INSERT INTO meta (key, value) VALUES ('synced_at', '5')")
            .unwrap();
        drop(cache);
        assert!(!Cache::open(&root, CacheMode::On).is_fresh());
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        fs::write(dir.join(".sokf/config.toml"), "roots = [\"/docs\"]\n").unwrap();
        assert!(Cache::open(&root, CacheMode::On).is_fresh());
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CACHE-1_AC-7
    // sokf-tests: CACHE-1_AC-9
    #[test]
    fn a_store_that_does_not_open_falls_back_to_memory() {
        let dir = temp("broken");
        // A file stands where the cache directory should be.
        fs::write(dir.join(".sokf"), "not a directory").unwrap();
        let root = RepoRoot::new(dir.clone());
        let cache = Cache::open(&root, CacheMode::On);
        assert!(cache.is_on());
        assert_eq!(cache.synced_at(), None);
        let rows: i64 = cache
            .conn()
            .query_row("SELECT count(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_store_that_is_not_a_database_is_rebuilt() {
        let dir = temp("garbage");
        let cache_dir = dir.join(CACHE_DIR);
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(
            cache_dir.join(STORE_FILE),
            "not sqlite at all, and long enough to matter",
        )
        .unwrap();
        let root = RepoRoot::new(dir.clone());
        let cache = Cache::open(&root, CacheMode::On);
        assert!(cache.is_on());
        assert_eq!(cache.synced_at(), None);
        assert!(format!("{cache:?}").starts_with("Cache"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
