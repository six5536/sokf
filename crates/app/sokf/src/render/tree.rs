//! This module writes the output of `graph` as text, in the form that the
//! Text form section of design-graph describes. It draws each tree with the
//! branch characters `├──` and `└──`, as the `tree` command does.

use std::io::{self, Write};

use serde_json::Value;
use sokf_core::{EdgeKind, Entry, Graph, GraphResult, NodeKey, NodeKind, Row, graph::node_json};

use super::{Template, template::path_or_target};

/// `text` writes each starting node of `result` with its tree below it, and
/// then the line of counts.
// sokf-implements: CLI-7_AC-77
pub fn text<W: Write>(
    w: &mut W,
    graph: &Graph,
    result: &GraphResult,
    template: &Template,
) -> io::Result<()> {
    for row in &result.rows {
        let line = match graph.kind(&row.node) {
            Some(NodeKind::Root) => root_line(graph, &row.node),
            // A starting node that is a directory or an asset prints as its path.
            Some(NodeKind::Directory | NodeKind::Asset) => row.node.as_str().to_string(),
            _ => node_line(graph, &row.node, template),
        };
        writeln!(w, "{line}{}", star(row.elsewhere))?;
        entries(w, graph, &row.entries, "", template)?;
    }
    // A blank line sets the counts apart from the tree.
    if !result.rows.is_empty() {
        writeln!(w)?;
    }
    writeln!(w, "{}", result.summary.line())
}

// sokf-implements: CLI-7_AC-26
fn entries<W: Write>(
    w: &mut W,
    graph: &Graph,
    entries: &[Entry],
    prefix: &str,
    template: &Template,
) -> io::Result<()> {
    let count = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let last = i + 1 == count;
        let branch = if last { "└── " } else { "├── " };
        let below = format!("{prefix}{}", if last { "    " } else { "│   " });
        match entry {
            Entry::Layout(row) => {
                writeln!(
                    w,
                    "{prefix}{branch}{}{}",
                    layout_line(graph, row, template),
                    star(row.elsewhere)
                )?;
                self::entries(w, graph, &row.entries, &below, template)?;
            }
            Entry::Edge {
                edge,
                target,
                expanded,
                elsewhere,
                folded,
                via,
            } => {
                let line = edge_line(graph, *edge, target, *folded, *via, template);
                writeln!(w, "{prefix}{branch}{line}{}", star(*elsewhere))?;
                self::entries(w, graph, expanded, &below, template)?;
            }
            // sokf-implements: CLI-7_AC-69
            Entry::More { rel, count } => {
                let what = match rel {
                    Some(rel) => format!("{count} more {rel}"),
                    None => format!("{count} more"),
                };
                writeln!(w, "{prefix}{branch}… {what} (--all)")?;
            }
        }
    }
    Ok(())
}

/// `edge_line` returns the line of an edge, `<rel> → <target>`. A region
/// is a part of an included file, or the key that a code marker cites. On
/// the declared edge the region follows the target after `#`, and on its
/// inverse it follows the `rel`. A folded edge, which stands for the
/// includes of two or more regions of one file, or for the markers of one
/// file under a document, shows its count in place of a region. A marker
/// edge that isn't folded ends with `(line <n>)`, the line of the marker
/// in its file. When the edge comes from a directory that holds the file of
/// the row, the node that the tree prints the line under, the line ends
/// with `(via <directory>)`. On a key's `defined-in` line, `(line <n>)`
/// gives the number of the line that defines the key.
// sokf-implements: CLI-7_AC-90
// sokf-implements: CLI-7_AC-93
fn edge_line(
    graph: &Graph,
    edge: usize,
    target: &NodeKey,
    folded: usize,
    via: bool,
    template: &Template,
) -> String {
    let e = &graph.edges[edge];
    let region = e
        .region
        .as_deref()
        .filter(|_| matches!(e.kind, EdgeKind::Includes | EdgeKind::Marker) && folded == 0);
    // An include's region follows its target, and an `included-by`'s its
    // `rel`. A marker's region follows its target, and its inverse's `rel`.
    let after_rel = match e.kind {
        EdgeKind::Includes => e.rel != "includes",
        _ => e.synthesised,
    };
    let rel = match region {
        Some(region) if after_rel => format!("{} #{region}", e.rel),
        _ => e.rel.clone(),
    };
    let mut line = format!("{rel} → {}", node_line(graph, target, template));
    if !after_rel && let Some(region) = region {
        line.push('#');
        line.push_str(region);
    }
    // sokf-implements: CLI-7_AC-70
    // sokf-implements: CLI-7_AC-91
    if folded > 0 {
        line.push_str(&format!(" ({})", folded_count(e.kind, folded)));
    } else if e.kind == EdgeKind::Marker
        && let Some(n) = e.line
    {
        line.push_str(&format!(" (line {n})"));
    }
    if via {
        line.push_str(&format!(" (via {})", e.from.as_str()));
    }
    if e.kind == EdgeKind::Keys
        && e.rel == "defined-in"
        && let Some(n) = graph
            .node(&e.from)
            .and_then(|n| n.fields.get("line"))
            .and_then(Value::as_u64)
    {
        line.push_str(&format!(" (line {n})"));
    }
    line
}

/// `folded_count` returns the count of a folded edge, such as `3 regions`
/// or `1 marker`.
pub(crate) fn folded_count(kind: EdgeKind, n: usize) -> String {
    match (kind, n) {
        (EdgeKind::Marker, 1) => "1 marker".to_string(),
        (EdgeKind::Marker, n) => format!("{n} markers"),
        (_, n) => format!("{n} regions"),
    }
}

/// `root_line` returns the path of a root, followed by ` - ` and the
/// description of its manifest when the manifest has one.
// sokf-implements: CLI-7_AC-66
fn root_line(graph: &Graph, key: &NodeKey) -> String {
    match graph
        .node(key)
        .and_then(|n| n.fields.get("description"))
        .and_then(Value::as_str)
    {
        Some(description) => format!("{} - {description}", key.as_str()),
        None => key.as_str().to_string(),
    }
}

/// `layout_line` returns the line of a node in a layout: a root as
/// `root_line` writes it, a directory as its name followed by `/`, and any
/// other node as `node_line` writes it.
fn layout_line(graph: &Graph, row: &Row, template: &Template) -> String {
    match graph.kind(&row.node) {
        Some(NodeKind::Root) => root_line(graph, &row.node),
        Some(NodeKind::Directory) => format!(
            "{}/",
            row.node.path().map(|p| p.file_name()).unwrap_or_default()
        ),
        _ => node_line(graph, &row.node, template),
    }
}

/// `node_line` returns the line of a starting node or a target node: a
/// document through the template, an asset as the id of its sidecar, a key
/// as it is written, and any other node as its path or target. The line of
/// a broken node ends with `[broken: <reason>]`.
// sokf-implements: CLI-7_AC-18
// sokf-implements: CLI-7_AC-25
// sokf-implements: CLI-7_AC-85
fn node_line(graph: &Graph, key: &NodeKey, template: &Template) -> String {
    let Some(node) = graph.node(key) else {
        return key.as_str().to_string();
    };
    let json = node_json(node);
    let field = |name: &str| json.get(name).and_then(Value::as_str).map(str::to_string);
    let mut line = match node.kind {
        NodeKind::Document => template.render(&json),
        NodeKind::Asset => field("sidecar").unwrap_or_else(|| path_or_target(&json)),
        NodeKind::Key => field("key").unwrap_or_else(|| path_or_target(&json)),
        _ => path_or_target(&json),
    };
    if json.get("broken") == Some(&Value::Bool(true)) {
        let reason = json
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or_default();
        line.push_str(&format!(" [broken: {reason}]"));
    }
    line
}

fn star(elsewhere: bool) -> &'static str {
    if elsewhere { " (*)" } else { "" }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use sokf_core::Node;

    use super::*;
    use crate::render::fixtures::{edge, key, node};

    fn graph() -> Graph {
        let nodes: BTreeMap<NodeKey, Node> = [
            node("/k", NodeKind::Root, json!({"name": "k"})),
            node("/k/core", NodeKind::Directory, json!({})),
            node(
                "/k/core/a.md",
                NodeKind::Document,
                json!({"id": "a", "title": "A"}),
            ),
            node(
                "/k/b.md",
                NodeKind::Document,
                json!({"broken": true, "reason": "no id"}),
            ),
            node("/README.md", NodeKind::Additional, json!({})),
            node("/src/x.rs", NodeKind::File, json!({})),
            node("gone", NodeKind::Missing, json!({})),
        ]
        .into_iter()
        .collect();
        Graph::new(
            nodes,
            vec![
                edge(
                    "/k/core/a.md",
                    EdgeKind::Includes,
                    "/src/x.rs",
                    "includes",
                    Some("cli"),
                    false,
                ),
                edge(
                    "/k/core/a.md",
                    EdgeKind::Links,
                    "gone",
                    "references",
                    None,
                    false,
                ),
                edge(
                    "/src/x.rs",
                    EdgeKind::Includes,
                    "/k/core/a.md",
                    "included-by",
                    Some("cli"),
                    false,
                ),
            ],
        )
    }

    fn row(s: &str, elsewhere: bool, entries: Vec<Entry>) -> Row {
        Row {
            node: key(s),
            elsewhere,
            entries,
        }
    }

    fn edge_entry(edge: usize, target: &str, expanded: Vec<Entry>, elsewhere: bool) -> Entry {
        Entry::Edge {
            edge,
            target: key(target),
            expanded,
            elsewhere,
            folded: 0,
            via: false,
        }
    }

    // sokf-tests: CLI-7_AC-26
    #[test]
    fn prints_the_prefixes_lines_regions_stars_and_broken_markers() {
        let g = graph();
        let result = GraphResult {
            start: vec![],
            rows: vec![
                row(
                    "/k",
                    false,
                    vec![
                        Entry::Layout(row("/README.md", false, vec![])),
                        Entry::Layout(row(
                            "/k/b.md",
                            false,
                            vec![edge_entry(2, "/k/core/a.md", vec![], true)],
                        )),
                        Entry::Layout(row(
                            "/k/core",
                            false,
                            vec![Entry::Layout(row(
                                "/k/core/a.md",
                                false,
                                vec![
                                    edge_entry(
                                        0,
                                        "/src/x.rs",
                                        vec![edge_entry(2, "/k/core/a.md", vec![], true)],
                                        false,
                                    ),
                                    edge_entry(1, "gone", vec![], false),
                                ],
                            ))],
                        )),
                    ],
                ),
                row("/k/core/a.md", true, vec![]),
                row("/src/x.rs", false, vec![]),
            ],
            nodes: vec![],
            edges: vec![],
            folded: Default::default(),
            summary: Default::default(),
            projection: None,
        };
        let mut buf = Vec::new();
        text(
            &mut buf,
            &g,
            &result,
            &Template::parse("{id} {title}").unwrap(),
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "/k\n\
             ├── /README.md\n\
             ├── /k/b.md  [broken: no id]\n\
             │   └── included-by #cli → a A (*)\n\
             └── core/\n\
             \x20   └── a A\n\
             \x20       ├── includes → /src/x.rs#cli\n\
             \x20       │   └── included-by #cli → a A (*)\n\
             \x20       └── references → gone\n\
             a A (*)\n\
             /src/x.rs\n\
             \n\
             0 / 0 docs\n"
        );
        // An unknown key prints as itself.
        assert_eq!(
            node_line(&g, &key("/nope"), &Template::parse("{id}").unwrap()),
            "/nope"
        );
    }
}
