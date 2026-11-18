//! For `sokf new`, this module reads the sequence numbers that the
//! repository's other worktrees and its local and remote-tracking branches
//! gave a type's documents, by adding or changing a file, since they parted
//! from the commit that the run's tree checks out (the decision "The
//! numbers of the other branches" of design-scaffold). Only such a
//! document can clash when its branch merges: every other document of a
//! branch is also in the run's tree, or the run's tree deleted it, and a
//! merge doesn't bring it back.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use super::IdTemplate;
use crate::{
    RepoPath, RepoRoot, Source,
    git::{self, Git, MIN_GIT, NO_LAZY_FETCH, Partial},
    model::core_fields,
    source::split,
    yaml,
};

/// A `Place` is where a document outside the run's own tree lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// A local or remote-tracking branch, by its short name.
    Branch(String),
    /// Another worktree of the repository, by its directory.
    Worktree(PathBuf),
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Place::Branch(name) => write!(f, "on the branch `{name}`"),
            Place::Worktree(dir) => write!(f, "in the worktree {}", dir.display()),
        }
    }
}

/// An `Elsewhere` is a document of the type outside the run's own tree:
/// where it lies, its path, its id, and its sequence number.
#[derive(Debug, Clone)]
pub struct Elsewhere {
    /// The branch or the worktree that holds the document.
    pub place: Place,
    /// The document's path in the repository.
    pub path: RepoPath,
    /// The document's id.
    pub id: String,
    /// The document's sequence number.
    pub seq: u64,
}

/// A `Scope` is where the scan looks: the directory that holds the type's
/// documents, and the roots of the stores nested inside it, whose
/// documents belong to another store.
pub struct Scope<'a> {
    /// The type's directory: the schema's `root`, or else the store's root.
    pub dir: &'a RepoPath,
    /// The roots of the other stores that lie inside `dir`.
    pub nested: &'a [RepoPath],
}

impl Scope<'_> {
    /// `holds` returns whether `path` lies in the scope and in no nested
    /// store.
    fn holds(&self, path: &RepoPath) -> bool {
        path.starts_with(self.dir) && !self.nested.iter().any(|n| path.starts_with(n))
    }

    /// `pathspec` returns the scope's directory as a git pathspec, relative
    /// to the top of the worktree, which `.` stands for, because git
    /// refuses an empty pathspec.
    fn pathspec(&self) -> &str {
        match self.dir.relative() {
            "" => ".",
            dir => dir,
        }
    }
}

/// `Blob` is a file outside the run's own tree: where it lies, its path,
/// and its text, when the repository holds it and the text is UTF-8.
type Blob = (Place, RepoPath, Option<String>);

/// `numbers` returns each document of `type_name` in `scope` that the
/// repository's other worktrees and its local and remote-tracking branches
/// added or changed since they parted from the commit that the run's tree
/// checks out, with its sequence number. A moved file counts as added at
/// its new path. It returns none outside a git repository, where `git`
/// fails to start, or where `root` isn't the top of the repository. It
/// leaves out a file whose frontmatter doesn't parse or whose id doesn't
/// match `template`, because only the run's own tree is the developer's to
/// repair. Where it can't read a file's text, in a partial clone, for a
/// blob that a clone lacks, or for text that isn't UTF-8, the file's name
/// gives the number, because reading a blob that a clone lacks would fetch
/// it from the network.
// sokf-implements: CLI-6_AC-74
// sokf-implements: CLI-6_AC-75
// sokf-implements: CLI-6_AC-76
pub fn numbers(
    root: &RepoRoot,
    scope: &Scope<'_>,
    type_name: &str,
    template: &IdTemplate,
) -> Vec<Elsewhere> {
    let git = Git::new(root.as_path());
    // A git older than `MIN_GIT` lacks some option of the scan, so the scan
    // reads nothing more than the working tree, as where git can't start.
    if git::version().is_none_or(|v| v < MIN_GIT) || !git.is_top() {
        return Vec::new();
    }
    let head = git.head();
    let partial = git.partial();
    let treeless = partial == Some(Partial::Treeless);
    let mut files = worktree_files(&git, head.as_deref(), scope, treeless);
    // A clone whose filter leaves out trees gives no branch's files,
    // because listing them would fetch the trees.
    let blobs = if treeless {
        Vec::new()
    } else {
        branch_blobs(&git, head.as_deref(), scope)
    };
    // A git that honours `GIT_NO_LAZY_FETCH` reads what a partial clone
    // holds and fetches nothing; an older one would fetch, so in a partial
    // clone the files' names alone give their numbers.
    if partial.is_some() && git::version().is_none_or(|v| v < NO_LAZY_FETCH) {
        files.extend(
            blobs
                .into_iter()
                .map(|(_, place, path)| (place, path, None)),
        );
    } else {
        files.extend(read_blobs(&git, &blobs));
    }
    files
        .into_iter()
        .filter_map(|(place, path, text)| {
            let (id, seq) = match &text {
                Some(text) => numbered(&path, text, type_name, template)?,
                None => named(&path, template)?,
            };
            Some(Elsewhere {
                place,
                path,
                id,
                seq,
            })
        })
        .collect()
}

/// `numbered` returns the id and the sequence number of the file at `path`
/// with `text`, when its frontmatter parses, its type is `type_name`, and
/// its id matches `template`.
fn numbered(
    path: &RepoPath,
    text: &str,
    type_name: &str,
    template: &IdTemplate,
) -> Option<(String, u64)> {
    let source = Source::from_text(path.clone(), text);
    let (front, line) = split(&source).frontmatter?;
    let parsed = yaml::parse(front, line);
    let core = core_fields(Some(parsed.as_ref()), path);
    if core.type_.as_deref() != Some(type_name) {
        return None;
    }
    let id = core.id?;
    let seq = template.seq_of(&id)?;
    Some((id, seq))
}

/// `named` returns the id and the sequence number that the name of the
/// file at `path` gives, when the name without `.md` matches `template`, as
/// the name of a document that `sokf new` wrote does.
fn named(path: &RepoPath, template: &IdTemplate) -> Option<(String, u64)> {
    let id = path.file_name().strip_suffix(".md")?;
    let seq = template.seq_of(id)?;
    Some((id.to_string(), seq))
}

/// `changed` returns each file in `scope` that the working tree of `git`
/// adds or changes, or turns into a file, against `base`, through `git
/// diff-index`, a plumbing command that never writes the index of a
/// worktree that another agent uses, and follows no diff configuration. A
/// moved file counts as added at its new path, and a file whose stat alone
/// differs counts as changed.
fn changed(git: &Git, base: &str, scope: &Scope<'_>) -> Option<Vec<RepoPath>> {
    let out = git.run(&[
        "diff-index",
        "--raw",
        "-z",
        "--no-renames",
        "--diff-filter=AMT",
        base,
        "--",
        scope.pathspec(),
    ])?;
    Some(
        raw(&out)
            .filter_map(|(_, path)| md_path(&path, scope))
            .collect(),
    )
}

/// `tracked` returns each file in `scope` that the index of `git` tracks,
/// through `git ls-files`, which reads no tree, and each that it doesn't
/// track and doesn't ignore, when `others` is set.
fn tracked(git: &Git, scope: &Scope<'_>, others: bool) -> Vec<RepoPath> {
    let mut args = vec!["ls-files", "-z"];
    if others {
        args.extend(["--others", "--exclude-standard"]);
    }
    args.extend(["--", scope.pathspec()]);
    git.run(&args)
        .map(|out| {
            out.split(|b| *b == 0)
                .filter_map(|p| md_path(&String::from_utf8_lossy(p), scope))
                .collect()
        })
        .unwrap_or_default()
}

/// `read_blobs` reads the text of each blob of `blobs` through one `git
/// cat-file --batch`, and returns each one with where it lies. A blob that
/// the repository lacks, whose text isn't UTF-8, or that `cat-file` didn't
/// give comes back without text, so that its file's name still gives its
/// number.
fn read_blobs(git: &Git, blobs: &[(String, Place, RepoPath)]) -> Vec<Blob> {
    let texts = git.blobs(blobs.iter().map(|(oid, _, _)| oid.as_str()));
    blobs
        .iter()
        .map(|(oid, place, path)| {
            let text = texts.get(oid.as_str()).cloned().flatten();
            (place.clone(), path.clone(), text)
        })
        .collect()
}

/// `raw` returns the new blob and the path of each entry of the output of
/// a plumbing diff with `--raw -z --no-renames`. A file of the working
/// tree that differs from the index has no blob yet, and keeps its path
/// with an empty blob.
fn raw(out: &[u8]) -> impl Iterator<Item = (String, String)> {
    git::raw(out)
        .into_iter()
        .filter_map(|r| Some((r.after.unwrap_or_default(), r.paths.last()?.clone())))
}

/// `md_path` returns `path`, a path from the top of the worktree, as a
/// repository path, when it is a `.md` file in `scope`.
fn md_path(path: &str, scope: &Scope<'_>) -> Option<RepoPath> {
    if Path::new(path).extension().is_none_or(|e| e != "md") {
        return None;
    }
    let path = RepoPath::parse(&format!("/{path}"))?;
    scope.holds(&path).then_some(path)
}

/// `worktree_files` returns each `.md` file in `scope` that another
/// worktree of the repository added or changed since its commit parted
/// from `head`, committed or not, and each file that it doesn't track and
/// doesn't ignore, as each stands on disk, so that a document that another
/// agent hasn't committed yet counts. A worktree whose commit has no merge
/// base with `head`, or any worktree of a treeless clone, where the merge
/// base's tree may be missing, gives every file in `scope` that its index
/// tracks.
fn worktree_files(git: &Git, head: Option<&str>, scope: &Scope<'_>, treeless: bool) -> Vec<Blob> {
    let own = fs::canonicalize(&git.dir).ok();
    // The list holds one path to a line, a form that every git with
    // worktrees prints.
    let list = git
        .run(&["worktree", "list", "--porcelain"])
        .unwrap_or_default();
    let others: Vec<PathBuf> = String::from_utf8_lossy(&list)
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .filter(|p| p.is_dir() && fs::canonicalize(p).ok() != own)
        .collect();
    let mut files = Vec::new();
    for wt in others {
        let there = Git::new(&wt);
        let base = (!treeless)
            .then(|| there.head())
            .flatten()
            .and_then(|commit| git.merge_base(head, &commit));
        let mut paths = base
            .and_then(|base| changed(&there, &base, scope))
            .unwrap_or_else(|| tracked(&there, scope, false));
        paths.extend(tracked(&there, scope, true));
        let wt_root = RepoRoot::new(wt.clone());
        for path in paths {
            if let Ok(bytes) = fs::read(path.to_fs(&wt_root)) {
                let text = String::from_utf8(bytes).ok();
                files.push((Place::Worktree(wt.clone()), path, text));
            }
        }
    }
    files
}

/// `branch_blobs` returns each `.md` file in `scope` that a local or
/// remote-tracking branch adds, changes, or turns into a file against its
/// merge base with `head`, through `git merge-base` and `git diff-tree`, so
/// that a number that the
/// branch later removed, or gave in a merge commit, counts as the branch's
/// tip has it. A moved file counts as added at its new path, so no rename
/// detection reads the blobs. A branch with no merge base, or any branch
/// without `head`, gives every file in `scope` of its tip. It leaves out a
/// branch merged into `head`, its own commit among them, and a symbolic ref such as
/// `refs/remotes/origin/HEAD`, which names another branch's commit under a
/// name that is no branch's. It lists each distinct commit once, with the
/// first branch, local ones first, that names it.
fn branch_blobs(
    git: &Git,
    head: Option<&str>,
    scope: &Scope<'_>,
) -> Vec<(String, Place, RepoPath)> {
    // A branch already merged into `head` gives nothing that the run's tree
    // lacks, so `--no-merged` leaves it out in the one call.
    let mut args = vec![
        "for-each-ref",
        "--format=%(objectname)%00%(symref)%00%(refname:short)",
    ];
    if let Some(head) = head {
        args.extend(["--no-merged", head]);
    }
    args.extend(["refs/heads", "refs/remotes"]);
    let refs = git.run(&args).unwrap_or_default();
    let mut commits = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut blobs = Vec::new();
    for line in String::from_utf8_lossy(&refs).lines() {
        let mut parts = line.split('\0');
        let (Some(commit), Some(""), Some(name)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if !commits.insert(commit.to_string()) {
            continue;
        }
        // `git merge-base` picks one base where criss-cross merges leave
        // several, and works on every git with worktrees.
        let diff = git.merge_base(head, commit).and_then(|base| {
            git.run(&[
                "diff-tree",
                "-r",
                "--raw",
                "-z",
                "--no-renames",
                "--no-abbrev",
                "--diff-filter=AMT",
                &base,
                commit,
                "--",
                scope.pathspec(),
            ])
        });
        let entries: Vec<(String, String)> = match diff {
            Some(out) => raw(&out).collect(),
            None => tree(git, commit, scope),
        };
        for (oid, path) in entries {
            if let Some(path) = md_path(&path, scope)
                && seen.insert((oid.clone(), path.clone()))
            {
                blobs.push((oid, Place::Branch(name.to_string()), path));
            }
        }
    }
    blobs
}

/// `tree` returns the blob and the path of each file in `scope` in
/// `commit`, for a branch that has no merge base with the run's tree.
fn tree(git: &Git, commit: &str, scope: &Scope<'_>) -> Vec<(String, String)> {
    let Some(out) = git.run(&["ls-tree", "-r", "-z", commit, "--", scope.pathspec()]) else {
        return Vec::new();
    };
    out.split(|b| *b == 0)
        .filter_map(|entry| {
            let entry = String::from_utf8_lossy(entry);
            let (meta, path) = entry.split_once('\t')?;
            let mut meta = meta.split(' ');
            match (meta.next(), meta.next(), meta.next()) {
                (Some(_), Some("blob"), Some(oid)) => Some((oid.to_string(), path.to_string())),
                _ => None,
            }
        })
        .collect()
}
