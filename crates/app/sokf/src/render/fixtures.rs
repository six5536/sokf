//! This module holds the fixtures that the unit tests of the renderers share:
//! the keys, nodes, and edges of a graph.

use serde_json::{Map, Value};
use sokf_core::{Edge, EdgeKind, Node, NodeKey, NodeKind, RepoPath};

/// `key` returns the key of a path, or of a missing node when `s` isn't a
/// path.
pub(super) fn key(s: &str) -> NodeKey {
    match RepoPath::parse(s) {
        Some(p) => NodeKey::Path(p),
        None => NodeKey::Missing(s.to_string()),
    }
}

/// `node` returns the node at `s`, with the fields of the object `fields`.
pub(super) fn node(s: &str, kind: NodeKind, fields: &Value) -> (NodeKey, Node) {
    let fields: Map<String, Value> = fields.as_object().cloned().unwrap_or_default();
    (
        key(s),
        Node {
            key: key(s),
            kind,
            fields,
        },
    )
}

/// `edge` returns an edge from `from` to `to`.
pub(super) fn edge(
    from: &str,
    kind: EdgeKind,
    to: &str,
    rel: &str,
    region: Option<&str>,
    synthesised: bool,
) -> Edge {
    Edge {
        from: key(from),
        kind,
        to: key(to),
        rel: rel.into(),
        region: region.map(str::to_string),
        line: None,
        synthesised,
        note: None,
    }
}
