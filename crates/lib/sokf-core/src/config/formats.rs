//! This module reads the formats files of the repository: `/.sokf/formats.yaml`,
//! the repository layer of format resolution, which tramlines calls the
//! host's layer, and the files of `/.sokf/formats.d/`, the layer `formats.d`
//! between the embedded formats and the repository layer, in which no file
//! takes precedence over another (`TL_128`, CONF-11).

use super::formats_d::{
    check_below, check_clashes, folder_files, refuse_findings, settled_types, without_alike,
};
use crate::{
    Error, RepoPath, RepoRoot, Result, Spanned,
    grammar::{FormatRegistry, FormatSource, Grammar, RegexCache},
    source::{Fnv, read_optional},
    yaml,
};

/// `FORMATS_DIR` is the drop-in folder of the layer `formats.d`, relative
/// to the repository root.
pub const FORMATS_DIR: &str = ".sokf/formats.d";

/// `formats_path` returns the path of the formats file.
pub fn formats_path() -> RepoPath {
    RepoPath::parse("/.sokf/formats.yaml").expect("literal")
}

/// `RepoFormats` holds the formats layers that the repository supplies: the
/// repository layer and the layer `formats.d`.
#[derive(Debug, Clone, Default)]
pub struct RepoFormats {
    /// The repository layer, `/.sokf/formats.yaml`, when the file exists.
    pub repo: Option<FormatRegistry>,
    /// The layer `formats.d` on the embedded formats: the files of
    /// `/.sokf/formats.d/` merged without precedence, with the types of
    /// `/.sokf/formats.yaml` that settle a clash of the folder, or `None`
    /// when the folder holds no file.
    pub folder: Option<FormatRegistry>,
    /// The hash of every formats file, read in the order of the paths,
    /// which changes when a layer changes.
    pub hash: u64,
}

impl RepoFormats {
    /// `effective` returns the registry through which a name resolves: the
    /// repository layer, then the layer `formats.d`, then `embedded`.
    pub fn effective(&self, embedded: &FormatRegistry) -> FormatRegistry {
        let none = FormatRegistry::default();
        let repo = self.repo.as_ref().unwrap_or(&none);
        self.below_repo(embedded).layered(repo)
    }

    /// `below_repo` returns the layers on which `/.sokf/formats.yaml` lies:
    /// the layer `formats.d`, else `embedded`.
    fn below_repo<'a>(&'a self, embedded: &'a FormatRegistry) -> &'a FormatRegistry {
        self.folder.as_ref().unwrap_or(embedded)
    }

    /// `folder_layer` returns the entries and the types of the layer
    /// `formats.d` alone, or `None` when `/.sokf/formats.d/` holds no file.
    pub fn folder_layer(&self) -> Option<FormatRegistry> {
        let layer = self.folder.as_ref()?;
        Some(only(layer, |s| matches!(s, FormatSource::File(_))))
    }
}

/// `only` returns the entries and the types of `layer` whose source `keep`
/// takes.
fn only(layer: &FormatRegistry, keep: impl Fn(&FormatSource) -> bool) -> FormatRegistry {
    let mut out = layer.clone();
    out.entries.retain(|_, (_, s)| keep(s));
    out.types.entries.retain(|_, (_, s)| keep(s));
    out
}

/// `load_repo_formats` reads the repository's formats layers: the files of
/// `/.sokf/formats.d/`, as the layer `formats.d`, which is built on the
/// embedded formats, and `/.sokf/formats.yaml`, as the repository layer,
/// which is built on the layer `formats.d`. It parses each file of
/// `formats.d` on the embedded formats, and merges the files with
/// tramlines' `with_packs`, so that no file takes precedence over another.
/// It returns an error when a file does not parse, when an entry does not
/// match the grammar's `format` rule, when a format of `formats.d` no longer
/// compiles against the types of the layer, or when two files of
/// `formats.d` declare a format or a type with declarations that aren't
/// alike and `/.sokf/formats.yaml` does not declare it in full. It reads
/// only the files of the working tree, so a check reports the same findings
/// for the same tree.
// sokf-implements: CONF-11_AC-1
// sokf-implements: CONF-11_AC-4
// sokf-implements: CONF-11_AC-5
// sokf-implements: CONF-11_AC-6
pub fn load_repo_formats(
    root: &RepoRoot,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<RepoFormats> {
    let mut hash = Fnv::new();
    let embedded = crate::grammar::embedded_formats(grammar, regexes)?;
    let mut docs = Vec::new();
    for path in super::d_folder::d_files(root, FORMATS_DIR, "yaml", false)? {
        let Some(text) = read_optional(&path.to_fs(root))? else {
            continue;
        };
        hash.update(path.as_str().as_bytes());
        hash.update(text.as_bytes());
        let doc = parse_doc(&path, &text)?;
        docs.push((path, doc));
    }
    let repo_path = formats_path();
    let repo_text = read_optional(&repo_path.to_fs(root))?;
    if let Some(text) = &repo_text {
        hash.update(text.as_bytes());
    }
    let repo_doc = match &repo_text {
        Some(text) => Some(parse_doc(&repo_path, text)?),
        None => None,
    };
    // A name that two files declare alike counts once, as the first file's,
    // so neither sokf's check nor `with_packs` takes it for a clash.
    without_alike(&mut docs, &embedded);
    // Each clash is decided before any file of the folder compiles, so that
    // the error names the clash rather than a format that the clash breaks.
    check_clashes(&docs, repo_doc.as_ref())?;
    let settled = settled_types(
        &mut docs,
        repo_doc.as_ref().map(|d| (&repo_path, d)),
        &embedded,
        grammar,
        regexes,
    )?;
    let base = settled.as_ref().unwrap_or(&embedded);
    let files = folder_files(&docs, base, grammar, regexes)?;
    let folder = (!files.is_empty()).then(|| base.with_packs(&files));
    let mut formats = RepoFormats {
        repo: None,
        folder: folder.as_ref().map(|f| f.registry().clone()),
        hash: 0,
    };
    if let Some(doc) = &repo_doc {
        let registry = doc
            .get("registry")
            .ok_or_else(|| Error::formats(repo_path.clone(), "The file has no `registry` map."))?;
        let below = formats.below_repo(&embedded);
        formats.repo = Some(layer_of(
            &repo_path,
            doc,
            registry,
            FormatSource::Repo,
            grammar,
            regexes,
            below,
        )?);
    }
    if let Some(folder) = &folder {
        let none = FormatRegistry::default();
        let repo = formats.repo.as_ref().unwrap_or(&none);
        // sokf named each clash in its own words above; tramlines' findings
        // hold each format of the layer that no longer compiles, and any
        // clash that its rule finds and sokf's doesn't.
        refuse_findings(folder, repo)?;
        check_below(&embedded, folder.registry(), &files, repo, grammar, regexes)?;
    }
    formats.hash = hash.finish();
    Ok(formats)
}

/// `line_message` renders a finding of a formats file as `line N: message
/// (KEY)`, or without the line where it has none.
pub(super) fn line_message(f: &crate::Finding) -> String {
    match f.line {
        Some(l) => format!("line {l}: {}", crate::report::message(f)),
        None => crate::report::message(f),
    }
}

/// `parse_doc` reads the text `text` of the formats file `path` as YAML.
fn parse_doc(path: &RepoPath, text: &str) -> Result<Spanned> {
    yaml::parse(text, 1).map_err(|e| Error::formats(path.clone(), e.to_string()))
}

/// `layer_of` reads the formats `registry` and the `types` of `doc`, the
/// formats file `path`, as a layer with the source `source` on `base`.
pub(super) fn layer_of(
    path: &RepoPath,
    doc: &Spanned,
    registry: &Spanned,
    source: FormatSource,
    grammar: &Grammar,
    regexes: &RegexCache,
    base: &FormatRegistry,
) -> Result<FormatRegistry> {
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
        let messages: Vec<String> = findings.iter().map(line_message).collect();
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
