//! This module writes the result of `graph` as one Mermaid `flowchart LR`
//! diagram. The design-graph design describes this form.

use std::{
    collections::{BTreeSet, HashMap},
    io::{self, Write},
};

use serde_json::Value;
use sokf_core::{
    EdgeKind, Entry, Graph, GraphResult, Node, NodeKey, NodeKind, RepoPath, Row, graph::node_json,
};

/// `text` writes the result as one Mermaid flowchart: a subgraph for each
/// printed root and directory, the nodes numbered in the order of
/// `result.nodes`, one arrow for each declared edge, and, for a folded edge
/// that covers several regions of an include or the markers of one file,
/// one arrow labelled with their number.
// sokf-implements: CLI-7_AC-79
pub fn text<W: Write>(w: &mut W, graph: &Graph, result: &GraphResult) -> io::Result<()> {
    writeln!(w, "flowchart LR")?;
    let ids: HashMap<&NodeKey, usize> = result
        .nodes
        .iter()
        .enumerate()
        .map(|(i, k)| (k, i + 1))
        .collect();
    let is_container = |key: &NodeKey| {
        ids.contains_key(key)
            && matches!(graph.kind(key), Some(NodeKind::Root | NodeKind::Directory))
    };
    // Put each root, directory, document, additional file, and asset in the
    // subgraph of its nearest printed ancestor root or directory. Put
    // `file`, `missing`, and `key` nodes, and nodes with no printed
    // ancestor, at the top level.
    let mut children: HashMap<Option<NodeKey>, Vec<&NodeKey>> = HashMap::new();
    for key in &result.nodes {
        let mut parent = None;
        if matches!(
            graph.kind(key),
            Some(
                NodeKind::Root
                    | NodeKind::Directory
                    | NodeKind::Document
                    | NodeKind::Additional
                    | NodeKind::Asset
            )
        ) {
            let mut up = Graph::parent(key);
            while let Some(p) = up {
                if is_container(&p) {
                    parent = Some(p);
                    break;
                }
                up = Graph::parent(&p);
            }
        }
        children.entry(parent).or_default().push(key);
    }
    declare(w, graph, &children, &ids, &result.start, None, 1)?;
    // Draw each printed edge once, in the direction in which it was
    // declared. `text` draws a printed synthesised edge as the declared
    // edge that it inverts, so that a declared edge appears once when the
    // tree also prints its inverse.
    let mut printed = BTreeSet::new();
    walk(&result.rows, &mut printed);
    let drawn: BTreeSet<usize> = printed
        .iter()
        .flat_map(|&i| {
            let e = &graph.edges[i];
            if e.synthesised {
                graph.inverse_indices(e).collect::<Vec<_>>()
            } else {
                vec![i]
            }
        })
        .collect();
    for &i in &result.edges {
        let e = &graph.edges[i];
        if e.synthesised || !drawn.contains(&i) {
            continue;
        }
        let (Some(from), Some(to)) = (ids.get(&e.from), ids.get(&e.to)) else {
            continue;
        };
        let arrow = if e.kind == EdgeKind::Links {
            "-->"
        } else {
            "-.->"
        };
        let mut label = escape(&e.rel);
        match (result.folded.get(&i), &e.region) {
            (Some(n), _) => {
                label.push_str(&format!(" ({})", super::tree::folded_count(e.kind, *n)))
            }
            (None, Some(region)) => label.push_str(&format!(" #{}", escape(region))),
            (None, None) => {}
        }
        writeln!(w, "  n{from} {arrow}|\"{label}\"| n{to}")?;
    }
    Ok(())
}

/// `walk` adds to `out` the index of the edge on each edge line of the
/// tree, including the lines inside layout rows and expanded edges.
fn walk(rows: &[Row], out: &mut BTreeSet<usize>) {
    fn entries(es: &[Entry], out: &mut BTreeSet<usize>) {
        for e in es {
            match e {
                Entry::Layout(r) => entries(&r.entries, out),
                Entry::Edge { edge, expanded, .. } => {
                    out.insert(*edge);
                    entries(expanded, out);
                }
                Entry::More { .. } => {}
            }
        }
    }
    for r in rows {
        entries(&r.entries, out);
    }
}

fn declare<W: Write>(
    w: &mut W,
    graph: &Graph,
    children: &HashMap<Option<NodeKey>, Vec<&NodeKey>>,
    ids: &HashMap<&NodeKey, usize>,
    starts: &[NodeKey],
    parent: Option<NodeKey>,
    depth: usize,
) -> io::Result<()> {
    let indent = "  ".repeat(depth);
    for &key in children.get(&parent).map(Vec::as_slice).unwrap_or(&[]) {
        let Some(node) = graph.node(key) else {
            continue;
        };
        let id = ids[key];
        match node.kind {
            NodeKind::Root | NodeKind::Directory => {
                // Label the subgraph as the text form labels its line: a root
                // or a start node by its path, and any other directory by its
                // name followed by `/`.
                let is_start = starts.contains(key);
                let text = if node.kind == NodeKind::Root || is_start {
                    key.as_str().to_string()
                } else {
                    format!(
                        "{}/",
                        key.path().map(RepoPath::file_name).unwrap_or_default()
                    )
                };
                writeln!(w, "{indent}subgraph n{id}[{}]", quote(&text))?;
                declare(
                    w,
                    graph,
                    children,
                    ids,
                    starts,
                    Some(key.clone()),
                    depth + 1,
                )?;
                writeln!(w, "{indent}end")?;
            }
            NodeKind::Document => writeln!(w, "{indent}n{id}[\"{}\"]", label(node))?,
            NodeKind::Additional => writeln!(w, "{indent}n{id}([{}])", quote(key.as_str()))?,
            NodeKind::Asset => {
                let sidecar = node.fields.get("sidecar").and_then(Value::as_str);
                writeln!(
                    w,
                    "{indent}n{id}[({})]",
                    quote(sidecar.unwrap_or(key.as_str()))
                )?;
            }
            NodeKind::Key => {
                let text = node.fields.get("key").and_then(Value::as_str);
                writeln!(
                    w,
                    "{indent}n{id}(({}))",
                    quote(text.unwrap_or(key.as_str()))
                )?;
            }
            NodeKind::File => writeln!(w, "{indent}n{id}[/{}/]", quote(key.as_str()))?,
            NodeKind::Missing => writeln!(w, "{indent}n{id}{{{{{}}}}}", quote(key.as_str()))?,
        }
    }
    Ok(())
}

/// `label` returns a document's escaped label, `<id><br/><title>`, or only
/// the id when the document has no title. A broken document that has no
/// `id` is labelled by its path.
fn label(node: &Node) -> String {
    let json = node_json(node);
    let text = |v: &Value| match v {
        Value::String(s) => escape(s),
        other => escape(&other.to_string()),
    };
    let Some(id) = json.get("id") else {
        return escape(node.key.as_str());
    };
    match json.get("title").filter(|t| !t.is_null()) {
        Some(title) => format!("{}<br/>{}", text(id), text(title)),
        None => text(id),
    }
}

/// `escape` prepares text for a Mermaid label: it writes `&`, `<`, `>`, and
/// `"` as entity codes, so that a title can't open a tag or close the quote.
pub fn escape(s: &str) -> String {
    s.replace('&', "#amp;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
        .replace('"', "#quot;")
}

/// `quote` returns the text escaped and in double quotes, as a Mermaid
/// label.
pub fn quote(s: &str) -> String {
    format!("\"{}\"", escape(s))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;
    use serde_json::json;

    use super::*;
    use crate::render::fixtures::{edge, key, node};

    fn graph() -> Graph {
        let nodes: BTreeMap<NodeKey, Node> = [
            node("/k", NodeKind::Root, json!({})),
            node(
                "/k/a.md",
                NodeKind::Document,
                json!({"id": "a", "title": "Say \"hi\""}),
            ),
            node("/k/core", NodeKind::Directory, json!({})),
            node(
                "/k/core/b.md",
                NodeKind::Document,
                json!({"id": "b", "title": null}),
            ),
            node("/k/core/deep", NodeKind::Directory, json!({})),
            node(
                "/k/core/deep/c.md",
                NodeKind::Document,
                json!({"broken": true, "title": "<c> & \"d\""}),
            ),
            node("/README.md", NodeKind::Additional, json!({})),
            node(
                "/k/core/pic.png",
                NodeKind::Asset,
                json!({"sidecar": "pic"}),
            ),
            node("/src/x.rs", NodeKind::File, json!({})),
            node("gone", NodeKind::Missing, json!({})),
        ]
        .into_iter()
        .collect();
        Graph::new(
            nodes,
            vec![
                edge(
                    "/k/a.md",
                    EdgeKind::Links,
                    "/k/core/b.md",
                    "depends-on",
                    None,
                    false,
                ),
                edge(
                    "/k/core/b.md",
                    EdgeKind::Links,
                    "/k/a.md",
                    "depended-on-by",
                    None,
                    true,
                ),
                edge(
                    "/k/core/b.md",
                    EdgeKind::Includes,
                    "/src/x.rs",
                    "includes",
                    Some("cli"),
                    false,
                ),
                edge(
                    "/src/x.rs",
                    EdgeKind::Includes,
                    "/k/core/b.md",
                    "included-by",
                    Some("cli"),
                    true,
                ),
                edge(
                    "/k/core/b.md",
                    EdgeKind::Links,
                    "gone",
                    "references",
                    None,
                    false,
                ),
                edge(
                    "gone",
                    EdgeKind::Links,
                    "/k/core/b.md",
                    "referenced-by",
                    None,
                    true,
                ),
            ],
        )
    }

    /// `printed` returns one row with an edge line for each edge whose index
    /// is in `edges`.
    fn printed(edges: std::ops::Range<usize>) -> Vec<Row> {
        vec![Row {
            node: key("/k"),
            elsewhere: false,
            entries: edges
                .map(|edge| Entry::Edge {
                    edge,
                    target: key("/k"),
                    expanded: vec![],
                    elsewhere: false,
                    folded: 0,
                    via: false,
                })
                .collect(),
        }]
    }

    #[test]
    fn nests_subgraphs_numbers_nodes_and_draws_declared_edges_once() {
        let g = graph();
        // `/k/core` isn't printed, so `b.md` and `deep/` appear in the subgraph
        // of `/k`. `/k` is a root, labelled by its path, and `/k/core/deep`
        // is a directory under it, labelled `deep/`. A null title counts as
        // no title, and `<`, `&`, and `"` are escaped.
        let result = GraphResult {
            start: vec![],
            rows: printed(0..6),
            nodes: vec![
                key("/README.md"),
                key("/k"),
                key("/k/a.md"),
                key("/k/core/b.md"),
                key("/k/core/deep"),
                key("/k/core/deep/c.md"),
                key("/k/core/pic.png"),
                key("/src/x.rs"),
                key("gone"),
            ],
            edges: (0..6).collect(),
            folded: Default::default(),
            summary: Default::default(),
            projection: None,
        };
        let mut buf = Vec::new();
        text(&mut buf, &g, &result).unwrap();
        // An asset is drawn as a cylinder in the subgraph of its nearest
        // printed directory, labelled by its sidecar's id.
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "flowchart LR\n\
             \x20 n1([\"/README.md\"])\n\
             \x20 subgraph n2[\"/k\"]\n\
             \x20   n3[\"a<br/>Say #quot;hi#quot;\"]\n\
             \x20   n4[\"b\"]\n\
             \x20   subgraph n5[\"deep/\"]\n\
             \x20     n6[\"/k/core/deep/c.md\"]\n\
             \x20   end\n\
             \x20   n7[(\"pic\")]\n\
             \x20 end\n\
             \x20 n8[/\"/src/x.rs\"/]\n\
             \x20 n9{{\"gone\"}}\n\
             \x20 n3 -->|\"depends-on\"| n4\n\
             \x20 n4 -.->|\"includes #cli\"| n8\n\
             \x20 n4 -->|\"references\"| n9\n"
        );
        // A directory that is a start node is labelled by its path, as its
        // text line is.
        let started = GraphResult {
            start: vec![key("/k/core/deep")],
            rows: vec![],
            nodes: vec![key("/k/core/deep"), key("/k/core/deep/c.md")],
            edges: vec![],
            folded: Default::default(),
            summary: Default::default(),
            projection: None,
        };
        let mut buf = Vec::new();
        text(&mut buf, &g, &started).unwrap();
        assert!(
            String::from_utf8(buf)
                .unwrap()
                .contains("subgraph n1[\"/k/core/deep\"]\n    n2[\"/k/core/deep/c.md\"]\n"),
        );
        assert_eq!(escape("a<b>&\"c\""), "a#lt;b#gt;#amp;#quot;c#quot;");
    }

    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn identifiers_follow_node_order_and_every_label_is_quoted_and_escaped(
            order in prop::collection::vec(0usize..10, 1..10),
            title in "[a-zA-Z<>&\" ]{0,10}",
            rel in "[a-z<>&\"-]{1,8}",
        ) {
            let mut g = graph();
            if let Some(n) = g.nodes.get_mut(&key("/k/a.md")) {
                n.fields.insert("title".into(), json!(title));
            }
            g.edges[0].rel = rel.clone();
            // Build `result.nodes` in an arbitrary order, without repeats.
            let all: Vec<NodeKey> = g.nodes.keys().cloned().collect();
            let mut nodes: Vec<NodeKey> = Vec::new();
            for &i in &order {
                if !nodes.contains(&all[i]) {
                    nodes.push(all[i].clone());
                }
            }
            let result = GraphResult { start: vec![], rows: printed(0..6), nodes: nodes.clone(), edges: (0..6).collect(), folded: Default::default(), summary: Default::default(), projection: None };
            let mut buf = Vec::new();
            text(&mut buf, &g, &result).unwrap();
            let out = String::from_utf8(buf).unwrap();
            let mut declared: Vec<usize> = Vec::new();
            for line in out.lines().skip(1) {
                let t = line.trim_start();
                if t == "end" {
                    continue;
                }
                // Each line holds exactly one quoted label, so it has two `"`
                // marks, and the label holds no raw `"`, `<`, `>`, or `&`
                // other than in the `<br/>` separator.
                prop_assert_eq!(t.matches('"').count(), 2, "{}", t);
                let label = &t[t.find('"').unwrap() + 1..t.rfind('"').unwrap()];
                let inner = label.replace("<br/>", "");
                prop_assert!(!inner.contains(['"', '<', '>', '&']), "{}", t);
                if t.contains("-->") || t.contains("-.->") {
                    // An edge line refers to both of its endpoints by number.
                    let ends: Vec<usize> = t
                        .split(|c: char| !c.is_ascii_digit())
                        .filter(|d| !d.is_empty())
                        .filter_map(|d| d.parse().ok())
                        .collect();
                    prop_assert!(ends.len() >= 2, "{}", t);
                    continue;
                }
                let rest = t.strip_prefix("subgraph ").unwrap_or(t).strip_prefix('n').unwrap();
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                let k: usize = digits.parse().unwrap();
                prop_assert!(!declared.contains(&k), "{}", out);
                // `n<k>` is the k-th node of `result.nodes`, so its label is
                // that node's label.
                let node = g.node(&nodes[k - 1]).unwrap();
                let expected = match node.kind {
                    NodeKind::Document => super::label(node),
                    NodeKind::Directory => format!("{}/", node.key.path().unwrap().file_name()),
                    NodeKind::Asset => escape(node.fields.get("sidecar").and_then(Value::as_str).unwrap_or(node.key.as_str())),
                    _ => escape(node.key.as_str()),
                };
                prop_assert_eq!(label, expected.as_str(), "{}", t);
                declared.push(k);
            }
            declared.sort_unstable();
            prop_assert_eq!(declared, (1..=nodes.len()).collect::<Vec<_>>());
            // The label of the edge from `a.md` to `b.md` is its escaped rel.
            if nodes.contains(&key("/k/a.md")) && nodes.contains(&key("/k/core/b.md")) {
                prop_assert!(out.contains(&format!("|\"{}\"|", escape(&rel))), "{}", out);
            }
        }
    }
}
