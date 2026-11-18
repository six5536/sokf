//! This module reports the citations of a deprecated document. The link check,
//! the include check, and the marker check record each citation with what the
//! citing document says about itself. After every document is checked, a pass
//! over each deprecated document reads these citations, with the document's own
//! `superseded-by` links (`SOKF_VALIDATION_57` to `SOKF_VALIDATION_66`).

use std::collections::{BTreeMap, BTreeSet};

use super::{Citation, Citations, Ctx, DocView};
use crate::{
    Document, Finding, MarkerCheck, Model, RepoPath,
    graph::{Target, resolve_to},
    model::{LinkEntry, Query, Status},
    report::and_list,
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
    /// (`SOKF_VALIDATION_61`, `SOKF_VALIDATION_62`). It supersedes each document
    /// that a `supersedes` entry of its `links` resolves to
    /// (`SOKF_VALIDATION_60`).
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
        citer.superseded = linked_documents(ctx.model, k, &core.links, "supersedes");
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
    /// body link mirrors the entry (`SOKF_VALIDATION_64`).
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

/// `check_document` reports each citation of `doc`, a document of the
/// knowledge `k`, when its status is `deprecated`. Each finding is a warning
/// at the line of the citation, in the file that holds it. The pass leaves
/// out a citation from a document that supersedes `doc` or that is retired,
/// and a marker when the run reads no markers. A document supersedes `doc`
/// whichever end records the link: it declares a `supersedes` entry for
/// `doc`, with or without a body link, or a `superseded-by` entry of `doc`
/// resolves to it. Each warning names the documents that supersede `doc`,
/// but not the documents that supersede those, and spells each id as the
/// store of the citing file spells it. A frontmatter `links` entry is no
/// citation, so it gets no warning.
// sokf-implements: SOKF_VALIDATION_57
// sokf-implements: SOKF_VALIDATION_58
// sokf-implements: SOKF_VALIDATION_59
// sokf-implements: SOKF_VALIDATION_60
// sokf-implements: SOKF_VALIDATION_61
// sokf-implements: SOKF_VALIDATION_62
// sokf-implements: SOKF_VALIDATION_63
// sokf-implements: SOKF_VALIDATION_64
// sokf-implements: SOKF_VALIDATION_65
// sokf-implements: SOKF_VALIDATION_66
// sokf-implements: CLI-4_AC-36
pub fn check_document(
    model: &Model,
    citations: &Citations,
    k: usize,
    doc: &Document,
) -> Vec<Finding> {
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
    // Only a citation from a document that is neither retired nor a
    // replacement can warn. Without one, the pass looks up no replacement,
    // and a new citation runs the pass again.
    let warnable: Vec<&Citation> = cited
        .iter()
        .copied()
        .filter(|c| !c.retired && !c.supersedes)
        .collect();
    if warnable.is_empty() {
        return Vec::new();
    }
    // The model records the lookup of each `superseded-by` target, so a
    // target that comes, goes, or changes its id checks `doc` again. A
    // citation from such a target draws no warning. The replacements leave
    // out `doc` itself, which can't replace itself.
    let mut named = linked_documents(model, k, &doc.core.links, "superseded-by");
    named.remove(&doc.path);
    let mut replacements = named.clone();
    replacements.extend(
        cited
            .iter()
            .filter(|c| c.supersedes && c.source != doc.path)
            .map(|c| c.source.clone()),
    );
    // Each warning spells each replacement as the store of the citing file
    // spells it, so that the author can write the name there. A file
    // outside every store takes the spelling of `doc`'s store.
    let mut clauses: BTreeMap<usize, String> = BTreeMap::new();
    let name = doc.id().unwrap_or(doc.path.as_str());
    warnable
        .iter()
        .filter(|c| !named.contains(&c.source))
        .map(|c| {
            let citing = model.knowledge_of(&c.source).unwrap_or(k);
            let replaced = clauses
                .entry(citing)
                .or_insert_with(|| replaced_by(&spellings(model, citing, &replacements)))
                .as_str();
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

/// `spellings` returns the name of each document of `replacements`, as a
/// document of the knowledge `citing` spells its id, in alphabetical order
/// and each once. A document in another store keeps its store prefix, so
/// two replacements with one id in two stores keep two names. A document
/// whose spelling is unknown, such as one in a store without an identity,
/// is named by its path.
fn spellings(model: &Model, citing: usize, replacements: &BTreeSet<RepoPath>) -> Vec<String> {
    let mut names: Vec<String> = replacements
        .iter()
        .map(|path| {
            model
                .id_of(path)
                .and_then(|(store, id)| model.canonical_include(citing, store, &id, None))
                .unwrap_or_else(|| path.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// `linked_documents` returns each document that an entry of `links` with
/// the relationship `rel` resolves to, from the knowledge `k`. An entry
/// that resolves to a file that isn't a document names none.
fn linked_documents(model: &Model, k: usize, links: &[LinkEntry], rel: &str) -> BTreeSet<RepoPath> {
    links
        .iter()
        .filter(|l| l.rel == rel)
        .filter_map(|l| match resolve_to(model, k, &l.to)? {
            Target::Document { path, .. } => Some(path),
            Target::File(_) => None,
        })
        .collect()
}

/// `replaced_by` returns the clause that names the documents that supersede
/// a deprecated one, such as `` : `design-cache` supersedes it ``, or an
/// empty string when no document does.
fn replaced_by(superseders: &[String]) -> String {
    let names: Vec<String> = superseders.iter().map(|s| format!("`{s}`")).collect();
    let verb = match names.as_slice() {
        [] => return String::new(),
        [_] => "supersedes",
        _ => "supersede",
    };
    format!(": {} {verb} it", and_list(&names))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clause_names_each_superseder_in_a_list() {
        let names =
            |list: &[&str]| -> Vec<String> { list.iter().map(ToString::to_string).collect() };
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
