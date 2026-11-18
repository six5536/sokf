//! This module reads the formats files of the repository: `/.sokf/formats.yaml`,
//! the repository layer of format resolution, which tramlines calls the
//! host's layer, and the files of `/.sokf/formats.d/`, the layer `formats.d`
//! between the embedded formats and the repository layer, in which no file
//! takes precedence over another (`TL_128`, CONF-11).

use super::formats_d::{
    check_clashes, folder_files, refuse_findings, settled_types, unsettled, without_alike,
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
    /// The registry through which a name resolves, which tramlines'
    /// `with_host` returns, or `None` when the repository has no formats
    /// file.
    merged: Option<FormatRegistry>,
    /// The hash of every formats file, read in the order of the paths,
    /// which changes when a layer changes.
    pub hash: u64,
}

impl RepoFormats {
    /// `effective` returns the registry through which a name resolves: the
    /// repository layer, then the layer `formats.d`, then `embedded`.
    pub fn effective(&self, embedded: &FormatRegistry) -> FormatRegistry {
        self.merged.clone().unwrap_or_else(|| embedded.clone())
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
/// tramlines' `with_packs`, so that no file takes precedence over another,
/// and lays `/.sokf/formats.yaml` on them with tramlines' `with_host`. It
/// reads only the files of the working tree, so a check reports the same
/// findings for the same tree. It returns an error when:
///
/// - a file does not parse, or an entry does not match the grammar's
///   `format` rule;
/// - a format of `formats.d` no longer compiles against the types of the
///   layer;
/// - two files of `formats.d` declare a format or a type with declarations
///   that aren't alike, and `/.sokf/formats.yaml` does not declare it in
///   full;
/// - a format below `/.sokf/formats.yaml` no longer compiles against a type
///   of a layer above its own, and `/.sokf/formats.yaml` leaves it broken.
pub fn load_repo_formats(
    root: &RepoRoot,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<RepoFormats> {
    let embedded = crate::grammar::embedded_formats(grammar, regexes)?;
    load_repo_formats_on(root, embedded, grammar, regexes)
}

/// `load_repo_formats_on` does what `load_repo_formats` does, on `embedded`
/// as the embedded formats, so that a test can lay the layers on formats
/// that the tool doesn't embed.
// sokf-implements: CONF-11_AC-1
// sokf-implements: CONF-11_AC-4
// sokf-implements: CONF-11_AC-5
// sokf-implements: CONF-11_AC-6
fn load_repo_formats_on(
    root: &RepoRoot,
    embedded: FormatRegistry,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<RepoFormats> {
    let mut hash = Fnv::new();
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
    let (base, broken) = match settled_types(
        &mut docs,
        repo_doc.as_ref().map(|d| (&repo_path, d)),
        &embedded,
        grammar,
        regexes,
    )? {
        Some((base, broken)) => (base, broken),
        None => (embedded, Vec::new()),
    };
    let files = folder_files(&docs, &base, grammar, regexes)?;
    // tramlines keeps in the registry of the layer each format of `base`
    // that a type of the folder breaks, so that `/.sokf/formats.yaml` can
    // settle it, and `with_host` decides it.
    let layer = base.with_packs(&files);
    let repo = match &repo_doc {
        Some(doc) => {
            let registry = doc.get("registry").ok_or_else(|| {
                Error::formats(repo_path.clone(), "The file has no `registry` map.")
            })?;
            Some(layer_of(
                &repo_path,
                doc,
                registry,
                FormatSource::Repo,
                grammar,
                regexes,
                layer.registry(),
            )?)
        }
        None => None,
    };
    let none = FormatRegistry::default();
    let host = repo.as_ref().unwrap_or(&none);
    let merged = refuse_findings(&layer, host)?;
    unsettled(&broken, host)?;
    let formats = RepoFormats {
        folder: (!files.is_empty()).then(|| layer.registry().clone()),
        merged: (repo.is_some() || !files.is_empty()).then_some(merged),
        repo,
        hash: hash.finish(),
    };
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
    use crate::grammar::{FormatMatch, load_grammar};

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

    // An embedded format that uses a type whose clash `/.sokf/formats.yaml`
    // settles compiles against the settled declaration, the one that a
    // schema sees.
    // sokf-tests: CONF-11_AC-10
    #[test]
    fn an_embedded_format_takes_the_type_that_settles_a_clash() {
        let l = Layers::new("settled");
        for (file, kinds) in [("a", "[Versioning]"), ("b", "[Deprecation]")] {
            l.write(
                &format!(".sokf/formats.d/{file}/formats.yaml"),
                Some(&format!(
                    "types:\n  evolution-kind: {kinds}\nregistry: {{}}\n"
                )),
            );
        }
        l.write(
            ".sokf/formats.yaml",
            Some("types:\n  evolution-kind: [Retirement]\nregistry: {}\n"),
        );
        let (format, _) = l.entry("requirement-evolution");
        assert!(
            matches!(
                format.check("Retirement: The file goes."),
                FormatMatch::Ok(_)
            ),
            "{:?}",
            format.check("Retirement: The file goes.")
        );
        assert!(matches!(
            format.check("Versioning: The file goes."),
            FormatMatch::Fail(_)
        ));
    }

    /// `ONE` and `TWO` are two files of `/.sokf/formats.d/`, each in a
    /// folder of its own.
    const ONE: &str = ".sokf/formats.d/pakf/@acme/one/formats.yaml";
    const TWO: &str = ".sokf/formats.d/pakf/@acme/two/formats.yaml";

    /// `BIG` is a type with which two holes exceed the size limit of a regex.
    const BIG: &str = "'\\w{1,200}'";

    /// `Layers` is a repository whose formats files a test writes, laid on
    /// the fixture's requirement formats as the embedded layer, because the
    /// tool embeds no format of its own.
    struct Layers {
        dir: PathBuf,
        grammar: Grammar,
        embedded: FormatRegistry,
    }

    impl Layers {
        fn new(name: &str) -> Self {
            let grammar = load_grammar().unwrap();
            let embedded = crate::test_support::requirement_formats(&grammar);
            Layers {
                dir: temp(name),
                grammar,
                embedded,
            }
        }

        /// `write` writes `text` to the file `path`, or removes the file
        /// where `text` is `None`.
        fn write(&self, path: &str, text: Option<&str>) {
            match text {
                Some(text) => crate::test_support::write(&self.dir, path, text),
                None => fs::remove_file(self.dir.join(path)).unwrap(),
            }
        }

        fn load(&self) -> Result<RepoFormats> {
            load_repo_formats_on(
                &RepoRoot::new(self.dir.clone()),
                self.embedded.clone(),
                &self.grammar,
                &RegexCache::new(),
            )
        }

        /// `error` returns the error that loading the layers stops with.
        fn error(&self) -> String {
            self.load().unwrap_err().to_string()
        }

        /// `entry` returns the effective entry `name` and its source.
        fn entry(&self, name: &str) -> (crate::grammar::Format, FormatSource) {
            let effective = self.load().unwrap().effective(&self.embedded);
            effective.entries[name].clone()
        }
    }

    impl Drop for Layers {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn one() -> FormatSource {
        FormatSource::File("/.sokf/formats.d/pakf/@acme/one/formats.yaml".into())
    }

    // sokf-tests: CONF-11_AC-1
    // sokf-tests: CONF-11_AC-4
    #[test]
    fn the_layer_formats_d_lies_above_the_embedded_formats() {
        let l = Layers::new("above");
        assert_eq!(l.entry("requirement-evolution").1, FormatSource::Embedded);
        // A patch of the folder merges onto the embedded entry.
        l.write(
            ONE,
            Some("registry:\n  requirement-evolution:\n    patch: true\n    example: 'Versioning: The file changes with each release.'\n"),
        );
        let (format, source) = l.entry("requirement-evolution");
        assert_eq!(source, one());
        assert!(
            format
                .examples()
                .contains(&"Versioning: The file changes with each release.")
        );
        assert!(matches!(
            format.check("Sunset: A sentence."),
            FormatMatch::Fail(_)
        ));
        // An entry in full of the folder replaces the embedded one.
        l.write(
            ONE,
            Some("registry:\n  requirement-evolution:\n    pattern: '.*'\n    example: A line.\n"),
        );
        let (format, source) = l.entry("requirement-evolution");
        assert_eq!(source, one());
        assert!(matches!(
            format.check("Sunset: anything"),
            FormatMatch::Ok(_)
        ));
    }

    // sokf-tests: CONF-11_AC-4
    #[test]
    fn a_patch_of_formats_d_merges_onto_the_layers_below() {
        let l = Layers::new("patch");
        l.write(
            ONE,
            Some("registry:\n  user-story:\n    patch: true\n    example: 'ST-1: AS A writer, I WANT a patch, SO THAT the folder changes a format.'\n"),
        );
        let (format, _) = l.entry("user-story");
        assert!(
            format.examples().contains(
                &"ST-1: AS A writer, I WANT a patch, SO THAT the folder changes a format."
            )
        );
        // The template and the key come from the embedded entry that the
        // patch merges onto.
        assert!(matches!(
            format.check("ST-1: I want a patch."),
            FormatMatch::Fail(_)
        ));
        let story = "ST-2: AS A writer, I WANT a story, SO THAT it reads.";
        assert!(
            matches!(format.check(story), FormatMatch::Ok(Some(ref c)) if c.key == "ST-2"),
            "{:?}",
            format.check(story)
        );
    }

    // sokf-tests: CONF-11_AC-9
    #[test]
    fn alike_patches_of_formats_d_count_once() {
        let l = Layers::new("alike");
        let story = "ST-1: AS A writer, I WANT a patch, SO THAT the folder changes a format.";
        l.write(
            ONE,
            Some(&format!(
                "registry:\n  user-story:\n    patch: true\n    example: '{story}'\n"
            )),
        );
        l.write(
            TWO,
            Some(&format!("# The same patch as one.\nregistry:\n  user-story: {{ example: \"{story}\", patch: true }}\n")),
        );
        // The first file in the order of the paths stands for the name.
        let layer = l.load().unwrap().folder_layer().unwrap();
        assert_eq!(layer.entries["user-story"].1, one());
        // Two patches that list the variants that they hold in another order
        // are alike too.
        let ubiquitous = "      ubiquitous:\n        example: 'PLUMB-4_AC-1 [ubiquitous]: the plumber SHALL queue a message.'\n";
        let event = "      event:\n        example: 'PLUMB-1_AC-1 [event]: WHEN a program writes, the plumber SHALL deliver the message.'\n";
        let patch = |first: &str, second: &str| {
            format!(
                "registry:\n  requirement-ears:\n    patch: true\n    variants:\n{first}{second}"
            )
        };
        l.write(ONE, Some(&patch(ubiquitous, event)));
        l.write(TWO, Some(&patch(event, ubiquitous)));
        assert!(l.load().is_ok());
    }

    // sokf-tests: CONF-11_AC-5
    // sokf-tests: CONF-11_AC-11
    #[test]
    fn an_embedded_format_that_a_type_of_formats_d_breaks_stops_the_run() {
        // `requirement-evolution` of the embedded formats uses the types
        // `evolution-kind` and `sentence`.
        let l = Layers::new("embedded-d");
        let types = format!("types:\n  evolution-kind: {BIG}\n  sentence: {BIG}\n");
        l.write(ONE, Some(&format!("{types}registry: {{}}\n")));
        let err = l.error();
        assert!(
            err.starts_with("/.sokf/formats.d/pakf/@acme/one/formats.yaml: ")
                && err.contains("`requirement-evolution`")
                && err.contains("the layer `embedded`")
                && err.contains("`evolution-kind`"),
            "{err}"
        );
        // A patch of the folder that takes a type out of the format settles
        // it too.
        l.write(
            ONE,
            Some(&format!(
                "{types}registry:\n  requirement-evolution:\n    patch: true\n    template: '${{:evolution-kind}}: A sentence.'\n    example: 'Versioning: A sentence.'\n"
            )),
        );
        assert!(l.load().is_ok());
        l.write(ONE, Some(&format!("{types}registry: {{}}\n")));
        // The project settles it by declaring the format in full, which no
        // schema then sees below the folder.
        l.write(
            ".sokf/formats.yaml",
            Some("registry:\n  requirement-evolution:\n    pattern: '.*'\n    example: 'Versioning: A sentence.'\n"),
        );
        assert!(l.load().is_ok());
        // Or by a patch that takes a type out of the format.
        l.write(
            ".sokf/formats.yaml",
            Some("registry:\n  requirement-evolution:\n    patch: true\n    template: '${:evolution-kind}: A sentence.'\n    example: 'Versioning: A sentence.'\n"),
        );
        assert!(l.load().is_ok());
        // Or by declaring a type, so that no schema sees the folder's.
        l.write(
            ".sokf/formats.yaml",
            Some("types:\n  sentence: '[A-Z].*\\.'\nregistry: {}\n"),
        );
        assert!(l.load().is_ok());
        // A type of the folder against which the format compiles passes.
        l.write(".sokf/formats.yaml", None);
        l.write(
            ONE,
            Some(&format!("types:\n  evolution-kind: [Versioning, Deprecation]\n  sentence: {BIG}\nregistry: {{}}\n")),
        );
        assert!(l.load().is_ok());
    }

    // sokf-tests: CONF-11_AC-11
    #[test]
    fn a_format_below_compiles_against_the_types_that_a_schema_sees() {
        let l = Layers::new("below");
        // The error names each type of the folder that the format uses, on
        // the file of the first.
        l.write(
            ONE,
            Some(&format!(
                "types:\n  evolution-kind: {BIG}\nregistry: {{}}\n"
            )),
        );
        l.write(
            TWO,
            Some("types:\n  sentence: '\\w{1,199}'\nregistry: {}\n"),
        );
        let err = l.error();
        assert!(
            err.starts_with("/.sokf/formats.d/pakf/@acme/one/formats.yaml: ")
                && err.contains("format `requirement-evolution`: ")
                && err.contains("the types `evolution-kind`, `sentence` of this layer"),
            "{err}"
        );
        // A type of `/.sokf/formats.yaml` takes part, so the format compiles.
        l.write(TWO, None);
        l.write(
            ".sokf/formats.yaml",
            Some("types:\n  sentence: [x]\nregistry: {}\n"),
        );
        assert!(l.load().is_ok());
        // `requirement-property` uses the two types that the folder changes,
        // so only that format fails, and `requirement-evolution` beside it
        // compiles.
        l.write(".sokf/formats.yaml", None);
        l.write(
            ONE,
            Some(&format!(
                "types:\n  property-id: {BIG}\n  property-shape: {BIG}\nregistry: {{}}\n"
            )),
        );
        let err = l.error();
        assert!(
            err.contains("`requirement-property`") && !err.contains("`requirement-evolution`"),
            "{err}"
        );
        // A format that compiles under the folder and that a type of
        // `/.sokf/formats.yaml` breaks lies on `/.sokf/formats.yaml`.
        l.write(
            ONE,
            Some("types:\n  evolution-kind: '\\w{1,199}'\nregistry: {}\n"),
        );
        l.write(
            ".sokf/formats.yaml",
            Some("types:\n  sentence: '\\w{1,201}'\nregistry: {}\n"),
        );
        let err = l.error();
        assert!(
            err.starts_with("/.sokf/formats.yaml: ")
                && err.contains("format `requirement-evolution`: ")
                && err.contains("the type `sentence` of this layer"),
            "{err}"
        );
        l.write(
            ".sokf/formats.yaml",
            Some("types:\n  sentence: [x]\nregistry: {}\n"),
        );
        assert!(l.load().is_ok());
    }

    // sokf-tests: CONF-11_AC-12
    #[test]
    fn an_embedded_format_that_a_type_of_formats_yaml_breaks_stops_the_run() {
        let l = Layers::new("embedded-repo");
        let types = format!("types:\n  evolution-kind: {BIG}\n  sentence: {BIG}\n");
        l.write(
            ".sokf/formats.yaml",
            Some(&format!("{types}registry: {{}}\n")),
        );
        let err = l.error();
        assert!(
            err.starts_with("/.sokf/formats.yaml: ")
                && err.contains(
                    "format `requirement-evolution`: the entry of the layer `embedded` no longer compiles against the types `evolution-kind`, `sentence` of this layer"
                ),
            "{err}"
        );
        // A type that `/.sokf/formats.yaml` settles for the folder breaks the
        // format alike.
        l.write(
            ONE,
            Some("types:\n  sentence: '[A-Z].*\\.'\nregistry: {}\n"),
        );
        l.write(
            TWO,
            Some("types:\n  sentence: '[A-Z].*[.!]'\nregistry: {}\n"),
        );
        let err = l.error();
        assert!(
            err.starts_with("/.sokf/formats.yaml: ") && err.contains("`requirement-evolution`"),
            "{err}"
        );
        // A type against which the format compiles passes.
        l.write(
            ".sokf/formats.yaml",
            Some(&format!("types:\n  sentence: {BIG}\nregistry: {{}}\n")),
        );
        assert!(l.load().is_ok());
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
