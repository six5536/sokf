//! This module holds SOKF's dialect of Markdown, and runs the parser and
//! the marker checks of the Markdown engine with it.

use std::collections::BTreeMap;

use tramlines_md::markdown::{Body, Dialect, Segments};

/// `SOKF` is the dialect of SOKF's markers: `sokf:only`, `sokf:guidance`, and
/// `sokf:optional`, the include block, the definition block `sokf:links`, and
/// `sokf:` labels.
// sokf-implements: SOKF_FORMAT_139
// sokf-implements: SOKF_FORMAT_154
pub const SOKF: Dialect = Dialect {
    namespace: "sokf",
    region: Some("include"),
    trailer: Some("links"),
    scheme: Some("sokf"),
};

/// `segments` returns the markers of `body`, whose first line is line
/// `first_line` of the file.
pub fn segments(body: &str, first_line: usize) -> Segments {
    tramlines_md::markdown::segments(body, first_line, &SOKF)
}

/// `parse` returns the parsed `body`, whose first line is line `first_line` of
/// the file.
pub fn parse(body: &str, first_line: usize, segments: &Segments) -> Body {
    tramlines_md::markdown::parse(body, first_line, segments, &SOKF)
}

/// `is_only_marker` reports whether `line` is a `sokf:only` marker.
pub fn is_only_marker(line: &str) -> bool {
    SOKF.is_only_marker(line)
}

/// `is_guidance_marker` reports whether `line` is a `sokf:guidance` marker.
pub fn is_guidance_marker(line: &str) -> bool {
    SOKF.is_guidance_marker(line)
}

/// `is_optional_mark` reports whether `line` is either end of a `sokf:optional`
/// region.
pub fn is_optional_mark(line: &str) -> bool {
    SOKF.is_optional_mark(line)
}

/// `only_marker` returns the marker's map from each dimension to its states
/// when `line` is a `sokf:only` marker.
pub fn only_marker(line: &str) -> Option<BTreeMap<String, Vec<String>>> {
    SOKF.only_marker(line)
}
