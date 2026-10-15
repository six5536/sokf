//! This module holds the hint, the list of the paths that changed
//! (CACHE-1_AC-23). It also builds the tree for a run with a hint. A caller
//! that knows which paths changed passes them in, such as `--changed` or a
//! language server's file watcher. The run then lists again only those
//! paths, by the walk's rules, and uses the store's rows for every other
//! file and directory. A hinted directory stands for every path under it.
//! The next run without a hint finds any change that the hint leaves out.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Arc,
};

use super::{Cache, sync::kind_of};
use crate::{
    Config, RepoPath, RepoRoot, Result, Stat, Tree,
    walk::{self, Keep, WalkedDir, WalkedFile, innermost},
};

impl Cache {
    /// `set_hint` records the paths that a caller knows changed, so that the
    /// next sync reads the file metadata of only those paths.
    pub fn set_hint(&self, paths: Vec<RepoPath>) {
        *self.hint.borrow_mut() = Some(paths);
    }

    /// `drop_hint` forgets the paths that a caller said changed, so that the
    /// next sync walks the repository.
    pub fn drop_hint(&self) {
        *self.hint.borrow_mut() = None;
        self.missed.set(false);
    }

    /// `note_missing` records that the run read a file that the walk lists
    /// and found it removed. In a run with a hint on a filled store, the
    /// hint left that removal out (CACHE-1_AC-53).
    pub fn note_missing(&self) {
        if self.hint.borrow().is_some() && !self.is_fresh() {
            self.missed.set(true);
        }
    }

    /// `missed` returns whether this run's hint left out a removed file
    /// that the run read.
    pub fn missed(&self) -> bool {
        self.missed.get()
    }

    /// `hint` returns the paths that a caller said changed, or `None` when
    /// the caller gave none.
    pub fn hint(&self) -> Option<Vec<RepoPath>> {
        self.hint.borrow().clone()
    }

    /// `tree_from_store` builds the tree that a run with a hint works on. It
    /// starts from the store's files and directories. It drops what the
    /// store recorded at or under each hinted path, and lists those paths
    /// again as the walk lists them. It adds each directory above a hinted
    /// path that the walk lists, up to its root. It removes each directory
    /// above a hinted path that is no longer a directory. When no check has
    /// filled the store, it walks the repository instead.
    // sokf-implements: CACHE-1_AC-23
    // sokf-implements: CACHE-1_AC-28
    // sokf-implements: CACHE-1_AC-46
    // sokf-implements: CACHE-1_AC-47
    // sokf-implements: CACHE-1_AC-49
    // sokf-implements: CACHE-1_AC-50
    // sokf-implements: CLI-4_AC-21
    pub fn tree_from_store(
        &self,
        root: &RepoRoot,
        config: &Config,
        changed: &[RepoPath],
    ) -> Result<Tree> {
        if self.is_fresh() {
            return walk::collect(root, config);
        }
        let roots = config.roots.clone();
        let hints = scopes(changed);
        // A hint that holds a whole root lists as much as a walk does.
        if covers_root(&hints, &roots) {
            return walk::collect(root, config);
        }
        let mut files = self.stored_files();
        drop_scopes(&mut files, &hints);
        let mut dirs = self.stored_dirs();
        let hint_keys = keys(&hints);
        dirs.retain(|d, _| !in_scope(&hint_keys, d.as_str()));

        let mut seen = BTreeSet::new();
        let mut found_dirs = Vec::new();
        let mut outside = BTreeSet::new();
        for path in &hints {
            if innermost(&roots, path).is_none() {
                outside.insert(path.clone());
                continue;
            }
            let listed =
                walk::list_under_root(root, &roots, path, &mut seen, &mut files, &mut found_dirs)?;
            let mut dir = path.parent();
            while let Some(d) = dir {
                let Some(owner) = innermost(&roots, &d) else {
                    break;
                };
                if listed {
                    dirs.entry(d.clone()).or_insert(owner);
                } else if is_walked_dir(root, &roots[owner], &d) {
                    break;
                } else {
                    dirs.remove(&d);
                }
                dir = d.parent();
            }
        }
        if !outside.is_empty() {
            // The walk outside the roots descends only into the directories
            // above a hinted path and into the hinted paths themselves.
            let above: BTreeSet<RepoPath> = outside
                .iter()
                .flat_map(|h| std::iter::successors(h.parent(), RepoPath::parent))
                .collect();
            let scope: Vec<String> = outside.iter().map(ToString::to_string).collect();
            let keep: Keep =
                Arc::new(move |p: &RepoPath| above.contains(p) || in_scope(&scope, p.as_str()));
            let outside_keys = keys(&outside);
            files.extend(
                walk::walk_outside(root, config, Some(keep))?
                    .into_iter()
                    .filter(|f| in_scope(&outside_keys, f.path.as_str())),
            );
        }
        for d in found_dirs {
            dirs.insert(d.path, d.root);
        }
        Ok(Tree::new(
            root.clone(),
            roots,
            files,
            dirs.into_iter()
                .map(|(path, root)| WalkedDir { path, root })
                .collect(),
        ))
    }

    /// `stored_files` returns the files that the store recorded, with their
    /// kinds, roots, and stats.
    fn stored_files(&self) -> Vec<WalkedFile> {
        let mut files = Vec::new();
        if let Ok(mut stmt) = self
            .conn()
            .prepare("SELECT path, kind, root, len, mtime FROM files ORDER BY path")
            && let Ok(mut rows) = stmt.query([])
        {
            while let Ok(Some(r)) = rows.next() {
                let Some(path) = r
                    .get_ref(0)
                    .ok()
                    .and_then(|v| v.as_str().ok())
                    .and_then(RepoPath::parse)
                else {
                    continue;
                };
                let (Ok(kind), Ok(owner), Ok(len), Ok(mtime)) = (
                    r.get::<_, u8>(1),
                    r.get::<_, Option<i64>>(2),
                    r.get::<_, i64>(3),
                    r.get::<_, Option<i64>>(4),
                ) else {
                    continue;
                };
                files.push(WalkedFile {
                    path,
                    root: owner.map(|r| r as usize),
                    kind: kind_of(kind),
                    stat: Stat {
                        len: len as u64,
                        mtime,
                    },
                });
            }
        }
        files
    }

    /// `stored_dirs` returns the directories that the store recorded, each
    /// with the index of its root.
    pub(super) fn stored_dirs(&self) -> BTreeMap<RepoPath, usize> {
        self.conn()
            .rows("SELECT path, root FROM dirs", [], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .into_iter()
            .filter_map(|(path, owner)| Some((RepoPath::parse(&path)?, owner as usize)))
            .collect()
    }
}

/// `scopes` returns the hinted paths in path order, each once, without a
/// path that lies under another hinted path, because the other path's
/// scope already covers it.
pub(super) fn scopes(changed: &[RepoPath]) -> BTreeSet<RepoPath> {
    let all: BTreeSet<&RepoPath> = changed.iter().collect();
    all.iter()
        .filter(|p| !std::iter::successors(p.parent(), RepoPath::parent).any(|a| all.contains(&a)))
        .map(|p| (*p).clone())
        .collect()
}

/// `covers_root` returns whether a knowledge root is one of `hints` or
/// lies under one.
pub(super) fn covers_root(hints: &BTreeSet<RepoPath>, roots: &[RepoPath]) -> bool {
    let keys = keys(hints);
    roots.iter().any(|r| in_scope(&keys, r.as_str()))
}

/// `in_scope` returns whether `path` is one of `hints`, which are sorted,
/// or lies under one. It looks up `path` and each directory above it by
/// binary search, without allocating.
fn in_scope<S: AsRef<str>>(hints: &[S], path: &str) -> bool {
    let mut p = path;
    loop {
        if hints.binary_search_by(|h| h.as_ref().cmp(p)).is_ok() {
            return true;
        }
        match p.rfind('/') {
            Some(0) if p.len() > 1 => p = "/",
            Some(i) if i > 0 => p = &p[..i],
            _ => return false,
        }
    }
}

/// `keys` returns the hinted paths as strings, in the order of `hints`.
fn keys(hints: &BTreeSet<RepoPath>) -> Vec<&str> {
    hints.iter().map(RepoPath::as_str).collect()
}

/// `drop_scopes` removes from `files`, which is sorted by path, each file
/// at or under one of `hints`. It finds each hint's files by binary search,
/// because a run with a hint holds the rows of every file in the store.
fn drop_scopes(files: &mut Vec<WalkedFile>, hints: &BTreeSet<RepoPath>) {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for h in hints {
        if let Ok(i) = files.binary_search_by(|f| f.path.cmp(h)) {
            ranges.push((i, i + 1));
        }
        let (low, high) = under(h);
        let start = files.partition_point(|f| f.path.as_str() < low.as_str());
        let end = files.partition_point(|f| f.path.as_str() < high.as_str());
        if start < end {
            ranges.push((start, end));
        }
    }
    if ranges.is_empty() {
        return;
    }
    ranges.sort_unstable();
    let (mut index, mut next) = (0, 0);
    files.retain(|_| {
        while next < ranges.len() && ranges[next].1 <= index {
            next += 1;
        }
        let dropped = next < ranges.len() && ranges[next].0 <= index;
        index += 1;
        !dropped
    });
}

/// `under` returns the bounds of the paths that lie strictly under `path`
/// in string order: each such path is at least the first bound and less
/// than the second.
pub(super) fn under(path: &RepoPath) -> (String, String) {
    let low = if path.as_str() == "/" {
        "/".to_string()
    } else {
        format!("{path}/")
    };
    // `0` is the character after `/`, so the second bound follows every
    // path that begins with the first.
    let high = format!("{}0", &low[..low.len() - 1]);
    (low, high)
}

/// `is_walked_dir` returns whether the walk of the root `owner` reaches
/// the directory `dir`: `dir` and each directory between it and `owner`
/// are directories, not links.
fn is_walked_dir(root: &RepoRoot, owner: &RepoPath, dir: &RepoPath) -> bool {
    std::iter::successors(Some(dir.clone()), RepoPath::parent)
        .take_while(|d| d != owner && d.starts_with(owner))
        .all(|d| fs::symlink_metadata(d.to_fs(root)).is_ok_and(|m| m.is_dir()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    #[test]
    fn scopes_drop_a_path_under_another() {
        let got = scopes(&[p("/a/b.md"), p("/a-b"), p("/a"), p("/a"), p("/c")]);
        assert_eq!(got, [p("/a"), p("/a-b"), p("/c")].into());
        assert!(in_scope(&keys(&got), "/a/b/c"));
        assert!(!in_scope(&keys(&got), "/a-c"));
        assert!(in_scope(&["/"], "/a/b"));
        assert!(covers_root(&got, &[p("/a/k")]));
        assert!(!covers_root(&got, &[p("/k")]));
    }

    #[test]
    fn drop_scopes_removes_each_hinted_path_and_the_paths_under_it() {
        let file = |s: &str| WalkedFile {
            path: p(s),
            root: None,
            kind: crate::FileKind::Candidate,
            stat: Stat::default(),
        };
        let mut files: Vec<WalkedFile> =
            ["/a", "/a-b", "/a-b/c", "/a/b", "/a/c/d", "/a0", "/b", "/c"]
                .into_iter()
                .map(file)
                .collect();
        drop_scopes(&mut files, &[p("/a"), p("/c")].into());
        let left: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(left, ["/a-b", "/a-b/c", "/a0", "/b"]);
    }

    #[test]
    fn under_bounds_the_paths_below_a_directory() {
        let (low, high) = under(&p("/a"));
        for inside in ["/a/b", "/a/b/c", "/a/~"] {
            assert!(low.as_str() <= inside && inside < high.as_str(), "{inside}");
        }
        for outside in ["/a", "/a-b", "/a0", "/ab", "/b"] {
            assert!(
                !(low.as_str() <= outside && outside < high.as_str()),
                "{outside}"
            );
        }
        assert_eq!(under(&p("/")), ("/".to_string(), "0".to_string()));
    }
}
