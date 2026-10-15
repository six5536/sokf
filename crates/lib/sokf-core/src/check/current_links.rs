//! This module checks the paragraphs that make a promise. A schema's
//! `current-links` lists their patterns, and such a paragraph links no
//! archived document, because the work that an archived document records is
//! done or dropped (SOKF_SCHEMA_54).

use tramlines_md::{
    check::{CheckOutput, DocView},
    markdown::{BlockKind, LinkTarget},
};

use super::Ctx;
use crate::{Finding, grammar::SokfSchema, model::keys::Label};

/// `check` reports each `sokf:` link to an archived document inside a
/// paragraph that one of the schema's `current-links` patterns matches.
// sokf-implements: SOKF_SCHEMA_54
pub fn check(doc: &DocView<'_>, schema: &SokfSchema, ctx: &Ctx<'_>, out: &mut CheckOutput) {
    if ctx.scaffold || schema.current_links.is_empty() {
        return;
    }
    let Some(k) = ctx.knowledge else {
        return;
    };
    let mut archived = None;
    for block in doc
        .body
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::Prose)
    {
        for unit in &block.units {
            let Some(pattern) = schema.current_links.iter().find(|p| p.is_match(&unit.text)) else {
                continue;
            };
            let end = unit.end_line.max(unit.line);
            for link in &doc.body.links {
                let LinkTarget::Host(label) = &link.target else {
                    continue;
                };
                if link.line < unit.line || link.line > end {
                    continue;
                }
                let parsed = Label::parse(label);
                let Some(id) = parsed.id else {
                    continue;
                };
                let store = match parsed.store {
                    Some(s) => match ctx.model.store(k, s) {
                        Some(hit) => hit.knowledge,
                        None => continue,
                    },
                    None => k,
                };
                let Some(path) = ctx.model.path_of_id(store, id) else {
                    continue;
                };
                let archived = archived.get_or_insert_with(|| ctx.model.archived_documents());
                if archived.contains(&path) {
                    out.findings.push(Finding::error(
                        doc.path.clone(),
                        Some(link.line),
                        "SOKF_SCHEMA_54",
                        format!(
                            "`[sokf:{label}]` links an archived document from a paragraph that matches `{}`, which links only current documents",
                            pattern.as_str()
                        ),
                    ));
                }
            }
        }
    }
}
