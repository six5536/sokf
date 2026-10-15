//! This module defines the types that model the graph of `sokf graph`
//! (CLI-7): `NodeKey`, `Node`, `Edge`, and `Graph`.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
};

use serde_json::{Map, Value};

use crate::RepoPath;

/// `Fields` holds the fields of a document node.
pub type Fields = Map<String, Value>;

/// `Hydrate` is a function that reads the fields of a document node from
/// the node's file.
pub type Hydrate = dyn Fn(&RepoPath) -> Option<Fields>;

/// `Hydrator` reads the fields of a document node from its file when the
/// graph first needs them. A graph built from the store holds no fields for
/// a node until the node is visited (CACHE-1_AC-22).
#[derive(Clone)]
pub struct Hydrator(pub Rc<Hydrate>);

impl std::fmt::Debug for Hydrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Hydrator")
    }
}

/// `NodeKey` identifies a node in one of three ways: by a path, by a key
/// written as its document's path and its slug, or by the target text of a
/// link or entry whose target is missing. Paths sort before keys, and keys
/// sort before missing targets, which is the node order of CLI-7_AC-13.
// sokf-implements: CLI-7_AC-13
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKey {
    /// A node at a path.
    Path(RepoPath),
    /// A key, as `<path>#<slug>`.
    Key(String),
    /// An unresolved target. The variant holds the target text through which
    /// a link or an entry refers to it.
    Missing(String),
}

impl NodeKey {
    /// `as_str` returns the path, the key, or the target text.
    pub fn as_str(&self) -> &str {
        match self {
            NodeKey::Path(p) => p.as_str(),
            NodeKey::Key(t) | NodeKey::Missing(t) => t,
        }
    }

    /// `path` returns the node's path, or `None` for a key or a missing
    /// target.
    pub fn path(&self) -> Option<&RepoPath> {
        match self {
            NodeKey::Path(p) => Some(p),
            NodeKey::Key(_) | NodeKey::Missing(_) => None,
        }
    }
}

/// `NodeKind` lists the kinds of node in `sokf graph`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A knowledge root.
    Root,
    /// A directory in a root, or a directory given as a `START` of
    /// `sokf graph`.
    Directory,
    /// A document or a schema.
    Document,
    /// An additional file.
    Additional,
    /// An asset (CLI-7_AC-25).
    Asset,
    /// A repository file that an edge points at, or a file given as a
    /// `START` of `sokf graph`.
    File,
    /// A key given as a `START` of `sokf graph` (CLI-7_AC-75).
    Key,
    /// An unresolved target.
    Missing,
}

impl NodeKind {
    /// `shows_via` says whether the row of a node of this kind shows the
    /// `via` edges: the edges to the documents that describe or include the
    /// node's file or one of the directories above it.
    pub(super) fn shows_via(self) -> bool {
        matches!(
            self,
            NodeKind::File | NodeKind::Additional | NodeKind::Asset | NodeKind::Directory
        )
    }

    /// `name` returns the node's `kind` value in the JSON output.
    pub fn name(self) -> &'static str {
        match self {
            NodeKind::Root => "root",
            NodeKind::Directory => "directory",
            NodeKind::Document => "document",
            NodeKind::Additional => "additional",
            NodeKind::Asset => "asset",
            NodeKind::File => "file",
            NodeKind::Key => "key",
            NodeKind::Missing => "missing",
        }
    }
}

/// `Node` is one node of the graph: its key, its kind, and the fields that
/// it carries, ready for the JSON output.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The key that identifies the node.
    pub key: NodeKey,
    /// The node's kind.
    pub kind: NodeKind,
    /// The fields other than `kind` and the key, in output order.
    pub fields: Map<String, Value>,
}

/// `EdgeKind` lists the kinds of edge in `sokf graph`, in alphabetical
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    /// An include marker.
    Includes,
    /// An edge from a key to its document, or a body link to the key
    /// (CLI-7_AC-75).
    Keys,
    /// A `links` entry or a body link.
    Links,
    /// A marker in a file outside the knowledge roots (CLI-7_AC-83).
    Marker,
    /// A `resource` path.
    Resource,
    /// A `sources[].resource` path.
    Sources,
}

impl EdgeKind {
    /// `name` returns the edge's `kind` value in the JSON output.
    pub fn name(self) -> &'static str {
        match self {
            EdgeKind::Includes => "includes",
            EdgeKind::Keys => "keys",
            EdgeKind::Links => "links",
            EdgeKind::Marker => "marker",
            EdgeKind::Resource => "resource",
            EdgeKind::Sources => "sources",
        }
    }
}

/// `Edge` is one edge of the graph. All of its fields together identify it,
/// and the order of the fields makes the derived `Ord` sort edges in the
/// order of CLI-7_AC-14.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    /// The source node.
    pub from: NodeKey,
    /// The edge's kind.
    pub kind: EdgeKind,
    /// The target node.
    pub to: NodeKey,
    /// The relationship.
    pub rel: String,
    /// The include region, when the marker specifies one, or the slug of
    /// the fragment that a code marker cites.
    pub region: Option<String>,
    /// The line of a code marker in its file.
    pub line: Option<usize>,
    /// Whether the edge is a synthesised inverse.
    pub synthesised: bool,
    /// The `note` of a `links` entry.
    pub note: Option<String>,
}

/// `inverse` returns the inverse `rel` of an edge. For a `keys` edge, it
/// uses the pairs of `rel` values that key edges carry, such as
/// `defined-in` and `defines`. It does so because key edges come from the
/// graph and never from a `links` entry. For every other edge, it uses the inverse
/// that the SOKF format defines for the `rel`.
fn inverse(kind: EdgeKind, rel: &str) -> &'static str {
    match (kind, rel) {
        (EdgeKind::Keys, "defined-in") => "defines",
        (EdgeKind::Keys, "defines") => "defined-in",
        (EdgeKind::Keys, "cites") => "cited-by",
        (EdgeKind::Keys, _) => "cites",
        _ => super::inverse_rel(rel),
    }
}

/// `Graph` holds every node and every edge, with the edges indexed by
/// their endpoints.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    /// The nodes, by key.
    pub nodes: BTreeMap<NodeKey, Node>,
    /// The edges, sorted, without duplicates.
    pub edges: Vec<Edge>,
    out: HashMap<NodeKey, Vec<usize>>,
    inbound: HashMap<NodeKey, Vec<usize>>,
    /// The hydrator that reads a document node's fields on the first visit,
    /// or `None` when every node was built with its fields.
    hydrator: Option<Hydrator>,
    /// The document nodes whose fields were read.
    hydrated: HashSet<NodeKey>,
    /// The archived documents (CLI-7_AC-67).
    archived: HashSet<NodeKey>,
}

impl Graph {
    /// `new` builds a graph from nodes and edges, and sorts the edges and
    /// removes duplicates.
    // sokf-implements: SOKF_FORMAT_114
    // sokf-implements: CLI-7_AC-14
    pub fn new(nodes: BTreeMap<NodeKey, Node>, mut edges: Vec<Edge>) -> Graph {
        edges.sort();
        edges.dedup();
        let mut out: HashMap<NodeKey, Vec<usize>> = HashMap::new();
        let mut inbound: HashMap<NodeKey, Vec<usize>> = HashMap::new();
        for (i, e) in edges.iter().enumerate() {
            out.entry(e.from.clone()).or_default().push(i);
            inbound.entry(e.to.clone()).or_default().push(i);
        }
        Graph {
            nodes,
            edges,
            out,
            inbound,
            hydrator: None,
            hydrated: HashSet::new(),
            archived: HashSet::new(),
        }
    }

    /// `set_archived` marks the documents at `paths` as archived
    /// (CLI-7_AC-67). The call replaces the archived documents from any
    /// earlier call.
    pub fn set_archived(&mut self, paths: impl IntoIterator<Item = RepoPath>) {
        self.archived = paths.into_iter().map(NodeKey::Path).collect();
    }

    /// `is_archived` returns whether the node at `key` is an archived
    /// document.
    pub fn is_archived(&self, key: &NodeKey) -> bool {
        self.archived.contains(key)
    }

    /// `add_key` adds a key node with a `defined-in` edge to its document
    /// and a `cites` edge from each document in `citing`, and gives each
    /// edge its inverse (CLI-7_AC-74, CLI-7_AC-75).
    // sokf-implements: CLI-7_AC-75
    pub fn add_key(
        &mut self,
        path: &RepoPath,
        slug: &str,
        key: &str,
        line: usize,
        citing: &[RepoPath],
    ) {
        let node = NodeKey::Key(format!("{path}#{slug}"));
        let mut fields = Map::new();
        fields.insert("key".into(), Value::from(key));
        fields.insert("line".into(), Value::from(line));
        self.nodes.insert(
            node.clone(),
            Node {
                key: node.clone(),
                kind: NodeKind::Key,
                fields,
            },
        );
        let doc = NodeKey::Path(path.clone());
        let pair = |from: &NodeKey, to: &NodeKey, rel: &str| {
            [false, true].map(|synthesised| {
                let (from, to, rel) = if synthesised {
                    (to, from, inverse(EdgeKind::Keys, rel))
                } else {
                    (from, to, rel)
                };
                Edge {
                    from: from.clone(),
                    kind: EdgeKind::Keys,
                    to: to.clone(),
                    rel: rel.to_string(),
                    region: None,
                    line: None,
                    synthesised,
                    note: None,
                }
            })
        };
        let mut edges: Vec<Edge> = pair(&node, &doc, "defined-in").into();
        for source in citing {
            edges.extend(pair(&NodeKey::Path(source.clone()), &node, "cites"));
        }
        self.extend_edges(edges);
    }

    /// `extend_edges` adds `edges` to the graph's edges, sorts them again,
    /// and removes duplicates. Each node keeps its kind, its fields, and
    /// whether it is hydrated.
    pub(super) fn extend_edges(&mut self, edges: Vec<Edge>) {
        let mut all = std::mem::take(&mut self.edges);
        all.extend(edges);
        let rebuilt = Graph::new(BTreeMap::new(), all);
        self.edges = rebuilt.edges;
        self.out = rebuilt.out;
        self.inbound = rebuilt.inbound;
    }

    /// `with_hydrator` gives the graph a hydrator. The document nodes in
    /// `built` already carry their fields. The hydrator reads the file of
    /// every other document node when the node is first visited
    /// (CACHE-1_AC-22).
    pub fn with_hydrator(mut self, hydrator: Hydrator, built: HashSet<NodeKey>) -> Graph {
        self.hydrator = Some(hydrator);
        self.hydrated = built;
        self
    }

    /// `hydrate` reads the fields of the document node at `key`, and does
    /// nothing when it has read them before or the graph has no hydrator.
    // sokf-implements: CACHE-1_AC-22
    pub fn hydrate(&mut self, key: &NodeKey) {
        let Some(hydrator) = &self.hydrator else {
            return;
        };
        if self.hydrated.contains(key) {
            return;
        }
        let Some(node) = self.nodes.get_mut(key) else {
            return;
        };
        if node.kind != NodeKind::Document {
            return;
        }
        if let Some(path) = key.path()
            && let Some(fields) = (hydrator.0)(path)
        {
            node.fields = fields;
        }
        self.hydrated.insert(key.clone());
    }

    /// `node` returns the node at `key`.
    pub fn node(&self, key: &NodeKey) -> Option<&Node> {
        self.nodes.get(key)
    }

    /// `kind` returns the kind of the node at `key`.
    pub fn kind(&self, key: &NodeKey) -> Option<NodeKind> {
        self.nodes.get(key).map(|n| n.kind)
    }

    /// `edges_from` returns the edges from `key`, in graph order.
    pub fn edges_from(&self, key: &NodeKey) -> impl Iterator<Item = &Edge> {
        self.edge_indices_from(key).iter().map(|&i| &self.edges[i])
    }

    /// `edge_indices_from` returns the indices of the edges from `key`, in
    /// graph order.
    pub fn edge_indices_from(&self, key: &NodeKey) -> &[usize] {
        self.out.get(key).map(Vec::as_slice).unwrap_or(&[])
    }

    /// `inverse_indices` returns the indices of the inverses of `edge`,
    /// declared or synthesised, in graph order. An inverse is an edge with
    /// the endpoints swapped, the same kind, region, and line, and a `rel`
    /// that is the inverse of `edge`'s `rel`. `inverse_indices` checks the
    /// match in both directions: the inverse of `edge`'s `rel` is the other
    /// edge's `rel`, or the inverse of the other edge's `rel` is `edge`'s
    /// `rel`. A custom `rel` is one that the SOKF format does not define,
    /// and its inverse is `relates-to`. So a declared `relates-to` edge is
    /// the inverse of every edge with a custom `rel` that runs in the
    /// opposite direction between the same endpoints. An edge can have
    /// several inverses.
    pub fn inverse_indices(&self, edge: &Edge) -> impl Iterator<Item = usize> {
        self.inbound
            .get(&edge.from)
            .into_iter()
            .flatten()
            .copied()
            .filter(move |&i| {
                let other = &self.edges[i];
                other.from == edge.to
                    && other.kind == edge.kind
                    && other.region == edge.region
                    && other.line == edge.line
                    && (other.rel == inverse(edge.kind, &edge.rel)
                        || inverse(other.kind, &other.rel) == edge.rel)
            })
    }

    /// `parent` returns the directory that contains a path node, or `None`
    /// for the repository root and for a node without a path.
    pub fn parent(key: &NodeKey) -> Option<NodeKey> {
        key.path()?.parent().map(NodeKey::Path)
    }

    /// `ensure` gives the node at `path` the kind `kind`. The code that
    /// resolves each `START` of `sokf graph` calls it when the `START` is a
    /// directory or a repository file (CLI-7_AC-35, CLI-7_AC-37). If the
    /// node exists, it takes the new kind and keeps its fields. If the node
    /// doesn't exist, `ensure` adds it with no fields.
    pub fn ensure(&mut self, path: RepoPath, kind: NodeKind) {
        let key = NodeKey::Path(path);
        self.nodes
            .entry(key.clone())
            .and_modify(|n| n.kind = kind)
            .or_insert_with(|| Node {
                key,
                kind,
                fields: Map::new(),
            });
    }
}

/// `ancestors` returns each directory that holds `path`, nearest first.
pub(super) fn ancestors(path: &RepoPath) -> impl Iterator<Item = RepoPath> {
    std::iter::successors(path.parent(), RepoPath::parent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> NodeKey {
        NodeKey::Path(RepoPath::parse(s).unwrap())
    }

    fn edge(from: &NodeKey, kind: EdgeKind, to: &NodeKey, rel: &str, synthesised: bool) -> Edge {
        Edge {
            from: from.clone(),
            kind,
            to: to.clone(),
            rel: rel.into(),
            region: None,
            line: None,
            synthesised,
            note: None,
        }
    }

    // sokf-tests: CLI-7_AC-13
    #[test]
    fn keys_order_paths_before_missing_targets() {
        let mut keys = vec![
            NodeKey::Missing("a".into()),
            p("/z.md"),
            p("/a.md"),
            NodeKey::Missing("b".into()),
        ];
        keys.sort();
        assert_eq!(
            keys,
            vec![
                p("/a.md"),
                p("/z.md"),
                NodeKey::Missing("a".into()),
                NodeKey::Missing("b".into())
            ]
        );
        assert_eq!(p("/a.md").as_str(), "/a.md");
        assert_eq!(NodeKey::Missing("x".into()).as_str(), "x");
        assert!(NodeKey::Missing("x".into()).path().is_none());
        assert_eq!(Graph::parent(&p("/k/a.md")), Some(p("/k")));
        assert_eq!(Graph::parent(&p("/")), None);
        assert_eq!(Graph::parent(&NodeKey::Missing("x".into())), None);
    }

    // sokf-tests: SOKF_FORMAT_114
    // sokf-tests: CLI-7_AC-14
    #[test]
    fn edges_sort_by_from_kind_to_rel_region_note_and_dedup() {
        let a = p("/a.md");
        let b = p("/b.md");
        let m = NodeKey::Missing("m".into());
        let mut edges = vec![
            edge(&b, EdgeKind::Links, &a, "depends-on", false),
            edge(&a, EdgeKind::Resource, &b, "describes", false),
            edge(&a, EdgeKind::Links, &m, "references", false),
            edge(&a, EdgeKind::Links, &b, "references", false),
            edge(&a, EdgeKind::Includes, &b, "includes", false),
            edge(&a, EdgeKind::Links, &b, "depends-on", false),
            edge(&a, EdgeKind::Links, &b, "depends-on", false),
        ];
        let g = Graph::new(BTreeMap::new(), std::mem::take(&mut edges));
        let shown: Vec<String> = g
            .edges
            .iter()
            .map(|e| {
                format!(
                    "{} {} {} {}",
                    e.from.as_str(),
                    e.kind.name(),
                    e.to.as_str(),
                    e.rel
                )
            })
            .collect();
        assert_eq!(
            shown,
            vec![
                "/a.md includes /b.md includes",
                "/a.md links /b.md depends-on",
                "/a.md links /b.md references",
                "/a.md links m references",
                "/a.md resource /b.md describes",
                "/b.md links /a.md depends-on",
            ]
        );
        assert_eq!(g.edges_from(&a).count(), 5);
        assert_eq!(g.edge_indices_from(&b), &[5]);
        assert!(g.edge_indices_from(&m).is_empty());
        assert_eq!(EdgeKind::Sources.name(), "sources");
    }

    #[test]
    fn inverses_find_the_declared_or_synthesised_side_and_every_custom_rel() {
        let a = p("/a.md");
        let b = p("/b.md");
        let c = p("/c.md");
        let g = Graph::new(
            BTreeMap::new(),
            vec![
                edge(&a, EdgeKind::Links, &b, "depends-on", false),
                edge(&b, EdgeKind::Links, &a, "depended-on-by", true),
                edge(&a, EdgeKind::Links, &c, "likes", false),
                edge(&a, EdgeKind::Links, &c, "hates", false),
                edge(&c, EdgeKind::Links, &a, "relates-to", false),
                edge(&b, EdgeKind::Resource, &c, "describes", false),
            ],
        );
        let rels = |e: &Edge| {
            g.inverse_indices(e)
                .map(|i| g.edges[i].rel.clone())
                .collect::<Vec<_>>()
        };
        let declared = &g.edges[g.edge_indices_from(&a)[0]];
        assert_eq!(declared.rel, "depends-on");
        assert_eq!(rels(declared), vec!["depended-on-by"]);
        let relates = g.edges.iter().find(|e| e.rel == "relates-to").unwrap();
        assert_eq!(rels(relates), vec!["hates", "likes"]);
        let custom = g.edges.iter().find(|e| e.rel == "likes").unwrap();
        assert_eq!(rels(custom), vec!["relates-to"]);
        let lone = g.edges.iter().find(|e| e.rel == "describes").unwrap();
        assert_eq!(rels(lone), Vec::<String>::new());
    }

    #[test]
    fn ensure_adds_or_rekinds_a_node() {
        let mut g = Graph::default();
        let path = RepoPath::parse("/src").unwrap();
        g.ensure(path.clone(), NodeKind::File);
        assert_eq!(g.kind(&p("/src")), Some(NodeKind::File));
        g.ensure(path, NodeKind::Directory);
        assert_eq!(g.kind(&p("/src")), Some(NodeKind::Directory));
        assert_eq!(g.nodes.len(), 1);
        assert!(g.node(&p("/nope")).is_none());
        assert_eq!(NodeKind::Additional.name(), "additional");
    }
}
