//! The unit tests of the sync cover the stat rule, the store in memory,
//! the order of paths, and the rows that the sync holds.

use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use super::*;
use crate::{CACHE_DIR, CacheMode, Config, RepoRoot, collect, source};

fn temp(name: &str) -> PathBuf {
    let dir = crate::test_support::fresh_dir(&format!("sync-{name}"));
    fs::create_dir_all(dir.join("knowledge")).unwrap();
    fs::write(dir.join("knowledge/manifest.sokf.yaml"), "sokf: \"0.6\"\n").unwrap();
    fs::write(
        dir.join("knowledge/a.md"),
        "---\ntype: A\nid: a\n---\n# A\n",
    )
    .unwrap();
    fs::write(
        dir.join("knowledge/b.md"),
        "---\ntype: A\nid: b\n---\n# B\n",
    )
    .unwrap();
    dir
}

fn p(s: &str) -> RepoPath {
    RepoPath::parse(s).unwrap()
}

/// `age` sets a file's modification time `secs` seconds into the past.
fn age(path: &std::path::Path, secs: u64) {
    let f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(secs))
        .unwrap();
}

/// `run` syncs the cache, reads each file for which `must_read` is true,
/// and commits.
fn run(root: &RepoRoot, mode: CacheMode) -> (Cache, SyncStats) {
    let cache = Cache::open(root, mode);
    let tree = collect(root, &Config::default()).unwrap();
    cache.sync(&tree);
    for f in &tree.files {
        if cache.must_read(&f.path) {
            let src = source::load(root, &f.path).unwrap();
            cache.observe(&f.path, &src.key);
        }
    }
    cache.commit();
    let stats = cache.stats();
    (cache, stats)
}

/// `counts` returns only the sync's counts: unchanged, read, changed, and
/// removed.
fn counts(stats: SyncStats) -> [usize; 4] {
    [stats.unchanged, stats.read, stats.changed, stats.removed]
}

// sokf-tests: CACHE-1_AC-1
// sokf-tests: CACHE-1_AC-2
// sokf-tests: CACHE-1_AC-3
// sokf-tests: CACHE-3_AC-4
#[test]
fn the_stat_rule_skips_old_files_and_the_hash_decides_the_rest() {
    let dir = temp("rule");
    let root = RepoRoot::new(dir.clone());
    for name in [
        "knowledge/manifest.sokf.yaml",
        "knowledge/a.md",
        "knowledge/b.md",
    ] {
        age(&dir.join(name), 10);
    }
    // With a new store, the sync reads every file, and every file counts
    // as changed.
    let (_, first) = run(&root, CacheMode::On);
    assert_eq!(counts(first), [0, 3, 3, 0]);
    // The twelve queries read the generation, the file rows, and the
    // directory rows, begin the transaction, read the generation again,
    // write three file rows and one directory row, write the sync time and
    // the next generation, and commit.
    assert_eq!(first.queries, 12);
    // When nothing changes, the stat rule lets the sync skip every file,
    // and the sync writes no row.
    let (_, second) = run(&root, CacheMode::On);
    assert_eq!(counts(second), [3, 0, 0, 0]);
    assert_eq!(second.queries, 8);
    // The sync reads a file touched within the racy margin, and its
    // unchanged hash keeps it out of the changed files.
    fs::write(
        dir.join("knowledge/a.md"),
        "---\ntype: A\nid: a\n---\n# A\n",
    )
    .unwrap();
    let (cache, third) = run(&root, CacheMode::On);
    assert_eq!(counts(third), [2, 1, 0, 0]);
    assert_eq!(cache.changed(), [] as [tramlines_md::RepoPath; 0]);
    drop(cache);
    // After the test edits `b.md` within the racy margin without changing
    // its length, the sync reads it, and the file counts as changed.
    fs::write(
        dir.join("knowledge/b.md"),
        "---\ntype: A\nid: b\n---\n# C\n",
    )
    .unwrap();
    let (cache, fourth) = run(&root, CacheMode::On);
    assert_eq!(fourth.changed, 1);
    assert_eq!(cache.changed(), vec![p("/knowledge/b.md")]);
    assert_eq!(
        cache.stored_hash(&p("/knowledge/b.md")),
        Some(source::fnv1a64(b"---\ntype: A\nid: b\n---\n# C\n"))
    );
    drop(cache);
    // The test changes the length of `a.md` and sets its modification time
    // outside the racy margin. The sync still reads it, because its size no
    // longer matches the stored stat.
    fs::write(
        dir.join("knowledge/a.md"),
        "---\ntype: A\nid: a\n---\n# A!\n",
    )
    .unwrap();
    age(&dir.join("knowledge/a.md"), 10);
    let (cache, fifth) = run(&root, CacheMode::On);
    assert_eq!(fifth.changed, 1);
    assert_eq!(cache.changed(), vec![p("/knowledge/a.md")]);
    drop(cache);
    // A removed file loses its row, and the sync reads a new file.
    fs::remove_file(dir.join("knowledge/a.md")).unwrap();
    fs::write(
        dir.join("knowledge/c.md"),
        "---\ntype: A\nid: c\n---\n# C\n",
    )
    .unwrap();
    let (cache, sixth) = run(&root, CacheMode::On);
    assert_eq!(sixth.removed, 1);
    assert_eq!(cache.changed(), vec![p("/knowledge/c.md")]);
    assert_eq!(cache.stored_hash(&p("/knowledge/a.md")), None);
    assert!(cache.must_read(&p("/never-walked.md")));
    drop(cache);
    fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CACHE-1_AC-8
#[test]
fn off_keeps_the_store_in_memory_and_reads_everything_each_run() {
    let dir = temp("off");
    let root = RepoRoot::new(dir.clone());
    let (_, first) = run(&root, CacheMode::Off);
    let (_, second) = run(&root, CacheMode::Off);
    assert_eq!(counts(first), counts(second));
    assert_eq!(first.read, 3);
    assert!(!dir.join(CACHE_DIR).exists());
    fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CACHE-1_AC-24
#[test]
fn the_store_orders_paths_as_the_walk_does() {
    // The sync's merge of walked paths with stored rows works only if
    // `RepoPath` sorts the same way as the store's `BINARY` collation,
    // which compares paths byte by byte.
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t (path TEXT PRIMARY KEY)")
        .unwrap();
    let mut paths: Vec<RepoPath> = [
        "/k/b.md",
        "/k/a/z.md",
        "/k/a.md",
        "/k/Z.md",
        "/k/a-b.md",
        "/k/a_b.md",
        "/k/é.md",
        "/k/a/index.md",
        "/k/a.md.md",
    ]
    .iter()
    .map(|s| p(s))
    .collect();
    for path in &paths {
        conn.execute("INSERT INTO t (path) VALUES (?1)", [path.as_str()])
            .unwrap();
    }
    let mut stmt = conn.prepare("SELECT path FROM t ORDER BY path").unwrap();
    let stored: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .flatten()
        .collect();
    paths.sort();
    let sorted: Vec<String> = paths.iter().map(ToString::to_string).collect();
    assert_eq!(stored, sorted);
}

// sokf-tests: CACHE-3_AC-2
#[test]
fn the_sync_holds_the_changed_rows_alone() {
    let dir = temp("held");
    let root = RepoRoot::new(dir.clone());
    for name in [
        "knowledge/manifest.sokf.yaml",
        "knowledge/a.md",
        "knowledge/b.md",
    ] {
        age(&dir.join(name), 10);
    }
    let (_, first) = run(&root, CacheMode::On);
    assert_eq!(first.held, 3);
    let (_, second) = run(&root, CacheMode::On);
    assert_eq!(second.held, 0);
    fs::write(
        dir.join("knowledge/b.md"),
        "---\ntype: A\nid: b\n---\n# C\n",
    )
    .unwrap();
    age(&dir.join("knowledge/b.md"), 10);
    let (_, third) = run(&root, CacheMode::On);
    assert_eq!(third.held, 1);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_walked_file_no_command_reads_keeps_a_row_without_a_hash() {
    let dir = temp("unread");
    let root = RepoRoot::new(dir.clone());
    let cache = Cache::open(&root, CacheMode::On);
    let tree = collect(&root, &Config::default()).unwrap();
    cache.sync(&tree);
    cache.commit();
    drop(cache);
    let cache = Cache::open(&root, CacheMode::On);
    let tree = collect(&root, &Config::default()).unwrap();
    cache.sync(&tree);
    assert!(cache.must_read(&p("/knowledge/a.md")));
    assert_eq!(cache.stored_hash(&p("/knowledge/a.md")), None);
    drop(cache);
    fs::remove_dir_all(&dir).unwrap();
}

// Run B syncs and reads `a.md`. The file then changes again, and run A
// syncs, reads it, and commits. Run B commits last. B's rows rest on the
// store as it stood before A's commit, so B writes nothing, and the store
// keeps A's row, which matches the file.
// sokf-tests: CACHE-4_AC-3
#[test]
fn a_run_whose_store_changed_after_its_sync_writes_nothing() {
    let dir = temp("race");
    let root = RepoRoot::new(dir.clone());
    for name in [
        "knowledge/manifest.sokf.yaml",
        "knowledge/a.md",
        "knowledge/b.md",
    ] {
        age(&dir.join(name), 10);
    }
    run(&root, CacheMode::On);
    let a_md = dir.join("knowledge/a.md");
    fs::write(&a_md, "---\ntype: A\nid: a\n---\n# A one\n").unwrap();
    let b = Cache::open(&root, CacheMode::On);
    b.sync(&collect(&root, &Config::default()).unwrap());
    let one = source::load(&root, &p("/knowledge/a.md")).unwrap();
    b.observe(&p("/knowledge/a.md"), &one.key);
    fs::write(&a_md, "---\ntype: A\nid: a\n---\n# A two, longer\n").unwrap();
    let (a, _) = run(&root, CacheMode::On);
    let after_a = a.stored_hash(&p("/knowledge/a.md"));
    b.commit();
    let fresh = Cache::open(&root, CacheMode::On);
    assert_eq!(fresh.stored_hash(&p("/knowledge/a.md")), after_a);
}
