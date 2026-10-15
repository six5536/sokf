//! This module holds the tree of files and directories that a run reads.

use std::{cell::RefCell, collections::HashMap};

use crate::{RepoPath, RepoRoot, Stat};

/// A `FileKind` is the kind of a walked file, as the store structure of
/// sokf-format-spec defines it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// `manifest.sokf.yaml` at a knowledge root.
    Manifest,
    /// An `index.md` inside a knowledge root.
    Index,
    /// A `.md` file in `schemas/` at a knowledge root, except `index.md`.
    Schema,
    /// Any other `.md` file inside a knowledge root.
    Document,
    /// A file that is none of the preceding kinds: a candidate additional file.
    Candidate,
}

/// A `WalkedFile` is one file that the walk found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedFile {
    /// The path of the file.
    pub path: RepoPath,
    /// The index of the knowledge root that the file lies under, if any.
    pub root: Option<usize>,
    /// The kind of the file.
    pub kind: FileKind,
    /// The length and modification time of the file, from the walk's stat.
    pub stat: Stat,
}

/// A `WalkedDir` is one directory that the walk visits under a knowledge
/// root. Each such directory must have an index (SOKF_FORMAT_96).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedDir {
    /// The path of the directory.
    pub path: RepoPath,
    /// The index of the innermost knowledge root that the directory lies under.
    pub root: usize,
}

/// A `Tree` is the walked repository.
#[derive(Debug)]
pub struct Tree {
    /// The repository root.
    pub root: RepoRoot,
    /// The knowledge roots, in configuration order.
    pub roots: Vec<RepoPath>,
    /// Every walked file, ordered by path.
    pub files: Vec<WalkedFile>,
    /// Every directory under a root, including the roots, ordered by path.
    pub dirs: Vec<WalkedDir>,
    exists: RefCell<HashMap<RepoPath, bool>>,
}

impl Tree {
    /// `new` builds a tree from its parts, and sorts the files and the
    /// directories by path.
    pub fn new(
        root: RepoRoot,
        roots: Vec<RepoPath>,
        mut files: Vec<WalkedFile>,
        mut dirs: Vec<WalkedDir>,
    ) -> Self {
        files.sort_by(|a, b| a.path.cmp(&b.path));
        dirs.sort_by(|a, b| a.path.cmp(&b.path));
        Tree {
            root,
            roots,
            files,
            dirs,
            exists: RefCell::new(HashMap::new()),
        }
    }

    /// `exists` returns whether `path` exists on disk, and caches the
    /// answer for the rest of the run. It asks the filesystem, not the
    /// walked files, so that a link into an excluded tree still resolves.
    // sokf-implements: CONF-3_AC-3
    // sokf-implements: CONF-3_AC-4
    pub fn exists(&self, path: &RepoPath) -> bool {
        if let Some(&known) = self.exists.borrow().get(path) {
            return known;
        }
        let known = path.to_fs(&self.root).exists();
        self.exists.borrow_mut().insert(path.clone(), known);
        known
    }

    /// `file` returns the walked file at `path`, if any, by a binary search
    /// over the sorted files.
    pub fn file(&self, path: &RepoPath) -> Option<&WalkedFile> {
        self.files
            .binary_search_by(|f| f.path.cmp(path))
            .ok()
            .map(|i| &self.files[i])
    }

    /// `classify` returns the index of the innermost root that `path` lies
    /// under, if any, and the kind of the file, as the store structure of
    /// sokf-format-spec defines it.
    pub fn classify(path: &RepoPath, roots: &[RepoPath]) -> (Option<usize>, FileKind) {
        let Some(root_index) = innermost(roots, path) else {
            return (None, FileKind::Candidate);
        };
        let root = &roots[root_index];
        let name = path.file_name();
        let parent = path.parent();
        let kind = if name == "manifest.sokf.yaml" && parent.as_ref() == Some(root) {
            FileKind::Manifest
        } else if !name.ends_with(".md") {
            FileKind::Candidate
        } else if name == "index.md" {
            FileKind::Index
        } else if parent == root.join("schemas") {
            FileKind::Schema
        } else {
            FileKind::Document
        };
        (Some(root_index), kind)
    }
}

/// `innermost` returns the index of the innermost root that contains
/// `path`. It picks the longest such root, so that a root inside another
/// root owns the files under it.
pub fn innermost<'a>(
    roots: impl IntoIterator<Item = &'a RepoPath>,
    path: &RepoPath,
) -> Option<usize> {
    roots
        .into_iter()
        .enumerate()
        .filter(|(_, r)| path.starts_with(r))
        .max_by_key(|(_, r)| r.as_str().len())
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    #[test]
    fn the_innermost_root_owns_a_nested_path() {
        let roots = vec![p("/knowledge"), p("/knowledge/inner"), p("/docs")];
        assert_eq!(innermost(&roots, &p("/knowledge/a.md")), Some(0));
        assert_eq!(innermost(&roots, &p("/knowledge/inner/i.md")), Some(1));
        assert_eq!(innermost(&roots, &p("/knowledge/inner")), Some(1));
        assert_eq!(innermost(&roots, &p("/x.md")), None);
        assert_eq!(
            Tree::classify(&p("/knowledge/inner/manifest.sokf.yaml"), &roots),
            (Some(1), FileKind::Manifest)
        );
    }

    #[test]
    fn classifies_per_spec_2() {
        let roots = vec![p("/knowledge"), p("/docs")];
        let cases = [
            ("/knowledge/manifest.sokf.yaml", Some(0), FileKind::Manifest),
            (
                "/knowledge/sub/manifest.sokf.yaml",
                Some(0),
                FileKind::Candidate,
            ),
            ("/knowledge/index.md", Some(0), FileKind::Index),
            ("/knowledge/schemas/index.md", Some(0), FileKind::Index),
            ("/knowledge/schemas/issue.md", Some(0), FileKind::Schema),
            ("/knowledge/schemas/sub/x.md", Some(0), FileKind::Document),
            ("/knowledge/a.md", Some(0), FileKind::Document),
            ("/knowledge/a.txt", Some(0), FileKind::Candidate),
            ("/docs/a.md", Some(1), FileKind::Document),
            ("/README.md", None, FileKind::Candidate),
            ("/knowledgebase/a.md", None, FileKind::Candidate),
        ];
        for (path, root, kind) in cases {
            assert_eq!(Tree::classify(&p(path), &roots), (root, kind), "{path}");
        }
    }

    #[test]
    fn exists_asks_the_filesystem_once() {
        let dir = crate::test_support::fresh_dir("tree");
        std::fs::write(dir.join("here.md"), "").unwrap();
        let tree = Tree::new(
            RepoRoot::new(dir.clone()),
            vec![],
            vec![WalkedFile {
                path: p("/b.md"),
                root: None,
                kind: FileKind::Candidate,
                stat: Stat::default(),
            }],
            vec![],
        );
        assert!(tree.exists(&p("/here.md")));
        assert!(!tree.exists(&p("/gone.md")));
        std::fs::write(dir.join("gone.md"), "").unwrap();
        // `exists` caches its first answer for the rest of the run.
        assert!(!tree.exists(&p("/gone.md")));
        assert_eq!(
            tree.files
                .iter()
                .filter(|f| f.kind == FileKind::Candidate)
                .count(),
            1
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
