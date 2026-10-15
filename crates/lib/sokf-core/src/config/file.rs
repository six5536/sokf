//! This module reads the configuration file, `/.sokf/config.toml`, that
//! contract-config defines.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::RepoRoot;
use crate::{Error, RepoPath, Result, Severity, source::read_optional};

// sokf-implements: CONF-1_AC-2
// sokf-implements: CONF-1_AC-4
// sokf-implements: CONF-1_AC-5
// sokf:begin config-file
/// `ConfigFile` is the configuration file as CONF-1 defines it, with one
/// field for each table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ConfigFile {
    knowledge: KnowledgeSection,
    repository: RepositorySection,
    markers: MarkersSection,
    harness: BTreeMap<String, HarnessSection>,
}
// sokf:end config-file

// sokf:begin config-harness
/// `HarnessSection` holds one `[harness.NAME]` table (CONF-4). `without`
/// lists the parts of the harness that `install` skips. `command` sets the
/// program that the harness's hook lines run.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct HarnessSection {
    without: Vec<String>,
    command: Option<String>,
}
// sokf:end config-harness

impl ConfigFile {
    fn declined(&self) -> BTreeMap<String, Vec<String>> {
        self.harness
            .iter()
            .map(|(name, section)| (name.clone(), section.without.clone()))
            .collect()
    }

    /// `hook_commands` returns the program that each harness's hook line
    /// runs, for each harness whose table sets a `command`.
    fn hook_commands(&self) -> BTreeMap<String, String> {
        self.harness
            .iter()
            .filter_map(|(name, section)| section.command.clone().map(|c| (name.clone(), c)))
            .collect()
    }
}

// sokf-implements: CONF-2_AC-1
// sokf:begin config-knowledge
/// `KnowledgeSection` holds the `[knowledge]` table (CONF-2): the knowledge
/// roots, each of which the tool checks as a separate knowledge store.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct KnowledgeSection {
    roots: Vec<String>,
}

impl Default for KnowledgeSection {
    fn default() -> Self {
        KnowledgeSection {
            roots: vec!["/knowledge".into()],
        }
    }
}
// sokf:end config-knowledge

// sokf:begin config-repository
/// `RepositorySection` holds the `[repository]` table (CONF-3): the
/// `/`-rooted globs that the tool prunes from its walk of the repository
/// outside the knowledge roots. That walk finds the files that
/// `additional-files` globs match.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct RepositorySection {
    exclude: Vec<String>,
}
// sokf:end config-repository

// sokf:begin config-markers
/// `MarkersSection` holds the `[markers]` table (CONF-7), which sets how a
/// run reports the markers of the files outside the knowledge roots.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct MarkersSection {
    check: MarkerCheck,
}

/// `MarkerCheck` selects what a run does with markers: the run reports a broken
/// marker as an error or as a warning, or it does not check markers at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerCheck {
    /// A broken marker is an error.
    #[default]
    Error,
    /// A broken marker is a warning.
    Warn,
    /// The run does not check markers.
    Off,
}
// sokf:end config-markers

impl MarkerCheck {
    /// `severity` returns the severity of a broken marker: a warning under
    /// `warn`, and an error otherwise.
    pub fn severity(self) -> Severity {
        match self {
            MarkerCheck::Warn => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

/// `Config` is the resolved configuration, with every root checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The knowledge roots, each an existing directory.
    pub roots: Vec<RepoPath>,
    /// `/`-rooted globs pruned from the repository walk.
    pub exclude: Vec<String>,
    /// The declined parts of each harness, by harness name.
    pub declined: BTreeMap<String, Vec<String>>,
    /// How a run reports markers.
    pub markers: MarkerCheck,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            roots: vec![RepoPath::parse("/knowledge").expect("literal")],
            exclude: Vec::new(),
            declined: BTreeMap::new(),
            markers: MarkerCheck::default(),
        }
    }
}

/// `config_path` returns the path of the configuration file.
pub fn config_path() -> RepoPath {
    RepoPath::parse("/.sokf/config.toml").expect("literal")
}

/// `load_config` reads `/.sokf/config.toml`, or returns the defaults when
/// the file is absent. It refuses a file that does not parse, or that has an
/// unknown key, a glob that is not `/`-rooted or does not parse, or a root
/// that is not a directory.
pub fn load_config(root: &RepoRoot) -> Result<Config> {
    let path = config_path();
    let fs_path = path.to_fs(root);
    // The tool reads an absent file as a file with no tables, so it
    // checks the default root as it checks a root written in the file.
    // sokf-implements: CONF-1_AC-1
    let text = read_optional(&fs_path)?.unwrap_or_default();
    // sokf-implements: CONF-1_AC-3
    // sokf-implements: CONF-6_AC-1
    let file: ConfigFile =
        toml::from_str(&text).map_err(|e| Error::config(path.clone(), e.message().to_string()))?;
    let mut roots = Vec::new();
    for r in &file.knowledge.roots {
        // sokf-implements: CONF-2_AC-7
        let Some(rp) = RepoPath::parse(r) else {
            return Err(Error::config(
                path,
                format!("knowledge.roots entry `{r}` is not a /-rooted path"),
            ));
        };
        // sokf-implements: CONF-2_AC-6
        if !rp.to_fs(root).is_dir() {
            return Err(Error::config(
                path,
                format!("knowledge root `{r}` is not a directory"),
            ));
        }
        roots.push(rp);
    }
    for g in &file.repository.exclude {
        // sokf-implements: CONF-3_AC-5
        if !g.starts_with('/') {
            return Err(Error::config(
                path,
                format!("repository.exclude glob `{g}` is not /-rooted"),
            ));
        }
        // sokf-implements: CONF-3_AC-6
        if let Err(e) = globset::Glob::new(g) {
            return Err(Error::config(
                path,
                format!("repository.exclude glob `{g}` does not parse: {e}"),
            ));
        }
    }
    Ok(Config {
        roots,
        declined: file.declined(),
        exclude: file.repository.exclude,
        markers: file.markers.check,
    })
}

/// `ConfigText` is the configuration as written, for `init` and the harness
/// commands: the roots as listed, whether or not each one is a directory,
/// and the file's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigText {
    /// `knowledge.roots` as written, or the default when the file is absent.
    pub roots: Vec<RepoPath>,
    /// The declined parts of each harness, as written.
    pub declined: BTreeMap<String, Vec<String>>,
    /// The program each harness's hook line runs, where one is set.
    pub hook_commands: BTreeMap<String, String>,
    /// The file's text, or `None` when the file is absent.
    pub text: Option<String>,
}

/// `read_lenient` reads `/.sokf/config.toml` without requiring its roots to
/// exist. It refuses a file that does not parse, or a root that is not
/// `/`-rooted, with an `init` error (CONF-1_AC-3). `init` and the harness
/// commands call `read_lenient`, not `load_config`, because `load_config`
/// refuses a root that is not a directory. That refusal is right for every
/// other command, but wrong for `init`, the command that creates the roots.
// sokf-implements: CONF-1_AC-1
// sokf-implements: CONF-1_AC-3
// sokf-implements: CONF-2_AC-7
// sokf-implements: CONF-2_AC-8
pub fn read_lenient(root: &RepoRoot) -> Result<ConfigText> {
    let path = config_path();
    let fs_path = path.to_fs(root);
    let Some(text) = read_optional(&fs_path)? else {
        return Ok(ConfigText {
            roots: Config::default().roots,
            declined: BTreeMap::new(),
            hook_commands: BTreeMap::new(),
            text: None,
        });
    };
    let file: ConfigFile =
        toml::from_str(&text).map_err(|e| Error::Init(format!("{path}: {}", e.message())))?;
    let mut roots = Vec::new();
    for r in &file.knowledge.roots {
        let rp = RepoPath::parse(r).ok_or_else(|| {
            Error::Init(format!(
                "{path}: knowledge.roots entry `{r}` is not a /-rooted path"
            ))
        })?;
        roots.push(rp);
    }
    Ok(ConfigText {
        roots,
        declined: file.declined(),
        hook_commands: file.hook_commands(),
        text: Some(text),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::*;

    // sokf-tests: CONF-1_AC-1
    // sokf-tests: CONF-1_AC-3
    // sokf-tests: CONF-1_AC-4
    // sokf-tests: CONF-2_AC-7
    // sokf-tests: CONF-2_AC-8
    #[test]
    fn reads_leniently_for_init() {
        let dir = temp("lenient");
        let root = RepoRoot::new(dir.clone());
        assert_eq!(
            read_lenient(&root).unwrap(),
            ConfigText {
                roots: Config::default().roots,
                declined: BTreeMap::new(),
                hook_commands: BTreeMap::new(),
                text: None
            }
        );
        // `read_lenient` keeps a root that is not a directory, and it keeps
        // the harness table unchanged.
        let text = "# c\n[knowledge]\nroots = [\"/missing\", \"/docs\"]\n[harness.claude]\nwithout = [\"hooks\"]\n";
        fs::write(dir.join(".sokf/config.toml"), text).unwrap();
        assert_eq!(
            read_lenient(&root).unwrap(),
            ConfigText {
                roots: vec![
                    RepoPath::parse("/missing").unwrap(),
                    RepoPath::parse("/docs").unwrap()
                ],
                declined: BTreeMap::from([("claude".to_string(), vec!["hooks".to_string()])]),
                hook_commands: BTreeMap::new(),
                text: Some(text.into())
            }
        );
        for bad in [
            "not toml [",
            "[knowledge]\nroots = [\"docs\"]\n",
            "[nope]\n",
        ] {
            fs::write(dir.join(".sokf/config.toml"), bad).unwrap();
            let e = read_lenient(&root).unwrap_err();
            assert!(matches!(e, Error::Init(_)), "{bad:?} -> {e}");
            assert!(
                e.to_string().starts_with("init: /.sokf/config.toml: "),
                "{e}"
            );
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("config-{name}"));
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        fs::create_dir_all(dir.join("knowledge")).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        dir
    }

    fn with(dir: &Path, text: &str) -> Result<Config> {
        fs::write(dir.join(".sokf/config.toml"), text).unwrap();
        load_config(&RepoRoot::new(dir.to_path_buf()))
    }

    #[test]
    fn absent_file_gives_defaults() {
        let dir = temp("absent");
        assert_eq!(
            load_config(&RepoRoot::new(dir.clone())).unwrap(),
            Config::default()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-1_AC-1
    // sokf-tests: CONF-2_AC-1
    #[test]
    fn an_absent_file_reads_as_a_file_with_no_tables() {
        let dir = temp("no-tables");
        fs::remove_file(dir.join(".sokf/config.toml")).ok();
        let absent = load_config(&RepoRoot::new(dir.clone())).unwrap();
        assert_eq!(absent.roots, vec![RepoPath::parse("/knowledge").unwrap()]);
        for text in ["", "# A comment only.\n"] {
            assert_eq!(with(&dir, text).unwrap(), absent, "{text:?}");
        }
        // Without a `/knowledge` directory, the absent file and the empty
        // file give the same refusal.
        fs::remove_dir_all(dir.join("knowledge")).unwrap();
        let empty = with(&dir, "").unwrap_err().to_string();
        fs::remove_file(dir.join(".sokf/config.toml")).unwrap();
        let absent = load_config(&RepoRoot::new(dir.clone()))
            .unwrap_err()
            .to_string();
        assert_eq!(absent, empty);
        assert!(absent.contains("is not a directory"), "{absent}");
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-1_AC-2
    // sokf-tests: CONF-6_AC-1
    #[test]
    fn reads_roots_and_excludes() {
        let dir = temp("ok");
        let c = with(
            &dir,
            "[knowledge]\nroots = [\"/knowledge\", \"/docs\"]\n[repository]\nexclude = [\"/vendor/**\"]\n",
        )
        .unwrap();
        assert_eq!(c.roots.len(), 2);
        assert_eq!(c.exclude, vec!["/vendor/**"]);
        let c = with(&dir, "[repository]\nexclude = []\n").unwrap();
        assert_eq!(c.roots, Config::default().roots);
        // TOML 1.1 accepts a `\xHH` escape, which TOML 1.0 refuses.
        let c = with(&dir, "[knowledge]\nroots = [\"\\x2Fdocs\"]\n").unwrap();
        assert_eq!(c.roots, vec![RepoPath::parse("/docs").unwrap()]);
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-7_AC-1
    // sokf-tests: CONF-7_AC-2
    #[test]
    fn reads_the_markers_table() {
        let dir = temp("markers");
        assert_eq!(with(&dir, "").unwrap().markers, MarkerCheck::Error);
        for (text, check) in [
            ("[markers]\ncheck = \"error\"\n", MarkerCheck::Error),
            ("[markers]\ncheck = \"warn\"\n", MarkerCheck::Warn),
            ("[markers]\ncheck = \"off\"\n", MarkerCheck::Off),
            ("[markers]\n", MarkerCheck::Error),
        ] {
            assert_eq!(with(&dir, text).unwrap().markers, check, "{text:?}");
        }
        for bad in [
            "[markers]\ncheck = \"loud\"\n",
            "[markers]\nchecks = \"off\"\n",
        ] {
            let e = with(&dir, bad).unwrap_err();
            assert!(matches!(e, Error::Config { .. }), "{bad:?} -> {e}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-4_AC-1
    #[test]
    fn reads_the_harness_table() {
        let dir = temp("harness");
        let c = with(
            &dir,
            "[harness.claude]\nwithout = [\"hooks\", \"skills\"]\n",
        )
        .unwrap();
        assert_eq!(
            c.declined,
            BTreeMap::from([(
                "claude".to_string(),
                vec!["hooks".to_string(), "skills".to_string()]
            )])
        );
        // A table with no `without` key declines nothing, and the tool reads
        // a table for another harness as written.
        let c = with(&dir, "[harness.claude]\n[harness.other]\nwithout = []\n").unwrap();
        assert_eq!(c.declined["claude"], Vec::<String>::new());
        assert_eq!(c.declined["other"], Vec::<String>::new());
        // The tool refuses an unknown key in the table, as it refuses every
        // unknown key.
        let e = with(&dir, "[harness.claude]\nwith = []\n").unwrap_err();
        assert!(matches!(e, Error::Config { .. }), "{e}");
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CONF-1_AC-3
    // sokf-tests: CONF-1_AC-4
    // sokf-tests: CONF-2_AC-6
    // sokf-tests: CONF-2_AC-7
    // sokf-tests: CONF-3_AC-5
    // sokf-tests: CONF-3_AC-6
    #[test]
    fn refuses_bad_files() {
        let dir = temp("bad");
        for text in [
            "not toml [",
            "[knowledge]\nrootz = []\n",
            "[other]\nx = 1\n",
            "[knowledge]\nroots = [\"knowledge\"]\n",
            "[knowledge]\nroots = [\"/missing\"]\n",
            "[repository]\nexclude = [\"vendor/**\"]\n",
            "[repository]\nexclude = [\"/[\"]\n",
        ] {
            let e = with(&dir, text).unwrap_err();
            assert!(matches!(e, Error::Config { .. }), "{text:?} -> {e}");
            assert!(e.to_string().starts_with("/.sokf/config.toml: "), "{e}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
