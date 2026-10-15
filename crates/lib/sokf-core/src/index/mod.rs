//! The `index` module builds the generated index of each directory of a
//! knowledge (format specification §9.4). It computes the expected text of
//! each index from the model and the moves that a fix plans, and renders that
//! text.

mod expected;
mod render;

pub use expected::{Indexes, expected, expected_for, touched};
pub use render::{IndexEntry, render};
