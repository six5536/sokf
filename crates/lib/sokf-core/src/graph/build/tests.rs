//! These tests check that `build` creates every kind of node, and every
//! edge together with its inverse, and keeps the fields of a broken
//! document. They also check how a frontmatter becomes JSON, which rel is
//! the inverse of each rel, and how a `to` resolves to a document or a
//! file.

use std::path::PathBuf;

use super::*;
use crate::test_support::{model_at, write};

fn temp(name: &str) -> PathBuf {
    crate::test_support::fresh_dir(&format!("graph-{name}"))
}

fn key(s: &str) -> NodeKey {
    match RepoPath::parse(s) {
        Some(p) => NodeKey::Path(p),
        None => NodeKey::Missing(s.to_string()),
    }
}

fn shown(g: &Graph) -> Vec<String> {
    g.edges
        .iter()
        .map(|e| {
            format!(
                "{} {} {} {}{}{}{}",
                e.from.as_str(),
                e.kind.name(),
                e.to.as_str(),
                e.rel,
                e.region
                    .as_ref()
                    .map(|r| format!(" #{r}"))
                    .unwrap_or_default(),
                e.note
                    .as_ref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default(),
                if e.synthesised { " *" } else { "" },
            )
        })
        .collect()
}

// sokf-tests: CLI-7_AC-5
// sokf-tests: CLI_P-8
// sokf-tests: SOKF_FORMAT_57
// sokf-tests: SOKF_FORMAT_59
// sokf-tests: SOKF_FORMAT_113
// sokf-tests: SOKF_FORMAT_115
// sokf-tests: SOKF_FORMAT_116
// sokf-tests: SOKF_FORMAT_118
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the test walks one scenario in order, which splitting would hide"
)]
fn builds_every_node_and_edge_kind_with_inverses() {
    let dir = temp("kinds");
    write(
        &dir,
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: k\nscope: test\ndescription: d\n",
    );
    // The tool generates each index, so no index is a node, whether it
    // is at the root or in a subdirectory.
    write(&dir, "knowledge/index.md", "# Index\n");
    write(&dir, "knowledge/core/index.md", "# core\n");
    write(
        &dir,
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nadditional-files: '/README.md'\nfrontmatter:\n  type:\n    const: Note\nsections: []\n````\n",
    );
    write(
        &dir,
        "knowledge/core/a.md",
        "---\ntype: Note\nid: a\ntitle: A\nresource: [/src/a.rs, /src/dir]\nsources:\n  - resource: /src/b.rs\n  - resource: https://x.y\n  - resource: /src/gone.rs\nlinks:\n  - rel: depends-on\n    to: b\n    note: n\n  - rel: likes\n    to: b\n  - rel: references\n    to: nope\n  - rel: references\n    to: /README.md\n  - rel: part-of\n    to: a\n---\n\n# A\n\n[b][sokf:b] [c][sokf:c] [nope][sokf:nope] [key][sokf:#K] [self][sokf:a]\n\n<!-- sokf:include /src/a.rs#cli -->\n<!-- /sokf:include -->\n<!-- sokf:include /src/a.rs#other -->\n<!-- /sokf:include -->\n<!-- sokf:include /src/a.rs -->\n<!-- /sokf:include -->\n<!-- sokf:include c -->\n<!-- /sokf:include -->\n<!-- sokf:include /src/dir -->\n<!-- /sokf:include -->\n",
    );
    write(
        &dir,
        "knowledge/core/b.md",
        "---\ntype: Note\nid: b\nstatus: draft\nlinks:\n  - rel: depended-on-by\n    to: a\n---\n\n# B\n\n[a][sokf:a]\n",
    );
    write(
        &dir,
        "knowledge/c.md",
        "---\ntype: Note\nid: c\n---\n\n# C\n",
    );
    write(
        &dir,
        "README.md",
        "# R\n\n[a][sokf:a]\n\n<!-- sokf:include a -->\n<!-- /sokf:include -->\n<!-- sokf:include /src/a.rs -->\n<!-- /sokf:include -->\n",
    );
    write(&dir, "src/a.rs", "");
    write(&dir, "src/b.rs", "");
    write(&dir, "src/dir/x", "");
    let m = model_at(&dir, &["/knowledge"]);
    let g = build(&m);
    let kinds: Vec<(String, &str)> = g
        .nodes
        .values()
        .map(|n| (n.key.as_str().to_string(), n.kind.name()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("/README.md".into(), "additional"),
            ("/knowledge".into(), "root"),
            ("/knowledge/c.md".into(), "document"),
            ("/knowledge/core".into(), "directory"),
            ("/knowledge/core/a.md".into(), "document"),
            ("/knowledge/core/b.md".into(), "document"),
            ("/knowledge/schemas".into(), "directory"),
            ("/knowledge/schemas/note.md".into(), "document"),
            ("/src/a.rs".into(), "file"),
            ("/src/b.rs".into(), "file"),
            ("/src/dir".into(), "file"),
            ("/src/gone.rs".into(), "missing"),
            ("a".into(), "missing"),
            ("nope".into(), "missing"),
        ]
    );
    let root = g.node(&key("/knowledge")).unwrap();
    assert_eq!(root.fields["name"], "k");
    assert_eq!(root.fields["description"], "d");
    assert_eq!(
        g.node(&key("/README.md")).unwrap().fields["schema"],
        "schema-note"
    );
    assert_eq!(
        shown(&g),
        vec![
            "/README.md includes /knowledge/core/a.md includes",
            "/README.md includes /src/a.rs includes",
            "/README.md links /knowledge/core/a.md referenced-by *",
            "/README.md links a relates-to",
            "/knowledge/c.md includes /knowledge/core/a.md included-by *",
            "/knowledge/c.md links /knowledge/core/a.md relates-to *",
            "/knowledge/core/a.md includes /README.md included-by *",
            "/knowledge/core/a.md includes /knowledge/c.md includes",
            "/knowledge/core/a.md includes /src/a.rs includes",
            "/knowledge/core/a.md includes /src/a.rs includes #cli",
            "/knowledge/core/a.md includes /src/a.rs includes #other",
            "/knowledge/core/a.md includes /src/dir includes",
            "/knowledge/core/a.md links /README.md references",
            "/knowledge/core/a.md links /knowledge/c.md relates-to",
            "/knowledge/core/a.md links /knowledge/core/a.md has-part *",
            "/knowledge/core/a.md links /knowledge/core/a.md part-of",
            "/knowledge/core/a.md links /knowledge/core/b.md depends-on (n)",
            "/knowledge/core/a.md links /knowledge/core/b.md likes",
            "/knowledge/core/a.md links nope references",
            "/knowledge/core/a.md resource /src/a.rs describes",
            "/knowledge/core/a.md resource /src/dir describes",
            "/knowledge/core/a.md sources /src/b.rs derives-from",
            "/knowledge/core/a.md sources /src/gone.rs derives-from",
            "/knowledge/core/b.md links /knowledge/core/a.md depended-on-by",
            "/knowledge/core/b.md links /knowledge/core/a.md relates-to *",
            "/src/a.rs includes /README.md included-by *",
            "/src/a.rs includes /knowledge/core/a.md included-by *",
            "/src/a.rs includes /knowledge/core/a.md included-by #cli *",
            "/src/a.rs includes /knowledge/core/a.md included-by #other *",
            "/src/a.rs resource /knowledge/core/a.md described-by *",
            "/src/b.rs sources /knowledge/core/a.md derived-by *",
            "/src/dir includes /knowledge/core/a.md included-by *",
            "/src/dir resource /knowledge/core/a.md described-by *",
            "/src/gone.rs sources /knowledge/core/a.md derived-by *",
            "a links /README.md relates-to *",
            "nope links /knowledge/core/a.md referenced-by *",
        ]
    );
    assert_eq!(
        g.node(&key("/knowledge/core/b.md")).unwrap().fields["status"],
        "draft"
    );
    assert_eq!(
        g.node(&key("/knowledge/core/a.md")).unwrap().fields["status"],
        "stable"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-18
// sokf-tests: CLI-7_AC-42
#[test]
fn broken_documents_and_a_root_without_a_manifest_are_nodes() {
    let dir = temp("broken");
    write(
        &dir,
        "knowledge/bad.md",
        "---\ntype: Note\nid: &x bad\n---\n\n# Bad\n",
    );
    write(
        &dir,
        "knowledge/noid.md",
        "---\ntype: Note\ntitle: T\nkind: dropped\npath: dropped\nbroken: dropped\nreason: dropped\nlinks:\n  - rel: references\n    to: bad\n---\n\n# No id\n",
    );
    write(&dir, "knowledge/none.md", "# No frontmatter\n");
    write(&dir, "knowledge/list.md", "---\n- a\n---\n\n# List\n");
    let m = model_at(&dir, &["/knowledge"]);
    let g = build(&m);
    let root = g.node(&key("/knowledge")).unwrap();
    assert_eq!(root.kind, NodeKind::Root);
    assert!(root.fields.is_empty());
    let fields = |s: &str| g.node(&key(s)).unwrap().fields.clone();
    let bad = fields("/knowledge/bad.md");
    assert_eq!(
        serde_json::to_string(&bad).unwrap(),
        r#"{"status":"stable","broken":true,"reason":"frontmatter does not parse"}"#
    );
    let noid = fields("/knowledge/noid.md");
    assert_eq!(
        serde_json::to_string(&noid).unwrap(),
        r#"{"type":"Note","title":"T","status":"stable","broken":true,"reason":"no id"}"#
    );
    assert_eq!(fields("/knowledge/none.md")["reason"], "no id");
    assert_eq!(fields("/knowledge/list.md")["reason"], "no id");
    // The graph keeps the edges that the tool could read from a
    // broken document.
    assert!(
        g.edges
            .iter()
            .any(|e| e.from == key("/knowledge/noid.md") && e.to == key("bad"))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn frontmatter_json_keeps_the_order_and_shapes_written() {
    let dir = temp("shapes");
    write(
        &dir,
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: k\nscope: test\n",
    );
    write(
        &dir,
        "knowledge/a.md",
        "---\ntitle: T\ntype: Note\nid: a\ntags: [x, y]\nn: 1\nf: 1.5\nb: true\nnothing: ~\nm:\n  k: v\n---\n\n# A\n",
    );
    let m = model_at(&dir, &["/knowledge"]);
    let doc = &m.knowledges[0].documents[0];
    let json = serde_json::to_string(&frontmatter_json(doc)).unwrap();
    assert_eq!(
        json,
        r#"{"title":"T","type":"Note","id":"a","tags":["x","y"],"n":1,"f":1.5,"b":true,"nothing":null,"m":{"k":"v"},"status":"stable"}"#
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn inverse_rels_cover_the_core_set_and_default_to_relates_to() {
    for (rel, inverse) in [
        ("relates-to", "relates-to"),
        ("part-of", "has-part"),
        ("has-part", "part-of"),
        ("depends-on", "depended-on-by"),
        ("depended-on-by", "depends-on"),
        ("references", "referenced-by"),
        ("supersedes", "superseded-by"),
        ("superseded-by", "supersedes"),
        ("implements", "implemented-by"),
        ("implemented-by", "implements"),
        ("tests", "tested-by"),
        ("tested-by", "tests"),
        ("contradicts", "contradicts"),
        ("describes", "described-by"),
        ("derives-from", "derived-by"),
        ("includes", "included-by"),
        ("included-by", "includes"),
        ("custom", "relates-to"),
    ] {
        assert_eq!(inverse_rel(rel), inverse, "{rel}");
    }
}

#[test]
fn resolve_to_reads_an_id_then_a_path() {
    let dir = temp("resolve");
    write(
        &dir,
        "knowledge/a.md",
        "---\ntype: Note\nid: a\n---\n\n# A\n",
    );
    write(
        &dir,
        "knowledge/schemas/s.md",
        "---\ntype: Schema\nid: s\n---\n\n````yaml\n````\n",
    );
    write(&dir, "other/n.md", "---\ntype: Note\nid: n\n---\n\n# N\n");
    write(&dir, "src/x.rs", "");
    let m = model_at(&dir, &["/knowledge", "/other"]);
    let doc = |path: &str, id: &str, knowledge| Target::Document {
        path: RepoPath::parse(path).unwrap(),
        id: id.into(),
        knowledge,
    };
    assert_eq!(resolve_to(&m, 0, "a"), Some(doc("/knowledge/a.md", "a", 0)));
    assert_eq!(
        resolve_to(&m, 0, "/knowledge/schemas/s.md"),
        Some(doc("/knowledge/schemas/s.md", "s", 0))
    );
    assert_eq!(
        resolve_to(&m, 0, "/knowledge/a.md"),
        Some(doc("/knowledge/a.md", "a", 0))
    );
    // A path resolves to a document in another root (SOKF_FORMAT_54),
    // but an id resolves only in its own root.
    assert_eq!(
        resolve_to(&m, 0, "/other/n.md"),
        Some(doc("/other/n.md", "n", 1))
    );
    assert_eq!(resolve_to(&m, 0, "n"), None);
    assert_eq!(
        resolve_to(&m, 0, "/src/x.rs"),
        Some(Target::File(RepoPath::parse("/src/x.rs").unwrap()))
    );
    assert_eq!(resolve_to(&m, 0, "/src/gone.rs"), None);
    assert_eq!(resolve_to(&m, 0, "relative"), None);
    std::fs::remove_dir_all(&dir).unwrap();
}
