use std::path::PathBuf;

use super::*;
use crate::{
    graph::build,
    test_support::{fresh_dir, model_at, write},
};

fn temp(name: &str) -> PathBuf {
    let dir = fresh_dir(&format!("start-{name}"));
    for (rel, text) in [
        (
            "knowledge/manifest.sokf.yaml",
            "sokf: \"0.7\"\nname: k\nscope: test\n",
        ),
        (
            "knowledge/core/a.md",
            "---\ntype: Note\nid: a\nresource: /src/a.rs\n---\n\n# A\n",
        ),
        (
            "knowledge/core/b.md",
            "---\ntype: Note\nid: dup\n---\n\n# B\n",
        ),
        ("knowledge/c.md", "---\ntype: Note\nid: dup\n---\n\n# C\n"),
        (
            "knowledge/core.md",
            "---\ntype: Note\nid: core\n---\n\n# Core\n",
        ),
        (
            "other/manifest.sokf.yaml",
            "sokf: \"0.7\"\nname: o\nscope: test\n",
        ),
        ("other/n.md", "---\ntype: Note\nid: a\n---\n\n# N\n"),
        ("src/a.rs", ""),
        ("src/plain.rs", ""),
        ("docs/x.txt", ""),
    ] {
        write(&dir, rel, text);
    }
    std::fs::create_dir_all(dir.join("knowledge/empty")).unwrap();
    dir
}

fn p(s: &str) -> RepoPath {
    RepoPath::parse(s).unwrap()
}

// sokf-tests: CLI-7_AC-4
// sokf-tests: CLI-7_AC-19
// sokf-tests: CLI-7_AC-20
// sokf-tests: CLI-7_AC-29
// sokf-tests: CLI-1_AC-7
// sokf-tests: CLI-1_AC-8
#[test]
fn each_reading_and_each_refusal() {
    let dir = temp("reading");
    let m = model_at(&dir, &["/knowledge"]);
    let mut g = build(&m);
    let mut start = |arg: &str, cwd: &Path| resolve_start(&mut g, &m, None, cwd, arg);
    // An id that identifies exactly one document starts at that document.
    assert_eq!(
        start("a", &dir).unwrap(),
        Start::Node(NodeKey::Path(p("/knowledge/core/a.md")))
    );
    // A root and a directory in a root each start as a container, in
    // both spellings.
    assert_eq!(
        start("knowledge", &dir).unwrap(),
        Start::Container(p("/knowledge"))
    );
    assert_eq!(
        start("/knowledge", &dir).unwrap(),
        Start::Container(p("/knowledge"))
    );
    assert_eq!(
        start("core/", &dir.join("knowledge")).unwrap(),
        Start::Container(p("/knowledge/core"))
    );
    assert_eq!(
        start("./core", &dir.join("knowledge")).unwrap(),
        Start::Container(p("/knowledge/core"))
    );
    assert_eq!(
        start("empty", &dir.join("knowledge")).unwrap(),
        Start::Container(p("/knowledge/empty"))
    );
    // A document given by its path, a file that an edge points to, and
    // a plain file each start at their own node.
    assert_eq!(
        start("core.md", &dir.join("knowledge")).unwrap(),
        Start::Node(NodeKey::Path(p("/knowledge/core.md")))
    );
    assert_eq!(
        start("src/a.rs", &dir).unwrap(),
        Start::Node(NodeKey::Path(p("/src/a.rs")))
    );
    assert_eq!(
        start("src/plain.rs", &dir).unwrap(),
        Start::Node(NodeKey::Path(p("/src/plain.rs")))
    );
    // A directory outside every knowledge store, the repository root
    // among them, starts outside the stores.
    assert_eq!(start("src", &dir).unwrap(), Start::Outside(p("/src")));
    assert_eq!(start(".", &dir).unwrap(), Start::Outside(p("/")));
    // `resolve_start` refuses a duplicate id, an id that is also a path,
    // and a word that resolves to nothing.
    for (arg, needle) in [
        (
            "dup",
            "identifies more than one document: /knowledge/c.md, /knowledge/core/b.md",
        ),
        (
            "core",
            "is both an id (/knowledge/core.md) and a path (/knowledge/core)",
        ),
        ("nothing", "neither an id nor a path"),
        ("/knowledge/gone.md", "neither an id nor a path"),
    ] {
        let err = start(arg, &dir.join("knowledge")).unwrap_err();
        assert!(matches!(err, Error::Start(_)), "{arg}: {err}");
        assert!(err.to_string().contains(needle), "{arg}: {err}");
    }
    // The graph holds the nodes that the starts resolved to, each with
    // its kind.
    assert_eq!(
        g.kind(&NodeKey::Path(p("/knowledge/empty"))),
        Some(NodeKind::Directory)
    );
    assert_eq!(g.kind(&NodeKey::Path(p("/src"))), Some(NodeKind::Directory));
    assert_eq!(
        g.kind(&NodeKey::Path(p("/src/plain.rs"))),
        Some(NodeKind::File)
    );
    assert_eq!(g.kind(&NodeKey::Path(p("/src/a.rs"))), Some(NodeKind::File));
    assert_eq!(Start::Outside(p("/src")).key(), NodeKey::Path(p("/src")));
    assert_eq!(Start::Node(NodeKey::Missing("x".into())).path(), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-8_AC-6
// sokf-tests: CLI-8_AC-9
// sokf-tests: CLI-8_AC-11
// sokf-tests: CLI-7_AC-30
#[test]
fn references_resolve_across_roots_or_refuse() {
    let dir = temp("ids");
    let m = model_at(&dir, &["/knowledge", "/other"]);
    assert_eq!(
        resolve_id(&m, None, "core").unwrap(),
        p("/knowledge/core.md")
    );
    let err = resolve_id(&m, None, "a").unwrap_err();
    assert!(matches!(err, Error::Locate(_)));
    assert_eq!(
        err.to_string(),
        "locate: `a` identifies more than one document: /knowledge/core/a.md, /other/n.md. Write `k:a` or `o:a`"
    );
    assert_eq!(
        resolve_id(&m, None, "zz").unwrap_err().to_string(),
        "locate: `zz` identifies no document"
    );
    // Each form of a reference resolves: a store given by its whole
    // identity or by its name, the `sokf:` scheme, and a fragment.
    for form in ["o:a", "test/o:a", "sokf:o:a", "o:a#n"] {
        assert_eq!(
            resolve_id(&m, None, form).unwrap(),
            p("/other/n.md"),
            "{form}"
        );
    }
    assert!(resolve_id(&m, None, "nope:a").is_err());
    assert!(is_key_reference("#k") && is_key_reference("sokf:o:#k"));
    assert!(!is_key_reference("o:a#k") && !is_key_reference("a"));
    // `resolve_start` also resolves an id across every root.
    let mut g = build(&m);
    let err = resolve_start(&mut g, &m, None, &dir, "a").unwrap_err();
    assert!(err.to_string().contains("/other/n.md"), "{err}");
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-31
// sokf-tests: CLI-7_AC-97
// sokf-tests: CLI-7_AC-98
// sokf-tests: CLI-7_AC-99
// sokf-tests: CLI-7_AC-100
#[test]
fn a_key_of_document_scope_starts_when_one_document_captures_it() {
    use crate::grammar::KeyScope;
    let dir = temp("document-scope");
    let m = model_at(&dir, &["/knowledge", "/other"]);
    let mut keys = KeyIndex::default();
    for (key, path, scope) in [
        ("X-1", "/knowledge/core/a.md", KeyScope::Document),
        ("Y-1", "/knowledge/core.md", KeyScope::Document),
        ("Y-1", "/knowledge/core/a.md", KeyScope::Document),
        (
            "R-1",
            "/knowledge/core/a.md",
            KeyScope::Host("knowledge".into()),
        ),
        ("R-1", "/knowledge/core.md", KeyScope::Document),
    ] {
        keys.add(KeyEntry {
            id: crate::markdown::slug(key),
            key: key.into(),
            path: p(path),
            line: 3,
            heading_slug: None,
            holds_anchor: true,
            scope,
            knowledge: m.knowledge_of(&p(path)).unwrap(),
            rule_id: None,
        });
    }
    let mut g = build(&m);
    for (path, slug) in [
        ("/knowledge/core/a.md", "x-1"),
        ("/knowledge/core/a.md", "y-1"),
        ("/knowledge/core.md", "y-1"),
    ] {
        g.add_key(&p(path), slug, &slug.to_uppercase(), 3, &[]);
    }
    let key = |s: &str| Start::Node(NodeKey::Key(s.into()));
    let mut start = |arg: &str| resolve_start(&mut g, &m, Some(&keys), &dir, arg);
    // One document captures `X-1`, so `#x-1` starts at the key, in each
    // spelling, and the note names the document. The id `a` is also an
    // id of the store `o`, so the reference gives the store.
    for arg in ["#x-1", "#X-1", "k:#x-1", "sokf:#x-1"] {
        assert_eq!(
            start(arg).unwrap(),
            key("/knowledge/core/a.md#x-1"),
            "{arg}"
        );
    }
    assert_eq!(
        note(&m, Some(&keys), "#x-1").as_deref(),
        Some("`#x-1` is unique only within `k:a`, so link it as `k:a#x-1`")
    );
    // `<id>#<key>` starts at the key that the document captures, and at
    // the document when the fragment names no key of it.
    assert_eq!(start("core#y-1").unwrap(), key("/knowledge/core.md#y-1"));
    assert_eq!(
        start("core#nothing").unwrap(),
        Start::Node(NodeKey::Path(p("/knowledge/core.md")))
    );
    // A key of knowledge scope keeps its start, with no note.
    assert_eq!(
        start("#r-1").unwrap(),
        Start::Node(NodeKey::Path(p("/knowledge/core/a.md")))
    );
    assert_eq!(note(&m, Some(&keys), "#r-1"), None);
    assert_eq!(note(&m, Some(&keys), "core#y-1"), None);
    // Two documents capture `Y-1`, so the error lists the reference of
    // each, in the order of their paths. No document of the store `o`
    // captures it, and no document captures `#none`.
    for (arg, message) in [
        (
            "#y-1",
            "graph: `#y-1` is a key of document scope that more than one document captures. Write `core#y-1` or `k:a#y-1`",
        ),
        (
            "o:#y-1",
            "graph: `o:#y-1` is neither an id nor a path in the repository",
        ),
        (
            "#none",
            "graph: `#none` is neither an id nor a path in the repository",
        ),
    ] {
        let err = start(arg).unwrap_err();
        assert!(matches!(err, Error::Start(_)), "{arg}: {err}");
        assert_eq!(err.to_string(), message);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-7_AC-99
#[test]
fn the_error_lists_ten_references_and_counts_the_rest() {
    let forms = |n: usize| (1..=n).map(|i| format!("`n{i}#y-1`")).collect::<Vec<_>>();
    let ten = (1..=10)
        .map(|i| format!("`n{i}#y-1`"))
        .collect::<Vec<_>>()
        .join(" or ");
    assert_eq!(capped(&forms(2)), "`n1#y-1` or `n2#y-1`");
    assert_eq!(capped(&forms(10)), ten);
    assert_eq!(capped(&forms(11)), format!("{ten}, and 1 more"));
    assert_eq!(capped(&forms(103)), format!("{ten}, and 93 more"));
}

// sokf-tests: CLI-7_AC-28
#[test]
fn a_word_shaped_as_a_path_is_read_only_as_a_path() {
    let dir = temp("path-shaped");
    // Each id breaks the slug rules and has the shape of a path.
    for (rel, id) in [
        ("knowledge/core.md.md", "core.md"),
        ("knowledge/e.md", "core/"),
        ("knowledge/f.md", ".hidden"),
    ] {
        std::fs::write(
            dir.join(rel),
            format!("---\ntype: Note\nid: \"{id}\"\n---\n\n# X\n"),
        )
        .unwrap();
    }
    std::fs::create_dir_all(dir.join(".hidden")).unwrap();
    let m = model_at(&dir, &["/knowledge"]);
    let mut g = build(&m);
    let mut start = |arg: &str, cwd: &Path| resolve_start(&mut g, &m, None, cwd, arg);
    let knowledge = dir.join("knowledge");
    assert_eq!(
        start("core.md", &knowledge).unwrap(),
        Start::Node(NodeKey::Path(p("/knowledge/core.md")))
    );
    assert_eq!(
        start("core/", &knowledge).unwrap(),
        Start::Container(p("/knowledge/core"))
    );
    assert_eq!(
        start(".hidden", &dir).unwrap(),
        Start::Outside(p("/.hidden"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// A reference to a key alone tries the slug of its text before the slug of
// its plain text, and names the key whose id found its document.
// sokf-tests: CLI-7_AC-98
#[test]
fn a_key_alone_takes_the_slug_of_its_text_first() {
    use crate::grammar::KeyScope;
    let dir = temp("text-first");
    let m = model_at(&dir, &["/knowledge", "/other"]);
    let mut keys = KeyIndex::default();
    // A key of a fence `_Short_` has the id `_short_`, and a key of
    // prose `_Short_` the id `short`: the reference `#_Short_` takes the
    // slug of its text first, so it names the key of the fence alone.
    for (id, path) in [
        ("_short_", "/knowledge/core/a.md"),
        ("short", "/knowledge/core.md"),
    ] {
        keys.add(KeyEntry {
            id: id.into(),
            key: "_Short_".into(),
            path: p(path),
            line: 4,
            heading_slug: None,
            holds_anchor: true,
            scope: KeyScope::Document,
            knowledge: m.knowledge_of(&p(path)).unwrap(),
            rule_id: None,
        });
    }
    assert_eq!(
        document_scoped(&m, &keys, "#_Short_"),
        [(p("/knowledge/core/a.md"), "_short_".to_string())]
    );
    // Where the document of the key of the fence also captures a key of
    // prose `_Short_` at knowledge scope, `#_Short_` names that key, by the
    // id that found the document, and not the key of the fence.
    keys.add(KeyEntry {
        id: "short".into(),
        key: "_Short_".into(),
        path: p("/knowledge/core/a.md"),
        line: 5,
        heading_slug: None,
        holds_anchor: true,
        scope: KeyScope::Host("knowledge".into()),
        knowledge: m.knowledge_of(&p("/knowledge/core/a.md")).unwrap(),
        rule_id: None,
    });
    let (entry, slug) = key_entry(&m, &keys, "#_Short_").unwrap();
    assert_eq!((entry.line, slug.as_str()), (5, "short"));
    std::fs::remove_dir_all(&dir).unwrap();
}
