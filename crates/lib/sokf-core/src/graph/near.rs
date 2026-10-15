//! This module finds the edges near the starts of a query. Before the graph
//! traverses from the starts, this module reads the edges near them from a
//! store of edges, hop by hop up to the depth of the query. Then
//! `Graph::add_edges` in the `build` module adds them to the graph
//! (CACHE-1_AC-38).

use std::collections::BTreeSet;

use super::{Graph, GraphQuery, NodeKey, NodeKind, RawEdge, Start, model::ancestors};
use crate::RepoPath;

/// `EdgeStore` holds the declared edges. The graph builds the inverse of
/// each edge itself.
pub(crate) trait EdgeStore {
    /// `every` returns every edge in the store.
    fn every(&self) -> Vec<RawEdge>;
    /// `touching` returns each edge that starts or ends at one of `paths`.
    fn touching(&self, paths: &BTreeSet<RepoPath>) -> Vec<RawEdge>;
    /// `under` returns each edge that ends at a path under `dir`, and each
    /// marker edge whose file is under `dir`.
    fn under(&self, dir: &RepoPath) -> Vec<RawEdge>;
}

/// A list of edges is a store that holds each edge in the list, so that a
/// test can build a store from a list.
#[cfg(test)]
impl EdgeStore for Vec<RawEdge> {
    fn every(&self) -> Vec<RawEdge> {
        self.clone()
    }

    fn touching(&self, paths: &BTreeSet<RepoPath>) -> Vec<RawEdge> {
        self.iter()
            .filter(|e| paths.contains(&e.from) || e.to.path().is_some_and(|p| paths.contains(p)))
            .cloned()
            .collect()
    }

    fn under(&self, dir: &RepoPath) -> Vec<RawEdge> {
        self.iter()
            .filter(|e| {
                let under = |p: &RepoPath| p != dir && p.starts_with(dir);
                e.to.path().is_some_and(under)
                    || e.kind == super::EdgeKind::Marker && under(&e.from)
            })
            .cloned()
            .collect()
    }
}

/// `edges_near` returns the edges that the output of a traversal from
/// `starts` shows, and possibly more. The edges of a node are those that
/// start at it and those that end at it, and the function reads them at
/// most once across all the hops. For each kind of start, the function
/// first reads these edges:
///
/// - For a node, the function reads the node's own edges. For a file, an
///   additional file, or an asset, it also reads the edges of each
///   directory that holds the node. The output shows these edges as `via`
///   edges (CLI-7_AC-72).
/// - For a key, the function reads no edges, because `graph` already holds
///   the key's edges.
/// - For a directory outside every knowledge, the function reads the
///   directory's own edges, the edges of each directory that holds it, and
///   the edges to each file under it, because its layout lists those files.
/// - For a knowledge root, or a directory in a knowledge, the function
///   reads no edges.
///
/// At each later hop, up to the depth, the function reads the edges of
/// each node that the previous hop reached. When a start is a key, the
/// edges of that key reach other nodes. At the second hop, the function
/// also reads the edges of those nodes.
///
/// A query can ask for the edges of each line of a layout. When it does,
/// and a start is a knowledge root, a directory in a knowledge, or a
/// directory outside every knowledge, the function reads every edge in the
/// store (CLI-7_AC-65). With depth 0, the output shows none of a start's
/// own edges.
///
/// Reading more edges than the traversal needs doesn't change the output,
/// for two reasons. First, the output shows an edge only from a node whose
/// edges the function read. Second, the function reads both the edges that
/// start at a node and those that end at it. So it also reads each declared
/// edge whose inverse starts at the node.
// sokf-implements: CACHE-1_AC-38
// sokf-implements: CACHE-1_AC-44
// sokf-implements: CACHE-1_AC-45
// sokf-implements: CACHE-1_AC-39
pub(crate) fn edges_near(
    graph: &Graph,
    starts: &[Start],
    query: &GraphQuery,
    store: &dyn EdgeStore,
) -> Vec<RawEdge> {
    let mut out: Vec<RawEdge> = Vec::new();
    let mut frontier: BTreeSet<RepoPath> = BTreeSet::new();
    let mut holders: BTreeSet<RepoPath> = BTreeSet::new();
    let mut second: BTreeSet<RepoPath> = BTreeSet::new();
    for start in starts {
        match start {
            Start::Container(_) | Start::Outside(_) if query.layout_edges => return store.every(),
            Start::Container(_) => {}
            Start::Outside(dir) => {
                out.extend(store.under(dir));
                frontier.insert(dir.clone());
                holders.extend(ancestors(dir));
            }
            Start::Node(key @ NodeKey::Path(p)) => {
                frontier.insert(p.clone());
                // The `hops` function adds `via` edges to the row of a node
                // of these kinds. A start that is a directory is a container
                // or a directory outside every knowledge, not a node.
                if graph.kind(key).is_some_and(NodeKind::shows_via) {
                    holders.extend(ancestors(p));
                }
            }
            Start::Node(key @ NodeKey::Key(_)) => {
                second.extend(graph.edges_from(key).filter_map(|e| e.to.path().cloned()));
            }
            Start::Node(NodeKey::Missing(_)) => {}
        }
    }
    if query.depth == 0 {
        return out;
    }
    if !holders.is_empty() {
        out.extend(store.touching(&holders));
    }
    let mut read: BTreeSet<RepoPath> = BTreeSet::new();
    for hop in 1..=query.depth {
        if hop == 2 {
            frontier.append(&mut second);
        }
        frontier.retain(|p| !read.contains(p));
        if frontier.is_empty() {
            if hop >= 2 {
                break;
            }
            continue;
        }
        let edges = store.touching(&frontier);
        let mut reached: BTreeSet<RepoPath> = BTreeSet::new();
        for e in &edges {
            if frontier.contains(&e.from)
                && let Some(p) = e.to.path()
            {
                reached.insert(p.clone());
            }
            if e.to.path().is_some_and(|p| frontier.contains(p)) {
                reached.insert(e.from.clone());
            }
        }
        out.extend(edges);
        read.append(&mut frontier);
        frontier = reached;
    }
    out
}
