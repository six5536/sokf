//! The `graph` module represents the knowledge as a graph of nodes and
//! edges, which it builds once from the model (CLI-7). `build` builds the
//! graph, `start` resolves the start of a command (a key, an id, or a path)
//! to a node, `traverse` walks the graph from the start nodes to a bounded
//! depth, `json` writes the graph as JSON, and `locate` turns ids into paths
//! (CLI-8).

mod build;
mod json;
mod locate;
mod model;
mod near;
mod start;
mod traverse;

pub use build::{
    RawEdge, Target, build, build_from, edges_of, frontmatter_json, inverse_rel, resolve_to,
};
pub use json::{GraphJson, bounded, edge_json, node_json};
pub use locate::{LocateReport, Location, locate};
pub use model::{Edge, EdgeKind, Graph, Hydrator, Node, NodeKey, NodeKind};
pub(crate) use near::{EdgeStore, edges_near};
pub use start::{Start, is_key_reference, key_entry, resolve_id, resolve_start};
pub(crate) use traverse::starts;
pub use traverse::{Count, Entry, GraphQuery, GraphResult, LIMIT, Row, Summary, run};
