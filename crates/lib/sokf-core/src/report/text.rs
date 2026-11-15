//! This module renders the text report of `validate` (CLI-4).
//! The report writes each path relative to the repository root, starting with `/` (CLI-1_AC-6).

use std::fmt::Write as _;

use super::{Finding, FixOp, Report, Severity};

/// `LISTED_WARNINGS` holds the keys of the warnings that the text report
/// lists even without `--warnings`: the warnings at a stale and at an absent
/// part of a harness, the warning that the formatter failed, and the warning
/// at a file of the packs that an older sokf wrote, which the agent can act
/// on at once (CLI-4_AC-43).
pub(crate) const LISTED_WARNINGS: [&str; 4] = [
    STALE_PART,
    ABSENT_PART,
    crate::formatter::WARNING,
    LEFTOVER_PACKS,
];
/// `STALE_PART` is the key of the warning at a stale part of a harness.
pub(crate) const STALE_PART: &str = "CLI-4_AC-40";
/// `ABSENT_PART` is the key of the warning at an absent part of a harness.
pub(crate) const ABSENT_PART: &str = "CLI-4_AC-42";
/// `LEFTOVER_PACKS` is the key of the warning at a file of the packs that an
/// older sokf wrote.
pub(crate) const LEFTOVER_PACKS: &str = "CLI-4_AC-104";

/// `message` returns the message of `f` followed by the key of the criterion
/// that `f` breaks, in parentheses, such as `(SOKF_FORMAT_53)`.
// sokf-implements: CLI-4_AC-8
pub fn message(f: &Finding) -> String {
    format!("{} ({})", f.message, f.criterion)
}

/// `text` renders the text report: the fixes, then the findings, then the counts line.
// sokf-implements: CLI-4_AC-25
// sokf-implements: CLI-4_AC-26
// sokf-implements: CLI-4_AC-7
// sokf-implements: CLI-4_AC-11
// sokf-implements: CLI-1_AC-6
pub fn text(report: &Report, warnings: bool) -> String {
    let mut out = String::new();
    for fix in &report.fixes {
        match fix {
            FixOp::Moved { from, to } => {
                let _ = writeln!(out, "moved   {from} -> {to}");
            }
            FixOp::Created { path } => {
                let _ = writeln!(out, "created {path}");
            }
            FixOp::Rewrote { path } => {
                let _ = writeln!(out, "rewrote {path}");
            }
        }
    }
    let mut findings: Vec<_> = report.errors.iter().collect();
    // sokf-implements: CLI-4_AC-43
    findings.extend(
        report
            .warnings
            .iter()
            .filter(|f| warnings || LISTED_WARNINGS.contains(&f.criterion.as_str())),
    );
    let listed = findings.len() - report.errors.len();
    if listed > 0 {
        findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    }
    for f in findings {
        let severity = match f.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let message = message(f);
        match f.line {
            Some(line) => {
                let _ = writeln!(out, "{}:{line}: {severity}: {message}", f.path);
            }
            None => {
                let _ = writeln!(out, "{}: {severity}: {message}", f.path);
            }
        }
    }
    // sokf-implements: CLI-4_AC-30
    if !out.is_empty() {
        out.push('\n');
    }
    // With no warnings, the counts line reads `0 warnings`, not `0 of 0 warnings listed`.
    let warnings = match report.warnings.len() {
        0 => "0 warnings".to_string(),
        all => format!("{listed} of {all} warnings listed"),
    };
    let _ = writeln!(
        out,
        "{}, {warnings}",
        tramlines_core::text::count(report.errors.len(), "error", "errors")
    );
    out
}
