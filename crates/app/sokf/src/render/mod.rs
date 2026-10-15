//! This module formats the binary's output: the text report of `validate`,
//! and the text, template, and Mermaid forms of a `graph` result.

#[cfg(test)]
mod fixtures;
pub mod mermaid;
mod report;
mod template;
pub mod tree;

pub use report::text;
pub use template::Template;
