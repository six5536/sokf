//! This module builds the text of the manifest and the two indexes of a new
//! store, which `sokf init` writes to disk.

use crate::{
    RepoPath,
    index::{IndexEntry, render},
    model::manifest::SOKF_VERSION,
};

/// `StoreFiles` holds the files of a new store and the text of each. Every
/// line of the text ends in a line feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreFiles {
    /// `files` holds each file's path and text.
    pub files: Vec<(RepoPath, String)>,
}

/// `store` returns the files of a new store under `dir`: the manifest, the
/// root index, and the `schemas/` index. The root index lists `schemas/`,
/// and the `schemas/` index lists nothing.
// sokf-implements: CLI-2_AC-24
// sokf-implements: SOKF_FORMAT_9
// sokf-implements: SOKF_FORMAT_10
// sokf-implements: SOKF_FORMAT_96
// sokf-implements: SOKF_FORMAT_99
// sokf-implements: CLI-2_AC-1
// sokf-implements: CLI-2_AC-2
// sokf-implements: CLI-2_AC-4
// sokf-implements: CLI-2_AC-5
// sokf-implements: CLI-2_AC-6
// sokf-implements: CLI-2_AC-7
pub fn store(dir: &RepoPath, name: &str, scope: &str, description: Option<&str>) -> StoreFiles {
    let mut manifest = format!(
        "sokf: \"{SOKF_VERSION}\"\nname: {}\nscope: {}\n",
        scalar(name),
        scalar(scope)
    );
    // sokf-implements: CLI-2_AC-22
    if let Some(d) = description {
        manifest.push_str(&format!("description: {}\n", scalar(d)));
    }
    let root_index = render(
        name,
        &[IndexEntry::Directory {
            name: "schemas".into(),
        }],
    );
    let schemas_index = render("schemas", &[]);
    let join = |rel: &str| dir.join(rel).expect("the file names are literal and valid");
    StoreFiles {
        files: vec![
            (join("manifest.sokf.yaml"), manifest),
            (join("index.md"), root_index),
            (
                join("schemas")
                    .join("index.md")
                    .expect("the file name is literal and valid"),
                schemas_index,
            ),
        ],
    }
}

/// `scalar` returns `text` as a YAML scalar: plain when the plain form reads
/// back as the same string, and double-quoted otherwise.
// sokf-implements: CLI-2_AC-26
fn scalar(text: &str) -> String {
    let plain = !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " _.-".contains(c))
        && !text.ends_with(' ')
        && !["true", "false", "null", "yes", "no", "on", "off"]
            .contains(&text.to_ascii_lowercase().as_str());
    if plain {
        return text.to_string();
    }
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    // sokf-tests: CLI-2_AC-4
    // sokf-tests: CLI-2_AC-6
    // sokf-tests: CLI-2_AC-22
    // sokf-tests: CLI-2_AC-24
    #[test]
    fn the_three_texts_with_and_without_a_description() {
        let files = store(
            &p("/docs/kb"),
            "project-kb",
            "test",
            Some("Knowledge for the project."),
        );
        let texts: Vec<(&str, &str)> = files
            .files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        assert_eq!(
            texts,
            vec![
                (
                    "/docs/kb/manifest.sokf.yaml",
                    "sokf: \"0.6\"\nname: project-kb\nscope: test\ndescription: Knowledge for the project.\n"
                ),
                (
                    "/docs/kb/index.md",
                    "# project-kb\n\n- [schemas/](schemas/index.md)\n"
                ),
                ("/docs/kb/schemas/index.md", "# schemas\n"),
            ]
        );
        let files = store(&p("/knowledge"), "knowledge", "test", None);
        assert_eq!(
            files.files[0].1,
            "sokf: \"0.6\"\nname: knowledge\nscope: test\n"
        );
        assert_eq!(files.files[1].0, p("/knowledge/index.md"));
    }

    #[test]
    fn a_name_that_needs_quoting_is_double_quoted() {
        assert_eq!(scalar("Project KB"), "Project KB");
        assert_eq!(scalar("a: b"), "\"a: b\"");
        assert_eq!(scalar(" lead"), "\" lead\"");
        assert_eq!(scalar("trail "), "\"trail \"");
        assert_eq!(scalar("#hash"), "\"#hash\"");
        assert_eq!(scalar("yes"), "\"yes\"");
        assert_eq!(scalar("1.0"), "\"1.0\"");
        assert_eq!(scalar("say \"hi\"\\"), "\"say \\\"hi\\\"\\\\\"");
        assert_eq!(scalar("two\nlines"), "\"two\\nlines\"");
        assert_eq!(scalar("é"), "\"é\"");
        // The quoted form reads back as the text.
        for text in ["a: b", "say \"hi\"", "two\nlines", "\t"] {
            let yaml = crate::yaml::parse(&format!("k: {}\n", scalar(text)), 1).unwrap();
            assert_eq!(yaml.get("k").unwrap().as_str(), Some(text), "{text:?}");
        }
    }
}
