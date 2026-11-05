//! This module checks the targets of the `sokf:` links in each unit that a
//! unit rule with `to-type` accepts. Each such link resolves to a document
//! of one of the rule's types, and a link with a fragment counts by the type
//! of the document that the fragment lies in (SOKF_SCHEMA_56).

use tramlines_md::{
    check::{CheckOutput, DocView},
    markdown::LinkTarget,
};

use super::{Ctx, coverage::or_list};
use crate::{
    Finding, Located, Locator, RepoPath,
    grammar::SokfSchema,
    model::keys::{self, Label},
};

/// `check` reports each `sokf:` link, in a unit that the engine records as
/// accepted by a rule with `to-type`, whose target is a document of another
/// type. It reports the link at its own line. A link that resolves to
/// nothing has a finding of its own (SOKF_FORMAT_61), so `check` passes it
/// by.
// sokf-implements: EXT-5_AC-2
// sokf-implements: EXT-5_AC-3
// sokf-implements: SOKF_SCHEMA_56
pub fn check(
    doc: &DocView<'_>,
    schema: &SokfSchema,
    ctx: &Ctx<'_>,
    locator: &Locator,
    out: &mut CheckOutput,
) {
    if ctx.scaffold || schema.unit_types.is_empty() {
        return;
    }
    let Some(k) = ctx.knowledge else {
        return;
    };
    let mut findings = Vec::new();
    for unit in &out.accepted {
        let Some(types) = schema.unit_types.get(&unit.rule_index) else {
            continue;
        };
        let end = unit.end_line.max(unit.line);
        for link in &doc.body.links {
            if link.line < unit.line || link.line > end {
                continue;
            }
            let LinkTarget::Host(label) = &link.target else {
                continue;
            };
            let (Ok(target), _) =
                keys::resolve_slugged(ctx.model, ctx.keys, k, Label::parse(label))
            else {
                continue;
            };
            let Some(found) = type_outside(ctx, &target.path, target.knowledge, types) else {
                continue;
            };
            let what = match found {
                Some(t) => format!("a document of type `{t}`"),
                None => "a document of another type".to_string(),
            };
            let rule = match &unit.rule_id {
                Some(id) => format!("`to-type` of rule `{id}`"),
                None => match schema.core.unit(unit.rule_index) {
                    Some(u) => format!("`to-type` of the rule at line {}", u.common.line),
                    None => "`to-type` of its rule".to_string(),
                },
            };
            findings.push(
                Finding::error(
                    doc.path.clone(),
                    Some(link.line),
                    "SOKF_SCHEMA_56",
                    format!(
                        "`[sokf:{label}]` links {what}, but its rule accepts only a link to a document of type {}",
                        or_list(types)
                    ),
                )
                .at(&locator.at(rule)),
            );
        }
    }
    out.findings.extend(findings);
}

/// `type_outside` returns `None` when the document at `path` has one of
/// `types`. Otherwise it returns the document's type, which is `None` when
/// the run holds only the store's summary of the document. The summary
/// answers which documents of a store have a type, but not the type of one
/// document.
fn type_outside(
    ctx: &Ctx<'_>,
    path: &RepoPath,
    knowledge: usize,
    types: &[String],
) -> Option<Option<String>> {
    let held = match ctx.model.by_path(path) {
        Some(Located::Document(_, d)) => Some(d.type_name()),
        Some(Located::Schema(_, s)) => Some(s.doc.type_name()),
        _ => None,
    };
    match held {
        Some(t) => match t {
            Some(t) if types.iter().any(|w| w == t) => None,
            t => Some(t.map(str::to_string)),
        },
        None => {
            let typed = types.iter().any(|t| {
                ctx.model
                    .ids_of_type(knowledge, t)
                    .iter()
                    .any(|(p, _)| p == path)
            });
            (!typed).then_some(None)
        }
    }
}
