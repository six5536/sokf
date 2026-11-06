//! The conflict check: a line that begins with git's conflict marker
//! `<<<<<<< ` is an error, because a merge of `sokf pack sync` left it for
//! the user to resolve (D-5 of design-pack). A line inside a fenced code
//! block of a document is an example, not a conflict, so the check passes it
//! by. The fence of a schema is the schema itself, so every line of a schema
//! counts.

use std::ops::RangeInclusive;

use crate::{
    Finding, RepoPath, RepoRoot, Source,
    markdown::{Block, BlockDetail, BlockKind, Body},
    model::{Document, ParseMode, parse_source},
    pack::{FILES, Lock},
};

/// `MARKER` is the start of the first line of a conflict that git writes.
pub const MARKER: &str = "<<<<<<< ";

/// `marker_lines` returns the number of each line of `text` that begins
/// with `MARKER`, from 1, apart from the lines that `skip` holds.
fn marker_lines(text: &str, skip: &[RangeInclusive<usize>]) -> Vec<usize> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .filter(|(n, l)| l.starts_with(MARKER) && !skip.iter().any(|r| r.contains(n)))
        .map(|(n, _)| n)
        .collect()
}

/// `fences` returns the lines of each fenced code block of `body`, at any
/// depth, from its opening fence to its closing fence.
fn fences(body: &Body) -> Vec<RangeInclusive<usize>> {
    fn walk(blocks: &[Block], out: &mut Vec<RangeInclusive<usize>>) {
        for b in blocks {
            if b.kind == BlockKind::Code
                && matches!(b.detail, BlockDetail::Code { fence, .. } if fence > 0)
            {
                out.push(b.line..=b.end_line);
            }
            for u in &b.units {
                walk(&u.nested, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&body.blocks, &mut out);
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
        fences(&doc.body)
    };
    marker_lines(&doc.source.text, &skip)
}

/// `conflict_lines` returns the number of each line of the file `path`,
/// with the text `text`, that begins a conflict. It reads a `.md` file as a
/// document, as `document_conflicts` does, and counts every line of any
/// other file.
pub fn conflict_lines(path: &RepoPath, text: &str) -> Vec<usize> {
    if !path.as_str().ends_with(".md") {
        return marker_lines(text, &[]);
    }
    let source = Source::from_text(path.clone(), text);
    let parsed = parse_source(&source, ParseMode::Document);
    document_conflicts(&Document::new(source, parsed))
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

/// `pack_files` reports an error at each line that begins a conflict in
/// each file of a pack's `files/` that `lock` records. Such a file lies
/// outside every store, so no other check reads it.
// sokf-implements: CLI-4_AC-39
pub fn pack_files(root: &RepoRoot, lock: &Lock) -> Vec<Finding> {
    let mut out = Vec::new();
    for pack in lock.packs.values() {
        for (rel, f) in &pack.files {
            // A shadowed record names a file that its owner's record also
            // names, so only the owner's record reports it.
            if !rel.starts_with(FILES) || f.shadowed_by.is_some() {
                continue;
            }
            let Some(path) = RepoPath::parse(&f.path) else {
                continue;
            };
            let Ok(bytes) = std::fs::read(path.to_fs(root)) else {
                continue;
            };
            let text = String::from_utf8_lossy(&bytes);
            for line in conflict_lines(&path, &text) {
                out.push(Finding::error(
                    path.clone(),
                    Some(line),
                    "CLI-4_AC-39",
                    "the line begins a conflict that a merge of `sokf pack sync` left; keep the lines that the file needs, and delete the markers `<<<<<<<`, `=======`, and `>>>>>>>`",
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(path: &str, text: &str) -> Vec<usize> {
        conflict_lines(&RepoPath::parse(path).unwrap(), text)
    }

    // sokf-tests: CLI-4_AC-37
    #[test]
    fn each_line_that_begins_a_conflict_is_an_error() {
        let text = "a\n<<<<<<< ours\nb\n=======\nc\n>>>>>>> theirs\n <<<<<<< x\n<<<<<<<\n";
        assert_eq!(lines("/knowledge/a.md", text), vec![2]);
        assert_eq!(lines("/.sokf/packs/m/formats.yaml", text), vec![2]);
        assert!(lines("/knowledge/a.md", "no conflict\n").is_empty());
    }

    // sokf-tests: CLI-4_AC-38
    #[test]
    fn a_marker_in_a_fence_of_a_document_is_an_example() {
        let example = "```text\n<<<<<<< /knowledge/a.md\nmine\n=======\ntheirs\n>>>>>>> /knowledge/a.md\n```\n";
        let doc = format!("---\ntype: Note\nid: a\n---\n\n# A\n\n{example}\n<<<<<<< ours\n");
        assert_eq!(lines("/knowledge/a.md", &doc), vec![16]);
        // A fence in a list item is a fence too.
        let nested = format!("# A\n\n- An item.\n\n  ```text\n  x\n  ```\n\n{example}");
        assert!(lines("/knowledge/a.md", &nested).is_empty());
        // The fence of a schema is the schema, so its markers count, and a
        // file that isn't Markdown has no fence.
        let schema = "---\ntype: Schema\nid: schema-a\n---\n\n````yaml\n<<<<<<< x\n````\n";
        assert_eq!(lines("/knowledge/schemas/a.md", schema), vec![7]);
        assert_eq!(lines("/.sokf/packs/m/formats.yaml", example), vec![2]);
    }
}
