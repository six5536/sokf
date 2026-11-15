//! These property tests check whole runs. A fix is idempotent, the line
//! endings of a file do not change what validation reports, a scope is a
//! filter, and the cache does not change a result. For the graph and
//! `locate`, every edge has one inverse, another walk order gives the same
//! graph and result, the output is bounded at any depth, a selector is a
//! projection, the layout is complete, every printed path is in the form that
//! the specification defines, a walk that starts from a document gives one
//! result whether the document is given by id or by path, and a projection is
//! a view of the JSON.

use std::fmt::Write as _;

use common::{
    TempTree,
    graph_rows::{edge_entries, graph_of, layout_lines, query, rows},
    new_options,
};
use proptest::prelude::*;
use sokf_core::{CacheMode, FixOp, RepoPath, Scope};

use crate::common;

/// A perturbation of the clean fixture: the repairs it needs, and whether its
/// files use CRLF line endings.
#[derive(Debug, Clone)]
struct Perturbation {
    stale_include: bool,
    misplaced: bool,
    setext: bool,
    stale_definition: bool,
    missing_index: bool,
    stale_index: bool,
    crlf: bool,
}

fn arb_perturbation() -> impl Strategy<Value = Perturbation> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                stale_include,
                misplaced,
                setext,
                stale_definition,
                missing_index,
                stale_index,
                crlf,
            )| {
                Perturbation {
                    stale_include,
                    misplaced,
                    setext,
                    stale_definition,
                    missing_index,
                    stale_index,
                    crlf,
                }
            },
        )
}

/// `tree_with` returns the clean fixture with the chosen perturbations applied.
fn tree_with(p: &Perturbation) -> TempTree {
    let tree = TempTree::fixture("clean");
    if p.stale_include {
        let text = tree
            .read("README.md")
            .replace("fn cli() -> &'static str {", "fn stale() {");
        tree.write("README.md", &text);
    }
    if p.misplaced {
        tree.write(
            "knowledge/issues/open/issue-002-done.md",
            "---\ntype: Issue\nid: issue-002-done\ntitle: Done\nstate: closed\nkind: feature\n---\n\n# Done\n\nShipped.\n\n## Conclusion\n\nDone.\n",
        );
    }
    if p.setext {
        let text = tree
            .read("knowledge/decisions/decision-single-binary.md")
            .replace("# Ship one binary\n", "Ship one binary\n===============\n");
        tree.write("knowledge/decisions/decision-single-binary.md", &text);
    }
    if p.stale_definition {
        let text = tree
            .read("knowledge/overview.md")
            .replace("[sokf:#CLI-1]: /knowledge/req-cli.md\n", "");
        tree.write("knowledge/overview.md", &text);
    }
    if p.missing_index {
        std::fs::remove_file(tree.dir().join("knowledge/decisions/index.md")).unwrap();
    }
    if p.stale_index {
        tree.write(
            "knowledge/schemas/index.md",
            "# Schemas\n\n- [nothing](nothing.md)\n",
        );
    }
    if p.crlf {
        for (rel, text) in tree.files() {
            tree.write(&rel, &text.replace('\n', "\r\n"));
        }
    }
    tree
}

// sokf-tests: CLI_P-5
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn fix_is_idempotent(p in arb_perturbation()) {
        let tree = tree_with(&p);
        let first = tree.validate(true, CacheMode::Off);
        let second = tree.validate(true, CacheMode::Off);
        prop_assert!(second.fixes.is_empty(), "{:?}", second.fixes);
        prop_assert!(first.errors.is_empty(), "{:?}", first.errors);
        prop_assert_eq!(second.errors, first.errors);
        prop_assert_eq!(second.warnings, first.warnings);
    }
}

// sokf-tests: CLI_P-6
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn line_endings_change_no_finding_and_survive_fix(p in arb_perturbation()) {
        let lf = tree_with(&Perturbation { crlf: false, ..p.clone() });
        let crlf = tree_with(&Perturbation { crlf: true, ..p.clone() });
        let a = lf.validate(false, CacheMode::Off);
        let b = crlf.validate(false, CacheMode::Off);
        prop_assert_eq!(a.errors, b.errors);
        prop_assert_eq!(a.warnings, b.warnings);
        let fixed = crlf.validate(true, CacheMode::Off);
        prop_assert_eq!(&fixed.fixes, &lf.validate(true, CacheMode::Off).fixes);
        // An index that the fix creates uses LF line endings (SOKF_VALIDATION_36).
        // Every other file keeps its CRLF line endings.
        let created: Vec<String> = fixed.fixes.iter().filter_map(|f| match f {
            FixOp::Created { path } => Some(path.relative().to_string()),
            _ => None,
        }).collect();
        for (rel, text) in crlf.files() {
            if created.contains(&rel) {
                prop_assert!(!text.contains('\r'), "an index that the fix creates uses LF line endings");
                continue;
            }
            let stripped = text.replace("\r\n", "");
            prop_assert!(!stripped.contains('\n'), "a CRLF file stays CRLF");
        }
    }
}

// sokf-tests: CLI_P-7
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn scope_is_a_filter(paths in prop::collection::btree_set(prop::sample::select(vec![
        "docs", "knowledge/other", "knowledge/stale.md", "knowledge/schemas", "README.md", "knowledge/issues",
    ]), 1..4)) {
        let tree = TempTree::fixture("broken");
        let full = tree.validate(false, CacheMode::Off);
        let paths: Vec<&str> = paths.iter().copied().collect();
        let narrowed = tree.validate_paths(&paths, false);
        let mut scope = Scope::default();
        for p in &paths {
            let rp = RepoPath::parse(&format!("/{p}")).unwrap();
            if tree.dir().join(p).is_dir() {
                scope.dirs.insert(rp);
            } else {
                scope.files.insert(rp);
            }
        }
        prop_assert_eq!(narrowed, full.filter(&scope));
    }
}

// sokf-tests: CLI_P-1
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(16))]

    #[test]
    fn a_cached_run_equals_an_uncached_run_after_edits(
        edits in prop::collection::vec((prop::sample::select(vec![
            "knowledge/req-cli.md", "knowledge/overview.md", "README.md", "knowledge/decisions/decision-single-binary.md",
        ]), "[a-z ]{0,20}"), 0..3),
    ) {
        let tree = TempTree::fixture("clean");
        let _ = tree.validate(false, CacheMode::On);
        for (rel, extra) in &edits {
            let text = tree.read(rel);
            tree.write(rel, &format!("{text}\n{extra}\n"));
        }
        let cached = tree.validate(false, CacheMode::On);
        let uncached = tree.validate(false, CacheMode::Off);
        prop_assert_eq!(cached, uncached);
    }
}

// ---------------------------------------------------------------------------
// The graph and `locate` commands (CLI-7, CLI-8).

use std::collections::BTreeSet;

use sokf_core::{
    Document, Entry, Expr, Graph, GraphOptions, GraphQuery, GraphResult, Model, NodeKey, NodeKind,
    Row,
    graph::{bounded, build, inverse_rel, run},
};

/// A `Doc` is one generated document.
#[derive(Debug, Clone)]
struct Doc {
    dir: u8,
    type_task: bool,
    tags: bool,
    links: Vec<(usize, usize)>,
    body_link: Option<usize>,
    resource: Option<bool>,
    source: Option<bool>,
    include: Option<u8>,
    broken: bool,
}

/// `RELS` holds the relations that the generated documents use: the core
/// relations, their inverse forms, and a custom relation. The inverse forms let a declared
/// inverse match a declared edge.
const RELS: [&str; 6] = [
    "depends-on",
    "references",
    "part-of",
    "custom",
    "depended-on-by",
    "relates-to",
];

fn arb_doc(n: usize) -> impl Strategy<Value = Doc> {
    (
        0u8..3,
        any::<bool>(),
        any::<bool>(),
        prop::collection::vec((0usize..6, 0..=n), 0..3),
        prop::option::of(0..=n),
        prop::option::of(any::<bool>()),
        prop::option::of(any::<bool>()),
        prop::option::of(0u8..3),
        prop::bool::weighted(0.15),
    )
        .prop_map(
            |(dir, type_task, tags, links, body_link, resource, source, include, broken)| Doc {
                dir,
                type_task,
                tags,
                links,
                body_link,
                resource,
                source,
                include,
                broken,
            },
        )
}

/// `arb_tree` generates one to five documents with links, body links,
/// resources, sources, and includes. A link can point to a missing target, and
/// a document can be broken. `write_tree` adds a schema that governs one
/// additional file inside the root and one outside it.
fn arb_tree() -> impl Strategy<Value = Vec<Doc>> {
    (1usize..=5).prop_flat_map(|n| prop::collection::vec(arb_doc(n), n))
}

/// `generated_dirs` returns the directories under `/knowledge` that the
/// generated documents and their schema sit in.
fn generated_dirs(docs: &[Doc]) -> BTreeSet<&'static str> {
    let mut dirs = BTreeSet::from(["/knowledge/schemas"]);
    for d in docs {
        if d.dir >= 1 {
            dirs.insert("/knowledge/sub");
        }
        if d.dir == 2 {
            dirs.insert("/knowledge/sub/deep");
        }
    }
    dirs
}

fn write_tree(docs: &[Doc]) -> TempTree {
    let tree = TempTree::empty("graph-prop");
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.6\"\nname: k\nscope: test\n",
    );
    tree.write("src/a.rs", "fn a() {}\n");
    tree.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\n---\n\n# Note\n\n````yaml\nline-limit: 0\nadditional-files: ['/docs/*.md', '/knowledge/*.txt']\nfrontmatter:\n  type:\n    const: Note\nsections: []\n````\n",
    );
    tree.write(
        "docs/guide.md",
        "# Guide\n\nSee [d0][sokf:d0].\n\n<!-- sokf:include d0 -->\n<!-- /sokf:include -->\n",
    );
    tree.write(
        "knowledge/notes.txt",
        "<!-- sokf:include /src/a.rs -->\n<!-- /sokf:include -->\n",
    );
    let target = |i: usize| {
        if i == docs.len() {
            "nope".to_string()
        } else {
            format!("d{i}")
        }
    };
    for (i, d) in docs.iter().enumerate() {
        let dir = ["", "sub/", "sub/deep/"][d.dir as usize];
        let mut fm = String::new();
        let _ = writeln!(fm, "type: {}", if d.type_task { "Task" } else { "Note" });
        if d.broken {
            let _ = writeln!(fm, "id: &x d{i}");
        } else {
            let _ = writeln!(fm, "id: d{i}");
        }
        if d.tags {
            fm.push_str("tags: [x]\n");
        }
        match d.resource {
            Some(true) => fm.push_str("resource: /src/a.rs\n"),
            Some(false) => fm.push_str("resource: /src/gone.rs\n"),
            None => {}
        }
        match d.source {
            Some(true) => {
                fm.push_str("sources:\n  - resource: /src/a.rs\n  - resource: https://x.y/z\n");
            }
            Some(false) => fm.push_str("sources:\n  - resource: /src/gone.rs\n"),
            None => {}
        }
        if !d.links.is_empty() {
            fm.push_str("links:\n");
            for (rel, to) in &d.links {
                let _ = write!(fm, "  - rel: {}\n    to: {}\n", RELS[*rel], target(*to));
            }
        }
        let mut body = format!("# D{i}\n\n");
        if let Some(to) = d.body_link {
            let _ = write!(body, "See [it][sokf:{}].\n\n", target(to));
        }
        match d.include {
            Some(0) => {
                let _ = write!(
                    body,
                    "<!-- sokf:include {} -->\n<!-- /sokf:include -->\n",
                    target((i + 1) % (docs.len() + 1))
                );
            }
            Some(1) => body.push_str("<!-- sokf:include /src/a.rs#r -->\n<!-- /sokf:include -->\n"),
            Some(_) => {
                body.push_str("<!-- sokf:include /src/gone.rs -->\n<!-- /sokf:include -->\n");
            }
            None => {}
        }
        tree.write(
            &format!("knowledge/{dir}d{i}.md"),
            &format!("---\n{fm}---\n\n{body}"),
        );
    }
    tree
}

fn run_on(tree: &TempTree, query: GraphQuery) -> (Graph, GraphResult) {
    graph_of(tree, query, CacheMode::Off).unwrap()
}

// sokf-tests: CLI_P-8
// sokf-tests: CLI-7_AC-5
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn every_edge_has_one_inverse(docs in arb_tree()) {
        let tree = write_tree(&docs);
        let graph = build(&tree.model());
        for edge in graph.edges.iter().filter(|e| !e.synthesised) {
            let inverses: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| {
                    e.from == edge.to
                        && e.to == edge.from
                        && e.kind == edge.kind
                        && e.region == edge.region
                        && e.rel == inverse_rel(&edge.rel)
                })
                .collect();
            prop_assert_eq!(inverses.len(), 1, "{:?} has {:?}", edge, inverses);
        }
        for edge in graph.edges.iter().filter(|e| e.synthesised) {
            prop_assert!(graph.edges.iter().any(|d| !d.synthesised
                && d.from == edge.to
                && d.to == edge.from
                && d.kind == edge.kind
                && d.region == edge.region
                && inverse_rel(&d.rel) == edge.rel));
        }
    }
}

/// `permuted` returns the model with its documents and additional files in
/// the order that another walk would give, and rebuilds its indexes.
fn permuted(mut model: Model, keys: &[u32]) -> Model {
    let by_key = |i: usize| keys.get(i).copied().unwrap_or(0);
    for knowledge in &mut model.knowledges {
        let mut order: Vec<usize> = (0..knowledge.documents.len()).collect();
        order.sort_by_key(|&i| by_key(i));
        let documents: Vec<Document> = order
            .iter()
            .map(|&i| knowledge.documents[i].clone())
            .collect();
        knowledge.documents = documents;
        knowledge.by_id.clear();
        knowledge.by_path.clear();
        knowledge.by_type.clear();
        for (i, doc) in knowledge.documents.iter().enumerate() {
            if let Some(id) = doc.id() {
                knowledge.by_id.entry(id.to_string()).or_default().push(i);
            }
            if let Some(t) = doc.type_name() {
                knowledge.by_type.entry(t.to_string()).or_default().push(i);
            }
            knowledge.by_path.insert(doc.path.clone(), i);
        }
    }
    let mut order: Vec<usize> = (0..model.additional.len()).collect();
    order.sort_by_key(|&i| by_key(i));
    model.additional = order.iter().map(|&i| model.additional[i].clone()).collect();
    model
}

// sokf-tests: CLI_P-9
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(128))]

    #[test]
    fn build_and_run_are_deterministic(docs in arb_tree(), depth in 0usize..4, keys in prop::collection::vec(any::<u32>(), 8)) {
        let a = write_tree(&docs);
        let b = write_tree(&docs);
        let (ga, ra) = run_on(&a, query(&[], depth));
        let (gb, rb) = run_on(&b, query(&[], depth));
        prop_assert_eq!(&ga.edges, &gb.edges);
        prop_assert_eq!(bounded(&ga, &ra), bounded(&gb, &rb));
        // The same model in another walk order gives the same graph and result.
        let model = a.model();
        let shuffled = permuted(a.model(), &keys);
        let mut g1 = build(&model);
        let mut g2 = build(&shuffled);
        prop_assert_eq!(&g1.edges, &g2.edges);
        prop_assert_eq!(&g1.nodes, &g2.nodes);
        let r1 = run(&mut g1, &model, None, a.dir(), &query(&[], depth)).unwrap();
        let r2 = run(&mut g2, &shuffled, None, a.dir(), &query(&[], depth)).unwrap();
        prop_assert_eq!(r1, r2);
    }
}

// sokf-tests: CLI_P-3
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn output_is_bounded_at_any_depth(docs in arb_tree(), depth in 0usize..12, starts in prop::collection::vec(0usize..6, 0..3)) {
        let tree = write_tree(&docs);
        let starts: Vec<String> = starts
            .iter()
            .map(|&i| if i >= docs.len() || docs[i].broken { "knowledge".to_string() } else { format!("d{i}") })
            .collect();
        let (graph, result) = run_on(&tree, GraphQuery { start: starts, ..query(&[], depth) });
        let entries = edge_entries(&result);
        prop_assert!(entries.len() <= graph.edges.len());
        let row_nodes: Vec<&NodeKey> = rows(&graph, &result).iter().map(|r| &r.node).collect();
        let unique: BTreeSet<&NodeKey> = row_nodes.iter().copied().collect();
        prop_assert_eq!(row_nodes.len(), unique.len());
        let expanded: Vec<&NodeKey> = entries.iter().filter(|(_, _, here)| *here).map(|(_, t, _)| t).collect();
        let unique: BTreeSet<&NodeKey> = expanded.iter().copied().collect();
        prop_assert_eq!(expanded.len(), unique.len());
        for t in &expanded {
            prop_assert!(!row_nodes.contains(t), "a row is never expanded");
        }
    }
}

// sokf-tests: CLI_P-1
proptest! {
    // Each case runs three graph builds and three `new --id-only` runs on
    // fixture copies, so the test runs fewer cases than the design's minimum.
    #![proptest_config(crate::common::proptest_cases::cases(16))]

    #[test]
    fn a_warm_cache_gives_the_same_graph_and_id(
        edits in prop::collection::vec((prop::sample::select(vec![
            "knowledge/config.md", "knowledge/core/planner.md", "knowledge/core/executor.md",
        ]), 0u8..4, "[a-z ]{0,20}"), 0..3),
    ) {
        let tree = TempTree::fixture("graph");
        let opts = |cache| GraphOptions { query: query(&["planner", "knowledge"], 2), cwd: tree.dir().to_path_buf(), cache };
        let _ = sokf_core::graph(&tree.root(), &opts(CacheMode::On)).unwrap();
        // An edit adds text, adds a link or an include, or rewrites the
        // document without edges. An addition goes before the link definition
        // block, in the part of the body that sokf reads.
        for (rel, kind, extra) in &edits {
            let text = tree.read(rel);
            let addition = match kind {
                0 => format!("{extra}\n"),
                1 => "See [it][sokf:executor] and [that][sokf:nope].\n".to_string(),
                2 => "<!-- sokf:include /crates/lib/example-core/src/planner.rs -->\n<!-- /sokf:include -->\n".to_string(),
                _ => String::new(),
            };
            let edited = if *kind == 3 {
                format!("---\ntype: Subsystem\nid: {}\n---\n\n# X\n", rel.rsplit('/').next().unwrap().trim_end_matches(".md"))
            } else {
                match text.split_once("<!-- sokf:links -->") {
                    Some((head, tail)) => format!("{head}{addition}\n<!-- sokf:links -->{tail}"),
                    None => format!("{text}\n{addition}"),
                }
            };
            tree.write(rel, &edited);
        }
        let (cold_graph, cold_result) = sokf_core::graph(&tree.root(), &opts(CacheMode::Off)).unwrap();
        let (warm_graph, warm_result) = sokf_core::graph(&tree.root(), &opts(CacheMode::On)).unwrap();
        prop_assert_eq!(bounded(&cold_graph, &cold_result), bounded(&warm_graph, &warm_result));
        // The last edit of a file determines its edges: if a later edit
        // rewrites the file, the link that an earlier edit added leaves no edge.
        let mut last: std::collections::BTreeMap<&str, u8> = std::collections::BTreeMap::new();
        for (rel, kind, _) in &edits {
            last.insert(rel, *kind);
        }
        if last.values().any(|k| *k == 1) {
            prop_assert!(cold_graph.edges.iter().any(|e| e.to == NodeKey::Missing("nope".into())));
        }
        let clean = TempTree::fixture("clean");
        let id_only = |cache| {
            sokf_core::new(
                &clean.root(),
                &sokf_core::NewOptions {
                    slug: Some("y".into()),
                    id_only: true,
                    cache,
                    ..new_options(clean.dir(), "Issue")
                },
            )
            .map(|r| r.id)
        };
        let _ = id_only(CacheMode::On).unwrap();
        clean.write(
            "knowledge/issues/open/issue-007-x.md",
            "---\ntype: Issue\nid: issue-007-x\ntitle: X\nstate: open\nkind: bug\n---\n\n# X\n\nx\n\n## Reproduction\n\n1. y\n",
        );
        let cached = id_only(CacheMode::On).unwrap();
        let uncached = id_only(CacheMode::Off).unwrap();
        prop_assert_eq!(&cached, &uncached);
        prop_assert_eq!(cached, "issue-008-y");
    }
}

// sokf-tests: CLI_P-10
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn a_selector_is_a_projection(docs in arb_tree(), which in 0usize..4) {
        let tree = write_tree(&docs);
        let source = ["type=='Task'", "contains(tags, 'x')", "broken", "!resource"][which];
        let expr = Expr::compile(source).unwrap();
        let (graph, selected) = run_on(&tree, GraphQuery { selector: vec![expr.clone()], all: true, ..query(&[], 0) });
        let listed: BTreeSet<NodeKey> = layout_lines(&graph, &selected)
            .iter()
            .filter(|r| graph.kind(&r.node) == Some(NodeKind::Document))
            .map(|r| r.node.clone())
            .collect();
        let (graph, all) = run_on(&tree, GraphQuery { all: true, ..query(&[], 0) });
        let json = bounded(&graph, &all);
        let expected: BTreeSet<NodeKey> = json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["kind"] == "document" && expr.truthy(n))
            .map(|n| NodeKey::Path(sokf_core::RepoPath::parse(n["path"].as_str().unwrap()).unwrap()))
            .collect();
        prop_assert_eq!(listed, expected);
    }
}

/// `walk` pushes each directory of the layout under `row` onto `out`.
fn walk<'a>(graph: &Graph, row: &'a Row, out: &mut Vec<&'a str>) {
    for e in &row.entries {
        if let Entry::Layout(r) = e {
            if graph.kind(&r.node) == Some(NodeKind::Directory) {
                out.push(r.node.as_str());
            }
            walk(graph, r, out);
        }
    }
}

// sokf-tests: CLI_P-11
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn the_layout_is_complete(docs in arb_tree()) {
        let tree = write_tree(&docs);
        let (graph, result) = run_on(&tree, GraphQuery { all: true, archived: true, ..query(&[], 0) });
        let listed: Vec<&Row> = layout_lines(&graph, &result);
        // The layout lists every generated document and the schema once each,
        // and the additional file inside the root once. It doesn't list the
        // file outside the root.
        let mut documents: Vec<&str> = listed
            .iter()
            .filter(|r| graph.kind(&r.node) == Some(NodeKind::Document))
            .map(|r| r.node.as_str())
            .collect();
        documents.sort_unstable();
        let mut expected: Vec<String> = docs
            .iter()
            .enumerate()
            .map(|(i, d)| format!("/knowledge/{}d{i}.md", ["", "sub/", "sub/deep/"][d.dir as usize]))
            .collect();
        expected.push("/knowledge/schemas/note.md".into());
        expected.sort();
        prop_assert_eq!(documents, expected.iter().map(String::as_str).collect::<Vec<_>>());
        let additional: Vec<&str> = listed
            .iter()
            .filter(|r| graph.kind(&r.node) == Some(NodeKind::Additional))
            .map(|r| r.node.as_str())
            .collect();
        prop_assert_eq!(additional, vec!["/knowledge/notes.txt"]);
        for key in &result.nodes {
            prop_assert!(!matches!(graph.kind(key), Some(NodeKind::File | NodeKind::Missing)));
        }
        // The layout lists each directory between the root and a document
        // exactly once.
        let mut directories: Vec<&str> = Vec::new();
        for r in &result.rows {
            walk(&graph, r, &mut directories);
        }
        directories.sort_unstable();
        prop_assert_eq!(directories, generated_dirs(&docs).into_iter().collect::<Vec<_>>());
        prop_assert!(result.edges.is_empty());
    }
}

// sokf-tests: CLI_P-12
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn every_printed_path_is_in_the_spec_form(docs in arb_tree(), depth in 0usize..3) {
        let tree = write_tree(&docs);
        let (graph, result) = run_on(&tree, query(&[], depth));
        let json = bounded(&graph, &result);
        for s in json["start"].as_array().unwrap() {
            prop_assert!(sokf_core::RepoPath::parse(s.as_str().unwrap()).is_some());
        }
        let mut missing = BTreeSet::new();
        for n in json["nodes"].as_array().unwrap() {
            match n.get("path") {
                Some(p) => prop_assert!(sokf_core::RepoPath::parse(p.as_str().unwrap()).is_some()),
                None => { missing.insert(n["target"].as_str().unwrap().to_string()); }
            }
        }
        for e in json["edges"].as_array().unwrap() {
            for end in ["from", "to"] {
                let s = e[end].as_str().unwrap();
                prop_assert!(missing.contains(s) || sokf_core::RepoPath::parse(s).is_some(), "{s}");
            }
        }
        let report = tree.validate(false, CacheMode::Off);
        let value = serde_json::to_value(&report).unwrap();
        for f in value["errors"].as_array().unwrap().iter().chain(value["warnings"].as_array().unwrap()) {
            prop_assert!(sokf_core::RepoPath::parse(f["path"].as_str().unwrap()).is_some());
        }
        for (i, d) in docs.iter().enumerate() {
            if !d.broken {
                let paths = sokf_core::locate(&tree.root(), &[format!("d{i}")], false, CacheMode::Off).unwrap();
                prop_assert!(paths[0].path.is_absolute() && paths[0].path.exists());
            }
        }
    }
}

// sokf-tests: CLI_P-13
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn a_document_started_by_id_or_by_path_gives_one_result(docs in arb_tree(), depth in 0usize..4) {
        let tree = write_tree(&docs);
        for (i, d) in docs.iter().enumerate() {
            if d.broken {
                continue;
            }
            let dir = ["", "sub/", "sub/deep/"][d.dir as usize];
            let (ga, ra) = run_on(&tree, query(&[&format!("d{i}")], depth));
            let (gb, rb) = run_on(&tree, query(&[&format!("knowledge/{dir}d{i}.md")], depth));
            prop_assert_eq!(&ra, &rb);
            prop_assert_eq!(bounded(&ga, &ra), bounded(&gb, &rb));
        }
    }
}

// sokf-tests: CLI_P-14
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn a_projection_is_a_view_of_the_json(docs in arb_tree(), which in 0usize..3) {
        let tree = write_tree(&docs);
        let source = ["nodes[?kind=='document'].id", "length(edges)", "edges[?synthesised].rel"][which];
        let expr = Expr::compile(source).unwrap();
        let (graph, plain) = run_on(&tree, query(&[], 1));
        let (_, projected) = run_on(&tree, GraphQuery { query: Some(expr.clone()), ..query(&[], 1) });
        prop_assert_eq!(projected.projection, Some(expr.eval(&bounded(&graph, &plain)).unwrap()));
    }
}
