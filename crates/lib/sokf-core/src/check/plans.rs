//! This module finds the files that a document plans. A schema's
//! `plans-paths` names its planning sections, and while the document is in
//! the states of an entry's `only`, each `/`-rooted path that such a section
//! links is planned (`SOKF_SCHEMA_53`). A link that ends with `/` plans a
//! directory and every path under it (`SOKF_SCHEMA_55`).

use std::collections::BTreeSet;

use tramlines_md::{check, markdown::LinkTarget};

use super::{doc_view, headings, states_of};
use crate::{
    Compiled, Document, Model, RepoPath,
    grammar::SokfSchema,
    model::{PlanIndex, Planned},
};

/// `planned_by` returns the paths that `doc` plans under `schema`: the
/// `/`-rooted path links inside each planning section that the document's
/// states admit. A heading opens a planning section where the check matches
/// it to the section, at the section's level, so that a formatter's
/// spelling of the heading's markup changes no plan. A planning section
/// runs from its heading to the next heading of the same or a higher level.
/// A link that ends with `/` plans a directory.
// sokf-implements: SOKF_SCHEMA_53
// sokf-implements: SOKF_SCHEMA_55
// sokf-implements: SOKF_SCHEMA_109
pub fn planned_by(doc: &Document, schema: &SokfSchema) -> BTreeSet<Planned> {
    let mut out = BTreeSet::new();
    if schema.plans_paths.is_empty() {
        return out;
    }
    let states = states_of(doc.frontmatter.as_ref(), schema);
    let headings = &doc.body.headings;
    let mut matches = None;
    for plan in &schema.plans_paths {
        if !plan.common.only.admits(&states) {
            continue;
        }
        let Some(j) = schema
            .core
            .sections
            .iter()
            .position(|s| s.rule_id.as_deref() == Some(plan.section.as_str()))
        else {
            continue;
        };
        let section = &schema.core.sections[j];
        let matches = matches.get_or_insert_with(|| {
            check::match_headings(doc_view(doc).engine(), &schema.core, &states)
        });
        for (i, h) in headings.iter().enumerate() {
            if !headings::opens(doc, &schema.core.sections, matches, i, j) {
                continue;
            }
            let end = headings[i + 1..]
                .iter()
                .find(|n| n.level <= section.level)
                .map_or(usize::MAX, |n| n.line);
            for link in &doc.body.links {
                if link.line > h.line
                    && link.line < end
                    && let LinkTarget::Repo { path, dir, .. } = &link.target
                {
                    out.insert(Planned {
                        path: path.clone(),
                        dir: *dir,
                    });
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
