//! The configuration check: a glob in `repository.exclude` that lies under
//! a knowledge root has no effect, because the walk lists every file under a
//! root (CONF-3_AC-7).

use crate::{Config, Finding, config};

/// `check_config` reports one warning on `/.sokf/config.toml`, with no line,
/// for each glob in `repository.exclude` and each knowledge root that the
/// glob lies under. A glob lies under a root when the part of the glob
/// before its first wildcard is the root, or a path under the root.
// sokf-implements: CONF-3_AC-8
pub fn check_config(config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    for glob in &config.exclude {
        let literal = &glob[..glob.find(['*', '?', '[', '{']).unwrap_or(glob.len())];
        for root in &config.roots {
            let under = literal
                .strip_prefix(root.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
            if under {
                findings.push(Finding::warning(
                    config::config_path(),
                    None,
                    "CONF-3_AC-8",
                    format!(
                        "the `repository.exclude` glob `{glob}` lies under the knowledge root {root}, so it has no effect: sokf checks every file under a root"
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RepoPath;

    fn config(roots: &[&str], exclude: &[&str]) -> Config {
        Config {
            roots: roots.iter().map(|r| RepoPath::parse(r).unwrap()).collect(),
            exclude: exclude.iter().map(|g| g.to_string()).collect(),
            ..Config::default()
        }
    }

    // sokf-tests: CONF-3_AC-8
    #[test]
    fn a_glob_under_a_root_is_a_warning_on_the_config() {
        let c = config(
            &["/analysis", "/knowledge"],
            &[
                "/analysis/chuckie-egg/**",
                "/analysis",
                "/analysis-old/**",
                "/anal*",
                "/vendor/**",
                "/**/*.md",
            ],
        );
        let findings = check_config(&c);
        let lines: Vec<(String, Option<usize>, String)> = findings
            .iter()
            .map(|f| (f.path.to_string(), f.line, crate::report::message(f)))
            .collect();
        assert_eq!(
            lines,
            vec![
                (
                    "/.sokf/config.toml".to_string(),
                    None,
                    "the `repository.exclude` glob `/analysis/chuckie-egg/**` lies under the knowledge root /analysis, so it has no effect: sokf checks every file under a root (CONF-3_AC-8)".to_string()
                ),
                (
                    "/.sokf/config.toml".to_string(),
                    None,
                    "the `repository.exclude` glob `/analysis` lies under the knowledge root /analysis, so it has no effect: sokf checks every file under a root (CONF-3_AC-8)".to_string()
                ),
            ]
        );
        assert!(
            findings
                .iter()
                .all(|f| f.severity == crate::Severity::Warning)
        );
    }
}
