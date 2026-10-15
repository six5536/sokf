//! sokf core: the library half of the sokf command-line interface (CLI).
//!
//! The binary (`crates/app/sokf`) parses the arguments and renders the output.
//! The logic that the binary drives lives here, so that tests can run that
//! logic without starting a process.
//!
//! The core loads and validates SOKF knowledge stores, their documents, and
//! the assets those documents describe, as the core specifications in
//! `sokf-spec/` define them. It also runs the `sokf` command that
//! `knowledge/contracts/` specifies. Its entry points are [`init`],
//! [`harness_install`], [`harness_status`], [`validate`], [`info_formats`],
//! [`info_grammar`], [`new`], [`graph`](fn@graph), and [`locate`]. The harness
//! hook is [`harness::hook`].
//!
//! SOKF uses the schema language of the `tramlines-md` engine. This crate
//! re-exports the engine's modules under their old `sokf-core` paths. SOKF
//! adds its own schema extension, dialect, and model.
//!
//! # Example
//!
//! ```
//! use sokf_core::{RepoPath, markdown};
//!
//! let body = "# Title\n\nA paragraph.\n";
//! let segments = markdown::segments(body, 1);
//! let parsed = markdown::parse(body, 1, &segments);
//! assert_eq!(parsed.headings[0].slug, "title");
//! assert_eq!(RepoPath::parse("/knowledge/a.md").unwrap().relative(), "knowledge/a.md");
//! ```

#![warn(missing_docs)]

mod api;
mod cache;
pub mod check;
mod config;
mod error;
pub mod fix;
pub mod grammar;
pub mod graph;
pub mod harness;
pub mod include;
pub mod index;
pub mod markdown;
pub mod markers;
pub mod model;
mod output_schema;
#[cfg(test)]
mod proptest_cases;
mod query;
mod report;
pub mod scaffold;
mod source;
#[cfg(test)]
mod test_support;
mod walk;

pub use agent_harness_adapter_core::Scope as HarnessScope;
pub use api::{
    ALL_HARNESSES, GraphOptions, HarnessDirs, HarnessOptions, HarnessReport, HarnessResult,
    InfoLayer, InitOptions, InitResult, Loaded, NewOptions, NewResult, PartResult, ValidateOptions,
    check_repository, graph, harness_ids, harness_install, harness_status, info_formats,
    info_grammar, init, load_for_lookup, load_keys_for, load_model, load_repository,
    load_with_keys, load_working, locate, new, validate,
};

pub use cache::{
    CACHE_DIR, Cache, CacheMode, FileState, STORE_FILE, STORE_FORMAT, StoreView, SyncStats,
};
pub use config::{
    Config, ConfigText, MarkerCheck, PathArg, Scope, discover_root, load_config, load_repo_formats,
    resolve_paths,
};
pub use error::{Error, Result};
pub use grammar::{
    Compiled, FormatRegistry, FormatSource, Grammar, compile_all, formats_text, grammar_text,
    load_grammar,
};
pub use graph::{
    Edge, EdgeKind, Entry, Graph, GraphJson, GraphQuery, GraphResult, LocateReport, Location, Node,
    NodeKey, NodeKind, Row, Start,
};
pub use include::{Expected, expected};
pub use index::{IndexEntry, Indexes};
pub use markdown::{Body, Segments};
pub use model::{Document, FsLoader, Knowledge, Loader, Located, Model, build};
pub use output_schema::{Output, output_schemas};
pub use query::Expr;
pub use report::{
    Finding, FixOp, Locator, Report, Severity, ValidateReport, message as report_message,
    text as report_text,
};
pub use scaffold::IdTemplate;
pub use source::{FileKey, LineEnding, Source, Stat, load};
pub use tramlines_md::{RepoPath, RepoRoot, Spanned, Yaml, YamlError, yaml};
pub use walk::{FileKind, Tree, WalkedDir, WalkedFile, collect};
