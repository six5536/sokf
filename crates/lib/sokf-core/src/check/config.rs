//! The configuration check: a glob in `repository.exclude` that lies under
//! a knowledge root has no effect, because the walk lists every file under a
//! root (CONF-3_AC-7), and the files of the packs that an older sokf wrote
//! are left over, because packs are no longer part of sokf (CLI-4_AC-104).

use crate::{Config, Finding, RepoPath, RepoRoot, config};

/// `LEFTOVERS` lists the paths of the files that the packs of an older sokf
/// wrote, each with what it is: the lock and the folder of the copies.
const LEFTOVERS: [(&str, &str); 2] = [
    ("/.sokf/packs.lock", "the lock of the packs"),
    ("/.sokf/packs", "the folder of the packs' copies"),
];

/// `leftover_packs` reports one warning, with no line, at each of the lock
/// and the folder of the copies that the packs of an older sokf wrote and
/// that still stands in the repository, whatever kind of entry it is.
// sokf-implements: CLI-4_AC-104
pub fn leftover_packs(root: &RepoRoot) -> Vec<Finding> {
    LEFTOVERS
        .iter()
        .map(|(p, what)| (RepoPath::parse(p).expect("literal"), what))
        .filter(|(p, _)| std::fs::symlink_metadata(p.to_fs(root)).is_ok())
        .map(|(path, what)| {
            let message = format!(
                "{path} is {what}, which an older sokf wrote, and packs are no longer part of sokf, so nothing reads it: delete it"
            );
            Finding::warning(path, None, crate::report::LEFTOVER_PACKS, message)
        })
        .collect()
}

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

    // sokf-tests: CLI-4_AC-104
    #[test]
    fn the_files_of_the_packs_are_a_warning_each() {
        let dir = crate::test_support::fresh_dir("leftover-packs");
        let root = RepoRoot::new(dir.clone());
        assert_eq!(leftover_packs(&root).len(), 0);
        std::fs::create_dir_all(dir.join(".sokf/packs/@acme/method/base")).unwrap();
        std::fs::write(dir.join(".sokf/packs.lock"), "lock-version = 4\n").unwrap();
        let found: Vec<(String, Option<usize>, crate::Severity)> = leftover_packs(&root)
            .iter()
            .map(|f| (f.path.to_string(), f.line, f.severity))
            .collect();
        assert_eq!(
            found,
            vec![
                ("/.sokf/packs.lock".into(), None, crate::Severity::Warning),
                ("/.sokf/packs".into(), None, crate::Severity::Warning),
            ]
        );
        let message = crate::report::message(&leftover_packs(&root)[0]);
        assert!(
            message.contains("packs are no longer part of sokf"),
            "{message}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn config(roots: &[&str], exclude: &[&str]) -> Config {
        Config {
            roots: roots.iter().map(|r| RepoPath::parse(r).unwrap()).collect(),
            exclude: exclude.iter().map(ToString::to_string).collect(),
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
