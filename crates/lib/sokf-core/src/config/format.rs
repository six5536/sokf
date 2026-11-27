//! This module reads the `[format]` table of `/.sokf/config.toml`
//! (CONF-10): the formatter or the command through which same-same compares
//! a generated block of a format that it doesn't compare by meaning, the
//! limit of one run of it, and the named things that count as the same.

use std::{
    fs,
    path::{Component, Path},
    time::Duration,
};

use same_same::{KnownFormatter, Named, NamedSet, Settings};
use serde::Deserialize;

// sokf:begin config-format
/// `FormatSection` holds the `[format]` table (CONF-10). `formatter` names
/// a formatter that same-same knows, with the project's configuration file
/// `config` and the way `settings` in which same-same takes it, and Prettier's
/// `plugins`; `command` names any other program, which runs as written, with
/// `config` as the file of `{settings}`; `detect` asks same-same to detect
/// the formatter at the repository root; `timeout` is the seconds that one
/// run lasts at most; `same` lists the named things of same-same beyond the
/// six that sokf reads as the same.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(super) struct FormatSection {
    formatter: Option<String>,
    config: Option<String>,
    settings: Option<toml::Value>,
    plugins: Option<Vec<String>>,
    command: Option<Vec<String>>,
    detect: Option<bool>,
    timeout: Option<i64>,
    same: Option<Vec<String>>,
}
// sokf:end config-format

/// `SETTINGS_TABLE` is the error of a `format.settings` that is a table,
/// such as the table of a formatter's settings that sokf 0.8 took, which
/// says where those settings go now.
const SETTINGS_TABLE: &str = "format.settings must be internal-with-overrides, as-written, or internal-only, not a table: put the formatter's settings in its configuration file, and name that file with format.config";

/// `FORMAT_TIMEOUT` is the longest that one run of the formatter lasts
/// when `format.timeout` is absent (CONF-10_AC-20).
pub const FORMAT_TIMEOUT: Duration = Duration::from_secs(10);

/// `CURRENT` holds the six named things that sokf reads as the same
/// whatever `format.same` says (CLI-4_AC-109).
pub const CURRENT: [Named; 6] = [
    Named::KeyOrder,
    Named::CommentPlace,
    Named::ImportOrder,
    Named::LineEndings,
    Named::ByteOrderMark,
    Named::ClassOrder,
];

/// `COUNTED` holds the named things that sokf counts as a difference unless
/// `format.same` names them, and so the entries that `format.same` accepts
/// (CONF-10_AC-22).
pub const COUNTED: [Named; 1] = [Named::Yaml11Value];

/// `FormatRule` is the formatter or the command that `[format]` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatRule {
    /// A formatter that same-same knows.
    Formatter {
        /// The formatter.
        formatter: KnownFormatter,
        /// The path of its configuration file from the repository root.
        config: Option<String>,
        /// The way in which it takes the configuration.
        settings: Settings,
        /// Prettier's plugins.
        plugins: Vec<String>,
    },
    /// Any other program, which runs as written.
    Command {
        /// The program and its arguments.
        argv: Vec<String>,
        /// The path from the repository root of the file that `{settings}`
        /// names.
        config: Option<String>,
    },
}

/// `FormatConfig` is the resolved `[format]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatConfig {
    /// The formatter or the command, or `None`.
    pub rule: Option<FormatRule>,
    /// Whether same-same detects the formatter at the repository root, for
    /// the texts that `rule` doesn't serve; off unless `format.detect` turns
    /// it on (CONF-10_AC-2, CONF-10_AC-26).
    pub detect: bool,
    /// The longest that one run of the formatter or the command lasts.
    pub timeout: Duration,
    /// The named things that sokf reads as the same: the six, and those that
    /// `format.same` lists.
    pub same: NamedSet,
}

impl Default for FormatConfig {
    // sokf-implements: CONF-10_AC-20
    fn default() -> FormatConfig {
        FormatConfig {
            rule: None,
            detect: false,
            timeout: FORMAT_TIMEOUT,
            same: CURRENT.iter().fold(NamedSet::new(), |mut set, &named| {
                set.insert(named);
                set
            }),
        }
    }
}

impl FormatSection {
    /// `resolve` checks the table and returns it as a `FormatConfig`, or the
    /// message of its first fault.
    // sokf-implements: CONF-10_AC-4
    // sokf-implements: CONF-10_AC-21
    // sokf-implements: CONF-10_AC-23
    // sokf-implements: CONF-10_AC-24
    // sokf-implements: CONF-10_AC-25
    pub(super) fn resolve(self) -> Result<FormatConfig, String> {
        let config = self.config.as_deref().map(config_path).transpose()?;
        let rule = match (self.formatter, self.command) {
            (Some(_), Some(_)) => {
                return Err(
                    "format.formatter and format.command are both set; set one of them".into(),
                );
            }
            (Some(name), None) => {
                let formatter = KnownFormatter::from_name(&name).ok_or_else(|| {
                    format!(
                        "format.formatter must be oxfmt, prettier, biome, or dprint, not `{name}`"
                    )
                })?;
                let settings = match self.settings {
                    None => Settings::default(),
                    Some(toml::Value::String(way)) => Settings::from_name(&way).ok_or_else(|| {
                        format!("format.settings must be internal-with-overrides, as-written, or internal-only, not `{way}`")
                    })?,
                    Some(_) => {
                        return Err(SETTINGS_TABLE.into());
                    }
                };
                if self.plugins.is_some() && formatter != KnownFormatter::Prettier {
                    return Err(format!(
                        "format.plugins needs format.formatter = \"prettier\", not `{name}`"
                    ));
                }
                let plugins = self.plugins.unwrap_or_default();
                Some(FormatRule::Formatter {
                    formatter,
                    config,
                    settings,
                    plugins,
                })
            }
            (None, command) => {
                if self.settings.as_ref().is_some_and(toml::Value::is_table) {
                    return Err(SETTINGS_TABLE.into());
                }
                if self.settings.is_some() || self.plugins.is_some() {
                    return Err("format.settings and format.plugins need format.formatter".into());
                }
                match command {
                    None if config.is_some() => {
                        return Err("format.config needs format.formatter or format.command".into());
                    }
                    None => None,
                    Some(argv) => Some(command_rule(argv, config)?),
                }
            }
        };
        let timeout = match self.timeout {
            None => FORMAT_TIMEOUT,
            Some(secs) => u64::try_from(secs)
                .ok()
                .filter(|s| *s > 0)
                .map(Duration::from_secs)
                .ok_or("format.timeout must be a whole number of seconds above 0")?,
        };
        let mut same = FormatConfig::default().same;
        for name in self.same.unwrap_or_default() {
            match Named::from_name(&name).filter(|n| COUNTED.contains(n)) {
                Some(named) => same.insert(named),
                None => {
                    return Err(format!(
                        "format.same can name only yaml11-value, not `{name}`"
                    ));
                }
            }
        }
        Ok(FormatConfig {
            rule,
            detect: self.detect.unwrap_or(false),
            timeout,
            same,
        })
    }
}

/// `format_config` returns the `[format]` table of `/.sokf/config.toml` at
/// `root`, or the default where the file is absent, doesn't resolve, or
/// names a file that leaves the repository, for the version of the store of
/// the cache, which a run reads before the configuration, and whose run then
/// stops on such a configuration, so that the comparer, which the cache
/// builds, never reads a file that the check refuses.
pub fn format_config(root: &crate::RepoRoot) -> FormatConfig {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        format: FormatSection,
    }
    std::fs::read_to_string(super::config_path().to_fs(root))
        .ok()
        .and_then(|text| toml::from_str::<toml::Table>(&text).ok())
        .and_then(|mut table| {
            let format = table
                .remove("format")
                .unwrap_or(toml::Value::Table(toml::Table::new()));
            let mut file = toml::Table::new();
            file.insert("format".into(), format);
            File::deserialize(toml::Value::Table(file)).ok()
        })
        .and_then(|file| file.format.resolve().ok())
        .and_then(|format| format.check_files(root).ok())
        .unwrap_or_default()
}

impl FormatConfig {
    /// `check_files` returns the table where each file that it names stays
    /// in the repository at `root`: the configuration file of
    /// `format.config`, which must be a file of the repository, a program of
    /// `format.command` named by a relative path, and a plugin of
    /// `format.plugins` named by a path, and the message of the fault
    /// otherwise, where a part of such a path is a symbolic link out of the
    /// repository or its canonical form lies outside the canonical root
    /// (CONF-10_AC-25).
    pub(super) fn check_files(self, root: &crate::RepoRoot) -> Result<FormatConfig, String> {
        let root = root.as_path();
        let (config, program, plugins): (Option<&str>, Option<&str>, &[String]) = match &self.rule {
            Some(FormatRule::Formatter {
                config, plugins, ..
            }) => (config.as_deref(), None, plugins),
            Some(FormatRule::Command { argv, config }) => (
                config.as_deref(),
                argv.first()
                    .map(String::as_str)
                    .filter(|p| p.contains(['/', '\\']) && !absolute(p)),
                &[],
            ),
            None => (None, None, &[]),
        };
        if let Some(file) = config {
            if leaves(root, file) {
                return Err(format!(
                    "format.config `{file}` lies outside the repository"
                ));
            }
            if !root.join(file).is_file() {
                return Err(format!(
                    "format.config `{file}` is no file of the repository"
                ));
            }
        }
        if let Some(program) = program.filter(|p| leaves(root, p)) {
            return Err(format!(
                "format.command's program `{program}` lies outside the repository"
            ));
        }
        if let Some(plugin) = plugins
            .iter()
            .find(|p| climbs(p) || (names_a_path(p) && leaves(root, p)))
        {
            return Err(format!(
                "format.plugins `{plugin}` lies outside the repository"
            ));
        }
        Ok(self)
    }
}

/// `names_a_path` returns whether the plugin `plugin` is named by a path,
/// absolute or from the root, as same-same takes it, and not by a package's
/// name.
fn names_a_path(plugin: &str) -> bool {
    absolute(plugin)
        || ["./", "../", ".\\", "..\\"]
            .iter()
            .any(|start| plugin.starts_with(start))
}

/// `absolute` returns whether `path` is absolute on any platform: with a
/// leading `/` or `\`, or absolute on this one, such as `C:\x`.
fn absolute(path: &str) -> bool {
    path.starts_with(['/', '\\']) || Path::new(path).is_absolute()
}

/// `climbs` returns whether a part of `path`, between `/` or `\`, is `..`,
/// which same-same joins onto a folder of the repository, such as
/// `node_modules`, for a package's name too, and which Windows resolves by
/// the path's text, whether the folders before it exist or not.
fn climbs(path: &str) -> bool {
    path.split(['/', '\\']).any(|part| part == "..")
}

/// `leaves` returns whether the path `path`, from `root` where it is
/// relative, leaves the repository: where a part of it is `..`, or where a
/// part of it that exists, a symbolic link among them, has a canonical form
/// outside the canonical root.
fn leaves(root: &Path, path: &str) -> bool {
    let Ok(base) = fs::canonicalize(root) else {
        return false;
    };
    if climbs(path) {
        return true;
    }
    let given = Path::new(path);
    if absolute(path) {
        return fs::canonicalize(given).map_or_else(
            |_| !given.starts_with(root),
            |real| !real.starts_with(&base),
        );
    }
    let mut at = root.to_path_buf();
    for part in given.components() {
        match part {
            Component::CurDir => continue,
            Component::Normal(name) => at.push(name),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return true,
        }
        match fs::canonicalize(&at) {
            Ok(real) if !real.starts_with(&base) => return true,
            Ok(_) => {}
            // No part below one that doesn't exist exists either.
            Err(_) => return false,
        }
    }
    false
}

/// `command_rule` checks a command and returns it as a rule: it names a
/// program and `{file}` or `{files}`, not both, and names `{settings}` only
/// with a configuration file.
fn command_rule(argv: Vec<String>, config: Option<String>) -> Result<FormatRule, String> {
    let files = argv.iter().skip(1).any(|a| a == "{files}");
    let file = argv.iter().skip(1).any(|a| a.contains("{file}"));
    if argv.first().is_none_or(String::is_empty) || !(files || file) {
        return Err(
            "format.command must name a program and have an element `{files}` or `{file}`".into(),
        );
    }
    if files && file {
        return Err("format.command names both `{file}` and `{files}`".into());
    }
    if config.is_none() && argv.iter().any(|a| a.contains("{settings}")) {
        return Err("format.command names `{settings}`, which needs format.config".into());
    }
    Ok(FormatRule::Command { argv, config })
}

/// `config_path` returns the path of a configuration file from the
/// repository root, which `config` gives relative to it, or with a leading
/// `/`, and which stays inside the repository.
fn config_path(config: &str) -> Result<String, String> {
    let path = config.trim_start_matches('/');
    let inside = !path.is_empty()
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    if inside {
        Ok(path.to_string())
    } else {
        Err(format!(
            "format.config must be a path inside the repository, not `{config}`"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<FormatConfig, String> {
        #[derive(Deserialize)]
        struct File {
            format: FormatSection,
        }
        toml::from_str::<File>(text)
            .map_err(|e| e.to_string())?
            .format
            .resolve()
    }

    // `[format]` names a formatter with its configuration and its way, or
    // a command, a limit in whole seconds, and the named things that count
    // as the same.
    // sokf-tests: CONF-10_AC-4
    // sokf-tests: CONF-10_AC-20
    // sokf-tests: CONF-10_AC-21
    // sokf-tests: CONF-10_AC-23
    // sokf-tests: CONF-10_AC-24
    // sokf-tests: CONF-10_AC-25
    // sokf-tests: CONF-10_AC-26
    #[test]
    fn reads_the_format_table() {
        for text in [
            "[format]\ncommand = []\n",
            "[format]\ncommand = [\"prettier\", \"--write\"]\n",
            "[format]\ncommand = [\"\", \"{files}\"]\n",
            "[format]\ncommand = \"prettier {files}\"\n",
            "[format]\ncommand = [\"fmt\", \"--config\", \"{settings}\", \"{files}\"]\n",
            "[format]\nwidth = 3\n",
            "[format]\ntimeout = 0\n",
            "[format]\ntimeout = -1\n",
            "[format]\ntimeout = 1.5\n",
            "[format]\nformatter = \"rustfmt\"\n",
            "[format]\nformatter = \"prettier\"\ncommand = [\"fmt\", \"{files}\"]\n",
            "[format]\nformatter = \"prettier\"\nsettings = \"sorted\"\n",
            "[format]\nformatter = \"prettier\"\nsettings = { printWidth = 80 }\n",
            "[format]\nformatter = \"oxfmt\"\nplugins = [\"x\"]\n",
            "[format]\nformatter = \"biome\"\nplugins = []\n",
            "[format]\nsettings = \"as-written\"\n",
            "[format]\nconfig = \".prettierrc.json\"\n",
            "[format]\nformatter = \"prettier\"\nconfig = \"../x.json\"\n",
            "[format]\nsame = [\"key-order\"]\n",
            "[format]\nsame = [\"region-indent\"]\n",
            "[format]\nsame = [\"yaml-1.1\"]\n",
            "[format]\ncommand = [\"fmt\", \"{file}\", \"{files}\"]\n",
            "[format]\nformatter = \"prettier\"\nconfig = \"C:/x/.prettierrc.json\"\n",
        ] {
            assert!(read(text).is_err(), "{text:?}");
        }
        let config = read(
            "[format]\nformatter = \"prettier\"\nconfig = \"/.prettierrc.json\"\nplugins = [\"prettier-plugin-sort-json\"]\n",
        )
        .unwrap();
        assert_eq!(
            config.rule,
            Some(FormatRule::Formatter {
                formatter: KnownFormatter::Prettier,
                config: Some(".prettierrc.json".into()),
                settings: Settings::InternalWithOverrides,
                plugins: vec!["prettier-plugin-sort-json".into()],
            })
        );
        assert_eq!(config.timeout, FORMAT_TIMEOUT);
        let config =
            read("[format]\nformatter = \"biome\"\nsettings = \"internal-only\"\n").unwrap();
        assert!(matches!(
            config.rule,
            Some(FormatRule::Formatter {
                settings: Settings::InternalOnly,
                ..
            })
        ));
        let config = read(
            "[format]\ncommand = [\"stylelint\", \"--config\", \"{settings}\", \"--fix\", \"{files}\"]\nconfig = \".stylelintrc.json\"\ntimeout = 3\n",
        )
        .unwrap();
        assert!(matches!(config.rule, Some(FormatRule::Command { .. })));
        assert_eq!(config.timeout, Duration::from_secs(3));
        assert!(read("[format]\ncommand = [\"fmt\", \"--input={file}\"]\n").is_ok());
        let config = read("[format]\nsame = [\"yaml11-value\"]\n").unwrap();
        assert!(config.same.contains(Named::Yaml11Value));
        assert!(config.same.contains(Named::KeyOrder));
        let default = read("[format]\n").unwrap();
        assert_eq!(default, FormatConfig::default());
        assert!(!default.detect, "detection is off by default");
        assert!(read("[format]\ndetect = true\n").unwrap().detect);
        assert!(read("[format]\ndetect = \"yes\"\n").is_err());
        let table = read(
            "[format]\ncommand = [\"fmt\", \"{files}\"]\nsettings = { proseWrap = \"never\" }\n",
        );
        assert!(
            table
                .unwrap_err()
                .ends_with("name that file with format.config")
        );
        assert!(!default.same.contains(Named::Yaml11Value));
        assert!(!default.same.contains(Named::RegionIndent));
    }

    // A configuration file of `[format]` that the repository doesn't hold
    // stops the run.
    // sokf-tests: CONF-10_AC-25
    #[test]
    fn the_configuration_file_is_a_file_of_the_repository() {
        let dir = crate::test_support::fresh_dir("format-config");
        let root = crate::RepoRoot::new(dir.clone());
        let config =
            read("[format]\nformatter = \"prettier\"\nconfig = \".prettierrc.json\"\n").unwrap();
        assert!(config.clone().check_files(&root).is_err());
        crate::test_support::write(&dir, ".prettierrc.json", "{}\n");
        assert_eq!(config.clone().check_files(&root), Ok(config));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // A file of `[format]` that leaves the repository through a symbolic
    // link stops the run: a link to a file and a link to a folder outside
    // it, for `config`, a program of `command`, and a plugin, while a link
    // inside the repository is a file of it.
    // sokf-tests: CONF-10_AC-25
    #[cfg(unix)]
    #[test]
    fn a_file_of_the_table_stays_in_the_repository() {
        use std::os::unix::fs::symlink;
        let outside = crate::test_support::fresh_dir("format-outside");
        crate::test_support::write(&outside, "fmt.json", "{}\n");
        crate::test_support::write(&outside, "bin/fmt.sh", "#!/bin/sh\n");
        crate::test_support::write(&outside, "plugin.js", "\n");
        let dir = crate::test_support::fresh_dir("format-links");
        let root = crate::RepoRoot::new(dir.clone());
        symlink(outside.join("fmt.json"), dir.join("fmt.json")).unwrap();
        symlink(&outside, dir.join("out")).unwrap();
        crate::test_support::write(&dir, "conf/inside.json", "{}\n");
        symlink(dir.join("conf/inside.json"), dir.join("inside.json")).unwrap();
        let check = |text: &str| read(text).unwrap().check_files(&root);
        for text in [
            "[format]\ncommand = [\"fmt\", \"{settings}\", \"{files}\"]\nconfig = \"fmt.json\"\n",
            "[format]\nformatter = \"oxfmt\"\nconfig = \"out/fmt.json\"\n",
            "[format]\ncommand = [\"./out/bin/fmt.sh\", \"{files}\"]\n",
            "[format]\ncommand = [\"../fmt.sh\", \"{files}\"]\n",
            "[format]\nformatter = \"prettier\"\nplugins = [\"./out/plugin.js\"]\n",
        ] {
            let error = check(text).expect_err(text);
            assert!(
                error.contains("lies outside the repository"),
                "{text}: {error}"
            );
        }
        for text in [
            "[format]\nformatter = \"oxfmt\"\nconfig = \"inside.json\"\n",
            "[format]\ncommand = [\"./conf/fmt.sh\", \"{files}\"]\n",
            "[format]\nformatter = \"prettier\"\nplugins = [\"prettier-plugin-sort-json\"]\n",
        ] {
            assert!(check(text).is_ok(), "{text}");
        }
        // The configuration that the cache reads drops such a table, so that
        // the comparer never reads the file.
        crate::test_support::write(
            &dir,
            ".sokf/config.toml",
            "[format]\ncommand = [\"fmt\", \"{settings}\", \"{files}\"]\nconfig = \"fmt.json\"\n",
        );
        assert_eq!(format_config(&root), FormatConfig::default());
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }
}
