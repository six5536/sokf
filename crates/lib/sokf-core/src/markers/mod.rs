//! Markers (sokf-format-spec §9.5). A marker is a comment line in a
//! repository file outside every knowledge store. It cites a key or a
//! document. This module scans a file for markers and resolves what each
//! marker cites.

mod resolve;
mod scan;

pub use resolve::{Reference, Unresolvable, resolve};
pub use scan::{BINARY_PROBE, IgnoreFault, MarkerRow, RowKind, is_binary, scan};
