//! These helpers run `sokf graph` on a tree, and collect the rows and the
//! edge entries of a graph result for the property tests of `sokf graph`.

use sokf_core::{
    CacheMode, Entry, Graph, GraphOptions, GraphQuery, GraphResult, NodeKey, NodeKind, Row,
};

use super::TempTree;

/// `query` returns the query from `start` that follows links up to `depth`
/// steps away.
pub fn query(start: &[&str], depth: usize) -> GraphQuery {
    GraphQuery {
        start: start.iter().map(ToString::to_string).collect(),
        depth,
        ..GraphQuery::default()
    }
}

/// `graph_of` runs `query` on `tree` with the cache set to `cache`.
pub fn graph_of(
    tree: &TempTree,
    query: GraphQuery,
    cache: CacheMode,
) -> sokf_core::Result<(Graph, GraphResult)> {
    sokf_core::graph(
        &tree.root(),
        &GraphOptions {
            query,
            cwd: tree.dir().to_path_buf(),
            cache,
        },
    )
}

/// `rows` returns, in order, every top-level row that is its node's main row
/// and lists the node's edges. It skips roots, directories, and any row whose
/// node has its main row elsewhere in the tree. It does not count the lines of
/// a layout as rows.
pub fn rows<'a>(graph: &Graph, result: &'a GraphResult) -> Vec<&'a Row> {
    result
        .rows
        .iter()
        .filter(|r| {
            !r.elsewhere
                && !matches!(
                    graph.kind(&r.node),
                    Some(NodeKind::Root | NodeKind::Directory)
                )
        })
        .collect()
}

/// `layout_lines` returns, in order, every line of a layout, at any depth,
/// whose node is neither a root nor a directory.
pub fn layout_lines<'a>(graph: &Graph, result: &'a GraphResult) -> Vec<&'a Row> {
    fn walk<'a>(graph: &Graph, row: &'a Row, out: &mut Vec<&'a Row>) {
        if !matches!(
            graph.kind(&row.node),
            Some(NodeKind::Root | NodeKind::Directory)
        ) {
            out.push(row);
        }
        for e in &row.entries {
            if let Entry::Layout(r) = e {
                walk(graph, r, out);
            }
        }
    }
    let mut out = Vec::new();
    for r in &result.rows {
        walk(graph, r, &mut out);
    }
    out
}

/// `edge_entries` returns every edge entry, at any depth, as a tuple of the
/// edge, its target, and whether the entry expands the target in place.
pub fn edge_entries(result: &GraphResult) -> Vec<(usize, NodeKey, bool)> {
    fn walk(entries: &[Entry], out: &mut Vec<(usize, NodeKey, bool)>) {
        for e in entries {
            match e {
                Entry::Layout(r) => walk(&r.entries, out),
                Entry::Edge {
                    edge,
                    target,
                    expanded,
                    elsewhere,
                    ..
                } => {
                    out.push((*edge, target.clone(), !expanded.is_empty() && !elsewhere));
                    walk(expanded, out);
                }
                Entry::More { .. } => {}
            }
        }
    }
    let mut out = Vec::new();
    for r in &result.rows {
        walk(&r.entries, &mut out);
    }
    out
}
