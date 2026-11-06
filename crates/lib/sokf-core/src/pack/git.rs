//! This module runs the user's `git`: it fetches a pack with a shallow
//! clone into a temporary directory (D-2 of design-pack), merges a file
//! with `git merge-file`, and prints a patch with `git diff --no-index`.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{Error, Result};

/// A `TempDir` is a directory under the system's temporary directory that
/// the tool removes when the value is dropped.
#[derive(Debug)]
pub struct TempDir(PathBuf);

/// `NEXT` numbers the temporary directories of one process.
static NEXT: AtomicUsize = AtomicUsize::new(0);

impl TempDir {
    /// `new` creates an empty temporary directory.
    pub fn new() -> Result<Self> {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("sokf-pack-{}-{n}-{nanos:08x}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        Ok(TempDir(dir))
    }

    /// `path` returns the directory.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `git` runs `git` with `args` in `dir` and returns its output. It fails
/// when the program can't start.
fn git(dir: Option<&Path>, args: &[&str]) -> Result<Output> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    // The clone never asks for a password on the terminal: a source that
    // needs one fails, and the user's credential helpers still answer.
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    // git converts no line ending of a file that it checks out or reads,
    // whatever the user's configuration says, such as the
    // `core.autocrlf=true` of git for Windows (the decision "The committed
    // bytes" of design-pack).
    cmd.args(["-c", "core.autocrlf=false", "-c", "core.eol=lf"]);
    cmd.args(args)
        .output()
        .map_err(|e| Error::Pack(format!("cannot run `git`: {e}")))
}

/// `NO_CONVERSION` is the attributes file of a clone. It turns off each
/// conversion that a pack's own `.gitattributes` can ask for at checkout,
/// so that the working tree holds the bytes that the pack committed. The
/// repository's `info/attributes` overrides every `.gitattributes`.
const NO_CONVERSION: &str = "* -text -ident -filter -working-tree-encoding\n";

/// `checkout` checks out `rev` in the clone `repo`, with the bytes of each
/// committed file.
// sokf-implements: CLI-13_AC-45
fn checkout(repo: &Path, rev: &str) -> Result<()> {
    let info = repo.join(".git").join("info");
    fs::create_dir_all(&info).map_err(|e| Error::io(&info, e))?;
    let attributes = info.join("attributes");
    fs::write(&attributes, NO_CONVERSION).map_err(|e| Error::io(&attributes, e))?;
    checked(Some(repo), &["checkout", "--quiet", "--force", rev])?;
    Ok(())
}

/// `checked` runs `git` and fails, with git's own message, when git exits
/// with an error.
fn checked(dir: Option<&Path>, args: &[&str]) -> Result<String> {
    let out = git(dir, args)?;
    if !out.status.success() {
        return Err(Error::Pack(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A `Clone` is a shallow clone of a source, with the ref that it checked
/// out and the commit of that ref.
#[derive(Debug)]
pub struct Clone {
    /// The temporary directory that holds the clone.
    pub dir: TempDir,
    /// The branch, tag, or commit that the clone checked out.
    pub git_ref: String,
    /// The commit that the clone checked out.
    pub commit: String,
}

impl Clone {
    /// `worktree` returns the clone's working tree.
    pub fn worktree(&self) -> PathBuf {
        self.dir.path().join("repo")
    }

    /// `executables` returns the path from the clone's root of each file
    /// that the checked-out commit records as executable. It reads the mode
    /// from git's tree, which records it on every platform, and not from the
    /// working tree, which has no mode on Windows.
    // sokf-implements: CLI-13_AC-59
    pub fn executables(&self) -> Result<BTreeSet<String>> {
        let out = git(Some(&self.worktree()), &["ls-tree", "-r", "-z", "HEAD"])?;
        if !out.status.success() {
            return Err(Error::Pack(format!(
                "`git ls-tree` failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter_map(|entry| {
                let (meta, path) = entry.split_once('\t')?;
                meta.starts_with("100755 ").then(|| path.to_string())
            })
            .collect())
    }
}

/// `shallow_clone` fetches `source` at `git_ref` into a temporary directory
/// with a clone of depth 1. Without `git_ref`, it fetches the head of the
/// source's default branch and returns that branch as the ref. A ref that
/// `clone --branch` refuses, such as a commit, is fetched by itself. The
/// clone checks out no file until `checkout` has turned off the pack's
/// conversions.
// sokf-implements: CLI-13_AC-2
// sokf-implements: CLI-13_AC-3
pub fn shallow_clone(source: &str, git_ref: Option<&str>) -> Result<Clone> {
    let dir = TempDir::new()?;
    let repo = dir.path().join("repo");
    let repo_arg = repo.to_string_lossy().into_owned();
    let clone = ["clone", "--quiet", "--no-checkout", "--depth", "1"];
    let git_ref = match git_ref {
        None => {
            checked(None, &[&clone[..], &["--", source, &repo_arg]].concat())?;
            checkout(&repo, "HEAD")?;
            checked(Some(&repo), &["rev-parse", "--abbrev-ref", "HEAD"])?
        }
        Some(r) => {
            let branch = git(
                None,
                &[&clone[..], &["--branch", r, "--", source, &repo_arg]].concat(),
            )?;
            if branch.status.success() {
                checkout(&repo, "HEAD")?;
            } else {
                let _ = fs::remove_dir_all(&repo);
                checked(None, &["init", "--quiet", &repo_arg])?;
                checked(
                    Some(&repo),
                    &["fetch", "--quiet", "--depth", "1", "--", source, r],
                )?;
                checkout(&repo, "FETCH_HEAD")?;
            }
            r.to_string()
        }
    };
    let commit = checked(Some(&repo), &["rev-parse", "HEAD"])?;
    Ok(Clone {
        dir,
        git_ref,
        commit,
    })
}

/// A `Merge` is the result of a three-way merge of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merge {
    /// The merged text, with git's conflict markers where the merge
    /// conflicts.
    pub text: Vec<u8>,
    /// Whether the merge conflicts.
    pub conflicted: bool,
}

/// `merge_file` merges the change from `base` to `upstream` into `current`
/// with `git merge-file`, and labels the sides of a conflict with `label`,
/// `base`, and `upstream`.
// sokf-implements: CLI-13_AC-9
pub fn merge_file(current: &[u8], base: &[u8], upstream: &[u8], label: &str) -> Result<Merge> {
    let dir = TempDir::new()?;
    let write = |name: &str, bytes: &[u8]| -> Result<String> {
        let p = dir.path().join(name);
        fs::write(&p, bytes).map_err(|e| Error::io(&p, e))?;
        Ok(p.to_string_lossy().into_owned())
    };
    let (c, b, u) = (
        write("current", current)?,
        write("base", base)?,
        write("upstream", upstream)?,
    );
    let out = git(
        None,
        &[
            "merge-file",
            "-p",
            "-L",
            label,
            "-L",
            "base",
            "-L",
            "upstream",
            &c,
            &b,
            &u,
        ],
    )?;
    // `git merge-file` exits with the number of conflicts, and with a
    // negative code, which the shell sees as 255, on an error.
    match out.status.code() {
        Some(0) => Ok(Merge {
            text: out.stdout,
            conflicted: false,
        }),
        Some(n) if (1..=127).contains(&n) => Ok(Merge {
            text: out.stdout,
            conflicted: true,
        }),
        _ => Err(Error::Pack(format!(
            "`git merge-file` failed on {label}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
    }
}

/// `diff` returns the unified diff from `base` to `current` of the file
/// `rel` of a pack, with the paths `a/<rel>` and `b/<rel>`, or an empty text
/// when the two are equal. `current` is `None` for a file that the user
/// deleted.
// sokf-implements: CLI-13_AC-30
pub fn diff(rel: &str, base: &[u8], current: Option<&[u8]>) -> Result<String> {
    let dir = TempDir::new()?;
    let write = |side: &str, bytes: &[u8]| -> Result<()> {
        let p = dir.path().join(side).join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        fs::write(&p, bytes).map_err(|e| Error::io(&p, e))
    };
    write("a", base)?;
    let a = format!("a/{rel}");
    let b = match current {
        Some(bytes) => {
            write("b", bytes)?;
            format!("b/{rel}")
        }
        None => "/dev/null".to_string(),
    };
    let out = git(
        Some(dir.path()),
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-index",
            "--no-color",
            "--no-ext-diff",
            "--no-prefix",
            "--",
            &a,
            &b,
        ],
    )?;
    // `git diff --no-index` exits with 1 when the files differ.
    match out.status.code() {
        Some(0) | Some(1) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Err(Error::Pack(format!(
            "`git diff` failed on {rel}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_merge_keeps_both_changes_or_marks_the_conflict() {
        let base = b"one\ntwo\nthree\nfour\nfive\n";
        let current = b"one\nTWO\nthree\nfour\nfive\n";
        let upstream = b"one\ntwo\nthree\nfour\nFIVE\n";
        let m = merge_file(current, base, upstream, "/knowledge/a.md").unwrap();
        assert!(!m.conflicted);
        assert_eq!(m.text, b"one\nTWO\nthree\nfour\nFIVE\n");
        let m = merge_file(current, base, b"one\n2\nthree\nfour\nfive\n", "/k/a.md").unwrap();
        assert!(m.conflicted);
        let text = String::from_utf8(m.text).unwrap();
        assert!(text.contains("<<<<<<< /k/a.md\n"), "{text}");
        assert!(text.contains(">>>>>>> upstream\n"), "{text}");
    }

    #[test]
    fn a_diff_names_the_paths_of_the_pack() {
        let d = diff("schemas/a.md", b"x\n", Some(b"y\n")).unwrap();
        assert!(d.contains("--- a/schemas/a.md\n"), "{d}");
        assert!(d.contains("+++ b/schemas/a.md\n"), "{d}");
        assert!(d.contains("-x\n+y\n"), "{d}");
        assert_eq!(diff("schemas/a.md", b"x\n", Some(b"x\n")).unwrap(), "");
    }

    #[test]
    fn a_temporary_directory_goes_when_dropped() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        assert!(path.is_dir());
        drop(dir);
        assert!(!path.exists());
    }
}
