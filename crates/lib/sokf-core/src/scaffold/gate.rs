//! This module gates the text of a scaffold by its `sokf:only` markers. It
//! follows the rule in the agent reference for what `sokf new` copies from a
//! scaffold.

use crate::{
    RepoPath, Source,
    grammar::{Only, States},
    markdown::{is_guidance_marker, is_only_marker, is_optional_mark, only_marker},
    model::{ParseMode, parse_source},
    source::split,
};

/// `is_marker` reports whether a line is a marker of any kind. The search
/// for the part that a `sokf:only` marker marks skips these lines.
fn is_marker(line: &str) -> bool {
    is_only_marker(line) || is_optional_mark(line) || is_guidance_marker(line)
}

/// `drop_guidance_above` drops the guidance lines directly above line `i`,
/// but not past line `floor`. These lines guide the part that a dropped
/// marker gated, and guidance left for a missing part would make the new
/// document fail validation for no reason.
fn drop_guidance_above(lines: &[&str], out: &mut [Option<String>], i: usize, floor: usize) {
    let mut k = i;
    while k > floor && is_guidance_marker(lines[k - 1]) {
        out[k - 1] = None;
        k -= 1;
    }
}

/// `gate` returns the scaffold without each part whose `sokf:only` marker
/// excludes `states`. It strips every remaining `sokf:only` marker. It
/// keeps a `sokf:optional` region whole, with its markers, because the
/// reader decides whether to keep that region. A marked part of the body is
/// the section that a heading opens, or else the block that starts on the
/// marked line. A marked frontmatter key includes its continuation lines.
// sokf-implements: CLI-6_AC-35
// sokf-implements: EXT_P-4
// sokf-implements: CLI-6_AC-2
// sokf-implements: CLI-6_AC-7
pub fn gate(document: &str, states: &States) -> String {
    let lines: Vec<&str> = document.lines().collect();
    let source = Source::from_text(RepoPath::root(), document);
    let parsed = parse_source(&source, ParseMode::Document);
    let mut out: Vec<Option<String>> = lines.iter().map(|l| Some((*l).to_string())).collect();
    // Gate the frontmatter, the lines between its fences, when the document
    // has one. The code finds the fences by their text, so it still gates,
    // line by line, a frontmatter that does not parse.
    let close = split(&source)
        .frontmatter
        .map(|_| parsed.body_line - 2)
        .filter(|&c| c < lines.len());
    if let Some(close) = close {
        let mut i = 1;
        while i < close {
            let line = lines[i];
            if !is_only_marker(line) {
                i += 1;
                continue;
            }
            if admitted(line, states) {
                let cut = line.rfind('#').unwrap_or(line.len());
                out[i] = Some(line[..cut].trim_end().to_string());
                i += 1;
                continue;
            }
            let end = continuation_end(&lines, i, close);
            out[i..end].fill(None);
            drop_guidance_above(&lines, &mut out, i, 1);
            i = end;
        }
    }
    // Gate the body.
    let body_start = close.map_or(0, |c| c + 1);
    for i in body_start..lines.len() {
        let line = lines[i];
        if !line.trim_start().starts_with("<!--")
            || !is_only_marker(line)
            || parsed.body.in_code(i + 1)
        {
            continue;
        }
        out[i] = None;
        if admitted(line, states) {
            continue;
        }
        drop_guidance_above(&lines, &mut out, i, body_start);
        let Some(marked) =
            (i + 1..lines.len()).find(|&j| !lines[j].trim().is_empty() && !is_marker(lines[j]))
        else {
            continue;
        };
        let end = extent(&parsed.body, &lines, marked);
        for slot in out.iter_mut().take(end + 1).skip(marked) {
            *slot = None;
        }
        if end + 1 < lines.len() && lines[end + 1].trim().is_empty() {
            out[end + 1] = None;
        }
    }
    let mut text: String = out.into_iter().flatten().collect::<Vec<_>>().join("\n");
    if document.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// `admitted` reports whether a marker admits the states. A malformed
/// marker admits them, because the validator has already reported it.
fn admitted(line: &str, states: &States) -> bool {
    only_marker(line).is_none_or(|map| Only(map).admits(states))
}

/// `continuation_end` returns the index after the last continuation line of
/// the frontmatter key on line `i`. The continuation lines are the lines
/// before `close` that follow the key with a deeper indent, including the
/// blank lines between them.
pub(super) fn continuation_end(lines: &[&str], i: usize, close: usize) -> usize {
    let own = indent(lines[i]);
    let mut j = i + 1;
    while j < close && (lines[j].trim().is_empty() || indent(lines[j]) > own) {
        if lines[j].trim().is_empty() && !next_deeper(lines, j, close, own) {
            break;
        }
        j += 1;
    }
    j
}

/// `indent` returns the indentation of a frontmatter line.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// `next_deeper` reports whether the next non-blank line from `from` up to
/// `close` is indented deeper than `own`. When it is, the blank line at
/// `from` belongs to the key's value.
fn next_deeper(lines: &[&str], from: usize, close: usize, own: usize) -> bool {
    (from..close)
        .find(|&j| !lines[j].trim().is_empty())
        .is_some_and(|j| indent(lines[j]) > own)
}

/// `extent` returns the index of the last line of the part that starts at
/// index `marked`. When a heading starts the part, the part is the
/// heading's section, up to the line before the next heading of the same or
/// a higher level. The part leaves out the section's trailing blank lines
/// and marker lines. Otherwise the
/// part is the block that starts on that line, or else the line alone.
fn extent(body: &crate::Body, lines: &[&str], marked: usize) -> usize {
    let line_no = marked + 1;
    if let Some(h) = body.headings.iter().find(|h| h.line == line_no) {
        let stop = body
            .headings
            .iter()
            .find(|n| n.line > h.line && n.level <= h.level)
            .map_or(lines.len(), |n| n.line - 1);
        let mut end = stop;
        while end > marked + 1
            && (lines[end - 1].trim().is_empty()
                || is_only_marker(lines[end - 1])
                || is_guidance_marker(lines[end - 1]))
        {
            end -= 1;
        }
        return end - 1;
    }
    if let Some(b) = body.blocks.iter().find(|b| b.line == line_no) {
        return b.end_line - 1;
    }
    marked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::fixtures::states;

    const TEMPLATE: &str = "---\ntype: Issue\nid: x\nstate: open\nresolution: none # sokf:only state=closed\nreviewers: # sokf:only kind=feature\n  - a\n  - b\ntags: [t]\n---\n\n# Title\n\nIntro.\n\n<!-- sokf:only kind=bug -->\n## Reproduction\n\nSteps.\n\n### Detail\n\nMore.\n\n<!-- sokf:only state=closed -->\n## Conclusion\n\nDone.\n\n## Notes\n\n<!-- sokf:only kind=bug -->\n- one\n- two\n\nAfter.\n\n```\n<!-- sokf:only state=closed -->\nkept as text\n```\n";

    // sokf-tests: CLI-6_AC-2
    #[test]
    fn an_open_bug_keeps_the_bug_parts_and_drops_the_closed_ones() {
        let out = gate(TEMPLATE, &states(&[("state", "open"), ("kind", "bug")]));
        assert_eq!(
            out,
            "---\ntype: Issue\nid: x\nstate: open\ntags: [t]\n---\n\n# Title\n\nIntro.\n\n## Reproduction\n\nSteps.\n\n### Detail\n\nMore.\n\n## Notes\n\n- one\n- two\n\nAfter.\n\n```\n<!-- sokf:only state=closed -->\nkept as text\n```\n"
        );
    }

    // sokf-tests: CLI-6_AC-2
    #[test]
    fn a_closed_feature_keeps_the_other_parts() {
        let out = gate(
            TEMPLATE,
            &states(&[("state", "closed"), ("kind", "feature")]),
        );
        assert_eq!(
            out,
            "---\ntype: Issue\nid: x\nstate: open\nresolution: none\nreviewers:\n  - a\n  - b\ntags: [t]\n---\n\n# Title\n\nIntro.\n\n## Conclusion\n\nDone.\n\n## Notes\n\nAfter.\n\n```\n<!-- sokf:only state=closed -->\nkept as text\n```\n"
        );
    }

    // sokf-tests: CLI-6_AC-2
    #[test]
    fn edge_cases_two_markers_a_top_heading_and_the_last_line() {
        // `gate` drops a part with two markers when either marker excludes the states.
        let two =
            "---\ntype: T\n---\n\n<!-- sokf:only a=x -->\n<!-- sokf:only b=y -->\n# H\n\nBody.\n";
        assert_eq!(
            gate(two, &states(&[("a", "x"), ("b", "y")])),
            "---\ntype: T\n---\n\n# H\n\nBody.\n"
        );
        assert_eq!(
            gate(two, &states(&[("a", "x"), ("b", "z")])),
            "---\ntype: T\n---\n\n"
        );
        // A level-one section runs to the end.
        let top = "# A\n\n<!-- sokf:only a=x -->\n# B\n\nb\n\n## B1\n\nb1\n";
        assert_eq!(gate(top, &states(&[("a", "y")])), "# A\n\n");
        assert_eq!(
            gate(top, &states(&[("a", "x")])),
            "# A\n\n# B\n\nb\n\n## B1\n\nb1\n"
        );
        // A marker on the last line marks nothing.
        assert_eq!(
            gate("# A\n\n<!-- sokf:only a=x -->\n", &states(&[("a", "y")])),
            "# A\n\n"
        );
        // A malformed marker admits the states, and `gate` strips it.
        assert_eq!(
            gate("# A\n\n<!-- sokf:only nonsense -->\nkept\n", &states(&[])),
            "# A\n\nkept\n"
        );
        // A document without a trailing newline gets none.
        assert_eq!(
            gate("# A\n<!-- sokf:only a=x -->\ngone", &states(&[("a", "y")])),
            "# A"
        );
    }
}
