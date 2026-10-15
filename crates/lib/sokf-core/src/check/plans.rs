//! This module finds the files that a document plans. A schema's
//! `plans-paths` names its planning sections, and while the document is in
//! the states of an entry's `only`, each `/`-rooted path that such a section
//! links is planned (SOKF_SCHEMA_53).

use std::collections::BTreeSet;

use tramlines_md::markdown::LinkTarget;

use super::states_of;
use crate::{Compiled, Document, Model, RepoPath, grammar::SokfSchema, model::PlanIndex};

/// `planned_by` returns the paths that `doc` plans under `schema`: the
/// `/`-rooted path links inside each planning section that the document's
/// states admit. A planning section runs from its heading to the next
/// heading of the same or a higher level.
// sokf-implements: SOKF_SCHEMA_53
pub fn planned_by(doc: &Document, schema: &SokfSchema) -> BTreeSet<RepoPath> {
    let mut out = BTreeSet::new();
    if schema.plans_paths.is_empty() {
        return out;
    }
    let states = states_of(doc.frontmatter.as_ref(), schema);
    let value = |key: &str| {
        doc.frontmatter
            .as_ref()
            .and_then(|f| f.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    let headings = &doc.body.headings;
    for plan in &schema.plans_paths {
        if !plan.common.only.admits(&states) {
            continue;
        }
        let Some(section) = schema
            .core
            .sections
            .iter()
            .find(|s| s.rule_id.as_deref() == Some(plan.section.as_str()))
        else {
            continue;
        };
        for (i, h) in headings.iter().enumerate() {
            if h.level != section.level || !section.heading.shape.matches(&h.text, value) {
                continue;
            }
            let end = headings[i + 1..]
                .iter()
                .find(|n| n.level <= section.level)
                .map_or(usize::MAX, |n| n.line);
            for link in &doc.body.links {
                if link.line > h.line
                    && link.line < end
                    && let LinkTarget::Repo { path, .. } = &link.target
                {
                    out.insert(path.clone());
                }
            }
        }
    }
    out
}

/// `capture_plans_of` returns the plans of the loaded documents in `only`,
/// or of every loaded document when `only` is `None`.
pub fn capture_plans_of(
    model: &Model,
    compiled: &Compiled,
    only: Option<&BTreeSet<RepoPath>>,
) -> PlanIndex {
    let mut plans = PlanIndex::default();
    for (k, doc) in model.documents() {
        if only.is_some_and(|o| !o.contains(&doc.path)) {
            continue;
        }
        let Some((_, schema)) = compiled.for_doc(model, k, doc) else {
            continue;
        };
        for target in planned_by(doc, schema) {
            plans.add(&doc.path, &target);
        }
    }
    plans
}
