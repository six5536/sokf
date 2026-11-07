//! This module reads the formats files of the repository: `/.sokf/formats.yaml`,
//! the repository layer of format resolution, which tramlines calls the
//! host's layer, and the `formats.yaml` of each pack in the lock, the pack
//! layer, stacked in the order of the packs (TL_34, CONF-8).

use crate::RepoRoot;
use crate::{
    Error, RepoPath, Result,
    grammar::{FormatRegistry, FormatSource, Grammar, RegexCache},
    pack,
    source::{Fnv, read_optional},
    yaml,
};

/// `formats_path` returns the path of the formats file.
pub fn formats_path() -> RepoPath {
    RepoPath::parse("/.sokf/formats.yaml").expect("literal")
}

/// `RepoFormats` holds the formats layers that the repository supplies: the
/// repository layer and the pack layer.
#[derive(Debug, Clone, Default)]
pub struct RepoFormats {
    /// The repository layer, `/.sokf/formats.yaml`, when the file exists.
    pub repo: Option<FormatRegistry>,
    /// The pack layer on the embedded formats: the formats file of each
    /// pack in the lock that has one, stacked in the order of the packs, or
    /// `None` when no pack has a formats file.
    pub packs: Option<FormatRegistry>,
    /// The hash of every formats file, read in the order of the packs,
    /// which changes when a layer or the order of the packs changes.
    pub hash: u64,
}

impl RepoFormats {
    /// `effective` returns the registry through which a name resolves: the
    /// repository layer, then the pack layer, then `embedded`.
    // sokf-implements: CLI-13_AC-33
    pub fn effective(&self, embedded: &FormatRegistry) -> FormatRegistry {
        let none = FormatRegistry::default();
        let repo = self.repo.as_ref().unwrap_or(&none);
        self.packs.as_ref().unwrap_or(embedded).layered(repo)
    }

    /// `pack_layer` returns the entries and the types of the pack layer
    /// alone, or `None` when no pack in the lock has a formats file.
    pub fn pack_layer(&self) -> Option<FormatRegistry> {
        let layer = self.packs.as_ref()?;
        let mut packs = FormatRegistry::default();
        let pack = |s: &FormatSource| matches!(s, FormatSource::Pack(_));
        packs.entries = layer
            .entries
            .iter()
            .filter(|(_, (_, s))| pack(s))
            .map(|(n, e)| (n.clone(), e.clone()))
            .collect();
        packs.types.entries = layer
            .types
            .entries
            .iter()
            .filter(|(_, (_, s))| pack(s))
            .map(|(n, e)| (n.clone(), e.clone()))
            .collect();
        Some(packs)
    }
}

/// `load_repo_formats` reads the repository's formats layers: the formats
/// file of each pack in the lock, as the pack layer, and
/// `/.sokf/formats.yaml`, as the repository layer, which is built on the
/// pack layer. It stacks the files of the packs from the lowest pack to the
/// highest, each parsed on the packs below it, so that a higher pack's
/// entry replaces a lower pack's entry of the same name, a `patch: true`
/// entry merges onto it, and the hash changes with the order. It returns an
/// error when a file does not parse, or when an entry does not match the
/// grammar's `format` rule.
/// It reads only the lock and the files of the working tree, never a pack's
/// source, so a check reports the same findings for the same tree.
// sokf-implements: CONF-8_AC-4
// sokf-implements: CLI-13_AC-35
// sokf-implements: CLI-13_AC-34
// sokf-implements: CLI-13_AC-83
// sokf-implements: CLI-13_AC-84
pub fn load_repo_formats(
    root: &RepoRoot,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<RepoFormats> {
    let mut hash = Fnv::new();
    let lock = pack::read_lock(root)?;
    let embedded = crate::grammar::embedded_formats(grammar, regexes)?;
    let mut layer: Option<FormatRegistry> = None;
    for name in pack::ordered(root, &lock) {
        let path = pack::formats_of(&name);
        let Some(text) = read_optional(&path.to_fs(root))? else {
            continue;
        };
        hash.update(path.as_str().as_bytes());
        hash.update(text.as_bytes());
        let below = layer.as_ref().unwrap_or(&embedded);
        let registry = parse(
            &path,
            &text,
            FormatSource::Pack(name.clone()),
            grammar,
            regexes,
            below,
        )?;
        layer = Some(below.layered(&registry));
    }
    let mut formats = RepoFormats {
        repo: None,
        packs: layer,
        hash: 0,
    };
    let path = formats_path();
    if let Some(text) = read_optional(&path.to_fs(root))? {
        hash.update(text.as_bytes());
        let below = formats.packs.as_ref().unwrap_or(&embedded);
        formats.repo = Some(parse(
            &path,
            &text,
            FormatSource::Repo,
            grammar,
            regexes,
            below,
        )?);
    }
    formats.hash = hash.finish();
    Ok(formats)
}

/// `parse` reads the formats file `path`, with the text `text`, as a layer
/// with the source `source` on `base`.
fn parse(
    path: &RepoPath,
    text: &str,
    source: FormatSource,
    grammar: &Grammar,
    regexes: &RegexCache,
    base: &FormatRegistry,
) -> Result<FormatRegistry> {
    let doc = yaml::parse(text, 1).map_err(|e| Error::formats(path.clone(), e.to_string()))?;
    let registry = doc
        .get("registry")
        .ok_or_else(|| Error::formats(path.clone(), "The file has no `registry` map."))?;
    FormatRegistry::from_yaml_at(
        registry,
        doc.get("types"),
        source,
        grammar,
        regexes,
        path,
        Some(base),
    )
    .map_err(|findings| {
        let messages: Vec<String> = findings
            .iter()
            .map(|f| match f.line {
                Some(l) => format!("line {l}: {}", crate::report::message(f)),
                None => crate::report::message(f),
            })
            .collect();
        Error::formats(path.clone(), messages.join("; "))
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::*;
    use crate::grammar::load_grammar;

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("formats-{name}"));
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        dir
    }

    fn with(dir: &Path, text: Option<&str>) -> Result<Option<FormatRegistry>> {
        with_all(dir, text).map(|f| f.repo)
    }

    fn with_all(dir: &Path, text: Option<&str>) -> Result<RepoFormats> {
        if let Some(text) = text {
            fs::write(dir.join(".sokf/formats.yaml"), text).unwrap();
        }
        let grammar = load_grammar().unwrap();
        load_repo_formats(
            &RepoRoot::new(dir.to_path_buf()),
            &grammar,
            &RegexCache::new(),
        )
    }

    #[test]
    fn absent_file_is_none_and_a_good_file_is_the_repo_layer() {
        let dir = temp("ok");
        assert!(with(&dir, None).unwrap().is_none());
        let r = with(
            &dir,
            Some("registry:\n  requirement-ears:\n    pattern: '.*'\n    example: x\n"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(r.entries["requirement-ears"].1, FormatSource::Repo);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_bad_files() {
        let dir = temp("bad");
        for text in [
            "registry: [",
            "other: 1\n",
            "registry:\n  requirement-ears:\n    doc: d\n",
            "registry:\n  requirement-ears:\n    doc: d\n    example: e\n    pattern: '('\n",
        ] {
            let e = with(&dir, Some(text)).unwrap_err();
            assert!(matches!(e, Error::FormatsFile { .. }), "{text:?} -> {e}");
            assert!(e.to_string().starts_with("/.sokf/formats.yaml: "), "{e}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
