//! This module holds the helpers that the unit tests of several modules
//! share: a fresh directory under the temp directory, a file written into
//! it, and a model built from it.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{Config, FsLoader, Model, RepoPath, RepoRoot, build, collect};

/// `fresh_dir` returns an empty directory under the temp directory, named
/// by `tag`, the process id, and a count of the calls, because tests run in
/// parallel threads of one process, and two of them can pass one tag.
pub(crate) fn fresh_dir(tag: &str) -> PathBuf {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sokf-{tag}-{}-{call}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `write` writes `text` to the file `rel` under `dir`, and creates the
/// directories above it.
pub(crate) fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// `requirement_formats` returns the registry of the fixture's formats of
/// the lines of requirements documents, as an embedded layer, because the
/// tool embeds no format of its own.
pub(crate) fn requirement_formats(
    grammar: &crate::grammar::Grammar,
) -> crate::grammar::FormatRegistry {
    crate::grammar::FormatRegistry::from_text(
        include_str!("../tests/fixtures/formats/requirements.formats.yaml"),
        &crate::grammar::SOKF_TYPES,
        grammar,
        &crate::grammar::RegexCache::new(),
    )
    .unwrap()
}

/// `model_at` builds the model of the repository at `dir`, with `roots` as
/// its knowledge roots.
pub(crate) fn model_at(dir: &Path, roots: &[&str]) -> Model {
    let root = RepoRoot::new(dir.to_path_buf());
    let config = Config {
        roots: roots.iter().map(|r| RepoPath::parse(r).unwrap()).collect(),
        ..Config::default()
    };
    let tree = collect(&root, &config).unwrap();
    build(tree, &mut FsLoader::new(&root)).unwrap()
}
