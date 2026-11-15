//! This module finds the fenced code blocks of a Markdown body, whose
//! lines are examples: a conflict check and a scan for markers pass them
//! by.

use std::ops::RangeInclusive;

use super::{Block, BlockDetail, BlockKind, Body};

/// `fenced_lines` returns the lines of each fenced code block of `body`,
/// whose text is `text`, from its opening fence to its closing fence, in
/// the order of the lines: each that the blocks of the body hold, at any
/// depth of its lists, and each in a quote, whose code blocks the parser
/// records only by their lines. A fence inside a footnote is missing,
/// because the parser records no block of a footnote, and so is a fence
/// in a quote that a list item opens or holds at a wider indent.
pub fn fenced_lines(body: &Body, text: &str) -> Vec<RangeInclusive<usize>> {
    fn walk(blocks: &[Block], fenced: &mut Vec<RangeInclusive<usize>>, code: &mut Vec<usize>) {
        for b in blocks {
            if b.kind == BlockKind::Code {
                code.push(b.line);
                if matches!(b.detail, BlockDetail::Code { fence, .. } if fence > 0) {
                    fenced.push(b.line..=b.end_line);
                }
            }
            for u in &b.units {
                walk(&u.nested, fenced, code);
            }
        }
    }
    let mut out = Vec::new();
    let mut code = Vec::new();
    walk(&body.blocks, &mut out, &mut code);
    code.sort_unstable();
    // Each other code block lies in a quote: it is fenced where its first
    // line opens a fence after the marks of the quotes.
    let quoted: Vec<&(usize, usize)> = body
        .fenced
        .iter()
        .filter(|(start, _)| code.binary_search(start).is_err())
        .collect();
    if !quoted.is_empty() {
        let lines: Vec<&str> = text.lines().collect();
        out.extend(
            quoted
                .into_iter()
                .filter(|&&(start, _)| {
                    start
                        .checked_sub(1)
                        .and_then(|i| lines.get(i))
                        .is_some_and(|line| opens_quoted_fence(line))
                })
                .map(|&(start, end)| start..=end),
        );
        out.sort_by_key(|r| *r.start());
    }
    out
}

/// `opens_quoted_fence` returns whether `line` opens a fenced code block
/// after the marks of the quotes that hold it: each `>` follows at most
/// three spaces, and takes one space or tab after it, and the fence,
/// `` ``` `` or `~~~`, follows at most three spaces.
fn opens_quoted_fence(line: &str) -> bool {
    let mut rest = line;
    loop {
        let indent = rest.len() - rest.trim_start_matches(' ').len();
        if indent > 3 {
            return false;
        }
        let trimmed = &rest[indent..];
        match trimmed.strip_prefix('>') {
            Some(after) => rest = after.strip_prefix([' ', '\t']).unwrap_or(after),
            None => return trimmed.starts_with("```") || trimmed.starts_with("~~~"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_fence_opens_after_the_marks_of_its_quotes() {
        for line in [
            "> ```",
            ">```",
            ">> ~~~rust",
            "   > ```",
            ">\t```",
            "> > ```",
        ] {
            assert!(opens_quoted_fence(line), "{line}");
        }
        for line in [">     ```", "    > ```", "> text", ""] {
            assert!(!opens_quoted_fence(line), "{line}");
        }
    }
}
