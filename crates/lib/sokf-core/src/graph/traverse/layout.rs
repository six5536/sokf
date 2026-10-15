//! This module builds the layout under a start that is a directory or a
//! root: its directories, documents, and additional files (CLI-7_AC-1). It
//! leaves out archived documents and cuts each directory to the entry limit.

use std::collections::HashMap;

use super::{Entry, LIMIT, NodeKind, Row, Traversal};
use crate::{
    RepoPath,
    graph::{Graph, NodeKey},
};

impl Traversal<'_> {
    /// `line` returns a layout line: a row for `key` with none of the node's
    /// edges.
    pub(super) fn line(key: NodeKey) -> Entry {
        Entry::Layout(Row {
            node: key,
            elsewhere: false,
            entries: Vec::new(),
        })
    }

    /// `layout_line` returns the layout line for `key`. When the query asks
    /// for edges in the layout, the line is a full row, and the node's edges
    /// print under it (CLI-7_AC-65). Otherwise the line is a plain line from
    /// `line`.
    pub(super) fn layout_line(&mut self, key: NodeKey) -> Entry {
        if self.query.layout_edges {
            Entry::Layout(self.row(key))
        } else {
            Self::line(key)
        }
    }

    /// `cap` keeps the first `LIMIT` entries of a directory and replaces the
    /// rest with one entry that counts them (CLI-7_AC-69). With `--all`, it
    /// keeps every entry.
    // sokf-implements: CLI-7_AC-69
    // sokf-implements: CLI-7_AC-81
    pub(super) fn cap(&self, mut out: Vec<Entry>) -> Vec<Entry> {
        if !self.query.all && out.len() > LIMIT {
            let count = out.len() - LIMIT;
            out.truncate(LIMIT);
            out.push(Entry::More { rel: None, count });
        }
        out
    }

    /// `layout` returns the layout under `dir`: its directories, documents,
    /// and additional files, nested, in key order, with no edges. It leaves
    /// out each archived document, and prunes a directory that is left empty
    /// because its documents are archived. With a selector, it keeps only the matching
    /// documents and prunes every empty directory. It leaves out an asset
    /// that has a sidecar, because the sidecar represents the asset.
    // sokf-implements: CLI-7_AC-1
    // sokf-implements: CLI-7_AC-25
    // sokf-implements: CLI-7_AC-84
    pub(super) fn layout(&mut self, dir: &RepoPath) -> Vec<Entry> {
        let graph = &self.graph;
        let index = self.children.get_or_insert_with(|| {
            let mut index: HashMap<NodeKey, Vec<(NodeKey, NodeKind)>> = HashMap::new();
            for n in graph.nodes.values() {
                if let Some(parent) = Graph::parent(&n.key) {
                    index
                        .entry(parent)
                        .or_default()
                        .push((n.key.clone(), n.kind));
                }
            }
            index
        });
        let children = index
            .get(&NodeKey::Path(dir.clone()))
            .cloned()
            .unwrap_or_default();
        let selecting = !self.query.selector.is_empty();
        let mut out = Vec::new();
        for (key, kind) in children {
            match kind {
                NodeKind::Root | NodeKind::Directory => {
                    let Some(path) = key.path() else { continue };
                    let before = self.hidden;
                    let entries = self.layout(&path.clone());
                    if entries.is_empty() && (selecting || self.hidden > before) {
                        continue;
                    }
                    out.push(Entry::Layout(Row {
                        node: key,
                        elsewhere: false,
                        entries,
                    }));
                }
                NodeKind::Document => {
                    let query = self.query;
                    if !self.hides(&key) && self.document_matches(&key, &query.selector) {
                        out.push(self.layout_line(key));
                    }
                }
                // An additional file is not a document, so no selector
                // matches it. An asset prints as its sidecar, unless it has
                // none.
                NodeKind::Additional | NodeKind::Asset => {
                    let sidecar = kind == NodeKind::Asset
                        && self
                            .graph
                            .node(&key)
                            .is_some_and(|n| n.fields.contains_key("sidecar"));
                    if !selecting && !sidecar {
                        out.push(self.layout_line(key));
                    }
                }
                NodeKind::File | NodeKind::Key | NodeKind::Missing => {}
            }
        }
        self.cap(out)
    }

    /// `outside` returns the layout under a directory outside every
    /// knowledge store: each root with its layout, and, without nesting,
    /// each additional file outside every root and each file that has edges
    /// of its own. The entries follow path order, and none prints its edges.
    /// A file that appears only as a start has no edges, so `outside` leaves
    /// it out. With a selector, it keeps only the roots that hold a matching
    /// document.
    // sokf-implements: CLI-7_AC-73
    // sokf-implements: CLI-7_AC-87
    pub(super) fn outside(&mut self, dir: &RepoPath) -> Vec<Entry> {
        let roots: Vec<RepoPath> = self
            .graph
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Root)
            .filter_map(|n| n.key.path().cloned())
            .filter(|p| p.starts_with(dir) && p != dir)
            .collect();
        let candidates: Vec<(NodeKey, NodeKind)> = self
            .graph
            .nodes
            .values()
            .filter(|n| n.key.path().is_some_and(|p| p.starts_with(dir) && p != dir))
            .filter(|n| match n.kind {
                NodeKind::Root => true,
                NodeKind::Additional => n
                    .key
                    .path()
                    .is_some_and(|p| !roots.iter().any(|r| p.starts_with(r))),
                NodeKind::File => !self.graph.edge_indices_from(&n.key).is_empty(),
                NodeKind::Directory
                | NodeKind::Document
                | NodeKind::Asset
                | NodeKind::Key
                | NodeKind::Missing => false,
            })
            .map(|n| (n.key.clone(), n.kind))
            .collect();
        let selecting = !self.query.selector.is_empty();
        let mut out = Vec::new();
        for (key, kind) in candidates {
            match kind {
                NodeKind::Root => {
                    let Some(path) = key.path() else { continue };
                    let entries = self.layout(&path.clone());
                    if selecting && entries.is_empty() {
                        continue;
                    }
                    out.push(Entry::Layout(Row {
                        node: key,
                        elsewhere: false,
                        entries,
                    }));
                }
                _ => {
                    if !selecting {
                        out.push(Self::line(key));
                    }
                }
            }
        }
        self.cap(out)
    }
}
