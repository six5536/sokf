use std::{fs, path::Path};

use super::*;
use crate::{
    Cache, CacheMode, RepoRoot, api::load_repository, config, config::RepoFormats, grammar,
};

/// `fixture` copies the `indexes` fixture to a directory, named for
/// `name`, under the temp directory, and returns its path.
fn fixture(name: &str) -> std::path::PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/indexes");
    let dir = crate::test_support::fresh_dir(&format!("plan-{name}"));
    copy(&source, &dir);
    dir
}

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn plan_on(dir: &Path, scope: &Scope) -> FixPlan {
    let root = RepoRoot::new(dir.to_path_buf());
    let config = config::load_config(&root).unwrap();
    let grammar = grammar::load_grammar().unwrap();
    let cache = Cache::open(&root, CacheMode::Off);
    let loaded =
        load_repository(&root, &config, &grammar, &RepoFormats::default(), &cache).unwrap();
    plan(
        &loaded.model,
        &loaded.compiled,
        &loaded.expected,
        &loaded.keys,
        scope,
        None,
    )
    .unwrap()
}

fn p(s: &str) -> RepoPath {
    RepoPath::parse(s).unwrap()
}

fn paths(rewrites: &[Rewrite]) -> Vec<&str> {
    rewrites.iter().map(|r| r.path.as_str()).collect()
}

// sokf-tests: SOKF_VALIDATION_33
// sokf-tests: SOKF_VALIDATION_36
// sokf-tests: CLI-4_AC-5
// sokf-tests: SOKF_VALIDATION_55
#[test]
fn indexes_are_planned_in_scope_and_where_a_move_touches() {
    let dir = fixture("scope");
    // With everything in scope, the plan moves the document, then
    // creates some indexes and rewrites others to match the moved tree.
    let all = plan_on(&dir, &Scope::all());
    assert_eq!(all.moves.len(), 1);
    assert_eq!(
        paths(&all.creates),
        vec![
            "/knowledge/issues/closed/index.md",
            "/knowledge/none/index.md"
        ]
    );
    assert_eq!(
        paths(&all.rewrites),
        vec![
            "/knowledge/issues/index.md",
            "/knowledge/issues/open/index.md",
            "/knowledge/sub/index.md"
        ]
    );
    assert!(all.creates.iter().all(|c| c.eol == LineEnding::Lf));
    // When `PATHS` contains only the document, the plan repairs the
    // indexes that the move touches: the directory it leaves, the one it
    // creates, and the parent that gains the new directory. It repairs
    // nothing outside them.
    let mut scope = Scope::default();
    scope.add(p("/knowledge/issues/open/issue-001-move.md"));
    let narrowed = plan_on(&dir, &scope);
    assert_eq!(narrowed.moves.len(), 1);
    assert_eq!(
        paths(&narrowed.creates),
        vec!["/knowledge/issues/closed/index.md"]
    );
    assert_eq!(
        paths(&narrowed.rewrites),
        vec![
            "/knowledge/issues/index.md",
            "/knowledge/issues/open/index.md"
        ]
    );
    // When `PATHS` contains a directory, the plan repairs only that
    // directory's index.
    let mut scope = Scope::default();
    scope.dirs.insert(p("/knowledge/sub"));
    let sub = plan_on(&dir, &scope);
    assert!(sub.moves.is_empty() && sub.creates.is_empty());
    assert_eq!(paths(&sub.rewrites), vec!["/knowledge/sub/index.md"]);
    fs::remove_dir_all(&dir).unwrap();
}

// sokf-tests: CLI-4_AC-107
#[test]
fn a_body_as_written_holds_each_code_span_at_its_columns_in_the_text() {
    let text = "<a id=\"x\"></a><a id=\"y\"></a>`[a](/p.md)` <a id=\"z\"></a>`b` [c](/p.md)\n";
    let body = crate::markdown::parse(text, 1, &crate::markdown::Segments::default());
    assert_eq!(body.anchors.len(), 3, "{body:?}");
    assert_eq!(body.code_spans.len(), 2, "{body:?}");
    let clean = body.clean_line(1, text.trim_end()).into_owned();
    let written = as_written(&body);
    assert!(written.anchors.is_empty(), "{:?}", written.anchors);
    for (parsed, moved) in body.code_spans.iter().zip(&written.code_spans) {
        assert_eq!(&text[moved.1..moved.2], &clean[parsed.1..parsed.2]);
    }
}

#[test]
fn a_value_ends_before_the_next_key() {
    let text = "---\nresource: /k/a.png\ntitle: T\n---\n";
    assert_eq!(value_end(text, 2), 3);
    let text = "---\nresource:\n  - /a\n\n  - /b\n- /c\ntitle: T\n---\n";
    assert_eq!(value_end(text, 2), 7);
    let text = "---\nresource: |\n  /a\n\ntitle: T\n---\n";
    assert_eq!(value_end(text, 2), 4);
    let text = "---\nresource: /a\n---\n  indented body\n";
    assert_eq!(value_end(text, 2), 3);
}

// sokf-tests: SOKF_VALIDATION_20
// sokf-tests: SOKF_VALIDATION_21
#[test]
fn a_refused_move_touches_no_index() {
    let dir = fixture("refused");
    fs::create_dir_all(dir.join("knowledge/issues/closed")).unwrap();
    fs::write(
        dir.join("knowledge/issues/closed/issue-001-move.md"),
        "occupied\n",
    )
    .unwrap();
    let mut scope = Scope::default();
    scope.add(p("/knowledge/issues/open/issue-001-move.md"));
    let out = plan_on(&dir, &scope);
    assert_eq!(out.moves, [] as [crate::fix::plan::Move; 0]);
    assert_eq!(out.refused.len(), 1);
    assert!(out.creates.is_empty() && out.rewrites.is_empty(), "{out:?}");
    fs::remove_dir_all(&dir).unwrap();
}
