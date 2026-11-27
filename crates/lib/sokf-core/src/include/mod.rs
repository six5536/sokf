//! The `include` module handles generated content (format specification §9).
//! It finds the regions of a file, chooses the length and language tag of the
//! code fence around each file-form include, builds the include graph, and
//! works out the expected content of every generated block.

mod expand;
mod fence;
mod graph;
mod pairs;
mod region;

pub use expand::{
    DefinitionEntry, Expected, Fenced, expected, expected_for, path_link_label, render_definitions,
};
pub use graph::{includes_body, knowledge_of_additional};
pub(crate) use pairs::index_labels_as_written;
pub use pairs::{definition_pair, index_pair, labels_as_written, pairs};
pub use region::{Extraction, RegionError, extract, extract_lines};
