//! These tests check the lookups that use the store (contract-cache). When
//! the caller gives a run a hint, which is a list of the paths that the caller
//! says have changed, the run checks only the paths in the hint for changes, and the next full run finds the other changes. `locate`, `graph`,
//! and `new` check the
//! changes, commit the result, and answer from the store, so the `validate`
//! that follows has nothing to check.

use std::fs;

use common::{
    TempTree, age,
    incremental::{cold, edit, run, run_hinted, set},
    new_options,
};
use sokf_core::CacheMode;

use crate::common;

// sokf-tests: CACHE-1_AC-23
// sokf-tests: CLI-4_AC-21
#[test]
fn a_hint_stats_the_named_paths_alone_and_the_next_full_run_finds_the_rest() {
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    // Two documents change, and the hint lists only one of them, so the run
    // keeps the store's record of the other as it was.
    let decision = "knowledge/decisions/decision-single-binary.md";
    let text = tree.read(decision);
    edit(&tree, decision, &format!("{text}\nA line more.\n"));
    let req = "knowledge/req-cli.md";
    let text = tree.read(req);
    edit(
        &tree,
        req,
        &text.replacen("title: CLI requirement", "title: The CLI", 1),
    );
    let (dirty, findings, read) = run_hinted(&tree, &["/knowledge/req-cli.md"]);
    assert_eq!(
        dirty,
        set(&["/knowledge/overview.md", "/knowledge/req-cli.md"]),
        "{dirty:?}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f.contains("decision-single-binary")),
        "{findings:?}"
    );
    // The run reads the requirement, the overview, the files that they read,
    // and the two indexes.
    assert!(read <= 12, "read {read} files");
    // When the hint lists a removed file, the run sees the removal.
    fs::remove_file(tree.dir().join("knowledge/issues/open/issue-001-retry.md")).unwrap();
    let (dirty, findings, _) = run_hinted(&tree, &["/knowledge/issues/open/issue-001-retry.md"]);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert!(
        findings
            .iter()
            .any(|f| f.starts_with("/knowledge/issues/open/index.md")),
        "{findings:?}"
    );
    // The next run without a hint finds the decision's change.
    let (dirty, _, findings, _) = run(&tree);
    assert!(
        dirty.contains("/knowledge/decisions/decision-single-binary.md"),
        "{dirty:?}"
    );
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-1_AC-21
// sokf-tests: CACHE-1_AC-34
// sokf-tests: CACHE-1_AC-36
// sokf-tests: CACHE-1_AC-37
// sokf-tests: CACHE_P-1
#[test]
fn locate_and_new_answer_from_the_store_and_check_what_changed() {
    use sokf_core::{NewOptions, locate, new};
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    run(&tree);
    let root = tree.root();
    let on = |ids: &[&str]| {
        locate(
            &root,
            &ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            false,
            CacheMode::On,
        )
    };
    // Ids and keys, looked up in the store's rows, resolve to the same paths
    // as in a cold run.
    for ids in [&["req-cli"][..], &["#cli-1_ac-2"], &["overview", "#cli-1"]] {
        let warm = on(ids).unwrap();
        let cold = locate(
            &root,
            &ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            false,
            CacheMode::Off,
        )
        .unwrap();
        assert_eq!(warm, cold, "{ids:?}");
    }
    // After the requirement's id changes, the lookup checks the change, so
    // the new id resolves and the old one doesn't.
    let rel = "knowledge/req-cli.md";
    let text = tree.read(rel);
    edit(&tree, rel, &text.replacen("id: req-cli", "id: req-cmd", 1));
    assert!(on(&["req-cmd"]).is_ok());
    assert!(on(&["req-cli"]).is_err());
    // The lookup's check updated the store. A key of the changed document now
    // gives the same result as in a cold run, through both `locate` and
    // `graph`.
    let key = ["#cli-1_ac-2".to_string()];
    assert_eq!(
        on(&["#cli-1_ac-2"]).unwrap(),
        locate(&root, &key, false, CacheMode::Off).unwrap()
    );
    let graph_of = |cache: CacheMode| {
        sokf_core::graph(
            &root,
            &sokf_core::GraphOptions {
                query: sokf_core::GraphQuery {
                    start: key.to_vec(),
                    depth: 1,
                    ..sokf_core::GraphQuery::default()
                },
                cwd: tree.dir().to_path_buf(),
                cache,
            },
        )
        .map(|(g, r)| serde_json::to_string_pretty(&sokf_core::graph::bounded(&g, &r)).unwrap())
        .unwrap()
    };
    assert_eq!(graph_of(CacheMode::On), graph_of(CacheMode::Off));
    // `new` takes the next sequence number of an issue from the ids in the
    // store, and gets the same number as in a cold run.
    let next = |cache: CacheMode| {
        new(
            &root,
            &NewOptions {
                slug: Some("again".into()),
                states: vec![("state".into(), "open".into())],
                id_only: true,
                cache,
                ..new_options(tree.dir(), "Issue")
            },
        )
        .map(|r| r.id)
    };
    assert_eq!(next(CacheMode::On).unwrap(), next(CacheMode::Off).unwrap());
    // The first lookup checked the change and committed the result, so the
    // `validate` that follows has nothing to check and reports the same
    // findings as a cold run.
    let (dirty, _, findings, read) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(read, 1, "only the manifest");
    assert_eq!(findings, cold(&tree));
}

// sokf-tests: CACHE-1_AC-34
// sokf-tests: CACHE-1_AC-36
#[test]
fn a_lookup_on_a_new_store_runs_the_first_check_once() {
    use sokf_core::locate;
    let tree = TempTree::fixture("clean");
    age(tree.dir(), 10);
    let root = tree.root();
    assert!(locate(&root, &["req-cli".to_string()], false, CacheMode::On).is_ok());
    // The `validate` that follows finds that the lookup has already run the
    // check.
    let (dirty, full, findings, read) = run(&tree);
    assert!(!full);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(read, 1, "only the manifest");
    assert_eq!(findings, cold(&tree));
}
