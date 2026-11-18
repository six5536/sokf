//! This module implements `sokf guide` (CLI-17), which prints the guidance that
//! a document's schema gives for the document's parts at the document's
//! lifecycle states. The engine's `derive::guide` returns that guidance, and
//! `sokf new` writes the same words in its scaffold.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use schemars::JsonSchema;
use serde::Serialize;
use tramlines_md::{check::Counts, derive};

use crate::{
    Cache, CacheMode, Document, Error, Model, RepoPath, RepoRoot, Result, config,
    grammar::{self, Label, LimitKind, Section, Shape, SokfSchema, States, Text, squash},
    graph,
    model::{ParseMode, parse_source},
    scaffold, source,
};

/// `GuideOptions` holds the options of `sokf guide`.
#[derive(Debug, Clone)]
pub struct GuideOptions {
    /// `documents` holds the arguments, each an id or a path of a document.
    pub documents: Vec<String>,
    /// `states` holds the values of `--state`, each a key and a state, in the
    /// order of the arguments.
    pub states: Vec<(String, String)>,
    /// `cwd` is the current directory, from which a relative path resolves.
    pub cwd: PathBuf,
    /// `cache` is the cache mode.
    pub cache: CacheMode,
}

// sokf:begin guide-report
/// `GuideReport` is the report of `sokf guide --json`: the guide of each
/// document that an argument names, in the order of the arguments
/// (CLI-17_AC-17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(title = "sokf guide report")]
pub struct GuideReport {
    /// `documents` holds one guide for each argument, in the order of the
    /// arguments.
    pub documents: Vec<DocumentGuide>,
}

/// `DocumentGuide` holds the guidance that a document's schema gives for
/// the parts that the document's lifecycle states admit, in the schema's
/// order (CLI-17_AC-6). Each `text` holds the words of the guidance line
/// that `sokf new` writes before the same part at the same states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DocumentGuide {
    /// `path` is the path of the document.
    pub path: RepoPath,
    /// `id` is the `id` of the document. It is absent when the frontmatter
    /// has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub id: Option<String>,
    /// `type` is the `type` of the document.
    #[serde(rename = "type")]
    pub type_name: String,
    /// `schema` is the path of the schema that governs the document. It is
    /// absent for a schema, which the grammar of the schema language
    /// governs. Each `line` of the guide is a line of this file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "RepoPath")]
    pub schema: Option<RepoPath>,
    /// `states` maps each lifecycle key, in the order of the lifecycle, to
    /// the state at which the guide shows the document: the document's own
    /// state, or the state that `--state` gives.
    #[schemars(with = "BTreeMap<String, String>")]
    pub states: serde_json::Map<String, serde_json::Value>,
    /// `guidance` is the schema's top-level `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `bans` holds the message of each ban of the whole body, in order.
    pub bans: Vec<String>,
    /// `text` holds the words of the guidance line that opens the body: the
    /// schema's guidance, then the message of each ban of the whole body. It
    /// is absent when the schema has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `labels` holds each attribution label that the schema declares, in
    /// the schema's order, once for the whole document (CLI-17_AC-35). It is
    /// empty for a schema without labels.
    pub labels: Vec<LabelGuide>,
    /// `frontmatter` holds each frontmatter key that the schema declares and
    /// the states admit, in the schema's order, then each lifecycle key.
    pub frontmatter: Vec<KeyGuide>,
    /// `links` holds each `links` rule that the states admit, in the schema's
    /// order, except a rule whose hard limit of 0 bans its links.
    pub links: Vec<LinkGuide>,
    /// `preamble` is the part of the body before the first heading, where
    /// the schema declares one that the states admit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "PreambleGuide")]
    pub preamble: Option<PreambleGuide>,
    /// `sections` holds each section that the states admit, in the schema's
    /// order, with each section that declares `parent` after its parent.
    pub sections: Vec<SectionGuide>,
}

/// `LabelGuide` holds one attribution label of the schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct LabelGuide {
    /// `key` is the label's key, `human` or `agent`.
    pub key: String,
    /// `words` are the words that a document writes between the label's
    /// square brackets: sokf's words for the label in the language of the
    /// schema, or else the key.
    pub words: String,
    /// `meaning` is sokf's sentence that says what the label means, in the
    /// language of the schema. It is absent where sokf's texts hold none in
    /// that language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub meaning: Option<String>,
}

/// `KeyGuide` holds the guidance of one frontmatter key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct KeyGuide {
    /// `key` is the key.
    pub key: String,
    /// `lifecycle` is true when the key is a lifecycle key rather than a
    /// declared one.
    pub lifecycle: bool,
    /// `optional` is true when the key can be absent.
    pub optional: bool,
    /// `guidance` is the key's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `limit` is the limit on the elements of the key's list. It is absent
    /// when the key has no `max`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GuideLimit")]
    pub limit: Option<GuideLimit>,
    /// `text` holds the words of the guidance line before the key: sokf's own
    /// text for the key, which is the rule for writing a `description`, then
    /// the key's guidance, then the words that state its limit and the limit's
    /// `when-at-limit`. It is absent when the key has none of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `line` is the line of the key's rule in the schema.
    pub line: usize,
}

/// `LinkGuide` holds the guidance of one `links` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct LinkGuide {
    /// `rel` is the relationship of the links that the rule governs.
    pub rel: String,
    /// `optional` is true when the rule's `min` is 0.
    pub optional: bool,
    /// `limit` is the limit on the links of the rule. It is absent when the
    /// rule has no `max`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GuideLimit")]
    pub limit: Option<GuideLimit>,
    /// `text` holds the words of the guidance line before the link: the
    /// rule's guidance, then the words that state its limit and the limit's
    /// `when-at-limit`. It is absent when the rule has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
}

/// `PreambleGuide` holds the guidance of the preamble.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct PreambleGuide {
    /// `guidance` is the preamble's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `text` holds the words of the guidance line before the preamble.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `blocks` holds each block that the states admit, in order.
    pub blocks: Vec<BlockGuide>,
    /// `line` is the line of the preamble in the schema.
    pub line: usize,
}

/// `SectionGuide` holds the guidance of one section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SectionGuide {
    /// `heading` is the heading as a finding of `validate` names it: its
    /// text, its template or pattern, or `<key>` for a heading that takes
    /// the value of the frontmatter key `key`.
    pub heading: String,
    /// `level` is the level of the heading.
    pub level: u8,
    /// `rule_id` is the section's `rule-id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `parent` is the `rule-id` of the section that this one lies inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub parent: Option<String>,
    /// `optional` is true when the section can be absent at the states.
    pub optional: bool,
    /// `repeat` is true when the heading can occur more than once.
    pub repeat: bool,
    /// `held` is true when the document has a heading at the section's
    /// level whose text has the section's heading shape, or one of its
    /// `renamed-from` texts (CLI-17_AC-15).
    pub held: bool,
    /// `guidance` is the section's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `text` holds the words of the guidance line before the heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `blocks` holds each block of the section's body that the states
    /// admit, in order.
    pub blocks: Vec<BlockGuide>,
    /// `line` is the line of the section's rule in the schema.
    pub line: usize,
}

/// `BlockGuide` holds the guidance of one block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct BlockGuide {
    /// `kind` is the kind of the block, as the grammar names it, such as
    /// `prose` or `bullet-list`.
    pub kind: String,
    /// `rule_id` is the block's `rule-id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `pos` is the block's place in the list that declares it, from 1.
    pub pos: usize,
    /// `optional` is true when the block can be absent at the states.
    pub optional: bool,
    /// `guidance` is the block's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `limits` holds the block's own limit, then the limit of each of its
    /// unit rules and line rules, in the schema's order.
    pub limits: Vec<RuleLimit>,
    /// `labels` holds the block's own `label`, then the `label` of each of its
    /// unit rules that declares one, in the schema's order (CLI-17_AC-37).
    pub labels: Vec<RuleLabel>,
    /// `bans` holds the message of each ban of the block's unit rules, of
    /// the formats that they bind, and of their line rules, each once, in
    /// order.
    pub bans: Vec<String>,
    /// `text` holds the words of the guidance line before the block: its
    /// guidance, then the words that state each limit and the limit's
    /// `when-at-limit`, then the message of each ban. It is absent when the
    /// block has none of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `blocks` holds the blocks of a group, and the blocks inside the items
    /// of a list, that the states admit, in order.
    pub blocks: Vec<BlockGuide>,
    /// `line` is the line of the block's rule in the schema.
    pub line: usize,
}

/// `RuleLimit` is one limit of a block: its own, or one of a unit rule's or
/// a line rule's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RuleLimit {
    /// `rule` is the place of the unit rule or the line rule whose limit this
    /// is, among the unit rules of the schema, counted from 0. It is absent for
    /// the block's own limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub rule: Option<usize>,
    /// `format` is the name of the format that the unit rule or the line
    /// rule binds. It is absent for the block's own limit and for a rule
    /// that binds no format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub format: Option<String>,
    /// `rule_id` is the `rule-id` of the unit rule or the line rule. It is
    /// absent for the block's own limit and for a rule without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `counts` is what the unit rule or the line rule counts: the units of
    /// the block, or the content lines of a code block or an HTML block. It
    /// is absent for the block's own limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GuideCounts")]
    pub counts: Option<GuideCounts>,
    /// `line` is the line of the rule in the schema.
    pub line: usize,
    /// The limit.
    #[serde(flatten)]
    pub limit: GuideLimit,
}

/// `RuleLabel` says whether the units of a block, or the units of one of its
/// unit rules, carry a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RuleLabel {
    /// `rule` is the place of the unit rule whose `label` this is, among the
    /// unit rules of the schema, counted from 0. It is absent for the block's
    /// own `label`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub rule: Option<usize>,
    /// `rule_id` is the `rule-id` of the unit rule. It is absent for the
    /// block's own `label` and for a rule without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `line` is the line of the rule in the schema.
    pub line: usize,
    /// `label` is `required` where each unit carries a label, and `optional`
    /// where a unit carries one if its writer gives one.
    pub label: GuideLabel,
}

/// `GuideLabel` says whether a unit carries a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuideLabel {
    /// Every unit carries a label.
    Required,
    /// A unit carries a label where its writer gives one.
    Optional,
}

/// `GuideLimit` is a limit on a part: its kind, the number of units, lines, or
/// elements that it states, and what the writer does at the limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct GuideLimit {
    /// `kind` is `soft` for the usual length, which nothing checks, or
    /// `hard` for the most that a part can hold.
    pub kind: GuideLimitKind,
    /// `count` is the number that the limit states.
    pub count: usize,
    /// `when_at_limit` is the schema's text that says what the writer does at
    /// the limit.
    pub when_at_limit: String,
}

/// `GuideCounts` is what the limit of a unit rule or a line rule counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuideCounts {
    /// The units of the block: its paragraphs, items, rows, or repetitions.
    Units,
    /// The content lines of a code block or an HTML block.
    Lines,
}

/// `GuideLimitKind` is the kind of a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuideLimitKind {
    /// The usual length, which nothing checks.
    Soft,
    /// The most that a part can hold.
    Hard,
}
// sokf:end guide-report

/// `guide` returns the guide of each document that `opts.documents` names,
/// in the order of the arguments. It reads the tree as a lookup does, reads
/// each document from its file, and compiles the schemas of their types.
/// It refuses an argument that names no document of a store, a document
/// without a `type` or without a schema that passes the grammar, and a
/// `--state` that the document's lifecycle doesn't declare, before it
/// returns anything. `guide` writes only the cache.
// sokf-implements: CLI-9_AC-3
// sokf-implements: CLI-17_AC-5
// sokf-implements: CACHE-1_AC-27
pub fn guide(root: &RepoRoot, opts: &GuideOptions) -> Result<GuideReport> {
    let config = config::load_config(root)?;
    let (grammar, repo) = super::load::load_formats(root)?;
    let cache = Cache::open(root, opts.cache);
    // Only a reference to a key alone needs the keys that the schemas
    // capture, as for `locate`.
    let (mut model, keys) = if opts.documents.iter().any(|d| graph::is_key_reference(d)) {
        let slugs = super::locate::key_slugs(&opts.documents, graph::is_key_reference);
        let (model, keys) = super::load_keys_for(root, &config, &cache, &slugs)?;
        (model, Some(keys))
    } else {
        (super::load_for_lookup(root, &config, &cache)?, None)
    };
    let mut documents = Vec::new();
    for arg in &opts.documents {
        let path = graph::resolve_document(&model, keys.as_ref(), &opts.cwd, arg)?;
        documents.push(read(root, &path)?);
    }
    let types: BTreeSet<&str> = documents.iter().map(|(_, t)| t.as_str()).collect();
    for type_name in types {
        if type_name != "Schema" {
            super::admit_schemas_of_type(&mut model, root, &cache, type_name)?;
        }
    }
    // sokf-implements: CONF-12_AC-7
    let texts = config::load_texts(root)?;
    let compiled = grammar::compile_all(&model, grammar, &repo, texts);
    let mut guides = Vec::new();
    for (doc, type_name) in &documents {
        let (schema, schema_path, k) = schema_of(&model, &compiled, doc, type_name)?;
        let language = scaffold::language(&model, &compiled, schema, type_name, Some(k), Some(k));
        let words = scaffold::Words::new(&compiled.texts, language);
        let mut states = schema.core.states(doc.frontmatter.as_ref());
        // sokf-implements: CLI-17_AC-11
        let given = scaffold::given_states(schema, &opts.states)
            .map_err(|e| Error::Guide(format!("{}: {e}", doc.path)))?;
        states.0.extend(given.0);
        own_states(doc, schema, &states)?;
        guides.push(document_guide(
            doc,
            type_name,
            schema,
            schema_path,
            &states,
            &words,
        ));
    }
    Ok(GuideReport { documents: guides })
}

/// `read` reads the document at `path` from its file, with its type. It
/// refuses a document whose frontmatter gives no `type`.
// sokf-implements: CLI-17_AC-4
fn read(root: &RepoRoot, path: &RepoPath) -> Result<(Document, String)> {
    let src = source::load(root, path)?;
    let parsed = parse_source(&src, ParseMode::Document);
    let doc = Document::new(src, parsed);
    let Some(type_name) = doc.type_name().map(str::to_string) else {
        return Err(Error::Guide(format!(
            "{path} has no `type`, or its frontmatter does not parse, so it has no schema"
        )));
    };
    Ok((doc, type_name))
}

/// `schema_of` returns the compiled schema of the document's type in the
/// document's store, as `sokf new` chooses it for a type of that store, with
/// the path of the schema, or the grammar's schema of a schema document,
/// with no path (CLI-17_AC-21), and the document's store. It refuses what
/// `scaffold::schema_for` refuses, with the document's path.
// sokf-implements: CLI-17_AC-4
// sokf-implements: CLI-17_AC-21
fn schema_of<'a>(
    model: &Model,
    compiled: &'a grammar::Compiled,
    doc: &Document,
    type_name: &str,
) -> Result<(&'a SokfSchema, Option<RepoPath>, usize)> {
    let path = &doc.path;
    let Some(k) = model.knowledge_of(path) else {
        return Err(Error::Guide(format!("{path} lies in no knowledge store")));
    };
    let (_, schema) =
        scaffold::schema_for(model, compiled, type_name, Some(k)).map_err(|e| match e {
            Error::New(why) => Error::Guide(format!("{path}: {why}")),
            e => e,
        })?;
    let schema_path = compiled
        .for_type(model, k, type_name)
        .filter(|_| type_name != "Schema")
        .map(|(r, _)| model.schema(r).doc.path.clone());
    Ok((schema, schema_path, k))
}

/// `own_states` refuses a document whose frontmatter sets no state, or a
/// state that the lifecycle doesn't declare, for a dimension that `--state`
/// doesn't give, because the guide would then leave out each part that the
/// dimension gates.
// sokf-implements: CLI-17_AC-24
fn own_states(doc: &Document, schema: &SokfSchema, states: &States) -> Result<()> {
    for dim in &schema.core.core.lifecycle {
        let key = &dim.key;
        let why = match states.0.get(key).and_then(|v| v.first()) {
            Some(state) if dim.states.contains(state) => continue,
            Some(state) => format!(
                "`{key}: {state}` is not a state of `{key}`. The states are {}",
                dim.states.join(", ")
            ),
            None => format!("the frontmatter sets no `{key}`"),
        };
        return Err(Error::Guide(format!(
            "{}: {why}, so give `--state {key}=STATE`",
            doc.path
        )));
    }
    Ok(())
}

/// `document_guide` builds the guide of `doc` from the engine's guide of its
/// schema at `states`, with `words`, sokf's texts in the language of the
/// schema, through the host that derives the scaffold.
// sokf-implements: CLI-17_AC-6
// sokf-implements: CLI-17_AC-7
// sokf-implements: CLI-17_AC-8
// sokf-implements: CLI-17_AC-9
// sokf-implements: CLI-17_AC-10
// sokf-implements: CLI-17_AC-29
fn document_guide(
    doc: &Document,
    type_name: &str,
    schema: &SokfSchema,
    schema_path: Option<RepoPath>,
    states: &States,
    words: &scaffold::Words<'_>,
) -> DocumentGuide {
    // The engine's guide holds each part that the states admit, each with
    // the words of the scaffold's guidance line for it, and no rule,
    // pattern, or example.
    let host = scaffold::SokfScaffold::with_words(schema, words);
    let g = derive::guide(&schema.core, states, &host);
    let admitted: Vec<&Section> = schema
        .core
        .sections
        .iter()
        .filter(|s| s.common.only.admits(states))
        .collect();
    // The scaffold opens its body with the schema's guidance, then the
    // message of each ban of the whole body, on one line.
    let opening: Vec<String> = g
        .guidance
        .iter()
        .chain(&g.bans)
        .map(|t| squash(t))
        .filter(|t| !t.is_empty())
        .collect();
    DocumentGuide {
        path: doc.path.clone(),
        id: doc.id().map(str::to_string),
        type_name: type_name.to_string(),
        schema: schema_path,
        states: schema
            .core
            .core
            .lifecycle
            .iter()
            .filter_map(|d| {
                let state = states.0.get(&d.key)?.first()?;
                Some((d.key.clone(), state.clone().into()))
            })
            .collect(),
        guidance: g.guidance,
        bans: g.bans,
        text: (!opening.is_empty()).then(|| opening.join(" ")),
        // sokf-implements: CLI-17_AC-35
        // sokf-implements: CLI-17_AC-36
        labels: g
            .labels
            .into_iter()
            .map(|l| LabelGuide {
                meaning: words.attribution(&l.key),
                key: l.key,
                words: l.words,
            })
            .collect(),
        frontmatter: g
            .frontmatter
            .into_iter()
            .map(|k| key_guide(k, words))
            .collect(),
        // The scaffold writes the guidance of each `links` rule that the
        // states admit before the rule's entries, as one line, and leaves
        // out a rule that a hard limit of 0 bans.
        links: schema
            .written_links(states)
            // sokf-implements: CLI-17_AC-33
            // sokf-implements: CLI-17_AC-34
            .map(|r| LinkGuide {
                rel: r.rel.clone(),
                optional: r.min == 0,
                limit: r.max.clone().map(limit),
                text: scaffold::link_text(r, Some(words)),
            })
            .collect(),
        preamble: g.preamble.map(|p| PreambleGuide {
            guidance: p.guidance,
            text: p.text,
            blocks: p.blocks.into_iter().map(block_guide).collect(),
            line: p.line,
        }),
        // The engine's guide holds the sections that the states admit, in
        // the schema's order, so the guide of a section and its rule share an
        // index in `admitted`.
        sections: g
            .sections
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                debug_assert_eq!(admitted[i].common.line, s.line);
                let held = holds(doc, &admitted, i);
                SectionGuide {
                    heading: s.heading,
                    level: s.level,
                    rule_id: s.rule_id,
                    parent: s.parent,
                    optional: s.optional,
                    repeat: s.repeat,
                    held,
                    guidance: s.guidance,
                    text: s.text,
                    blocks: s.blocks.into_iter().map(block_guide).collect(),
                    line: s.line,
                }
            })
            .collect(),
    }
}

/// `key_guide` copies the engine's guide of one frontmatter key, with sokf's
/// own text for the key before the schema's, as the scaffold writes them on
/// one line.
fn key_guide(k: derive::KeyGuide, words: &scaffold::Words<'_>) -> KeyGuide {
    let text = match (host_text(&k.key, words), k.text) {
        (Some(host), Some(own)) => Some(format!("{host} {own}")),
        (host, own) => host.or(own),
    };
    KeyGuide {
        lifecycle: k.lifecycle,
        optional: k.optional,
        guidance: k.guidance,
        limit: k.limit.map(limit),
        text,
        line: k.line,
        key: k.key,
    }
}

/// `host_text` returns sokf's own text for the frontmatter key `key`, which
/// the guide puts before the schema's text for the key, as the scaffold puts
/// it at the start of the guidance line before the key: the rule for
/// writing a description, in the language of the schema.
// sokf-implements: CLI-17_AC-32
fn host_text(key: &str, words: &scaffold::Words<'_>) -> Option<String> {
    (key == "description")
        .then(|| words.description())
        .flatten()
}

/// `block_guide` copies the engine's guide of one block, with the format,
/// the `rule-id`, and what each rule's limit counts, and the `label` of the
/// block and of each of its unit rules.
// sokf-implements: CLI-17_AC-31
// sokf-implements: CLI-17_AC-37
fn block_guide(b: derive::BlockGuide) -> BlockGuide {
    BlockGuide {
        kind: b.kind.name().to_string(),
        rule_id: b.rule_id,
        pos: b.pos,
        optional: b.optional,
        guidance: b.guidance,
        limits: b
            .limits
            .into_iter()
            .map(|l| RuleLimit {
                rule: l.rule,
                format: l.format,
                rule_id: l.rule_id,
                counts: l.counts.map(|c| match c {
                    Counts::Units => GuideCounts::Units,
                    Counts::Lines => GuideCounts::Lines,
                }),
                line: l.line,
                limit: limit(l.limit),
            })
            .collect(),
        labels: b
            .labels
            .into_iter()
            .map(|l| RuleLabel {
                rule: l.rule,
                rule_id: l.rule_id,
                line: l.line,
                label: match l.label {
                    Label::Required => GuideLabel::Required,
                    Label::Optional => GuideLabel::Optional,
                },
            })
            .collect(),
        bans: b.bans,
        text: b.text,
        blocks: b.blocks.into_iter().map(block_guide).collect(),
        line: b.line,
    }
}

fn limit(l: grammar::Limit) -> GuideLimit {
    GuideLimit {
        kind: match l.kind {
            LimitKind::Soft => GuideLimitKind::Soft,
            LimitKind::Hard => GuideLimitKind::Hard,
        },
        count: l.count,
        when_at_limit: l.when_at_limit,
    }
}

/// `holds` reports whether `doc` has a heading of the section `i` of
/// `sections`, the sections that the states admit, as `takes` reads a
/// heading.
// sokf-implements: CLI-17_AC-15
fn holds(doc: &Document, sections: &[&Section], i: usize) -> bool {
    (0..doc.body.headings.len()).any(|h| takes(doc, sections, i, h))
}

/// `takes` reports whether the heading `h` of `doc` is a heading of the
/// section `i` of `sections`, the sections that the states admit: a heading
/// at the section's level whose text has the section's heading shape or one
/// of its `renamed-from` texts. A section whose heading has a template or a
/// pattern leaves a heading whose text another section gives exactly, at
/// any level, as the check gives the heading to that section first, and
/// a `renamed-from` text takes a heading only where no section takes it by
/// its text or its shape. A section inside a parent takes only a heading
/// that lies under a heading of the parent.
// sokf-implements: CLI-17_AC-25
// sokf-implements: CLI-17_AC-26
// sokf-implements: CLI-17_AC-27
fn takes(doc: &Document, sections: &[&Section], i: usize, h: usize) -> bool {
    let section = sections[i];
    let headings = &doc.body.headings;
    let heading = &headings[h];
    if heading.level != section.level {
        return false;
    }
    // A heading of exact text equals the text as written, as the check
    // compares it, and a shape reads the text squashed.
    let exact = |s: &Section| match &s.heading.shape {
        Shape::Text(Text::Literal(t)) => t == &heading.text,
        Shape::Text(Text::From(key)) => doc
            .frontmatter
            .as_ref()
            .and_then(|f| f.get(key))
            .and_then(crate::Spanned::scalar_text)
            .is_some_and(|v| v == heading.text),
        _ => false,
    };
    let text = squash(&heading.text);
    let shaped = |s: &Section| {
        !s.is_exact() && s.level == heading.level && s.heading.shape.matches(&text, |_| None)
    };
    let named = if section.is_exact() {
        exact(section)
    } else {
        shaped(section) && !sections.iter().any(|s| exact(s))
    };
    // A `renamed-from` text takes only a heading that no section takes by
    // its exact text or by its shape, as the check tries those first.
    let renamed = section.renamed_from.iter().any(|r| r == &heading.text)
        && !sections.iter().any(|s| exact(s) || shaped(s));
    if !(named || renamed) {
        return false;
    }
    let Some(parent) = &section.parent else {
        return true;
    };
    let Some(p) = sections
        .iter()
        .position(|s| s.rule_id.as_deref() == Some(parent.as_str()))
    else {
        return false;
    };
    // The heading lies under the nearest heading above it at the parent's
    // level or above, as the check finds the parent of a heading.
    let level = sections[p].level;
    (0..h)
        .rev()
        .find(|&j| headings[j].level <= level)
        .is_some_and(|j| takes(doc, sections, p, j))
}
