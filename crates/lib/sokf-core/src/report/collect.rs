//! `Report` is the result of a run. This module orders it and serialises it
//! as CLI-4 specifies.

use schemars::JsonSchema;
use serde::Serialize;

use super::{Finding, FixOp};
use crate::{Error, Expr, RepoPath, Scope};

// sokf:begin report
/// `Report` holds the errors, the warnings, and the fixes of a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The errors, which `finish` orders by path, then by line.
    pub errors: Vec<Finding>,
    /// The warnings, which `finish` orders as it orders the errors.
    pub warnings: Vec<Finding>,
    /// The fixes, which `finish` orders with the moves first, then the created
    /// files, then the rewritten files, each group by path.
    pub fixes: Vec<FixOp>,
}
// sokf:end report

impl Report {
    /// `push` adds a finding to the errors or to the warnings, according to its
    /// severity. The report drops the finding's `rule_line`, which the report
    /// doesn't print and the cache doesn't store, so that a finding read from
    /// the cache equals the same finding made fresh.
    pub fn push(&mut self, mut finding: Finding) {
        finding.rule_line = None;
        match finding.severity {
            super::Severity::Error => self.errors.push(finding),
            super::Severity::Warning => self.warnings.push(finding),
        }
    }

    /// `extend` adds every finding in `findings`, as `push` adds one.
    pub fn extend(&mut self, findings: impl IntoIterator<Item = Finding>) {
        for f in findings {
            self.push(f);
        }
    }

    /// `finish` orders the findings by path, then by line, removes duplicate
    /// findings and fixes, and orders the fixes as CLI-4_AC-6 lists them.
    // sokf-implements: CLI-4_AC-6
    // sokf-implements: CLI-4_AC-9
    pub fn finish(mut self) -> Self {
        for list in [&mut self.errors, &mut self.warnings] {
            list.sort_by(|a, b| {
                (&a.path, a.line, &a.message, &a.criterion).cmp(&(
                    &b.path,
                    b.line,
                    &b.message,
                    &b.criterion,
                ))
            });
            list.dedup();
        }
        self.fixes.sort_by(|a, b| fix_key(a).cmp(&fix_key(b)));
        self.fixes.dedup();
        self
    }

    /// `filter` returns a report of the findings in `scope`. It keeps every
    /// fix, whatever its path.
    // sokf-implements: CLI-4_AC-2
    pub fn filter(&self, scope: &Scope) -> Report {
        let keep = |list: &[Finding]| -> Vec<Finding> {
            list.iter()
                .filter(|f| scope.contains(&f.path))
                .cloned()
                .collect()
        };
        Report {
            errors: keep(&self.errors),
            warnings: keep(&self.warnings),
            fixes: self.fixes.clone(),
        }
    }

    /// `has_errors` returns whether the run found an error.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// `project` serialises the report as JSON and evaluates `expr` against the
    /// result (CLI-4_AC-14).
    // sokf-implements: CLI-4_AC-14
    // sokf-implements: CLI-4_AC-17
    pub fn project(&self, expr: &Expr) -> crate::Result<serde_json::Value> {
        let value = serde_json::to_value(self).map_err(|e| Error::Internal(e.to_string()))?;
        expr.eval(&value)
    }
}

/// `fix_key` returns the sort key of a fix: first its group (moves, then
/// created files, then rewritten files), then its path.
fn fix_key(op: &FixOp) -> (u8, &RepoPath) {
    match op {
        FixOp::Moved { from, .. } => (0, from),
        FixOp::Created { path } => (1, path),
        FixOp::Rewrote { path } => (2, path),
    }
}

/// `ValidateReport` is the report that `sokf validate --json` prints. It holds every error,
/// every warning, and each repair by `--fix` (CLI-4_AC-12).
// `Report` serialises itself as a `ValidateReport`, and the tool publishes the schema of
// `ValidateReport`.
#[derive(Serialize, JsonSchema)]
#[schemars(title = "sokf validate report")]
pub struct ValidateReport {
    /// Every error, ordered by path, then by line.
    errors: Vec<FindingJson>,
    /// Every warning, ordered by path, then by line.
    warnings: Vec<FindingJson>,
    /// The repairs that `--fix` made: the moves first, then the created files, then the rewritten
    /// files, with each group ordered by path. The list is empty when the command runs without `--fix`.
    fixes: Vec<FixJson>,
    /// The number of errors, warnings, and fixes in the report.
    summary: SummaryJson,
}

/// The summary holds the number of errors, warnings, and fixes in the report. The last line of
/// the text output shows the same counts of errors and warnings (CLI-4_AC-31).
// sokf-implements: CLI-4_AC-31
#[derive(Serialize, JsonSchema)]
#[schemars(rename = "Summary")]
struct SummaryJson {
    /// The number of errors.
    errors: usize,
    /// The number of warnings.
    warnings: usize,
    /// The number of repairs that `--fix` made.
    fixes: usize,
}

/// One finding.
#[derive(Serialize, JsonSchema)]
#[schemars(rename = "Finding")]
struct FindingJson {
    /// The path of the file where the finding occurs, relative to the repository root and
    /// starting with `/`.
    path: String,
    /// The line of the finding in the file. It is absent when no line applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    line: Option<usize>,
    /// What is wrong, followed in parentheses by the key of the criterion that the file
    /// breaks, such as `SOKF_FORMAT_53` or `TLMD_152`. When a schema rule raises the finding,
    /// the id of the schema comes in parentheses before the key. When the finding has the place
    /// of the rule in the schema, that place comes in the parentheses too.
    message: String,
}

/// One repair that `--fix` made.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "op", rename_all = "lowercase")]
#[schemars(rename = "Fix")]
enum FixJson {
    /// A file moved to the directory of its lifecycle state.
    Moved {
        /// The path before the move.
        from: String,
        /// The path after the move.
        to: String,
    },
    /// A file that `--fix` created, such as an index.
    Created {
        /// The file.
        path: String,
    },
    /// A file that `--fix` rewrote.
    Rewrote {
        /// The file.
        path: String,
    },
}

impl From<&Finding> for FindingJson {
    fn from(f: &Finding) -> Self {
        FindingJson {
            path: f.path.as_str().to_string(),
            line: f.line,
            message: super::message(f),
        }
    }
}

impl From<&FixOp> for FixJson {
    fn from(op: &FixOp) -> Self {
        match op {
            FixOp::Moved { from, to } => FixJson::Moved {
                from: from.as_str().to_string(),
                to: to.as_str().to_string(),
            },
            FixOp::Created { path } => FixJson::Created {
                path: path.as_str().to_string(),
            },
            FixOp::Rewrote { path } => FixJson::Rewrote {
                path: path.as_str().to_string(),
            },
        }
    }
}

impl From<&Report> for ValidateReport {
    fn from(r: &Report) -> Self {
        ValidateReport {
            errors: r.errors.iter().map(FindingJson::from).collect(),
            warnings: r.warnings.iter().map(FindingJson::from).collect(),
            fixes: r.fixes.iter().map(FixJson::from).collect(),
            summary: SummaryJson {
                errors: r.errors.len(),
                warnings: r.warnings.len(),
                fixes: r.fixes.len(),
            },
        }
    }
}

// sokf-implements: CLI-4_AC-12
// sokf-implements: CLI-4_AC-13
impl Serialize for Report {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        ValidateReport::from(self).serialize(s)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::Severity;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn finding(path: &str, line: Option<usize>, msg: &str) -> Finding {
        Finding::error(p(path), line, "SOKF_FORMAT_53", msg)
    }

    #[test]
    fn json_has_the_shape_of_the_tool_spec() {
        let mut r = Report::default();
        r.push(finding("/knowledge/a.md", Some(41), "x"));
        r.push(Finding::warning(p("/b.md"), None, "SOKF_VALIDATION_4", "w"));
        r.fixes.push(FixOp::Rewrote {
            path: p("/knowledge/a.md"),
        });
        r.fixes.push(FixOp::Moved {
            from: p("/k/open/i.md"),
            to: p("/k/closed/i.md"),
        });
        r.fixes.push(FixOp::Created {
            path: p("/k/closed/index.md"),
        });
        let r = r.finish();
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "errors": [{"path": "/knowledge/a.md", "line": 41, "message": "x (SOKF_FORMAT_53)"}],
                "warnings": [{"path": "/b.md", "message": "w (SOKF_VALIDATION_4)"}],
                "fixes": [
                    {"op": "moved", "from": "/k/open/i.md", "to": "/k/closed/i.md"},
                    {"op": "created", "path": "/k/closed/index.md"},
                    {"op": "rewrote", "path": "/knowledge/a.md"}
                ],
                "summary": {"errors": 1, "warnings": 1, "fixes": 3}
            })
        );
        assert!(r.has_errors());
        assert!(!Report::default().has_errors());
        let expr = Expr::compile("errors[].path").unwrap();
        assert_eq!(
            r.project(&expr).unwrap(),
            serde_json::json!(["/knowledge/a.md"])
        );
        assert_eq!(
            r.project(&Expr::compile("length(warnings)").unwrap())
                .unwrap(),
            serde_json::json!(1)
        );
    }

    #[test]
    fn filter_keeps_findings_in_scope_and_every_fix() {
        let mut r = Report::default();
        r.push(finding("/k/a.md", None, "x"));
        r.push(finding("/other.md", None, "y"));
        r.push(Finding::warning(
            p("/k/b.md"),
            None,
            "SOKF_VALIDATION_4",
            "w",
        ));
        r.fixes.push(FixOp::Rewrote {
            path: p("/other.md"),
        });
        let mut scope = Scope::default();
        scope.dirs.insert(p("/k"));
        let filtered = r.filter(&scope);
        assert_eq!(filtered.errors.len(), 1);
        assert_eq!(filtered.warnings.len(), 1);
        assert_eq!(filtered.fixes.len(), 1);
        assert_eq!(r.filter(&Scope::all()), r);
    }

    fn arb_finding() -> impl Strategy<Value = Finding> {
        (
            prop::sample::select(vec!["/a.md", "/b/c.md", "/b/a.md", "/z.md"]),
            prop::option::of(1usize..50),
            prop::sample::select(vec!["m1", "m2"]),
            any::<bool>(),
        )
            .prop_map(|(path, line, msg, warn)| {
                let severity = if warn {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                Finding::new(p(path), line, severity, "SOKF_FORMAT_53", msg)
            })
    }

    // sokf-tests: CLI-4_AC-6
    // sokf-tests: CLI-4_AC-9
    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn finish_orders_by_path_then_line_and_dedups(findings in prop::collection::vec(arb_finding(), 0..12)) {
            let mut r = Report::default();
            r.extend(findings.clone());
            r.fixes.push(FixOp::Rewrote { path: p("/z.md") });
            r.fixes.push(FixOp::Moved { from: p("/b.md"), to: p("/c.md") });
            r.fixes.push(FixOp::Rewrote { path: p("/a.md") });
            r.fixes.push(FixOp::Created { path: p("/z/index.md") });
            r.fixes.push(FixOp::Moved { from: p("/a.md"), to: p("/c.md") });
            r.fixes.push(FixOp::Created { path: p("/c/index.md") });
            let r = r.finish();
            for list in [&r.errors, &r.warnings] {
                for w in list.windows(2) {
                    prop_assert!((&w[0].path, w[0].line) <= (&w[1].path, w[1].line));
                    prop_assert_ne!(&w[0], &w[1]);
                }
            }
            prop_assert!(r.errors.iter().all(|f| f.severity == Severity::Error));
            prop_assert!(r.warnings.iter().all(|f| f.severity == Severity::Warning));
            let mut expected: Vec<_> = findings.clone();
            expected.sort();
            expected.dedup();
            prop_assert_eq!(r.errors.len() + r.warnings.len(), expected.len());
            prop_assert_eq!(&r.fixes, &vec![
                FixOp::Moved { from: p("/a.md"), to: p("/c.md") },
                FixOp::Moved { from: p("/b.md"), to: p("/c.md") },
                FixOp::Created { path: p("/c/index.md") },
                FixOp::Created { path: p("/z/index.md") },
                FixOp::Rewrote { path: p("/a.md") },
                FixOp::Rewrote { path: p("/z.md") },
            ]);
        }
    }

    // sokf-tests: CLI-4_AC-12
    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn json_shape_and_finding_round_trip(findings in prop::collection::vec(arb_finding(), 0..6)) {
            let mut r = Report::default();
            r.extend(findings.clone());
            let text = serde_json::to_string(&r).unwrap();
            let e = text.find("\"errors\":").unwrap();
            let w = text.find("\"warnings\":").unwrap();
            let f = text.find("\"fixes\":").unwrap();
            let sum = text.find("\"summary\":").unwrap();
            prop_assert!(e < w && w < f && f < sum, "{text}");
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            let obj = json.as_object().unwrap();
            prop_assert_eq!(obj.len(), 4);
            prop_assert_eq!(obj["summary"]["errors"].as_u64(), Some(r.errors.len() as u64));
            for entry in obj["errors"].as_array().unwrap().iter().chain(obj["warnings"].as_array().unwrap()) {
                let e = entry.as_object().unwrap();
                prop_assert!(e.contains_key("path") && e.contains_key("message"));
                prop_assert!(e["path"].as_str().unwrap().starts_with('/'));
                prop_assert!(e.keys().all(|k| k == "path" || k == "line" || k == "message"));
            }
            for f in findings {
                let back: Finding = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
                prop_assert_eq!(back, f);
            }
        }
    }
}
