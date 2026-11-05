//! This module reads a pack from its source: a directory of a git clone, or
//! a pack that the binary embeds (D-14, D-23, and D-24 of design-pack).

use std::{collections::BTreeMap, fs, path::Path};

use super::manifest::{MANIFEST, PackManifest, parse_manifest};
use crate::{Error, Result};

/// `BUILTIN` is the prefix of the source of an embedded pack.
pub const BUILTIN: &str = "builtin:";

/// A `Source` is where a pack comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An embedded pack, by name.
    Builtin(String),
    /// A git repository, by the URL or path that `git clone` reads.
    Git(String),
}

impl Source {
    /// `parse` reads a source. `builtin:<NAME>` and a source with neither `:`
    /// nor `/` name an embedded pack, and any other source is a git
    /// repository.
    // sokf-implements: CLI-13_AC-19
    pub fn parse(s: &str) -> Source {
        if let Some(name) = s.strip_prefix(BUILTIN) {
            Source::Builtin(name.to_string())
        } else if !s.contains(':') && !s.contains('/') {
            Source::Builtin(s.to_string())
        } else {
            Source::Git(s.to_string())
        }
    }

    /// `label` returns the source as the lock records it.
    pub fn label(&self) -> String {
        match self {
            Source::Builtin(name) => format!("{BUILTIN}{name}"),
            Source::Git(url) => url.clone(),
        }
    }
}

/// A `PackFiles` is a pack as sokf reads it: its manifest and each other
/// file, by its path in the pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFiles {
    /// The manifest.
    pub manifest: PackManifest,
    /// Each file of the pack other than the manifest, by its path in the
    /// pack, with `/` between the segments.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// An `EmbeddedPack` is a pack that the build embeds from `/packs/`.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedPack {
    /// The pack's directory under `/packs/`.
    pub dir: &'static str,
    /// Each file of the pack, the manifest among them, by its path in the
    /// pack.
    pub files: &'static [(&'static str, &'static str)],
}

/// `EMBEDDED` lists the packs that the build embeds: every directory under
/// `/packs/` that holds `pack.sokf.yaml`.
// sokf-implements: CLI-13_AC-18
pub static EMBEDDED: &[EmbeddedPack] = include!(concat!(env!("OUT_DIR"), "/packs.rs"));

impl EmbeddedPack {
    /// `read` returns the pack's manifest and files.
    pub fn read(&self) -> Result<PackFiles> {
        let mut manifest = None;
        let mut files = BTreeMap::new();
        for (rel, text) in self.files {
            if *rel == MANIFEST {
                manifest = Some(parse_manifest(text, &format!("builtin:{}", self.dir))?);
            } else {
                files.insert(rel.to_string(), text.as_bytes().to_vec());
            }
        }
        let manifest = manifest.ok_or_else(|| {
            Error::Internal(format!("the embedded pack {} has no manifest", self.dir))
        })?;
        Ok(PackFiles { manifest, files })
    }
}

/// `embedded_packs` returns the manifest of each embedded pack, in the order
/// of their names.
pub fn embedded_packs(embedded: &[EmbeddedPack]) -> Result<Vec<PackManifest>> {
    let mut out: Vec<PackManifest> = embedded
        .iter()
        .map(|p| p.read().map(|f| f.manifest))
        .collect::<Result<_>>()?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// `find_embedded` returns the embedded pack `name`, and refuses a name that
/// no embedded pack has, with an error that lists the embedded packs.
// sokf-implements: CLI-13_AC-20
pub fn find_embedded(embedded: &[EmbeddedPack], name: &str) -> Result<PackFiles> {
    for pack in embedded {
        let files = pack.read()?;
        if files.manifest.name == name {
            return Ok(files);
        }
    }
    let names: Vec<String> = embedded_packs(embedded)?
        .into_iter()
        .map(|m| m.name)
        .collect();
    let list = if names.is_empty() {
        "the binary embeds no pack".to_string()
    } else {
        format!("the embedded packs are {}", names.join(", "))
    };
    Err(Error::Pack(format!(
        "no embedded pack is named `{name}`: {list}"
    )))
}

/// `read_dir` reads the pack in `dir`: its manifest and every file under it,
/// except the directory `.git`. It refuses a pack that holds a symbolic
/// link, because a link in a cloned repository can point at any file of the
/// machine, and sync would copy that file into the store.
// sokf-implements: SOKF_FORMAT_160
// sokf-implements: CLI-13_AC-41
pub fn read_dir(dir: &Path, label: &str) -> Result<PackFiles> {
    if !dir.is_dir() {
        return Err(Error::Pack(format!("{label} holds no {MANIFEST}")));
    }
    let mut files = BTreeMap::new();
    collect(dir, "", label, &mut files)?;
    let text = files
        .remove(MANIFEST)
        .ok_or_else(|| Error::Pack(format!("{label} holds no {MANIFEST}")))?;
    let text = String::from_utf8(text)
        .map_err(|_| Error::Pack(format!("{label}/{MANIFEST} is not UTF-8")))?;
    let manifest = parse_manifest(&text, &format!("{label}/{MANIFEST}"))?;
    Ok(PackFiles { manifest, files })
}

/// `collect` adds each file under `dir` to `out`, by its path under the pack
/// with the prefix `prefix`. It reads the type of each entry without
/// following a symbolic link, and refuses the first link that it meets.
fn collect(
    dir: &Path,
    prefix: &str,
    label: &str,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|e| Error::io(dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let path = entry.path();
        let rel = format!("{prefix}{name}");
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| Error::io(&path, e))?
            .file_type();
        if kind.is_symlink() {
            return Err(Error::Pack(format!(
                "{label}: {rel} is a symbolic link, and a pack from git holds only files and directories"
            )));
        }
        if kind.is_dir() {
            collect(&path, &format!("{rel}/"), label, out)?;
        } else {
            let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;
            out.insert(rel, bytes);
        }
    }
    Ok(())
}

/// `clone_dir` returns the directory `path` of the clone `worktree`, and
/// refuses a path with a `..` segment or a symbolic link on the way, so the
/// directory lies inside the clone.
// sokf-implements: CLI-13_AC-41
pub fn clone_dir(worktree: &Path, path: &str) -> Result<std::path::PathBuf> {
    let mut dir = worktree.to_path_buf();
    for segment in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if segment == ".." {
            return Err(Error::Pack(format!(
                "--path {path}: the path leaves the repository"
            )));
        }
        dir.push(segment);
        let Ok(meta) = fs::symlink_metadata(&dir) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            return Err(Error::Pack(format!(
                "--path {path}: {segment} is a symbolic link, so the path can leave the repository"
            )));
        }
    }
    Ok(dir)
}

/// `destination` returns where sync writes the file `rel` of the pack
/// `name` into the store `store`: a file of `schemas/` goes to the store's
/// `schemas/`, a file under `documents/` to the same path in the store, and
/// `formats.yaml` and `instructions.md` to the same name under
/// `/.sokf/packs/<NAME>/`. Any other file,
/// and each `index.md`, which `validate --fix` writes, has no destination.
// sokf-implements: CLI-13_AC-4
// sokf-implements: SOKF_FORMAT_162
// sokf-implements: CLI-13_AC-5
pub fn destination(name: &str, store: &str, rel: &str) -> Option<String> {
    if rel == "formats.yaml" {
        return Some(super::lock::formats_of(name).as_str().to_string());
    }
    // sokf-implements: CLI-13_AC-36
    if rel == "instructions.md" {
        return Some(super::lock::instructions_of(name).as_str().to_string());
    }
    if rel == "index.md" || rel.ends_with("/index.md") {
        return None;
    }
    let store = store.trim_end_matches('/');
    if let Some(rest) = rel.strip_prefix("schemas/") {
        return Some(format!("{store}/schemas/{rest}"));
    }
    rel.strip_prefix("documents/")
        .map(|rest| format!("{store}/{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: CLI-13_AC-19
    #[test]
    fn a_source_without_a_colon_or_a_slash_is_an_embedded_pack() {
        assert_eq!(Source::parse("sokf"), Source::Builtin("sokf".into()));
        assert_eq!(
            Source::parse("builtin:sokf"),
            Source::Builtin("sokf".into())
        );
        assert_eq!(
            Source::parse("git@example.com:o/r.git"),
            Source::Git("git@example.com:o/r.git".into())
        );
        assert_eq!(Source::parse("../pack"), Source::Git("../pack".into()));
        assert_eq!(Source::parse("sokf").label(), "builtin:sokf");
    }

    #[test]
    fn each_file_lands_at_its_destination() {
        let d = |rel| destination("m", "/knowledge", rel);
        assert_eq!(
            d("schemas/issue.md").unwrap(),
            "/knowledge/schemas/issue.md"
        );
        assert_eq!(
            d("documents/documentation-standards/general.md").unwrap(),
            "/knowledge/documentation-standards/general.md"
        );
        assert_eq!(d("formats.yaml").unwrap(), "/.sokf/packs/m/formats.yaml");
        assert_eq!(
            d("instructions.md").unwrap(),
            "/.sokf/packs/m/instructions.md"
        );
        assert_eq!(d("README.md"), None);
        assert_eq!(d("schemas/index.md"), None);
    }

    static FAKE: &[EmbeddedPack] = &[EmbeddedPack {
        dir: "fake",
        files: &[
            (
                "pack.sokf.yaml",
                "name: fake\nversion: 1.0.0\nrequires-sokf: 0.1.0\ndescription: A fake pack.\n",
            ),
            ("schemas/a.md", "a\n"),
        ],
    }];

    // sokf-tests: CLI-13_AC-20
    #[test]
    fn an_unknown_embedded_pack_lists_the_embedded_packs() {
        assert_eq!(find_embedded(FAKE, "fake").unwrap().files.len(), 1);
        let e = find_embedded(FAKE, "other").unwrap_err().to_string();
        assert!(e.contains("the embedded packs are fake"), "{e}");
        let e = find_embedded(&[], "other").unwrap_err().to_string();
        assert!(e.contains("the binary embeds no pack"), "{e}");
    }
}
