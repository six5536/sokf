//! This module implements `sokf guide` (CLI-17), which prints the guidance that
//! a document's schema gives for the document's parts at the document's
//! lifecycle states, with each part's example. The engine's
//! `derive::guide_for` returns that guidance, and the example that the
//! schema's scaffold writes for each part with the document's own values,
//! and `sokf new` prints the guide of the document that it creates.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use tramlines_md::{
    check::{self, Counts, HeadingMatch},
    derive,
};

use crate::{
    Cache, CacheMode, Document, Error, Model, RepoPath, RepoRoot, Result, Spanned, Yaml, config,
    grammar::{self, Label, LimitKind, Section, SokfSchema, States, squash},
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

mod report;

pub use report::*;

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
// sokf-implements: CLI-17_AC-38
fn document_guide(
    doc: &Document,
    type_name: &str,
    schema: &SokfSchema,
    schema_path: Option<RepoPath>,
    states: &States,
    words: &scaffold::Words<'_>,
) -> DocumentGuide {
    // The engine's guide holds each part that the states admit, each with
    // the words of the scaffold's guidance line for it, and the example that
    // the scaffold writes for it with the document's own values, and no rule
    // or pattern.
    let host = scaffold::SokfScaffold::with_words(schema, words);
    let (g, plain) = guides(doc, schema, states, &host);
    let (frontmatter_text, body_text) = texts(doc);
    let key_example = |key: &str| {
        plain
            .frontmatter
            .iter()
            .find(|k| k.key == key)
            .and_then(|k| k.example.clone())
    };
    let admitted: Vec<usize> = (0..schema.core.sections.len())
        .filter(|&i| schema.core.sections[i].common.only.admits(states))
        .collect();
    // The check's own match of each heading to its section.
    let matches = check::match_headings(crate::check::doc_view(doc).engine(), &schema.core, states);
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
            .map(|mut k| {
                k.example = key_example(&k.key);
                key_guide(k, words)
            })
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
                example: scaffold::link_example(r),
            })
            .collect(),
        preamble: g.preamble.map(|p| PreambleGuide {
            guidance: p.guidance,
            text: p.text,
            example: p.example,
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
                let section = admitted[i];
                debug_assert_eq!(schema.core.sections[section].common.line, s.line);
                let held = holds(doc, &schema.core.sections, &matches, section);
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
                    example: s.example,
                    blocks: s.blocks.into_iter().map(block_guide).collect(),
                    line: s.line,
                }
            })
            .collect(),
        frontmatter_text,
        body_text,
    }
}

/// `texts` returns the YAML of the frontmatter of `doc` and its body, as the
/// source's split gives them.
fn texts(doc: &Document) -> (String, String) {
    let split = source::split(&doc.source);
    let front = split.frontmatter.map_or("", |(yaml, _)| yaml);
    (front.to_string(), split.body.to_string())
}

/// `guides` returns the engine's guide of `doc` at `states` twice: with each
/// scalar frontmatter value of the document, and with its id and its title
/// alone, from which the example of each other frontmatter key comes, because
/// the writer's own value, right or wrong, is no example of the key. A value
/// of the document that no scaffold can hold, such as a title that a
/// paragraph bound to it can't hold, would leave every part without an
/// example, so both then carry the schema's values.
fn guides(
    doc: &Document,
    schema: &SokfSchema,
    states: &States,
    host: &scaffold::SokfScaffold<'_>,
) -> (derive::Guide, derive::Guide) {
    let mut values = scalar_values(doc);
    if derive::derive_for(&schema.core, states, host, &values).is_err() {
        values.clear();
    }
    let own: BTreeMap<String, String> = values
        .iter()
        .filter(|(k, _)| matches!(k.as_str(), "id" | "title"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    (
        derive::guide_for(&schema.core, states, host, &values),
        derive::guide_for(&schema.core, states, host, &own),
    )
}

/// `scalar_values` returns each top-level frontmatter key of `doc` whose
/// value is a scalar that holds text, with that text, which the engine
/// splices into each example in place of the schema's, so that an example
/// carries the document's own values.
// sokf-implements: CLI-17_AC-39
fn scalar_values(doc: &Document) -> BTreeMap<String, String> {
    let Some(Spanned {
        value: Yaml::Map(entries),
        ..
    }) = &doc.frontmatter
    else {
        return BTreeMap::new();
    };
    entries
        .iter()
        .filter(|(_, v)| !matches!(v.value, Yaml::Null | Yaml::Seq(_) | Yaml::Map(_)))
        .filter_map(|(k, v)| Some((k.value.clone(), v.scalar_text()?)))
        .filter(|(_, v)| !v.trim().is_empty())
        .collect()
}

/// `new_guide` returns the guide of the new document `text`, which `sokf new`
/// builds at `path` of the type `type_name` at `states`, as `sokf guide`
/// returns it for the document once written, with `words`, sokf's texts in
/// the language of the schema.
// sokf-implements: CLI-6_AC-127
#[expect(
    clippy::too_many_arguments,
    reason = "the guide of a new document needs what `new` resolved, field by field"
)]
pub(super) fn new_guide(
    path: &RepoPath,
    text: &str,
    type_name: &str,
    schema: &SokfSchema,
    model: &Model,
    compiled: &grammar::Compiled,
    knowledge: Option<usize>,
    states: &States,
    words: &scaffold::Words<'_>,
) -> DocumentGuide {
    let source = crate::Source::from_text(path.clone(), text);
    let parsed = parse_source(&source, ParseMode::Document);
    let doc = Document::new(source, parsed);
    // The store that holds the path chooses the schema's path, as `guide`
    // chooses it for the document once written.
    let schema_path = model
        .knowledge_of(path)
        .or(knowledge)
        .and_then(|k| compiled.for_type(model, k, type_name))
        .filter(|_| type_name != "Schema")
        .map(|(r, _)| model.schema(r).doc.path.clone());
    document_guide(&doc, type_name, schema, schema_path, states, words)
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
        example: k.example,
        line: k.line,
        key: k.key,
    }
}

/// `host_text` returns sokf's own text for the frontmatter key `key`, which
/// the guide puts before the schema's text for the key, as the scaffold puts
/// it at the start of the guidance line before the key: the rule for
/// writing a description, in the language of the schema.
// sokf-implements: CLI-17_AC-32
// sokf-implements: CLI-6_AC-118
// sokf-implements: CLI-6_AC-119
// sokf-implements: CLI-6_AC-123
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
        example: b.example,
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

/// `holds` reports whether `doc` has a heading of the section `i` of the
/// schema's sections, as `takes` reads a heading, given `matches`, the
/// section that the check matches each heading to.
// sokf-implements: CLI-17_AC-15
fn holds(doc: &Document, sections: &[Section], matches: &[Option<HeadingMatch>], i: usize) -> bool {
    (0..doc.body.headings.len()).any(|h| takes(doc, sections, matches, i, h))
}

/// `takes` reports whether the heading `h` of `doc` is a heading of the
/// section `i` of `sections`, the schema's sections: the check matches the
/// heading to the section, the heading stands at the section's level, and
/// a heading bound to the frontmatter reads as the value. The match reads
/// each heading in tramlines' fixed spelling of its markup, gives a heading
/// whose text another section gives exactly to that section, and gives a
/// heading to a section by a `renamed-from` text only where no section
/// takes it by its text or its shape. A section inside a parent takes only
/// a heading that lies under a heading of the parent, which the match
/// doesn't read.
// sokf-implements: CLI-17_AC-25
// sokf-implements: CLI-17_AC-26
// sokf-implements: CLI-17_AC-27
fn takes(
    doc: &Document,
    sections: &[Section],
    matches: &[Option<HeadingMatch>],
    i: usize,
    h: usize,
) -> bool {
    if !crate::check::headings::opens(doc, sections, matches, h, i) {
        return false;
    }
    let headings = &doc.body.headings;
    let Some(parent) = &sections[i].parent else {
        return true;
    };
    // The heading lies under the nearest heading above it at the parent's
    // level or above, as the check finds the parent of a heading. Only a
    // parent that the states admit takes a heading.
    (0..sections.len())
        .filter(|&p| sections[p].rule_id.as_deref() == Some(parent.as_str()))
        .any(|p| {
            let level = sections[p].level;
            (0..h)
                .rev()
                .find(|&j| headings[j].level <= level)
                .is_some_and(|j| takes(doc, sections, matches, p, j))
        })
}

#[cfg(test)]
mod tests;
