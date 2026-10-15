//! This module checks the markers in a file outside the knowledge roots
//! (sokf-format-spec §9.5). It checks that each reference resolves, that
//! each tag is spelt correctly, and that each ignore region is opened and
//! closed once, and it warns about each relationship outside the core set.

use super::{Citation, Citations, Ctx, warnings::CORE_RELS};
use crate::{
    Finding, RepoPath, Severity,
    markers::{IgnoreFault, MarkerRow, RowKind, resolve},
};

/// `check` returns the findings for the marker rows of one file, and
/// records in `citations` each marker that resolves, for the coverage
/// pass. When `markers.check = "warn"`, each failure is a warning, not an
/// error.
// sokf-implements: SOKF_FORMAT_143
// sokf-implements: SOKF_FORMAT_144
// sokf-implements: SOKF_FORMAT_145
// sokf-implements: SOKF_FORMAT_152
// sokf-implements: SOKF_FORMAT_153
// sokf-implements: SOKF_VALIDATION_51
// sokf-implements: SOKF_VALIDATION_52
// sokf-implements: CONF-7_AC-1
// sokf-implements: CONF-7_AC-2
// sokf-implements: EXT-9_AC-6
pub fn check(
    path: &RepoPath,
    rows: &[MarkerRow],
    ctx: &Ctx<'_>,
    citations: &mut Citations,
) -> Vec<Finding> {
    let failure = ctx.model.marker_check.severity();
    let mut findings = Vec::new();
    let mut push = |line: usize, severity: Severity, message: String, key: &'static str| {
        findings.push(Finding::new(
            path.clone(),
            Some(line),
            severity,
            key,
            message,
        ));
    };
    for row in rows {
        let line = row.line;
        match &row.kind {
            RowKind::Marker { rel, target } => {
                let written = format!("sokf-{rel}: {target}");
                if !CORE_RELS.contains(&rel.as_str()) {
                    push(
                        line,
                        Severity::Warning,
                        format!(
                            "`{written}` has the relationship `{rel}`, which SOKF doesn't define, so consumers read it as `relates-to`"
                        ),
                        "SOKF_VALIDATION_51",
                    );
                }
                match resolve(ctx.model, ctx.keys, target) {
                    Ok(r) => citations.add(
                        r.path,
                        Citation {
                            slug: r.slug,
                            source: path.clone(),
                            source_type: String::new(),
                            rel: Some(rel.clone()),
                            line,
                        },
                    ),
                    Err(u) => push(
                        line,
                        failure,
                        format!("`{written}` {}", u.message),
                        u.criterion,
                    ),
                }
            }
            RowKind::Malformed(tag) => push(
                line,
                failure,
                format!(
                    "`{tag}` has no reference after it, or has a word after its reference. A marker holds exactly one reference"
                ),
                "SOKF_FORMAT_144",
            ),
            RowKind::Case(tag) => push(
                line,
                failure,
                format!(
                    "`{tag}` matches a marker tag only when case is ignored. Write `{}`",
                    tag.to_lowercase()
                ),
                "SOKF_VALIDATION_52",
            ),
            RowKind::Ignore(fault) => {
                let (message, key) = match fault {
                    IgnoreFault::BeginInside => (
                        "`sokf:ignore-begin` appears inside an ignore region that is already open",
                        "SOKF_FORMAT_152",
                    ),
                    IgnoreFault::NeverEnds => (
                        "`sokf:ignore-begin` has no `sokf:ignore-end` after it",
                        "SOKF_FORMAT_152",
                    ),
                    IgnoreFault::EndOutside => (
                        "`sokf:ignore-end` appears outside an ignore region",
                        "SOKF_FORMAT_153",
                    ),
                };
                push(line, failure, message.to_string(), key);
            }
        }
    }
    findings
}
