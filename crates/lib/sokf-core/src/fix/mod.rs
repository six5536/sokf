//! The `fix` module repairs files as the validation specification defines:
//! it plans which files to rewrite and move, and then applies those changes
//! on disk.

mod apply;
mod edits;
mod plan;

pub use apply::apply;
pub use edits::{ResourceEdit, definition_edit, include_edit, rewrite};
pub use plan::{FixPlan, Move, Rewrite, plan};
pub use tramlines_md::edit::{AnchorEdit, Edit, Rename};
