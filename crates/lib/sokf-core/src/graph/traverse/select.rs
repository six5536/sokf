//! This module chooses the edges of a row: it adds the documents that
//! describe each directory holding a file, drops each mention that only
//! repeats a typed link, and folds the includes and the markers of one file (see
//! design-graph, 'Build the view of a row').

use std::collections::{BTreeSet, HashMap};

use super::Traversal;
use crate::{
    RepoPath,
    graph::{EdgeKind, NodeKey, NodeKind, model::ancestors},
};

impl Traversal<'_> {
    /// `ancestors` returns the edges from each directory above `node` to the
    /// document that describes it, nearest directory first (CLI-7_AC-72).
    // sokf-implements: CLI-7_AC-72
    // sokf-implements: CLI-7_AC-89
    pub(super) fn ancestors(&self, node: &NodeKey) -> Vec<(usize, bool)> {
        node.path()
            .into_iter()
            .flat_map(ancestors)
            .flat_map(|d| {
                self.graph
                    .edge_indices_from(&NodeKey::Path(d))
                    .iter()
                    .copied()
            })
            .filter(|&i| {
                let e = &self.graph.edges[i];
                e.kind == EdgeKind::Resource && e.rel == "described-by"
            })
            .map(|i| (i, true))
            .collect()
    }

    /// `mirror` drops each untyped mention whose two nodes a typed link
    /// already joins, and returns the edges that remain (CLI-7_AC-71).
    // sokf-implements: CLI-7_AC-71
    pub(super) fn mirror(&self, kept: Vec<(usize, bool)>) -> Vec<(usize, bool)> {
        let untyped = |i: usize| {
            let e = &self.graph.edges[i];
            e.kind == EdgeKind::Links && e.rel == "relates-to"
        };
        // Every edge that is not an untyped mention is typed. A typed edge to
        // an asset also counts for the asset's sidecar, because both print
        // as the same id.
        let mut typed: BTreeSet<NodeKey> = BTreeSet::new();
        for (i, _) in &kept {
            if untyped(*i) {
                continue;
            }
            let to = &self.graph.edges[*i].to;
            typed.insert(to.clone());
            if self.graph.kind(to) == Some(NodeKind::Asset)
                && let Some(p) = to.path()
                && let Some(sidecar) = RepoPath::parse(&format!("{p}.md"))
            {
                typed.insert(NodeKey::Path(sidecar));
            }
        }
        // A document can describe both the row and a directory that holds the
        // row, or two such directories. It prints once, at the nearest
        // directory.
        let mut described: BTreeSet<NodeKey> = BTreeSet::new();
        kept.into_iter()
            .filter(|(i, _)| !(untyped(*i) && typed.contains(&self.graph.edges[*i].to)))
            .filter(|(i, _)| {
                let e = &self.graph.edges[*i];
                e.kind != EdgeKind::Resource
                    || e.rel != "described-by"
                    || described.insert(e.to.clone())
            })
            .collect()
    }

    /// Unless the query sets `--all`, `fold` merges edges into the first
    /// of them, and records on that edge how many it covers:
    ///
    /// - the include edges of regions of one file (CLI-7_AC-70). An include
    ///   of the whole file stays a separate edge, and a single region is no
    ///   fold.
    /// - the marker edges of one file and one rel under a document
    ///   (CLI-7_AC-91), even a single one. Under a file or a key, each
    ///   marker stays an edge of its own (CLI-7_AC-93, CLI-7_AC-94).
    ///
    /// Each group returns with its count, which is 0 for an edge that is no
    /// fold.
    // sokf-implements: CLI-7_AC-70
    // sokf-implements: CLI-7_AC-81
    // sokf-implements: CLI-7_AC-91
    // sokf-implements: CLI-7_AC-92
    pub(super) fn fold(&self, kept: Vec<(usize, bool)>) -> Vec<(usize, bool, usize)> {
        let mut groups: Vec<(usize, bool, usize)> = Vec::new();
        let mut markers: BTreeSet<usize> = BTreeSet::new();
        let mut at: HashMap<(EdgeKind, NodeKey, String), usize> = HashMap::new();
        for (i, via) in kept {
            let e = &self.graph.edges[i];
            let marker =
                e.kind == EdgeKind::Marker && self.graph.kind(&e.from) == Some(NodeKind::Document);
            if !self.query.all && (marker || e.kind == EdgeKind::Includes && e.region.is_some()) {
                let key = (e.kind, e.to.clone(), e.rel.clone());
                if let Some(&g) = at.get(&key) {
                    groups[g].2 += 1;
                    continue;
                }
                at.insert(key, groups.len());
                if marker {
                    markers.insert(groups.len());
                }
            }
            groups.push((i, via, 1));
        }
        for (g, group) in groups.iter_mut().enumerate() {
            if group.2 == 1 && !markers.contains(&g) {
                group.2 = 0;
            }
        }
        groups
    }
}
