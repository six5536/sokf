//! This module reports the citations of a deprecated document. The link
//! check, the include check, and the marker check record each citation with
//! what the citing document says about itself, and the pass of each
//! deprecated document reads them after every document is checked
//! (SOKF_VALIDATION_57 to SOKF_VALIDATION_64).

use std::collections::BTreeSet;

use super::{Citation, Citations, Ctx, DocView};
use crate::{
    Document, Finding, MarkerCheck, Model, RepoPath,
    graph::{Target, resolve_to},
    model::{Query, Status},
};

/// A `Citer` describes the file that holds a citation, as the citation
/// records it: its type, whether it is retired, and the documents that it
/// supersedes.
#[derive(Debug, Clone)]
pub struct Citer {
    source: RepoPath,
    source_type: String,
    retired: bool,
    superseded: BTreeSet<RepoPath>,
}

impl Citer {
    /// `of` describes the document or additional file of `doc`. It returns
    /// `None` for a document without a type, whose links cite nothing. A
    /// document is retired when it is archived or deprecated
    /// (SOKF_VALIDATION_61, SOKF_VALIDATION_62). It supersedes each document
    /// that a `supersedes` entry of its `links` resolves to
    /// (SOKF_VALIDATION_60).
    pub fn of(doc: DocView<'_>, ctx: &Ctx<'_>) -> Option<Citer> {
        let mut citer = Citer {
            source: doc.path.clone(),
            source_type: String::new(),
            retired: false,
            superseded: BTreeSet::new(),
        };
        let Some(core) = doc.core else {
            return Some(citer);
        };
        citer.source_type = core.type_.clone()?;
        let Some(k) = ctx.knowledge else {
            return Some(citer);
        };
        citer.retired = core.status == Some(Status::Deprecated)
            || ctx
                .model
                .is_archived(k, core.type_.as_deref(), doc.frontmatter);
        citer.superseded = core
            .links
            .iter()
            .filter(|l| l.rel == "supersedes")
            .filter_map(|l| match resolve_to(ctx.model, k, &l.to)? {
                Target::Document { path, .. } => Some(path),
                Target::File(_) => None,
            })
            .collect();
        Some(citer)
    }

    /// `cite` returns the citation of `target`, or of its key or heading
    /// `slug`, at `line`. With `include`, the citation is an include block.
    pub fn cite(
        &self,
        target: &RepoPath,
        slug: Option<String>,
        line: usize,
        include: bool,
    ) -> Citation {
        Citation {
            slug,
            source: self.source.clone(),
            source_type: self.source_type.clone(),
            rel: None,
            line,
            include,
            supersedes: self.superseded.contains(target),
            retired: self.retired,
            declared: false,
        }
    }

    /// `declare` returns the record that the `links` entry at `line`
    /// supersedes `target`. It cites nothing and draws no warning, but the
    /// pass of `target` names its document as a replacement, even when no
    /// body link mirrors the entry (SOKF_VALIDATION_64).
    pub fn declare(&self, line: usize) -> Citation {
        Citation {
            slug: None,
            source: self.source.clone(),
            source_type: self.source_type.clone(),
            rel: None,
            line,
            include: false,
            supersedes: true,
            retired: self.retired,
            declared: true,
        }
    }
}

/// `check_document` reports each citation of `doc` when its status is
/// `deprecated`. Each finding is a warning at the line of the citation, in
/// the file that holds it. The pass leaves out a citation from a document
/// that supersedes `doc` or that is retired, and a marker when the run reads
/// no markers. Each warning names the documents that supersede `doc`: each
/// document that declares a `supersedes` entry for it, with or without a
/// body link. A frontmatter `links` entry is no citation, so it gets no
/// warning.
// sokf-implements: SOKF_VALIDATION_57
// sokf-implements: SOKF_VALIDATION_58
// sokf-implements: SOKF_VALIDATION_59
// sokf-implements: SOKF_VALIDATION_60
// sokf-implements: SOKF_VALIDATION_61
// sokf-implements: SOKF_VALIDATION_62
// sokf-implements: SOKF_VALIDATION_63
// sokf-implements: SOKF_VALIDATION_64
// sokf-implements: CLI-4_AC-36
pub fn check_document(model: &Model, citations: &Citations, doc: &Document) -> Vec<Finding> {
    if doc.core.status != Some(Status::Deprecated) {
        return Vec::new();
    }
    model.note(Query::Citations(doc.path.clone()));
    let markers_off = model.marker_check == MarkerCheck::Off;
    let cited: Vec<&Citation> = citations
        .all_of(&doc.path)
        .filter(|c| !(markers_off && c.rel.is_some()))
        .collect();
    // A warning holds the line of its citation, and the id of each document
    // that supersedes this one. A change to a citing file can move either,
    // so the pass reads the body of each citing file.
    for c in &cited {
        model.note(Query::Body(c.source.clone()));
    }
    let mut superseders: Vec<String> = cited
        .iter()
        .filter(|c| c.supersedes)
        .map(|c| match model.id_of(&c.source) {
            Some((_, id)) => id,
            None => c.source.to_string(),
        })
        .collect();
    superseders.sort();
    superseders.dedup();
    let name = doc.id().unwrap_or(doc.path.as_str());
    let replaced = replaced_by(&superseders);
    cited
        .iter()
        .filter(|c| !c.supersedes && !c.retired)
        .map(|c| {
            let at = match &c.slug {
                Some(slug) => format!("`{name}#{slug}`, in `{name}`"),
                None => format!("`{name}`"),
            };
            let (key, message) = match (&c.rel, c.include) {
                (Some(rel), _) => (
                    "SOKF_VALIDATION_58",
                    format!("`sokf-{rel}:` cites {at}, which is deprecated{replaced}"),
                ),
                (None, true) => (
                    "SOKF_VALIDATION_59",
                    format!("`sokf:include` includes {at}, which is deprecated{replaced}"),
                ),
                (None, false) => (
                    "SOKF_VALIDATION_57",
                    format!("link to {at}, which is deprecated{replaced}"),
                ),
            };
            Finding::warning(c.source.clone(), Some(c.line), key, message)
        })
        .collect()
}

/// `replaced_by` returns the clause that names the documents that supersede
/// a deprecated one, such as `` : `design-cache` supersedes it ``, or an
/// empty string when no document does.
fn replaced_by(superseders: &[String]) -> String {
    let names: Vec<String> = superseders.iter().map(|s| format!("`{s}`")).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => format!(": {one} supersedes it"),
        [first, second] => format!(": {first} and {second} supersede it"),
        [rest @ .., last] => format!(": {}, and {last} supersede it", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clause_names_each_superseder_in_a_list() {
        let names = |list: &[&str]| -> Vec<String> { list.iter().map(|s| s.to_string()).collect() };
        assert_eq!(replaced_by(&[]), "");
        assert_eq!(replaced_by(&names(&["a"])), ": `a` supersedes it");
        assert_eq!(
            replaced_by(&names(&["a", "b"])),
            ": `a` and `b` supersede it"
        );
        assert_eq!(
            replaced_by(&names(&["a", "b", "c"])),
            ": `a`, `b`, and `c` supersede it"
        );
    }
}
