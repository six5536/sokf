//! This module scans one repository file for markers (sokf-format-spec §9.5).
//! A case-insensitive search finds the lines that contain `sokf-` or
//! `sokf:`, and the scan parses only those lines.

use std::sync::OnceLock;

use regex::bytes::Regex;
use serde::{Deserialize, Serialize};

/// `BINARY_PROBE` is the number of bytes at the start of a file that the
/// scan searches for a NUL byte to decide whether the file is binary
/// (SOKF_VALIDATION_50).
pub const BINARY_PROBE: usize = 8 * 1024;

/// A `MarkerRow` is one line that the scan found: a marker, or a fault in
/// a line that looks like a marker.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MarkerRow {
    /// The line number, counted from 1.
    pub line: usize,
    /// The kind of the line: a marker or a fault.
    pub kind: RowKind,
}

/// A `RowKind` is the kind of a scanned line: a marker or a fault.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RowKind {
    /// A marker, with its rel and its reference as written.
    Marker {
        /// The rel of the tag.
        rel: String,
        /// The reference of the tag.
        target: String,
    },
    /// A tag with no reference after it, or with a letter or a digit after
    /// its reference (SOKF_FORMAT_144). The value is the tag as written.
    Malformed(String),
    /// A tag that matches `sokf-<rel>:` only when case is ignored
    /// (SOKF_VALIDATION_52). The value is the tag as written.
    Case(String),
    /// An ignore directive that is out of order (SOKF_FORMAT_152 and
    /// SOKF_FORMAT_153).
    Ignore(IgnoreFault),
}

/// An `IgnoreFault` is the way in which an ignore directive is out of
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IgnoreFault {
    /// A `sokf:ignore-begin` inside an ignore region.
    BeginInside,
    /// A `sokf:ignore-end` outside an ignore region.
    EndOutside,
    /// A `sokf:ignore-begin` that no `sokf:ignore-end` closes.
    NeverEnds,
}

/// A `Directive` is one of the ignore directives, which stop the scan from
/// reading text as markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Directive {
    Begin,
    End,
    NextLine,
}

/// `prefilter` returns the case-insensitive pattern that finds the lines
/// that contain `sokf-` or `sokf:`.
fn prefilter() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)sokf[-:]").expect("a literal pattern"))
}

/// `is_binary` returns whether a file is binary, that is, whether its first
/// 8 KB contain a NUL byte.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(BINARY_PROBE)].contains(&0)
}

/// `scan` returns the markers of a file and the faults in the lines that
/// look like markers, in line order. It returns `None` for a binary file,
/// which it doesn't read for markers (SOKF_VALIDATION_50).
// sokf-implements: SOKF_FORMAT_142
// sokf-implements: SOKF_FORMAT_143
// sokf-implements: SOKF_FORMAT_144
// sokf-implements: SOKF_FORMAT_151
// sokf-implements: SOKF_FORMAT_152
// sokf-implements: SOKF_FORMAT_153
// sokf-implements: SOKF_VALIDATION_50
// sokf-implements: SOKF_VALIDATION_52
pub fn scan(bytes: &[u8]) -> Option<Vec<MarkerRow>> {
    if is_binary(bytes) {
        return None;
    }
    let mut rows = Vec::new();
    // The scan tracks the line of the last hit and the offset where that
    // line starts, so that it counts each line once, however many hits the
    // line holds.
    let mut line = 1;
    let mut counted = 0;
    let mut last_line = 0;
    let mut open: Option<usize> = None;
    let mut skip: Option<usize> = None;
    for hit in prefilter().find_iter(bytes) {
        let start = bytes[..hit.start()]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        if start < counted {
            continue;
        }
        line += bytes[counted..start]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        counted = start;
        if line == last_line {
            continue;
        }
        last_line = line;
        let end = bytes[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |i| start + i);
        let text = String::from_utf8_lossy(&bytes[start..end]);
        let word = first_word(&text);
        if let Some(directive) = directive(word) {
            match (directive, open) {
                (Directive::Begin, Some(_)) => rows.push(MarkerRow {
                    line,
                    kind: RowKind::Ignore(IgnoreFault::BeginInside),
                }),
                (Directive::Begin, None) => open = Some(line),
                (Directive::End, Some(_)) => open = None,
                (Directive::End, None) => rows.push(MarkerRow {
                    line,
                    kind: RowKind::Ignore(IgnoreFault::EndOutside),
                }),
                // Inside a region, this directive adds nothing, because the
                // region already skips the next line.
                (Directive::NextLine, _) => skip = Some(line + 1),
            }
            continue;
        }
        if open.is_some() || skip == Some(line) {
            continue;
        }
        if let Some(kind) = parse(word) {
            rows.push(MarkerRow { line, kind });
        }
    }
    if let Some(begin) = open {
        rows.push(MarkerRow {
            line: begin,
            kind: RowKind::Ignore(IgnoreFault::NeverEnds),
        });
        rows.sort();
    }
    Some(rows)
}

/// `first_word` returns the rest of the line from the first character
/// that is a letter, a digit, or a non-ASCII character. It skips the
/// whitespace and ASCII punctuation before a marker, such as the `//` or
/// `#` that opens a comment (SOKF_FORMAT_142).
fn first_word(line: &str) -> &str {
    line.trim_start_matches(|c: char| c.is_ascii() && !c.is_ascii_alphanumeric())
}

/// `directive` returns the ignore directive that `word` starts with, when
/// only whitespace and characters that are neither letters nor digits
/// follow it, such as the `-->` that ends an HTML comment. A letter or a
/// digit after it, as in `sokf:ignore-begin-x`, makes it no directive.
// sokf-implements: SOKF_FORMAT_156
fn directive(word: &str) -> Option<Directive> {
    let rest = word.strip_prefix("sokf:ignore-")?;
    let (directive, tail) = [
        ("begin", Directive::Begin),
        ("end", Directive::End),
        ("next-line", Directive::NextLine),
    ]
    .into_iter()
    .find_map(|(name, d)| rest.strip_prefix(name).map(|tail| (d, tail)))?;
    (!tail.chars().any(char::is_alphanumeric)).then_some(directive)
}

/// `parse` returns the marker that starts `word`, or the fault in a tag
/// that looks like a marker. It returns `None` when `word` doesn't start
/// with a tag.
fn parse(word: &str) -> Option<RowKind> {
    let (tag, rest) = tag(word)?;
    if !tag.starts_with("sokf-") || tag.bytes().any(|b| b.is_ascii_uppercase()) {
        return Some(RowKind::Case(tag.to_string()));
    }
    let rel = &tag["sokf-".len()..tag.len() - 1];
    let rest = rest.trim_start();
    let len = rest
        .find(|c: char| !is_reference_char(c))
        .unwrap_or(rest.len());
    // The scan trims a full stop, a hyphen, a slash, or a colon from the end
    // of a reference, because these characters follow a reference as the end
    // of a sentence or of a comment, such as `-->`.
    let target = rest[..len].trim_end_matches(['.', '-', '/', ':']);
    let after = &rest[target.len()..];
    if target.is_empty() || after.chars().any(char::is_alphanumeric) {
        return Some(RowKind::Malformed(tag.to_string()));
    }
    Some(RowKind::Marker {
        rel: rel.to_string(),
        target: target.to_string(),
    })
}

/// `tag` returns the tag that starts `word` and the text after the tag. A
/// tag is `sokf-<rel>:` in any case, followed by whitespace or the end of
/// the line. A qualified reference, such as `sokf-spec:glossary`, has no
/// whitespace after its colon, and so it isn't a tag.
fn tag(word: &str) -> Option<(&str, &str)> {
    let head = word.get(..5)?;
    if !head.eq_ignore_ascii_case("sokf-") {
        return None;
    }
    let colon = word.find(':')?;
    let rel = &word[5..colon];
    let kebab = !rel.is_empty()
        && rel
            .split('-')
            .all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_alphanumeric()));
    let spaced = word[colon + 1..]
        .chars()
        .next()
        .is_none_or(char::is_whitespace);
    (kebab && spaced).then(|| word.split_at(colon + 1))
}

/// `is_reference_char` returns whether `c` can appear in a reference, that
/// is, in an id, a store, a key, or a fragment.
fn is_reference_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | ':' | '#' | '/' | '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str) -> Vec<MarkerRow> {
        scan(text.as_bytes()).expect("text")
    }

    fn marker(line: usize, rel: &str, target: &str) -> MarkerRow {
        MarkerRow {
            line,
            kind: RowKind::Marker {
                rel: rel.into(),
                target: target.into(),
            },
        }
    }

    // sokf-tests: SOKF_FORMAT_142
    // sokf-tests: SOKF_FORMAT_144
    #[test]
    fn a_marker_is_the_first_word_of_its_line_in_any_comment() {
        let text = "fn a() {}\n// sokf-implements: CLI-1_AC-2\n  # sokf-tests: CLI_P-5\n/* sokf-references: design-cache#merge-sync */\n<!-- sokf-implements: sokf-spec:#sokf_format_96-->\n-- sokf-mitigates: THREAT-3.\n";
        assert_eq!(
            rows(text),
            vec![
                marker(2, "implements", "CLI-1_AC-2"),
                marker(3, "tests", "CLI_P-5"),
                marker(4, "references", "design-cache#merge-sync"),
                marker(5, "implements", "sokf-spec:#sokf_format_96"),
                marker(6, "mitigates", "THREAT-3"),
            ]
        );
    }

    #[test]
    fn prose_strings_and_directives_are_no_markers() {
        let text = "Tag it with `// sokf-implements: CLI-1_AC-2`.\nlet s = \"// sokf-tests: X\";\n// sokf:begin config\n// sokf:links\n[sokf:config]: /a.md\n// sokf-implements is a tag\n    \"sokf-spec:sokf-format-spec#9\",\n/// `sokf-spec:glossary`, `#key`\n";
        assert_eq!(rows(text), vec![]);
    }

    // sokf-tests: SOKF_FORMAT_144
    #[test]
    fn a_word_after_the_reference_or_no_reference_is_malformed() {
        let text = "// sokf-implements: CLI-1_AC-2 and more\n// sokf-tests:\n// sokf-tests: A, B\n";
        assert_eq!(
            rows(text),
            vec![
                MarkerRow {
                    line: 1,
                    kind: RowKind::Malformed("sokf-implements:".into())
                },
                MarkerRow {
                    line: 2,
                    kind: RowKind::Malformed("sokf-tests:".into())
                },
                MarkerRow {
                    line: 3,
                    kind: RowKind::Malformed("sokf-tests:".into())
                },
            ]
        );
    }

    // sokf-tests: SOKF_FORMAT_143
    // sokf-tests: SOKF_VALIDATION_52
    #[test]
    fn a_tag_in_another_case_is_reported() {
        let text = "// SOKF-Implements: CLI-1_AC-2\n// sokf-Tests: X\n// Sokf-implements: X\n";
        let kinds: Vec<RowKind> = rows(text).into_iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            vec![
                RowKind::Case("SOKF-Implements:".into()),
                RowKind::Case("sokf-Tests:".into()),
                RowKind::Case("Sokf-implements:".into()),
            ]
        );
    }

    // sokf-tests: SOKF_FORMAT_151
    #[test]
    fn ignore_directives_keep_marker_text() {
        let text = "// sokf:ignore-begin\nlet a = \"\n    // sokf-implements: CLI-1_AC-2\n\";\n// sokf:ignore-end\n// sokf:ignore-next-line\n// sokf-tests: X\n// sokf-tests: Y\n";
        assert_eq!(rows(text), vec![marker(8, "tests", "Y")]);
    }

    // A directive is read when only whitespace and characters that are
    // neither letters nor digits follow it, such as the end of an HTML or a
    // block comment. A letter or a digit after it, even after a hyphen,
    // makes it no directive.
    // sokf-tests: SOKF_FORMAT_151
    // sokf-tests: SOKF_FORMAT_156
    #[test]
    fn a_comment_end_can_follow_a_directive() {
        let html = "<!-- sokf:ignore-begin -->\n<!-- sokf-implements: CLI-1_AC-2 -->\n<!-- sokf:ignore-end -->\n";
        assert_eq!(rows(html), vec![]);
        let block = "/* sokf:ignore-begin */\n/* sokf-tests: X */\n/* sokf:ignore-end*/\n/* sokf:ignore-next-line -->\n// sokf-tests: Y\n";
        assert_eq!(rows(block), vec![]);
        let named = "<!-- sokf:ignore-begin-x -->\n// sokf-tests: X\n<!-- sokf:ignore-begin 2 -->\n// sokf-tests: Y\n";
        assert_eq!(
            rows(named),
            vec![marker(2, "tests", "X"), marker(4, "tests", "Y")]
        );
    }

    // sokf-tests: SOKF_FORMAT_152
    // sokf-tests: SOKF_FORMAT_153
    #[test]
    fn ignore_directives_out_of_order_are_faults() {
        let text = "// sokf:ignore-end\n// sokf:ignore-begin\n// sokf:ignore-begin\n// sokf:ignore-next-line\n// sokf-tests: X\n";
        let kinds: Vec<(usize, RowKind)> =
            rows(text).into_iter().map(|r| (r.line, r.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                (1, RowKind::Ignore(IgnoreFault::EndOutside)),
                (2, RowKind::Ignore(IgnoreFault::NeverEnds)),
                (3, RowKind::Ignore(IgnoreFault::BeginInside)),
            ]
        );
    }

    // sokf-tests: SOKF_VALIDATION_50
    #[test]
    fn a_binary_file_is_not_read() {
        let mut bytes = b"// sokf-tests: X\n".to_vec();
        bytes.push(0);
        assert_eq!(scan(&bytes), None);
        // A NUL past the first 8 KB does not make a file binary.
        let mut late = vec![b' '; BINARY_PROBE];
        late.extend_from_slice(b"\n// sokf-tests: X\n\0");
        assert_eq!(scan(&late).unwrap().len(), 1);
    }

    #[test]
    fn crlf_lines_and_hits_late_in_a_line_count_once() {
        let text = "a\r\n// sokf-tests: X\r\n// sokf-tests: Y sokf-z\r\n";
        assert_eq!(
            rows(text),
            vec![
                marker(2, "tests", "X"),
                MarkerRow {
                    line: 3,
                    kind: RowKind::Malformed("sokf-tests:".into())
                },
            ]
        );
    }
}
