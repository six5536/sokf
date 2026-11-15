//! This module holds the helpers for the tests of incremental runs: an
//! edit, a run through the cache, a run with a hint, and a run with no cache.

use std::collections::BTreeSet;

use sokf_core::{Cache, CacheMode, Loaded, Report, check_repository, load_config, load_grammar};

use super::{TempTree, age_file, error_lines, lines};

/// `edit` writes `rel` and sets its modification time ten seconds back, so
/// that the next run reads it again and later runs treat it as unchanged.
pub fn edit(tree: &TempTree, rel: &str, text: &str) {
    tree.write(rel, text);
    age_file(&tree.dir().join(rel), 10);
}

thread_local! {
    /// `HELD` lists the documents that the last run on this thread held in
    /// memory, whether it loaded them in full or kept them as summaries. It is
    /// per thread, so a test sees only its own runs, even when other tests run
    /// in parallel.
    static HELD: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `check` makes one incremental run through the cache, with a hint when
/// `changed` lists the paths that changed. It returns what the run loaded,
/// its finished report, and the cache, whose counts the caller can read.
pub fn check(tree: &TempTree, changed: Option<&[&str]>) -> (Loaded, Report, Cache) {
    let root = tree.root();
    let cache = Cache::open(&root, CacheMode::On);
    if let Some(changed) = changed {
        cache.set_hint(
            changed
                .iter()
                .map(|p| sokf_core::RepoPath::parse(p).unwrap())
                .collect(),
        );
    }
    let grammar = load_grammar().unwrap();
    let regexes = sokf_core::grammar::RegexCache::new();
    let formats = sokf_core::load_repo_formats(&root, &grammar, &regexes).unwrap();
    let (loaded, findings) = check_repository(
        &root,
        &load_config(&root).unwrap(),
        &grammar,
        &formats,
        &cache,
        false,
    )
    .unwrap();
    let mut report = Report::default();
    report.extend(findings);
    (loaded, report.finish(), cache)
}

/// `run` makes one incremental run through the cache and returns the
/// documents it checked, whether it checked them all, its findings, and the
/// number of files it read.
pub fn run(tree: &TempTree) -> (BTreeSet<String>, bool, Vec<String>, usize) {
    let (loaded, report, cache) = check(tree, None);
    HELD.set(
        loaded
            .model
            .documents()
            .map(|(_, d)| d.path.to_string())
            .collect(),
    );
    (
        loaded.dirty.iter().map(ToString::to_string).collect(),
        loaded.full,
        lines(&report),
        cache.stats().read,
    )
}

/// `held` returns the documents that the last run on this thread held in
/// memory (CACHE-3_AC-2).
pub fn held() -> BTreeSet<String> {
    HELD.with_borrow(|held| held.iter().cloned().collect())
}

/// `graph_json` returns the whole graph, cut off at depth one, as JSON. It
/// builds the graph with the cache set to `mode`.
pub fn graph_json(tree: &TempTree, mode: CacheMode) -> String {
    use sokf_core::{GraphQuery, graph::bounded};
    let (g, r) = super::graph_rows::graph_of(tree, GraphQuery::default(), mode).unwrap();
    serde_json::to_string(&bounded(&g, &r)).unwrap()
}

/// `cold` returns the findings of a run without the cache, in the form
/// that `run` returns them.
pub fn cold(tree: &TempTree) -> Vec<String> {
    tree.findings(CacheMode::Off)
}

pub fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(ToString::to_string).collect()
}

/// `run_hinted` makes one incremental run with a hint, so that the cache
/// checks the metadata of only the paths in `changed`. It returns the
/// documents it checked, its errors, and the number of files it read.
pub fn run_hinted(tree: &TempTree, changed: &[&str]) -> (BTreeSet<String>, Vec<String>, usize) {
    let (loaded, report, cache) = check(tree, Some(changed));
    (
        loaded.dirty.iter().map(ToString::to_string).collect(),
        error_lines(&report),
        cache.stats().read,
    )
}
