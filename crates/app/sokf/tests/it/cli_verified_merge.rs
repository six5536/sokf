//! End-to-end tests of the check of a lapsed `verified` entry that a merge
//! commit adds (`CLI-4_AC-105`): each test reviews a document on a branch,
//! merges the branch with `--no-ff`, and runs `sokf validate` on `main`.

use crate::{
    cli_verified::{NOTE_PATH, Repo, dated, lapsed, none, note},
    common::git,
};

/// `merge` merges `branch` into the branch that `repo` checks out, with a
/// merge commit on `date`, and returns the merge commit.
fn merge(repo: &Repo, date: &str, branch: &str) -> String {
    dated(
        &repo.0,
        date,
        &["merge", "-q", "--no-ff", "--no-edit", branch],
    );
    git(&repo.0, &["rev-parse", "HEAD"]).trim().to_string()
}

/// `review` writes the note on `main`, and then, on the new branch
/// `review`, commits an entry of `human:ann` with an `at` of 2026-01-05.
fn review(repo: &Repo) {
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "review"]);
    repo.write(NOTE_PATH, &note("First.", &[("human:ann", "2026-01-05")]));
    repo.commit("2026-01-05T12:00:00Z", "verify");
}

/// `change` commits a change to the note's body on 2026-01-06, after the
/// entry's `at`, and returns the commit.
fn change(repo: &Repo) -> String {
    repo.write(NOTE_PATH, &note("Second.", &[("human:ann", "2026-01-05")]));
    repo.commit("2026-01-06T00:00:00Z", "change")
}

// A change that the branch makes after the commit that adds the entry
// lapses the entry after a merge commit brings the branch in, as it does
// on the branch, and the warning names the branch's commit.
// sokf-tests: CLI-4_AC-105
#[test]
fn a_merge_keeps_the_lapse_of_its_branch() {
    let repo = Repo::new("merge-lapse", true);
    review(&repo);
    let commit = change(&repo);
    let out = repo.warnings(false);
    assert_eq!(lapsed(&out).len(), 1, "{out}");
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("/knowledge/note-a.md:7:"), "{out}");
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// A merge on the branch brings in a nested branch whose commit adds the
// entry, and the check follows both merges to that commit.
// sokf-tests: CLI-4_AC-105
#[test]
fn a_nested_merge_keeps_the_lapse_of_its_branch() {
    let repo = Repo::new("merge-nested", true);
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "topic"]);
    git(&repo.0, &["checkout", "-q", "-b", "review"]);
    repo.write(NOTE_PATH, &note("First.", &[("human:ann", "2026-01-05")]));
    repo.commit("2026-01-05T12:00:00Z", "verify");
    let commit = change(&repo);
    git(&repo.0, &["checkout", "-q", "topic"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-08T00:00:00Z", "topic");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// A change that the branch makes to a sidecar's asset after the commit
// that adds the sidecar's entry lapses the entry after the merge.
// sokf-tests: CLI-4_AC-105
#[test]
fn a_merge_keeps_the_lapse_by_the_asset_of_its_branch() {
    let repo = Repo::new("merge-asset", true);
    let sidecar = |verified: &str| {
        format!(
            "---\ntype: Dataset\nid: rates\ntitle: Rates\ndescription: Rates.\nresource: /knowledge/rates.csv\n{verified}---\n\nQuarterly rates.\n"
        )
    };
    repo.write("knowledge/rates.csv", "quarter,rate\nq3,0.25\n");
    repo.write("knowledge/rates.csv.md", &sidecar(""));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "review"]);
    let entry = "verified:\n  - { by: \"human:ann\", at: \"2026-01-05\" }\n";
    repo.write("knowledge/rates.csv.md", &sidecar(entry));
    repo.commit("2026-01-05T12:00:00Z", "verify");
    repo.write("knowledge/rates.csv", "quarter,rate\nq3,0.26\n");
    let commit = repo.commit("2026-01-06T00:00:00Z", "change the rates");
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("/knowledge/rates.csv.md:8:"), "{out}");
    assert!(
        found[0].contains("the asset `/knowledge/rates.csv`"),
        "{out}"
    );
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// An entry that the merge commit itself adds, which no commit of the
// branch adds, confirms the content of the merge, so no change of the
// branch lapses it.
// sokf-tests: CLI-4_AC-97
// sokf-tests: CLI-4_AC-105
#[test]
fn an_entry_that_the_merge_adds_keeps_the_merge_as_its_commit() {
    let repo = Repo::new("merge-adds", true);
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "review"]);
    repo.write(NOTE_PATH, &note("Second.", &[]));
    repo.commit("2026-01-06T00:00:00Z", "change");
    git(&repo.0, &["checkout", "-q", "main"]);
    git(
        &repo.0,
        &["merge", "-q", "--no-ff", "--no-commit", "review"],
    );
    repo.write(NOTE_PATH, &note("Second.", &[("human:ann", "2026-01-05")]));
    repo.commit("2026-01-07T00:00:00Z", "merge and verify");
    let parents = git(&repo.0, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(parents.split_whitespace().count(), 3, "{parents}");
    none(&repo.warnings(false));
}

// A merge after the review, of a branch that changed nothing after the
// commit that adds the entry, lapses nothing, though the merge's change
// to the document follows the entry's `at`.
// sokf-tests: CLI-4_AC-97
// sokf-tests: CLI-4_AC-105
#[test]
fn a_merge_after_the_review_lapses_nothing() {
    let repo = Repo::new("merge-current", true);
    review(&repo);
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    none(&repo.warnings(false));
}

// A change of the branch keeps the date of its own commit after the merge,
// so a change on the day of the review leaves an entry whose `at` gives
// only a date current, on `main` as on the branch.
// sokf-tests: CLI-4_AC-94
// sokf-tests: CLI-4_AC-105
#[test]
fn a_change_on_the_day_of_the_review_lapses_nothing_after_a_merge() {
    let repo = Repo::new("merge-same-day", true);
    review(&repo);
    repo.write(NOTE_PATH, &note("Second.", &[("human:ann", "2026-01-05")]));
    repo.commit("2026-01-05T18:00:00Z", "change");
    none(&repo.warnings(false));
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    none(&repo.warnings(false));
}

// The first parent moved the document after the branch parted, so the
// merged branch holds the document at the path that the merge moved it
// from.
// sokf-tests: CLI-4_AC-105
#[test]
fn a_merge_keeps_the_lapse_of_its_branch_after_a_move_on_main() {
    let repo = Repo::new("merge-moved", true);
    review(&repo);
    let commit = change(&repo);
    git(&repo.0, &["checkout", "-q", "main"]);
    std::fs::create_dir_all(repo.0.join("knowledge/moved")).unwrap();
    git(&repo.0, &["mv", NOTE_PATH, "knowledge/moved/note-a.md"]);
    repo.commit("2026-01-06T12:00:00Z", "move");
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("/knowledge/moved/note-a.md:7:"),
        "{out}"
    );
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// An octopus merge brings in a branch from each parent after the first,
// and the check reads the one that adds the entry.
// sokf-tests: CLI-4_AC-105
#[test]
fn an_octopus_merge_keeps_the_lapse_of_its_branch() {
    let repo = Repo::new("merge-octopus", true);
    review(&repo);
    let commit = change(&repo);
    git(&repo.0, &["checkout", "-q", "main"]);
    git(&repo.0, &["checkout", "-q", "-b", "other"]);
    repo.write("notes.txt", "Other work.\n");
    repo.commit("2026-01-06T12:00:00Z", "other");
    git(&repo.0, &["checkout", "-q", "main"]);
    dated(
        &repo.0,
        "2026-01-07T00:00:00Z",
        &["merge", "-q", "--no-ff", "--no-edit", "other", "review"],
    );
    let parents = git(&repo.0, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(parents.split_whitespace().count(), 4, "{parents}");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// A merge of `main` into the branch after the review brings a change that
// lapses the entry on the branch, at the date of that merge, and the merge
// of the branch into `main` keeps the lapse.
// sokf-tests: CLI-4_AC-94
// sokf-tests: CLI-4_AC-105
#[test]
fn a_merge_keeps_the_lapse_that_its_branch_merged_in() {
    let repo = Repo::new("merge-sync", true);
    repo.write(NOTE_PATH, &note("First.\n\nSecond.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "review"]);
    repo.write(
        NOTE_PATH,
        &note("First.\n\nSecond.", &[("human:ann", "2026-01-05")]),
    );
    repo.commit("2026-01-05T12:00:00Z", "verify");
    git(&repo.0, &["checkout", "-q", "main"]);
    repo.write(NOTE_PATH, &note("First.\n\nChanged.", &[]));
    repo.commit("2026-01-02T00:00:00Z", "change on main");
    git(&repo.0, &["checkout", "-q", "review"]);
    let sync = merge(&repo, "2026-01-06T00:00:00Z", "main");
    let out = repo.warnings(false);
    assert!(
        lapsed(&out)
            .first()
            .is_some_and(|l| l.contains(&sync[..12])),
        "{out}"
    );
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(&sync[..12]), "{out}");
}

// A branch that drops the entry after the review doesn't hold it, so the
// merge that adds the entry again added it itself, and no change of the
// branch lapses it.
// sokf-tests: CLI-4_AC-105
#[test]
fn an_entry_that_the_branch_dropped_keeps_the_merge_as_its_commit() {
    let repo = Repo::new("merge-dropped", true);
    review(&repo);
    repo.write(NOTE_PATH, &note("Second.", &[]));
    repo.commit("2026-01-06T00:00:00Z", "change and drop the entry");
    git(&repo.0, &["checkout", "-q", "main"]);
    git(
        &repo.0,
        &["merge", "-q", "--no-ff", "--no-commit", "review"],
    );
    repo.write(NOTE_PATH, &note("Second.", &[("human:ann", "2026-01-05")]));
    repo.commit("2026-01-07T00:00:00Z", "merge and verify");
    none(&repo.warnings(false));
}

// Two entries that one merge brings in each lapse through the change of
// the branch, which the check reads once for the document.
// sokf-tests: CLI-4_AC-105
#[test]
fn each_entry_that_a_merge_brings_in_keeps_its_lapse() {
    let repo = Repo::new("merge-two", true);
    let entries = [("human:ann", "2026-01-05"), ("human:bob", "2026-01-05")];
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "review"]);
    repo.write(NOTE_PATH, &note("First.", &entries));
    repo.commit("2026-01-05T12:00:00Z", "verify");
    repo.write(NOTE_PATH, &note("Second.", &entries));
    let commit = repo.commit("2026-01-06T00:00:00Z", "change");
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 2, "{out}");
    assert!(found.iter().all(|l| l.contains(&commit[..12])), "{out}");
}

// A move of the document on the branch, after the change that lapses the
// entry, keeps the lapse after the merge, at the new path.
// sokf-tests: CLI-4_AC-105
#[test]
fn a_merge_keeps_the_lapse_of_its_branch_after_a_move_on_the_branch() {
    let repo = Repo::new("merge-branch-move", true);
    review(&repo);
    let commit = change(&repo);
    std::fs::create_dir_all(repo.0.join("knowledge/moved")).unwrap();
    git(&repo.0, &["mv", NOTE_PATH, "knowledge/moved/note-a.md"]);
    repo.commit("2026-01-06T12:00:00Z", "move");
    git(&repo.0, &["checkout", "-q", "main"]);
    merge(&repo, "2026-01-07T00:00:00Z", "review");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("/knowledge/moved/note-a.md:7:"),
        "{out}"
    );
    assert!(found[0].contains(&commit[..12]), "{out}");
}
