//! These tests run `sokf new` through the core in git repositories with
//! other branches and worktrees, and check that a new number avoids the
//! numbers that they gave their documents.

use common::{TempTree, new_options};
use sokf_core::new;

use crate::{
    common,
    new::{err, issue},
};

/// `git` runs `git` with `args` in `dir`, with an identity for a machine
/// that has none, and returns its standard output. It clears each `GIT_`
/// variable that it inherits, such as the `GIT_INDEX_FILE` that git sets
/// for a commit hook, so that it reads and writes only the repository in
/// `dir`, reads no system configuration, such as the `core.autocrlf=true` of
/// Git for Windows, and signs nothing. It panics when git fails.
fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let mut cmd = std::process::Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1");
    let out = cmd
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// `committed` turns the tree into a git repository on `main`, with its
/// files in one commit.
fn committed(tree: &TempTree) {
    git(tree.dir(), &["init", "-q", "-b", "main"]);
    git(tree.dir(), &["add", "-A"]);
    git(tree.dir(), &["commit", "-q", "-m", "start"]);
}

/// `numbered` is the text of an issue of the `new` fixture with `id`.
fn numbered(id: &str) -> String {
    format!(
        "---\ntype: Issue\nid: {id}\ntitle: Elsewhere\nstate: open\nkind: bug\n---\n\n# Elsewhere\n\nThe index writer drops the entry of a moved file.\n\n## Reproduction\n\n1. Move a document with `--fix`.\n"
    )
}

/// `on_branch` commits an issue with `id` on a new branch `name`, cut from
/// `main`, and returns to `main`.
fn on_branch(tree: &TempTree, name: &str, id: &str) {
    git(tree.dir(), &["switch", "-q", "-c", name]);
    tree.write(&format!("knowledge/issues/open/bug/{id}.md"), &numbered(id));
    git(tree.dir(), &["add", "-A"]);
    git(tree.dir(), &["commit", "-q", "-m", id]);
    git(tree.dir(), &["switch", "-q", "main"]);
}

// sokf-tests: CLI-6_AC-4
#[test]
fn a_number_on_another_branch_is_left_free() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    on_branch(&tree, "other", "issue-002-elsewhere");
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-003-here");
}

// sokf-tests: CLI-6_AC-4
#[test]
fn a_number_on_a_remote_tracking_branch_is_left_free() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    on_branch(&tree, "pushed", "issue-004-pushed");
    // The commit of the branch stays reachable only as a remote-tracking
    // branch, as after a fetch of a colleague's work.
    let commit = git(tree.dir(), &["rev-parse", "pushed"]);
    git(
        tree.dir(),
        &["update-ref", "refs/remotes/origin/pushed", commit.trim()],
    );
    git(tree.dir(), &["branch", "-q", "-D", "pushed"]);
    // A remote's `HEAD` points at one of its branches, and names no branch.
    git(
        tree.dir(),
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/pushed",
        ],
    );
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-005-here");
    // The branch is named as itself, not by the symbolic ref before it.
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("here".into());
    o.seq = Some(4);
    let e = err(new(&tree.root(), &o));
    assert!(e.contains("on the branch `origin/pushed`"), "{e}");
}

// sokf-tests: CLI-6_AC-4
#[test]
fn an_uncommitted_number_in_another_worktree_is_left_free() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    let other = TempTree::empty("new-worktree");
    let wt = other.dir().join("wt");
    git(
        tree.dir(),
        &["worktree", "add", "-q", "-b", "agent", wt.to_str().unwrap()],
    );
    // The other agent's new issue isn't committed, so it is on no branch.
    std::fs::write(
        wt.join("knowledge/issues/open/bug/issue-002-agent.md"),
        numbered("issue-002-agent"),
    )
    .unwrap();
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-003-here");
}

// A number of another worktree that its index holds, and that its working
// tree changed again, is left free, though the file has no blob yet.
// sokf-tests: CLI-6_AC-4
#[test]
fn a_staged_and_changed_number_in_another_worktree_is_left_free() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    let other = TempTree::empty("new-worktree-staged");
    let wt = other.dir().join("wt");
    git(
        tree.dir(),
        &["worktree", "add", "-q", "-b", "agent", wt.to_str().unwrap()],
    );
    let file = wt.join("knowledge/issues/open/bug/issue-002-agent.md");
    std::fs::write(&file, numbered("issue-002-agent")).unwrap();
    git(&wt, &["add", "-A"]);
    std::fs::write(&file, numbered("issue-002-agent") + "\nMore.\n").unwrap();
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-003-here");
}

// sokf-tests: CLI-6_AC-70
#[test]
fn a_given_number_on_another_branch_is_refused() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    on_branch(&tree, "other", "issue-002-elsewhere");
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("here".into());
    o.seq = Some(2);
    let e = err(new(&tree.root(), &o));
    assert!(e.contains("`issue-002-elsewhere`"), "{e}");
    assert!(
        e.contains("on the branch `other` already has the number 2"),
        "{e}"
    );
}

// sokf-tests: CLI-6_AC-75
#[test]
fn a_broken_file_on_another_branch_is_left_out() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    git(tree.dir(), &["switch", "-q", "-c", "broken"]);
    tree.write(
        "knowledge/issues/open/bug/issue-009-broken.md",
        "---\ntype: Issue\nid: [unclosed\n---\n\n# Broken\n",
    );
    tree.write(
        "knowledge/issues/open/bug/issue-008-off-template.md",
        "---\ntype: Issue\nid: not-an-issue-id\n---\n\n# Off\n",
    );
    git(tree.dir(), &["add", "-A"]);
    git(tree.dir(), &["commit", "-q", "-m", "broken"]);
    git(tree.dir(), &["switch", "-q", "main"]);
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-002-here");
}

// sokf-tests: CLI-6_AC-74
#[test]
fn outside_git_the_working_tree_alone_gives_the_number() {
    // The fixture's copy is no git repository.
    let tree = TempTree::fixture("new");
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-002-here");
}

// sokf-tests: CLI-6_AC-70
#[test]
fn a_given_number_in_another_worktree_is_refused() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    let other = TempTree::empty("new-worktree-seq");
    let wt = other.dir().join("wt");
    git(
        tree.dir(),
        &["worktree", "add", "-q", "-b", "agent", wt.to_str().unwrap()],
    );
    std::fs::write(
        wt.join("knowledge/issues/open/bug/issue-002-agent.md"),
        numbered("issue-002-agent"),
    )
    .unwrap();
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("here".into());
    o.seq = Some(2);
    let e = err(new(&tree.root(), &o));
    assert!(e.contains("`issue-002-agent`"), "{e}");
    assert!(e.contains("in the worktree"), "{e}");
}

// sokf-tests: CLI-6_AC-74
#[test]
fn a_root_below_the_top_of_the_repository_reads_its_own_tree_alone() {
    // The store lies in `inner/`, and the git repository is the directory
    // above it, so `/knowledge` would mean another directory to git.
    let tree = TempTree::fixture("new");
    let inner = tree.dir().join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    std::fs::rename(tree.dir().join("knowledge"), inner.join("knowledge")).unwrap();
    committed(&tree);
    on_branch(&tree, "other", "issue-005-elsewhere");
    let root = sokf_core::RepoRoot::new(inner.clone());
    let mut o = new_options(&inner, "Issue");
    o.slug = Some("here".into());
    assert_eq!(new(&root, &o).unwrap().id, "issue-002-here");
}

// sokf-tests: CLI-6_AC-69
#[test]
fn a_number_that_the_working_tree_frees_stays_free() {
    // The run's own tree stands for the commit that it checks out, so a
    // committed document that the tree deletes frees its number.
    let tree = TempTree::fixture("new");
    committed(&tree);
    std::fs::remove_file(
        tree.dir()
            .join("knowledge/issues/open/bug/issue-001-first.md"),
    )
    .unwrap();
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("again".into());
    o.seq = Some(1);
    assert_eq!(new(&tree.root(), &o).unwrap().id, "issue-001-again");
}

// sokf-tests: CLI-6_AC-4
#[test]
fn a_nested_store_keeps_its_numbers_to_itself() {
    let tree = TempTree::fixture("new");
    tree.write(
        ".sokf/config.toml",
        "[knowledge]\nroots = [\"/knowledge\", \"/knowledge/vendor\"]\n",
    );
    tree.write(
        "knowledge/vendor/manifest.sokf.yaml",
        "sokf: \"0.6\"\nname: vendor\nscope: test\n",
    );
    committed(&tree);
    git(tree.dir(), &["switch", "-q", "-c", "vendored"]);
    tree.write(
        "knowledge/vendor/issue-050-theirs.md",
        &numbered("issue-050-theirs"),
    );
    git(tree.dir(), &["add", "-A"]);
    git(tree.dir(), &["commit", "-q", "-m", "vendored"]);
    git(tree.dir(), &["switch", "-q", "main"]);
    // Only the outer store declares `Issue`, and the inner one holds
    // `issue-050-theirs`, which the outer store's numbers leave out.
    let r = issue(&tree, "ours", &[]).unwrap();
    assert_eq!(r.id, "issue-002-ours");
}

// sokf-tests: CLI-6_AC-21
#[test]
fn a_next_number_too_wide_names_the_branch_that_holds_the_highest() {
    let tree = TempTree::fixture("new");
    committed(&tree);
    on_branch(&tree, "big", "issue-999-big");
    let e = err(issue(&tree, "here", &[]));
    assert!(
        e.contains("the next number 1000 needs more than 3 digits, because `issue-999-big`"),
        "{e}"
    );
    assert!(e.contains("on the branch `big` has 999"), "{e}");
}

// sokf-tests: CLI-6_AC-70
#[test]
fn a_number_freed_on_this_branch_stays_free_while_an_older_branch_keeps_the_file() {
    // `older` was cut before `main` deleted the issue, so merging it doesn't
    // bring the file back, and its copy holds no number in use.
    let tree = TempTree::fixture("new");
    committed(&tree);
    git(tree.dir(), &["branch", "older"]);
    git(
        tree.dir(),
        &["rm", "-q", "knowledge/issues/open/bug/issue-001-first.md"],
    );
    git(tree.dir(), &["commit", "-q", "-m", "drop the first issue"]);
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("again".into());
    o.seq = Some(1);
    assert_eq!(new(&tree.root(), &o).unwrap().id, "issue-001-again");
}

// sokf-tests: CLI-6_AC-4
#[test]
fn an_ignored_draft_in_another_worktree_holds_no_number() {
    let tree = TempTree::fixture("new");
    tree.write(".gitignore", "drafts/\n");
    committed(&tree);
    let other = TempTree::empty("new-worktree-ignored");
    let wt = other.dir().join("wt");
    git(
        tree.dir(),
        &["worktree", "add", "-q", "-b", "agent", wt.to_str().unwrap()],
    );
    let drafts = wt.join("knowledge/issues/open/bug/drafts");
    std::fs::create_dir_all(&drafts).unwrap();
    std::fs::write(
        drafts.join("issue-900-scratch.md"),
        numbered("issue-900-scratch"),
    )
    .unwrap();
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-002-here");
}

// sokf-tests: CLI-6_AC-76
#[test]
fn a_partial_clone_takes_a_branch_number_from_the_file_name() {
    // The source serves a clone without blobs, and its branch `pushed`
    // adds an issue whose blob the clone never fetches.
    let source = TempTree::fixture("new");
    committed(&source);
    git(source.dir(), &["config", "uploadpack.allowFilter", "true"]);
    on_branch(&source, "pushed", "issue-007-pushed");
    let clones = TempTree::empty("new-partial");
    let clone = clones.dir().join("clone");
    let url = format!("file://{}", source.dir().display());
    git(
        clones.dir(),
        &[
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            &url,
            clone.to_str().unwrap(),
        ],
    );
    git(&clone, &["checkout", "-q", "main"]);
    let root = sokf_core::RepoRoot::new(clone.clone());
    let mut o = new_options(&clone, "Issue");
    o.slug = Some("here".into());
    assert_eq!(new(&root, &o).unwrap().id, "issue-008-here");
}

// sokf-tests: CLI-6_AC-21
#[test]
fn a_next_number_too_wide_in_the_own_tree_names_no_branch() {
    // The tree and a branch cut before it both gave a document 999, and the
    // error leaves the branch out, because the tree itself holds the cause.
    let tree = TempTree::fixture("new");
    committed(&tree);
    on_branch(&tree, "twin", "issue-999-twin");
    tree.write(
        "knowledge/issues/open/bug/issue-999-big.md",
        &numbered("issue-999-big"),
    );
    git(tree.dir(), &["add", "-A"]);
    git(tree.dir(), &["commit", "-q", "-m", "big"]);
    let e = err(issue(&tree, "here", &[]));
    assert!(
        e.contains("the next number 1000 needs more than 3 digits"),
        "{e}"
    );
    assert!(!e.contains("on the branch"), "{e}");
}

// sokf-tests: CLI-6_AC-76
#[test]
fn a_partial_clone_counts_a_file_that_a_branch_moved_and_changed() {
    // Closing an issue moves its file and changes it; in a clone without
    // blobs, no rename detection may read them.
    let source = TempTree::fixture("new");
    committed(&source);
    git(source.dir(), &["config", "uploadpack.allowFilter", "true"]);
    git(source.dir(), &["switch", "-q", "-c", "closing"]);
    std::fs::create_dir_all(source.dir().join("knowledge/issues/closed")).unwrap();
    git(
        source.dir(),
        &[
            "mv",
            "knowledge/issues/open/bug/issue-001-first.md",
            "knowledge/issues/closed/issue-001-first.md",
        ],
    );
    let closed = source.read("knowledge/issues/closed/issue-001-first.md");
    source.write(
        "knowledge/issues/closed/issue-001-first.md",
        &closed.replace("state: open", "state: closed"),
    );
    source.write(
        "knowledge/issues/open/bug/issue-007-new.md",
        &numbered("issue-007-new"),
    );
    git(source.dir(), &["add", "-A"]);
    git(source.dir(), &["commit", "-q", "-m", "close"]);
    git(source.dir(), &["switch", "-q", "main"]);
    let clones = TempTree::empty("new-partial-move");
    let clone = clones.dir().join("clone");
    let url = format!("file://{}", source.dir().display());
    git(
        clones.dir(),
        &[
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            &url,
            clone.to_str().unwrap(),
        ],
    );
    git(&clone, &["checkout", "-q", "main"]);
    let root = sokf_core::RepoRoot::new(clone.clone());
    let mut o = new_options(&clone, "Issue");
    o.slug = Some("here".into());
    assert_eq!(new(&root, &o).unwrap().id, "issue-008-here");
}

// sokf-tests: CLI-6_AC-4
#[test]
fn the_scan_leaves_the_index_of_another_worktree_alone() {
    // Another agent's index must stay as it is, or its next commit could
    // meet a lock that the scan took.
    let tree = TempTree::fixture("new");
    committed(&tree);
    let other = TempTree::empty("new-worktree-index");
    let wt = other.dir().join("wt");
    git(
        tree.dir(),
        &["worktree", "add", "-q", "-b", "agent", wt.to_str().unwrap()],
    );
    // A touched tracked file makes a porcelain diff refresh the index.
    let first = wt.join("knowledge/issues/open/bug/issue-001-first.md");
    let text = std::fs::read(&first).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&first, text).unwrap();
    let index = tree.dir().join(".git/worktrees/wt/index");
    let before = std::fs::read(&index).unwrap();
    let stamp = std::fs::metadata(&index).unwrap().modified().unwrap();
    let r = issue(&tree, "here", &[]).unwrap();
    assert_eq!(r.id, "issue-002-here");
    assert_eq!(std::fs::read(&index).unwrap(), before);
    assert_eq!(
        std::fs::metadata(&index).unwrap().modified().unwrap(),
        stamp
    );
}

// sokf-tests: CLI-6_AC-4
// sokf-tests: CLI-6_AC-70
#[test]
fn only_the_number_that_a_branch_ends_with_counts() {
    // `wip` took 900 by mistake and renumbered the document to 005 in a
    // later commit, so merging it brings 005 and no 900.
    let tree = TempTree::fixture("new");
    committed(&tree);
    on_branch(&tree, "wip", "issue-900-typo");
    git(tree.dir(), &["switch", "-q", "wip"]);
    git(
        tree.dir(),
        &[
            "mv",
            "knowledge/issues/open/bug/issue-900-typo.md",
            "knowledge/issues/open/bug/issue-005-fixed.md",
        ],
    );
    let path = "knowledge/issues/open/bug/issue-005-fixed.md";
    let fixed = tree.read(path).replace("issue-900-typo", "issue-005-fixed");
    tree.write(path, &fixed);
    git(tree.dir(), &["commit", "-q", "-am", "renumber"]);
    git(tree.dir(), &["switch", "-q", "main"]);
    assert_eq!(issue(&tree, "here", &[]).unwrap().id, "issue-006-here");
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("again".into());
    o.seq = Some(900);
    assert_eq!(new(&tree.root(), &o).unwrap().id, "issue-900-again");
}

// sokf-tests: CLI-6_AC-4
// sokf-tests: CLI-6_AC-70
#[test]
fn a_criss_cross_history_still_counts_only_what_a_branch_gave() {
    // `b` merges `main`, and `main` merges a branch cut from `b`, so `main`
    // and `b` have two merge bases.
    let tree = TempTree::fixture("new");
    committed(&tree);
    let note = |name: &str| {
        tree.write(&format!("notes/{name}.txt"), name);
        git(tree.dir(), &["add", "-A"]);
        git(tree.dir(), &["commit", "-q", "-m", name]);
    };
    git(tree.dir(), &["switch", "-q", "-c", "b"]);
    note("b1");
    git(tree.dir(), &["switch", "-q", "-c", "c"]);
    note("c1");
    git(tree.dir(), &["switch", "-q", "main"]);
    note("m1");
    git(tree.dir(), &["switch", "-q", "b"]);
    git(tree.dir(), &["merge", "-q", "--no-edit", "main"]);
    git(tree.dir(), &["switch", "-q", "main"]);
    git(tree.dir(), &["merge", "-q", "--no-edit", "c"]);
    // `b` then gives a document a number, and `main` frees one.
    on_branch_from(&tree, "b", "issue-007-b");
    git(
        tree.dir(),
        &["rm", "-q", "knowledge/issues/open/bug/issue-001-first.md"],
    );
    git(tree.dir(), &["commit", "-q", "-m", "free the first"]);
    assert_eq!(issue(&tree, "here", &[]).unwrap().id, "issue-008-here");
    let mut o = new_options(tree.dir(), "Issue");
    o.slug = Some("again".into());
    o.seq = Some(1);
    assert_eq!(new(&tree.root(), &o).unwrap().id, "issue-001-again");
}

/// `on_branch_from` commits an issue with `id` on the existing branch
/// `name`, and returns to `main`.
fn on_branch_from(tree: &TempTree, name: &str, id: &str) {
    git(tree.dir(), &["switch", "-q", name]);
    tree.write(&format!("knowledge/issues/open/bug/{id}.md"), &numbered(id));
    git(tree.dir(), &["add", "-A"]);
    git(tree.dir(), &["commit", "-q", "-m", id]);
    git(tree.dir(), &["switch", "-q", "main"]);
}
