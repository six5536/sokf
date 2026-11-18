//! This module formats the binary's output: the text report of `validate`,
//! the text, template, and Mermaid forms of a `graph` result, and the text
//! output of `guide`.

#[cfg(test)]
mod fixtures;
mod guide;
pub mod mermaid;
mod report;
mod template;
pub mod tree;

pub use guide::text as guide;
pub use report::text;
pub use template::Template;
