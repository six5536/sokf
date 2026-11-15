//! End-to-end tests of the check of a lapsed `verified` entry (CLI-4_AC-92
//! to CLI-4_AC-102, CACHE-1_AC-97): each test builds a repository, commits
//! to it with fixed dates, and runs `sokf validate` on it.

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use crate::common::{git, output, sokf};

/// `NOTE` is the schema of a note: a title and any prose.
const NOTE: &str = "---\ntype: Schema\nid: schema-note\ntitle: Note Schema\ndescription: A note.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: { from: title }\n    level: 1\n    guidance: The title.\n    blocks:\n      - kind: prose\n        repeat: true\n        example: The kitchen, the week the flowers came.\n````\n";

/// `DATASET` is the schema of a CSV file and its sidecar.
const DATASET: &str = "---\ntype: Schema\nid: schema-dataset\ntitle: Dataset Schema\ndescription: A CSV file and its description.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Dataset\n  resource:\n    example: /knowledge/rates.csv\nasset-files: [\"/knowledge/**/*.csv\"]\npreamble:\n  guidance: What the data is.\n  blocks:\n    - kind: prose\n      max: 1\n      example: Exchange rates by day.\n````\n";

/// A `Repo` is a repository in a temporary directory, which the test
/// commits to. Dropping it removes the directory.
struct Repo(PathBuf);

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl Repo {
    /// `new` returns a store with the note and dataset schemas, and a git
    /// repository whose first commit, on 2020-01-01, holds them, when `git`
    /// is set.
    fn new(name: &str, with_git: bool) -> Repo {
        let dir = std::env::temp_dir().join(format!(
            "sokf-verified-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let repo = Repo(dunce::canonicalize(&dir).unwrap());
        repo.write(
            "knowledge/manifest.sokf.yaml",
            "sokf: \"0.6\"\nname: notes\nscope: test\n",
        );
        repo.write("knowledge/schemas/note.md", NOTE);
        repo.write("knowledge/schemas/dataset.md", DATASET);
        if with_git {
            git(&repo.0, &["init", "-q"]);
        }
        repo.fix();
        if with_git {
            repo.commit("2020-01-01T00:00:00Z", "store");
        }
        repo
    }

    /// `write` writes `text` to `rel`, a path from the repository root.
    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// `fix` runs `sokf validate --fix`, which writes the indexes, and then
    /// runs it a second time, which fixes nothing.
    fn fix(&self) {
        for run in 0..2 {
            let (stdout, stderr, code) = output(sokf().arg("--repo-root").arg(&self.0).args([
                "--no-cache",
                "validate",
                "--fix",
            ]));
            assert_eq!(code, 0, "validate --fix failed: {stdout}{stderr}");
            if run == 1 {
                for op in ["moved ", "created ", "rewrote "] {
                    assert!(!stdout.contains(op), "the second fix fixed: {stdout}");
                }
            }
        }
    }

    /// `commit` writes the indexes, commits every change, with `date` as the
    /// author and the committer date, and returns the commit.
    fn commit(&self, date: &str, message: &str) -> String {
        self.fix();
        git(&self.0, &["add", "-A"]);
        dated(
            &self.0,
            date,
            &["commit", "-q", "--allow-empty", "-m", message],
        );
        git(&self.0, &["rev-parse", "HEAD"]).trim().to_string()
    }

    /// `warnings` runs `sokf validate --warnings` and returns its stdout,
    /// with the cache off unless `cache` is set.
    fn warnings(&self, cache: bool) -> String {
        let mut cmd = sokf();
        cmd.arg("--repo-root").arg(&self.0);
        if !cache {
            cmd.arg("--no-cache");
        }
        let (stdout, stderr, code) = output(cmd.args(["validate", "--warnings"]));
        assert!(code == 0, "validate failed: {stdout}{stderr}");
        stdout
    }
}

/// `dated` runs git in `dir`, as `git` does, with `date` as the author and
/// the committer date.
fn dated(dir: &Path, date: &str, args: &[&str]) {
    let mut cmd = std::process::Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1");
    let out = cmd
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `note` returns a note with `body` and the `verified` entries `entries`,
/// each a `(by, at)` pair, one to a line.
fn note(body: &str, entries: &[(&str, &str)]) -> String {
    let mut text = "---\ntype: Note\nid: note-a\ntitle: A note\ndescription: A note.\n".to_string();
    if !entries.is_empty() {
        text.push_str("verified:\n");
        for (by, at) in entries {
            let _ = writeln!(text, "  - {{ by: \"{by}\", at: \"{at}\" }}");
        }
    }
    let _ = write!(text, "---\n\n# A note\n\n{body}\n");
    text
}

/// `NOTE_PATH` is where the tests write the note.
const NOTE_PATH: &str = "knowledge/note-a.md";

/// `none` asserts that no warning of `out` says that an entry lapsed.
fn none(out: &str) {
    let found = lapsed(out);
    assert!(found.is_empty(), "{found:?}");
}

/// `lapsed` returns the warnings of `out` that say an entry lapsed.
fn lapsed(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| l.contains("(CLI-4_AC-92)"))
        .collect()
}

// A commit that changes the content after an entry's `at` lapses the entry,
// and the warning lies at the line of the entry and names the commit.
// sokf-tests: CLI-4_AC-92
// sokf-tests: CLI-4_AC-94
#[test]
fn a_later_commit_of_the_content_lapses_an_entry() {
    let repo = Repo::new("later", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-02T12:00:00Z", "verify");
    none(&repo.warnings(false));
    repo.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    let commit = repo.commit("2026-02-01T00:00:00Z", "change");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("/knowledge/note-a.md:7: warning: "),
        "{out}"
    );
    assert!(found[0].contains("`human:ann`"), "{out}");
    assert!(found[0].contains("has lapsed"), "{out}");
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// A commit dated before an entry's `at` lapses nothing.
// sokf-tests: CLI-4_AC-94
#[test]
fn a_commit_before_the_at_lapses_nothing() {
    let repo = Repo::new("before", true);
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-03-01T00:00:00Z")]),
    );
    repo.commit("2026-03-02T00:00:00Z", "verify");
    none(&repo.warnings(false));
}

// A second reviewer's entry changes only `verified`, so the first entry
// stays current.
// sokf-tests: CLI-4_AC-96
#[test]
fn a_change_of_only_verified_lapses_nothing() {
    let repo = Repo::new("only-verified", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-02T12:00:00Z", "verify");
    repo.write(
        NOTE_PATH,
        &note(
            "First.",
            &[
                ("human:ann", "2026-01-02T00:00:00Z"),
                ("process:links", "2026-02-01T00:00:00Z"),
            ],
        ),
    );
    repo.commit("2026-02-01T12:00:00Z", "verify again");
    none(&repo.warnings(false));
}

// A reviewer who corrects the content and adds an entry in one commit
// verified what the commit holds, though its date follows the `at`.
// sokf-tests: CLI-4_AC-97
#[test]
fn a_change_of_the_commit_that_adds_the_entry_lapses_nothing() {
    let repo = Repo::new("same-commit", true);
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    repo.write(
        NOTE_PATH,
        &note("Corrected.", &[("human:ann", "2026-02-01T00:00:00Z")]),
    );
    repo.commit("2026-02-01T00:30:00Z", "correct and verify");
    none(&repo.warnings(false));
}

// A change of the working tree is a change at the time of the run.
// sokf-tests: CLI-4_AC-95
#[test]
fn a_change_that_isnt_committed_lapses_an_entry() {
    let repo = Repo::new("uncommitted", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-02T12:00:00Z", "verify");
    repo.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("committed"), "{out}");
}

// An entry that `HEAD` lacks comes with the working tree, so no change of
// the working tree lapses it.
// sokf-tests: CLI-4_AC-98
#[test]
fn an_entry_that_isnt_committed_survives_the_working_tree() {
    let repo = Repo::new("new-entry", true);
    repo.write(NOTE_PATH, &note("First.", &[]));
    repo.commit("2020-06-01T00:00:00Z", "write");
    repo.write(
        NOTE_PATH,
        &note("Corrected.", &[("human:ann", "2021-01-01")]),
    );
    none(&repo.warnings(false));
}

// An `at` that gives only a date covers the whole of that day in UTC.
// sokf-tests: CLI-4_AC-99
#[test]
fn a_date_alone_covers_its_day() {
    let repo = Repo::new("date", true);
    repo.write(NOTE_PATH, &note("First.", &[("human:ann", "2026-02-01")]));
    repo.commit("2026-01-30T00:00:00Z", "verify");
    repo.write(NOTE_PATH, &note("Second.", &[("human:ann", "2026-02-01")]));
    repo.commit("2026-02-01T23:59:59Z", "same day");
    none(&repo.warnings(false));
    repo.write(NOTE_PATH, &note("Third.", &[("human:ann", "2026-02-01")]));
    repo.commit("2026-02-02T00:00:00Z", "next day");
    assert_eq!(lapsed(&repo.warnings(false)).len(), 1);
}

// An `at` with a time and no offset is a time in UTC.
// sokf-tests: CLI-4_AC-100
#[test]
fn a_time_without_an_offset_is_utc() {
    let repo = Repo::new("utc", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-02-01T10:00")]),
    );
    repo.commit("2026-01-30T00:00:00Z", "verify");
    repo.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-02-01T10:00")]),
    );
    // 10:30 at +02:00 is 08:30 in UTC, before the `at`.
    repo.commit("2026-02-01T10:30:00+02:00", "before");
    none(&repo.warnings(false));
    repo.write(
        NOTE_PATH,
        &note("Third.", &[("human:ann", "2026-02-01T10:00")]),
    );
    repo.commit("2026-02-01T10:00:01Z", "after");
    assert_eq!(lapsed(&repo.warnings(false)).len(), 1);
}

// The history follows a moved document, so a change before the move still
// lapses the entry.
// sokf-tests: CLI-4_AC-93
#[test]
fn the_history_follows_a_move() {
    let repo = Repo::new("move", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-05T00:00:00Z")]),
    );
    repo.commit("2026-01-01T00:00:00Z", "verify");
    repo.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-01-05T00:00:00Z")]),
    );
    repo.commit("2026-01-10T00:00:00Z", "change");
    fs::create_dir_all(repo.0.join("knowledge/moved")).unwrap();
    git(&repo.0, &["mv", NOTE_PATH, "knowledge/moved/note-a.md"]);
    repo.fix();
    repo.commit("2026-01-20T00:00:00Z", "move");
    let out = repo.warnings(false);
    assert_eq!(lapsed(&out).len(), 1, "{out}");
    assert!(
        lapsed(&out)[0].starts_with("/knowledge/moved/note-a.md:7:"),
        "{out}"
    );
}

// A change that a merge brings counts at the date of the merge, on the
// first-parent history, whatever the date of the commit on the other
// branch.
// sokf-tests: CLI-4_AC-93
// sokf-tests: CLI-4_AC-94
#[test]
fn a_merge_dates_the_change_that_it_brings() {
    let repo = Repo::new("merge", true);
    repo.write(NOTE_PATH, &note("First.\n\nSecond.", &[]));
    repo.commit("2026-01-01T00:00:00Z", "write");
    git(&repo.0, &["checkout", "-q", "-b", "side"]);
    repo.write(NOTE_PATH, &note("First.\n\nChanged.", &[]));
    repo.commit("2026-01-02T00:00:00Z", "side");
    git(&repo.0, &["checkout", "-q", "main"]);
    repo.write(
        NOTE_PATH,
        &note(
            "First.\n\nSecond.",
            &[("human:ann", "2026-01-05T00:00:00Z")],
        ),
    );
    repo.commit("2026-01-05T12:00:00Z", "verify");
    dated(
        &repo.0,
        "2026-01-10T00:00:00Z",
        &["merge", "-q", "--no-edit", "side"],
    );
    let merge = git(&repo.0, &["rev-parse", "HEAD"]).trim().to_string();
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(&merge[..12]), "{out}");
}

// A sidecar's entry lapses when its asset changes after the `at`, in a
// commit or in the working tree.
// sokf-tests: CLI-4_AC-92
// sokf-tests: CLI-4_AC-95
#[test]
fn a_change_to_the_asset_lapses_the_sidecars_entry() {
    let repo = Repo::new("asset", true);
    repo.write("knowledge/rates.csv", "quarter,rate\nq3,0.25\n");
    repo.write(
        "knowledge/rates.csv.md",
        "---\ntype: Dataset\nid: rates\ntitle: Rates\ndescription: Rates.\nresource: /knowledge/rates.csv\nverified:\n  - { by: \"human:ann\", at: \"2026-01-02T00:00:00Z\" }\n---\n\nQuarterly rates.\n",
    );
    repo.fix();
    repo.commit("2026-01-02T12:00:00Z", "verify");
    none(&repo.warnings(true));
    repo.write("knowledge/rates.csv", "quarter,rate\nq3,0.26\n");
    // The cache checks the sidecar again when its asset changes.
    assert_eq!(lapsed(&repo.warnings(true)).len(), 1);
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("/knowledge/rates.csv.md:8:"), "{out}");
    assert!(found[0].contains("/knowledge/rates.csv"), "{out}");
    assert!(found[0].contains("committed"), "{out}");
    let commit = repo.commit("2026-02-01T00:00:00Z", "change the rates");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(&commit[..12]), "{out}");
}

// Outside a git repository, and in a repository without a commit, the
// check reads no history and reports nothing.
// sokf-tests: CLI-4_AC-101
#[test]
fn no_history_skips_the_check() {
    for with_git in [false, true] {
        let repo = Repo::new("no-history", false);
        if with_git {
            git(&repo.0, &["init", "-q"]);
        }
        repo.write(NOTE_PATH, &note("First.", &[("human:ann", "2000-01-01")]));
        repo.fix();
        none(&repo.warnings(false));
    }
}

// A partial clone reads no history, so that the check fetches nothing.
// sokf-tests: CLI-4_AC-102
#[test]
fn a_partial_clone_skips_the_check() {
    let repo = Repo::new("partial", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-02T12:00:00Z", "verify");
    repo.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-02-01T00:00:00Z", "change");
    assert_eq!(lapsed(&repo.warnings(false)).len(), 1);
    git(&repo.0, &["config", "core.repositoryformatversion", "1"]);
    git(
        &repo.0,
        &["config", "remote.origin.url", "https://example.com/x.git"],
    );
    git(&repo.0, &["config", "remote.origin.promisor", "true"]);
    git(&repo.0, &["config", "extensions.partialclone", "origin"]);
    none(&repo.warnings(false));
}

// A checkout of another history that leaves the file as it is still
// checks the entry again.
// sokf-tests: CACHE-1_AC-97
#[test]
fn a_new_head_checks_the_entry_again() {
    let repo = Repo::new("head", true);
    let text = note("Second.", &[("human:ann", "2026-01-05T00:00:00Z")]);
    // `main` holds the note as one commit before the `at`.
    repo.write(NOTE_PATH, &text);
    repo.commit("2026-01-01T00:00:00Z", "write");
    // `later` changes the content after the `at`, to the same text.
    git(&repo.0, &["checkout", "-q", "-b", "later", "HEAD~1"]);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-05T00:00:00Z")]),
    );
    repo.commit("2026-01-01T00:00:00Z", "write");
    repo.write(NOTE_PATH, &text);
    repo.commit("2026-01-10T00:00:00Z", "change");
    git(&repo.0, &["checkout", "-q", "main"]);
    none(&repo.warnings(true));
    git(&repo.0, &["checkout", "-q", "later"]);
    assert_eq!(lapsed(&repo.warnings(true)).len(), 1);
}

// A document that the working tree moved, by `git mv` or by a move of
// `validate --fix`, keeps the history of the file that it moved from.
// sokf-tests: CLI-4_AC-93
#[test]
fn the_history_follows_a_move_that_isnt_committed() {
    for staged in [true, false] {
        let repo = Repo::new("moved", true);
        repo.write(
            NOTE_PATH,
            &note("First.", &[("human:ann", "2026-01-05T00:00:00Z")]),
        );
        repo.commit("2026-01-01T00:00:00Z", "verify");
        repo.write(
            NOTE_PATH,
            &note("Second.", &[("human:ann", "2026-01-05T00:00:00Z")]),
        );
        repo.commit("2026-01-10T00:00:00Z", "change");
        fs::create_dir_all(repo.0.join("knowledge/moved")).unwrap();
        if staged {
            git(&repo.0, &["mv", NOTE_PATH, "knowledge/moved/note-a.md"]);
        } else {
            fs::rename(
                repo.0.join(NOTE_PATH),
                repo.0.join("knowledge/moved/note-a.md"),
            )
            .unwrap();
        }
        repo.fix();
        let out = repo.warnings(false);
        let found = lapsed(&out);
        assert_eq!(found.len(), 1, "staged {staged}: {out}");
        assert!(
            found[0].starts_with("/knowledge/moved/note-a.md:7:"),
            "{out}"
        );
    }
}

// A file that git's own conversion, such as `ident`, makes differ from its
// blob, and that `git status` lists as unchanged, has no change of the
// working tree.
// sokf-tests: CLI-4_AC-95
#[test]
fn a_conversion_of_git_is_no_change() {
    let repo = Repo::new("ident", true);
    repo.write(".gitattributes", "*.md ident\n");
    repo.write(
        NOTE_PATH,
        &note("First, $Id$.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-01T00:00:00Z", "verify");
    // A checkout writes the file with the id that `ident` expands.
    fs::remove_file(repo.0.join(NOTE_PATH)).unwrap();
    git(&repo.0, &["checkout", "--", NOTE_PATH]);
    let text = fs::read_to_string(repo.0.join(NOTE_PATH)).unwrap();
    assert!(text.contains("$Id: "), "{text}");
    none(&repo.warnings(false));
}

// A shallow clone reads no history, because its oldest commit shows every
// file as added.
// sokf-tests: CLI-4_AC-102
#[test]
fn a_shallow_clone_skips_the_check() {
    let repo = Repo::new("shallow-source", true);
    repo.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-02T12:00:00Z", "verify");
    repo.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-02-01T00:00:00Z", "change");
    assert_eq!(lapsed(&repo.warnings(false)).len(), 1);
    let clone = Repo(repo.0.with_file_name(format!(
        "{}-clone",
        repo.0.file_name().unwrap().to_string_lossy()
    )));
    let url = format!("file://{}", repo.0.display());
    git(
        repo.0.parent().unwrap(),
        &[
            "clone",
            "-q",
            "--depth",
            "1",
            &url,
            clone.0.to_str().unwrap(),
        ],
    );
    none(&clone.warnings(false));
    // A change of the working tree would lapse the entry of a history that
    // the clone's oldest commit seemed to add.
    clone.write(
        NOTE_PATH,
        &note("Third.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    none(&clone.warnings(false));
}

// The entries of a schema document lapse as those of any document do.
// sokf-tests: CLI-4_AC-92
#[test]
fn a_schema_documents_entry_lapses() {
    let repo = Repo::new("schema", true);
    let verified = NOTE.replacen(
        "description: A note.\n",
        "description: A note.\nverified:\n  - { by: \"human:ann\", at: \"2026-01-02T00:00:00Z\" }\n",
        1,
    );
    repo.write("knowledge/schemas/note.md", &verified);
    repo.commit("2026-01-02T12:00:00Z", "verify");
    none(&repo.warnings(false));
    repo.write(
        "knowledge/schemas/note.md",
        &verified.replace("The title.", "The title of the note."),
    );
    repo.commit("2026-02-01T00:00:00Z", "change");
    let out = repo.warnings(false);
    let found = lapsed(&out);
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("/knowledge/schemas/note.md:7:"),
        "{out}"
    );
}

// A move of a sidecar and its asset, whose `resource` `--fix` rewrites,
// changes no content.
// sokf-tests: CLI-4_AC-96
#[test]
fn a_move_of_a_sidecar_and_its_asset_lapses_nothing() {
    for staged in [true, false] {
        let repo = Repo::new("pair", true);
        repo.write("knowledge/rates.csv", "quarter,rate\nq3,0.25\n");
        repo.write(
            "knowledge/rates.csv.md",
            "---\ntype: Dataset\nid: rates\ntitle: Rates\ndescription: Rates.\nresource: /knowledge/rates.csv\nverified:\n  - { by: \"human:ann\", at: \"2026-01-02T00:00:00Z\" }\n---\n\nQuarterly rates.\n",
        );
        repo.commit("2026-01-02T12:00:00Z", "verify");
        fs::create_dir_all(repo.0.join("knowledge/sub")).unwrap();
        for name in ["rates.csv", "rates.csv.md"] {
            let (from, to) = (format!("knowledge/{name}"), format!("knowledge/sub/{name}"));
            if staged {
                git(&repo.0, &["mv", &from, &to]);
            } else {
                fs::rename(repo.0.join(&from), repo.0.join(&to)).unwrap();
            }
        }
        repo.fix();
        let text = fs::read_to_string(repo.0.join("knowledge/sub/rates.csv.md")).unwrap();
        assert!(
            text.contains("resource: /knowledge/sub/rates.csv"),
            "{text}"
        );
        none(&repo.warnings(false));
        repo.commit("2026-02-01T00:00:00Z", "move");
        none(&repo.warnings(false));
    }
}

// A change of an asset's mode alone changes no byte, in the working tree
// as in a commit.
// sokf-tests: CLI-4_AC-92
#[cfg(unix)]
#[test]
fn a_change_of_an_assets_mode_lapses_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let repo = Repo::new("mode", true);
    git(&repo.0, &["config", "core.fileMode", "true"]);
    repo.write("knowledge/rates.csv", "quarter,rate\nq3,0.25\n");
    repo.write(
        "knowledge/rates.csv.md",
        "---\ntype: Dataset\nid: rates\ntitle: Rates\ndescription: Rates.\nresource: /knowledge/rates.csv\nverified:\n  - { by: \"human:ann\", at: \"2026-01-02T00:00:00Z\" }\n---\n\nQuarterly rates.\n",
    );
    repo.commit("2026-01-02T12:00:00Z", "verify");
    let asset = repo.0.join("knowledge/rates.csv");
    fs::set_permissions(&asset, fs::Permissions::from_mode(0o755)).unwrap();
    none(&repo.warnings(false));
}

// A root below the top of a git worktree reads no history, because git
// names each path from the top.
// sokf-tests: CLI-4_AC-103
#[test]
fn a_root_below_the_top_skips_the_check() {
    let repo = Repo::new("below", false);
    let top = repo.0.with_file_name(format!(
        "{}-top",
        repo.0.file_name().unwrap().to_string_lossy()
    ));
    let top = Repo(top);
    fs::create_dir_all(&top.0).unwrap();
    git(&top.0, &["init", "-q"]);
    let inner = top.0.join("proj");
    fs::rename(&repo.0, &inner).unwrap();
    let inner = Repo(inner);
    inner.write(
        NOTE_PATH,
        &note("First.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    inner.fix();
    git(&top.0, &["add", "-A"]);
    dated(
        &top.0,
        "2026-01-02T12:00:00Z",
        &["commit", "-q", "-m", "verify"],
    );
    inner.write(
        NOTE_PATH,
        &note("Second.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    git(&top.0, &["add", "-A"]);
    dated(
        &top.0,
        "2026-02-01T00:00:00Z",
        &["commit", "-q", "-m", "change"],
    );
    none(&inner.warnings(false));
}

// A renewed entry beside an entry that `ident` keeps current changes only
// `verified`, though the text on disk holds the expanded id.
// sokf-tests: CLI-4_AC-96
#[test]
fn a_renewal_beside_a_conversion_of_git_lapses_nothing() {
    let repo = Repo::new("ident-renew", true);
    repo.write(".gitattributes", "*.md ident\n");
    repo.write(
        NOTE_PATH,
        &note("First, $Id$.", &[("human:ann", "2026-01-02T00:00:00Z")]),
    );
    repo.commit("2026-01-01T00:00:00Z", "verify");
    fs::remove_file(repo.0.join(NOTE_PATH)).unwrap();
    git(&repo.0, &["checkout", "--", NOTE_PATH]);
    let text = fs::read_to_string(repo.0.join(NOTE_PATH)).unwrap();
    let renewed = text.replace(
        "  - { by: \"human:ann\", at: \"2026-01-02T00:00:00Z\" }\n",
        "  - { by: \"human:ann\", at: \"2026-01-02T00:00:00Z\" }\n  - { by: \"process:links\", at: \"2026-03-01T00:00:00Z\" }\n",
    );
    assert_ne!(renewed, text);
    fs::write(repo.0.join(NOTE_PATH), renewed).unwrap();
    none(&repo.warnings(false));
}
