//! This module checks the relation of each link: it reports a relation outside
//! the core set as a warning (SOKF_VALIDATION_4), and one that is not
//! lowercase kebab-case as an error (SOKF_FORMAT_52).

use crate::{Document, Finding, markdown::is_slug};

/// `CORE_RELS` holds the core relations of the format specification, each
/// listed with its inverse where it has one.
pub const CORE_RELS: &[&str] = &[
    "relates-to",
    "part-of",
    "has-part",
    "depends-on",
    "depended-on-by",
    "references",
    "referenced-by",
    "supersedes",
    "superseded-by",
    "implements",
    "implemented-by",
    "tests",
    "tested-by",
    "contradicts",
];

/// `check` reports a `rel` outside the core set as a warning, and one that is
/// not lowercase kebab-case as an error (SOKF_FORMAT_52).
// sokf-implements: SOKF_FORMAT_52
// sokf-implements: SOKF_VALIDATION_4
pub fn check(doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    for link in &doc.core.links {
        if CORE_RELS.contains(&link.rel.as_str()) {
            continue;
        }
        if !is_slug(&link.rel) {
            findings.push(Finding::error(
                doc.path.clone(),
                Some(link.line),
                "SOKF_FORMAT_52",
                format!("`rel: {}` is not lowercase kebab-case", link.rel),
            ));
        } else {
            findings.push(Finding::warning(
                doc.path.clone(),
                Some(link.line),
                "SOKF_VALIDATION_4",
                format!(
                    "`rel: {}` is outside the core set, so consumers read it as `relates-to`",
                    link.rel
                ),
            ));
        }
    }
    findings
}
