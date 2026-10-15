//! This module defines the JSON form of `sokf graph` (CLI-7_AC-12) as wire types, and the
//! `output_schema` module generates the published schema from them. The predicates of `-l`, `--edge`, and `--to`
//! (CLI-7_AC-43) read the same objects, so a predicate reads exactly what the tool prints.

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde_json::{Map, Value};

use super::{Edge, EdgeKind, Graph, GraphResult, Node, NodeKind};

/// The graph of `sokf graph --json`: the starts (the document ids, paths, and keys given on
/// the command line), then every node and edge that the text form prints, and every inverse
/// of each printed edge (CLI_P-8).
#[derive(Serialize, JsonSchema)]
#[schemars(title = "sokf graph")]
pub struct GraphJson {
    /// Each resolved start as a path, in the order given: a key is the path of its document,
    /// then `#` and the slug of the key. When no start is given, the list holds the knowledge
    /// roots.
    start: Vec<String>,
    /// The nodes, ordered by path. The missing nodes follow, ordered by target.
    nodes: Vec<NodeJson>,
    /// The edges, ordered by `from`, then kind, then `to`, then `rel`, then `region`, then
    /// `note`.
    edges: Vec<EdgeJson>,
    /// The counts that the last line of the text form shows: what the graph printed and
    /// what it left out.
    summary: SummaryJson,
}

/// The summary counts what the graph printed and what the same view prints with `--all`,
/// `--archived`, and no filter, as the last line of the text form does (CLI-7_AC-78).
// sokf-implements: CLI-7_AC-78
#[derive(Serialize, JsonSchema)]
#[schemars(rename = "Summary")]
struct SummaryJson {
    /// The document nodes.
    documents: CountJson,
    /// The archived documents.
    archived: CountJson,
}

/// What the graph printed, out of what the whole view holds.
#[derive(Serialize, JsonSchema)]
#[schemars(rename = "Count")]
struct CountJson {
    /// What the graph printed.
    shown: usize,
    /// What the view prints with `--all`, `--archived`, and no filter.
    total: usize,
}

impl From<super::Count> for CountJson {
    fn from(c: super::Count) -> Self {
        CountJson {
            shown: c.shown,
            total: c.total,
        }
    }
}

/// One node. Its parent is the parent of its path.
// sokf-implements: CLI-7_AC-62
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[schemars(rename = "Node")]
enum NodeJson {
    /// A knowledge root.
    Root {
        /// The path of the root, starting with `/` at the repository root.
        path: String,
        /// The `name` of the manifest. It is absent when the manifest is absent or does
        /// not parse.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        name: Option<String>,
        /// The `scope` of the manifest: the owner of the store. The identity of the store is
        /// `SCOPE/NAME`, where `SCOPE` is the scope and `NAME` is the name. Absent when the
        /// manifest has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        scope: Option<String>,
        /// The `description` of the manifest, when it has one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        description: Option<String>,
    },
    /// A directory in a root, with a document, an additional file, or an asset somewhere
    /// beneath it. A directory that a start points to is a directory node too.
    Directory {
        /// The directory.
        path: String,
    },
    /// A document, including a schema.
    Document {
        /// The document.
        path: String,
        /// The frontmatter as read, without `links` and with its keys in the order written,
        /// followed by `status`, and by `broken` and `reason` when the document is broken.
        #[serde(flatten)]
        #[schemars(schema_with = "document_fields")]
        fields: Map<String, Value>,
    },
    /// An additional file.
    Additional {
        /// The file.
        path: String,
        /// The `id` of the schema that governs the file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        schema: Option<String>,
    },
    /// An asset.
    Asset {
        /// The asset.
        path: String,
        /// The `id` of the sidecar of the asset.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        sidecar: Option<String>,
    },
    /// A repository file or directory as the target of an edge. A repository file that a
    /// start points to is a file node too.
    File {
        /// The file or the directory.
        path: String,
    },
    /// A key that a start identifies.
    Key {
        /// The document that defines the key, then `#` and the slug of the key.
        path: String,
        /// The key as it is written in its document.
        key: String,
        /// The line of the document where the key is defined.
        line: usize,
    },
    /// A target that resolves to nothing.
    Missing {
        /// The text that named the target.
        target: String,
    },
}

/// One edge.
#[derive(Serialize, JsonSchema)]
#[schemars(rename = "Edge")]
struct EdgeJson {
    /// The kind of the edge.
    kind: EdgeKindJson,
    /// The path of the source node, or the `target` of a missing node.
    from: String,
    /// The path of the target node, or the `target` of a missing node.
    to: String,
    /// The relationship, from the source to the target.
    rel: String,
    /// The `note` of a `links` entry. An inverse edge, which the tool makes from a declared
    /// one, never has a note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    note: Option<String>,
    /// The region of an include, when the marker names one, or the slug of the fragment that
    /// a code marker cites. It is absent from a folded include or marker edge, the one edge
    /// that replaces all the includes or markers of one file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    region: Option<String>,
    /// The line of a code marker in its file. It is absent from a folded marker edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    line: Option<usize>,
    /// The number of regions that a folded include replaces. Absent unless the include is
    /// folded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    regions: Option<usize>,
    /// The number of markers that a folded marker edge replaces. Absent unless the marker edge
    /// is folded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    markers: Option<usize>,
    /// True when the tool made the edge as the inverse of a declared one. It is absent otherwise.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(extend("const" = true))]
    synthesised: bool,
}

/// The kinds of edge.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "EdgeKind")]
enum EdgeKindJson {
    /// An include marker.
    Includes,
    /// A key and its document, or a document that cites the key.
    Keys,
    /// A `links` entry, or a body link that no entry declares.
    Links,
    /// A code marker in a file outside the knowledge roots, and the document or the key that
    /// it cites.
    Marker,
    /// A `resource` that is a `/` path.
    Resource,
    /// A `sources[].resource` that is a `/` path.
    Sources,
}

// `document_fields` describes the keys that a document node always or often carries, and any
// other frontmatter key can follow them. The tool prints the frontmatter as read. The graph
// is permissive: it takes a document whatever values its frontmatter holds. So a key of the
// frontmatter can hold any value. The schema constrains only the keys that the tool writes.
fn document_fields(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "properties": {
            "type": { "description": "The `type` of the frontmatter, as read." },
            "id": { "description": "The `id` of the frontmatter, as read. It is absent when the document has none." },
            "title": { "description": "The `title` of the frontmatter, as read." },
            "status": { "description": "The `status` of the frontmatter, as read. It is `stable` when the frontmatter has none." },
            "broken": { "description": "True when the frontmatter does not parse or has no `id`.", "const": true },
            "reason": {
                "description": "Why the document is broken.",
                "enum": ["frontmatter does not parse", "no id"]
            }
        },
        "required": ["status"]
    })
}

fn text(fields: &Map<String, Value>, key: &str) -> Option<String> {
    fields.get(key).and_then(Value::as_str).map(str::to_string)
}

impl From<&Node> for NodeJson {
    fn from(node: &Node) -> Self {
        let path = || node.key.as_str().to_string();
        match node.kind {
            NodeKind::Root => NodeJson::Root {
                path: path(),
                name: text(&node.fields, "name"),
                scope: text(&node.fields, "scope"),
                description: text(&node.fields, "description"),
            },
            NodeKind::Directory => NodeJson::Directory { path: path() },
            NodeKind::Document => NodeJson::Document {
                path: path(),
                fields: node.fields.clone(),
            },
            NodeKind::Additional => NodeJson::Additional {
                path: path(),
                schema: text(&node.fields, "schema"),
            },
            NodeKind::Asset => NodeJson::Asset {
                path: path(),
                sidecar: text(&node.fields, "sidecar"),
            },
            NodeKind::File => NodeJson::File { path: path() },
            NodeKind::Key => NodeJson::Key {
                path: path(),
                key: text(&node.fields, "key").unwrap_or_default(),
                line: node
                    .fields
                    .get("line")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
            },
            NodeKind::Missing => NodeJson::Missing {
                target: node.key.as_str().to_string(),
            },
        }
    }
}

impl From<&Edge> for EdgeJson {
    fn from(edge: &Edge) -> Self {
        EdgeJson {
            kind: match edge.kind {
                EdgeKind::Includes => EdgeKindJson::Includes,
                EdgeKind::Keys => EdgeKindJson::Keys,
                EdgeKind::Links => EdgeKindJson::Links,
                EdgeKind::Marker => EdgeKindJson::Marker,
                EdgeKind::Resource => EdgeKindJson::Resource,
                EdgeKind::Sources => EdgeKindJson::Sources,
            },
            from: edge.from.as_str().to_string(),
            to: edge.to.as_str().to_string(),
            rel: edge.rel.clone(),
            note: edge.note.clone(),
            region: edge.region.clone(),
            line: edge.line,
            regions: None,
            markers: None,
            synthesised: edge.synthesised,
        }
    }
}

fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).expect("a wire type could not be serialised to JSON")
}

/// `node_json` returns the node object: `kind`, then `path` or `target`, then the fields.
// sokf-implements: CLI-7_AC-49
pub fn node_json(node: &Node) -> Value {
    value(NodeJson::from(node))
}

/// `edge_json` returns the edge object: `kind`, `from`, `to`, and `rel`, then `note`,
/// `region`, `line`, `regions`, or `markers`, then `synthesised` when it is true.
// sokf-implements: CLI-7_AC-50
pub fn edge_json(edge: &Edge) -> Value {
    value(EdgeJson::from(edge))
}

/// `bounded` returns the `{start, nodes, edges, summary}` object of a result, in which a
/// folded include carries the number of its regions and no region, and a folded marker edge
/// carries the number of its markers, and no region and no line.
// sokf-implements: CLI-7_AC-12
// sokf-implements: CLI-7_AC-13
// sokf-implements: CLI-7_AC-80
// sokf-implements: CLI-7_AC-96
pub fn bounded(graph: &Graph, result: &GraphResult) -> Value {
    value(GraphJson {
        start: result
            .start
            .iter()
            .map(|k| k.as_str().to_string())
            .collect(),
        nodes: result
            .nodes
            .iter()
            .filter_map(|k| graph.node(k))
            .map(NodeJson::from)
            .collect(),
        edges: result
            .edges
            .iter()
            .map(|&i| {
                let mut edge = EdgeJson::from(&graph.edges[i]);
                if let Some(&n) = result.folded.get(&i) {
                    edge.region = None;
                    if graph.edges[i].kind == EdgeKind::Marker {
                        edge.line = None;
                        edge.markers = Some(n);
                    } else {
                        edge.regions = Some(n);
                    }
                }
                edge
            })
            .collect(),
        summary: SummaryJson {
            documents: result.summary.documents.into(),
            archived: result.summary.archived.into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        RepoPath,
        graph::{EdgeKind, NodeKey, NodeKind},
    };

    fn key(s: &str) -> NodeKey {
        NodeKey::Path(RepoPath::parse(s).unwrap())
    }

    // sokf-tests: CLI-7_AC-49
    // sokf-tests: CLI-7_AC-50
    #[test]
    fn node_and_edge_objects_have_the_key_order_and_omit_absent_keys() {
        let mut fields = Map::new();
        fields.insert("type".into(), json!("Subsystem"));
        fields.insert("id".into(), json!("planner"));
        fields.insert("status".into(), json!("stable"));
        let doc = Node {
            key: key("/knowledge/core/planner.md"),
            kind: NodeKind::Document,
            fields,
        };
        assert_eq!(
            serde_json::to_string(&node_json(&doc)).unwrap(),
            r#"{"kind":"document","path":"/knowledge/core/planner.md","type":"Subsystem","id":"planner","status":"stable"}"#
        );
        let missing = Node {
            key: NodeKey::Missing("planer".into()),
            kind: NodeKind::Missing,
            fields: Map::new(),
        };
        assert_eq!(
            serde_json::to_string(&node_json(&missing)).unwrap(),
            r#"{"kind":"missing","target":"planer"}"#
        );
        let declared = Edge {
            from: key("/a.md"),
            kind: EdgeKind::Links,
            to: key("/b.md"),
            rel: "depends-on".into(),
            region: None,
            line: None,
            synthesised: false,
            note: Some("n".into()),
        };
        assert_eq!(
            serde_json::to_string(&edge_json(&declared)).unwrap(),
            r#"{"kind":"links","from":"/a.md","to":"/b.md","rel":"depends-on","note":"n"}"#
        );
        let synthesised = Edge {
            from: key("/src/main.rs"),
            kind: EdgeKind::Includes,
            to: key("/a.md"),
            rel: "included-by".into(),
            region: Some("cli".into()),
            line: None,
            synthesised: true,
            note: None,
        };
        assert_eq!(
            serde_json::to_string(&edge_json(&synthesised)).unwrap(),
            r#"{"kind":"includes","from":"/src/main.rs","to":"/a.md","rel":"included-by","region":"cli","synthesised":true}"#
        );
    }

    // sokf-tests: CLI-7_AC-78
    // sokf-tests: CLI-7_AC-80
    #[test]
    fn bounded_has_start_nodes_edges_and_summary() {
        let mut nodes = std::collections::BTreeMap::new();
        for k in ["/a.md", "/b.md"] {
            nodes.insert(
                key(k),
                Node {
                    key: key(k),
                    kind: NodeKind::Document,
                    fields: Map::new(),
                },
            );
        }
        let graph = Graph::new(
            nodes,
            vec![Edge {
                from: key("/a.md"),
                kind: EdgeKind::Links,
                to: key("/b.md"),
                rel: "relates-to".into(),
                region: None,
                line: None,
                synthesised: false,
                note: None,
            }],
        );
        let result = GraphResult {
            start: vec![key("/a.md")],
            rows: vec![],
            nodes: vec![key("/a.md"), key("/b.md"), key("/gone.md")],
            edges: vec![0],
            folded: Default::default(),
            summary: crate::graph::Summary {
                documents: crate::graph::Count { shown: 2, total: 5 },
                archived: crate::graph::Count { shown: 0, total: 3 },
            },
            projection: None,
        };
        assert_eq!(
            bounded(&graph, &result),
            json!({
                "start": ["/a.md"],
                "nodes": [{"kind": "document", "path": "/a.md"}, {"kind": "document", "path": "/b.md"}],
                "edges": [{"kind": "links", "from": "/a.md", "to": "/b.md", "rel": "relates-to"}],
                "summary": {
                    "documents": {"shown": 2, "total": 5},
                    "archived": {"shown": 0, "total": 3}
                }
            })
        );
        // A folded include carries the count of its regions and no region.
        let folded = GraphResult {
            folded: [(0, 3)].into_iter().collect(),
            ..result
        };
        assert_eq!(
            bounded(&graph, &folded)["edges"][0],
            json!({"kind": "links", "from": "/a.md", "to": "/b.md", "rel": "relates-to", "regions": 3})
        );
    }
}
