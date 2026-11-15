//! The generated-block checks find an include block or a definition block
//! that is stale, unclosed, on a cycle, written in the wrong form, or
//! unresolved (`SOKF_FORMAT_77` to `SOKF_FORMAT_81`, `SOKF_FORMAT_85` to
//! `SOKF_FORMAT_91`, `SOKF_FORMAT_95`, `SOKF_FORMAT_112`).

use super::{Citations, Ctx, DocView, deprecated::Citer};
use crate::{
    Finding, RepoPath,
    include::{self, RegionError, extract, render_definitions},
    markdown::{IncludeTarget, Region},
    source,
};

/// `check` reports each include block of a document that is not current or
/// not well-formed, and does the same for the document's definition block.
/// It records each include block that resolves to a document in
/// `citations`, for the pass of a deprecated document (`SOKF_VALIDATION_59`).
// sokf-implements: SOKF_FORMAT_85
// sokf-implements: SOKF_FORMAT_86
// sokf-implements: SOKF_FORMAT_87
// sokf-implements: SOKF_FORMAT_95
// sokf-implements: SOKF_FORMAT_112
// sokf-implements: FORMAT-2_AC-6
// sokf-implements: CLI-4_AC-45
// sokf-implements: CLI-4_AC-46
// sokf-implements: CLI-4_AC-47
// sokf-implements: CLI-4_AC-49
pub fn check(doc: DocView<'_>, ctx: &Ctx<'_>, citations: &mut Citations) -> Vec<Finding> {
    let mut pass = Pass {
        doc,
        ctx,
        citer: Citer::of(doc, ctx),
        citations,
        findings: Vec::new(),
    };
    if let Some(line) = doc.segments.unclosed {
        pass.fail(
            Some(line),
            "the include block has no close marker".into(),
            "SOKF_FORMAT_86",
        );
    }
    for line in &doc.segments.stray_closes {
        pass.fail(
            Some(*line),
            "a close marker has no open marker".into(),
            "SOKF_FORMAT_86",
        );
    }
    for block in &doc.segments.regions {
        match &IncludeTarget::parse(&block.target) {
            IncludeTarget::Document {
                store,
                id,
                fragment,
            } => pass.document_include(block, store.as_deref(), id, fragment.as_deref()),
            IncludeTarget::File { path: file, region } => {
                pass.path_include(block, file, region.as_deref());
            }
        }
    }
    pass.definition_block();
    pass.findings
}

/// A `Pass` holds the state that the checks of one document's generated
/// blocks share: the document, the context, the citations that the
/// include blocks resolve, and the findings so far.
struct Pass<'a> {
    /// The document whose blocks the pass checks.
    doc: DocView<'a>,
    /// The context of the check.
    ctx: &'a Ctx<'a>,
    /// The document as a source of citations, or `None` when its includes
    /// cite nothing.
    citer: Option<Citer>,
    /// The citations of the run.
    citations: &'a mut Citations,
    /// The findings of the pass, in the order the pass reports them.
    findings: Vec<Finding>,
}

impl Pass<'_> {
    /// `fail` reports an error at `line` of the document.
    fn fail(&mut self, line: Option<usize>, msg: String, key: &'static str) {
        self.findings
            .push(Finding::error(self.doc.path.clone(), line, key, msg));
    }

    /// `document_include` checks an include block whose target is a
    /// document id: it checks that the target resolves and is written in
    /// its canonical form, records the citation, and checks the block as a
    /// file-form include of an asset or as an include of a body.
    // sokf-implements: SOKF_FORMAT_65
    // sokf-implements: SOKF_FORMAT_90
    fn document_include(
        &mut self,
        block: &Region,
        store: Option<&str>,
        id: &str,
        fragment: Option<&str>,
    ) {
        let ctx = self.ctx;
        let written = &block.target;
        let found = ctx
            .knowledge
            .and_then(|k| ctx.model.document_in(k, store, id).map(|t| (k, t)));
        let Some((k, (target_k, target))) = found else {
            self.fail(
                Some(block.open_line),
                format!("`sokf:include {written}` resolves to no document"),
                "SOKF_FORMAT_87",
            );
            return;
        };
        if let Some(citer) = &self.citer {
            self.citations.add(
                target.path.clone(),
                citer.cite(&target.path, None, block.open_line, true),
            );
        }
        // Each target has one written form: the shortest form that
        // identifies it from this store.
        if let Some(canonical) = ctx.model.canonical_include(k, target_k, id, fragment)
            && canonical != *written
        {
            self.fail(
                Some(block.open_line),
                format!(
                    "`sokf:include {written}`: from this store, write `sokf:include {canonical}`, the shortest form that identifies the target; `--fix` rewrites it"
                ),
                "SOKF_FORMAT_65",
            );
        }
        // The check treats an include of an asset, whole or by
        // region, as a file-form include (FORMAT-2_AC-5).
        if !include::includes_body(ctx.model, &target.path, fragment) {
            match ctx.model.asset_of(&target.path) {
                Some((_, asset)) => {
                    check_file_include(
                        self.doc,
                        ctx,
                        block,
                        written,
                        &asset.path,
                        fragment,
                        &mut self.findings,
                    );
                }
                None => self.fail(
                    Some(block.open_line),
                    format!(
                        "`sokf:include {written}` has a fragment, but the document is not a sidecar. A fragment can only identify a region of an asset"
                    ),
                    "SOKF_FORMAT_90",
                ),
            }
            return;
        }
        self.body_include(block, &target.path);
    }

    /// `body_include` checks an include block that includes the body of the
    /// document at `target`: it reports a block on a cycle of includes, and
    /// otherwise a block that is not current.
    // sokf-implements: SOKF_FORMAT_91
    fn body_include(&mut self, block: &Region, target: &RepoPath) {
        let ctx = self.ctx;
        let path = self.doc.path;
        let written = &block.target;
        if let Some(cycle) = ctx
            .expected
            .cycles
            .iter()
            .find(|c| c.contains(target) && c.contains(path))
        {
            let names: Vec<String> = cycle.iter().map(ToString::to_string).collect();
            self.fail(
                Some(block.open_line),
                format!(
                    "`sokf:include {written}` lies on a cycle: {}",
                    names.join(", ")
                ),
                "SOKF_FORMAT_91",
            );
            return;
        }
        match ctx.expected.include_current(path, block.open_line, &block.content) {
            Some(true) => {}
            Some(false) => {
                self.findings.extend(ctx.expected.include_warning(
                    path,
                    block.open_line,
                    &block.content,
                ));
                self.fail(
                    Some(block.open_line),
                    format!("`sokf:include {written}` is stale. Run `sokf validate --fix`"),
                    "SOKF_FORMAT_85",
                );
            }
            None => self.fail(
                Some(block.open_line),
                format!(
                    "`sokf:include {written}` cannot be regenerated: {target} contains an include that does not resolve"
                ),
                "SOKF_FORMAT_85",
            ),
        }
    }

    /// `path_include` checks an include block whose target is a path: it
    /// reports a path outside the repository, and a path to a document, to
    /// an additional file, or to an asset, which an include cannot name by
    /// path, and checks any other file as a file-form include.
    // sokf-implements: SOKF_FORMAT_88
    // sokf-implements: SOKF_FORMAT_89
    fn path_include(&mut self, block: &Region, file: &str, region: Option<&str>) {
        let ctx = self.ctx;
        let written = &block.target;
        let Some(rp) = RepoPath::parse(file) else {
            self.fail(
                Some(block.open_line),
                format!("`sokf:include {written}` points to a path outside the repository"),
                "SOKF_FORMAT_87",
            );
            return;
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
            self.fail(
                Some(block.open_line),
                format!(
                    "`sokf:include {written}` refers to a document by path. Include its body by id as `sokf:include {canonical}`, or show its source in a code block"
                ),
                "SOKF_FORMAT_88",
            );
            return;
        }
        if ctx.model.is_materialized_in(ctx.knowledge, &rp) {
            self.fail(
                Some(block.open_line),
                format!(
                    "`sokf:include {written}` refers by path to an additional file of this store, and an include block cannot include such a file by path"
                ),
                "SOKF_FORMAT_89",
            );
            return;
        }
        // An asset of any store is included by its sidecar's id,
        // and that include yields the same file.
        if ctx.model.asset_at(&rp).is_some() {
            let canonical = ctx
                .knowledge
                .and_then(|k| asset_include(ctx.model, k, &rp, region))
                .unwrap_or_else(|| "<id>".into());
            self.fail(
                Some(block.open_line),
                format!(
                    "`sokf:include {written}` refers to an asset by path; include it by its sidecar's id as `sokf:include {canonical}`, which `--fix` writes"
                ),
                "SOKF_FORMAT_88",
            );
            return;
        }
        check_file_include(
            self.doc,
            ctx,
            block,
            written,
            &rp,
            region,
            &mut self.findings,
        );
    }

    /// `definition_block` reports a definition block that is missing,
    /// stale, or followed by other content, when the document has an
    /// expected definition block.
    // sokf-implements: SOKF_FORMAT_77
    // sokf-implements: SOKF_FORMAT_78
    // sokf-implements: SOKF_FORMAT_79
    // sokf-implements: SOKF_FORMAT_80
    fn definition_block(&mut self) {
        let doc = self.doc;
        let Some(expected) = self
            .ctx
            .expected
            .definitions
            .get(doc.path)
            .and_then(|d| d.as_ref())
        else {
            return;
        };
        let rendered = render_definitions(expected);
        let missing = if doc.core.is_some() {
            "SOKF_FORMAT_77"
        } else {
            "SOKF_FORMAT_78"
        };
        match &doc.segments.definition {
            None if expected.is_empty() => {}
            None => self.fail(None, "the definition block is missing; run `sokf validate --fix`".into(), missing),
            Some(def) if expected.is_empty() => self.fail(Some(def.line), "the definition block is stale: the body cites no `sokf:` label; run `sokf validate --fix`".into(), "SOKF_FORMAT_80"),
            Some(def) => {
                let (rendered, written) = (rendered.trim_end(), def.text.trim_end());
                if def.trailing {
                    self.fail(Some(def.line), "content follows the definition block, which must come last in the document".into(), "SOKF_FORMAT_79");
                } else if !self.ctx.expected.layout.same("md", rendered, written) {
                    self.findings.extend(self.ctx.expected.layout.warning("md", rendered, written));
                    self.fail(Some(def.line), "the definition block is stale. Run `sokf validate --fix`".into(), "SOKF_FORMAT_80");
                }
            }
        }
    }
}

/// `asset_include` returns the canonical include target that a file of the
/// store `citing` uses for the asset at `path` and, when given, its
/// `region`. The target is the id of the asset's sidecar. Its store prefix
/// is the shortest one that identifies the sidecar (`SOKF_FORMAT_65`).
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
    block: &Region,
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
        Ok(Some(_)) => {
            if ctx
                .expected
                .include_current(doc.path, block.open_line, &block.content)
                != Some(true)
            {
                findings.extend(ctx.expected.include_warning(
                    doc.path,
                    block.open_line,
                    &block.content,
                ));
                fail(
                    findings,
                    format!("`sokf:include {written}` is stale. Run `sokf validate --fix`"),
                    "SOKF_FORMAT_85",
                );
            }
        }
    }
}
