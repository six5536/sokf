//! This module runs one `GraphQuery` over the graph. It returns a tree of
//! rows, and the nodes and edges that the tree mentions. The query's depth,
//! limit, and filters decide which nodes and edges the tree holds (CLI-7).

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

use serde_json::Value;

use super::{Graph, NodeKey, NodeKind, Start, json, resolve_start};
use crate::{Expr, Model, RepoPath, Result, model::KeyIndex};

/// `LIMIT` is the number of entries of one group that print without
/// `--all` (CLI-7_AC-69). A group is the entries of a directory or the edges
/// of one relationship of a node.
pub const LIMIT: usize = 20;

/// `GraphQuery` holds the bounds of `sokf graph`, one field for each bound.
#[derive(Debug, Clone)]
pub struct GraphQuery {
    /// The starts that `START...` gives. When it is empty, the query starts
    /// at every knowledge root.
    pub start: Vec<String>,
    /// The number of hops that `--depth` follows from a start that is a
    /// document, a file, or a key.
    pub depth: usize,
    /// Whether `--depth` was given. When it was, each line of a layout
    /// prints its edges to `depth` hops, as a document start does
    /// (CLI-7_AC-65).
    pub layout_edges: bool,
    /// The `-l` selectors. A document matches when any one of them is true.
    pub selector: Vec<Expr>,
    /// The `--edge` selectors. An edge matches when any one of them is true.
    pub edge: Vec<Expr>,
    /// The `--to` selectors. A target matches when any one of them is true.
    pub to: Vec<Expr>,
    /// The expression of `--query`, which projects the JSON form of the
    /// result.
    pub query: Option<Expr>,
    /// `all` is whether `--all` was given. When it was, the result prints
    /// every entry past the limit and every region of a folded include.
    pub all: bool,
    /// `archived` is whether `--archived` was given. When it was, the result
    /// prints the archived documents and their edges.
    pub archived: bool,
}

impl Default for GraphQuery {
    fn default() -> Self {
        GraphQuery {
            start: Vec::new(),
            depth: 1,
            layout_edges: false,
            selector: Vec::new(),
            edge: Vec::new(),
            to: Vec::new(),
            query: None,
            all: false,
            archived: false,
        }
    }
}

/// `Row` is a node that the tree prints with the lines under it. A row is
/// a start or a node of a layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The node.
    pub node: NodeKey,
    /// Whether the node is a row elsewhere in the tree, in which case this
    /// row prints as one line marked `(*)`.
    pub elsewhere: bool,
    /// The layout under a container, or the edges under a row.
    pub entries: Vec<Entry>,
}

/// `Entry` is one line under a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A node of a container's layout.
    Layout(Row),
    /// An edge of a row or an expanded node.
    Edge {
        /// The edge's index in the graph.
        edge: usize,
        /// The edge's target.
        target: NodeKey,
        /// The target's edges, when the target expands at this entry.
        expanded: Vec<Entry>,
        /// Whether the target's edges print elsewhere, in which case the
        /// line is marked `(*)`.
        elsewhere: bool,
        /// The number of regions of a folded include, or of markers of a
        /// folded marker edge, or 0 when the edge is not folded (CLI-7_AC-70,
        /// CLI-7_AC-91).
        folded: usize,
        /// Whether the edge belongs to a directory that holds the row's
        /// file, not to the row itself (CLI-7_AC-72).
        via: bool,
    },
    /// The number of entries of a group past the limit, where the group is
    /// a directory or one relationship (CLI-7_AC-69).
    More {
        /// The relationship of the group, or `None` when the group is a
        /// directory.
        rel: Option<String>,
        /// The entries left out.
        count: usize,
    },
}

/// `Summary` counts what a view printed, and what the same view prints
/// with `--all`, `--archived`, and no filter (CLI-7_AC-77).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// The documents printed, out of the documents of the whole view.
    pub documents: Count,
    /// The archived documents printed, out of the archived documents of
    /// the whole view.
    pub archived: Count,
}

/// `Count` is the number of items that printed, out of a total.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    /// The number that printed.
    pub shown: usize,
    /// The number that the whole view holds.
    pub total: usize,
}

impl Summary {
    /// `line` returns the last line of the text form: `<a> / <b> docs`,
    /// followed by ` (<c> / <d> archived)` when the view reaches an archived
    /// document (CLI-7_AC-77).
    pub fn line(&self) -> String {
        let docs = format!("{} / {} docs", self.documents.shown, self.documents.total);
        if self.archived.total > 0 {
            format!(
                "{docs} ({} / {} archived)",
                self.archived.shown, self.archived.total
            )
        } else {
            docs
        }
    }
}

/// `GraphResult` is the result of one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphResult {
    /// The node of each start, in the order given.
    pub start: Vec<NodeKey>,
    /// One tree for each start. It is empty when a `query` is given.
    pub rows: Vec<Row>,
    /// Every node the tree mentions, in node order.
    pub nodes: Vec<NodeKey>,
    /// Every edge the tree mentions and the inverse of each, in edge order.
    pub edges: Vec<usize>,
    /// The number of regions of each folded include, or of markers of each
    /// folded marker edge, and of its inverse, keyed by edge.
    pub folded: BTreeMap<usize, usize>,
    /// The counts of the last line of the text form.
    pub summary: Summary,
    /// The `query` result, when one was given.
    pub projection: Option<Value>,
}

/// `starts` returns the starts of `query` in the order given, or each
/// knowledge root when `query` gives none. It adds to `graph` each
/// directory or file that a start refers to.
// sokf-implements: CLI-7_AC-3
pub(crate) fn starts(
    graph: &mut Graph,
    model: &Model,
    keys: Option<&KeyIndex>,
    cwd: &Path,
    query: &GraphQuery,
) -> Result<Vec<Start>> {
    if query.start.is_empty() {
        return Ok(model
            .tree
            .roots
            .iter()
            .map(|r| Start::Container(r.clone()))
            .collect());
    }
    query
        .start
        .iter()
        .map(|arg| resolve_start(graph, model, keys, cwd, arg))
        .collect()
}

/// `run` runs `query` over `graph`. It adds to `graph` each directory or
/// file that a start refers to.
// sokf-implements: CLI-7_AC-34
// sokf-implements: CLI-7_AC-35
// sokf-implements: CLI-7_AC-87
// sokf-implements: CLI-7_AC-38
// sokf-implements: CLI-7_AC-46
// sokf-implements: CLI-7_AC-47
// sokf-implements: CLI-7_AC-79
pub fn run(
    graph: &mut Graph,
    model: &Model,
    keys: Option<&KeyIndex>,
    cwd: &Path,
    query: &GraphQuery,
) -> Result<GraphResult> {
    let starts = starts(graph, model, keys, cwd, query)?;
    // A node that a start refers to prints even when it is archived. So does
    // the defining document of each key that a start refers to.
    let named: BTreeSet<NodeKey> = starts
        .iter()
        .filter(|s| matches!(s, Start::Node(_)))
        .map(Start::key)
        .flat_map(|k| {
            let document = match &k {
                NodeKey::Key(text) => text
                    .split_once('#')
                    .and_then(|(p, _)| RepoPath::parse(p))
                    .map(NodeKey::Path),
                _ => None,
            };
            std::iter::once(k).chain(document)
        })
        .collect();
    let (rows, m) = tree(graph, &starts, &named, query);
    // The whole view has no limit and no fold, does not hide archived
    // documents, and applies no filter.
    let whole_query = GraphQuery {
        selector: Vec::new(),
        edge: Vec::new(),
        to: Vec::new(),
        query: None,
        all: true,
        archived: true,
        ..query.clone()
    };
    let whole = if query.all && query.archived && !filtered(query) {
        None
    } else {
        Some(tree(graph, &starts, &named, &whole_query).1)
    };
    let whole = whole.as_ref().unwrap_or(&m);
    // `run` hydrates each node that the result prints. Hydrating a node loads
    // its fields (CACHE-1_AC-22).
    for key in &m.nodes {
        graph.hydrate(key);
    }
    // The total counts each node that either view reaches. A node expands
    // only at the first edge that reaches it. It then follows its own edges
    // for the hops that remain after that edge. The whole view expands a node
    // at its first edge. This run's view applies its filters and its limit,
    // which can drop that edge. When they do, this run's view expands the node
    // at a different edge, where more hops can remain. This run's view can
    // then reach nodes that the whole view does not reach.
    let union: BTreeSet<&NodeKey> = m.nodes.iter().chain(&whole.nodes).collect();
    let documents = |nodes: &mut dyn Iterator<Item = &NodeKey>| {
        nodes
            .filter(|k| graph.kind(k) == Some(NodeKind::Document))
            .count()
    };
    let archived = |nodes: &mut dyn Iterator<Item = &NodeKey>| {
        nodes
            .filter(|k| graph.kind(k) == Some(NodeKind::Document) && graph.is_archived(k))
            .count()
    };
    let summary = Summary {
        documents: Count {
            shown: documents(&mut m.nodes.iter()),
            total: documents(&mut union.iter().copied()),
        },
        archived: Count {
            shown: archived(&mut m.nodes.iter()),
            total: archived(&mut union.iter().copied()),
        },
    };
    let mut result = GraphResult {
        start: starts.iter().map(Start::key).collect(),
        rows,
        nodes: m.nodes.into_iter().collect(),
        edges: m.edges.into_iter().collect(),
        folded: m.folded,
        summary,
        projection: None,
    };
    if let Some(q) = &query.query {
        // sokf-implements: CLI-7_AC-22
        result.projection = Some(q.eval(&json::bounded(graph, &result))?);
        result.rows.clear();
    }
    Ok(result)
}

/// `filtered` returns whether a query filters, that is, whether it gives a
/// selector, an `--edge`, or a `--to`.
fn filtered(query: &GraphQuery) -> bool {
    !(query.selector.is_empty() && query.edge.is_empty() && query.to.is_empty())
}

/// `tree` builds the tree of `starts`. It lays out each start, adds the
/// edges under each row, and then marks each line whose node prints
/// elsewhere. It returns the rows of the tree, and the nodes, edges, and
/// folded counts that those rows mention.
fn tree(
    graph: &mut Graph,
    starts: &[Start],
    named: &BTreeSet<NodeKey>,
    query: &GraphQuery,
) -> (Vec<Row>, Mentions) {
    let mut t = Traversal::new(graph, query, named.clone());
    let mut rows: Vec<Row> = Vec::new();
    for start in starts {
        let row = match start {
            Start::Container(p) => Row {
                node: NodeKey::Path(p.clone()),
                elsewhere: false,
                entries: t.layout(p),
            },
            Start::Outside(p) => {
                // The directory is a row: its own edges, and those of the
                // directories that hold it, print before the nodes under it.
                let node = NodeKey::Path(p.clone());
                t.rows.insert(node.clone());
                Row {
                    node,
                    elsewhere: false,
                    entries: t.outside(p),
                }
            }
            // sokf-implements: CLI-7_AC-37
            Start::Node(k) => t.row(k.clone()),
        };
        // With a selector, `tree` drops a container start that holds no
        // matching node. It keeps every other start, even one with no
        // entries yet. The reason is that `fill` adds the edges of a node start
        // and of a directory outside every knowledge root after this loop.
        let container = matches!(start, Start::Container(_));
        if container && !query.selector.is_empty() && row.entries.is_empty() {
            continue;
        }
        rows.push(row);
    }
    for row in &mut rows {
        t.fill(row, true);
    }
    let mut seen = BTreeSet::new();
    for row in &mut rows {
        t.settle(&mut row.entries, 1, &mut seen);
    }
    let mut m = Mentions::default();
    for row in &rows {
        t.mention(row, &mut m);
    }
    (rows, m)
}

/// `Mentions` holds the nodes, edges, and folded counts that the tree
/// mentions.
#[derive(Default)]
struct Mentions {
    nodes: BTreeSet<NodeKey>,
    edges: BTreeSet<usize>,
    folded: BTreeMap<usize, usize>,
}

struct Traversal<'a> {
    graph: &'a mut Graph,
    query: &'a GraphQuery,
    rows: BTreeSet<NodeKey>,
    expanded: BTreeSet<NodeKey>,
    json: HashMap<NodeKey, Value>,
    /// The nodes that the starts refer to, and the defining document of each
    /// key among them. These nodes print even when they are archived.
    named: BTreeSet<NodeKey>,
    /// The number of times that a document was left out because it is
    /// archived.
    hidden: usize,
    /// The nodes under each directory, in key order. The first layout
    /// builds this map in one pass over the nodes.
    children: Option<HashMap<NodeKey, Vec<(NodeKey, NodeKind)>>>,
}

impl<'a> Traversal<'a> {
    fn new(graph: &'a mut Graph, query: &'a GraphQuery, named: BTreeSet<NodeKey>) -> Self {
        Traversal {
            graph,
            query,
            rows: BTreeSet::new(),
            expanded: BTreeSet::new(),
            json: HashMap::new(),
            named,
            hidden: 0,
            children: None,
        }
    }

    /// `node_json` returns the JSON form of a node, which it builds once.
    /// A predicate reads the node's fields, so `node_json` hydrates the
    /// node first (CACHE-1_AC-22).
    fn node_json(&mut self, key: &NodeKey) -> &Value {
        if !self.json.contains_key(key) {
            self.graph.hydrate(key);
            let value = self
                .graph
                .node(key)
                .map(json::node_json)
                .unwrap_or(Value::Null);
            self.json.insert(key.clone(), value);
        }
        &self.json[key]
    }

    /// `document_matches` returns whether some expression of `exprs`, the
    /// selector or `--to`, is true of the node, which must be a document.
    /// When there is no expression, every node matches.
    // sokf-implements: CLI-7_AC-8
    // sokf-implements: CLI-7_AC-45
    // sokf-implements: CLI-7_AC-86
    fn document_matches(&mut self, key: &NodeKey, exprs: &[Expr]) -> bool {
        if exprs.is_empty() {
            return true;
        }
        if self.graph.kind(key) != Some(NodeKind::Document) {
            return false;
        }
        let value = self.node_json(key);
        exprs.iter().any(|e| e.truthy(value))
    }

    /// `hides` returns whether `key` is an archived document that this run
    /// leaves out, and counts it when it is. The archive rule applies before
    /// the filters (CLI-7_AC-67, CLI-7_AC-68).
    // sokf-implements: CLI-7_AC-67
    // sokf-implements: CLI-7_AC-68
    // sokf-implements: CLI-7_AC-82
    fn hides(&mut self, key: &NodeKey) -> bool {
        let hidden =
            !self.query.archived && self.graph.is_archived(key) && !self.named.contains(key);
        if hidden {
            self.hidden += 1;
        }
        hidden
    }

    /// `row` claims `key` as a row, or marks it as a row elsewhere when
    /// another row has claimed it.
    fn row(&mut self, key: NodeKey) -> Row {
        let elsewhere = !self.rows.insert(key.clone());
        Row {
            node: key,
            elsewhere,
            entries: Vec::new(),
        }
    }

    /// `fill` adds the edges under every start that is a row. A line of a
    /// layout gets no edges unless `--depth` was given.
    // sokf-implements: CLI-7_AC-2
    fn fill(&mut self, row: &mut Row, start: bool) {
        if row.elsewhere {
            return;
        }
        let mut entries = if self.rows.contains(&row.node)
            && (start || self.query.layout_edges)
            && self.query.depth >= 1
        {
            self.hops(&row.node, 1)
        } else {
            Vec::new()
        };
        for entry in &mut row.entries {
            if let Entry::Layout(inner) = entry {
                self.fill(inner, false);
            }
        }
        entries.append(&mut row.entries);
        row.entries = entries;
    }

    /// `hops` returns the edges of `node` at `hop`, and expands each target
    /// at its first appearance below `depth`. First it drops each edge to
    /// an archived document that this run leaves out. Then it drops each
    /// edge that a filter rejects. Next, `mirror` drops each untyped mention
    /// that a typed link repeats. Then `fold` merges into one edge the
    /// includes of regions of the same file, and the markers of one file
    /// under a document. Last, the limit leaves out the
    /// edges past `LIMIT`. `node` can be a row that is a file, or a directory
    /// outside every knowledge root. In that case, `hops` also adds the edges
    /// from each directory above `node` to the document that describes that
    /// directory.
    // sokf-implements: CLI-7_AC-9
    // sokf-implements: CLI-7_AC-10
    // sokf-implements: CLI-7_AC-48
    fn hops(&mut self, node: &NodeKey, hop: usize) -> Vec<Entry> {
        let mut candidates: Vec<(usize, bool)> = self
            .graph
            .edge_indices_from(node)
            .iter()
            .map(|&i| (i, false))
            .collect();
        // A key's document comes first, then the documents that cite it.
        if self.graph.kind(node) == Some(NodeKind::Key) {
            candidates.sort_by_key(|&(i, _)| self.graph.edges[i].rel != "defined-in");
        }
        if hop == 1 && self.graph.kind(node).is_some_and(NodeKind::shows_via) {
            candidates.extend(self.ancestors(node));
        }
        let query = self.query;
        let mut kept = Vec::new();
        for (i, via) in candidates {
            let to = self.graph.edges[i].to.clone();
            if self.hides(&to) {
                continue;
            }
            if !query.edge.is_empty() {
                let value = json::edge_json(&self.graph.edges[i]);
                if !query.edge.iter().any(|e| e.truthy(&value)) {
                    continue;
                }
            }
            if !self.document_matches(&to, &query.to) {
                continue;
            }
            kept.push((i, via));
        }
        let groups = self.fold(self.mirror(kept));
        let mut out = Vec::new();
        let mut per_rel: HashMap<String, usize> = HashMap::new();
        let mut past: Vec<(String, usize)> = Vec::new();
        for (i, via, members) in groups {
            let rel = self.graph.edges[i].rel.clone();
            let n = per_rel.entry(rel.clone()).or_insert(0);
            *n += 1;
            // sokf-implements: CLI-7_AC-69
            if !self.query.all && *n > LIMIT {
                match past.iter_mut().find(|(r, _)| *r == rel) {
                    Some((_, count)) => *count += 1,
                    None => past.push((rel, 1)),
                }
                continue;
            }
            let target = self.graph.edges[i].to.clone();
            let expanded =
                if !via && self.expands(&target, hop) && self.expanded.insert(target.clone()) {
                    self.hops(&target, hop + 1)
                } else {
                    Vec::new()
                };
            out.push(Entry::Edge {
                edge: i,
                target,
                expanded,
                elsewhere: false,
                folded: members,
                via,
            });
        }
        for (rel, count) in past {
            out.push(Entry::More {
                rel: Some(rel),
                count,
            });
        }
        out
    }

    /// `expands` returns whether a target reached at `hop` can expand: the
    /// hop is below the depth, and the target is neither a row nor a missing
    /// node.
    // sokf-implements: CLI-7_AC-6
    // sokf-implements: CLI-7_AC-39
    // sokf-implements: CLI-7_AC-40
    fn expands(&self, target: &NodeKey, hop: usize) -> bool {
        hop < self.query.depth
            && !self.rows.contains(target)
            && self.graph.kind(target) != Some(NodeKind::Missing)
    }

    /// `settle` is the second pass over the tree, after the first pass, in
    /// which `fill` adds the edges under each row. `settle` sets `elsewhere` on each edge whose
    /// target is a row or expands at another entry. `settle` walks the
    /// entries in the same order as the first pass, and tests each target
    /// with the same rule. So the first entry where a target can expand is
    /// the entry where the first pass expanded it. `settle` records that
    /// target in `seen` and leaves that entry unmarked.
    fn settle(&self, entries: &mut [Entry], hop: usize, seen: &mut BTreeSet<NodeKey>) {
        for entry in entries {
            match entry {
                Entry::Layout(row) => self.settle(&mut row.entries, 1, seen),
                Entry::Edge {
                    target,
                    expanded,
                    elsewhere,
                    via,
                    ..
                } => {
                    let expanded_here =
                        !*via && self.expands(target, hop) && seen.insert(target.clone());
                    *elsewhere = !expanded_here
                        && (self.rows.contains(target) || self.expanded.contains(target));
                    self.settle(expanded, hop + 1, seen);
                }
                Entry::More { .. } => {}
            }
        }
    }

    /// `mention` collects every node and edge that the tree mentions, and
    /// the inverse of each edge.
    fn mention(&self, row: &Row, m: &mut Mentions) {
        m.nodes.insert(row.node.clone());
        self.mention_entries(&row.entries, m);
    }

    fn mention_entries(&self, entries: &[Entry], m: &mut Mentions) {
        for entry in entries {
            match entry {
                Entry::Layout(row) => self.mention(row, m),
                Entry::Edge {
                    edge,
                    target,
                    expanded,
                    folded,
                    via,
                    ..
                } => {
                    let e = &self.graph.edges[*edge];
                    m.nodes.insert(target.clone());
                    if *via {
                        m.nodes.insert(e.from.clone());
                    }
                    m.edges.insert(*edge);
                    // sokf-implements: CLI-7_AC-56
                    let inverses: Vec<usize> = self.graph.inverse_indices(e).collect();
                    if *folded > 0 {
                        m.folded.insert(*edge, *folded);
                        for &i in &inverses {
                            m.folded.insert(i, *folded);
                        }
                    }
                    m.edges.extend(inverses);
                    self.mention_entries(expanded, m);
                }
                Entry::More { .. } => {}
            }
        }
    }
}

mod layout;
mod select;

#[cfg(test)]
mod tests;
