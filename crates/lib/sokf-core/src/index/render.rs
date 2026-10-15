//! This module holds the only renderer of the text of an index (format
//! specification §9.4).

use crate::{RepoPath, include::DefinitionEntry};

/// An `IndexEntry` is one entry of an index. The entries of an index render
/// in the order of the list that holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexEntry {
    /// `Document` is a document that a link can address by its id.
    Document {
        /// The link text, which is the `title` when the title is a string,
        /// and the `id` otherwise.
        title: String,
        /// The `id` of the document.
        id: String,
        /// The path of the document, which the definition block lists.
        path: RepoPath,
        /// The `description` of the document, when the description is a string.
        description: Option<String>,
    },
    /// `File` is a `.md` file that a link cannot address by an id, so the
    /// entry links the file by its relative path.
    File {
        /// The name of the file, without `.md`.
        name: String,
    },
    /// `Directory` is a subdirectory, which the entry links by its `index.md`.
    Directory {
        /// The name of the directory.
        name: String,
    },
}

/// `render` returns the text of an index: the heading, the list, and the
/// definition block, each separated from the next by a blank line. The text
/// ends with a single line break. The text of an index with no entries is the
/// heading alone, and an index with no document entries has no definition block.
// sokf-implements: SOKF_FORMAT_97
// sokf-implements: SOKF_FORMAT_98
// sokf-implements: SOKF_FORMAT_103
// sokf-implements: SOKF_FORMAT_104
// sokf-implements: SOKF_FORMAT_105
// sokf-implements: SOKF_FORMAT_106
// sokf-implements: SOKF_FORMAT_107
// sokf-implements: SOKF_FORMAT_108
// sokf-implements: SOKF_FORMAT_109
// sokf-implements: SOKF_FORMAT_110
pub fn render(heading: &str, entries: &[IndexEntry]) -> String {
    let mut out = format!("# {heading}\n");
    if entries.is_empty() {
        return out;
    }
    out.push('\n');
    let mut definitions = Vec::new();
    for entry in entries {
        match entry {
            IndexEntry::Document {
                title,
                id,
                path,
                description,
            } => {
                out.push_str(&format!("- [{}][sokf:{id}]", one_line(title)));
                if let Some(d) = description {
                    out.push_str(" - ");
                    out.push_str(&one_line(d));
                }
                out.push('\n');
                definitions.push(DefinitionEntry {
                    label: format!("sokf:{id}"),
                    path: path.clone(),
                    slug: None,
                });
            }
            IndexEntry::File { name } => out.push_str(&format!("- [{name}]({name}.md)\n")),
            IndexEntry::Directory { name } => {
                out.push_str(&format!("- [{name}/]({name}/index.md)\n"));
            }
        }
    }
    definitions.sort();
    definitions.dedup();
    if !definitions.is_empty() {
        out.push('\n');
        out.push_str(&crate::include::render_definitions(&definitions));
    }
    out
}

/// `one_line` returns `text` with each line break written as one space.
fn one_line(text: &str) -> String {
    text.lines().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn doc(title: &str, id: &str, path: &str, description: Option<&str>) -> IndexEntry {
        IndexEntry::Document {
            title: title.into(),
            id: id.into(),
            path: p(path),
            description: description.map(str::to_string),
        }
    }

    // sokf-tests: SOKF_FORMAT_103
    // sokf-tests: SOKF_FORMAT_108
    // sokf-tests: SOKF_FORMAT_109
    // sokf-tests: SOKF_FORMAT_110
    #[test]
    fn renders_the_example_of_spec_9() {
        let entries = vec![
            doc(
                "Executor",
                "executor",
                "/knowledge/core/executor.md",
                Some("Applies planned actions."),
            ),
            doc(
                "Planner",
                "planner",
                "/knowledge/core/planner.md",
                Some("Pure planning stage. No filesystem writes."),
            ),
            IndexEntry::Directory {
                name: "plugins".into(),
            },
        ];
        assert_eq!(
            render("core", &entries),
            "# core\n\n\
             - [Executor][sokf:executor] - Applies planned actions.\n\
             - [Planner][sokf:planner] - Pure planning stage. No filesystem writes.\n\
             - [plugins/](plugins/index.md)\n\n\
             <!-- sokf:links -->\n\n\
             [sokf:executor]: /knowledge/core/executor.md\n\
             [sokf:planner]: /knowledge/core/planner.md\n"
        );
    }

    // sokf-tests: SOKF_FORMAT_105
    // sokf-tests: SOKF_FORMAT_106
    // sokf-tests: SOKF_FORMAT_107
    // sokf-tests: SOKF_FORMAT_108
    // sokf-tests: SOKF_FORMAT_109
    #[test]
    fn each_entry_form_and_the_empty_cases() {
        assert_eq!(render("schemas", &[]), "# schemas\n");
        // When no entry links a document by its id, the index has no definition block.
        let entries = vec![
            IndexEntry::Directory {
                name: "core".into(),
            },
            IndexEntry::File {
                name: "core".into(),
            },
        ];
        assert_eq!(
            render("k", &entries),
            "# k\n\n- [core/](core/index.md)\n- [core](core.md)\n"
        );
        // An entry without a description has no ` - `. Each line break becomes
        // one space. `render` sorts the definitions by label, whatever the
        // order of the list.
        let entries = vec![
            doc("Zed\nTwo", "zed", "/k/zed.md", None),
            doc("A", "a", "/k/a.md", Some("line one\nline two\n")),
        ];
        assert_eq!(
            render("k", &entries),
            "# k\n\n- [Zed Two][sokf:zed]\n- [A][sokf:a] - line one line two\n\n\
             <!-- sokf:links -->\n\n[sokf:a]: /k/a.md\n[sokf:zed]: /k/zed.md\n"
        );
    }
}
