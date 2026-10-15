//! This module holds the grammar: the core grammar of the engine with
//! SOKF's extension, SOKF's formats and reference types, and the
//! compilation of every schema in the model.

mod compile;
mod load;
mod sokf_schema;
#[cfg(test)]
mod tests;
mod uses;

pub use compile::{Compiled, compile_all};
pub use load::{
    SOKF_TYPES, compile, embedded_formats, extension_text, formats_text, grammar_text, load_grammar,
};
pub use sokf_schema::{CitationRule, CitedFrom, Dispatch, LinkRule, SokfSchema, glob_inside};
pub use tramlines_md::grammar::*;
pub use uses::{fingerprint, schema_uses};
