//! The `report` module holds the findings, the fix operations, and the report of
//! `validate` (CLI-4).

mod collect;
mod fix_op;
mod text;

pub use collect::{Report, ValidateReport};
pub use fix_op::FixOp;
pub use text::{message, text};
pub use tramlines_md::{Finding, Locator, Severity};
