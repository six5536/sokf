//! The incremental cache (contract-cache) keeps what a run derives from the
//! knowledge in one SQLite store, and its sync finds the files that changed.

mod conn;
mod edges;
mod hint;
mod markers;
mod outputs;
mod plans;
mod rows;
mod store;
mod sync;
mod view;

pub use outputs::{AdditionalRow, AssetRow, CitationRow, CitedFact, SchemaRow};
pub use rows::DocRow;
pub use store::{CACHE_DIR, Cache, CacheMode, STORE_FILE, STORE_FORMAT};
pub use view::StoreView;

pub use sync::{FileState, SyncStats};
