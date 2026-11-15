//! This module holds the helpers that the unit tests of several modules
//! share: a fresh directory under the temp directory, a file written into
//! it, and a model built from it.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{Config, FsLoader, Model, RepoPath, RepoRoot, build, collect};

/// `fresh_dir` returns an empty directory under the temp directory, named
/// by `tag` and the process id. Each test passes its own tag, because
/// tests run in parallel.
pub(crate) fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sokf-{tag}-{}", std::process::id()));
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
