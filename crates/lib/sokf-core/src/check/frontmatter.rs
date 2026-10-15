//! This module holds the frontmatter checks that the format specification
//! requires of each document, and the identity checks across a whole
//! knowledge store (SOKF_FORMAT_33 and SOKF_SCHEMA_32).

use super::Ctx;
use crate::{
    Document, Finding, Model, RepoPath,
    markdown::is_slug,
    model::{CoreFields, frontmatter::STAMPED_DOCUMENT_KEYS},
};

/// `check` runs the frontmatter checks on one document. It checks that the
/// frontmatter parses, that `type` and `id` are present and well-formed,
/// that no stamped field appears, that no other document in the store has
/// the same id, and that no other document of the same type has the same
/// sequence number.
// sokf-implements: SOKF_FORMAT_19
// sokf-implements: SOKF_FORMAT_24
// sokf-implements: SOKF_FORMAT_32
// sokf-implements: SOKF_FORMAT_33
// sokf-implements: SOKF_SCHEMA_32
pub fn check(k: usize, doc: &Document, ctx: &Ctx<'_>) -> Vec<Finding> {
    let mut findings = check_core(doc);
    findings.extend(check_ids(ctx.model, k, doc));
    findings.extend(check_seq(ctx, k, doc));
    findings
}

/// `check_ids` reports an error when another document or schema in the
/// store has the same id as the document (SOKF_FORMAT_33).
pub fn check_ids(model: &Model, k: usize, doc: &Document) -> Vec<Finding> {
    let Some(id) = doc.id() else {
        return Vec::new();
    };
    let others: Vec<String> = model
        .ids(k, id)
        .into_iter()
        .filter(|p| *p != doc.path)
        .map(|p| p.to_string())
        .collect();
    if others.is_empty() {
        return Vec::new();
    }
    vec![Finding::error(
        doc.path.clone(),
        doc.core.line_of("id"),
        "SOKF_FORMAT_33",
        format!("{} id `{id}`", also_use(&others)),
    )]
}

/// `also_use` returns `others`, joined, followed by "also uses" or "also
/// use" to agree with their number.
pub(super) fn also_use(others: &[String]) -> String {
    let verb = if others.len() == 1 { "uses" } else { "use" };
    format!("{} also {verb}", others.join(", "))
}

/// `check_seq` reports an error when another document of the same type
/// has the same sequence number as the document (SOKF_SCHEMA_32).
fn check_seq(ctx: &Ctx<'_>, k: usize, doc: &Document) -> Vec<Finding> {
    let Some(type_name) = doc.type_name() else {
        return Vec::new();
    };
    let Some((_, schema)) = ctx.compiled.for_type(ctx.model, k, type_name) else {
        return Vec::new();
    };
    let Some(template) = schema.core.core.id_template() else {
        return Vec::new();
    };
    let Some(seq) = doc.id().and_then(|id| template.seq_of(id)) else {
        return Vec::new();
    };
    let others: Vec<String> = ctx
        .model
        .ids_of_type(k, type_name)
        .into_iter()
        .filter(|(p, _)| *p != doc.path)
        .filter(|(_, id)| template.seq_of(id) == Some(seq))
        .map(|(p, _)| p.to_string())
        .collect();
    if others.is_empty() {
        return Vec::new();
    }
    vec![Finding::error(
        doc.path.clone(),
        doc.core.line_of("id"),
        "SOKF_SCHEMA_32",
        format!("{} sequence number {seq}", also_use(&others)),
    )]
}

/// `check_core` checks the core fields of a document's frontmatter: that
/// the frontmatter parses, that `type` and `id` are present and
/// well-formed, and that no stamped field appears. `check` adds the checks
/// that compare the document with other documents.
///
/// The engine, not `check_core`, checks the markers left in a governed
/// document.
pub fn check_core(doc: &Document) -> Vec<Finding> {
    check_core_fields(&doc.core, &doc.path)
}

/// `check_core_fields` checks `core`, the core fields of the frontmatter of
/// the document at `path`, as `check_core` does.
pub fn check_core_fields(core: &CoreFields, path: &RepoPath) -> Vec<Finding> {
    let mut findings = core.problems.clone();
    if core.present {
        if core.type_.is_none() && !core.problems.iter().any(|f| f.message.contains("`type`")) {
            findings.push(Finding::error(
                path.clone(),
                Some(1),
                "SOKF_FORMAT_19",
                "`type` is missing",
            ));
        }
        for key in STAMPED_DOCUMENT_KEYS {
            if let Some(line) = core.line_of(key) {
                findings.push(Finding::error(
                    path.clone(),
                    Some(line),
                    "SOKF_FORMAT_24",
                    format!("`{key}` is a field that export tooling writes when it publishes a store, and it mustn't appear in the working tree"),
                ));
            }
        }
        match core.id.as_deref() {
            None => findings.push(Finding::error(
                path.clone(),
                Some(1),
                "SOKF_FORMAT_32",
                "`id` is missing",
            )),
            Some(id) if !is_slug(id) => findings.push(Finding::error(
                path.clone(),
                core.line_of("id"),
                "SOKF_FORMAT_32",
                format!("id `{id}` is not a slug"),
            )),
            Some(_) => {}
        }
    }
    findings
}
