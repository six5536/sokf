//! This module checks coverage across documents. The link check and the marker check record
//! each citation they resolve, and the `cited-by` pass reads them after
//! every document is checked (SOKF_SCHEMA_17, SOKF_SCHEMA_49).

use std::collections::HashMap;

use super::states_of;
use crate::{
    Compiled, Finding, Locator, MarkerCheck, Model, RepoPath,
    grammar::{CitationRule, CitedFrom},
    markdown::slug,
    model::{KeyEntry, KeyIndex, Query},
};

/// A `Citation` is one resolved `sokf:` body link or marker, seen from
/// its target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// The slug of the fragment, which is a key or a heading, or `None`
    /// when the citation links to the whole document.
    pub slug: Option<String>,
    /// The citing document, or the file of the marker.
    pub source: RepoPath,
    /// The citing document's type, or empty for a marker.
    pub source_type: String,
    /// The relationship of a marker, or `None` for a body link.
    pub rel: Option<String>,
    /// The line of the link or the marker.
    pub line: usize,
}

impl Citation {
    /// `counts_for` returns whether this citation counts toward coverage
    /// for an entry with the given `from`. A body link counts when its
    /// document has one of the entry's types, and a marker counts when it
    /// has the entry's relationship.
    // sokf-implements: EXT-9_AC-6
    pub fn counts_for(&self, from: &CitedFrom) -> bool {
        match (from, &self.rel) {
            (CitedFrom::Types(types), None) => types.contains(&self.source_type),
            (CitedFrom::Marker(rel), Some(r)) => r == rel,
            _ => false,
        }
    }
}

/// `Citations` holds every citation of the run, grouped by the target
/// document. The link check fills it while it resolves each label, so no
/// link is resolved twice, and the coverage pass reads it.
#[derive(Debug, Clone, Default)]
pub struct Citations {
    by_target: HashMap<RepoPath, Vec<Citation>>,
    /// Every citation, in the order it was added, so that a run can read
    /// back the citations of one source.
    added: Vec<(RepoPath, Citation)>,
}

impl Citations {
    /// `add` records that the source of `citation` cites `target`.
    pub fn add(&mut self, target: RepoPath, citation: Citation) {
        self.added.push((target.clone(), citation.clone()));
        self.by_target.entry(target).or_default().push(citation);
    }

    /// `len` returns the number of citations added.
    pub fn len(&self) -> usize {
        self.added.len()
    }

    /// `is_empty` returns whether no citation was added.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
    }

    /// `since` returns the citations from index `mark` onward, as
    /// `(target, citation)` pairs.
    pub fn since(&self, mark: usize) -> Vec<(RepoPath, Citation)> {
        self.added[mark..].to_vec()
    }

    /// `of` returns the citations of `target` that count for an entry with
    /// the given `from`. It leaves out the citations from `target` itself.
    /// With `slug`, it returns the citations of that key alone. Without it,
    /// it returns every citation of the document, with or without a
    /// fragment.
    pub fn of<'a>(
        &'a self,
        target: &'a RepoPath,
        slug: Option<&'a str>,
        from: &'a CitedFrom,
    ) -> impl Iterator<Item = &'a Citation> + 'a {
        self.by_target
            .get(target)
            .into_iter()
            .flatten()
            .filter(move |c| &c.source != target)
            .filter(move |c| c.counts_for(from))
            .filter(move |c| slug.is_none_or(|s| c.slug.as_deref() == Some(s)))
    }

    /// `markers_of` returns the markers that cite `target` or one of its
    /// keys.
    pub fn markers_of<'a>(&'a self, target: &'a RepoPath) -> impl Iterator<Item = &'a Citation> {
        self.by_target
            .get(target)
            .into_iter()
            .flatten()
            .filter(|c| c.rel.is_some())
    }
}

/// `check_document` runs the `cited-by` pass on one document. When the
/// document's schema demands citations, the pass checks each entry that
/// the document's states admit. It reports an uncited document or key
/// once, with the entry's severity, on the document without the citation
/// (SOKF_SCHEMA_17). The pass also reports each marker that cites the
/// document with a relationship outside the schema's `markers`. It
/// reports the marker at the marker's own line (SOKF_SCHEMA_49).
// sokf-implements: EXT-9_AC-1
// sokf-implements: EXT-9_AC-2
// sokf-implements: EXT-9_AC-3
// sokf-implements: EXT-9_AC-4
// sokf-implements: EXT-9_AC-8
pub fn check_document(
    model: &Model,
    compiled: &Compiled,
    keys: &KeyIndex,
    citations: &Citations,
    k: usize,
    doc: &crate::Document,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some((r, schema)) = compiled.for_doc(model, k, doc) else {
        return findings;
    };
    if schema.cited_by.is_empty() && schema.markers.is_none() {
        return findings;
    }
    model.note(Query::Citations(doc.path.clone()));
    let schema_id = model.schema(r).id();
    if let Some(allowed) = &schema.markers {
        findings.extend(refused_markers(model, citations, doc, allowed, &schema_id));
    }
    let states = states_of(doc.frontmatter.as_ref(), schema);
    // When the marker check is off, the run reads no file for markers, so the
    // pass skips an entry that counts them. The schema's own check warns
    // about such an entry.
    let markers_off = model.marker_check == MarkerCheck::Off;
    for rule in schema
        .cited_by
        .iter()
        .filter(|r| r.common.only.admits(&states))
        .filter(|r| !(markers_off && matches!(r.from, CitedFrom::Marker(_))))
    {
        let from = from_phrase(rule);
        let locator = match &rule.keys {
            None => format!("cited-by {}", locator_of(rule)),
            Some(id) => format!("cited-by {} of `{id}`", locator_of(rule)),
        };
        let locator = Locator::schema(schema_id.clone()).at(locator);
        let cited = |slug: Option<&str>| citations.of(&doc.path, slug, &rule.from).next().is_some();
        let uncited = |line: Option<usize>, message: String| {
            Finding::new(
                doc.path.clone(),
                line,
                rule.common.severity,
                "SOKF_SCHEMA_17",
                message,
            )
            .at(&locator)
        };
        match &rule.keys {
            None => {
                if !cited(None) {
                    findings.push(uncited(None, format!("no {from} cites this document")));
                }
            }
            Some(id) => {
                // The pass reports each key once: a rule can capture one key in
                // two places, such as a diagram line and a table row, but
                // the key is either cited or not.
                let mut owned: Vec<&KeyEntry> = keys
                    .of_document(&doc.path)
                    .filter(|e| e.rule_id.as_deref() == Some(id.as_str()))
                    .collect();
                owned.sort_by_key(|e| (e.line, e.key.clone()));
                owned.dedup_by_key(|e| slug(&e.key));
                for e in owned {
                    // A heading key is cited by its own slug or by its
                    // heading's slug, because both point to the same
                    // anchor line.
                    let by_heading = e.heading_slug.as_deref().is_some_and(|h| cited(Some(h)));
                    if !cited(Some(&slug(&e.key))) && !by_heading {
                        findings.push(uncited(
                            Some(e.line),
                            format!("no {from} cites key `{}`", e.key),
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// `refused_markers` reports each marker that cites `doc` or one of its
/// keys with a relationship outside `allowed`. Each finding is
/// an error at the marker, or a warning when `markers.check` is `warn`
/// (CONF-7_AC-2).
// sokf-implements: EXT-9_AC-7
// sokf-implements: SOKF_SCHEMA_49
fn refused_markers(
    model: &Model,
    citations: &Citations,
    doc: &crate::Document,
    allowed: &[String],
    schema_id: &str,
) -> Vec<Finding> {
    let severity = model.marker_check.severity();
    let held = if allowed.is_empty() {
        "no marker".to_string()
    } else {
        let rels: Vec<String> = allowed.iter().map(|r| format!("`sokf-{r}:`")).collect();
        format!("only {}", rels.join(", "))
    };
    let name = doc.id().unwrap_or(doc.path.as_str());
    citations
        .markers_of(&doc.path)
        .filter(|c| c.rel.as_ref().is_some_and(|r| !allowed.contains(r)))
        .map(|c| {
            let rel = c.rel.as_deref().unwrap_or_default();
            Finding::new(
                c.source.clone(),
                Some(c.line),
                severity,
                "SOKF_SCHEMA_49",
                format!("`sokf-{rel}:` cites `{name}`, whose schema admits {held}"),
            )
            .at(&Locator::schema(schema_id).at("markers"))
        })
        .collect()
}

/// `locator_of` describes which citations an entry accepts, in the form
/// that its locator uses: `` `Stories` `` or `` marker `implements` ``.
fn locator_of(rule: &CitationRule) -> String {
    match &rule.from {
        CitedFrom::Types(types) => format!("`{}`", types.join("`, `")),
        CitedFrom::Marker(rel) => format!("marker `{rel}`"),
    }
}

/// `or_list` returns `types`, each in backticks, joined with " or ".
pub(super) fn or_list(types: &[String]) -> String {
    types
        .iter()
        .map(|t| format!("`{t}`"))
        .collect::<Vec<_>>()
        .join(" or ")
}

/// `from_phrase` describes which citations an entry accepts, in the form
/// that a message uses: `` `Stories` document ``, `` `Overview` or
/// `Design` document ``, or `` `sokf-implements:` marker ``.
fn from_phrase(rule: &CitationRule) -> String {
    match &rule.from {
        CitedFrom::Types(types) => format!("{} document", or_list(types)),
        CitedFrom::Marker(rel) => format!("`sokf-{rel}:` marker"),
    }
}
