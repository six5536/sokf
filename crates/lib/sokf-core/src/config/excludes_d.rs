//! This module reads the drop-in folder `/.sokf/excludes.d/`, whose files, at
//! any depth, each list further globs beside `repository.exclude` (CONF-3).
//! A tool that keeps files in the repository, such as the copies of a pack,
//! adds the paths that sokf skips in a file of its own there, and never edits
//! `/.sokf/config.toml`.

use std::fs;

use serde::Deserialize;

use crate::{Error, RepoPath, RepoRoot, Result};

/// `EXCLUDES_DIR` is the drop-in folder of the added excludes, relative to
/// the repository root.
pub const EXCLUDES_DIR: &str = ".sokf/excludes.d";

/// `ExcludesFile` is one file of `/.sokf/excludes.d/`: only a list of globs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExcludesFile {
    #[serde(default)]
    exclude: Vec<String>,
}

/// An `AddedExclude` is one glob that a file of `/.sokf/excludes.d/` lists,
/// with the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedExclude {
    /// The file that lists the glob.
    pub file: RepoPath,
    /// The `/`-rooted glob.
    pub glob: String,
}

/// `read_added` reads every `*.toml` file of `/.sokf/excludes.d/`, at any
/// depth, in the byte order of their paths, and returns the globs that they
/// list, in the order of each file. It refuses a file that doesn't parse,
/// that has a key other than `exclude`, or that lists a glob that isn't
/// `/`-rooted or doesn't parse, with an error that names the file. An absent
/// folder adds no glob.
// sokf-implements: CONF-3_AC-9
// sokf-implements: CONF-3_AC-10
pub(crate) fn read_added(root: &RepoRoot) -> Result<Vec<AddedExclude>> {
    let mut found = Vec::new();
    for file in excludes_files(root)? {
        let text =
            fs::read_to_string(file.to_fs(root)).map_err(|e| Error::io(file.to_fs(root), e))?;
        let parsed: ExcludesFile = toml::from_str(&text)
            .map_err(|e| Error::config(file.clone(), e.message().to_string()))?;
        for glob in parsed.exclude {
            // sokf-implements: CONF-3_AC-11
            if let Some(m) = super::file::glob_fault("exclude", &glob) {
                return Err(Error::config(file, m));
            }
            found.push(AddedExclude {
                file: file.clone(),
                glob,
            });
        }
    }
    Ok(found)
}

/// `excludes_files` returns the files of `/.sokf/excludes.d/` that list
/// globs: each `*.toml` at any depth that is a file, through a symbolic link
/// too, in the byte order of their paths. An absent folder holds none.
fn excludes_files(root: &RepoRoot) -> Result<Vec<RepoPath>> {
    super::d_folder::d_files(root, EXCLUDES_DIR, "toml", false)
}

/// `excludes_files_lenient` returns the files of `/.sokf/excludes.d/` as
/// `excludes_files` does, and skips each directory that can't be read, so
/// that the files of every other directory still count.
pub fn excludes_files_lenient(root: &RepoRoot) -> Vec<RepoPath> {
    super::d_folder::d_files(root, EXCLUDES_DIR, "toml", true).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: CONF-3_AC-9
    // sokf-tests: CONF-3_AC-10
    // sokf-tests: CONF-3_AC-11
    #[test]
    fn reads_each_file_in_path_order_and_refuses_a_bad_one() {
        let dir = crate::test_support::fresh_dir("excludes-d");
        let root = RepoRoot::new(dir.clone());
        assert_eq!(read_added(&root).unwrap(), vec![]);
        crate::test_support::write(
            &dir,
            ".sokf/excludes.d/b.toml",
            "exclude = [\"/b1\", \"/b2/**\"]\n",
        );
        crate::test_support::write(&dir, ".sokf/excludes.d/a.toml", "exclude = [\"/a/**\"]\n");
        crate::test_support::write(
            &dir,
            ".sokf/excludes.d/a/deep.toml",
            "exclude = [\"/deep/*.rs\"]\n",
        );
        crate::test_support::write(&dir, ".sokf/excludes.d/empty.toml", "");
        crate::test_support::write(&dir, ".sokf/excludes.d/notes.txt", "exclude = 1\n");
        let globs: Vec<(String, String)> = read_added(&root)
            .unwrap()
            .into_iter()
            .map(|e| (e.file.to_string(), e.glob))
            .collect();
        assert_eq!(
            globs,
            vec![
                ("/.sokf/excludes.d/a.toml".into(), "/a/**".into()),
                ("/.sokf/excludes.d/a/deep.toml".into(), "/deep/*.rs".into()),
                ("/.sokf/excludes.d/b.toml".into(), "/b1".into()),
                ("/.sokf/excludes.d/b.toml".into(), "/b2/**".into()),
            ]
        );
        for (bad, says) in [
            ("exclude = [", "unclosed array"),
            ("exclude = []\nroots = []\n", "unknown field `roots`"),
            ("exclude = \"/x\"\n", "invalid type: string"),
            (
                "exclude = [\"x/**\"]\n",
                "exclude glob `x/**` is not /-rooted",
            ),
            ("exclude = [\"/[\"]\n", "exclude glob `/[` does not parse"),
        ] {
            crate::test_support::write(&dir, ".sokf/excludes.d/c.toml", bad);
            let e = read_added(&root).unwrap_err();
            assert!(matches!(e, Error::Config { .. }), "{bad:?} -> {e}");
            let e = e.to_string();
            assert!(
                e.starts_with("/.sokf/excludes.d/c.toml: "),
                "{bad:?} -> {e}"
            );
            assert!(e.contains(says), "{bad:?} -> {e}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
