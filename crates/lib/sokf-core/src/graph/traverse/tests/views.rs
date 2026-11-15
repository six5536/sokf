//! The view tests run on the fixture that `views` writes: archived
//! documents, the limit of a group, folded includes, the documents that
//! describe a directory, keys, and the edges near the starts.

use std::fmt::Write as _;

use super::*;
use crate::graph::build::declared_edges;

/// `views` builds the fixture for the view tests. It writes 23 notes, n00 to
/// n22, in `knowledge/many`. `hub` declares a typed
/// `depends-on` link to n00, and its body links to n00 as well, which mirrors
/// that typed link. Its body also links to n01 through n22 and to old. It
/// includes the regions `one` and `two` of `/src/c.rs`, and the whole of
/// `/src/plain.rs`. `old` is a closed issue that links to hub. `d` declares
/// the directory `/src`, which holds `c.rs` and `plain.rs`, as its resource.
fn views(name: &str) -> (PathBuf, Model) {
    let dir = temp(name);
    let mut hub = String::from(
        "---\ntype: Note\nid: hub\nlinks:\n  - rel: depends-on\n    to: n00\n---\n\n# Hub\n\n[n00][sokf:n00] [old][sokf:old]\n",
    );
    for i in 0..23 {
        let name = format!("n{i:02}");
        std::fs::create_dir_all(dir.join("knowledge/many")).unwrap();
        std::fs::write(
            dir.join(format!("knowledge/many/{name}.md")),
            format!("---\ntype: Note\nid: {name}\n---\n\n# N\n"),
        )
        .unwrap();
        if i > 0 {
            let _ = writeln!(hub, "[{name}][sokf:{name}]");
        }
    }
    hub.push_str(
        "\n<!-- sokf:include /src/c.rs#one -->\n<!-- /sokf:include -->\n<!-- sokf:include /src/c.rs#two -->\n<!-- /sokf:include -->\n<!-- sokf:include /src/plain.rs -->\n<!-- /sokf:include -->\n",
    );
    std::fs::write(dir.join("knowledge/hub.md"), hub).unwrap();
    std::fs::create_dir_all(dir.join("knowledge/closed")).unwrap();
    std::fs::write(
        dir.join("knowledge/closed/old.md"),
        "---\ntype: Issue\nid: old\nstate: closed\n---\n\n# Old\n\n[hub][sokf:hub]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("knowledge/d.md"),
        "---\ntype: Note\nid: d\nresource: /src\n---\n\n# D\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/c.rs"),
        "// sokf:begin one\n// sokf:end one\n// sokf:begin two\n// sokf:end two\n",
    )
    .unwrap();
    let m = model(&dir);
    (dir, m)
}

// sokf-tests: CLI-7_AC-67
// sokf-tests: CLI-7_AC-68
// sokf-tests: CLI-7_AC-82
// sokf-tests: CLI-7_AC-77
#[test]
fn an_archived_document_and_its_edges_print_only_when_asked_or_named() {
    let (dir, m) = views("archive");
    let mut g = build(&m);
    g.set_archived([RepoPath::parse("/knowledge/closed/old.md").unwrap()]);
    // The layout leaves out the archived document and prunes the directory
    // that it empties.
    let r = run(&mut g, &m, None, &dir, &query(&[], 1)).unwrap();
    let shown = lines(&g, &r);
    assert!(!shown.iter().any(|l| l.contains("closed")), "{shown:?}");
    assert_eq!(r.summary.archived, Count { shown: 0, total: 1 });
    assert_eq!(r.summary.line(), "26 / 30 docs (0 / 1 archived)");
    // No edge ends at the archived document, and the filters see only the
    // documents that are not archived.
    let r = run(&mut g, &m, None, &dir, &query(&["hub"], 1)).unwrap();
    assert!(!lines(&g, &r).iter().any(|l| l.contains("old")));
    assert_eq!(r.summary.archived, Count { shown: 0, total: 1 });
    let to = GraphQuery {
        to: vec![Expr::compile("id=='old'").unwrap()],
        ..query(&["hub"], 1)
    };
    let r = run(&mut g, &m, None, &dir, &to).unwrap();
    assert_eq!(lines(&g, &r), vec!["/knowledge/hub.md"]);
    // `--archived` shows the archived document, and so does a query that
    // starts from it.
    let asked = GraphQuery {
        archived: true,
        ..query(&["hub"], 1)
    };
    let r = run(&mut g, &m, None, &dir, &asked).unwrap();
    assert!(lines(&g, &r).iter().any(|l| l.contains("closed/old.md")));
    assert_eq!(r.summary.archived, Count { shown: 1, total: 1 });
    let r = run(&mut g, &m, None, &dir, &query(&["old"], 1)).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/knowledge/closed/old.md",
            "  relates-to -> /knowledge/hub.md"
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-69
// sokf-tests: CLI-7_AC-70
// sokf-tests: CLI-7_AC-71
// sokf-tests: CLI-7_AC-81
#[test]
fn groups_stop_at_the_limit_includes_fold_and_a_typed_link_hides_its_mirror() {
    let (dir, m) = views("limits");
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&["hub"], 1)).unwrap();
    let shown = lines(&g, &r);
    // Each included file prints on one line, which counts its included regions.
    assert_eq!(shown[1], "  includes -> /src/c.rs (2 regions)");
    assert_eq!(shown[2], "  includes -> /src/plain.rs");
    // The typed link to n00 stands alone, with no `relates-to` edge beside it.
    assert!(shown.contains(&"  depends-on -> /knowledge/many/n00.md".to_string()));
    assert!(!shown.contains(&"  relates-to -> /knowledge/many/n00.md".to_string()));
    // Under n00, the inverse edge also stands alone.
    let r00 = run(&mut g, &m, None, &dir, &query(&["n00"], 1)).unwrap();
    assert_eq!(
        lines(&g, &r00),
        vec![
            "/knowledge/many/n00.md",
            "  depended-on-by -> /knowledge/hub.md"
        ]
    );
    // Of the 22 body links and the mention from old, 20 print, and then a
    // line counts the rest.
    let relates = shown
        .iter()
        .filter(|l| l.starts_with("  relates-to"))
        .count();
    assert_eq!(relates, LIMIT);
    assert_eq!(shown.last().unwrap(), "  3 more relates-to");
    // The JSON counts the regions of the folded include and of its inverse.
    assert_eq!(r.folded.values().copied().collect::<Vec<_>>(), vec![2, 2]);
    // With `--all`, every region and every edge prints.
    let all = GraphQuery {
        all: true,
        ..query(&["hub"], 1)
    };
    let r = run(&mut g, &m, None, &dir, &all).unwrap();
    let shown = lines(&g, &r);
    assert!(shown.contains(&"  includes -> /src/c.rs".to_string()));
    assert_eq!(
        shown
            .iter()
            .filter(|l| l.starts_with("  relates-to"))
            .count(),
        23
    );
    // A directory with more entries than the limit prints 20 of them, then a
    // line that counts the rest.
    let r = run(&mut g, &m, None, &dir, &query(&["knowledge/many"], 1)).unwrap();
    let shown = lines(&g, &r);
    assert_eq!(shown.len(), 1 + LIMIT + 1);
    assert_eq!(shown.last().unwrap(), "  3 more");
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-72
// sokf-tests: CLI-7_AC-73
#[test]
fn a_file_or_an_outside_directory_shows_what_describes_its_directories() {
    let (dir, m) = views("files");
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&["src/c.rs"], 1)).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/src/c.rs",
            "  included-by -> /knowledge/hub.md (2 regions)",
            "  described-by -> /knowledge/sub/c.md",
            "  described-by -> /knowledge/d.md (via /src)",
        ]
    );
    // The directory node of a `via` edge is in the bounded set.
    assert!(
        r.nodes
            .contains(&NodeKey::Path(RepoPath::parse("/src").unwrap()))
    );
    let r = run(&mut g, &m, None, &dir, &query(&["src"], 1)).unwrap();
    assert_eq!(
        lines(&g, &r),
        vec![
            "/src",
            "  described-by -> /knowledge/d.md",
            "  /src/c.rs",
            "  /src/plain.rs",
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-74
// sokf-tests: CLI-7_AC-75
#[test]
fn a_key_shows_its_document_first_then_who_cites_it() {
    let (dir, m) = views("keys");
    let mut g = build(&m);
    let doc = RepoPath::parse("/knowledge/sub/b.md").unwrap();
    let citing = [
        RepoPath::parse("/knowledge/a.md").unwrap(),
        RepoPath::parse("/knowledge/closed/old.md").unwrap(),
    ];
    g.add_key(&doc, "k-1", "K-1", 7, &citing);
    g.set_archived([citing[1].clone()]);
    // When a key's document is archived, the key keeps its edge to that
    // document.
    let mut archived_doc = g.clone();
    archived_doc.set_archived([doc.clone(), citing[1].clone()]);
    let named: BTreeSet<NodeKey> = [
        NodeKey::Key("/knowledge/sub/b.md#k-1".into()),
        NodeKey::Path(doc.clone()),
    ]
    .into();
    let q = query(&[], 1);
    let mut t = Traversal::new(&mut archived_doc, &q, named);
    let kept = t.hops(&NodeKey::Key("/knowledge/sub/b.md#k-1".into()), 1);
    assert_eq!(kept.len(), 2);
    let key = NodeKey::Key("/knowledge/sub/b.md#k-1".into());
    assert_eq!(g.kind(&key), Some(NodeKind::Key));
    let mut t = Traversal::new(&mut g, &q, BTreeSet::new());
    let entries = t.hops(&key, 1);
    let shown: Vec<String> = entries
        .iter()
        .map(|e| match e {
            Entry::Edge { edge, target, .. } => {
                format!("{} -> {}", t.graph.edges[*edge].rel, target.as_str())
            }
            _ => String::new(),
        })
        .collect();
    assert_eq!(
        shown,
        vec![
            "defined-in -> /knowledge/sub/b.md",
            "cited-by -> /knowledge/a.md",
        ]
    );
    assert_eq!(t.hidden, 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-72
// sokf-tests: CLI-7_AC-71
#[test]
fn a_document_over_a_file_and_its_directory_prints_once_and_cites_is_custom() {
    let (dir, _) = views("once");
    std::fs::write(
        dir.join("knowledge/e.md"),
        "---\ntype: Note\nid: e\nresource: [/src, /src/c.rs]\nlinks:\n  - rel: cites\n    to: d\n---\n\n# E\n\n[d][sokf:d]\n",
    )
    .unwrap();
    let m = model(&dir);
    let mut g = build(&m);
    let r = run(&mut g, &m, None, &dir, &query(&["src/c.rs"], 1)).unwrap();
    let shown = lines(&g, &r);
    assert_eq!(
        shown
            .iter()
            .filter(|l| l.contains("/knowledge/e.md"))
            .count(),
        1,
        "{shown:?}"
    );
    assert!(shown.contains(&"  described-by -> /knowledge/e.md".to_string()));
    // The rel `cites` of a `links` entry is a custom rel, so its inverse is
    // `relates-to`, even though the edges of a key use the name `cited-by`.
    let r = run(&mut g, &m, None, &dir, &query(&["d"], 1)).unwrap();
    assert!(
        lines(&g, &r).contains(&"  relates-to -> /knowledge/e.md".to_string()),
        "{:?}",
        lines(&g, &r)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-77
#[test]
fn a_filter_that_lets_a_node_expand_sooner_never_counts_more_than_the_total() {
    let dir = temp("monotonic");
    for (id, links) in [
        ("aa", "[bb][sokf:bb] [xx][sokf:xx]"),
        ("bb", "[xx][sokf:xx]"),
        ("xx", "[yy][sokf:yy]"),
        ("yy", "[z1][sokf:z1] [z2][sokf:z2]"),
        ("z1", ""),
        ("z2", ""),
    ] {
        std::fs::write(
            dir.join(format!("knowledge/{id}.md")),
            format!("---\ntype: Note\nid: {id}\n---\n\n# {id}\n\n{links}\n"),
        )
        .unwrap();
    }
    let m = model(&dir);
    let mut g = build(&m);
    // With bb filtered out, xx expands at hop 1, so the query reaches z1 and
    // z2 at hop 3. In the unfiltered view, xx expands first under bb, at
    // hop 2, so the query stops at depth 3 before it reaches z1 and z2.
    let filtered = GraphQuery {
        to: vec![Expr::compile("id != 'bb'").unwrap()],
        ..query(&["aa"], 3)
    };
    let r = run(&mut g, &m, None, &dir, &filtered).unwrap();
    assert!(
        r.summary.documents.shown <= r.summary.documents.total,
        "{:?}",
        r.summary
    );
    assert!(lines(&g, &r).iter().any(|l| l.contains("z1.md")));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `queries` returns, for each start list in `starts` and each depth from 0
/// to 3, a query for every combination of the layout's edges, `--all`, and
/// `--archived`. At depth 3 it also returns one query with a `--to` filter
/// and no other flags.
fn queries(starts: &[&[&str]]) -> Vec<GraphQuery> {
    let mut out = Vec::new();
    for start in starts {
        for depth in 0..4 {
            for flags in 0..8u8 {
                let mut q = query(start, depth);
                q.layout_edges = flags & 1 != 0;
                q.all = flags & 2 != 0;
                q.archived = flags & 4 != 0;
                out.push(q.clone());
                if depth == 3 && flags == 0 {
                    q.to = vec![Expr::compile("id != 'bb' && id != 'b'").unwrap()];
                    out.push(q);
                }
            }
        }
    }
    out
}

// sokf-tests: CACHE-1_AC-38
// sokf-tests: CACHE-1_AC-44
// sokf-tests: CACHE-1_AC-45
// sokf-tests: CACHE-1_AC-39
#[test]
fn the_edges_near_the_starts_print_what_every_edge_prints() {
    let archived = || [RepoPath::parse("/knowledge/closed/old.md").unwrap()];
    let compare = |dir: &Path, m: &Model, starts: &[&[&str]]| {
        let every = declared_edges(m);
        for q in queries(starts) {
            let mut full = build(m);
            full.set_archived(archived());
            let want = run(&mut full, m, None, dir, &q).unwrap();
            let mut near = crate::graph::build_from(m, Vec::new(), None);
            near.set_archived(archived());
            let starts = crate::graph::starts(&mut near, m, None, dir, &q).unwrap();
            let edges = crate::graph::edges_near(&near, &starts, &q, &every);
            near.add_edges(edges);
            let got = run(&mut near, m, None, dir, &q).unwrap();
            assert_eq!(lines(&near, &got), lines(&full, &want), "{q:?}");
            assert_eq!(got.summary, want.summary, "{q:?}");
            assert_eq!(
                json::bounded(&near, &got),
                json::bounded(&full, &want),
                "{q:?}"
            );
        }
    };
    let (dir, m) = views("near");
    // A query of one document at depth 1 reads only the edges near that
    // document, not every edge in the store.
    let mut near = crate::graph::build_from(&m, Vec::new(), None);
    let q = query(&["a"], 1);
    let starts = crate::graph::starts(&mut near, &m, None, &dir, &q).unwrap();
    let every = build(&m);
    let read = crate::graph::edges_near(&near, &starts, &q, &declared_edges(&m));
    assert!(
        !read.is_empty() && read.len() * 2 < every.edges.len(),
        "{read:?}"
    );
    compare(
        &dir,
        &m,
        &[
            &[],
            &["."],
            &["a"],
            &["c"],
            &["hub"],
            &["n00"],
            &["old"],
            &["d"],
            &["src"],
            &["src/c.rs"],
            &["src/plain.rs"],
            &["docs/guide.md"],
            &["docs"],
            &["knowledge"],
            &["knowledge/many"],
            &["a", "docs/guide.md", "knowledge/sub"],
        ],
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
