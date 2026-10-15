//! This module renders the text report of `validate` (CLI-4).
//! The report writes each path relative to the repository root, starting with `/` (CLI-1_AC-6).

use super::{Finding, FixOp, Report, Severity};

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
            FixOp::Moved { from, to } => out.push_str(&format!("moved   {from} -> {to}\n")),
            FixOp::Created { path } => out.push_str(&format!("created {path}\n")),
            FixOp::Rewrote { path } => out.push_str(&format!("rewrote {path}\n")),
        }
    }
    let mut findings: Vec<_> = report.errors.iter().collect();
    if warnings {
        findings.extend(report.warnings.iter());
        findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    }
    for f in findings {
        let severity = match f.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let message = message(f);
        match f.line {
            Some(line) => out.push_str(&format!("{}:{line}: {severity}: {message}\n", f.path)),
            None => out.push_str(&format!("{}: {severity}: {message}\n", f.path)),
        }
    }
    // sokf-implements: CLI-4_AC-30
    let listed = if warnings { report.warnings.len() } else { 0 };
    if !out.is_empty() {
        out.push('\n');
    }
    // With no warnings, the counts line reads `0 warnings`, not `0 of 0 warnings listed`.
    let warnings = match report.warnings.len() {
        0 => "0 warnings".to_string(),
        all => format!("{listed} of {all} warnings listed"),
    };
    out.push_str(&format!(
        "{}, {warnings}\n",
        tramlines_core::text::count(report.errors.len(), "error", "errors")
    ));
    out
}
