//! This module finds the formatter of a repository: the command that
//! `[format]` names, or the formatter whose configuration file lies at the
//! repository root, with sokf's own command and settings for it (CONF-10).

use std::{fs, path::Path};

use crate::{RepoPath, RepoRoot, config::FormatConfig};

/// A `Preset` is a formatter that sokf detects, with the command and the
/// settings that sokf runs it with.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    /// The formatter's name, as a warning names it.
    pub name: &'static str,
    /// The program, which sokf finds in `node_modules/.bin` or on the path.
    pub program: &'static str,
    /// The npm package whose `package.json` gives the formatter's version.
    pub package: &'static str,
    /// The configuration files that reveal the formatter, at the repository
    /// root. A name that ends with `*` matches each name that starts with
    /// the rest.
    pub config_files: &'static [&'static str],
    /// The arguments, in which `{files}` stands for the files to format,
    /// `{settings}` for the path of the settings, and `{empty}` for the path
    /// of an empty file.
    pub args: &'static [&'static str],
    /// The name of the file that holds the settings.
    pub settings_file: &'static str,
    /// sokf's settings, as JSON.
    pub settings: &'static str,
    /// The extensions of the languages that the formatter formats, or
    /// `None` for every extension.
    pub extensions: Option<&'static [&'static str]>,
}

/// `PRETTIER_EXTENSIONS` lists the extensions of the languages that Prettier
/// formats without a plugin.
const PRETTIER_EXTENSIONS: &[&str] = &[
    "js",
    "jsx",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "mts",
    "cts",
    "json",
    "jsonc",
    "json5",
    "yaml",
    "yml",
    "md",
    "markdown",
    "mdx",
    "css",
    "scss",
    "less",
    "html",
    "htm",
    "vue",
    "graphql",
    "gql",
    "hbs",
    "handlebars",
];

/// `OXFMT_EXTENSIONS` lists the extensions of the languages that oxfmt
/// formats: those of Prettier, and TOML.
const OXFMT_EXTENSIONS: &[&str] = &[
    "js",
    "jsx",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "mts",
    "cts",
    "json",
    "jsonc",
    "json5",
    "yaml",
    "yml",
    "toml",
    "md",
    "markdown",
    "mdx",
    "css",
    "scss",
    "less",
    "html",
    "htm",
    "vue",
    "graphql",
    "gql",
    "hbs",
    "handlebars",
];

// sokf:begin presets
/// `PRESETS` lists the formatters that sokf detects, in the order in which
/// it looks for them, each with the command and the settings that sokf runs
/// it with. The settings choose one layout for each
/// choice that the formatter leaves open: the widest line, objects and
/// arrays on one line where they fit, double quotes, and prose that isn't
/// wrapped. Each command passes the settings explicitly and turns off the
/// project's ignore files.
// sokf-implements: CONF-10_AC-2
// sokf-implements: CONF-10_AC-5
pub const PRESETS: [Preset; 4] = [
    Preset {
        name: "oxfmt",
        program: "oxfmt",
        package: "oxfmt",
        config_files: &[".oxfmtrc.json", ".oxfmtrc.jsonc", "oxfmt.config.*"],
        args: &[
            "--write",
            "--config={settings}",
            "--ignore-path={empty}",
            "--disable-nested-config",
            "--no-error-on-unmatched-pattern",
            "{files}",
        ],
        settings_file: ".oxfmtrc.json",
        settings: r#"{"printWidth": 320, "tabWidth": 2, "useTabs": false, "endOfLine": "lf", "semi": true, "singleQuote": false, "jsxSingleQuote": false, "quoteProps": "as-needed", "trailingComma": "all", "bracketSpacing": true, "bracketSameLine": false, "objectWrap": "collapse", "arrowParens": "always", "proseWrap": "never", "htmlWhitespaceSensitivity": "css", "embeddedLanguageFormatting": "auto", "singleAttributePerLine": false, "vueIndentScriptAndStyle": false, "insertFinalNewline": true, "ignorePatterns": []}"#,
        extensions: Some(OXFMT_EXTENSIONS),
    },
    Preset {
        name: "Prettier",
        program: "prettier",
        package: "prettier",
        config_files: &[".prettierrc*", "prettier.config.*"],
        args: &[
            "--write",
            "--config={settings}",
            "--ignore-path={empty}",
            "--no-editorconfig",
            "--ignore-unknown",
            "--log-level=warn",
            "{files}",
        ],
        settings_file: ".prettierrc.json",
        settings: r#"{"printWidth": 320, "tabWidth": 2, "useTabs": false, "endOfLine": "lf", "semi": true, "singleQuote": false, "jsxSingleQuote": false, "quoteProps": "as-needed", "trailingComma": "all", "bracketSpacing": true, "bracketSameLine": false, "objectWrap": "collapse", "arrowParens": "always", "proseWrap": "never", "htmlWhitespaceSensitivity": "css", "embeddedLanguageFormatting": "auto", "singleAttributePerLine": false, "vueIndentScriptAndStyle": false, "experimentalTernaries": false}"#,
        extensions: Some(PRETTIER_EXTENSIONS),
    },
    Preset {
        name: "Biome",
        program: "biome",
        package: "@biomejs/biome",
        config_files: &["biome.json", "biome.jsonc"],
        args: &[
            "format",
            "--write",
            "--config-path={settings}",
            "--vcs-enabled=false",
            "--files-ignore-unknown=true",
            "--no-errors-on-unmatched",
            "--colors=off",
            "{files}",
        ],
        settings_file: "biome.json",
        settings: r#"{"vcs": {"enabled": false}, "files": {"ignoreUnknown": true}, "assist": {"enabled": false}, "linter": {"enabled": false}, "formatter": {"enabled": true, "indentStyle": "space", "indentWidth": 2, "lineWidth": 320, "lineEnding": "lf", "bracketSpacing": true, "bracketSameLine": false, "attributePosition": "auto", "expand": "never"}, "javascript": {"formatter": {"quoteStyle": "double", "jsxQuoteStyle": "double", "quoteProperties": "asNeeded", "trailingCommas": "all", "semicolons": "always", "arrowParentheses": "always"}}, "json": {"formatter": {"trailingCommas": "none"}}, "css": {"formatter": {"quoteStyle": "double"}}}"#,
        extensions: None,
    },
    Preset {
        name: "dprint",
        program: "dprint",
        package: "dprint",
        config_files: &[
            "dprint.json",
            ".dprint.json",
            "dprint.jsonc",
            ".dprint.jsonc",
        ],
        args: &[
            "fmt",
            "--config={settings}",
            "--allow-no-files",
            "--incremental=false",
            "--no-gitignore",
            "--log-level=warn",
            "--includes-override",
            "{files}",
        ],
        settings_file: "dprint.json",
        settings: r#"{"lineWidth": 320, "indentWidth": 2, "useTabs": false, "newLineKind": "lf"}"#,
        extensions: None,
    },
];

/// `DPRINT_PLUGINS` holds sokf's settings for each plugin of dprint that it
/// knows, by the plugin's name. dprint refuses the settings of a plugin
/// that its configuration doesn't load, so sokf adds only these.
const DPRINT_PLUGINS: [(&str, &str); 3] = [
    (
        "json",
        r#"{"preferSingleLine": true, "trailingCommas": "never"}"#,
    ),
    ("markdown", r#"{"textWrap": "never"}"#),
    (
        "typescript",
        r#"{"quoteStyle": "alwaysDouble", "preferSingleLine": true, "semiColons": "always", "trailingCommas": "onlyMultiLine"}"#,
    ),
];
// sokf:end presets

/// A `Formatter` is the formatter of a run: its program and arguments, the
/// settings that sokf passes it, and where the repository names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatter {
    /// The formatter's name, as a warning names it.
    pub name: String,
    /// The program as the command names it.
    pub program: String,
    /// The arguments, with their placeholders.
    pub args: Vec<String>,
    /// The settings, as JSON.
    pub settings: String,
    /// The name of the file that holds the settings.
    pub settings_file: String,
    /// The npm package whose `package.json` gives the version, for a
    /// detected formatter.
    pub package: Option<String>,
    /// The extensions that the formatter formats, or `None` for every one.
    pub extensions: Option<&'static [&'static str]>,
    /// The file that names the formatter: `/.sokf/config.toml` or the
    /// formatter's own configuration file.
    pub origin: RepoPath,
}

impl Formatter {
    /// `formats` returns whether the formatter formats files with the
    /// extension `ext`.
    pub fn formats(&self, ext: &str) -> bool {
        !ext.is_empty() && self.extensions.is_none_or(|list| list.contains(&ext))
    }

    /// `identity` returns the text that names the formatter in the key of a
    /// formatted text: the program, the arguments, and the settings.
    pub fn identity(&self) -> String {
        format!(
            "{}\0{}\0{}",
            self.program,
            self.args.join("\0"),
            self.settings
        )
    }
}

/// `find` returns the formatter of the repository at `root`: the command of
/// `config`, or else the first preset whose configuration file lies at the
/// root, with the settings of `config` in place of sokf's own when it has
/// them. It returns `None` when the repository names no formatter.
// sokf-implements: CONF-10_AC-1
// sokf-implements: CONF-10_AC-2
// sokf-implements: CONF-10_AC-3
pub fn find(root: &RepoRoot, config: &FormatConfig) -> Option<Formatter> {
    if let Some(command) = &config.command {
        let (program, args) = command.split_first()?;
        return Some(Formatter {
            name: program.clone(),
            program: program.clone(),
            args: args.to_vec(),
            settings: config.settings.clone().unwrap_or_else(|| "{}".into()),
            settings_file: "settings.json".into(),
            package: None,
            extensions: None,
            origin: crate::config::config_path(),
        });
    }
    let names: Vec<String> = fs::read_dir(root.as_path())
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    let (preset, origin) = PRESETS.iter().find_map(|p| {
        let found = names
            .iter()
            .find(|n| p.config_files.iter().any(|pattern| matches(pattern, n)))
            .cloned()
            .or_else(|| {
                (p.program == "prettier" && has_prettier_key(root.as_path()))
                    .then(|| "package.json".into())
            })?;
        Some((p, found))
    })?;
    let settings = match (&config.settings, preset.program) {
        (Some(s), "dprint") => with_plugins(s, &dprint_plugins(root, &origin)),
        (Some(s), _) => s.clone(),
        (None, "dprint") => dprint_settings(&dprint_plugins(root, &origin)),
        (None, _) => preset.settings.to_string(),
    };
    Some(Formatter {
        name: preset.name.into(),
        program: preset.program.into(),
        args: preset.args.iter().map(|a| (*a).to_string()).collect(),
        settings,
        settings_file: preset.settings_file.into(),
        package: Some(preset.package.into()),
        extensions: preset.extensions,
        origin: RepoPath::parse(&format!("/{origin}")).unwrap_or_else(crate::config::config_path),
    })
}

/// `matches` returns whether the file name `name` matches `pattern`, a
/// name that can end with `*`.
fn matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == pattern,
    }
}

/// `has_prettier_key` returns whether the `package.json` in `dir` has a
/// top-level `prettier` key.
fn has_prettier_key(dir: &Path) -> bool {
    fs::read_to_string(dir.join("package.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .is_some_and(|v| v.get("prettier").is_some())
}

/// `dprint_plugins` returns the plugins of the project's dprint
/// configuration, the file `name` at `root`. A plugin given by a relative
/// path becomes the absolute path from the root, because dprint runs in the
/// temporary directory.
fn dprint_plugins(root: &RepoRoot, name: &str) -> Vec<serde_json::Value> {
    let plugins = fs::read_to_string(root.as_path().join(name))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&strip_jsonc(&t)).ok())
        .and_then(|v| v.get("plugins").and_then(|p| p.as_array()).cloned())
        .unwrap_or_default();
    plugins
        .into_iter()
        .map(|p| match p.as_str() {
            Some(s) if !s.contains(':') && !Path::new(s).is_absolute() => {
                serde_json::Value::String(root.as_path().join(s).to_string_lossy().into_owned())
            }
            _ => p,
        })
        .collect()
}

/// `dprint_settings` returns sokf's settings for dprint with `plugins`: the
/// settings for every plugin, those of each plugin of `plugins` that sokf
/// knows, and the plugins. A project whose configuration names its plugins
/// in another file, by `extends`, gives no plugins here, and sokf then
/// compares byte for byte, as with no formatter.
fn dprint_settings(plugins: &[serde_json::Value]) -> String {
    let mut settings: serde_json::Map<String, serde_json::Value> = PRESETS
        .iter()
        .find(|p| p.program == "dprint")
        .and_then(|p| serde_json::from_str(p.settings).ok())
        .expect("the literal settings of dprint parse");
    for plugin in plugins.iter().filter_map(|p| p.as_str()) {
        let name = plugin_name(plugin);
        if let Some((key, value)) = DPRINT_PLUGINS.iter().find(|(k, _)| *k == name) {
            settings.insert(
                (*key).to_string(),
                serde_json::from_str(value).expect("the literal settings parse"),
            );
        }
    }
    settings.insert("plugins".into(), serde_json::Value::Array(plugins.to_vec()));
    serde_json::Value::Object(settings).to_string()
}

/// `with_plugins` returns `settings`, the settings that `[format]` names,
/// with the project's `plugins` added when the settings name none.
fn with_plugins(settings: &str, plugins: &[serde_json::Value]) -> String {
    match serde_json::from_str::<serde_json::Value>(settings) {
        Ok(serde_json::Value::Object(mut map)) if !map.contains_key("plugins") => {
            map.insert("plugins".into(), serde_json::Value::Array(plugins.to_vec()));
            serde_json::Value::Object(map).to_string()
        }
        _ => settings.to_string(),
    }
}

/// `plugin_name` returns the name of a dprint plugin from its URL or its
/// npm specifier, such as `json` from `npm:@dprint/json@0.26.0` or from
/// `https://plugins.dprint.dev/json-0.19.3.wasm`.
fn plugin_name(plugin: &str) -> &str {
    let last = plugin.rsplit('/').next().unwrap_or(plugin);
    last.split(['@', '-', '.']).next().unwrap_or(last)
}

/// `strip_jsonc` returns `text`, JSON with comments, without its comments
/// and its trailing commas, so that a JSON parser reads it.
fn strip_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                '\\' => out.extend(chars.next()),
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            (',', _) => {
                if !closes_next(chars.clone()) {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// `closes_next` returns whether the next character of `chars` that is
/// neither whitespace nor part of a comment closes an array or an object,
/// so that a comma before it is a trailing comma.
fn closes_next(mut chars: std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            (c, _) if c.is_whitespace() => {}
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            (c, _) => return matches!(c, ']' | '}'),
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fresh_dir, write};

    fn root(tag: &str, files: &[(&str, &str)]) -> RepoRoot {
        let dir = fresh_dir(&format!("preset-{tag}"));
        for (name, text) in files {
            write(&dir, name, text);
        }
        RepoRoot::new(dir)
    }

    // sokf-tests: CONF-10_AC-2
    #[test]
    fn each_formatter_is_detected_from_its_configuration_file() {
        let none = FormatConfig::default();
        let cases = [
            (".oxfmtrc.json", "oxfmt"),
            ("oxfmt.config.mts", "oxfmt"),
            (".prettierrc", "Prettier"),
            (".prettierrc.yaml", "Prettier"),
            ("prettier.config.js", "Prettier"),
            ("biome.json", "Biome"),
            ("biome.jsonc", "Biome"),
            ("dprint.json", "dprint"),
            (".dprint.json", "dprint"),
            ("dprint.jsonc", "dprint"),
        ];
        for (i, (file, name)) in cases.iter().enumerate() {
            let root = root(&format!("detect-{i}"), &[(file, "{}")]);
            let found = find(&root, &none).unwrap_or_else(|| panic!("{file}"));
            assert_eq!(found.name, *name, "{file}");
            assert_eq!(found.origin.as_str(), format!("/{file}"));
        }
        let key = root(
            "key",
            &[("package.json", r#"{"name": "x", "prettier": {}}"#)],
        );
        assert_eq!(find(&key, &none).unwrap().name, "Prettier");
        let plain = root("plain", &[("package.json", r#"{"name": "x"}"#)]);
        assert_eq!(find(&plain, &none), None);
        // oxfmt comes first when two formatters are configured.
        let both = root("both", &[(".prettierrc", "{}"), (".oxfmtrc.json", "{}")]);
        assert_eq!(find(&both, &none).unwrap().name, "oxfmt");
    }

    // sokf-tests: CONF-10_AC-1
    // sokf-tests: CONF-10_AC-3
    #[test]
    fn the_configuration_names_the_command_and_the_settings() {
        let root = root("configured", &[(".oxfmtrc.json", "{}")]);
        let command = FormatConfig {
            command: Some(vec!["fmt".into(), "{files}".into()]),
            ..FormatConfig::default()
        };
        let found = find(&root, &command).unwrap();
        assert_eq!(
            (found.program.as_str(), found.args.as_slice()),
            ("fmt", &["{files}".to_string()][..])
        );
        assert_eq!(found.settings, "{}");
        assert_eq!(found.origin.as_str(), "/.sokf/config.toml");
        assert!(found.formats("rs"));
        let settings = FormatConfig {
            settings: Some(r#"{"printWidth":100}"#.into()),
            ..FormatConfig::default()
        };
        let found = find(&root, &settings).unwrap();
        assert_eq!(found.name, "oxfmt");
        assert_eq!(found.settings, r#"{"printWidth":100}"#);
        assert!(!found.formats("rs") && found.formats("json") && !found.formats(""));
    }

    // dprint gets the plugins of the project's configuration, which can hold
    // comments and trailing commas, and sokf's settings for each plugin that
    // it knows.
    // sokf-tests: CONF-10_AC-2
    #[test]
    fn dprint_gets_the_plugins_of_the_project() {
        let config = "{\n  // the plugins\n  \"plugins\": [\n    \"npm:@dprint/json@0.26.0\", /* json */\n    \"https://plugins.dprint.dev/markdown-0.17.8.wasm\",\n    \"https://plugins.dprint.dev/g-plane/pretty_yaml-v0.5.0.wasm\",\n  ],\n}\n";
        let repo = root("dprint", &[("dprint.json", config)]);
        let found = find(&repo, &FormatConfig::default()).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&found.settings).unwrap();
        assert_eq!(settings["lineWidth"], 320);
        assert_eq!(settings["plugins"].as_array().unwrap().len(), 3);
        assert_eq!(settings["json"]["preferSingleLine"], true);
        assert_eq!(settings["markdown"]["textWrap"], "never");
        assert!(settings.get("typescript").is_none());
        let own = FormatConfig {
            settings: Some(r#"{"lineWidth":80}"#.into()),
            ..FormatConfig::default()
        };
        let found = find(&repo, &own).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&found.settings).unwrap();
        assert_eq!(settings["lineWidth"], 80);
        assert_eq!(settings["plugins"].as_array().unwrap().len(), 3);
        assert_eq!(plugin_name("npm:@dprint/typescript@0.95.0"), "typescript");
        assert_eq!(
            strip_jsonc(r#"{"a": "//x", "b": [1,],}"#),
            r#"{"a": "//x", "b": [1]}"#
        );
        assert_eq!(strip_jsonc("[1, // one\n]"), "[1 \n]");
        assert_eq!(strip_jsonc("[1, /* a */ 2]"), "[1,  2]");
        let relative = root(
            "relative",
            &[("dprint.jsonc", r#"{"plugins": ["./p/x.wasm"]}"#)],
        );
        let found = find(&relative, &FormatConfig::default()).unwrap();
        // The settings are JSON, which escapes each backslash of a Windows
        // path, so the test compares the parsed value, not the text.
        let settings: serde_json::Value = serde_json::from_str(&found.settings).unwrap();
        assert_eq!(
            settings["plugins"][0],
            relative
                .as_path()
                .join("./p/x.wasm")
                .to_string_lossy()
                .as_ref(),
            "{}",
            found.settings
        );
    }
}
