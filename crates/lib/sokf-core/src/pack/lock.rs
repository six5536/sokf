//! This module reads and writes `/.sokf/packs.lock`, the record of each
//! schema pack that `sokf pack sync` copied into the repository (CONF-8).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Error, RepoPath, RepoRoot, Result, source::read_optional};

/// `HEADER` is the comment at the top of the lock.
const HEADER: &str = "# sokf pack sync writes this file. Do not edit it.\n";

/// `lock_path` returns the path of the lock.
pub fn lock_path() -> RepoPath {
    RepoPath::parse("/.sokf/packs.lock").expect("literal")
}

/// `PACKS_DIR` is the directory, relative to the repository root, that holds
/// a directory for each pack: its formats file and its base copies.
pub const PACKS_DIR: &str = ".sokf/packs";

/// `pack_dir` returns the directory of the pack `name` under `/.sokf/packs/`.
pub fn pack_dir(name: &str) -> RepoPath {
    RepoPath::parse(&format!("/{PACKS_DIR}/{name}")).expect("a pack name is a slug")
}

/// `base_path` returns the path of the base copy of the file `rel` of the
/// pack `name`.
pub fn base_path(name: &str, rel: &str) -> RepoPath {
    RepoPath::parse(&format!("/{PACKS_DIR}/{name}/base/{rel}")).expect("a pack path is relative")
}

/// `formats_of` returns the path of the formats file of the pack `name`.
pub fn formats_of(name: &str) -> RepoPath {
    RepoPath::parse(&format!("/{PACKS_DIR}/{name}/formats.yaml")).expect("a pack name is a slug")
}

/// `instructions_of` returns the path of the copy of the instructions
/// fragment of the pack `name`, which `sokf harness install` reads.
pub fn instructions_of(name: &str) -> RepoPath {
    RepoPath::parse(&format!("/{PACKS_DIR}/{name}/instructions.md")).expect("a pack name is a slug")
}

/// `fragments` returns the instructions fragment of each pack in `lock` that
/// has one, by the pack's name, in the order of the lock. It reads only the
/// copies under `/.sokf/packs/`, so it needs no network.
// sokf-implements: CLI-3_AC-74
pub fn fragments(root: &RepoRoot, lock: &Lock) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for name in lock.packs.keys() {
        if let Some(text) = read_optional(&instructions_of(name).to_fs(root))? {
            out.push((name.clone(), text));
        }
    }
    Ok(out)
}

/// A `Lock` is the content of `/.sokf/packs.lock`: one entry for each pack,
/// by name.
// sokf-implements: CONF-8_AC-1
// sokf:begin lock
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    /// The packs, by name.
    #[serde(default)]
    pub packs: BTreeMap<String, LockedPack>,
}

/// A `LockedPack` is the record of one pack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPack {
    /// The source: a git URL or path, or `builtin:<NAME>`.
    pub source: String,
    /// The branch, tag, or commit of a git source.
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// The pack's directory in the source, when it isn't the source's root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The knowledge root that holds the pack's schemas and documents.
    pub store: String,
    /// The resolved commit, or the tool's version for an embedded pack.
    pub commit: String,
    /// The version that the pack's manifest gives.
    pub version: String,
    /// The pack's files, by their paths in the pack.
    #[serde(default)]
    pub files: BTreeMap<String, LockedFile>,
}

/// A `LockedFile` is the record of one file of a pack.
// sokf-implements: CONF-8_AC-2
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedFile {
    /// The file's path in the repository.
    pub path: String,
    /// The hash of the file's base: the pack's version of the file at the
    /// last sync, as `fnv1a64:<HEX>`.
    pub base: String,
}
// sokf:end lock

impl Lock {
    /// `owner_of` returns the pack whose lock entry records the destination
    /// `dest`, and the file's path in that pack.
    pub fn owner_of(&self, dest: &str) -> Option<(&str, &str)> {
        self.packs.iter().find_map(|(name, pack)| {
            pack.files
                .iter()
                .find(|(_, f)| f.path == dest)
                .map(|(rel, _)| (name.as_str(), rel.as_str()))
        })
    }
}

/// `hash_of` returns the hash of `bytes` as the lock records it.
pub fn hash_of(bytes: &[u8]) -> String {
    format!("fnv1a64:{:016x}", crate::source::fnv1a64(bytes))
}

/// `read_lock` reads the lock, and returns an empty lock when the file is
/// absent. It refuses a lock that doesn't parse.
// sokf-implements: CONF-8_AC-3
pub fn read_lock(root: &RepoRoot) -> Result<Lock> {
    let path = lock_path();
    let Some(text) = read_optional(&path.to_fs(root))? else {
        return Ok(Lock::default());
    };
    let lock: Lock = toml::from_str(&text).map_err(|e| Error::config(path.clone(), e.message()))?;
    for name in lock.packs.keys() {
        if !super::manifest::is_slug(name) {
            return Err(Error::config(
                path,
                format!("the pack name `{name}` is not a slug"),
            ));
        }
    }
    Ok(lock)
}

/// `lock_text` returns the text of the lock, with its header.
pub fn lock_text(lock: &Lock) -> Result<String> {
    let body = toml::to_string(lock).map_err(|e| Error::Internal(e.to_string()))?;
    Ok(format!("{HEADER}{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_round_trips_through_its_text() {
        let mut lock = Lock::default();
        let mut pack = LockedPack {
            source: "https://example.com/method.git".into(),
            git_ref: Some("main".into()),
            path: None,
            store: "/knowledge".into(),
            commit: "0123abcd".into(),
            version: "1.0.0".into(),
            files: BTreeMap::new(),
        };
        pack.files.insert(
            "schemas/issue.md".into(),
            LockedFile {
                path: "/knowledge/schemas/issue.md".into(),
                base: hash_of(b"x"),
            },
        );
        lock.packs.insert("method".into(), pack);
        let text = lock_text(&lock).unwrap();
        assert!(text.starts_with(HEADER), "{text}");
        assert!(text.contains("[packs.method]"), "{text}");
        assert!(text.contains("ref = \"main\""), "{text}");
        assert!(!text.contains("path = \"\""), "{text}");
        let back: Lock = toml::from_str(&text).unwrap();
        assert_eq!(back, lock);
        assert_eq!(
            lock.owner_of("/knowledge/schemas/issue.md"),
            Some(("method", "schemas/issue.md"))
        );
        assert_eq!(lock.owner_of("/knowledge/schemas/other.md"), None);
    }

    #[test]
    fn the_paths_of_a_pack_lie_under_its_directory() {
        assert_eq!(pack_dir("sokf").as_str(), "/.sokf/packs/sokf");
        assert_eq!(
            base_path("sokf", "schemas/issue.md").as_str(),
            "/.sokf/packs/sokf/base/schemas/issue.md"
        );
        assert_eq!(
            formats_of("sokf").as_str(),
            "/.sokf/packs/sokf/formats.yaml"
        );
        assert_eq!(
            instructions_of("sokf").as_str(),
            "/.sokf/packs/sokf/instructions.md"
        );
        assert_eq!(hash_of(b""), "fnv1a64:cbf29ce484222325");
    }
}
