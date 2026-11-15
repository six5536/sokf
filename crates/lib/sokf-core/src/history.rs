//! This module reads the changes to a file from the history of `HEAD` in
//! git and from the working tree, for the check of a lapsed `verified`
//! entry (the concern "Lapsed entries" of design-operations).

use std::{
    cell::OnceCell,
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    RepoPath, RepoRoot,
    git::{self, Git, MIN_GIT},
    model::core_fields,
    source::{self, Source},
    yaml,
};

/// `When` says where a change lies: in a commit, by its id, or in the
/// working tree and the index, against `HEAD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum When {
    /// A commit of the first-parent history of `HEAD`.
    Commit(String),
    /// The working tree or the index, against `HEAD`.
    WorkingTree,
}

/// A `Change` is one change to a file: where it lies, when it happened, in
/// seconds since the epoch, and the text of the file before and after it.
/// A text is `None` where the file is absent, where its blob is missing or
/// isn't UTF-8, or for a file whose texts the history doesn't read.
#[derive(Debug, Clone)]
pub(crate) struct Change {
    /// Where the change lies.
    pub(crate) when: When,
    /// When the change happened.
    pub(crate) time: i64,
    /// The text before the change.
    pub(crate) before: Option<String>,
    /// The text after the change.
    pub(crate) after: Option<String>,
}

/// A `Failed` says that a `git` that the history needed failed, so that
/// the history of the file is unknown in this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Failed;

/// A `Changes` is the changes to a document and to its asset, each newest
/// first.
#[derive(Debug, Clone, Default)]
pub(crate) struct Changes {
    /// The changes to the document.
    pub(crate) document: Vec<Change>,
    /// The changes to the asset.
    pub(crate) asset: Vec<Change>,
}

/// A `History` reads the history of the repository at the root, once in
/// each run. It reads nothing outside a git repository, without a commit,
/// below the top of a worktree, with a git older than `MIN_GIT`, or in a
/// partial or a shallow clone.
#[derive(Debug)]
pub(crate) struct History {
    /// The runner.
    git: Git,
    /// The commit that `HEAD` names, when it names one.
    head: Option<String>,
    /// The time of the run, in seconds since the epoch.
    now: i64,
    /// Whether the history is readable, which `readable` works out once.
    readable: OnceCell<bool>,
    /// The renames that the index holds against `HEAD`, from each new path
    /// to its old one, which `moved_from` reads once.
    renames: OnceCell<Option<BTreeMap<String, String>>>,
    /// The files that the working tree deleted, which `moved_from` reads
    /// once.
    deleted: OnceCell<Option<Vec<RepoPath>>>,
}

impl History {
    /// `open` opens the history of the repository at `root`. It runs one
    /// `git`, which reads the commit that `HEAD` names, and takes the time
    /// of the run.
    pub(crate) fn open(root: &RepoRoot) -> History {
        let git = Git::new(root.as_path());
        let head = git.head();
        History {
            git,
            head,
            now: now(),
            readable: OnceCell::new(),
            renames: OnceCell::new(),
            deleted: OnceCell::new(),
        }
    }

    /// `head` returns the commit that `HEAD` names, or an empty text where
    /// it names none, as the query of the history holds it.
    pub(crate) fn head(&self) -> &str {
        self.head.as_deref().unwrap_or_default()
    }

    /// `readable` returns whether the history can be read: `HEAD` names a
    /// commit, the root is the top of the worktree, so that a path from the
    /// root means the same in git, git is `MIN_GIT` or later, and the clone
    /// is neither partial, where reading old objects would fetch them, nor
    /// shallow, where the oldest commit shows every file as added.
    // sokf-implements: CLI-4_AC-101
    // sokf-implements: CLI-4_AC-102
    // sokf-implements: CLI-4_AC-103
    pub(crate) fn readable(&self) -> bool {
        *self.readable.get_or_init(|| {
            self.head.is_some()
                && git::version().is_some_and(|v| v >= MIN_GIT)
                && self.git.is_top()
                && self.git.partial().is_none()
                && !self.git.is_shallow()
        })
    }

    /// `changes` returns the changes to the document at `path`, whose text
    /// in the working tree is `text` and whose id is `id`, and to its asset
    /// at `asset`, newest first, or `Failed` where a `git` failed. The
    /// history must be readable.
    ///
    /// The changes of a file are its change of the working tree, where it
    /// has one, and then each change of a commit. A document that `HEAD`
    /// lacks, or that `HEAD` deleted, takes the history of the file that
    /// the working tree moved it from, and its asset the history of the
    /// file of the asset's name beside that file. The texts of the
    /// document have LF line endings, and sokf reads no byte of the asset.
    // sokf-implements: CLI-4_AC-95
    pub(crate) fn changes(
        &self,
        path: &RepoPath,
        text: &str,
        id: Option<&str>,
        asset: Option<&RepoPath>,
    ) -> Result<Changes, Failed> {
        let mut entries = log(&self.git, path)?;
        let mut from = None;
        // A file that `HEAD` deleted at the path is another file, whose
        // history isn't the document's.
        if entries.first().is_none_or(|e| e.after.is_none()) {
            from = self.moved_from(path, id)?;
            entries = match &from {
                Some(from) => log(&self.git, from)?,
                None => Vec::new(),
            };
        }
        let document = self.document(&entries, from.as_ref().unwrap_or(path), path, text)?;
        let asset = match asset {
            Some(asset) => {
                let before = from
                    .as_ref()
                    .and_then(RepoPath::parent)
                    .and_then(|dir| RepoPath::parse(&format!("{dir}/{}", asset.file_name())));
                self.asset(asset, before.as_ref())?
            }
            None => Vec::new(),
        };
        Ok(Changes { document, asset })
    }

    /// `document` returns the changes to a document from the entries of
    /// the log of `head_path`, its path in `HEAD`: its change of the working
    /// tree, where `git status` lists the file at `path`, after git's own
    /// conversions, such as a filter or `ident`, or where `HEAD` lacks it,
    /// and then each change of a commit. The text of `HEAD` that the change
    /// of the working tree starts from has the same conversions as the text
    /// on disk.
    fn document(
        &self,
        entries: &[Entry],
        head_path: &RepoPath,
        path: &RepoPath,
        text: &str,
    ) -> Result<Vec<Change>, Failed> {
        let texts = self.git.blobs(
            entries
                .iter()
                .flat_map(|e| [e.before.as_deref(), e.after.as_deref()])
                .flatten(),
        );
        let text_of = |oid: &Option<String>| {
            oid.as_ref()
                .and_then(|o| texts.get(o).cloned().flatten())
                .map(|t| t.replace("\r\n", "\n"))
        };
        // The newest entry gives the blob of `HEAD`, a rename among them.
        let head = entries.first().and_then(|e| e.after.as_ref());
        let mut changes = Vec::new();
        if head.is_none() || listed(&self.git, path)? {
            let before = head.and_then(|_| {
                self.git
                    .run(&[
                        "cat-file",
                        "--filters",
                        &format!("HEAD:{}", head_path.relative()),
                    ])
                    .and_then(|out| String::from_utf8(out).ok())
                    .map(|t| t.replace("\r\n", "\n"))
                    .or_else(|| entries.first().and_then(|e| text_of(&e.after)))
            });
            changes.push(Change {
                when: When::WorkingTree,
                time: self.now,
                before,
                after: Some(text.to_string()),
            });
        }
        changes.extend(commits(entries, text_of));
        Ok(changes)
    }

    /// `asset` returns the changes to the asset at `path`, or, where `HEAD`
    /// lacks it, to the file at `before` that it moved from, without their
    /// texts: its change of the working tree, where git's blob of the file
    /// on disk differs from the blob of `HEAD`, and then each change of a
    /// commit. An asset that git never tracked has no change, because git
    /// holds no history of it.
    fn asset(&self, path: &RepoPath, before: Option<&RepoPath>) -> Result<Vec<Change>, Failed> {
        let mut entries = log(&self.git, path)?;
        if entries.first().is_none_or(|e| e.after.is_none()) {
            entries = match before {
                Some(before) => log(&self.git, before)?,
                None => Vec::new(),
            };
        }
        let Some(head) = entries.first().and_then(|e| e.after.clone()) else {
            return Ok(commits(&entries, |_| None).collect());
        };
        let mut changes = Vec::new();
        let changed = hash(&self.git, path)? != head;
        if changed {
            changes.push(Change {
                when: When::WorkingTree,
                time: self.now,
                before: None,
                after: None,
            });
        }
        changes.extend(commits(&entries, |_| None));
        Ok(changes)
    }

    /// `moved_from` returns the file that the document at `path`, which
    /// `HEAD` lacks, moved from: the source of a rename that the index holds
    /// against `HEAD`, or else the one file of its name that the working
    /// tree deleted and whose text in `HEAD` has the document's `id`, as a
    /// move of `validate --fix` leaves it.
    fn moved_from(&self, path: &RepoPath, id: Option<&str>) -> Result<Option<RepoPath>, Failed> {
        let renames = self.renames.get_or_init(|| {
            let out = self.git.run(&[
                "diff-index",
                "--cached",
                "-M",
                "--raw",
                "-z",
                "--no-color",
                "HEAD",
            ])?;
            Some(
                git::raw(&out)
                    .into_iter()
                    .filter(|r| r.status.starts_with('R'))
                    .filter_map(|r| match r.paths.as_slice() {
                        [from, to] => Some((to.clone(), from.clone())),
                        _ => None,
                    })
                    .collect(),
            )
        });
        let renames = renames.as_ref().ok_or(Failed)?;
        if let Some(from) = renames.get(path.relative()) {
            return Ok(RepoPath::parse(&format!("/{from}")));
        }
        let Some(id) = id else {
            return Ok(None);
        };
        let deleted = self.deleted.get_or_init(|| {
            let out = self.git.run(&["ls-files", "--deleted", "-z"])?;
            Some(
                out.split(|b| *b == 0)
                    .filter_map(|p| RepoPath::parse(&format!("/{}", String::from_utf8_lossy(p))))
                    .collect(),
            )
        });
        let named: Vec<(RepoPath, String)> = deleted
            .as_ref()
            .ok_or(Failed)?
            .iter()
            .filter(|p| p.file_name() == path.file_name())
            .cloned()
            .map(|p| {
                let object = format!("HEAD:{}", p.relative());
                (p, object)
            })
            .collect();
        let texts = self.git.blobs(named.iter().map(|(_, o)| o.as_str()));
        let mut same = named.into_iter().filter(|(p, object)| {
            texts
                .get(object)
                .cloned()
                .flatten()
                .is_some_and(|t| id_of(p, &t).as_deref() == Some(id))
        });
        Ok(match (same.next(), same.next()) {
            (Some((from, _)), None) => Some(from),
            _ => None,
        })
    }
}

/// `commits` returns the changes of the commits of `entries`, with the
/// texts that `text_of` gives each blob. A rename whose blob stays the same
/// changes nothing.
fn commits<'a>(
    entries: &'a [Entry],
    text_of: impl Fn(&Option<String>) -> Option<String> + 'a,
) -> impl Iterator<Item = Change> + 'a {
    entries
        .iter()
        .filter(|e| e.before != e.after)
        .map(move |e| Change {
            when: When::Commit(e.commit.clone()),
            time: e.time,
            before: text_of(&e.before),
            after: text_of(&e.after),
        })
}

/// `id_of` returns the `id` of the document at `path` with `text`.
fn id_of(path: &RepoPath, text: &str) -> Option<String> {
    let source = Source::from_text(path.clone(), text);
    let (front, line) = source::split(&source).frontmatter?;
    core_fields(Some(yaml::parse(front, line).as_ref()), path).id
}

/// `listed` returns whether `git status` lists the file at `path`: whether
/// the working tree or the index changes it against `HEAD`, after git's own
/// conversions, such as a filter or `ident`, or git doesn't track it.
fn listed(git: &Git, path: &RepoPath) -> Result<bool, Failed> {
    git.run(&[
        "--no-optional-locks",
        "status",
        "--porcelain",
        "-z",
        "--untracked-files=all",
        "--",
        path.relative(),
    ])
    .map(|out| !out.is_empty())
    .ok_or(Failed)
}

/// `hash` returns git's blob of the file at `path` on disk, after git's own
/// conversions for the path, through `git hash-object`, which writes
/// nothing, or an empty text where the file is absent.
fn hash(git: &Git, path: &RepoPath) -> Result<String, Failed> {
    if !path.to_fs(&RepoRoot::new(git.dir.clone())).exists() {
        return Ok(String::new());
    }
    git.text(&["hash-object", "--", path.relative()])
        .ok_or(Failed)
}

/// `now` returns the time of the run, in seconds since the epoch.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(i64::MAX)
}

/// An `Entry` is one change of a commit to a file, as `git log --raw`
/// gives it: the commit, its committer date, and the blobs of the file
/// before and after, which are `None` where the file is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    commit: String,
    time: i64,
    before: Option<String>,
    after: Option<String>,
}

/// `log` returns each change to the file at `path` on the first-parent
/// history of `HEAD`, newest first, across each rename of the file, with
/// the committer date of its commit, or `Failed` where `git log` fails. A
/// merge counts by its change against its first parent, which is when the
/// change reached the branch.
// sokf-implements: CLI-4_AC-93
// sokf-implements: CLI-4_AC-94
fn log(git: &Git, path: &RepoPath) -> Result<Vec<Entry>, Failed> {
    let out = git
        .run(&[
            "log",
            "-m",
            "--first-parent",
            "--follow",
            "--root",
            "--raw",
            "--no-abbrev",
            "--no-color",
            "--no-show-signature",
            "-z",
            "--format=commit %H %ct",
            "HEAD",
            "--",
            path.relative(),
        ])
        .ok_or(Failed)?;
    Ok(entries(&out))
}

/// `entries` returns the entries of the output of `log`, whose format gives
/// each commit as `commit <id> <time>` before its raw entries.
fn entries(out: &[u8]) -> Vec<Entry> {
    git::raw(out)
        .into_iter()
        .filter_map(|r| {
            let header = r.header?;
            let mut parts = header.strip_prefix("commit ")?.split(' ');
            let commit = parts.next()?.to_string();
            let time = parts.next()?.parse().ok()?;
            Some(Entry {
                commit,
                time,
                before: r.before,
                after: r.after,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entries_of_a_log_hold_each_change_with_its_commit() {
        let out = b"commit aaa 200\0\n:100644 100644 1111 2222 M\0b/x.md\0\0commit bbb 100\0\n:100644 100644 1111 1111 R100\0a/x.md\0b/x.md\0\0commit ccc 50\0\n:000000 100644 0000 1111 A\0a/x.md\0";
        let found = entries(out);
        assert_eq!(
            found,
            [
                Entry {
                    commit: "aaa".into(),
                    time: 200,
                    before: Some("1111".into()),
                    after: Some("2222".into()),
                },
                Entry {
                    commit: "bbb".into(),
                    time: 100,
                    before: Some("1111".into()),
                    after: Some("1111".into()),
                },
                Entry {
                    commit: "ccc".into(),
                    time: 50,
                    before: None,
                    after: Some("1111".into()),
                },
            ]
        );
    }

    #[test]
    fn a_log_without_commits_holds_no_entry() {
        let empty = entries(b"");
        assert!(empty.is_empty(), "{empty:?}");
        let headless = entries(b":100644 100644 1 2 M\0x\0");
        assert!(headless.is_empty(), "{headless:?}");
    }
}
