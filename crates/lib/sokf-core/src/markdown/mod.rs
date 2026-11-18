//! This module reads Markdown in SOKF's dialect. It runs the parser of the
//! Markdown engine, `tramlines_md`, with SOKF's markers and its `sokf:`
//! reference syntax, as sokf-format-spec defines them.

mod dialect;
mod fences;
mod headings;
mod refs;

pub use dialect::{
    SOKF, is_guidance_marker, is_markdown, is_only_marker, is_optional_mark, only_marker, parse,
    segments,
};
pub use fences::fenced_bytes;
pub use headings::{link_headings, suffixed_headings};
pub use refs::{IncludeTarget, label_parts, sokf_label, split_store};
pub use tramlines_md::markdown::*;
