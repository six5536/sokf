//! This module builds the include graph from the source markers, never from
//! materialised content (SOKF_FORMAT_91).

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{Model, RepoPath, markdown::IncludeTarget};

/// `Graph` is the include graph. Each document and each additional file is a
/// node, and each open marker that includes a document's body is an edge. A
/// marker with a named fragment is not an edge, and neither is a marker that
/// includes an asset by its sidecar's id with no fragment, because the asset
/// is a file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Graph {
    /// The path of each node.
    pub nodes: Vec<RepoPath>,
    /// The edges, each from the including node to the included node.
    pub edges: Vec<(usize, usize)>,
}

/// `Order` is the order of a graph. It holds two things: the nodes that aren't
/// on a cycle, in topological order with the leaves first, and the cycles.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Order {
    /// The indices of the nodes that aren't on a cycle, each after every node
    /// it includes.
    pub sorted: Vec<usize>,
    /// Every cycle, as the sorted paths of its nodes.
    pub cycles: Vec<Vec<RepoPath>>,
}

/// `knowledge_of_additional` returns the knowledge in which the includes of an
/// additional file resolve: the knowledge of its schema, or else the knowledge
/// that holds the file.
pub fn knowledge_of_additional(
    model: &Model,
    file: &crate::model::AdditionalFile,
) -> Option<usize> {
    file.schema
        .map(|s| s.knowledge)
        .or_else(|| model.knowledge_of(&file.path))
}

/// `graph_for` builds the include graph of every document, schema, and
/// additional file, with every node. With `only`, it adds the edges from a
/// set of nodes: the paths in `only` and every node they include, directly
/// or indirectly. It stops when no new node joins that set, and returns the
/// set too. With `None`, it adds every edge. A run that loads a document but
/// doesn't expand it resolves none of that document's includes, as the cache
/// contract requires.
// sokf-implements: SOKF_FORMAT_91
pub(super) fn graph_for(
    model: &Model,
    only: Option<&BTreeSet<RepoPath>>,
) -> (Graph, Option<HashSet<usize>>) {
    let mut g = Graph::default();
    let mut index: HashMap<RepoPath, usize> = HashMap::new();
    let mut sources: Vec<(usize, Option<usize>, Vec<IncludedId>)> = Vec::new();
    let documents = model
        .documents_and_schemas()
        .map(|(k, doc)| (&doc.path, Some(k), &doc.segments));
    let additional = model
        .additional
        .iter()
        .map(|f| (&f.path, knowledge_of_additional(model, f), &f.segments));
    for (path, knowledge, segments) in documents.chain(additional) {
        let i = g.nodes.len();
        index.insert(path.clone(), i);
        g.nodes.push(path.clone());
        sources.push((i, knowledge, included_ids(segments)));
    }
    let mut wanted: Option<HashSet<usize>> = only.map(|o| {
        o.iter()
            .filter_map(|p| index.get(p).copied())
            .collect::<HashSet<usize>>()
    });
    let mut done: HashSet<usize> = HashSet::new();
    loop {
        let mut grew = false;
        for (from, knowledge, ids) in &sources {
            if done.contains(from) || wanted.as_ref().is_some_and(|w| !w.contains(from)) {
                continue;
            }
            done.insert(*from);
            let Some(k) = knowledge else { continue };
            for (store, id, fragment) in ids {
                if let Some((_, target)) = model.document_in(*k, store.as_deref(), id)
                    && includes_body(model, &target.path, fragment.as_deref())
                    && let Some(&to) = index.get(&target.path)
                {
                    g.edges.push((*from, to));
                    if let Some(w) = wanted.as_mut()
                        && w.insert(to)
                    {
                        grew = true;
                    }
                }
            }
        }
        if !grew {
            break;
        }
    }
    (g, wanted)
}

/// `includes_body` says whether an id-form include of `target` with
/// `fragment` materialises the document's body (SOKF_FORMAT_85,
/// FORMAT-2_AC-5). An include with an empty fragment includes the body of any
/// document, an include with no fragment includes the body of a document that
/// isn't an asset's sidecar, and an include with a named fragment never
/// includes the body.
// sokf-implements: FORMAT-2_AC-5
pub fn includes_body(model: &Model, target: &RepoPath, fragment: Option<&str>) -> bool {
    match fragment {
        Some("") => true,
        Some(_) => false,
        None => model.asset_of(target).is_none(),
    }
}

/// An `IncludedId` holds the store, id, and fragment of an include by id.
type IncludedId = (Option<String>, String, Option<String>);

fn included_ids(segments: &crate::Segments) -> Vec<IncludedId> {
    segments
        .regions
        .iter()
        .filter_map(|i| match &IncludeTarget::parse(&i.target) {
            IncludeTarget::Document {
                store,
                id,
                fragment,
            } => Some((store.clone(), id.clone(), fragment.clone())),
            IncludeTarget::File { .. } => None,
        })
        .collect()
}

/// `order` orders the graph with Tarjan's algorithm for strongly connected
/// components. The algorithm emits the components in reverse topological
/// order, so every node follows the nodes it includes.
pub(super) fn order(graph: &Graph) -> Order {
    let n = graph.nodes.len();
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(from, to) in &graph.edges {
        adjacency[from].push(to);
    }
    let mut state = Tarjan {
        adjacency: &adjacency,
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        components: Vec::new(),
    };
    for v in 0..n {
        if state.index[v].is_none() {
            state.visit(v);
        }
    }
    let mut out = Order::default();
    for component in state.components {
        let self_loop = component.len() == 1 && adjacency[component[0]].contains(&component[0]);
        if component.len() > 1 || self_loop {
            let mut paths: Vec<RepoPath> =
                component.iter().map(|&i| graph.nodes[i].clone()).collect();
            paths.sort();
            out.cycles.push(paths);
        } else {
            out.sorted.push(component[0]);
        }
    }
    out.cycles.sort();
    out
}

struct Tarjan<'a> {
    adjacency: &'a [Vec<usize>],
    index: Vec<Option<usize>>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    next: usize,
    components: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, v: usize) {
        self.index[v] = Some(self.next);
        self.low[v] = self.next;
        self.next += 1;
        self.stack.push(v);
        self.on_stack[v] = true;
        for &w in &self.adjacency[v] {
            match self.index[w] {
                None => {
                    self.visit(w);
                    self.low[v] = self.low[v].min(self.low[w]);
                }
                Some(i) if self.on_stack[w] => self.low[v] = self.low[v].min(i),
                Some(_) => {}
            }
        }
        if self.low[v] == self.index[v].expect("set earlier") {
            let mut component = Vec::new();
            loop {
                let w = self.stack.pop().expect("v is on the stack");
                self.on_stack[w] = false;
                component.push(w);
                if w == v {
                    break;
                }
            }
            component.sort_unstable();
            self.components.push(component);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn g(nodes: &[&str], edges: &[(usize, usize)]) -> Graph {
        Graph {
            nodes: nodes.iter().map(|n| p(n)).collect(),
            edges: edges.to_vec(),
        }
    }

    #[test]
    fn orders_leaves_first_and_finds_cycles() {
        let graph = g(
            &["/a", "/b", "/c", "/d", "/e", "/f"],
            &[(0, 1), (1, 2), (3, 4), (4, 3), (5, 5)],
        );
        let o = order(&graph);
        let pos = |i: usize| o.sorted.iter().position(|&x| x == i).unwrap();
        assert!(pos(2) < pos(1) && pos(1) < pos(0));
        assert_eq!(o.sorted.len(), 3);
        assert_eq!(o.cycles, vec![vec![p("/d"), p("/e")], vec![p("/f")]]);
        let empty = order(&Graph::default());
        assert!(empty.sorted.is_empty() && empty.cycles.is_empty());
    }
}
