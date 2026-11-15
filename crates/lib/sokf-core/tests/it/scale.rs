//! These tests check that an incremental run reads and checks only the
//! changed document and the documents that depend on it (contract-cache
//! CACHE-3). Each builds a synthetic store of thousands of documents and
//! makes one edit. It then checks that two of the cache's counters stay under
//! a bound: the number of files that the run reads, and the number of
//! statements that it runs. The benchmark of 100,000 documents for
//! CACHE-3_AC-5 is ignored by default, so run it by hand on a release
//! build.

use std::{fs, time::Instant};

use common::{
    TempTree, age,
    incremental::{check, cold, edit},
    lines,
    synthetic::{self, Shape, Synthetic},
};
use sokf_core::{CACHE_DIR, CacheMode, STORE_FILE, SyncStats};

use crate::common;

/// `Run` records the result of one incremental run through the store.
struct Run {
    dirty: usize,
    full: bool,
    findings: Vec<String>,
    stats: SyncStats,
}

fn run(tree: &TempTree) -> Run {
    let (loaded, report, cache) = check(tree, None);
    Run {
        dirty: loaded.dirty.len(),
        full: loaded.full,
        findings: lines(&report),
        stats: cache.stats(),
    }
}

/// `append` adds a paragraph to the body of `rel`: before its first
/// include, or, if it has none, before its definition block, or, if it has
/// neither, at its end.
fn append(tree: &TempTree, rel: &str, line: &str) {
    let text = tree.read(rel);
    let edited = match text
        .split_once("<!-- sokf:include ")
        .map(|(head, tail)| (head, "<!-- sokf:include ", tail))
        .or_else(|| {
            text.split_once("<!-- sokf:links -->")
                .map(|(head, tail)| (head, "<!-- sokf:links -->", tail))
        }) {
        Some((head, marker, tail)) => format!("{head}{line}\n\n{marker}{tail}"),
        None => format!("{text}\n{line}\n"),
    };
    edit(tree, rel, &edited);
}

/// `floor` counts the files that every run reads, whatever changed: the
/// manifests. A run reads a schema only when a check needs to compile it.
fn floor(tree: &TempTree) -> usize {
    tree.files()
        .iter()
        .filter(|(rel, _)| rel.ends_with("manifest.sokf.yaml"))
        .count()
}

/// `Measure` records, for each document, the bytes of the store file and the
/// bytes of the rows that the store keeps for the document.
struct Measure {
    /// The store file's bytes for each document.
    bytes: u64,
    /// The bytes of one document's row.
    row: i64,
    /// The number of dependency rows for each document.
    deps: i64,
    /// The bytes of one document's dependency rows, with their interned keys.
    dep_bytes: i64,
}

impl std::fmt::Display for Measure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the store file holds {} bytes for each document; each document's row holds {} \
             bytes; each document has {} dependency rows, which hold {} bytes with their \
             interned keys",
            self.bytes, self.row, self.deps, self.dep_bytes
        )
    }
}

fn measure(tree: &TempTree, docs: usize) -> Measure {
    let file = tree.dir().join(CACHE_DIR).join(STORE_FILE);
    let bytes = fs::metadata(&file).map_or(0, |m| m.len());
    let conn =
        rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    let rows = count(
        "SELECT coalesce(sum(length(path) + length(coalesce(id, '')) + length(coalesce(type, '')) \
         + length(coalesce(title, '')) + length(coalesce(description, '')) + 16), 0) FROM documents",
    );
    let deps = count("SELECT count(*) FROM deps");
    let interned = count("SELECT coalesce(sum(length(key) + 8), 0) FROM queries");
    let per_doc = i64::try_from(docs).expect("the count of documents fits in i64");
    Measure {
        bytes: bytes / docs as u64,
        row: rows / per_doc,
        deps: deps / per_doc,
        dep_bytes: (deps * 16 + interned) / per_doc,
    }
}

// sokf-tests: CACHE-3_AC-1
// sokf-tests: CACHE-3_AC-4
// sokf-tests: CACHE-3_AC-7
// sokf-tests: CACHE_P-3
// sokf-tests: CACHE_P-4
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the test walks one scenario in order, which splitting would hide"
)]
fn a_document_edit_costs_the_change_and_its_dependents() {
    let tree = TempTree::empty("scale");
    let shape = Shape {
        docs: 1200,
        per_dir: 100,
        seed: 7,
    };
    let store: Synthetic = synthetic::write(&tree, &shape);
    age(tree.dir(), 10);
    let first = run(&tree);
    assert!(first.full);
    assert!(first.findings.is_empty(), "{:?}", first.findings);
    // Measure the store's size in bytes for each document.
    // sokf-tests: CACHE-3_AC-7
    let size = measure(&tree, shape.docs);
    eprintln!("{size}");
    assert!(size.row < 200, "{size}");
    assert!(size.dep_bytes < 300, "{size}");
    let floor = floor(&tree);

    // When nothing has changed, the run reads no document and checks none.
    let quiet = run(&tree);
    assert!(!quiet.full);
    assert_eq!(quiet.dirty, 0);
    assert!(
        quiet.stats.read <= floor,
        "read {} files; the floor is {floor}",
        quiet.stats.read
    );
    assert!(quiet.findings.is_empty(), "{:?}", quiet.findings);

    // Edit a note's body: the run reads the note, the three specs that its
    // check reads, the index of its directory, and the note schema, but not
    // the spec schema.
    let note = &store.notes[17];
    append(&tree, note, "A line more.");
    let r = run(&tree);
    assert!(!r.full);
    assert_eq!(r.dirty, 1);
    assert!(
        r.stats.read <= floor + 1 + 3 + 1 + 1,
        "read {} files; the floor is {floor}",
        r.stats.read
    );
    assert!(r.findings.is_empty(), "{:?}", r.findings);
    assert_eq!(r.findings, cold(&tree));

    // Edit a spec's body without changing any of its facts. The run checks
    // the spec and the notes that include it. It also reads what those notes
    // read. It neither checks nor reads a note that only links to the spec.
    let s = 3;
    let readers = store.includers_of(s).len();
    assert!(readers > 0);
    assert!(store.readers_of(s).len() > readers);
    let spec = &store.specs[s];
    let text = tree.read(spec);
    edit(
        &tree,
        spec,
        &text.replacen("rule one of spec 3.", "rule one, of spec 3.", 1),
    );
    let r = run(&tree);
    assert!(!r.full);
    assert_eq!(
        r.dirty,
        1 + readers,
        "{} documents dirty for {readers} readers",
        r.dirty
    );
    // The run reads the spec, its readers and what they read, the touched
    // index, and the two schemas that the checks compile.
    assert!(
        r.stats.read <= floor + 1 + readers * 4 + 1 + 2,
        "read {} files for {readers} readers; the floor is {floor}",
        r.stats.read
    );
    assert_eq!(r.findings, cold(&tree));
    // The run issues statements only for the changed documents: it makes no
    // pass over all the documents.
    assert!(
        r.stats.queries <= 2000,
        "{} statements for {} documents",
        r.stats.queries,
        shape.docs
    );
    assert!(
        quiet.stats.queries <= 200,
        "{} statements for a quiet run over {} documents",
        quiet.stats.queries,
        shape.docs
    );
    // The sync holds only the rows that it has read and must read, never
    // every row.
    assert!(
        quiet.stats.held <= floor,
        "{} rows held in a quiet run",
        quiet.stats.held
    );
    assert!(
        r.stats.held <= floor + 1 + readers * 4 + 1 + 2,
        "{} rows held for one spec edit",
        r.stats.held
    );
    eprintln!(
        "one spec edit with {readers} readers: read {} files, {} statements",
        r.stats.read, r.stats.queries
    );

    // Edit the note schema: the run checks every note, reads the schema and
    // the specs that the notes read, and checks no spec.
    let schema = "knowledge/schemas/note.md";
    let text = tree.read(schema);
    edit(
        &tree,
        schema,
        &text.replacen("Reads rule one of spec 1.", "Reads rule two of spec 1.", 1),
    );
    let r = run(&tree);
    assert!(!r.full);
    assert_eq!(
        r.dirty,
        store.notes.len() + 1,
        "{} documents dirty",
        r.dirty
    );
    assert!(
        r.stats.read <= floor + 1 + shape.docs + 1 + 1,
        "read {} files; the floor is {floor}",
        r.stats.read
    );
    // Both the incremental and the cold report still list the stale
    // includes that the spec edit left.
    assert_eq!(r.findings, cold(&tree));
    eprintln!(
        "one schema edit over {} notes: read {} files, {} statements",
        store.notes.len(),
        r.stats.read,
        r.stats.queries
    );
}

// The benchmark stands in the local tier of tests, which a developer runs
// with `mise run test:local` and every CI run skips, because it measures
// nothing that a run on a developer's machine doesn't, and in CI it ran
// for over half an hour on each system.
mod local_only {
    use super::*;

    // sokf-tests: CACHE-3_AC-3
    // sokf-tests: CACHE-3_AC-5
    // sokf-tests: CACHE-3_AC-6
    // sokf-tests: CACHE-3_AC-10
    #[test]
    #[ignore = "local: 100,000 documents: run it by hand on a release build, and set SOKF_SCALE_DOCS to change the number"]
    #[expect(
        clippy::too_many_lines,
        reason = "the test walks one scenario in order, which splitting would hide"
    )]
    fn a_hundred_thousand_documents_warm_in_under_a_second() {
        let docs: usize = std::env::var("SOKF_SCALE_DOCS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100_000);
        let tree = TempTree::empty("bench");
        let shape = Shape {
            docs,
            per_dir: 500,
            seed: 11,
        };
        let t = Instant::now();
        let store = synthetic::write(&tree, &shape);
        eprintln!("wrote and fixed {docs} documents in {:?}", t.elapsed());
        age(tree.dir(), 10);
        // Time a run without the store, then the first run that fills it. The
        // run that fills the store takes as long as the run without it, plus the
        // time to write the store. It takes no more than twice as long.
        let t = Instant::now();
        let _ = cold(&tree);
        let without = t.elapsed();
        eprintln!("a run without the store in {without:?}");
        let t = Instant::now();
        let first = run(&tree);
        let filled = t.elapsed();
        eprintln!(
            "first warm run in {filled:?}: read {}, {} statements",
            first.stats.read, first.stats.queries
        );
        if docs >= 100_000 && !cfg!(debug_assertions) {
            assert!(
                filled.as_secs_f64() <= without.as_secs_f64() * 2.0,
                "the first run with the store took {filled:?}; the run without it took {without:?}"
            );
        }
        eprintln!("{}", measure(&tree, docs));
        let t = Instant::now();
        let quiet = run(&tree);
        eprintln!(
            "quiet run in {:?}: read {}, {} statements",
            t.elapsed(),
            quiet.stats.read,
            quiet.stats.queries
        );
        let note = &store.notes[docs / 3];
        append(&tree, note, "A line more.");
        let t = Instant::now();
        let r = run(&tree);
        let took = t.elapsed();
        eprintln!(
            "one note edit in {took:?}: dirty {}, read {}, {} statements",
            r.dirty, r.stats.read, r.stats.queries
        );
        assert!(r.findings.is_empty(), "{:?}", r.findings);
        if docs >= 100_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "one edit took {took:?}");
        }
        // Edit the schema of the memos, a type that makes up 1% of the
        // documents: 1,000 memos in a store of 100,000.
        let text = tree.read(synthetic::MEMO_SCHEMA_PATH);
        edit(
            &tree,
            synthetic::MEMO_SCHEMA_PATH,
            &text.replacen(
                "This memo notes the day's decision.",
                "This memo notes the week's decision.",
                1,
            ),
        );
        let t = Instant::now();
        let r = run(&tree);
        let took = t.elapsed();
        eprintln!(
            "one schema edit over {} memos in {took:?}: dirty {}, read {}, {} statements",
            store.memos.len(),
            r.dirty,
            r.stats.read,
            r.stats.queries
        );
        assert_eq!(r.dirty, store.memos.len() + 1);
        assert!(r.findings.is_empty(), "{:?}", r.findings);
        if docs >= 100_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 10.0, "one schema edit took {took:?}");
        }
        // Edit one note, then look up its id: the lookup checks the edit and
        // commits the result within the time budget of a run, and the run
        // after it checks nothing (CACHE-1_AC-34, CACHE-1_AC-36).
        let note = &store.notes[docs / 5];
        append(&tree, note, "A line more.");
        let id = tree
            .read(note)
            .lines()
            .find_map(|l| l.strip_prefix("id: ").map(str::to_string))
            .unwrap();
        let t = Instant::now();
        let found = sokf_core::locate(&tree.root(), &[id], false, CacheMode::On);
        let took = t.elapsed();
        eprintln!("one note edit, then locate, in {took:?}");
        assert!(found.is_ok(), "{found:?}");
        if docs >= 100_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "the lookup took {took:?}");
        }
        let r = run(&tree);
        eprintln!("the run after it: dirty {}, read {}", r.dirty, r.stats.read);
        assert_eq!(r.dirty, 0);
        // Build the graph that an agent asks for first, the layout, and then
        // the graph of one note. Each finishes within the time budget of a run.
        // The layout prints at most 20 entries of a directory (CLI-7_AC-69).
        let graph = |start: Vec<String>| {
            let opts = sokf_core::GraphOptions {
                query: sokf_core::GraphQuery {
                    start,
                    ..sokf_core::GraphQuery::default()
                },
                cwd: tree.dir().to_path_buf(),
                cache: CacheMode::On,
            };
            let t = Instant::now();
            let (_, result) = sokf_core::graph(&tree.root(), &opts).unwrap();
            (t.elapsed(), result)
        };
        let (took, layout) = graph(Vec::new());
        eprintln!("the layout in {took:?}: {}", layout.summary.line());
        if docs >= 100_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "the layout took {took:?}");
        }
        // Edit one note, then build the graph of that note: the graph reads
        // only the edges near its start (CACHE-3_AC-10).
        append(&tree, note, "A line more.");
        let (took, one) = graph(vec![note.clone()]);
        eprintln!("the graph of a note in {took:?}: {}", one.summary.line());
        assert!(one.summary.documents.shown > 1, "{}", one.summary.line());
        if docs >= 100_000 && !cfg!(debug_assertions) {
            assert!(
                took.as_secs_f64() < 1.0,
                "the graph of a note took {took:?}"
            );
        }
    }
}
