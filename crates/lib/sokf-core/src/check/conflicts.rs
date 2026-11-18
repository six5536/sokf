//! The conflict check: a line of a document that begins with git's conflict
//! marker `<<<<<<< ` is an error, because a merge left it for the user to
//! resolve. A line inside a fenced code
//! block of a document is an example, not a conflict, so the check passes it
//! by. The fence of a schema is the schema itself, so every line of a schema
//! counts.

use std::ops::Range;

use crate::{Finding, markdown, model::Document};

/// `MARKER` is the start of the first line of a conflict that git writes.
pub const MARKER: &str = "<<<<<<< ";

/// `marker_lines` returns the number of each line of `text` that begins
/// with `MARKER`, from 1, apart from the lines whose start lies in a range
/// of bytes that `skip` holds.
fn marker_lines(text: &str, skip: &[Range<usize>]) -> Vec<usize> {
    let mut start = 0;
    let mut out = Vec::new();
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if line.starts_with(MARKER) && !skip.iter().any(|r| r.contains(&start)) {
            out.push(i + 1);
        }
        start += line.len();
    }
    out
}

/// `document_conflicts` returns the number of each line of the document
/// `doc` that begins a conflict: every such line of a schema, and each such
/// line outside a fenced code block of any other document.
// sokf-implements: CLI-4_AC-38
pub fn document_conflicts(doc: &Document) -> Vec<usize> {
    let skip = if doc.type_name() == Some("Schema") {
        Vec::new()
    } else {
        markdown::fenced_bytes(&doc.body, &doc.source.text)
    };
    marker_lines(&doc.source.text, &skip)
}

/// `check` reports an error at each line of the document `doc` that begins
/// a conflict.
// sokf-implements: CLI-4_AC-37
pub fn check(doc: &Document) -> Vec<Finding> {
    document_conflicts(doc)
        .into_iter()
        .map(|line| {
            Finding::error(
                doc.path.clone(),
                Some(line),
                "CLI-4_AC-37",
                "the line begins a conflict that a merge left; keep the lines that the file needs, and delete the markers `<<<<<<<`, `=======`, and `>>>>>>>`",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RepoPath, Source,
        model::{ParseMode, parse_source},
    };

    fn lines(path: &str, text: &str) -> Vec<usize> {
        let source = Source::from_text(RepoPath::parse(path).unwrap(), text);
        let parsed = parse_source(&source, ParseMode::Document);
        document_conflicts(&Document::new(source, parsed))
    }

    // sokf-tests: CLI-4_AC-37
    #[test]
    fn each_line_that_begins_a_conflict_is_an_error() {
        let text = "a\n<<<<<<< ours\nb\n=======\nc\n>>>>>>> theirs\n <<<<<<< x\n<<<<<<<\n";
        assert_eq!(lines("/knowledge/a.md", text), vec![2]);
        assert_eq!(lines("/knowledge/a.md", "no conflict\n"), [] as [usize; 0]);
    }

    // sokf-tests: CLI-4_AC-38
    #[test]
    fn a_marker_in_a_fence_of_a_document_is_an_example() {
        let example = "```text\n<<<<<<< /knowledge/a.md\nmine\n=======\ntheirs\n>>>>>>> /knowledge/a.md\n```\n";
        let doc = format!("---\ntype: Note\nid: a\n---\n\n# A\n\n{example}\n<<<<<<< ours\n");
        assert_eq!(lines("/knowledge/a.md", &doc), vec![16]);
        // A fence in a list item is a fence too.
        let nested = format!("# A\n\n- An item.\n\n  ```text\n  x\n  ```\n\n{example}");
        assert_eq!(lines("/knowledge/a.md", &nested), [] as [usize; 0]);
        // A fence after carriage returns alone, which end a line for the
        // parser but not for the check, is a fence too.
        let crs = "\r".repeat(10);
        let cr = format!("---\ntype: Note\nid: a\n---\n# A{crs}\n{example}\n<<<<<<< ours\n");
        assert_eq!(lines("/knowledge/a.md", &cr), vec![14]);
        // The fence of a schema is the schema, so its markers count.
        let schema = "---\ntype: Schema\nid: schema-a\n---\n\n````yaml\n<<<<<<< x\n````\n";
        assert_eq!(lines("/knowledge/schemas/a.md", schema), vec![7]);
    }
}
