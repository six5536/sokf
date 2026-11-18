//! This module scans one repository file for markers (sokf-format-spec §9.5).
//! A case-insensitive search finds the lines that contain `sokf-` or
//! `sokf:`, and the scan parses only those lines.

use std::{ops::Range, sync::OnceLock};

use regex::bytes::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    RepoPath,
    markdown::{self, is_slug},
};

/// `BINARY_PROBE` is the number of bytes at the start of a file that the
/// scan searches for a NUL byte to decide whether the file is binary
/// (`SOKF_VALIDATION_50`).
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
    /// its reference (`SOKF_FORMAT_144`). The value is the tag as written.
    Malformed(String),
    /// A tag whose rel isn't kebab case, such as `sokf-foo_bar:`
    /// (CLI-4_AC-41). The value is the tag as written.
    Rel(String),
    /// An ignore directive that is out of order (`SOKF_FORMAT_152` and
    /// `SOKF_FORMAT_153`).
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

/// `scan` returns the markers of the file at `path` and the faults in the
/// lines that look like markers, in line order. It returns `None` for a
/// binary file, which it doesn't read for markers (`SOKF_VALIDATION_50`).
/// It skips each line whose first word lies inside a fenced code block of
/// a Markdown file, because such a block holds an example. It parses the
/// file as Markdown only when a line holds a marker, or a fault in a line
/// that looks like one, or when the file holds a directive. It then parses
/// the whole file, from its first line, as a body without frontmatter and
/// without the regions of a document's generated blocks, because a file
/// outside the roots is not a document, and a `---` that opens the file is
/// a thematic break. The scan then reads the bytes of the text that the
/// parser read, with each byte that isn't UTF-8 replaced, because the bytes
/// of the fences count from the start of that text.
// sokf-implements: SOKF_FORMAT_250
pub fn scan(path: &RepoPath, bytes: &[u8]) -> Option<Vec<MarkerRow>> {
    let rows = scan_lines(bytes, &[])?;
    let directive = || bytes.windows(DIRECTIVE.len()).any(|w| w == DIRECTIVE);
    if !markdown::is_markdown(path) || (rows.is_empty() && !directive()) {
        return Some(rows);
    }
    let text = String::from_utf8_lossy(bytes);
    let body = markdown::parse(&text, 1, &markdown::Segments::default());
    scan_lines(text.as_bytes(), &markdown::fenced_bytes(&body, &text))
}

/// `DIRECTIVE` begins every ignore directive.
const DIRECTIVE: &[u8] = b"sokf:ignore-";

/// `scan_lines` scans `bytes` for markers, and skips each line whose
/// first word lies in one of the ranges of bytes of `examples`, in the
/// order of the bytes: neither a marker nor a directive begins on that
/// line.
// sokf-implements: SOKF_FORMAT_142
// sokf-implements: SOKF_FORMAT_143
// sokf-implements: SOKF_FORMAT_144
// sokf-implements: SOKF_FORMAT_151
// sokf-implements: SOKF_FORMAT_152
// sokf-implements: SOKF_FORMAT_153
// sokf-implements: SOKF_VALIDATION_50
fn scan_lines(bytes: &[u8], examples: &[Range<usize>]) -> Option<Vec<MarkerRow>> {
    if is_binary(bytes) {
        return None;
    }
    // The offsets of the hits rise, so the scan passes each range once.
    let mut example = examples.iter().peekable();
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
        #[expect(
            clippy::naive_bytecount,
            reason = "each byte is counted once per file, so a crate that counts faster isn't worth adding"
        )]
        let newlines = bytes[counted..start]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        line += newlines;
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
        // `first_word` trims only ASCII, which is one byte in `bytes` too.
        let at = start + text.len() - word.len();
        while example.next_if(|r| r.end <= at).is_some() {}
        if example.peek().is_some_and(|r| r.contains(&at)) {
            continue;
        }
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
/// `#` that opens a comment (`SOKF_FORMAT_142`).
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
/// with a tag. It reads the tag in any case, as the tag in lowercase, so
/// that a linter's fix of a comment's case, such as `Sokf-Implements:`,
/// breaks no marker (`SOKF_FORMAT_143`), and it reads a reference between
/// backticks, which a linter of doc comments adds, as the reference inside
/// them (`SOKF_FORMAT_144`).
// sokf-implements: CLI-4_AC-41
// sokf-implements: SOKF_FORMAT_143
// sokf-implements: SOKF_FORMAT_144
fn parse(word: &str) -> Option<RowKind> {
    let (tag, rest) = tag(word)?;
    let rel = tag["sokf-".len()..tag.len() - 1].to_ascii_lowercase();
    if !is_slug(&rel) {
        return Some(RowKind::Rel(tag.to_string()));
    }
    let rest = rest.trim_start();
    let (rest, ticked) = match rest.strip_prefix('`') {
        Some(inside) => (inside, true),
        None => (rest, false),
    };
    let len = rest
        .find(|c: char| !is_reference_char(c))
        .unwrap_or(rest.len());
    // The scan trims a full stop, a hyphen, a slash, or a colon from the end
    // of a reference, because these characters follow a reference as the end
    // of a sentence or of a comment, such as `-->`, inside the backticks too.
    let target = rest[..len].trim_end_matches(['.', '-', '/', ':']);
    let after = if ticked {
        rest[len..].strip_prefix('`')
    } else {
        Some(&rest[target.len()..])
    };
    if target.is_empty() || after.is_none_or(|a| a.chars().any(char::is_alphanumeric)) {
        return Some(RowKind::Malformed(tag.to_string()));
    }
    Some(RowKind::Marker {
        rel,
        target: target.to_string(),
    })
}

/// `tag` returns the tag that starts `word` and the text after the tag. A
/// tag is `sokf-<rel>:` in any case, followed by whitespace or the end of
/// the line, where the rel is a word of letters, digits, `-`, and `_`,
/// kebab case or not, so that the scan reports a rel that isn't kebab case.
/// A `.` ends the word, because a file name such as `sokf-spec.md:` starts
/// many a line that holds no marker.
/// A qualified reference, such as `sokf-spec:glossary`, has no whitespace
/// after its colon, and so it isn't a tag.
fn tag(word: &str) -> Option<(&str, &str)> {
    let head = word.get(..5)?;
    if !head.eq_ignore_ascii_case("sokf-") {
        return None;
    }
    let colon = word.find(':')?;
    let rel = &word[5..colon];
    let word_like = !rel.is_empty()
        && rel
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
    let spaced = word[colon + 1..]
        .chars()
        .next()
        .is_none_or(char::is_whitespace);
    (word_like && spaced).then(|| word.split_at(colon + 1))
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
        scan_lines(text.as_bytes(), &[]).expect("text")
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

    // sokf-tests: CLI-4_AC-41
    #[test]
    fn a_rel_that_isnt_kebab_case_is_a_fault() {
        let text = "// sokf-foo_bar: CLI-1_AC-2\n# sokf-implements-: X\n// sokf-a--b: X\n// sokf-v1.2: X\n// sokf-implements: CLI-1_AC-2\n";
        assert_eq!(
            rows(text),
            vec![
                MarkerRow {
                    line: 1,
                    kind: RowKind::Rel("sokf-foo_bar:".into())
                },
                MarkerRow {
                    line: 2,
                    kind: RowKind::Rel("sokf-implements-:".into())
                },
                MarkerRow {
                    line: 3,
                    kind: RowKind::Rel("sokf-a--b:".into())
                },
                marker(5, "implements", "CLI-1_AC-2"),
            ]
        );
        // A qualified reference, a file name, and a word with a slash stay
        // no tags, and so does line 4 above.
        assert_eq!(
            rows("sokf-spec:glossary\n- sokf-spec.md: the spec\n// sokf-a/b: X\n"),
            [] as [crate::markers::scan::MarkerRow; 0]
        );
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

    // A tag in another case reads as the tag in lowercase, such as the
    // `Sokf-Implements:` of eslint's `capitalized-comments`.
    // sokf-tests: SOKF_FORMAT_143
    #[test]
    fn a_tag_in_another_case_reads_as_the_tag_in_lowercase() {
        let text = "// SOKF-Implements: CLI-1_AC-2\n// sokf-Tests: X\n// Sokf-implements: X\n// Sokf-Foo_Bar: X\n";
        assert_eq!(
            rows(text),
            vec![
                marker(1, "implements", "CLI-1_AC-2"),
                marker(2, "tests", "X"),
                marker(3, "implements", "X"),
                MarkerRow {
                    line: 4,
                    kind: RowKind::Rel("Sokf-Foo_Bar:".into())
                },
            ]
        );
    }

    // A reference between backticks, as clippy's `doc_markdown` writes it
    // in a doc comment, reads as the reference inside them. A backtick that
    // nothing closes, or a word after the closing one, is malformed.
    // sokf-tests: SOKF_FORMAT_144
    #[test]
    fn a_reference_between_backticks_reads_as_the_reference() {
        let text = "/// sokf-implements: `CLI-1_AC-2.`\n// sokf-tests: `req-cli#CLI-1_AC-1`.\n<!-- sokf-references: `design-cache` -->\n// sokf-tests: `X\n// sokf-tests: `X` and Y\n// sokf-tests: ``\n";
        assert_eq!(
            rows(text),
            vec![
                marker(1, "implements", "CLI-1_AC-2"),
                marker(2, "tests", "req-cli#CLI-1_AC-1"),
                marker(3, "references", "design-cache"),
                MarkerRow {
                    line: 4,
                    kind: RowKind::Malformed("sokf-tests:".into())
                },
                MarkerRow {
                    line: 5,
                    kind: RowKind::Malformed("sokf-tests:".into())
                },
                MarkerRow {
                    line: 6,
                    kind: RowKind::Malformed("sokf-tests:".into())
                },
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
        assert_eq!(scan_lines(&bytes, &[]), None);
        // A NUL past the first 8 KB does not make a file binary.
        let mut late = vec![b' '; BINARY_PROBE];
        late.extend_from_slice(b"\n// sokf-tests: X\n\0");
        assert_eq!(scan_lines(&late, &[]).unwrap().len(), 1);
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

    // A byte that isn't UTF-8 before a fence moves no fence off its lines,
    // because the scan reads the text that the parser read.
    // sokf-tests: SOKF_FORMAT_250
    #[test]
    fn a_byte_that_isnt_utf8_moves_no_fence() {
        let mut bytes = b"# A ".to_vec();
        bytes.extend([0xFF; 40]);
        bytes.extend_from_slice(b"\n\n```\n// sokf-tests: X\n```\n\n// sokf-tests: Y\n");
        let path = RepoPath::parse("/a.md").expect("a path");
        assert_eq!(scan(&path, &bytes), Some(vec![marker(7, "tests", "Y")]));
    }
}
