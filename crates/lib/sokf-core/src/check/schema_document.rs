//! This module holds the checks that the schema specification requires:
//! that sokf checks each document against the schema of its type, that each
//! type has one schema, that a `Schema` document lives in `schemas/`, and
//! that the scaffolds derived from a schema and the schema's examples pass
//! the schema.

use tramlines_md::check as engine;

use super::{Ctx, SchemaHost, check_against, doc_view, lifecycle, states_of};
use crate::{
    Finding, Knowledge, Locator, MarkerCheck, Severity,
    grammar::{CitedFrom, SokfSchema, States},
    model::{Document, SchemaRef},
};

/// `check_schema` checks schema `i` of `knowledge` against the grammar's
/// `schema-document` schema. It also checks the scaffolds that the schema
/// derives and the schema's examples against the schema. The check records
/// that it read the schema's key. The key is the type that the schema claims.
/// The cache therefore runs this check again when a user adds or removes a
/// schema for the same type. It also runs this check again when the formats
/// file changes a format or a type that the schema uses (CACHE-1_AC-15).
// sokf-implements: SOKF_SCHEMA_13
// sokf-implements: SOKF_SCHEMA_15
// sokf-implements: CACHE-1_AC-15
pub fn check_schema(knowledge: &Knowledge, i: usize, ctx: &Ctx<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(k) = ctx.knowledge else {
        return findings;
    };
    let document = &knowledge.schemas[i];
    let r = SchemaRef {
        knowledge: k,
        index: i,
    };
    ctx.model.note(ctx.model.schema_query(r));
    let out = check_against(
        doc_view(&document.doc),
        &ctx.compiled.schema_document,
        &States::default(),
        ctx,
        Locator::schema("schema-document"),
    );
    findings.extend(out.findings);
    if let Some(t) = &document.type_const {
        let claimants = knowledge.schemas_for(t);
        if claimants.len() > 1 {
            let others: Vec<String> = claimants
                .iter()
                .filter(|&&j| j != i)
                .map(|&j| knowledge.schemas[j].doc.path.to_string())
                .collect();
            findings.push(document.finding(
                "SOKF_SCHEMA_13",
                format!(
                    "{} also claims type `{t}`. A type has one schema",
                    others.join(", ")
                ),
            ));
        }
    }
    if let Some(schema) = ctx.compiled.schema(r) {
        findings.extend(self_check(document, schema, r, ctx));
        findings.extend(markers_off(document, schema, ctx));
    }
    findings
}

/// `markers_off` returns one warning when the marker check is off and the
/// schema has a `cited-by` entry that uses `from-marker`. In every other
/// case it returns no warning. When the check is off, sokf reads no file for
/// markers, so it doesn't check the schema's `from-marker` entries.
// sokf-implements: EXT-9_AC-8
fn markers_off(
    document: &crate::model::SchemaDocument,
    schema: &SokfSchema,
    ctx: &Ctx<'_>,
) -> Option<Finding> {
    if ctx.model.marker_check != MarkerCheck::Off {
        return None;
    }
    let first = schema
        .cited_by
        .iter()
        .find(|c| matches!(c.from, CitedFrom::Marker(_)))?;
    Some(
        Finding::new(
            document.doc.path.clone(),
            Some(first.common.line),
            Severity::Warning,
            "EXT-9_AC-8",
            "`markers.check` is `off`, so sokf checks no `cited-by` entry that uses `from-marker`",
        )
        .at(&Locator::schema(document.id()).at("cited-by")),
    )
}

/// `check_typed` checks one document against the one schema of its type
/// (SOKF_SCHEMA_13, SOKF_SCHEMA_14).
// sokf-implements: SOKF_FORMAT_2
// sokf-implements: SOKF_SCHEMA_13
// sokf-implements: SOKF_SCHEMA_14
// sokf-implements: SOKF_VALIDATION_41
pub fn check_typed(knowledge: &Knowledge, doc: &Document, ctx: &Ctx<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(k) = ctx.knowledge else {
        return findings;
    };
    let Some(type_name) = doc.type_name() else {
        return stray_anchors(doc);
    };
    if type_name == "Schema" {
        findings.push(Finding::error(
            doc.path.clone(),
            doc.core.line_of("type"),
            "SOKF_FORMAT_2",
            "a `Schema` document lives in `schemas/` at the knowledge root, nowhere else",
        ));
        return findings;
    }
    ctx.model.note(crate::model::Query::Schema {
        knowledge: k,
        key: type_name.to_string(),
    });
    let claimants = knowledge.schemas_for(type_name);
    match claimants {
        [] => {
            findings.push(Finding::error(
                doc.path.clone(),
                doc.core.line_of("type"),
                "SOKF_SCHEMA_13",
                format!("type `{type_name}` has no schema"),
            ));
            findings.extend(stray_anchors(doc));
        }
        [one] => {
            let r = SchemaRef {
                knowledge: k,
                index: *one,
            };
            match ctx.compiled.schema(r) {
                Some(schema) => findings.extend(check_document(doc, schema, r, ctx)),
                None => findings.push(Finding::error(
                    doc.path.clone(),
                    doc.core.line_of("type"),
                    "SOKF_VALIDATION_41",
                    format!("the schema of `{type_name}` ({}) breaks the grammar, so sokf doesn't check the document", knowledge.schemas[*one].doc.path),
                )),
            }
        }
        many => findings.push(Finding::error(
            doc.path.clone(),
            doc.core.line_of("type"),
            "SOKF_SCHEMA_13",
            format!("type `{type_name}` resolves to {} schemas", many.len()),
        )),
    }
    findings
}

/// `stray_anchors` reports each anchor of a document that no schema governs:
/// only a schema captures keys, so no anchor of such a document names one
/// (SOKF_FORMAT_83). The fix leaves the anchors, because the writer may yet
/// give the document a schema whose keys they name.
// sokf-implements: SOKF_FORMAT_83
fn stray_anchors(doc: &Document) -> Vec<Finding> {
    doc.body
        .anchors
        .iter()
        .map(|a| {
            Finding::error(
                doc.path.clone(),
                Some(a.line),
                "SOKF_FORMAT_83",
                format!(
                    "the anchor `#{}` names no key: no schema governs the document, so it has none",
                    a.id
                ),
            )
        })
        .collect()
}

fn check_document(
    doc: &Document,
    schema: &SokfSchema,
    r: SchemaRef,
    ctx: &Ctx<'_>,
) -> Vec<Finding> {
    let schema_id = ctx.model.schema(r).id();
    let states = states_of(doc.frontmatter.as_ref(), schema);
    let mut findings = check_against(
        doc_view(doc),
        schema,
        &states,
        ctx,
        Locator::schema(schema_id),
    )
    .findings;
    findings.extend(lifecycle::check_directory(&doc.path, schema, &states));
    findings
}

/// `check_additional` checks an additional file against the one schema
/// whose glob matches it.
pub fn check_additional(view: super::DocView<'_>, r: SchemaRef, ctx: &Ctx<'_>) -> Vec<Finding> {
    ctx.model.note(ctx.model.schema_query(r));
    let Some(schema) = ctx.compiled.schema(r) else {
        return vec![Finding::error(
            view.path.clone(),
            None,
            "SOKF_VALIDATION_41",
            format!(
                "the schema {} breaks the grammar, so sokf doesn't check the file",
                ctx.model.schema(r).doc.path
            ),
        )];
    };
    let schema_id = ctx.model.schema(r).id();
    check_against(
        view,
        schema,
        &States::default(),
        ctx,
        Locator::schema(schema_id),
    )
    .findings
}

/// `self_check` checks a schema's scaffolds and examples against the schema
/// with the engine's `check_schema`. The engine derives each scaffold from
/// the schema. It checks each scaffold and each example against the schema's
/// rules. Through `SchemaHost`, it also checks them against the rules that
/// only sokf knows: the core fields of the frontmatter and the schema's
/// `links`. It checks an example as it checks a scaffold, with every rule
/// that depends on the repository turned off, so the links of a scaffold or
/// an example need no real file or document.
// sokf-implements: FORMAT-1_AC-7
fn self_check(
    document: &crate::model::SchemaDocument,
    schema: &SokfSchema,
    r: SchemaRef,
    ctx: &Ctx<'_>,
) -> Vec<Finding> {
    let ctx = Ctx {
        knowledge: Some(r.knowledge),
        scaffold: true,
        ..*ctx
    };
    let host = SchemaHost::new(&ctx, schema);
    engine::check_schema(&schema.core, &host, &document.doc.path, &document.id())
}
