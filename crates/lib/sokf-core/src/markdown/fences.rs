//! This module finds the fenced code blocks of a Markdown body, whose
//! lines are examples: a conflict check and a scan for markers pass them
//! by.

use std::ops::Range;

use super::Body;

/// `fenced_bytes` returns the bytes of each fenced code block of `body`,
/// from its opening fence to its closing fence, in the order of the text,
/// at any depth of quotes, lists, and footnotes: each fenced block of the
/// list of code blocks that the parser gives the body. The offsets count
/// from the start of `text`, the text of the file, in which the body begins
/// at line `body.first_line`, with lines counted at each line feed.
pub fn fenced_bytes(body: &Body, text: &str) -> Vec<Range<usize>> {
    let start: usize = text
        .split_inclusive('\n')
        .take(body.first_line.saturating_sub(1))
        .map(str::len)
        .sum();
    body.code_blocks
        .iter()
        .filter(|b| b.fenced)
        .map(|b| b.bytes.start + start..b.bytes.end + start)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::{Segments, parse};

    #[test]
    fn a_fence_counts_its_bytes_from_the_start_of_the_file() {
        let text = "---\nid: a\n---\n# A\r\r\n> - ```\n>   x\n>   ```\n\n    indented\n";
        let body_text = &text["---\nid: a\n---\n".len()..];
        let body = parse(body_text, 4, &Segments::default());
        let fenced = fenced_bytes(&body, text);
        let texts: Vec<&str> = fenced.iter().map(|r| &text[r.clone()]).collect();
        assert_eq!(texts, ["> - ```\n>   x\n>   ```"]);
    }
}
