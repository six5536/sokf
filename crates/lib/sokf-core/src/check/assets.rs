//! This module checks assets and their sidecars (FORMAT-1): every asset has a
//! sidecar, a sidecar has its asset's type, a document of an asset type is a
//! sidecar, and a sidecar's `resource` is its asset's path. The checks never
//! read an asset.

use super::Ctx;
use crate::{Document, Finding, Knowledge, model::Resource};

/// `check_assets` reports each asset in the knowledge that has no sidecar.
// sokf-implements: FORMAT-1_AC-3
pub fn check_assets(knowledge: &Knowledge) -> Vec<Finding> {
    let mut findings = Vec::new();
    for asset in &knowledge.assets {
        let schema = &knowledge.schemas[asset.schema.index];
        if !asset.has_sidecar {
            findings.push(Finding::error(
                asset.path.clone(),
                None,
                "SOKF_FORMAT_4",
                format!(
                    "the asset has no sidecar. Create `{}` with `type: {}` and an id",
                    asset.sidecar_path(),
                    schema.type_const.as_deref().unwrap_or("<type>")
                ),
            ));
        }
    }
    findings
}

/// `check_document` checks one document as a sidecar: its type must be the
/// type of its asset's schema, and its `resource` must be its asset's path.
/// It also checks that a document of a type with `asset-files` has an asset
/// beside it.
// sokf-implements: FORMAT-1_AC-4
// sokf-implements: FORMAT-1_AC-5
// sokf-implements: FORMAT-1_AC-6
pub fn check_document(knowledge: &Knowledge, doc: &Document, ctx: &Ctx<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(k) = ctx.knowledge else {
        return findings;
    };
    if let Some((_, asset)) = ctx.model.asset_of(&doc.path) {
        let schema = &knowledge.schemas[asset.schema.index];
        let sidecar = doc;
        if let (Some(have), Some(want)) = (sidecar.type_name(), schema.type_const.as_deref())
            && have != want
        {
            findings.push(Finding::error(
                sidecar.path.clone(),
                sidecar.core.line_of("type"),
                "SOKF_SCHEMA_12",
                format!(
                    "the sidecar of `{}` has type `{have}`. The glob that matches the asset belongs to type `{want}`",
                    asset.path
                ),
            ));
        }
        let expected = asset.path.as_str();
        let have = sidecar.core.resource.as_ref();
        if have.and_then(|r| r.one()) != Some(expected) {
            let message = match have {
                Some(Resource::One(have)) => format!(
                    "`resource` is `{have}`; a sidecar's `resource` is its asset's path, `{expected}`"
                ),
                Some(Resource::List(_)) => format!(
                    "`resource` is a list; a sidecar's `resource` is its asset's path, {expected}"
                ),
                None => format!(
                    "`resource` is missing; a sidecar's `resource` is its asset's path, {expected}"
                ),
            };
            findings.push(Finding::error(
                sidecar.path.clone(),
                sidecar
                    .core
                    .line_of("resource")
                    .or_else(|| sidecar.core.line_of("id")),
                "SOKF_FORMAT_31",
                message,
            ));
        }
    }
    // A document of a type with `asset-files` must be a sidecar.
    if let Some(type_name) = doc.type_name()
        && let Some((_, schema)) = ctx.compiled.for_type(ctx.model, k, type_name)
        && schema.has_assets()
        && ctx.model.asset_of(&doc.path).is_none()
    {
        findings.push(Finding::error(
            doc.path.clone(),
            doc.core.line_of("type"),
            "SOKF_SCHEMA_11",
            format!(
                "no asset sits beside this document. Type `{type_name}` declares `asset-files`, so its documents are sidecars"
            ),
        ));
    }
    findings
}
