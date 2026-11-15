//! This module reads the drop-in folder `/.sokf/roots.d/`, whose files, at
//! any depth, each list further knowledge roots beside `knowledge.roots`
//! (CONF-2). A tool that delivers a store registers it in a file of its own
//! there, and never edits `/.sokf/config.toml`.

use std::fs;

use serde::Deserialize;

use crate::{Error, RepoPath, RepoRoot, Result};

/// `ROOTS_DIR` is the drop-in folder of the registered roots, relative to
/// the repository root.
pub const ROOTS_DIR: &str = ".sokf/roots.d";

/// `RootsFile` is one file of `/.sokf/roots.d/`: only a list of roots.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootsFile {
    #[serde(default)]
    roots: Vec<String>,
}

/// A `Registered` is one root that a file of `/.sokf/roots.d/` lists, with
/// the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    /// The file that lists the root.
    pub file: RepoPath,
    /// The root.
    pub root: RepoPath,
}

/// `read_registered` reads every `*.toml` file of `/.sokf/roots.d/`, at any
/// depth, in the byte order of their paths, and returns the roots that they list, in the
/// order of each file. It refuses, through `fail`, a file that doesn't
/// parse, that has a key other than `roots`, or that lists a root that
/// isn't `/`-rooted. An absent folder registers no root.
// sokf-implements: CONF-2_AC-49
// sokf-implements: CONF-2_AC-51
pub(crate) fn read_registered(
    root: &RepoRoot,
    fail: &dyn Fn(RepoPath, String) -> Error,
) -> Result<Vec<Registered>> {
    let mut found = Vec::new();
    for file in registry_files(root)? {
        found.extend(read_file(root, &file, fail)?);
    }
    Ok(found)
}

/// `read_file` returns the roots that the file `file` of `/.sokf/roots.d/`
/// lists, and refuses, through `fail`, a file that doesn't parse, that has
/// a key other than `roots`, or that lists a root that isn't `/`-rooted.
fn read_file(
    root: &RepoRoot,
    file: &RepoPath,
    fail: &dyn Fn(RepoPath, String) -> Error,
) -> Result<Vec<Registered>> {
    let text = fs::read_to_string(file.to_fs(root)).map_err(|e| Error::io(file.to_fs(root), e))?;
    let parsed: RootsFile =
        toml::from_str(&text).map_err(|e| fail(file.clone(), e.message().to_string()))?;
    parsed
        .roots
        .into_iter()
        .map(|r| {
            // sokf-implements: CONF-2_AC-52
            let root = RepoPath::parse(&r).ok_or_else(|| {
                fail(
                    file.clone(),
                    format!("roots entry `{r}` is not a /-rooted path"),
                )
            })?;
            Ok(Registered {
                file: file.clone(),
                root,
            })
        })
        .collect()
}

/// `registered_lenient` returns each root that a file of `/.sokf/roots.d/`
/// lists. A file that doesn't read or parse registers nothing here, and
/// leaves the roots of every other file as they are.
pub fn registered_lenient(root: &RepoRoot) -> Vec<RepoPath> {
    registry_files_lenient(root)
        .iter()
        .flat_map(|file| read_file(root, file, &|p, m| Error::config(p, m)).unwrap_or_default())
        .map(|r| r.root)
        .collect()
}

/// `registry_files` returns the files of `/.sokf/roots.d/` that list
/// roots: each `*.toml` at any depth that is a file, through a symbolic link
/// too, in the byte order of their paths. An absent folder holds none.
// sokf-implements: CONF-2_AC-48
pub fn registry_files(root: &RepoRoot) -> Result<Vec<RepoPath>> {
    super::d_folder::d_files(root, ROOTS_DIR, "toml", false)
}

/// `registry_files_lenient` returns the files of `/.sokf/roots.d/` as
/// `registry_files` does, and skips each directory that can't be read, so
/// that the files of every other directory still count.
pub fn registry_files_lenient(root: &RepoRoot) -> Vec<RepoPath> {
    super::d_folder::d_files(root, ROOTS_DIR, "toml", true).unwrap_or_default()
}

/// `check_apart` refuses each registered root that equals, lies inside, or
/// holds another knowledge root, so that a registered store lies beside
/// every other store, never inside one.
// sokf-implements: CONF-2_AC-50
pub(crate) fn check_apart(own: &[RepoPath], registered: &[Registered]) -> Result<()> {
    for (i, r) in registered.iter().enumerate() {
        let others = own.iter().chain(
            registered
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, o)| &o.root),
        );
        if let Some(other) = others
            .into_iter()
            .find(|o| r.root.starts_with(o) || o.starts_with(&r.root))
        {
            let how = if *other == r.root {
                "is also the knowledge root"
            } else if r.root.starts_with(other) {
                "lies inside the knowledge root"
            } else {
                "holds the knowledge root"
            };
            return Err(Error::config(
                r.file.clone(),
                format!(
                    "the registered root `{}` {how} `{other}`; a registered store lies beside every other store",
                    r.root
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn fail(path: RepoPath, m: String) -> Error {
        Error::config(path, m)
    }

    // sokf-tests: CONF-2_AC-48
    // sokf-tests: CONF-2_AC-49
    // sokf-tests: CONF-2_AC-51
    // sokf-tests: CONF-2_AC-52
    #[test]
    fn reads_each_file_in_name_order_and_refuses_a_bad_one() {
        let dir = crate::test_support::fresh_dir("roots-d");
        let root = RepoRoot::new(dir.clone());
        assert_eq!(read_registered(&root, &fail).unwrap(), vec![]);
        fs::create_dir_all(dir.join(ROOTS_DIR).join("sub.toml")).unwrap();
        fs::write(
            dir.join(ROOTS_DIR).join("b.toml"),
            "roots = [\"/b1\", \"/b2\"]\n",
        )
        .unwrap();
        fs::write(dir.join(ROOTS_DIR).join("a.toml"), "roots = [\"/a\"]\n").unwrap();
        crate::test_support::write(&dir, ".sokf/roots.d/a/deep.toml", "roots = [\"/deep\"]\n");
        fs::write(dir.join(ROOTS_DIR).join("notes.txt"), "roots = 1\n").unwrap();
        let roots: Vec<(String, String)> = read_registered(&root, &fail)
            .unwrap()
            .into_iter()
            .map(|r| (r.file.to_string(), r.root.to_string()))
            .collect();
        assert_eq!(
            roots,
            vec![
                ("/.sokf/roots.d/a.toml".into(), "/a".into()),
                ("/.sokf/roots.d/a/deep.toml".into(), "/deep".into()),
                ("/.sokf/roots.d/b.toml".into(), "/b1".into()),
                ("/.sokf/roots.d/b.toml".into(), "/b2".into()),
            ]
        );
        for bad in [
            "roots = [",
            "roots = []\nother = 1\n",
            "roots = [\"rel\"]\n",
        ] {
            fs::write(dir.join(ROOTS_DIR).join("c.toml"), bad).unwrap();
            let e = read_registered(&root, &fail).unwrap_err();
            assert!(
                e.to_string().contains("/.sokf/roots.d/c.toml"),
                "{bad:?} -> {e}"
            );
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_lenient_read_skips_only_the_file_that_breaks() {
        let dir = crate::test_support::fresh_dir("roots-d-lenient");
        let root = RepoRoot::new(dir.clone());
        fs::create_dir_all(dir.join(ROOTS_DIR)).unwrap();
        fs::write(dir.join(ROOTS_DIR).join("a.toml"), "<<<<<<< ours\n").unwrap();
        fs::write(dir.join(ROOTS_DIR).join("b.toml"), "roots = [\"/b\"]\n").unwrap();
        fs::write(dir.join(ROOTS_DIR).join("c.toml"), "roots = [\"/c\"]\n").unwrap();
        assert_eq!(registered_lenient(&root), vec![p("/b"), p("/c")]);
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-2_AC-50
    #[test]
    fn a_registered_root_lies_beside_every_other_root() {
        let reg = |r: &str| Registered {
            file: p("/.sokf/roots.d/packs.toml"),
            root: p(r),
        };
        let own = [p("/knowledge"), p("/docs/inner")];
        assert!(check_apart(&own, &[reg("/packs/a"), reg("/packs/b")]).is_ok());
        for (bad, says) in [
            (
                vec![reg("/knowledge")],
                "is also the knowledge root `/knowledge`",
            ),
            (
                vec![reg("/knowledge/x")],
                "lies inside the knowledge root `/knowledge`",
            ),
            (vec![reg("/docs")], "holds the knowledge root `/docs/inner`"),
            (
                vec![reg("/packs"), reg("/packs/b")],
                "holds the knowledge root `/packs/b`",
            ),
        ] {
            let e = check_apart(&own, &bad).unwrap_err().to_string();
            assert!(e.contains(says), "{e}");
            assert!(e.contains("/.sokf/roots.d/packs.toml"), "{e}");
        }
    }
}
