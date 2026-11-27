//! This module holds `TempDir`, a directory of its own under the system's
//! temporary directory, which the diff of `harness diff` writes its texts
//! into.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A `TempDir` is a new directory under the system's temporary directory,
/// which it removes when it drops.
pub(crate) struct TempDir {
    /// The directory.
    path: PathBuf,
}

impl TempDir {
    /// `new` creates a directory whose name holds `tag` and that no other
    /// run uses.
    pub(crate) fn new(tag: &str) -> std::io::Result<TempDir> {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        loop {
            let n = COUNT.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("sokf-{tag}-{}-{nanos}-{n}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(TempDir { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
    }

    /// `path` returns the directory.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
