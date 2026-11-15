//! This module implements the `sokf info` command (CLI-5).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    Cache, CacheMode, Error, RepoRoot, Result, config,
    grammar::{self, FormatRegistry, RegexCache, TypeDef},
    graph::yaml_json,
    model::SchemaRef,
};

/// `InfoLayer` lists the values of the `--layer` option of `sokf info
/// formats` (CLI-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoLayer {
    /// `Embedded` selects the formats embedded in the tool, without those of
    /// the repository.
    Embedded,
    /// `Repo` selects the formats of `/.sokf/formats.yaml`, without those of
    /// the tool.
    Repo,
    /// `FormatsD` selects the formats of the files of `/.sokf/formats.d/`,
    /// without those of the tool and of `/.sokf/formats.yaml`.
    FormatsD,
    /// `Effective` selects the formats of the tool, with the layer `formats.d`
    /// and then the repository's formats merged on top, where a nearer layer's
    /// format replaces a farther layer's format of the same name.
    Effective,
}

/// `info_grammar` returns the schema language verbatim: the tramlines core
/// grammar, then the tramlines-md grammar, and then the schema extension of
/// SOKF.
// sokf-implements: CLI-5_AC-1
// sokf-implements: CLI-5_AC-2
// sokf-implements: CLI-5_AC-12
pub fn info_grammar() -> String {
    grammar::grammar_text()
}

/// The registry of `sokf info formats --json`: the types and the formats of
/// the registry that the command prints as YAML, in the shape of a formats
/// file, each entry with its source (CLI-5_AC-38).
// `FormatsJson` is a wire type: the `output_schema` module generates
// `info-formats-registry.schema.json` from it.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(title = "sokf info formats registry")]
pub struct FormatsJson {
    /// The placeholder types, by name, an empty map where the registry
    /// declares none.
    pub types: BTreeMap<String, FormatType>,
    /// The line formats, by name, an empty map where the registry holds
    /// none.
    pub registry: BTreeMap<String, FormatEntry>,
}

/// One placeholder type of the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct FormatType {
    /// What the type matches: a regular expression, or a list of literal
    /// choices.
    #[serde(rename = "match")]
    pub matches: TypeMatch,
    /// Where the type comes from: `built-in`, `embedded`, `repo`, the path
    /// of a file of `/.sokf/formats.d/`, or the id of a schema.
    pub source: String,
}

/// What a placeholder type matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum TypeMatch {
    /// A regular expression with no anchors and no capture groups.
    Regex(String),
    /// Literal choices.
    Choices(Vec<String>),
}

/// One line format of the registry: its keys as a formats file writes
/// them, which the grammar `format` of `sokf info grammar` defines, such as
/// `template`, `example`, and `variants`, and its source.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct FormatEntry {
    /// The keys of the format as its formats file writes them, with a patch
    /// merged onto the entry that it patches.
    #[serde(flatten)]
    pub entry: serde_json::Map<String, serde_json::Value>,
    /// Where the format comes from: `embedded`, `repo`, the path of a file
    /// of `/.sokf/formats.d/`, or the id of a schema.
    pub source: String,
}

/// `formats_json` returns `registry` as the object of `--json`, with the
/// built-in types where `built_ins` asks for them, as the YAML has them.
// sokf-implements: CLI-5_AC-38
// sokf-implements: CLI-5_AC-39
fn formats_json(registry: &FormatRegistry, built_ins: bool) -> FormatsJson {
    let types = if built_ins {
        registry.types_seen()
    } else {
        registry.types.clone()
    };
    FormatsJson {
        types: types
            .entries
            .iter()
            .map(|(name, (def, source))| {
                let matches = match def {
                    TypeDef::Regex(r) => TypeMatch::Regex(r.clone()),
                    TypeDef::Choices(c) => TypeMatch::Choices(c.clone()),
                };
                let source = source.label().into_owned();
                (name.clone(), FormatType { matches, source })
            })
            .collect(),
        registry: registry
            .entries
            .iter()
            .map(|(name, (format, source))| {
                let mut entry = match yaml_json(&format.raw) {
                    serde_json::Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };
                // The source replaces any key of that name, as in the YAML,
                // so that the object holds the key once.
                entry.retain(|key, _| key != "source");
                let source = source.label().into_owned();
                (name.clone(), FormatEntry { entry, source })
            })
            .collect(),
    }
}

/// `info_formats` returns a format registry as YAML, from `registry`.
// sokf-implements: CLI-5_AC-3
// sokf-implements: CLI-5_AC-13
pub fn info_formats(
    root: Option<&RepoRoot>,
    layer: InfoLayer,
    schema: Option<&str>,
    cache: CacheMode,
) -> Result<String> {
    let (formats, built_ins) = registry(root, layer, schema, cache)?;
    Ok(formats.to_yaml(built_ins))
}

/// `info_formats_json` returns the format registry that `info_formats`
/// prints as YAML, as the object of `--json`.
pub fn info_formats_json(
    root: Option<&RepoRoot>,
    layer: InfoLayer,
    schema: Option<&str>,
    cache: CacheMode,
) -> Result<FormatsJson> {
    let (formats, built_ins) = registry(root, layer, schema, cache)?;
    Ok(formats_json(&formats, built_ins))
}

/// `registry` returns a format registry, and whether its view prints the
/// built-in types, which every view but the `Repo` layer does. When the
/// caller gives `schema`, the registry holds the formats of that schema, and
/// `registry` ignores `layer`. Each entry records the source it comes from.
/// A call for any layer except `Embedded` needs `root`. A call that gives
/// `schema` also needs `root`, because `registry` reads the knowledge
/// through the cache to find the schema.
// sokf-implements: CLI-5_AC-6
// sokf-implements: CLI-5_AC-15
// sokf-implements: CACHE-1_AC-27
// sokf-implements: CLI-5_AC-4
// sokf-implements: CLI-5_AC-5
// sokf-implements: CLI-5_AC-7
// sokf-implements: CLI-5_AC-8
// sokf-implements: CLI-5_AC-10
// sokf-implements: CLI-5_AC-11
// sokf-implements: CLI-5_AC-14
fn registry(
    root: Option<&RepoRoot>,
    layer: InfoLayer,
    schema: Option<&str>,
    cache: CacheMode,
) -> Result<(FormatRegistry, bool)> {
    let grammar = grammar::load_grammar()?;
    let regexes = RegexCache::new();
    if let Some(id) = schema {
        let root = root.ok_or(Error::NoRepository)?;
        let config = config::load_config(root)?;
        let repo = config::load_repo_formats(root, &grammar, &regexes)?;
        let cache = Cache::open(root, cache);
        let mut model = super::load_for_lookup(root, &config, &cache)?;
        super::admit_schema_by_id(&mut model, root, &cache, id)?;
        let compiled = grammar::compile_all(&model, grammar, &repo);
        let found: Vec<SchemaRef> = model
            .knowledges
            .iter()
            .enumerate()
            .filter_map(|(k, knowledge)| {
                knowledge.schema_by_id(id).map(|i| SchemaRef {
                    knowledge: k,
                    index: i,
                })
            })
            .collect();
        let roots = |ks: &mut dyn Iterator<Item = usize>| -> String {
            ks.map(|k| model.knowledges[k].root.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let r = match found.as_slice() {
            [] => {
                return Err(Error::Info(format!(
                    "schema `{id}` is not in any root. The roots are {}",
                    roots(&mut (0..model.knowledges.len()))
                )));
            }
            [one] => *one,
            many => {
                return Err(Error::Info(format!(
                    "schema `{id}` is declared in more than one root: {}",
                    roots(&mut many.iter().map(|r| r.knowledge))
                )));
            }
        };
        let Some(schema) = compiled.schema(r) else {
            return Err(Error::Info(format!(
                "schema `{id}` does not match the grammar. Run `sokf validate` to see the errors"
            )));
        };
        return Ok((schema.core.core.formats.clone(), true));
    }
    let embedded = grammar::embedded_formats(&grammar, &regexes)?;
    match layer {
        InfoLayer::Embedded => Ok((embedded, true)),
        InfoLayer::Repo => {
            let root = root.ok_or(Error::NoRepository)?;
            match config::load_repo_formats(root, &grammar, &regexes)?.repo {
                Some(repo) => Ok((repo, false)),
                None => Err(Error::Info(
                    "the repository has no /.sokf/formats.yaml".into(),
                )),
            }
        }
        // sokf-implements: CLI-5_AC-42
        // sokf-implements: CLI-5_AC-43
        InfoLayer::FormatsD => {
            let root = root.ok_or(Error::NoRepository)?;
            match config::load_repo_formats(root, &grammar, &regexes)?.folder_layer() {
                Some(folder) => Ok((folder, false)),
                None => Err(Error::Info(format!(
                    "/{}/ holds no file *.yaml",
                    config::FORMATS_DIR
                ))),
            }
        }
        InfoLayer::Effective => {
            let root = root.ok_or(Error::NoRepository)?;
            let formats = config::load_repo_formats(root, &grammar, &regexes)?;
            Ok((formats.effective(&embedded), true))
        }
    }
}
