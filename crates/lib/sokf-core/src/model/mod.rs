//! This module holds the model of the knowledge in memory: the manifests,
//! schemas, documents, indexes, and additional files of each store. It keys
//! them by id, path, and type.

pub mod archive;
mod build;
mod document;
mod fallback;
pub(crate) mod frontmatter;
pub mod keys;
mod knowledge;
mod loader;
pub(crate) mod manifest;
mod plans;
mod reads;
pub mod stores;

pub use crate::markdown::split_store;
pub use archive::Archived;
pub use build::{build, build_with};
pub use document::{
    AdditionalFile, Asset, Document, IndexFile, ParseMode, Parsed, SchemaDocument, SchemaRef,
    parse_source, sidecar_path,
};
pub use fallback::{Fallback, Fields, SchemaMeta};
pub use frontmatter::{
    CoreFields, LinkEntry, Resource, SourceEntry, Status, VerifiedEntry, core_fields,
};
pub use keys::{KeyEntry, KeyIndex};
pub use knowledge::{Knowledge, Located, Model};
pub use loader::{FsLoader, Loader};
pub use manifest::{Manifest, manifest};
pub use plans::PlanIndex;
pub use reads::{Query, Reads, Recorder, SCHEMA_PREFIX};
pub use stores::{Identity, IdentityRow, StoreHit};
