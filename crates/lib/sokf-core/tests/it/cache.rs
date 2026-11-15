//! These tests cover the incremental cache (contract-cache). A warm run, a
//! run that reads an existing cache, holds no unchanged document. The rows of
//! the store supply what the run needs of the documents it does not hold, and
//! the keys of a warm run equal those of a cold run, a run without a cache.

use std::fs;

use common::{TempTree, age, age_file};
use sokf_core::{
    Cache, CacheMode, Config, FileKind, Model, RepoFormats, RepoPath, check, collect, index,
    load_config, load_model, load_repository, load_with_keys, load_working,
    model::{KeyIndex, Query},
};

use crate::common;

fn config(tree: &TempTree) -> Config {
    load_config(&tree.root()).unwrap()
}

/// `working` loads the working set through a cache in the given `mode` and
/// returns it with the cache's statistics. The working set holds only the changed
/// documents, and the store supplies the rest.
fn working(tree: &TempTree, mode: CacheMode) -> (Model, sokf_core::SyncStats, Cache) {
    let root = tree.root();
    let cache = Cache::open(&root, mode);
    let model = load_working(&root, &config(tree), &cache).unwrap();
    cache.commit();
    let stats = cache.stats();
    (model, stats, cache)
}

/// `load` loads every document through a cache in the given `mode` and
/// returns the model with the cache's statistics.
fn load(tree: &TempTree, mode: CacheMode) -> (Model, sokf_core::SyncStats, Cache) {
    let root = tree.root();
    let cache = Cache::open(&root, mode);
    let model = load_model(&root, &config(tree), &cache).unwrap();
    cache.commit();
    let stats = cache.stats();
    (model, stats, cache)
}

fn keys_of(index: &KeyIndex) -> Vec<(String, String, usize)> {
    let mut v: Vec<(String, String, usize)> = index
        .document
        .values()
        .flatten()
        .map(|e| (e.path.to_string(), e.key.clone(), e.line))
        .collect();
    v.sort();
    v.dedup();
    v
}

// sokf-tests: CACHE-1_AC-1
// sokf-tests: CACHE-1_AC-2
// sokf-tests: CACHE-1_AC-13
// sokf-tests: CACHE-3_AC-1
// sokf-tests: CACHE_P-2
#[test]
fn a_warm_run_holds_no_unchanged_document_and_the_store_answers_for_it() {
    let tree = TempTree::fixture("graph");
    age(tree.dir(), 10);
    tree.validate(false, CacheMode::On);
    let (warm, stats, _) = working(&tree, CacheMode::On);
    assert_eq!(warm.documents().count(), 0, "the run holds no document");
    // The run reads only the files that are not documents: the manifest, the
    // schemas, the indexes, and the additional files.
    let walked = collect(&tree.root(), &config(&tree)).unwrap();
    let not_documents = walked
        .files
        .iter()
        .filter(|f| f.kind != FileKind::Document && f.kind != FileKind::Candidate)
        .count();
    assert!(
        stats.read <= not_documents + warm.additional.len(),
        "{stats:?}"
    );
    assert_eq!(stats.changed, 0, "{stats:?}");
    // The rows of the store give the same answers as a cold model: each id,
    // the documents of each type, whether a path is a document, the fields
    // that an index lists, and whether a document's frontmatter was parsed.
    let (cold, _, _) = load(&tree, CacheMode::Off);
    assert!(cold.documents().count() > 0);
    let store = warm.fallback.as_ref().expect("the run has a store");
    for (k, doc) in cold.documents() {
        let id = doc.id().unwrap();
        assert_eq!(warm.ids(k, id), cold.ids(k, id));
        assert!(warm.is_document(&doc.path));
        let fields = store.fields(&doc.path).unwrap();
        assert_eq!(fields.id.as_deref(), Some(id));
        assert_eq!(fields.title, doc.core.title);
        assert_eq!(fields.description, doc.core.description);
        assert!(!store.broken(&doc.path));
        if let Some(t) = doc.type_name() {
            assert_eq!(warm.ids_of_type(k, t), cold.ids_of_type(k, t));
        }
    }
    assert!(!warm.is_document(&RepoPath::parse("/knowledge/nope.md").unwrap()));
}

// sokf-tests: CACHE-1_AC-3
// sokf-tests: CACHE-1_AC-14
#[test]
fn a_changed_document_is_read_and_its_row_replaces_the_old() {
    let tree = TempTree::fixture("graph");
    age(tree.dir(), 10);
    tree.validate(false, CacheMode::On);
    let rel = "knowledge/config.md";
    let text = tree.read(rel);
    tree.write(rel, &text.replace("title: ", "title: Renamed "));
    age_file(&tree.dir().join(rel), 10);
    let (model, stats, _) = working(&tree, CacheMode::On);
    let path = RepoPath::parse("/knowledge/config.md").unwrap();
    let held: Vec<&RepoPath> = model.documents().map(|(_, d)| &d.path).collect();
    assert_eq!(held, vec![&path], "the run holds only the changed document");
    let changed = match model.by_path(&path) {
        Some(sokf_core::Located::Document(_, d)) => d,
        other => panic!("{other:?}"),
    };
    assert!(
        changed
            .core
            .title
            .as_deref()
            .unwrap()
            .starts_with("Renamed ")
    );
    assert_eq!(stats.changed, 1, "{stats:?}");
    // The store of the next run holds the new row.
    let (again, stats, _) = working(&tree, CacheMode::On);
    assert_eq!(again.documents().count(), 0);
    let fields = again.fallback.as_ref().unwrap().fields(&path).unwrap();
    assert_eq!(fields.title, changed.core.title);
    assert_eq!(stats.changed, 0);
    // A removed document loses its row and its id.
    fs::remove_file(tree.dir().join(rel)).unwrap();
    let (gone, _, _) = working(&tree, CacheMode::On);
    assert_eq!(gone.ids(0, "config"), [] as [sokf_core::RepoPath; 0]);
    assert!(!gone.is_document(&path));
}

// sokf-tests: CACHE_P-2
#[test]
fn the_keys_of_a_run_through_the_store_equal_a_cold_run() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    let report = tree.validate(false, CacheMode::On);
    assert!(report.errors.is_empty(), "{report:?}");
    let root = tree.root();
    let cache = Cache::open(&root, CacheMode::On);
    let (_, keys) = load_with_keys(&root, &config(&tree), &cache).unwrap();
    let cold = Cache::open(&root, CacheMode::Off);
    let (_, fresh) = load_with_keys(&root, &config(&tree), &cold).unwrap();
    assert_ne!(
        keys_of(&fresh),
        [] as [(std::string::String, std::string::String, usize); 0]
    );
    assert_eq!(keys_of(&keys), keys_of(&fresh));
}

// sokf-tests: CACHE-1_AC-8
#[test]
fn off_reads_everything_each_run() {
    let tree = TempTree::fixture("graph");
    age(tree.dir(), 10);
    tree.validate(false, CacheMode::Off);
    let (model, stats, _) = load(&tree, CacheMode::Off);
    assert!(model.documents().count() > 0);
    assert_eq!(stats.unchanged, 0);
    assert!(!tree.dir().join(".sokf/cache/sokf.sqlite").exists());
}

// sokf-tests: CACHE-1_AC-14
#[test]
fn a_check_records_what_it_reads_of_other_documents_and_files() {
    let tree = TempTree::fixture("graph");
    let root = tree.root();
    let cache = Cache::open(&root, CacheMode::Off);
    let grammar = sokf_core::load_grammar().unwrap();
    let loaded = load_repository(
        &root,
        &config(&tree),
        &grammar,
        &RepoFormats::default(),
        &cache,
    )
    .unwrap();
    let indexes = index::expected(&loaded.model, &[]);
    let _ = check::run(
        &loaded.model,
        &loaded.compiled,
        &loaded.expected,
        &loaded.keys,
        &indexes,
    );
    let p = |s: &str| RepoPath::parse(s).unwrap();
    let reads = loaded.model.reads_of(&p("/knowledge/core/planner.md"));
    let doc = |id: &str| Query::Doc {
        knowledge: 0,
        id: id.into(),
    };
    for want in [
        doc("config"),
        doc("planer"),
        Query::Exists(p("/crates/lib/example-core/src/planner.rs")),
        Query::Exists(p("/crates/app/sokf/src/main.rs")),
        Query::Body(p("/crates/app/sokf/src/main.rs")),
    ] {
        assert!(reads.contains(&want), "{want:?} not in {reads:?}");
    }
    // A document that cites nothing reads nothing of another document. It
    // reads only its own id and path, and the schema of its type.
    let quiet = loaded.model.reads_of(&p("/knowledge/config.md"));
    let own = p("/knowledge/config.md");
    assert!(
        quiet.iter().all(|q| match q {
            Query::Doc { id, .. } => id == "config",
            Query::Path(path) => *path == own,
            Query::Schema { key, .. } => key == "Subsystem",
            _ => false,
        }),
        "{quiet:?}"
    );
    assert!(
        quiet.iter().any(|q| matches!(q, Query::Schema { .. })),
        "{quiet:?}"
    );
    // Every reader is a walked file.
    for reader in loaded.model.reads().keys() {
        assert!(loaded.model.by_path(reader).is_some(), "{reader}");
    }
}

// sokf-tests: CACHE-1_AC-11
// sokf-tests: CACHE-4_AC-1
// sokf-tests: CACHE-4_AC-2
#[test]
fn two_runs_on_one_store_at_once_leave_it_whole_and_each_reports_its_own_sync() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    tree.validate(false, CacheMode::On);
    let rel = "knowledge/decisions/decision-single-binary.md";
    let text = tree.read(rel);
    tree.write(
        rel,
        &text.replacen("title: Ship one binary", "title: Ship one", 1),
    );
    let root = tree.root();
    let expected = tree.validate_all(false, CacheMode::Off);
    let run = || {
        let root = root.clone();
        std::thread::spawn(move || {
            let cache = Cache::open(&root, CacheMode::On);
            let grammar = sokf_core::load_grammar().unwrap();
            let config = load_config(&root).unwrap();
            let (_, findings) = sokf_core::check_repository(
                &root,
                &config,
                &grammar,
                &RepoFormats::default(),
                &cache,
                false,
            )
            .unwrap();
            let mut report = sokf_core::Report::default();
            report.extend(findings);
            report.finish()
        })
    };
    let (a, b) = (run(), run());
    let (a, b) = (a.join().unwrap(), b.join().unwrap());
    assert_eq!(a.errors, expected.errors);
    assert_eq!(b.errors, expected.errors);
    // The store is intact: a later run reads it and reports the same errors.
    let after = tree.validate_all(false, CacheMode::On);
    assert_eq!(after.errors, expected.errors);
}

// The store records a line for each finding. A finding on a schema that the
// run did not read appears at that line. Every run computes the glob findings of the
// broken fixture again, and each appears at its line in the body of the
// schema.
// sokf-tests: CACHE_P-1
#[test]
fn a_warm_run_reports_each_finding_at_the_line_of_a_cold_run() {
    let tree = TempTree::fixture("broken");
    let _ = fs::remove_dir_all(tree.dir().join(".sokf/cache"));
    age(tree.dir(), 10);
    let cold = tree.findings(CacheMode::Off);
    assert!(
        cold.iter()
            .any(|f| f.starts_with("/knowledge/schemas/globs.md:Some(7):")),
        "{cold:?}"
    );
    assert_eq!(tree.findings(CacheMode::On), cold);
    // The second warm run takes the schema from the store.
    assert_eq!(tree.findings(CacheMode::On), cold);
}

/// `check_once` runs one check through `cache` and returns the counts of its
/// sync.
fn check_once(tree: &TempTree, cache: &Cache) -> sokf_core::SyncStats {
    let root = tree.root();
    let grammar = sokf_core::load_grammar().unwrap();
    sokf_core::check_repository(
        &root,
        &config(tree),
        &grammar,
        &RepoFormats::default(),
        cache,
        false,
    )
    .unwrap();
    cache.stats()
}

// sokf-tests: CACHE-1_AC-51
// sokf-tests: CACHE-3_AC-4
// sokf-tests: CACHE-3_AC-11
// sokf-tests: CACHE-3_AC-12
#[test]
fn a_second_run_on_one_cache_compares_with_the_first_and_counts_alone() {
    let tree = TempTree::fixture("clean");
    let _ = fs::remove_dir_all(tree.dir().join(".sokf/cache"));
    age(tree.dir(), 10);
    let root = tree.root();
    let cache = Cache::open(&root, CacheMode::On);
    assert!(cache.is_fresh());
    let first = check_once(&tree, &cache);
    // The first run committed, so the cache no longer counts as fresh, and
    // the second run takes its hint instead of walking the repository.
    assert!(!cache.is_fresh());
    let rel = "knowledge/decisions/decision-single-binary.md";
    let text = tree.read(rel);
    tree.write(rel, &format!("{text}\nA line more.\n"));
    age_file(&tree.dir().join(rel), 10);
    cache.set_hint(vec![
        RepoPath::parse(&format!("/{rel}")).unwrap(),
        RepoPath::parse("/knowledge/req-cli.md").unwrap(),
    ]);
    let second = check_once(&tree, &cache);
    assert!(second.read < first.read, "{first:?} {second:?}");
    assert!(second.read <= 6, "{second:?}");
    // The counts cover the second run alone.
    assert!(second.queries < first.queries, "{first:?} {second:?}");
    // Of the two hinted files, only the one whose stat matches counts as
    // unchanged.
    assert_eq!(second.unchanged, 1, "{second:?}");
    // A store in memory stays fresh after a commit, so each run with
    // `--no-cache` checks everything.
    let off = Cache::open(&root, CacheMode::Off);
    check_once(&tree, &off);
    assert!(off.is_fresh());
}

// sokf-tests: CACHE-1_AC-52
#[test]
fn a_removed_additional_file_leaves_no_output_in_the_store() {
    let tree = TempTree::fixture("clean");
    let _ = fs::remove_dir_all(tree.dir().join(".sokf/cache"));
    age(tree.dir(), 10);
    tree.validate(false, CacheMode::On);
    let outputs = || {
        let store = rusqlite::Connection::open(
            tree.dir()
                .join(sokf_core::CACHE_DIR)
                .join(sokf_core::STORE_FILE),
        )
        .unwrap();
        store
            .query_row(
                "SELECT count(*) FROM outputs WHERE path = '/README.md'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
    };
    assert_eq!(outputs(), 1);
    let text = tree.read("README.md");
    fs::remove_file(tree.dir().join("README.md")).unwrap();
    tree.validate(false, CacheMode::On);
    tree.validate(false, CacheMode::On);
    assert_eq!(outputs(), 0);
    // A run with a hint that names the removed file keeps no output either.
    tree.write("README.md", &text);
    age(tree.dir(), 10);
    tree.validate(false, CacheMode::On);
    assert_eq!(outputs(), 1);
    fs::remove_file(tree.dir().join("README.md")).unwrap();
    common::incremental::run_hinted(&tree, &["/README.md"]);
    assert_eq!(outputs(), 0);
}
