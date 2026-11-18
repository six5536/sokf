//! This module checks the configuration. It warns about a glob in
//! `repository.exclude` or in a file of `/.sokf/excludes.d/` that lies under a
//! knowledge root, which has no effect, because the walk lists every file under
//! a root (CONF-3_AC-7). It also warns about the files of packs that an older
//! sokf wrote, which are left over, because sokf no longer has packs
//! (CLI-4_AC-104).

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

/// `check_config` reports one warning, with no line, for each pair of a glob
/// that the walk prunes and a knowledge root that the glob lies under: on
/// `/.sokf/config.toml` for a glob of `repository.exclude`, and on the file
/// that lists it for a glob of `/.sokf/excludes.d/`. A glob lies under a root
/// when the part of the glob before its first wildcard is the root, or a path
/// under the root.
pub fn check_config(config: &Config) -> Vec<Finding> {
    // sokf-implements: CONF-3_AC-8
    let own = config.exclude.iter().map(|glob| {
        (
            config::config_path(),
            glob.as_str(),
            "CONF-3_AC-8",
            "repository.exclude",
        )
    });
    // sokf-implements: CONF-3_AC-12
    let added = config
        .added_exclude
        .iter()
        .map(|e| (e.file.clone(), e.glob.as_str(), "CONF-3_AC-12", "exclude"));
    let mut findings = Vec::new();
    for (path, glob, key, list) in own.chain(added) {
        let literal = &glob[..glob.find(['*', '?', '[', '{']).unwrap_or(glob.len())];
        for root in &config.roots {
            let under = literal
                .strip_prefix(root.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
            if under {
                findings.push(Finding::warning(
                    path.clone(),
                    None,
                    key,
                    format!(
                        "the `{list}` glob `{glob}` lies under the knowledge root {root}, so it has no effect: sokf checks every file under a root"
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
        std::fs::create_dir_all(dir.join(".sokf/packs/@acme/tool/base")).unwrap();
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

    // sokf-tests: CONF-3_AC-12
    #[test]
    fn a_glob_of_excludes_d_under_a_root_is_a_warning_on_its_file() {
        let mut c = config(&["/analysis"], &[]);
        c.added_exclude = ["/analysis/old/**", "/vendor/**"]
            .into_iter()
            .map(|g| crate::AddedExclude {
                file: RepoPath::parse("/.sokf/excludes.d/pakf/@acme/tool/excludes.toml").unwrap(),
                glob: g.to_string(),
            })
            .collect();
        let lines: Vec<(String, Option<usize>, String)> = check_config(&c)
            .iter()
            .map(|f| (f.path.to_string(), f.line, crate::report::message(f)))
            .collect();
        assert_eq!(
            lines,
            vec![(
                "/.sokf/excludes.d/pakf/@acme/tool/excludes.toml".to_string(),
                None,
                "the `exclude` glob `/analysis/old/**` lies under the knowledge root /analysis, so it has no effect: sokf checks every file under a root (CONF-3_AC-12)".to_string()
            )]
        );
    }
}
