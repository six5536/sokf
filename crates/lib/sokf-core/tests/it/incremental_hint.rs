//! These tests check that a run with a hint lists each hinted path as the
//! walk of the repository lists it (contract-cache). Each test fills the
//! store, changes the repository, and compares the tree that the hint
//! builds with the tree of a walk, and the findings of the hinted run with
//! those of a run without the cache.

use std::fs;

use common::{
    TempTree, age, error_lines,
    incremental::{run, run_hinted},
};
use sokf_core::{Cache, CacheMode, RepoPath, collect, load_config};

use crate::common;

/// `filled` copies the clean fixture, writes `config` as its configuration
/// when there is one, and fills the store with one run.
fn filled(config: Option<&str>, extra: &[(&str, &str)]) -> TempTree {
    let tree = TempTree::fixture("clean");
    if let Some(config) = config {
        tree.write(".sokf/config.toml", config);
    }
    for (rel, text) in extra {
        tree.write(rel, text);
    }
    age(tree.dir(), 10);
    run(&tree);
    tree
}

/// `assert_same_tree` builds the tree of a run with `hint` and the tree of
/// a walk, and asserts that they list the same files and directories.
fn assert_same_tree(tree: &TempTree, hint: &[&str]) {
    let root = tree.root();
    let config = load_config(&root).unwrap();
    let cache = Cache::open(&root, CacheMode::On);
    let hint: Vec<RepoPath> = hint.iter().map(|p| RepoPath::parse(p).unwrap()).collect();
    let hinted = cache.tree_from_store(&root, &config, &hint).unwrap();
    let walked = collect(&root, &config).unwrap();
    assert_eq!(hinted.files, walked.files);
    assert_eq!(hinted.dirs, walked.dirs);
}

/// `cold_errors` returns the errors of a run without the cache, in the
/// form that `run_hinted` returns them.
fn cold_errors(tree: &TempTree) -> Vec<String> {
    error_lines(&tree.validate_all(false, CacheMode::Off))
}

/// `broken` appends a link to a missing document to `rel`.
fn broken(tree: &TempTree, rel: &str) {
    let text = tree.read(rel);
    tree.write(
        rel,
        &format!("{text}\nSee [nothing][sokf:no-such-document].\n"),
    );
}

// sokf-tests: CACHE-1_AC-46
#[test]
fn an_exclude_inside_a_root_hides_no_hinted_file() {
    let tree = filled(
        Some("[repository]\nexclude = [\"/knowledge/decisions/**\"]\n"),
        &[],
    );
    let decision = "knowledge/decisions/decision-single-binary.md";
    broken(&tree, decision);
    assert_same_tree(&tree, &[&format!("/{decision}")]);
    let (_, errors, _) = run_hinted(&tree, &[&format!("/{decision}")]);
    assert!(
        errors.iter().any(|e| e.contains("decision-single-binary")),
        "{errors:?}"
    );
    assert_eq!(errors, cold_errors(&tree));
}

// sokf-tests: CACHE-1_AC-46
#[test]
fn a_hinted_file_that_the_walk_prunes_stays_out_of_the_tree() {
    let tree = filled(
        Some("[repository]\nexclude = [\"/vendor\"]\n"),
        &[
            (".gitignore", "/ignored/\n"),
            ("vendor/x.md", "x\n"),
            ("ignored/y.md", "y\n"),
            (".git/info/exclude", ""),
        ],
    );
    tree.write("vendor/x.md", "x, changed\n");
    tree.write("ignored/y.md", "y, changed\n");
    tree.write(".git/config", "");
    assert_same_tree(
        &tree,
        &[
            "/vendor/x.md",
            "/ignored/y.md",
            "/.sokf/cache/sokf.sqlite",
            "/.git/config",
        ],
    );
}

// sokf-tests: CACHE-1_AC-47
// sokf-tests: CACHE-1_AC-23
// sokf-tests: CACHE-1_AC-49
// sokf-tests: CACHE-1_AC-50
#[test]
fn a_hinted_directory_stands_for_every_path_under_it() {
    let tree = filled(
        None,
        &[
            ("src/lib/a.rs", "// a\n"),
            ("knowledge/a/index.md", "x\n"),
            ("knowledge/a-b/index.md", "x\n"),
        ],
    );
    // A directory that was deleted takes its files and its index with it.
    fs::remove_dir_all(tree.dir().join("knowledge/decisions")).unwrap();
    assert_same_tree(&tree, &["/knowledge/decisions"]);
    let (_, errors, _) = run_hinted(&tree, &["/knowledge/decisions"]);
    assert_eq!(errors, cold_errors(&tree));
    // A deleted directory beside one whose name sorts between it and its
    // files leaves the other one alone.
    fs::remove_dir_all(tree.dir().join("knowledge/a")).unwrap();
    assert_same_tree(&tree, &["/knowledge/a"]);
    run_hinted(&tree, &["/knowledge/a"]);
    // A new directory brings its files and the directories above them,
    // and one outside the roots brings its files too.
    tree.write("knowledge/drafts/deep/a.md", "x\n");
    tree.write("src/lib/b.rs", "// b\n");
    assert_same_tree(&tree, &["/knowledge/drafts/deep/a.md", "/src/lib"]);
    // A hint of the repository root lists everything again.
    assert_same_tree(&tree, &["/"]);
}

#[cfg(unix)]
// sokf-tests: CACHE-1_AC-50
#[test]
fn a_directory_reached_through_a_link_leaves_the_tree() {
    let tree = filled(None, &[]);
    // The link points out of the repository, so no walk lists its target.
    let elsewhere = TempTree::empty("link-target");
    let outside = elsewhere.dir().join("issues");
    fs::rename(tree.dir().join("knowledge/issues"), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, tree.dir().join("knowledge/issues")).unwrap();
    let hint = [
        "/knowledge/issues/index.md",
        "/knowledge/issues/open/index.md",
        "/knowledge/issues/open/issue-001-retry.md",
        "/knowledge/issues/closed/index.md",
    ];
    assert_same_tree(&tree, &hint);
    let (_, errors, _) = run_hinted(&tree, &hint);
    assert_eq!(errors, cold_errors(&tree));
}

#[cfg(unix)]
// sokf-tests: CACHE-1_AC-46
#[test]
fn a_hinted_link_stays_out_of_the_tree() {
    let tree = filled(None, &[]);
    std::os::unix::fs::symlink("overview.md", tree.dir().join("knowledge/link.md")).unwrap();
    std::os::unix::fs::symlink("main.rs", tree.dir().join("src/link.rs")).unwrap();
    assert_same_tree(&tree, &["/knowledge/link.md", "/src/link.rs"]);
}

#[cfg(unix)]
// sokf-tests: CACHE-1_AC-48
#[test]
fn a_hinted_file_whose_metadata_cannot_be_read_is_listed_as_the_walk_lists_it() {
    use std::os::unix::fs::PermissionsExt;
    let tree = filled(None, &[]);
    let dir = tree.dir().join("knowledge/decisions");
    // Without search permission, the directory lists its entries, but the
    // metadata of each entry can't be read.
    // The other files of the directory keep their stored stat, because the
    // hint leaves them out.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o644)).unwrap();
    let root = tree.root();
    let config = load_config(&root).unwrap();
    let path = RepoPath::parse("/knowledge/decisions/decision-single-binary.md").unwrap();
    let hinted = Cache::open(&root, CacheMode::On).tree_from_store(
        &root,
        &config,
        std::slice::from_ref(&path),
    );
    let walked = collect(&root, &config);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let (hinted, walked) = (hinted.unwrap(), walked.unwrap());
    assert_eq!(hinted.file(&path), walked.file(&path));
    assert!(hinted.file(&path).is_some());
}

/// `findings_of` runs `validate` on `tree` and returns its fixes and its
/// errors, each as a line. With `changed`, the run goes through the cache
/// with `--changed` set to those paths, and without, it runs with no cache.
fn findings_of(tree: &TempTree, changed: Option<&[&str]>, fix: bool) -> Vec<String> {
    let (report, _) = sokf_core::validate(
        &tree.root(),
        &sokf_core::ValidateOptions {
            paths: vec![],
            cwd: tree.dir().to_path_buf(),
            fix,
            cache: if changed.is_some() {
                CacheMode::On
            } else {
                CacheMode::Off
            },
            query: None,
            changed: changed
                .unwrap_or_default()
                .iter()
                .map(std::path::PathBuf::from)
                .collect(),
            home: None,
        },
    )
    .unwrap();
    report
        .fixes
        .iter()
        .map(|f| format!("{f:?}"))
        .chain(error_lines(&report))
        .collect()
}

/// `assert_hint_matches_cold` builds two copies of the clean fixture. It
/// runs `before` on each, fills the store of the first, and runs `change`
/// on both. It then runs the first with `--changed` set to `changed` and
/// the second with no cache, and asserts that both report the same fixes
/// and errors and leave the same files.
fn assert_hint_matches_cold(
    before: impl Fn(&TempTree),
    change: impl Fn(&TempTree),
    changed: &[&str],
    fix: bool,
) {
    let (hinted, cold) = (filled(None, &[]), TempTree::fixture("clean"));
    for tree in [&hinted, &cold] {
        before(tree);
    }
    run(&hinted);
    for tree in [&hinted, &cold] {
        change(tree);
    }
    assert_eq!(
        findings_of(&hinted, Some(changed), fix),
        findings_of(&cold, None, fix)
    );
    assert_eq!(hinted.files(), cold.files());
}

/// `append` adds a line to the file at `rel`.
fn append(tree: &TempTree, rel: &str, line: &str) {
    let text = tree.read(rel);
    tree.write(rel, &format!("{text}{line}\n"));
}

// sokf-tests: CACHE-1_AC-53
#[test]
fn a_hint_that_leaves_out_a_removed_file_falls_back_to_a_walk() {
    // The readme includes a region of the source, so an edit of the source
    // makes the readme dirty, and the run reads it.
    assert_hint_matches_cold(
        |_| {},
        |t| {
            fs::remove_file(t.dir().join("README.md")).unwrap();
            append(t, "src/main.rs", "// An edit.");
        },
        &["src/main.rs"],
        false,
    );
    // The issue links the decision, so an edit of the issue reads it.
    assert_hint_matches_cold(
        |_| {},
        |t| {
            fs::remove_file(
                t.dir()
                    .join("knowledge/decisions/decision-single-binary.md"),
            )
            .unwrap();
            append(
                t,
                "knowledge/issues/open/issue-001-retry.md",
                "A line more.",
            );
        },
        &["knowledge/issues/open/issue-001-retry.md"],
        false,
    );
}

// sokf-tests: CACHE-1_AC-53
#[test]
fn a_fix_with_a_hint_that_leaves_out_a_removed_file_falls_back_to_a_walk() {
    // A document with a finding is removed and left out of the hint, so the
    // plan of the fix would read it.
    assert_hint_matches_cold(
        |t| broken(t, "knowledge/req-cli.md"),
        |t| {
            fs::remove_file(t.dir().join("knowledge/req-cli.md")).unwrap();
            append(t, "src/main.rs", "// An edit.");
        },
        &["src/main.rs"],
        true,
    );
}

// sokf-tests: CACHE-1_AC-54
// sokf-tests: CACHE-1_AC-55
#[test]
fn a_fix_with_a_hint_never_writes_into_a_removed_directory() {
    // The store records a finding on the stale index of a directory, and
    // the directory is then removed and left out of the hint.
    assert_hint_matches_cold(
        |t| append(t, "knowledge/decisions/index.md", "- stale"),
        |t| {
            fs::remove_dir_all(t.dir().join("knowledge/decisions")).unwrap();
            append(t, "src/main.rs", "// An edit.");
        },
        &["src/main.rs"],
        true,
    );
}
