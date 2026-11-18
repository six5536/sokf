//! This module checks a document's `links` against the rules in its schema's
//! `links`, which list the edges a document must declare and the edges it may
//! declare.

use tramlines_core::text;

use super::{CheckOutput, Ctx, Gate};
use crate::{
    Finding, Locator, RepoPath,
    grammar::{LinkRule, SokfSchema, States},
    graph::{Target, resolve_to},
    model::CoreFields,
};

/// `check` checks the `links` of the document at `path`, from its core
/// fields `core`, against the edges that its schema demands and permits. Each finding names its schema and the link's `rel` in
/// its location. Each gated rule, a rule whose `only` limits it to some
/// lifecycle states, records the entries that it matched.
// sokf-implements: SOKF_SCHEMA_33
// sokf-implements: SOKF_SCHEMA_34
pub fn check(
    path: &RepoPath,
    core: &CoreFields,
    schema: &SokfSchema,
    states: &States,
    ctx: &Ctx<'_>,
    locator: &Locator,
    out: &mut CheckOutput,
) {
    let entries = &core.links;
    // A finding from a link rule names the rule's `rel` in its location,
    // because a reader looks for that `rel` in the schema's `links`.
    let at = |rel: &str| locator.at(format!("link `{rel}`"));
    for rule in schema.links.iter().filter(|r| r.common.only.admits(states)) {
        if rule.common.only.is_gated() {
            for e in entries.iter().filter(|e| e.rel == rule.rel) {
                out.gates.push(Gate {
                    line: e.line,
                    only: rule.common.only.clone(),
                    rule: rule.common.line,
                });
            }
        }
        let count = entries
            .iter()
            .filter(|e| e.rel == rule.rel)
            .filter(|e| {
                if ctx.scaffold {
                    return true;
                }
                let Some(k) = ctx.knowledge else {
                    return false;
                };
                match resolve_to(ctx.model, k, &e.to) {
                    Some(Target::Document {
                        path, knowledge, ..
                    }) => {
                        let target = &ctx.model.knowledges[knowledge];
                        rule.to_type.is_empty()
                            || target
                                .by_path
                                .get(&path)
                                .and_then(|&i| target.documents[i].type_name())
                                .is_some_and(|t| rule.to_type.iter().any(|w| w == t))
                    }
                    Some(Target::File(_)) => rule.to_type.is_empty(),
                    None => false,
                }
            })
            .count();
        let line = entries
            .iter()
            .find(|e| e.rel == rule.rel)
            .map(|e| e.line)
            .or_else(|| core.line_of("links"));
        let mut emit = |message: String| {
            out.findings.push(
                Finding::new(
                    path.clone(),
                    line,
                    rule.common.severity,
                    "SOKF_SCHEMA_33",
                    message,
                )
                .at(&at(&rule.rel)),
            );
        };
        if count < rule.min {
            let types = if rule.to_type.is_empty() {
                String::new()
            } else {
                format!(
                    " to a document of type {}",
                    super::coverage::or_list(&rule.to_type)
                )
            };
            emit(format!(
                "`links` needs at least {} `{}` entr{}{types}, but it has {count}",
                rule.min,
                rule.rel,
                if rule.min == 1 { "y" } else { "ies" }
            ));
        }
        if let Some(message) = too_many(rule, count) {
            emit(message);
        }
    }
    if !schema.links_open {
        for entry in entries {
            if !schema.links.iter().any(|r| r.rel == entry.rel) {
                out.findings.push(
                    Finding::error(
                        path.clone(),
                        Some(entry.line),
                        "SOKF_SCHEMA_34",
                        format!(
                            "the schema declares no `links` rule for `rel: {}`",
                            entry.rel
                        ),
                    )
                    .at(locator),
                );
            }
        }
    }
}

/// `too_many` returns the words of the finding on `count` links of `rule`,
/// ending with the way out of its limit, where the links pass its hard
/// limit, or `None` where they don't or the rule has no hard limit. A soft
/// limit bounds nothing.
// sokf-implements: EXT-8_AC-2
// sokf-implements: EXT-8_AC-3
// sokf-implements: SOKF_SCHEMA_103
fn too_many(rule: &LinkRule, count: usize) -> Option<String> {
    let limit = rule.max.as_ref()?;
    let max = limit.hard().filter(|&max| count > max)?;
    let entries = if max == 1 { "entry" } else { "entries" };
    Some(text::at_limit(
        format!(
            "`links` allows at most {max} `{}` {entries}, but it has {count}",
            rule.rel
        ),
        &limit.when_at_limit,
    ))
}
