//! The generated-block checks find an include block or a definition block
//! that is stale, unclosed, on a cycle, written in the wrong form, or
//! unresolved (SOKF_FORMAT_77 to SOKF_FORMAT_81, SOKF_FORMAT_85 to
//! SOKF_FORMAT_91, SOKF_FORMAT_95, SOKF_FORMAT_112).

use super::{Ctx, DocView};
use crate::{
    Finding, RepoPath,
    include::{self, RegionError, extract, render_definitions},
    markdown::IncludeTarget,
    source,
};

/// `check` reports each include block of a document that is not current or
/// not well-formed, and does the same for the document's definition block.
// sokf-implements: SOKF_FORMAT_65
// sokf-implements: SOKF_FORMAT_77
// sokf-implements: SOKF_FORMAT_78
// sokf-implements: SOKF_FORMAT_79
// sokf-implements: SOKF_FORMAT_80
// sokf-implements: SOKF_FORMAT_85
// sokf-implements: SOKF_FORMAT_86
// sokf-implements: SOKF_FORMAT_87
// sokf-implements: SOKF_FORMAT_88
// sokf-implements: SOKF_FORMAT_89
// sokf-implements: SOKF_FORMAT_90
// sokf-implements: SOKF_FORMAT_91
// sokf-implements: SOKF_FORMAT_95
// sokf-implements: SOKF_FORMAT_112
// sokf-implements: FORMAT-2_AC-6
pub fn check(doc: DocView<'_>, ctx: &Ctx<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let path = doc.path;
    let fail =
        |findings: &mut Vec<Finding>, line: Option<usize>, msg: String, key: &'static str| {
            findings.push(Finding::error(path.clone(), line, key, msg));
        };
    if let Some(line) = doc.segments.unclosed {
        fail(
            &mut findings,
            Some(line),
            "the include block has no close marker".into(),
            "SOKF_FORMAT_86",
        );
    }
    for line in &doc.segments.stray_closes {
        fail(
            &mut findings,
            Some(*line),
            "a close marker has no open marker".into(),
            "SOKF_FORMAT_86",
        );
    }
    for block in &doc.segments.regions {
        let written = block.target.clone();
        match &IncludeTarget::parse(&block.target) {
            IncludeTarget::Document {
                store,
                id,
                fragment,
            } => {
                let found = ctx.knowledge.and_then(|k| {
                    ctx.model
                        .document_in(k, store.as_deref(), id)
                        .map(|t| (k, t))
                });
                let Some((k, (target_k, target))) = found else {
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!("`sokf:include {written}` resolves to no document"),
                        "SOKF_FORMAT_87",
                    );
                    continue;
                };
                // Each target has one written form: the shortest form that
                // identifies it from this store.
                if let Some(canonical) =
                    ctx.model
                        .canonical_include(k, target_k, id, fragment.as_deref())
                    && canonical != written
                {
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!(
                            "`sokf:include {written}`: from this store, write `sokf:include {canonical}`, the shortest form that identifies the target; `--fix` rewrites it"
                        ),
                        "SOKF_FORMAT_65",
                    );
                }
                // The check treats an include of an asset, whole or by
                // region, as a file-form include (FORMAT-2_AC-5).
                if !include::includes_body(ctx.model, &target.path, fragment.as_deref()) {
                    match ctx.model.asset_of(&target.path) {
                        Some((_, asset)) => {
                            let path = asset.path.clone();
                            check_file_include(
                                doc,
                                ctx,
                                block,
                                &written,
                                &path,
                                fragment.as_deref(),
                                &mut findings,
                            );
                        }
                        None => fail(
                            &mut findings,
                            Some(block.open_line),
                            format!(
                                "`sokf:include {written}` has a fragment, but the document is not a sidecar. A fragment can only identify a region of an asset"
                            ),
                            "SOKF_FORMAT_90",
                        ),
                    }
                    continue;
                }
                if let Some(cycle) = ctx
                    .expected
                    .cycles
                    .iter()
                    .find(|c| c.contains(&target.path) && c.contains(path))
                {
                    let names: Vec<String> = cycle.iter().map(ToString::to_string).collect();
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!(
                            "`sokf:include {written}` lies on a cycle: {}",
                            names.join(", ")
                        ),
                        "SOKF_FORMAT_91",
                    );
                    continue;
                }
                match ctx.expected.include(path, block.open_line) {
                    Some(expected) if expected == block.content => {}
                    Some(_) => fail(
                        &mut findings,
                        Some(block.open_line),
                        format!("`sokf:include {written}` is stale. Run `sokf validate --fix`"),
                        "SOKF_FORMAT_85",
                    ),
                    None => fail(
                        &mut findings,
                        Some(block.open_line),
                        format!(
                            "`sokf:include {written}` cannot be regenerated: {} contains an include that does not resolve",
                            target.path
                        ),
                        "SOKF_FORMAT_85",
                    ),
                }
            }
            IncludeTarget::File { path: file, region } => {
                let Some(rp) = RepoPath::parse(file) else {
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!("`sokf:include {written}` points to a path outside the repository"),
                        "SOKF_FORMAT_87",
                    );
                    continue;
                };
                // A document of any store is included by id. The id form
                // includes the document's body and the path form would
                // include its source, so `--fix` leaves the choice to the
                // writer.
                if let Some((target_k, id)) = ctx.model.id_at(&rp) {
                    let canonical = ctx
                        .knowledge
                        .and_then(|k| ctx.model.canonical_include(k, target_k, &id, None))
                        .unwrap_or(id);
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!(
                            "`sokf:include {written}` refers to a document by path. Include its body by id as `sokf:include {canonical}`, or show its source in a code block"
                        ),
                        "SOKF_FORMAT_88",
                    );
                    continue;
                }
                if ctx.model.is_materialized_in(ctx.knowledge, &rp) {
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!(
                            "`sokf:include {written}` refers by path to an additional file of this store, and an include block cannot include such a file by path"
                        ),
                        "SOKF_FORMAT_89",
                    );
                    continue;
                }
                // An asset of any store is included by its sidecar's id,
                // and that include yields the same file.
                if ctx.model.asset_at(&rp).is_some() {
                    let canonical = ctx
                        .knowledge
                        .and_then(|k| asset_include(ctx.model, k, &rp, region.as_deref()))
                        .unwrap_or_else(|| "<id>".into());
                    fail(
                        &mut findings,
                        Some(block.open_line),
                        format!(
                            "`sokf:include {written}` refers to an asset by path; include it by its sidecar's id as `sokf:include {canonical}`, which `--fix` writes"
                        ),
                        "SOKF_FORMAT_88",
                    );
                    continue;
                }
                check_file_include(
                    doc,
                    ctx,
                    block,
                    &written,
                    &rp,
                    region.as_deref(),
                    &mut findings,
                );
            }
        }
    }
    if let Some(expected) = ctx.expected.definitions.get(path).and_then(|d| d.as_ref()) {
        let rendered = render_definitions(expected);
        let missing = if doc.core.is_some() {
            "SOKF_FORMAT_77"
        } else {
            "SOKF_FORMAT_78"
        };
        match &doc.segments.definition {
            None if expected.is_empty() => {}
            None => fail(&mut findings, None, "the definition block is missing; run `sokf validate --fix`".into(), missing),
            Some(def) if expected.is_empty() => fail(&mut findings, Some(def.line), "the definition block is stale: the body cites no `sokf:` label; run `sokf validate --fix`".into(), "SOKF_FORMAT_80"),
            Some(def) => {
                if def.trailing {
                    fail(&mut findings, Some(def.line), "content follows the definition block, which must come last in the document".into(), "SOKF_FORMAT_79");
                } else if def.text.trim_end() != rendered.trim_end() {
                    fail(&mut findings, Some(def.line), "the definition block is stale. Run `sokf validate --fix`".into(), "SOKF_FORMAT_80");
                }
            }
        }
    }
    findings
}

/// `asset_include` returns the canonical include target that a file of the
/// store `citing` uses for the asset at `path` and, when given, its
/// `region`. The target is the id of the asset's sidecar. Its store prefix
/// is the shortest one that identifies the sidecar (SOKF_FORMAT_65).
pub fn asset_include(
    model: &crate::Model,
    citing: usize,
    path: &RepoPath,
    region: Option<&str>,
) -> Option<String> {
    let (k, asset) = model.asset_at(path)?;
    model.note(crate::model::Query::Path(asset.sidecar_path()));
    let id = model.knowledges[k].documents[asset.sidecar?].id()?;
    model.canonical_include(citing, k, id, region)
}

/// `check_file_include` checks a file-form include, or an asset included by
/// its sidecar's id: it checks that the file exists, that the region can be
/// extracted, and that the block is current.
fn check_file_include(
    doc: DocView<'_>,
    ctx: &Ctx<'_>,
    block: &crate::markdown::Region,
    written: &str,
    rp: &RepoPath,
    region: Option<&str>,
    findings: &mut Vec<Finding>,
) {
    let fail = |findings: &mut Vec<Finding>, msg: String, key: &'static str| {
        findings.push(Finding::error(
            doc.path.clone(),
            Some(block.open_line),
            key,
            msg,
        ));
    };
    if !ctx.model.exists(rp) {
        // An include of a planned file holds the writer's sketch until the
        // file exists, and the fix keeps it (SOKF_VALIDATION_16).
        // sokf-implements: SOKF_SCHEMA_53
        if !ctx.model.is_planned(rp) {
            fail(
                findings,
                format!("`sokf:include {written}` refers to a path where no file exists"),
                "SOKF_FORMAT_87",
            );
        }
        return;
    }
    let text = match source::load_text(&ctx.model.tree.root, rp) {
        Ok(t) => t,
        Err(e) => {
            fail(
                findings,
                format!("`sokf:include {written}`: {e}"),
                "SOKF_FORMAT_87",
            );
            return;
        }
    };
    match extract(&text, region) {
        // The finding belongs to the included file, at the marker that
        // breaks the pairing. The check of each document that includes the
        // file produces the same finding, and the report removes the
        // duplicates.
        // sokf-implements: CLI-4_AC-28
        Err(e) => {
            let key = match &e {
                RegionError::AlreadyOpen { .. } => "SOKF_FORMAT_112",
                _ => "SOKF_FORMAT_95",
            };
            findings.push(Finding::error(
                rp.clone(),
                Some(e.line()),
                key,
                e.to_string(),
            ));
        }
        Ok(None) => fail(
            findings,
            format!(
                "`sokf:include {written}`: {rp} has no region `{}`",
                region.unwrap_or("")
            ),
            "SOKF_FORMAT_87",
        ),
        Ok(Some(_)) => match ctx.expected.include(doc.path, block.open_line) {
            Some(expected) if expected == block.content => {}
            _ => fail(
                findings,
                format!("`sokf:include {written}` is stale. Run `sokf validate --fix`"),
                "SOKF_FORMAT_85",
            ),
        },
    }
}
