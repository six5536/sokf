//! This module collects the pairs of texts that a pass of the check asks
//! same-same to compare: each include block, definition block, and index of
//! the checked files that differs from its expected text byte for byte
//! (CLI-4_AC-45, CLI-4_AC-46).

use std::collections::BTreeSet;

use super::{DefinitionEntry, Expected, render_definitions};
use crate::{Located, Model, RepoPath, Segments, compare::Pair, index};

/// `pairs` returns the pairs of the files in `units`, or of every file when
/// `units` is `None`, and of each index in `indexes`. A block that holds its
/// expected text gives no pair, so a pass whose blocks are current needs no
/// comparison (CLI-4_AC-44).
// sokf-implements: CLI-4_AC-46
pub fn pairs(
    model: &Model,
    expected: &Expected,
    indexes: Option<&index::Indexes>,
    units: Option<&BTreeSet<RepoPath>>,
) -> Vec<Pair> {
    let mut out = Vec::new();
    let wanted = |path: &RepoPath| units.is_none_or(|u| u.contains(path));
    let files = model
        .documents_and_schemas()
        .map(|(_, d)| (&d.path, &d.segments))
        .chain(model.additional.iter().map(|f| (&f.path, &f.segments)));
    for (path, segments) in files {
        if wanted(path) {
            file_pairs(expected, path, segments, &mut out);
        }
    }
    for (path, text) in indexes.map(|i| &i.expected).into_iter().flatten() {
        // An index that a formatter re-laid can fall out of the generated
        // form, such as an entry wrapped over two lines, so the pair holds
        // any index that differs.
        if let Some(Located::Index(_, file)) = model.by_path(path)
            && file.source.text != *text
        {
            out.push(index_pair(path, text, &file.source.text));
        }
    }
    out
}

/// `file_pairs` adds the pairs of the include blocks and the definition
/// block of the file at `path` to `out`.
fn file_pairs(expected: &Expected, path: &RepoPath, segments: &Segments, out: &mut Vec<Pair>) {
    for block in &segments.regions {
        out.extend(expected.include_pair(path, block.open_line, &block.content));
    }
    let Some(Some(entries)) = expected.definitions.get(path) else {
        return;
    };
    if let Some(def) = &segments.definition
        && !def.trailing
        && !entries.is_empty()
    {
        let rendered = render_definitions(entries);
        let (rendered, written) = (rendered.trim_end(), def.text.trim_end());
        if rendered != written {
            out.push(definition_pair(path, rendered, written));
        }
    }
}

/// `index_pair` returns the pair of the index at `path`, which sokf would
/// write as `expected` and which holds `actual`, both read as a whole
/// Markdown file under the index's path (CLI-4_AC-113).
pub fn index_pair(path: &RepoPath, expected: &str, actual: &str) -> Pair {
    Pair {
        path: path.clone(),
        name: crate::compare::name_of(path),
        fragment: false,
        expected: expected.into(),
        actual: actual.into(),
        source: None,
    }
}

/// `index_labels_as_written` returns whether the index `actual` holds the
/// labels that sokf writes in `expected`, the label of each reference and
/// of each definition, in their case and their order. Markdown matches a
/// reference to its definition whatever the label's case, so same-same reads
/// a label in another case, or an inline link in place of a reference, as
/// the same, which no formatter writes, and sokf compares the labels itself
/// (CLI-4_AC-117).
// sokf-implements: CLI-4_AC-117
pub(crate) fn index_labels_as_written(expected: &str, actual: &str) -> bool {
    written_labels(expected) == written_labels(actual)
}

/// `written_labels` returns the labels of `text` as written, in their order:
/// the label of each full reference, `[text][label]`, before the marker of
/// the definition block, and of each definition, `[label]: destination`,
/// after it.
fn written_labels(text: &str) -> Vec<&str> {
    let mut labels = Vec::new();
    let mut definitions = false;
    for line in text.lines() {
        if line.trim() == "<!-- sokf:links -->" {
            definitions = true;
            continue;
        }
        if definitions {
            let open = line.trim_start_matches(' ');
            if line.len() - open.len() <= 3
                && let Some(rest) = open.strip_prefix('[')
                && let Some(end) = rest.find(']')
                && rest[end + 1..].starts_with(':')
            {
                labels.push(&rest[..end]);
            }
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find("][") {
            let after = &rest[at + 2..];
            let Some(end) = after.find(']') else { break };
            labels.push(&after[..end]);
            rest = &after[end..];
        }
    }
    labels
}

/// `labels_as_written` returns whether the definition block `written` holds
/// the labels of `entries` as sokf writes them, in their order. Markdown
/// matches a label whatever its case, so same-same reads a label written in
/// another case as the same, which no formatter writes, and sokf compares
/// the labels itself (CLI-4_AC-112).
pub fn labels_as_written(
    entries: &[DefinitionEntry],
    written: &crate::markdown::DefinitionBlock,
) -> bool {
    entries.len() == written.entries.len()
        && entries
            .iter()
            .zip(&written.entries)
            .all(|(e, (label, _))| e.label == *label)
}

/// `definition_pair` returns the pair of the definition block of the file
/// at `path`, which sokf would write as `rendered` and which holds
/// `written`, both read as a part of a Markdown file under the file's path,
/// or its path with `.md` added where same-same doesn't read its name as
/// Markdown, such as an additional file `/README` (CLI-4_AC-112,
/// CLI-4_AC-113).
// sokf-implements: CLI-4_AC-112
pub fn definition_pair(path: &RepoPath, rendered: &str, written: &str) -> Pair {
    let name = crate::compare::name_of(path);
    Pair {
        path: path.clone(),
        name: if same_same::Format::from_name(&name) == Some(same_same::Format::Markdown) {
            name
        } else {
            format!("{name}.md")
        },
        fragment: true,
        expected: rendered.into(),
        actual: written.into(),
        source: None,
    }
}
