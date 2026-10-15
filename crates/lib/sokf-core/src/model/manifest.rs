//! This module parses a store's manifest into `Manifest`, as the format
//! specification defines it, and holds the version of the specification
//! that the manifest declares.

use std::collections::HashMap;

use crate::{Finding, RepoPath, Spanned, YamlError};

/// The stamped keys of the manifest (SOKF_FORMAT_16): keys that sokf writes
/// itself.
pub const STAMPED_MANIFEST_KEYS: &[&str] = &["producer", "generated", "counts"];

/// The version of the format specification that this tool implements.
// sokf-implements: CLI-10_AC-1
pub const SOKF_VERSION: &str = "0.6";

/// The manifest, as parsed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    /// `sokf`.
    pub sokf: Option<String>,
    /// `name`.
    pub name: Option<String>,
    /// `scope`, the owner of the store (SOKF_FORMAT_10).
    pub scope: Option<String>,
    /// `renamed-from`, the store's former identities, each with its line.
    pub renamed_from: Vec<(String, usize)>,
    /// `description`.
    pub description: Option<String>,
    /// The stamped keys that the manifest has, each with its line.
    pub stamped_present: Vec<(String, usize)>,
    /// The line of every key.
    pub line_of: HashMap<String, usize>,
    /// The findings that the parser records about the manifest.
    pub problems: Vec<Finding>,
}

/// `manifest` parses a manifest.
// sokf-implements: SOKF_FORMAT_8
// sokf-implements: SOKF_FORMAT_11
pub fn manifest(yaml: Result<Spanned, YamlError>, path: &RepoPath) -> Manifest {
    let mut m = Manifest::default();
    let node = match yaml {
        Ok(n) => n,
        Err(e) => {
            m.problems.push(Finding::error(
                path.clone(),
                Some(e.line()),
                "SOKF_FORMAT_8",
                format!("manifest: {e}"),
            ));
            return m;
        }
    };
    let Some(entries) = node.as_map() else {
        m.problems.push(Finding::error(
            path.clone(),
            Some(node.line),
            "SOKF_FORMAT_8",
            "the manifest must be a map",
        ));
        return m;
    };
    for (key, value) in entries {
        m.line_of.insert(key.value.clone(), key.line);
        match key.value.as_str() {
            "sokf" => m.sokf = value.scalar_text(),
            "name" => m.name = value.as_str().map(str::to_string),
            "scope" => m.scope = value.as_str().map(str::to_string),
            "renamed-from" => {
                let Some(items) = value.as_seq() else {
                    m.problems.push(Finding::error(
                        path.clone(),
                        Some(value.line),
                        "SOKF_FORMAT_11",
                        "`renamed-from` must be a list of `<scope>/<name>` entries",
                    ));
                    continue;
                };
                for item in items {
                    match item.as_str() {
                        Some(old) => m.renamed_from.push((old.to_string(), item.line)),
                        None => m.problems.push(Finding::error(
                            path.clone(),
                            Some(item.line),
                            "SOKF_FORMAT_11",
                            "each `renamed-from` entry must be a `<scope>/<name>` string",
                        )),
                    }
                }
            }
            "description" => m.description = value.as_str().map(str::to_string),
            k if STAMPED_MANIFEST_KEYS.contains(&k) => {
                m.stamped_present.push((k.to_string(), key.line));
            }
            _ => {}
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml;

    #[test]
    fn manifest_parses_and_notes_stamped_keys() {
        let path = RepoPath::parse("/k/manifest.sokf.yaml").unwrap();
        let m = manifest(
            yaml::parse(
                "sokf: \"0.6\"\nname: n\nscope: test\nrenamed-from: [test/old, 1]\ndescription: d\ngenerated: x\n",
                1,
            ),
            &path,
        );
        assert_eq!(m.sokf.as_deref(), Some("0.6"));
        assert_eq!(m.name.as_deref(), Some("n"));
        assert_eq!(m.scope.as_deref(), Some("test"));
        assert_eq!(m.renamed_from, vec![("test/old".to_string(), 4)]);
        assert_eq!(m.problems.len(), 1, "{:?}", m.problems);
        assert_eq!(m.stamped_present, vec![("generated".to_string(), 6)]);
        assert_eq!(m.line_of["name"], 2);
        let m = manifest(yaml::parse("sokf: 0.6\n", 1), &path);
        assert_eq!(m.sokf.as_deref(), Some("0.6"));
        // A scalar is not a list, so the parser reports it, not drops it.
        let m = manifest(yaml::parse("renamed-from: test/old\n", 1), &path);
        assert!(m.renamed_from.is_empty());
        assert_eq!(m.problems.len(), 1, "{:?}", m.problems);
        assert_eq!(m.problems[0].criterion, "SOKF_FORMAT_11");
        assert_eq!(m.problems[0].line, Some(1));
        assert!(
            manifest(yaml::parse("- x\n", 1), &path).problems[0]
                .message
                .contains("must be a map")
        );
        assert!(
            manifest(yaml::parse("a: &x 1\n", 1), &path).problems[0]
                .message
                .contains("anchor")
        );
    }
}
