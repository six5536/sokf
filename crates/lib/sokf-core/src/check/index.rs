//! The index check: every directory of a knowledge has its generated
//! `index.md` (SOKF_FORMAT_96 to SOKF_FORMAT_110).

use crate::{Finding, Indexes, Located, Model};

/// `check_indexes` reports one finding, at the index's path and with no line,
/// for each directory that has no index and for each index whose text differs
/// from its expected text.
// sokf-implements: SOKF_FORMAT_96
// sokf-implements: SOKF_FORMAT_97
// sokf-implements: CLI-4_AC-27
pub fn check_indexes(model: &Model, indexes: &Indexes) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in &indexes.expected {
        match model.by_path(path) {
            Some(Located::Index(_, file)) => {
                if file.source.text != *text {
                    findings.push(Finding::error(
                        path.clone(),
                        None,
                        "SOKF_FORMAT_97",
                        "the index is stale: it differs from the generated content",
                    ));
                }
            }
            _ => findings.push(Finding::error(
                path.clone(),
                None,
                "SOKF_FORMAT_96",
                "the directory has no `index.md`",
            )),
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::{Config, FsLoader, RepoRoot, build, collect, index};

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("check-index-{name}"));
        fs::create_dir_all(dir.join("knowledge/schemas")).unwrap();
        fs::create_dir_all(dir.join("knowledge/none")).unwrap();
        fs::write(
            dir.join("knowledge/manifest.sokf.yaml"),
            "sokf: \"0.6\"\nname: k\nscope: test\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/a.md"),
            "---\ntype: Note\nid: a\ntitle: A\n---\n\n# A\n",
        )
        .unwrap();
        // The root index is current, with CRLF line endings.
        fs::write(
            dir.join("knowledge/index.md"),
            "# k\r\n\r\n- [A][sokf:a]\r\n- [none/](none/index.md)\r\n- [schemas/](schemas/index.md)\r\n\r\n<!-- sokf:links -->\r\n\r\n[sokf:a]: /knowledge/a.md\r\n",
        )
        .unwrap();
        // The schemas index is stale.
        fs::write(dir.join("knowledge/schemas/index.md"), "# Schemas\n").unwrap();
        dir
    }

    // sokf-tests: SOKF_FORMAT_96
    // sokf-tests: SOKF_FORMAT_97
    // sokf-tests: CLI-4_AC-27
    // sokf-tests: SOKF_VALIDATION_2
    #[test]
    fn a_missing_and_a_stale_index_are_findings_and_a_current_one_is_not() {
        let dir = temp("index");
        let root = RepoRoot::new(dir.clone());
        let tree = collect(&root, &Config::default()).unwrap();
        let model = build(tree, &mut FsLoader::new(&root)).unwrap();
        let indexes = index::expected(&model, &[]);
        let findings = check_indexes(&model, &indexes);
        let lines: Vec<(String, Option<usize>, String)> = findings
            .iter()
            .map(|f| (f.path.to_string(), f.line, crate::report::message(f)))
            .collect();
        assert_eq!(
            lines,
            vec![
                (
                    "/knowledge/none/index.md".to_string(),
                    None,
                    "the directory has no `index.md` (SOKF_FORMAT_96)".to_string()
                ),
                (
                    "/knowledge/schemas/index.md".to_string(),
                    None,
                    "the index is stale: it differs from the generated content (SOKF_FORMAT_97)"
                        .to_string()
                ),
            ]
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
