//! The `report` module holds the findings, the fix operations, and the report of
//! `validate` (CLI-4).

mod collect;
mod fix_op;
mod fixable;
mod text;

pub use collect::{Report, ValidateReport};
pub use fix_op::FixOp;
pub(crate) use text::{ABSENT_PART, LEFTOVER_PACKS, STALE_PART, UNVOUCHED_PART, and_list};
pub use text::{fixable_line, message, text};
pub use tramlines_md::{Finding, Locator, Severity};
