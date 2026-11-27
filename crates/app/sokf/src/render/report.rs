//! This module writes the text report of `validate` (CLI-4_AC-7). It writes
//! each path from the repository root, starting with `/` (CLI-1_AC-6).

use std::io::{self, Write};

use sokf_core::Report;

/// `text` writes the text report: the fixes, then the findings, then the line
/// of counts, and, in a run without `--fix`, the line that counts the
/// findings that `--fix` repairs, where it lists one. The core builds the
/// text, so `validate` and every other command that prints it print the
/// report in the same form.
// sokf-implements: CLI-4_AC-7
// sokf-implements: CLI-4_AC-11
// sokf-implements: CLI-4_AC-25
pub fn text<W: Write>(w: &mut W, report: &Report, warnings: bool, fix: bool) -> io::Result<()> {
    w.write_all(sokf_core::report_text(report, warnings).as_bytes())?;
    match sokf_core::report_fixable_line(report, warnings).filter(|_| !fix) {
        Some(line) => writeln!(w, "{line}"),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use sokf_core::{Finding, FixOp, Locator, RepoPath, Severity};

    use super::*;

    fn path(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn report() -> Report {
        let mut report = Report::default();
        report.errors.push(Finding::new(
            path("/knowledge/a.md"),
            Some(41),
            Severity::Error,
            "SOKF_FORMAT_61",
            "`[sokf:planer]` resolves to no document",
        ));
        report.errors.push(
            Finding::new(
                path("/README.md"),
                None,
                Severity::Error,
                "TLMD_18",
                "section `## Usage` is missing",
            )
            .at(&Locator::schema("readme")),
        );
        report.warnings.push(Finding::new(
            path("/knowledge/a.md"),
            Some(3),
            Severity::Warning,
            "SOKF_VALIDATION_4",
            "`rel: likes` is outside the core set",
        ));
        report.fixes.push(FixOp::Moved {
            from: path("/knowledge/open/x.md"),
            to: path("/knowledge/closed/x.md"),
        });
        report.fixes.push(FixOp::Rewrote {
            path: path("/knowledge/a.md"),
        });
        report.fixes.push(FixOp::Created {
            path: path("/knowledge/closed/index.md"),
        });
        report.finish()
    }

    // sokf-tests: CLI-4_AC-6
    // sokf-tests: CLI-4_AC-7
    // sokf-tests: CLI-4_AC-8
    // sokf-tests: CLI-4_AC-9
    // sokf-tests: CLI-4_AC-30
    #[test]
    fn text_report_matches_the_tool_spec() {
        let mut buf = Vec::new();
        text(&mut buf, &report(), false, false).unwrap();
        insta::assert_snapshot!(String::from_utf8(buf).unwrap());
    }

    #[test]
    fn warnings_are_listed_on_request() {
        let mut buf = Vec::new();
        text(&mut buf, &report(), true, false).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("/knowledge/a.md:3: warning:"), "{out}");
        assert!(
            out.ends_with("\n2 errors, 1 of 1 warnings listed\n"),
            "{out}"
        );
    }

    #[test]
    fn counts_pluralize() {
        let mut buf = Vec::new();
        text(&mut buf, &Report::default(), false, false).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "0 errors, 0 warnings\n");
    }

    // A run without `--fix` that lists a finding which `--fix` repairs ends
    // with one line that counts such findings, and a run with `--fix` or
    // with none ends with the line of counts.
    // sokf-tests: CLI-4_AC-30
    #[test]
    fn the_fixable_findings_are_counted_once() {
        let mut report = report();
        for line in [7, 9] {
            report.errors.push(Finding::new(
                path("/knowledge/b.md"),
                Some(line),
                Severity::Error,
                "SOKF_FORMAT_85",
                "`sokf:include /src/a.rs` is stale",
            ));
        }
        let report = report.finish();
        let render = |fix: bool| {
            let mut buf = Vec::new();
            text(&mut buf, &report, false, fix).unwrap();
            String::from_utf8(buf).unwrap()
        };
        assert!(
            render(false).ends_with(
                "\n4 errors, 0 of 1 warnings listed\n2 potentially fixable with `sokf validate --fix`\n"
            ),
            "{}",
            render(false)
        );
        assert!(
            render(true).ends_with("\n4 errors, 0 of 1 warnings listed\n"),
            "{}",
            render(true)
        );
    }
}
