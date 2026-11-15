//! This module collects the pairs of texts that a pass of the check asks the
//! formatter to compare: each include block, definition block, and index of
//! the checked files that differs from its expected text byte for byte
//! (CLI-4_AC-45, CLI-4_AC-46).

use std::collections::BTreeSet;

use super::{Expected, render_definitions};
use crate::{Located, Model, RepoPath, Segments, formatter::Pair, index};

/// `pairs` returns the pairs of the files in `units`, or of every file when
/// `units` is `None`, and of each index in `indexes`. A block that holds its
/// expected text gives no pair, so a pass whose blocks are current needs no
/// formatter (CLI-4_AC-44).
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
            out.push(Pair {
                path: path.clone(),
                ext: "md".into(),
                expected: text.clone(),
                actual: file.source.text.clone(),
            });
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
            out.push(Pair {
                path: path.clone(),
                ext: "md".into(),
                expected: rendered.to_string(),
                actual: written.to_string(),
            });
        }
    }
}
