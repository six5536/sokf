//! This module finds the repository root and turns the `PATHS` arguments
//! of a command into a scope.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::{Error, RepoPath, RepoRoot, Result};

/// `discover_root` returns the repository root. When the caller passes
/// `given`, the root is that directory's canonical path, and the directory
/// must exist. Otherwise the root is the nearest ancestor of `cwd` that holds
/// a `.git` directory or file. The root is canonical so that a path resolved
/// from the current directory, which the operating system reports in
/// canonical form, lies under it even when `--repo-root` reaches the
/// directory through a symbolic link, as the temporary directory of macOS
/// does.
// sokf-implements: CLI-1_AC-3
// sokf-implements: CLI-1_AC-4
// sokf-implements: CLI-1_AC-5
// sokf-implements: CLI-1_AC-18
pub fn discover_root(cwd: &Path, given: Option<&Path>) -> Result<RepoRoot> {
    if let Some(dir) = given {
        let abs = std::path::absolute(dir).map_err(|e| Error::io(dir, e))?;
        if !abs.is_dir() {
            return Err(Error::io(
                &abs,
                std::io::Error::new(std::io::ErrorKind::NotFound, "no such directory"),
            ));
        }
        let canonical = dunce::canonicalize(&abs).map_err(|e| Error::io(&abs, e))?;
        return Ok(RepoRoot::new(canonical));
    }
    let abs = std::path::absolute(cwd).map_err(|e| Error::io(cwd, e))?;
    abs.ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(|dir| RepoRoot::new(dir.to_path_buf()))
        .ok_or(Error::NoRepository)
}

/// `Scope` holds the files and directories that the `PATHS` arguments of
/// `validate` point to (CLI-4_AC-2). When `PATHS` is empty, the scope is the
/// whole repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    /// `files` holds the files that the arguments point to.
    pub files: BTreeSet<RepoPath>,
    /// `dirs` holds the directories that the arguments point to.
    pub dirs: BTreeSet<RepoPath>,
}

impl Scope {
    /// `all` returns a scope that holds the whole repository.
    pub fn all() -> Self {
        let mut s = Scope::default();
        s.dirs.insert(RepoPath::root());
        s
    }

    /// `contains` reports whether `path` is one of the scope's files or lies
    /// under one of its directories.
    pub fn contains(&self, path: &RepoPath) -> bool {
        self.files.contains(path) || self.dirs.iter().any(|d| path.starts_with(d))
    }

    /// `add` adds a file to the scope. When `validate --fix` moves a file,
    /// it uses `add` to put the file's new path in the scope.
    pub fn add(&mut self, path: RepoPath) {
        self.files.insert(path);
    }
}

/// `PathArg` is a path argument, written either as a spec path or as a
/// filesystem path (CLI-1_AC-7, CLI-1_AC-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathArg<'a>(pub &'a str);

impl PathArg<'_> {
    /// `resolve` returns the path that the argument refers to, but only when
    /// a file or directory exists there. It first reads an argument that
    /// begins with `/` as a spec path, and then as an absolute filesystem
    /// path inside the root. It resolves any other argument from `cwd`. An
    /// argument can end with `/` only when it refers to a directory.
    // sokf-implements: CLI-1_AC-7
    // sokf-implements: CLI-1_AC-8
    pub fn resolve(&self, root: &RepoRoot, cwd: &Path) -> Option<RepoPath> {
        let arg = self.0;
        let dir_only = arg.len() > 1 && arg.ends_with('/');
        let found = if arg.starts_with('/') {
            let trimmed = if dir_only {
                arg.trim_end_matches('/')
            } else {
                arg
            };
            match RepoPath::parse(trimmed) {
                Some(spec) if spec.to_fs(root).exists() => Some(spec),
                _ => Self::existing(root, Path::new(arg)),
            }
        } else {
            Self::existing(root, &cwd.join(arg))
        };
        found.filter(|p| !dir_only || p.to_fs(root).is_dir())
    }

    /// `resolve_new` returns the path of a directory that might not exist
    /// yet, as in `init DIR`. It resolves an existing path as `resolve` does.
    /// It reads an absent path that begins with `/` as an absolute filesystem
    /// path when the path starts with the path of the repository root, and as
    /// a spec path otherwise. It resolves any other argument from `cwd`. The
    /// argument can end with `/`, because the path is always a directory.
    // sokf-implements: CLI-2_AC-23
    // sokf-implements: CLI-2_AC-29
    pub fn resolve_new(&self, root: &RepoRoot, cwd: &Path) -> Option<RepoPath> {
        if let Some(found) = self.resolve(root, cwd) {
            return Some(found);
        }
        let arg = if self.0.len() > 1 {
            self.0.trim_end_matches('/')
        } else {
            self.0
        };
        if !arg.starts_with('/') {
            return RepoPath::from_fs(root, &cwd.join(arg));
        }
        let under_root = root.as_path().to_str().is_some_and(|r| {
            arg == r
                || arg
                    .strip_prefix(r)
                    .is_some_and(|rest| rest.starts_with('/'))
        });
        if under_root {
            RepoPath::from_fs(root, Path::new(arg))
        } else {
            RepoPath::parse(arg)
        }
    }

    fn existing(root: &RepoRoot, fs: &Path) -> Option<RepoPath> {
        RepoPath::from_fs(root, fs).filter(|p| p.to_fs(root).exists())
    }
}

/// `resolve_paths` turns the `PATHS` arguments into a scope. It refuses a
/// path that doesn't exist or that lies outside the root.
// sokf-implements: CLI-4_AC-15
pub fn resolve_paths(root: &RepoRoot, cwd: &Path, paths: &[PathBuf]) -> Result<Scope> {
    if paths.is_empty() {
        return Ok(Scope::all());
    }
    let mut scope = Scope::default();
    for path in paths {
        let shown = path.display().to_string();
        let repo_path = path
            .to_str()
            .and_then(|s| PathArg(s).resolve(root, cwd))
            .ok_or(Error::Path(shown))?;
        if repo_path.to_fs(root).is_dir() {
            scope.dirs.insert(repo_path);
        } else {
            scope.files.insert(repo_path);
        }
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("root-{name}"));
        fs::create_dir_all(dir.join("k/sub")).unwrap();
        fs::write(dir.join("k/a.md"), "").unwrap();
        dir
    }

    // sokf-tests: CLI-1_AC-4
    #[test]
    fn discovers_the_git_ancestor_or_takes_the_given_root() {
        let dir = temp("discover");
        assert!(matches!(
            discover_root(&dir.join("k/sub"), None),
            Err(Error::NoRepository)
        ));
        fs::write(dir.join(".git"), "gitdir: elsewhere").unwrap();
        assert_eq!(
            discover_root(&dir.join("k/sub"), None).unwrap().as_path(),
            dir
        );
        assert_eq!(
            discover_root(&dir.join("k/sub"), Some(&dir.join("k")))
                .unwrap()
                .as_path(),
            dunce::canonicalize(dir.join("k")).unwrap()
        );
        assert!(discover_root(&dir, Some(&dir.join("nope"))).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolves_paths_into_files_and_dirs() {
        let dir = temp("paths");
        let root = RepoRoot::new(dir.clone());
        let scope = resolve_paths(&root, &dir, &["k/a.md".into(), "k/sub".into()]).unwrap();
        assert!(scope.files.contains(&RepoPath::parse("/k/a.md").unwrap()));
        assert!(scope.dirs.contains(&RepoPath::parse("/k/sub").unwrap()));
        assert!(scope.contains(&RepoPath::parse("/k/sub/x.md").unwrap()));
        assert!(!scope.contains(&RepoPath::parse("/k/b.md").unwrap()));
        // From a subdirectory, a path resolves relative to that directory.
        let scope = resolve_paths(&root, &dir.join("k"), &["a.md".into()]).unwrap();
        assert!(scope.contains(&RepoPath::parse("/k/a.md").unwrap()));
        // An absolute path also resolves.
        let scope = resolve_paths(&root, &dir, &[dir.join("k")]).unwrap();
        assert!(scope.dirs.contains(&RepoPath::parse("/k").unwrap()));
        assert!(matches!(
            resolve_paths(&root, &dir, &["k/missing.md".into()]),
            Err(Error::Path(_))
        ));
        assert!(matches!(
            resolve_paths(&root, &dir, &["../outside".into()]),
            Err(Error::Path(_))
        ));
        assert_eq!(resolve_paths(&root, &dir, &[]).unwrap(), Scope::all());
        // A spec path also resolves (CLI-1_AC-7).
        let scope = resolve_paths(&root, &dir.join("k/sub"), &["/k/a.md".into()]).unwrap();
        assert!(scope.files.contains(&RepoPath::parse("/k/a.md").unwrap()));
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-1_AC-7
    // sokf-tests: CLI-1_AC-8
    #[test]
    fn a_path_argument_has_two_spellings() {
        let dir = temp("patharg");
        let root = RepoRoot::new(dir.clone());
        let p = |s: &str| RepoPath::parse(s).unwrap();
        // An argument that begins with `/` is a spec path when that path exists.
        assert_eq!(PathArg("/k/a.md").resolve(&root, &dir), Some(p("/k/a.md")));
        assert_eq!(PathArg("/k/sub/").resolve(&root, &dir), Some(p("/k/sub")));
        // A trailing `/` means a directory, never a file.
        assert_eq!(PathArg("/k/a.md/").resolve(&root, &dir), None);
        assert_eq!(PathArg("a.md/").resolve(&root, &dir.join("k")), None);
        assert_eq!(PathArg("/").resolve(&root, &dir), Some(p("/")));
        // When no spec path exists there, an argument that begins with `/`
        // is an absolute filesystem path inside the root.
        let abs = dir.join("k/a.md");
        assert_eq!(
            PathArg(abs.to_str().unwrap()).resolve(&root, &dir),
            Some(p("/k/a.md"))
        );
        assert_eq!(
            PathArg(dir.to_str().unwrap()).resolve(&root, &dir),
            Some(p("/"))
        );
        // Any other argument resolves from the current directory.
        assert_eq!(
            PathArg("a.md").resolve(&root, &dir.join("k")),
            Some(p("/k/a.md"))
        );
        assert_eq!(
            PathArg("./a.md").resolve(&root, &dir.join("k")),
            Some(p("/k/a.md"))
        );
        assert_eq!(
            PathArg("sub/").resolve(&root, &dir.join("k")),
            Some(p("/k/sub"))
        );
        assert_eq!(PathArg("..").resolve(&root, &dir.join("k")), Some(p("/")));
        // An argument that points to nothing, or to a path outside the root,
        // resolves to no path.
        assert_eq!(PathArg("/k/missing.md").resolve(&root, &dir), None);
        assert_eq!(PathArg("missing.md").resolve(&root, &dir), None);
        assert_eq!(PathArg("../outside").resolve(&root, &dir), None);
        assert_eq!(PathArg("/definitely/not/here").resolve(&root, &dir), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-2_AC-23
    // sokf-tests: CLI-2_AC-29
    #[test]
    fn a_new_path_resolves_in_every_spelling() {
        let dir = temp("new");
        let root = RepoRoot::new(dir.clone());
        let p = |s: &str| RepoPath::parse(s).unwrap();
        // An existing path resolves as `resolve` does.
        assert_eq!(PathArg("k/sub").resolve_new(&root, &dir), Some(p("/k/sub")));
        assert_eq!(
            PathArg("/k/a.md").resolve_new(&root, &dir),
            Some(p("/k/a.md"))
        );
        // An absent spec path resolves, with or without a trailing slash.
        assert_eq!(
            PathArg("/docs/kb").resolve_new(&root, &dir),
            Some(p("/docs/kb"))
        );
        assert_eq!(
            PathArg("/docs/kb/").resolve_new(&root, &dir),
            Some(p("/docs/kb"))
        );
        // An absent absolute path inside the repository resolves.
        let abs = dir.join("docs/kb");
        assert_eq!(
            PathArg(abs.to_str().unwrap()).resolve_new(&root, &dir),
            Some(p("/docs/kb"))
        );
        // An absent absolute path outside the repository reads as a spec path.
        assert_eq!(
            PathArg("/definitely/not/here").resolve_new(&root, &dir),
            Some(p("/definitely/not/here"))
        );
        // A relative path resolves from the current directory, and one that
        // leaves the repository resolves to no path.
        assert_eq!(
            PathArg("kb").resolve_new(&root, &dir.join("k")),
            Some(p("/k/kb"))
        );
        assert_eq!(
            PathArg("new/dir/").resolve_new(&root, &dir),
            Some(p("/new/dir"))
        );
        assert_eq!(PathArg("../outside").resolve_new(&root, &dir), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn scope_all_contains_everything_and_add_names_a_file() {
        let all = Scope::all();
        assert!(all.contains(&RepoPath::parse("/x/y.md").unwrap()));
        let mut s = Scope::default();
        assert!(!s.contains(&RepoPath::parse("/x.md").unwrap()));
        s.add(RepoPath::parse("/x.md").unwrap());
        assert!(s.contains(&RepoPath::parse("/x.md").unwrap()));
    }
}
