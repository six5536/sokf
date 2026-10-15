//! This module creates the store's directories and files for `sokf init`,
//! and then applies the config change.

use std::{fs, io::Write as _, path::Path};

use super::{ConfigChange, StoreFiles};
use crate::{Error, RepoPath, RepoRoot, Result, config::config_path};

/// `write` writes every store file, with LF line endings, and then applies the
/// config change. It creates each new file only if nothing exists at its path
/// yet, so if a file has appeared there since `store` and `register` ran,
/// `write` returns an I/O error. It returns the paths it created and the paths it
/// updated.
// sokf-implements: CLI-2_AC-6
// sokf-implements: CLI-2_AC-9
// sokf-implements: CLI-2_AC-11
// sokf-implements: CLI-12_AC-1
pub fn write(
    root: &RepoRoot,
    files: &StoreFiles,
    change: &ConfigChange,
) -> Result<(Vec<RepoPath>, Vec<RepoPath>)> {
    let mut created = Vec::new();
    let mut updated = Vec::new();
    for (path, text) in &files.files {
        create_new(&path.to_fs(root), text)?;
        created.push(path.clone());
    }
    let config = config_path();
    match change {
        ConfigChange::Unchanged => {}
        ConfigChange::Create(text) => {
            create_new(&config.to_fs(root), text)?;
            created.push(config);
        }
        ConfigChange::Update(text) => {
            let fs_path = config.to_fs(root);
            fs::write(&fs_path, text).map_err(|e| Error::io(&fs_path, e))?;
            updated.push(config);
        }
    }
    created.sort();
    Ok((created, updated))
}

/// `create_new` creates the parent directories and then the file, which must
/// not exist.
pub fn create_new(fs_path: &Path, text: &str) -> Result<()> {
    crate::source::create_parent(fs_path)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fs_path)
        .map_err(|e| Error::io(fs_path, e))?;
    file.write_all(text.as_bytes())
        .map_err(|e| Error::io(fs_path, e))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        crate::test_support::fresh_dir(&format!("write-{name}"))
    }

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    #[test]
    fn creates_files_and_directories_then_the_config() {
        let dir = temp("create");
        let root = RepoRoot::new(dir.clone());
        let files = StoreFiles {
            files: vec![
                (p("/docs/kb/manifest.sokf.yaml"), "m\n".into()),
                (p("/docs/kb/schemas/index.md"), "# schemas\n".into()),
            ],
        };
        let (created, updated) = write(
            &root,
            &files,
            &ConfigChange::Create("[knowledge]\nroots = [\"/docs/kb\"]\n".into()),
        )
        .unwrap();
        assert_eq!(
            created,
            vec![
                p("/.sokf/config.toml"),
                p("/docs/kb/manifest.sokf.yaml"),
                p("/docs/kb/schemas/index.md")
            ]
        );
        assert!(updated.is_empty());
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/config.toml")).unwrap(),
            "[knowledge]\nroots = [\"/docs/kb\"]\n"
        );
        // A file that exists is never rewritten.
        assert!(matches!(
            write(&root, &files, &ConfigChange::Unchanged),
            Err(Error::Io { .. })
        ));
        // An update rewrites the config only.
        let (created, updated) = write(
            &root,
            &StoreFiles { files: vec![] },
            &ConfigChange::Update("[knowledge]\nroots = []\n".into()),
        )
        .unwrap();
        assert!(created.is_empty());
        assert_eq!(updated, vec![p("/.sokf/config.toml")]);
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/config.toml")).unwrap(),
            "[knowledge]\nroots = []\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
