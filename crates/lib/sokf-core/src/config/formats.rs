//! This module reads `/.sokf/formats.yaml`, the repository layer of format
//! resolution (TL_34).

use crate::RepoRoot;
use crate::{
    Error, RepoPath, Result,
    grammar::{FormatRegistry, FormatSource, Grammar, RegexCache},
    source::read_optional,
    yaml,
};

/// `formats_path` returns the path of the formats file.
pub fn formats_path() -> RepoPath {
    RepoPath::parse("/.sokf/formats.yaml").expect("literal")
}

/// `load_repo_formats` reads `/.sokf/formats.yaml` as the repository layer,
/// and returns `None` when the file is absent. It returns an error when the
/// file does not parse, or when an entry does not match the grammar's
/// `format` rule.
pub fn load_repo_formats(
    root: &RepoRoot,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<Option<FormatRegistry>> {
    let path = formats_path();
    let fs_path = path.to_fs(root);
    let Some(text) = read_optional(&fs_path)? else {
        return Ok(None);
    };
    let doc = yaml::parse(&text, 1).map_err(|e| Error::formats(path.clone(), e.to_string()))?;
    let registry = doc
        .get("registry")
        .ok_or_else(|| Error::formats(path.clone(), "The file has no `registry` map."))?;
    let embedded = crate::grammar::embedded_formats(grammar, regexes)?;
    FormatRegistry::from_yaml_at(
        registry,
        doc.get("types"),
        FormatSource::Repo,
        grammar,
        regexes,
        &path,
        Some(&embedded),
    )
    .map(Some)
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
