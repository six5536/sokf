//! The walk collects the files that a run reads: every file under each
//! knowledge root, then the rest of the repository as CONF-3 sets out.

use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::Path,
    sync::{Arc, Mutex},
};

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use ignore::{DirEntry, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder, WalkState};

use super::{Tree, WalkedDir, WalkedFile, innermost};
use crate::{Config, Error, RepoPath, RepoRoot, Result, Stat, cache::CACHE_DIR};

/// A `Keep` limits the walk outside the knowledge roots to the paths for
/// which it returns true. The walk descends into a directory only when
/// `Keep` keeps the directory.
pub(crate) type Keep = Arc<dyn Fn(&RepoPath) -> bool + Send + Sync>;

/// `collect` walks the repository and returns its tree: every file and
/// directory under each knowledge root, then every other file that no
/// exclude and no gitignore rule matches.
// sokf-implements: CONF-2_AC-2
// sokf-implements: CLI-4_AC-1
// sokf-implements: SOKF_FORMAT_96
pub fn collect(root: &RepoRoot, config: &Config) -> Result<Tree> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let roots = config.roots.clone();

    // The walk visits a root inside another root once: a file or a
    // directory belongs to the innermost root that contains it. No exclude
    // and no gitignore rule prunes a root, so the run checks each knowledge
    // in full.
    // sokf-implements: CONF-2_AC-4
    // sokf-implements: CONF-3_AC-7
    let mut seen = BTreeSet::new();
    for knowledge in &roots {
        walk_under_root(root, &roots, knowledge, &mut seen, &mut files, &mut dirs)?;
    }
    files.extend(walk_outside(root, config, None)?);
    Ok(Tree::new(root.clone(), roots, files, dirs))
}

/// `walk_under_root` lists `dir`, which is a knowledge root or lies under
/// one, and every file and directory below it, as the walk of a root does.
/// It lists each path once across calls that share `seen`.
fn walk_under_root(
    root: &RepoRoot,
    roots: &[RepoPath],
    dir: &RepoPath,
    seen: &mut BTreeSet<RepoPath>,
    files: &mut Vec<WalkedFile>,
    dirs: &mut Vec<WalkedDir>,
) -> Result<()> {
    let mut visit = |path: &Path, is_dir: bool| {
        if let Some(rp) = RepoPath::from_fs(root, path)
            && seen.insert(rp.clone())
        {
            if is_dir {
                if let Some(owner) = innermost(roots, &rp) {
                    dirs.push(WalkedDir {
                        path: rp,
                        root: owner,
                    });
                }
            } else {
                push(files, rp, roots, stat(path));
            }
        }
    };
    visit(&dir.to_fs(root), true);
    walk_dir(&dir.to_fs(root), &mut visit)
}

/// `list_under_root` lists `path`, which lies under a knowledge root, as
/// the walk of that root would list it: a regular file alone, or a
/// directory with every file and directory below it. The walk reaches
/// `path` only through directories, never through a link, and it lists
/// neither a link nor a special file. `list_under_root` returns whether
/// the walk lists `path`.
// sokf-implements: CACHE-1_AC-48
pub(crate) fn list_under_root(
    root: &RepoRoot,
    roots: &[RepoPath],
    path: &RepoPath,
    seen: &mut BTreeSet<RepoPath>,
    files: &mut Vec<WalkedFile>,
    dirs: &mut Vec<WalkedDir>,
) -> Result<bool> {
    let Some(owner) = innermost(roots, path) else {
        return Ok(false);
    };
    let mut dir = path.parent();
    while let Some(d) = dir {
        if !d.starts_with(&roots[owner]) || d == roots[owner] {
            break;
        }
        if !is_dir(&d.to_fs(root)) {
            return Ok(false);
        }
        dir = d.parent();
    }
    let fs_path = path.to_fs(root);
    // The walk takes the type of an entry from its directory. When the
    // metadata of `path` can't be read, `list_under_root` does the same.
    let ty = match fs::symlink_metadata(&fs_path) {
        Ok(meta) => meta.file_type(),
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => match entry_type(&fs_path) {
            Some(ty) => ty,
            // The walk fails where it can't list a directory.
            None => return Err(Error::io(&fs_path, e)),
        },
    };
    if ty.is_dir() {
        walk_under_root(root, roots, path, seen, files, dirs)?;
        Ok(true)
    } else if ty.is_file() {
        if seen.insert(path.clone()) {
            push(files, path.clone(), roots, stat(&fs_path));
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// `is_dir` returns whether `path` is a directory and not a link to one.
/// When the metadata can't be read for a reason other than its absence,
/// `is_dir` returns true, so that the caller goes on to the entry below,
/// whose own read decides the result.
fn is_dir(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(meta) => meta.is_dir(),
        Err(e) => e.kind() != ErrorKind::NotFound,
    }
}

/// `entry_type` returns the type of `path` as its directory lists it.
fn entry_type(path: &Path) -> Option<fs::FileType> {
    let name = path.file_name()?;
    fs::read_dir(path.parent()?)
        .ok()?
        .flatten()
        .find(|e| e.file_name() == name)?
        .file_type()
        .ok()
}

/// `walk_outside` walks the repository outside the knowledge roots and
/// returns every file that no exclude and no gitignore rule matches. With
/// `keep`, it walks only the paths that `keep` keeps.
pub(crate) fn walk_outside(
    root: &RepoRoot,
    config: &Config,
    keep: Option<Keep>,
) -> Result<Vec<WalkedFile>> {
    let roots = &config.roots;
    // sokf-implements: CONF-3_AC-1
    // sokf-implements: CONF-3_AC-2
    let exclude = exclude_set(&config.exclude)?;
    let root_fs = root.as_path().to_path_buf();
    let pruned: Vec<std::path::PathBuf> = roots.iter().map(|r| r.to_fs(root)).collect();
    let mut builder = WalkBuilder::new(&root_fs);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .require_git(false);
    let root_for_filter = root.clone();
    builder.filter_entry(move |entry| {
        let path = entry.path();
        if path.file_name().is_some_and(|n| n == ".git") {
            return false;
        }
        if pruned.iter().any(|p| path == p) {
            return false;
        }
        match RepoPath::from_fs(&root_for_filter, path) {
            // The walk skips the cache, which belongs to the tool: sokf
            // rebuilds an unreadable cache and doesn't report it
            // (CLI-4_AC-19).
            // sokf-implements: CACHE-2_AC-1
            Some(rp) => {
                rp.relative() != CACHE_DIR
                    && !exclude.is_match(rp.relative())
                    && keep.as_ref().is_none_or(|keep| keep(&rp))
            }
            None => false,
        }
    });
    // The walk runs on many threads, and each thread stats the files it
    // finds. The tree sorts the files afterwards (CACHE-1_AC-24).
    // sokf-implements: CACHE-3_AC-9
    let shared = Shared {
        root,
        roots,
        files: Mutex::new(Vec::new()),
        failed: Mutex::new(None),
    };
    builder.build_parallel().visit(&mut &shared);
    if let Some(e) = shared
        .failed
        .into_inner()
        .unwrap_or_else(|e| e.into_inner())
    {
        return Err(Error::io(&root_fs, std::io::Error::other(e)));
    }
    Ok(shared.files.into_inner().unwrap_or_else(|e| e.into_inner()))
}

/// `Shared` holds what the threads of the walk share: the files they
/// found and the first error that any thread hits while walking.
struct Shared<'a> {
    root: &'a RepoRoot,
    roots: &'a [RepoPath],
    files: Mutex<Vec<WalkedFile>>,
    failed: Mutex<Option<ignore::Error>>,
}

/// `Visitor` is one thread of the walk: it holds the files that the
/// thread finds, and adds them to the shared list when it is dropped.
struct Visitor<'s, 'a> {
    shared: &'s Shared<'a>,
    found: Vec<WalkedFile>,
}

impl<'s, 'a> ParallelVisitorBuilder<'s> for &'s Shared<'a> {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        Box::new(Visitor {
            shared: self,
            found: Vec::new(),
        })
    }
}

impl ParallelVisitor for Visitor<'_, '_> {
    fn visit(&mut self, entry: std::result::Result<DirEntry, ignore::Error>) -> WalkState {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                self.shared
                    .failed
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_or_insert(e);
                return WalkState::Quit;
            }
        };
        if entry.file_type().is_some_and(|t| t.is_file())
            && let Some(rp) = RepoPath::from_fs(self.shared.root, entry.path())
        {
            push(&mut self.found, rp, self.shared.roots, stat(entry.path()));
        }
        WalkState::Continue
    }
}

impl Drop for Visitor<'_, '_> {
    fn drop(&mut self) {
        self.shared
            .files
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .append(&mut self.found);
    }
}

fn push(files: &mut Vec<WalkedFile>, path: RepoPath, roots: &[RepoPath], stat: Stat) {
    let (root, kind) = Tree::classify(&path, roots);
    files.push(WalkedFile {
        path,
        root,
        kind,
        stat,
    });
}

/// `stat` returns the stat of a file that the walk lists: it makes one
/// call per file and doesn't read the file (CACHE-1_AC-1). When that call
/// fails, `stat` returns the default stat, which matches no stored key, so
/// the run reads the file.
fn stat(path: &Path) -> Stat {
    fs::metadata(path).map(|m| Stat::of(&m)).unwrap_or_default()
}

/// `walk_dir` visits every file and directory under `dir` in path order,
/// each directory before its contents. The second argument of `visit` is
/// `true` when the entry is a directory.
fn walk_dir(dir: &Path, visit: &mut dyn FnMut(&Path, bool)) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| Error::io(dir, e))?
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| Error::io(dir, e))?;
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let ty = entry.file_type().map_err(|e| Error::io(&path, e))?;
        if ty.is_dir() {
            visit(&path, true);
            walk_dir(&path, visit)?;
        } else if ty.is_file() {
            visit(&path, false);
        }
    }
    Ok(())
}

/// `repo_glob` builds a glob that a path from the repository root, without
/// its leading `/`, matches. As in gitignore, `*` doesn't match `/`, as it
/// doesn't in the globs of the format specification.
pub(crate) fn repo_glob(g: &str) -> std::result::Result<Glob, globset::Error> {
    GlobBuilder::new(g.trim_start_matches('/'))
        .literal_separator(true)
        .build()
}

/// `exclude_set` builds the walk's excludes into one matcher.
fn exclude_set(globs: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        let glob = repo_glob(g).map_err(|e| Error::Internal(format!("exclude glob `{g}`: {e}")))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| Error::Internal(format!("exclude globs: {e}")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::FileKind;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("collect-{name}"));
        for d in [
            "knowledge/schemas",
            "knowledge/sub",
            "docs/vendor",
            "ignored",
            ".git/objects",
            ".sokf",
        ] {
            fs::create_dir_all(dir.join(d)).unwrap();
        }
        for f in [
            "knowledge/manifest.sokf.yaml",
            "knowledge/index.md",
            "knowledge/schemas/issue.md",
            "knowledge/sub/a.md",
            "knowledge/sub/ignored.md",
            "knowledge/notes.txt",
            "docs/README.md",
            "docs/vendor/x.md",
            "ignored/y.md",
            ".git/objects/z",
            ".sokf/config.toml",
            "README.md",
        ] {
            fs::write(dir.join(f), "").unwrap();
        }
        fs::write(dir.join(".gitignore"), "/ignored/\nignored.md\n").unwrap();
        dir
    }

    // sokf-tests: CONF-3_AC-1
    // sokf-tests: CONF-3_AC-2
    // sokf-tests: CONF-3_AC-7
    #[test]
    fn collects_roots_fully_and_the_repository_per_ignore_rules() {
        let dir = temp("roots");
        let root = RepoRoot::new(dir.clone());
        let config = Config {
            roots: vec![p("/knowledge")],
            // An exclude inside the root hides no file in the knowledge.
            exclude: vec!["/docs/vendor/**".into(), "/knowledge/sub/**".into()],
            ..Config::default()
        };
        let tree = collect(&root, &config).unwrap();
        let paths: Vec<&str> = tree.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "/.gitignore",
                "/.sokf/config.toml",
                "/README.md",
                "/docs/README.md",
                "/knowledge/index.md",
                "/knowledge/manifest.sokf.yaml",
                "/knowledge/notes.txt",
                "/knowledge/schemas/issue.md",
                "/knowledge/sub/a.md",
                "/knowledge/sub/ignored.md",
            ]
        );
        let kind = |s: &str| {
            tree.files
                .iter()
                .find(|f| f.path.as_str() == s)
                .unwrap()
                .kind
        };
        assert_eq!(kind("/knowledge/manifest.sokf.yaml"), FileKind::Manifest);
        assert_eq!(kind("/knowledge/schemas/issue.md"), FileKind::Schema);
        assert_eq!(kind("/knowledge/sub/a.md"), FileKind::Document);
        assert_eq!(kind("/knowledge/index.md"), FileKind::Index);
        assert_eq!(kind("/README.md"), FileKind::Candidate);
        assert_eq!(kind("/knowledge/notes.txt"), FileKind::Candidate);
        // Path resolution still finds the excluded and ignored files.
        assert!(tree.exists(&p("/docs/vendor/x.md")));
        assert!(tree.exists(&p("/ignored/y.md")));
        assert_eq!(tree.roots, vec![p("/knowledge")]);
        // The tree lists every directory under the root in path order,
        // the root first.
        let dirs: Vec<(&str, usize)> = tree
            .dirs
            .iter()
            .map(|d| (d.path.as_str(), d.root))
            .collect();
        assert_eq!(
            dirs,
            vec![
                ("/knowledge", 0),
                ("/knowledge/schemas", 0),
                ("/knowledge/sub", 0)
            ]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    // sokf-tests: CACHE-2_AC-1
    #[test]
    fn an_unreadable_cache_directory_is_not_walked() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp("cache");
        let cache = dir.join(CACHE_DIR);
        fs::create_dir_all(&cache).unwrap();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o000)).unwrap();
        let root = RepoRoot::new(dir.clone());
        let config = Config::default();
        let result = collect(&root, &config);
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o755)).unwrap();
        let tree = result.unwrap();
        assert!(
            tree.files
                .iter()
                .all(|f| !f.path.as_str().starts_with("/.sokf/cache"))
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-2_AC-4
    #[test]
    fn a_nested_root_is_walked_once_and_owns_its_files() {
        let dir = temp("nested");
        fs::create_dir_all(dir.join("knowledge/inner")).unwrap();
        fs::write(dir.join("knowledge/inner/manifest.sokf.yaml"), "").unwrap();
        fs::write(dir.join("knowledge/inner/i.md"), "").unwrap();
        let root = RepoRoot::new(dir.clone());
        let config = Config {
            roots: vec![p("/knowledge"), p("/knowledge/inner")],
            ..Config::default()
        };
        let tree = collect(&root, &config).unwrap();
        let inner: Vec<&WalkedFile> = tree
            .files
            .iter()
            .filter(|f| f.path.starts_with(&p("/knowledge/inner")))
            .collect();
        assert_eq!(inner.len(), 2, "{inner:?}");
        assert!(inner.iter().all(|f| f.root == Some(1)));
        assert_eq!(
            inner.iter().map(|f| f.kind).collect::<Vec<_>>(),
            vec![FileKind::Document, FileKind::Manifest]
        );
        assert_eq!(
            tree.files
                .iter()
                .filter(|f| f.path == p("/knowledge/sub/a.md"))
                .count(),
            1
        );
        // The walk visits the nested root's directory once, and the
        // nested root owns it.
        let inner_dirs: Vec<(&str, usize)> = tree
            .dirs
            .iter()
            .filter(|d| d.path.starts_with(&p("/knowledge/inner")))
            .map(|d| (d.path.as_str(), d.root))
            .collect();
        assert_eq!(inner_dirs, vec![("/knowledge/inner", 1)]);
        assert_eq!(tree.dirs.len(), 4, "{:?}", tree.dirs);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_root_is_an_io_error() {
        let dir = temp("missing");
        let root = RepoRoot::new(dir.clone());
        let config = Config {
            roots: vec![p("/nope")],
            ..Config::default()
        };
        assert!(matches!(collect(&root, &config), Err(Error::Io { .. })));
        assert!(matches!(
            exclude_set(&["/[".into()]),
            Err(Error::Internal(_))
        ));
        fs::remove_dir_all(&dir).unwrap();
    }
}
