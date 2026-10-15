//! This module implements the `sokf info` command (CLI-5).

use crate::{
    Cache, CacheMode, Error, RepoRoot, Result, config,
    grammar::{self, RegexCache},
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
    /// `Effective` selects the formats of the tool merged with those of the
    /// repository, where a format of the repository replaces the tool's
    /// format of the same name.
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

/// `info_formats` returns a format registry as YAML. When the caller gives
/// `schema`, the registry holds the formats of that schema, and
/// `info_formats` ignores `layer`. Each entry records the source it comes
/// from, except in the `Repo` layer. A call for any layer except `Embedded`
/// needs `root`. A call that gives `schema` also needs `root`, because
/// `info_formats` reads the knowledge through the cache to find the schema.
// sokf-implements: CLI-5_AC-6
// sokf-implements: CLI-5_AC-15
// sokf-implements: CACHE-1_AC-27
// sokf-implements: CLI-5_AC-3
// sokf-implements: CLI-5_AC-4
// sokf-implements: CLI-5_AC-5
// sokf-implements: CLI-5_AC-7
// sokf-implements: CLI-5_AC-8
// sokf-implements: CLI-5_AC-10
// sokf-implements: CLI-5_AC-11
// sokf-implements: CLI-5_AC-13
// sokf-implements: CLI-5_AC-14
pub fn info_formats(
    root: Option<&RepoRoot>,
    layer: InfoLayer,
    schema: Option<&str>,
    cache: CacheMode,
) -> Result<String> {
    let grammar = grammar::load_grammar()?;
    let regexes = RegexCache::new();
    if let Some(id) = schema {
        let root = root.ok_or(Error::NoRepository)?;
        let config = config::load_config(root)?;
        let repo = config::load_repo_formats(root, &grammar, &regexes)?;
        let cache = Cache::open(root, cache);
        let mut model = super::load_for_lookup(root, &config, &cache)?;
        super::admit_schema_by_id(&mut model, root, &cache, id)?;
        let compiled = grammar::compile_all(&model, grammar, repo);
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
        return Ok(schema.core.core.formats.to_yaml(true));
    }
    let embedded = grammar::embedded_formats(&grammar, &regexes)?;
    match layer {
        InfoLayer::Embedded => Ok(embedded.to_yaml(true)),
        InfoLayer::Repo => {
            let root = root.ok_or(Error::NoRepository)?;
            match config::load_repo_formats(root, &grammar, &regexes)? {
                Some(repo) => Ok(repo.to_yaml(false)),
                None => Err(Error::Info(
                    "the repository has no /.sokf/formats.yaml".into(),
                )),
            }
        }
        InfoLayer::Effective => {
            let root = root.ok_or(Error::NoRepository)?;
            let repo = config::load_repo_formats(root, &grammar, &regexes)?;
            Ok(match repo {
                Some(repo) => embedded.layered(&repo).to_yaml(true),
                None => embedded.to_yaml(true),
            })
        }
    }
}
