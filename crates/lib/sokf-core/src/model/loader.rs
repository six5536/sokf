//! The `loader` module reads files and parses them for the model.

use super::{Document, IndexFile, ParseMode, Parsed, parse_source};
use crate::{RepoPath, RepoRoot, Result, Source, source};

// sokf:begin loader
/// A `Loader` reads files and parses them. An implementation can put a cache
/// behind it.
pub trait Loader {
    /// `load` reads one file.
    fn load(&mut self, path: &RepoPath) -> Result<Source>;

    /// `parse` parses one source.
    fn parse(&mut self, source: &Source, mode: ParseMode) -> Parsed {
        parse_source(source, mode)
    }

    /// `document` reads and parses one document. When a cache is behind the
    /// loader, `document` returns `None` for an unchanged document whose
    /// results the cache already holds, until a check needs the document
    /// (CACHE-1_AC-20).
    fn document(&mut self, path: &RepoPath) -> Result<Option<Document>> {
        self.full_document(path).map(Some)
    }

    /// `full_document` reads and parses one document, whatever the cache holds.
    fn full_document(&mut self, path: &RepoPath) -> Result<Document> {
        let source = self.load(path)?;
        let parsed = self.parse(&source, ParseMode::Document);
        Ok(Document::new(source, parsed))
    }

    /// `additional` reads the text of one additional file. When a cache is
    /// behind the loader, `additional` returns `None` for an unchanged file
    /// until a check needs the file (CACHE-1_AC-20).
    fn additional(&mut self, path: &RepoPath) -> Result<Option<Source>> {
        self.load(path).map(Some)
    }

    /// `schema` reads and parses one schema document. When a cache is behind
    /// the loader, `schema` returns `None` for an unchanged schema until a
    /// check needs the schema compiled (CACHE-1_AC-25).
    fn schema(&mut self, path: &RepoPath) -> Result<Option<Document>> {
        self.full_document(path).map(Some)
    }

    /// `index` reads one index file. When a cache is behind the loader, `index`
    /// returns `None` for an unchanged index file until the check of the
    /// file's directory needs the file (CACHE-1_AC-19).
    fn index(&mut self, path: &RepoPath) -> Result<Option<IndexFile>> {
        let source = self.load(path)?;
        Ok(Some(IndexFile {
            path: path.clone(),
            source,
        }))
    }
}
// sokf:end loader

/// `FsLoader` is a loader without a cache: it reads every file from disk.
#[derive(Debug, Clone)]
pub struct FsLoader<'a> {
    root: &'a RepoRoot,
}

impl<'a> FsLoader<'a> {
    /// `new` makes a loader that reads files under `root`.
    pub fn new(root: &'a RepoRoot) -> Self {
        FsLoader { root }
    }
}

impl Loader for FsLoader<'_> {
    fn load(&mut self, path: &RepoPath) -> Result<Source> {
        source::load(self.root, path)
    }
}
