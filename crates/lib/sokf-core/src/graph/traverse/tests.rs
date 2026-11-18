use std::path::PathBuf;

use super::*;
use crate::{
    graph::build,
    test_support::{fresh_dir, model_at, write},
};

fn temp(name: &str) -> PathBuf {
    let dir = fresh_dir(&format!("traverse-{name}"));
    for (rel, text) in [
        (
            "knowledge/manifest.sokf.yaml",
            "sokf: \"0.7\"\nname: k\nscope: test\n",
        ),
        (
            "knowledge/a.md",
            "---\ntype: Note\nid: a\ntags: [x]\nlinks:\n  - rel: depends-on\n    to: b\n---\n\n# A\n\n[b][sokf:b]\n",
        ),
        (
            "knowledge/sub/b.md",
            "---\ntype: Note\nid: b\nlinks:\n  - rel: depends-on\n    to: c\n---\n\n# B\n\n[c][sokf:c]\n",
        ),
        (
            "knowledge/sub/c.md",
            "---\ntype: Task\nid: c\nresource: /src/c.rs\nlinks:\n  - rel: depends-on\n    to: a\n  - rel: references\n    to: gone\n---\n\n# C\n\n[a][sokf:a]\n",
        ),
        (
            "knowledge/schemas/note.md",
            "---\ntype: Schema\nid: note\n---\n\n````yaml\nline-limit: 0\nadditional-files: '/docs/*.md'\nfrontmatter:\n  type:\n    const: Note\nsections: []\n````\n",
        ),
        (
            "docs/guide.md",
            "# Guide\n\n<!-- sokf:include a -->\n<!-- /sokf:include -->\n",
        ),
        ("src/c.rs", ""),
        ("src/plain.rs", ""),
    ] {
        write(&dir, rel, text);
    }
    dir
}

fn model(dir: &Path) -> Model {
    model_at(dir, &["/knowledge"])
}

fn query(start: &[&str], depth: usize) -> GraphQuery {
    GraphQuery {
        start: start.iter().map(ToString::to_string).collect(),
        depth,
        ..GraphQuery::default()
    }
}

/// `lines` renders the tree as indented strings, one for each node, edge, or
/// `n more` line, with the `(*)`, `(n regions)`, and `(via dir)` marks.
fn lines(graph: &Graph, result: &GraphResult) -> Vec<String> {
    fn row(graph: &Graph, r: &Row, depth: usize, out: &mut Vec<String>) {
        let star = if r.elsewhere { " (*)" } else { "" };
        out.push(format!("{}{}{star}", "  ".repeat(depth), r.node.as_str()));
        entries(graph, &r.entries, depth + 1, out);
    }
    fn entries(graph: &Graph, es: &[Entry], depth: usize, out: &mut Vec<String>) {
        for e in es {
            match e {
                Entry::Layout(r) => row(graph, r, depth, out),
                Entry::Edge {
                    edge,
                    target,
                    expanded,
                    elsewhere,
                    folded,
                    via,
                } => {
                    let star = if *elsewhere { " (*)" } else { "" };
                    let folded = if *folded > 0 {
                        format!(" ({folded} regions)")
                    } else {
                        String::new()
                    };
                    let via = if *via {
                        format!(" (via {})", graph.edges[*edge].from.as_str())
                    } else {
                        String::new()
                    };
                    out.push(format!(
                        "{}{} -> {}{folded}{via}{star}",
                        "  ".repeat(depth),
                        graph.edges[*edge].rel,
                        target.as_str()
                    ));
                    entries(graph, expanded, depth + 1, out);
                }
                Entry::More { rel, count } => out.push(format!(
                    "{}{count} more{}",
                    "  ".repeat(depth),
                    rel.as_deref().map(|r| format!(" {r}")).unwrap_or_default()
                )),
            }
        }
    }
    let mut out = Vec::new();
    for r in &result.rows {
        row(graph, r, 0, &mut out);
    }
    out
}

// sokf-tests: CLI-7_AC-1
// sokf-tests: CLI-7_AC-2
// sokf-tests: CLI-7_AC-3
// sokf-tests: CLI-7_AC-34
#[test]
fn the_layout_has_no_edge_and_a_node_start_has_its_edges() {
    let dir = temp("layout");
    let m = model(&dir);
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&[], 1)).unwrap();
    assert_eq!(
        r.start,
        vec![NodeKey::Path(RepoPath::parse("/knowledge").unwrap())]
    );
    assert_eq!(
        lines(&g, &r),
        vec![
            "/knowledge",
            "  /knowledge/a.md",
            "  /knowledge/schemas",
            "    /knowledge/schemas/note.md",
            "  /knowledge/sub",
            "    /knowledge/sub/b.md",
            "    /knowledge/sub/c.md",
        ]
    );
    // The container's own node and every layout node are in the bounded set.
    let nodes: Vec<&str> = r.nodes.iter().map(NodeKey::as_str).collect();
    assert_eq!(
        nodes,
        vec![
            "/knowledge",
            "/knowledge/a.md",
            "/knowledge/schemas",
            "/knowledge/schemas/note.md",
            "/knowledge/sub",
            "/knowledge/sub/b.md",
            "/knowledge/sub/c.md",
        ]
    );
    assert_eq!(r.edges, [] as [usize; 0]);
    assert_eq!(
        r.summary,
        Summary {
            documents: Count { shown: 4, total: 4 },
            archived: Count::default(),
        }
    );
    // When the query asks for the layout's edges, the output prints each
    // layout line's edges under it.
    let deep = GraphQuery {
        layout_edges: true,
        ..query(&[], 1)
    };
    let r = run(&mut g, &m, None, &dir, &deep).unwrap();
    // b is also a line of the layout, so its edges print there, and the edge
    // from a to b is marked `(*)`.
    assert!(lines(&g, &r).contains(&"    depends-on -> /knowledge/sub/b.md (*)".to_string()));
    // When the starts are a container and a document, the document's edges
    // print under its row, and the container's layout lists it as a line.
    let r2 = run(&mut g, &m, None, &dir, &query(&["a", "knowledge"], 1)).unwrap();
    let shown = lines(&g, &r2);
    assert_eq!(
        shown[..3],
        [
            "/knowledge/a.md",
            "  included-by -> /docs/guide.md",
            "  depends-on -> /knowledge/sub/b.md",
        ]
    );
    assert!(
        shown.contains(&"  /knowledge/a.md".to_string()),
        "{shown:?}"
    );
    // A start given twice expands once: its second row is marked `(*)`.
    let r3 = run(&mut g, &m, None, &dir, &query(&["a", "a"], 1)).unwrap();
    assert_eq!(lines(&g, &r3).last().unwrap(), "/knowledge/a.md (*)");
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-6
// sokf-tests: CLI-7_AC-39
// sokf-tests: CLI-7_AC-40
// sokf-tests: CLI_P-3
#[test]
fn expansion_happens_once_below_the_depth_and_never_for_rows_or_missing() {
    let dir = temp("expansion");
    let m = model(&dir);
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&["a"], 3)).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/knowledge/a.md",
            "  included-by -> /docs/guide.md",
            "    includes -> /knowledge/a.md (*)",
            "  depends-on -> /knowledge/sub/b.md",
            "    depended-on-by -> /knowledge/a.md (*)",
            "    depends-on -> /knowledge/sub/c.md",
            "      depends-on -> /knowledge/a.md (*)",
            "      depended-on-by -> /knowledge/sub/b.md (*)",
            "      references -> gone",
            "      describes -> /src/c.rs",
            "  depended-on-by -> /knowledge/sub/c.md (*)",
        ]
    );
    // When the depth is larger than the number of nodes, the rule that expands
    // each node only once keeps the tree finite.
    let deep = run(&mut g, &m, None, &dir, &query(&["a"], 50)).unwrap();
    assert_eq!(lines(&g, &deep).len(), lines(&g, &r).len() + 1);
    // When a target expands at a later, shallower appearance, its earlier
    // appearance at the depth limit is marked `(*)`.
    let two = run(&mut g, &m, None, &dir, &query(&["a", "docs/guide.md"], 2)).unwrap();
    let shown = lines(&g, &two);
    assert!(
        shown.contains(&"  included-by -> /docs/guide.md (*)".to_string()),
        "{shown:?}"
    );
    assert!(shown.contains(&"/docs/guide.md".to_string()), "{shown:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-8
// sokf-tests: CLI-7_AC-9
// sokf-tests: CLI-7_AC-10
// sokf-tests: CLI-7_AC-11
// sokf-tests: CLI-7_AC-45
// sokf-tests: CLI-7_AC-46
// sokf-tests: CLI-7_AC-47
// sokf-tests: CLI-7_AC-48
#[test]
fn selectors_choose_rows_and_edge_filters_prune_at_every_hop() {
    let dir = temp("selectors");
    let m = model(&dir);
    let mut g = build(&m);
    let selected = GraphQuery {
        selector: vec![Expr::compile("type=='Task'").unwrap()],
        ..query(&[], 1)
    };
    let r = run(&mut g, &m, None, &dir, &selected).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec!["/knowledge", "  /knowledge/sub", "    /knowledge/sub/c.md"]
    );
    // When nothing matches, nothing is printed.
    let none = GraphQuery {
        selector: vec![Expr::compile("type=='Nope'").unwrap()],
        ..query(&[], 1)
    };
    let r = run(&mut g, &m, None, &dir, &none).unwrap();
    assert!(r.rows.is_empty() && r.nodes.is_empty() && r.edges.is_empty());
    // A node start is always printed, and an additional file matches no selector.
    let r = run(
        &mut g,
        &m,
        None,
        &dir,
        &GraphQuery {
            start: vec!["docs/guide.md".into()],
            ..none.clone()
        },
    )
    .unwrap();
    assert_eq!(lines(&g, &r)[0], "/docs/guide.md");
    // `--edge` and `--to` apply at every hop. A hidden target is dropped, not replaced.
    let filtered = GraphQuery {
        edge: vec![Expr::compile("rel=='depends-on'").unwrap()],
        to: vec![Expr::compile("type=='Note'").unwrap()],
        ..query(&["c"], 3)
    };
    let r = run(&mut g, &m, None, &dir, &filtered).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/knowledge/sub/c.md",
            "  depends-on -> /knowledge/a.md",
            "    depends-on -> /knowledge/sub/b.md",
        ]
    );
    // A predicate that fails to evaluate is false.
    let failing = GraphQuery {
        to: vec![Expr::compile("contains(tags, 'x')").unwrap()],
        ..query(&["c"], 1)
    };
    let r = run(&mut g, &m, None, &dir, &failing).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec!["/knowledge/sub/c.md", "  depends-on -> /knowledge/a.md"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_directory_an_edge_names_prints_its_edges_then_what_is_under_it() {
    let dir = temp("named-dir");
    std::fs::write(
        dir.join("knowledge/d.md"),
        "---\ntype: Note\nid: d\nresource: [/src, /docs]\n---\n\n# D\n",
    )
    .unwrap();
    let m = model(&dir);
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&["src"], 1)).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec!["/src", "  described-by -> /knowledge/d.md", "  /src/c.rs"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn outside_directories_file_starts_and_projections() {
    let dir = temp("outside");
    let m = model(&dir);
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&["."], 0)).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/",
            "  /docs/guide.md",
            "  /knowledge",
            "    /knowledge/a.md",
            "    /knowledge/schemas",
            "      /knowledge/schemas/note.md",
            "    /knowledge/sub",
            "      /knowledge/sub/b.md",
            "      /knowledge/sub/c.md",
            "  /src/c.rs",
        ]
    );
    assert_eq!(
        g.kind(&NodeKey::Path(RepoPath::root())),
        Some(NodeKind::Directory)
    );
    let r = run(
        &mut g,
        &m,
        None,
        &dir,
        &query(&["src/plain.rs", "src/c.rs"], 1),
    )
    .unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/src/plain.rs",
            "/src/c.rs",
            "  described-by -> /knowledge/sub/c.md"
        ]
    );
    let projected = GraphQuery {
        query: Some(Expr::compile("nodes[?kind=='document'].id").unwrap()),
        ..query(&[], 0)
    };
    let r = run(&mut g, &m, None, &dir, &projected).unwrap();
    assert_eq!(r.rows, [] as [crate::graph::traverse::Row; 0]);
    assert_eq!(
        r.projection,
        Some(serde_json::json!(["a", "note", "b", "c"]))
    );
    let bad = GraphQuery {
        start: vec!["nothing".into()],
        ..GraphQuery::default()
    };
    assert!(matches!(
        run(&mut g, &m, None, &dir, &bad),
        Err(crate::Error::Start(_))
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}

mod views;
