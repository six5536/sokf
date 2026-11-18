//! This module lists the files of a drop-in folder of `/.sokf/`, such as
//! `/.sokf/roots.d/`, `/.sokf/excludes.d/`, and `/.sokf/formats.d/`, at any
//! depth, so that each tool that ships files for sokf keeps them in a folder
//! of its own, such as `/.sokf/formats.d/pakf/@acme/tool/formats.yaml`
//! (CONF-2, CONF-3, CONF-11).

use std::{fs, io::ErrorKind, path::Path};

use crate::{Error, RepoPath, RepoRoot, Result};

/// `d_files` returns each file with the extension `ext` at any depth below
/// the folder `dir`, a path relative to the repository root, such as
/// `.sokf/roots.d`, in the byte order of the paths. It takes a file through
/// a symbolic link too, but descends into no symbolic link to a directory
/// inside the folder, so that a link can't make the walk loop. An absent
/// folder, or a file that stands where the folder would, holds no file. A
/// directory that can't be read is an error, or, when `lenient` is set,
/// holds no file, so that the files of every other directory still count.
// sokf-implements: CONF-2_AC-53
// sokf-implements: CONF-3_AC-13
// sokf-implements: CONF-11_AC-7
pub(crate) fn d_files(
    root: &RepoRoot,
    dir: &str,
    ext: &str,
    lenient: bool,
) -> Result<Vec<RepoPath>> {
    let mut found = Vec::new();
    let mut walk = Walk {
        ext,
        lenient,
        found: &mut found,
    };
    walk.dir(&root.as_path().join(dir), &format!("/{dir}"))?;
    found.sort();
    Ok(found)
}

/// A `Walk` lists the files with the extension `ext` of a drop-in folder
/// into `found`.
struct Walk<'a> {
    ext: &'a str,
    lenient: bool,
    found: &'a mut Vec<RepoPath>,
}

impl Walk<'_> {
    /// `dir` adds each file below the directory `path`, whose path from the
    /// repository root is `rel`.
    fn dir(&mut self, path: &Path, rel: &str) -> Result<()> {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
                return Ok(());
            }
            Err(_) if self.lenient => return Ok(()),
            Err(e) => return Err(Error::io(path, e)),
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) if self.lenient => continue,
                Err(e) => return Err(Error::io(path, e)),
            };
            // A name that isn't UTF-8 has no path from the repository root.
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(child) = RepoPath::parse(&format!("{rel}/{name}")) else {
                continue;
            };
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) if self.lenient => continue,
                Err(e) => return Err(Error::io(entry.path(), e)),
            };
            if kind.is_dir() {
                self.dir(&entry.path(), child.as_str())?;
            } else if Path::new(&name).extension().is_some_and(|e| e == self.ext)
                && (kind.is_file() || kind.is_symlink() && entry.path().is_file())
            {
                self.found.push(child);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: CONF-2_AC-53
    // sokf-tests: CONF-11_AC-7
    #[test]
    fn lists_each_file_at_any_depth_in_the_order_of_the_paths() {
        let dir = crate::test_support::fresh_dir("d-folder");
        let root = RepoRoot::new(dir.clone());
        assert_eq!(d_files(&root, ".sokf/x.d", "toml", false).unwrap(), vec![]);
        for rel in [
            "z.toml",
            "a.toml",
            "a/b.toml",
            "pakf/@six5536/rust/roots.toml",
            "pakf/@six5536/rust/notes.txt",
            ".hidden.toml",
        ] {
            crate::test_support::write(&dir, &format!(".sokf/x.d/{rel}"), "roots = []\n");
        }
        // A directory whose name has the extension is no file.
        fs::create_dir_all(dir.join(".sokf/x.d/sub.toml")).unwrap();
        let got: Vec<String> = d_files(&root, ".sokf/x.d", "toml", false)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            got,
            [
                "/.sokf/x.d/.hidden.toml",
                "/.sokf/x.d/a.toml",
                "/.sokf/x.d/a/b.toml",
                "/.sokf/x.d/pakf/@six5536/rust/roots.toml",
                "/.sokf/x.d/z.toml",
            ]
        );
        // A file that stands where the folder would holds no file.
        crate::test_support::write(&dir, ".sokf/y.d", "x\n");
        assert_eq!(d_files(&root, ".sokf/y.d", "toml", false).unwrap(), vec![]);
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-2_AC-53
    // sokf-tests: CONF-3_AC-13
    // sokf-tests: CONF-11_AC-7
    #[cfg(unix)]
    #[test]
    fn follows_a_link_to_a_file_and_no_link_to_a_directory() {
        let dir = crate::test_support::fresh_dir("d-folder-links");
        let root = RepoRoot::new(dir.clone());
        crate::test_support::write(&dir, "elsewhere/kb/roots.toml", "roots = []\n");
        crate::test_support::write(&dir, ".sokf/x.d/real.toml", "roots = []\n");
        std::os::unix::fs::symlink(
            dir.join("elsewhere/kb/roots.toml"),
            dir.join(".sokf/x.d/linked.toml"),
        )
        .unwrap();
        std::os::unix::fs::symlink(dir.join("elsewhere"), dir.join(".sokf/x.d/dir")).unwrap();
        // A link back up the tree would loop if the walk followed it.
        std::os::unix::fs::symlink(dir.join(".sokf"), dir.join(".sokf/x.d/up")).unwrap();
        let got: Vec<String> = d_files(&root, ".sokf/x.d", "toml", false)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(got, ["/.sokf/x.d/linked.toml", "/.sokf/x.d/real.toml"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_lenient_listing_skips_only_a_directory_that_it_cannot_read() {
        use std::os::unix::fs::PermissionsExt;
        let dir = crate::test_support::fresh_dir("d-folder-lenient");
        let root = RepoRoot::new(dir.clone());
        crate::test_support::write(&dir, ".sokf/x.d/a.toml", "roots = []\n");
        crate::test_support::write(&dir, ".sokf/x.d/shut/b.toml", "roots = []\n");
        let shut = dir.join(".sokf/x.d/shut");
        fs::set_permissions(&shut, fs::Permissions::from_mode(0o000)).unwrap();
        // A process that may read every directory, such as root's, reads it.
        if fs::read_dir(&shut).is_err() {
            assert!(d_files(&root, ".sokf/x.d", "toml", false).is_err());
            let got = d_files(&root, ".sokf/x.d", "toml", true).unwrap();
            assert_eq!(got, vec![RepoPath::parse("/.sokf/x.d/a.toml").unwrap()]);
        }
        fs::set_permissions(&shut, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
