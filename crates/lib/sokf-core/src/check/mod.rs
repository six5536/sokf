//! This module checks documents against the validation specification, with
//! one submodule for each group of criteria. The check engine checks the
//! grammar's rules, and the `host` module answers the engine's questions
//! about the repository.

mod assets;
mod coverage;
mod current_links;
mod frontmatter;
mod generated;
mod host;
mod index;
mod lifecycle;
mod link_rules;
mod links;
mod manifest;
mod markers;
mod plans;
mod schema_document;
mod warnings;

use std::collections::{BTreeMap, BTreeSet};

pub use coverage::{Citation, Citations};
pub use generated::asset_include;
pub use host::{SchemaHost, SokfHost, check_against};
pub use index::check_indexes;
pub use lifecycle::{derived_dir, placed};
pub use plans::{capture_plans_of, planned_by};
pub use tramlines_md::check::{
    Capture, CheckOutput, Gate, Placement, Repair, Spot, heading_col, placements,
};

use crate::{
    Body, Compiled, Expected, Finding, Indexes, Knowledge, Locator, Model, RepoPath, Segments,
    Spanned,
    grammar::{KeyScope, SokfSchema, States},
    model::{CoreFields, KeyEntry, KeyIndex, Query},
};

/// A `DocView` is what a check sees of a document, a scaffold, or an
/// additional file.
#[derive(Debug, Clone, Copy)]
pub struct DocView<'a> {
    /// The file, or the schema for a template or an example.
    pub path: &'a RepoPath,
    /// The core fields, or `None` for an additional file.
    pub core: Option<&'a CoreFields>,
    /// The frontmatter as written.
    pub frontmatter: Option<&'a Spanned>,
    /// The body.
    pub body: &'a Body,
    /// The markers.
    pub segments: &'a Segments,
    /// The number of lines.
    pub line_count: usize,
    /// The whole text, with LF line endings.
    pub text: &'a str,
    /// The line that the body starts on.
    pub body_line: usize,
}

impl<'a> DocView<'a> {
    /// `engine` returns the view that the check engine sees. The view
    /// carries the frontmatter only when the core check found frontmatter
    /// to read.
    pub fn engine(&self) -> tramlines_md::check::DocView<'a> {
        tramlines_md::check::DocView {
            path: self.path,
            frontmatter: self.core.filter(|c| c.present).and(self.frontmatter),
            body: self.body,
            segments: self.segments,
            line_count: self.line_count,
            text: self.text,
            body_line: self.body_line,
        }
    }
}

/// A `Ctx` holds the state that every check of a run shares.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    /// The model.
    pub model: &'a Model,
    /// The compiled schemas.
    pub compiled: &'a Compiled,
    /// The expected generated content.
    pub expected: &'a Expected,
    /// The captured keys of every document.
    pub keys: &'a KeyIndex,
    /// The knowledge that the checked file belongs to.
    pub knowledge: Option<usize>,
    /// Whether the checked file is a scaffold. A scaffold turns off every
    /// rule that depends on the repository.
    pub scaffold: bool,
}

impl<'a> Ctx<'a> {
    /// `new` returns the context of a check of a file in `knowledge`, outside
    /// any scaffold.
    pub fn new(
        model: &'a Model,
        compiled: &'a Compiled,
        expected: &'a Expected,
        keys: &'a KeyIndex,
        knowledge: Option<usize>,
    ) -> Self {
        Ctx {
            model,
            compiled,
            expected,
            keys,
            knowledge,
            scaffold: false,
        }
    }
}

/// `states_of` returns a document's lifecycle states, one for each
/// dimension that the document carries.
pub fn states_of(frontmatter: Option<&Spanned>, schema: &SokfSchema) -> States {
    schema.core.states(frontmatter)
}

/// `capture_keys_of` captures the keys of the loaded documents in `only`,
/// or the keys of every loaded document when `only` is `None`. The key
/// index needs this pre-pass before any fragment can resolve.
pub fn capture_keys_of(
    model: &Model,
    compiled: &Compiled,
    only: Option<&BTreeSet<RepoPath>>,
) -> KeyIndex {
    let mut keys = KeyIndex::default();
    let empty = KeyIndex::default();
    let expected = &Expected::default();
    for (k, doc) in model.documents() {
        if only.is_some_and(|o| !o.contains(&doc.path)) {
            continue;
        }
        let Some((_, schema)) = compiled.for_doc(model, k, doc) else {
            continue;
        };
        let ctx = Ctx::new(model, compiled, expected, &empty, Some(k));
        model.watch(&doc.path);
        let out = check_against(
            doc_view(doc),
            schema,
            &states_of(doc.frontmatter.as_ref(), schema),
            &ctx,
            Locator::schema(""),
        );
        model.unwatch();
        for c in out.captures {
            keys.add(KeyEntry {
                key: c.key,
                path: doc.path.clone(),
                line: c.line,
                heading_slug: c.heading_slug,
                scope: c.scope,
                knowledge: k,
                rule_id: c.rule_id,
            });
        }
    }
    keys
}

/// `doc_view` returns a document as a check sees it.
pub fn doc_view(doc: &crate::Document) -> DocView<'_> {
    DocView {
        path: &doc.path,
        core: Some(&doc.core),
        frontmatter: doc.frontmatter.as_ref(),
        body: &doc.body,
        segments: &doc.segments,
        line_count: doc.line_count(),
        text: &doc.source.text,
        body_line: doc.body_line,
    }
}

/// `additional_view` returns an additional file as a check sees it.
pub fn additional_view(file: &crate::model::AdditionalFile) -> DocView<'_> {
    DocView {
        path: &file.path,
        core: None,
        frontmatter: None,
        body: &file.body,
        segments: &file.segments,
        line_count: file.source.line_count(),
        text: &file.source.text,
        body_line: 1,
    }
}

/// A `RunOutput` is what one run of the checker produces: the findings of the
/// checks that run every time, the findings of each checked document, and
/// the citations that the checked documents resolved (CACHE-1_AC-14).
#[derive(Debug, Default)]
pub struct RunOutput {
    /// The findings of the build, the manifests, and the assets, which
    /// every run makes afresh.
    pub fresh: Vec<Finding>,
    /// The findings of each checked document, schema document, index, and
    /// additional file, by its path. The findings of a schema document
    /// include its compile problems.
    pub documents: BTreeMap<RepoPath, Vec<Finding>>,
    /// The citations that the checked documents resolved, by source.
    pub citations: BTreeMap<RepoPath, Vec<(RepoPath, Citation)>>,
}

impl RunOutput {
    /// `findings` returns every finding of the run.
    pub fn findings(&self) -> Vec<Finding> {
        let mut all = self.fresh.clone();
        for list in self.documents.values() {
            all.extend(list.iter().cloned());
        }
        all
    }
}

/// `run` runs every step of the check over every knowledge and every
/// additional file, and returns the findings.
// sokf-implements: SOKF_VALIDATION_1
// sokf-implements: CLI-4_AC-1
pub fn run(
    model: &Model,
    compiled: &Compiled,
    expected: &Expected,
    keys: &KeyIndex,
    indexes: &Indexes,
) -> Vec<Finding> {
    run_units(
        model,
        compiled,
        expected,
        keys,
        indexes,
        None,
        Vec::new(),
        true,
    )
    .findings()
}

/// `run_units` runs the checks of the documents in `units`, or of every
/// document when `units` is `None`. With `fresh`, it also runs the checks
/// that run every time. Before the coverage pass, it adds the stored
/// citations of the units to the citations that the run resolves. It adds
/// only the stored citations from sources outside this run's checks
/// (CACHE-1_AC-14).
// sokf-implements: SOKF_VALIDATION_1
// sokf-implements: CLI-4_AC-20
// sokf-implements: CACHE-1_AC-4
#[allow(clippy::too_many_arguments)]
pub fn run_units(
    model: &Model,
    compiled: &Compiled,
    expected: &Expected,
    keys: &KeyIndex,
    indexes: &Indexes,
    units: Option<&BTreeSet<RepoPath>>,
    stored_citations: Vec<(RepoPath, Citation)>,
    fresh: bool,
) -> RunOutput {
    let mut out = RunOutput::default();
    let checked = |path: &RepoPath| units.is_none_or(|u| u.contains(path));
    let ctx = |knowledge: Option<usize>| Ctx::new(model, compiled, expected, keys, knowledge);
    // A compile problem of a schema goes in that schema's output. A compile
    // problem that belongs to no schema, such as one in the grammar
    // itself, goes in the findings that every run makes afresh.
    let schema_paths: BTreeSet<&RepoPath> = model
        .knowledges
        .iter()
        .flat_map(|k| k.schemas.iter().map(|s| &s.doc.path))
        .collect();
    if fresh {
        out.fresh.extend(model.problems.iter().cloned());
        out.fresh.extend(
            compiled
                .problems
                .iter()
                .filter(|p| !schema_paths.contains(&p.path))
                .cloned(),
        );
        out.fresh.extend(manifest::check_identities(model));
    }
    // The link check keeps every citation that it resolves, and the
    // coverage pass reads them after the run checks every document. The
    // model records the reads of each file against that file
    // (CACHE-1_AC-4).
    let mut citations = Citations::default();
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        let ctx = ctx(Some(k));
        if fresh {
            out.fresh.extend(manifest::check(knowledge));
            out.fresh.extend(assets::check_assets(knowledge));
        }
        for doc in &knowledge.documents {
            if !checked(&doc.path) {
                continue;
            }
            check_unit(model, &mut out, &mut citations, &doc.path, |c| {
                check_document(knowledge, k, doc, &ctx, c)
            });
        }
        // A schema document is a unit, as a document is. The run checks it
        // when it's dirty, and stores its output under its path
        // (CACHE-1_AC-4).
        for (i, schema) in knowledge.schemas.iter().enumerate() {
            if !checked(&schema.doc.path) {
                continue;
            }
            check_unit(model, &mut out, &mut citations, &schema.doc.path, |c| {
                let mut findings: Vec<Finding> = compiled
                    .problems
                    .iter()
                    .filter(|p| p.path == schema.doc.path)
                    .cloned()
                    .collect();
                findings.extend(schema_document::check_schema(knowledge, i, &ctx));
                findings.extend(frontmatter::check_core(&schema.doc));
                findings.extend(frontmatter::check_ids(model, k, &schema.doc));
                findings.extend(links::check_document(doc_view(&schema.doc), &ctx, c));
                findings.extend(generated::check(doc_view(&schema.doc), &ctx));
                findings
            });
        }
    }
    // The index of each directory in `indexes` is a unit. In an incremental
    // run, `indexes` holds only the touched directories. The run stores the
    // output of each index under its path, and the output is empty when the
    // index is current (CACHE-1_AC-19).
    let index_findings = index::check_indexes(model, indexes);
    for path in indexes.expected.keys() {
        if !checked(path) {
            continue;
        }
        out.documents.insert(
            path.clone(),
            index_findings
                .iter()
                .filter(|f| &f.path == path)
                .cloned()
                .collect(),
        );
    }
    // An additional file is a unit, as a document is (CACHE-1_AC-20).
    for file in &model.additional {
        if !checked(&file.path) {
            continue;
        }
        let knowledge = crate::include::knowledge_of_additional(model, file);
        let ctx = ctx(knowledge);
        check_unit(model, &mut out, &mut citations, &file.path, |c| {
            check_additional(file, &ctx, c)
        });
    }
    // A file outside the knowledge roots is a unit for its markers. When
    // the file is also an additional file, both checks share one output.
    for (path, rows) in &model.markers {
        if !checked(path) {
            continue;
        }
        let ctx = ctx(None);
        check_unit(model, &mut out, &mut citations, path, |c| {
            markers::check(path, rows, &ctx, c)
        });
    }
    for (target, citation) in stored_citations {
        citations.add(target, citation);
    }
    // After the run checks every document, it checks what other documents
    // say about each document: whether its keys are unique, and
    // whether it has the citations it needs.
    for (k, doc) in model.documents() {
        if !checked(&doc.path) {
            continue;
        }
        model.watch(&doc.path);
        let mut findings = uniqueness_of(model, keys, doc);
        findings.extend(coverage::check_document(
            model, compiled, keys, &citations, k, doc,
        ));
        model.unwatch();
        out.documents
            .entry(doc.path.clone())
            .or_default()
            .extend(findings);
    }
    out
}

/// `check_unit` runs `check` on the unit at `path`. The model records the
/// reads of the check against `path`. `check_unit` adds the findings of the
/// check, and the citations that the check resolves, to the unit's output.
fn check_unit(
    model: &Model,
    out: &mut RunOutput,
    citations: &mut Citations,
    path: &RepoPath,
    check: impl FnOnce(&mut Citations) -> Vec<Finding>,
) {
    model.watch(path);
    let before = citations.len();
    let findings = check(citations);
    out.citations
        .entry(path.clone())
        .or_default()
        .extend(citations.since(before));
    model.unwatch();
    out.documents
        .entry(path.clone())
        .or_default()
        .extend(findings);
}

/// `check_document` runs the checks of one document. These checks need no
/// result from another check. They check the document's frontmatter and
/// ids, its schema, its asset, its links, its generated blocks, and its
/// `rel` values.
// sokf-implements: FORMAT-1_AC-2
pub fn check_document(
    knowledge: &Knowledge,
    k: usize,
    doc: &crate::Document,
    ctx: &Ctx<'_>,
    citations: &mut Citations,
) -> Vec<Finding> {
    let mut findings = frontmatter::check(k, doc, ctx);
    findings.extend(schema_document::check_typed(knowledge, doc, ctx));
    findings.extend(assets::check_document(knowledge, doc, ctx));
    findings.extend(links::check_document(doc_view(doc), ctx, citations));
    findings.extend(generated::check(doc_view(doc), ctx));
    findings.extend(warnings::check(doc));
    findings
}

/// `check_additional` runs the checks of one additional file, which check
/// its links, its generated blocks, and its schema.
pub fn check_additional(
    file: &crate::model::AdditionalFile,
    ctx: &Ctx<'_>,
    citations: &mut Citations,
) -> Vec<Finding> {
    let view = additional_view(file);
    let mut findings = links::check_document(view, ctx, citations);
    findings.extend(generated::check(view, ctx));
    if let Some(r) = file.schema {
        findings.extend(schema_document::check_additional(view, r, ctx));
    }
    findings
}

/// `uniqueness_of` checks that each key of one document is unique beyond
/// that document, in the key's scope. The finding for a clash names the key
/// and the other documents that capture it (grammar `unique`).
// sokf-implements: EXT-3_AC-1
// sokf-implements: EXT-5_AC-1
fn uniqueness_of(model: &Model, keys: &KeyIndex, doc: &crate::Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut clashes: BTreeMap<(RepoPath, usize, String), Vec<RepoPath>> = BTreeMap::new();
    for e in keys.of_document(&doc.path) {
        let s = crate::markdown::slug(&e.key);
        // Two captures in one document belong to the document's own check
        // (grammar `unique`, document scope). That check can tell a pairing
        // that `for-each-key-of` demands from a clash. A report here would list
        // the document as clashing with itself.
        let entries: &[KeyEntry] = match &e.scope {
            KeyScope::Host(scope) if scope == "knowledge" => {
                model.note(Query::KeyKnowledge {
                    knowledge: e.knowledge,
                    slug: s.clone(),
                });
                keys.knowledge
                    .get(&(e.knowledge, s))
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
            }
            KeyScope::Host(scope) if scope == "repository" => {
                model.note(Query::KeyRepository(s.clone()));
                keys.repository.get(&s).map(Vec::as_slice).unwrap_or(&[])
            }
            _ => continue,
        };
        let others: Vec<RepoPath> = entries
            .iter()
            .filter(|o| o.path != e.path)
            .map(|o| o.path.clone())
            .collect();
        if others.is_empty() {
            continue;
        }
        clashes
            .entry((e.path.clone(), e.line, e.key.clone()))
            .or_default()
            .extend(others);
    }
    for ((path, line, key), mut others) in clashes {
        others.sort();
        others.dedup();
        let names: Vec<String> = others.iter().map(ToString::to_string).collect();
        findings.push(Finding::error(
            path,
            Some(line),
            "SOKF_FORMAT_69",
            format!("key `{key}` is also captured by {}", names.join(", ")),
        ));
    }
    findings
}
