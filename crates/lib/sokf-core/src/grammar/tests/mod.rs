//! These tests check the grammar as SOKF extends it: the merge of the
//! extension into the core grammar, the extension's formats, and its lowering.

mod formats;
mod lower;

use super::*;

/// `requirements` returns the fixture's formats of the lines of
/// requirements documents, as an embedded layer.
fn requirements() -> FormatRegistry {
    crate::test_support::requirement_formats(&load_grammar().unwrap())
}

#[test]
fn the_extension_adds_to_the_core() {
    let core = tramlines_md::grammar::load_grammar().unwrap();
    let g = load_grammar().unwrap();
    assert!(core.section("schema-document").is_none());
    assert!(
        g.section("schema-document")
            .unwrap()
            .get("frontmatter")
            .is_some()
    );
    assert!(core.schema.key("links").is_none());
    assert!(g.schema.key("links").is_some());
    assert!(g.accepts_common("link"));
    assert_eq!(
        g.rule("unit").unwrap().key("unique").unwrap().enum_,
        vec!["document", "knowledge", "repository"]
    );
    assert_eq!(
        g.rule("block")
            .unwrap()
            .key("source-pattern")
            .unwrap()
            .requires_kind,
        vec!["image", "include"]
    );
    assert!(g.rule("lifecycle-dimension").unwrap().key("path").is_some());
    assert!(grammar_text().starts_with("grammar: tramlines\n"));
    assert!(grammar_text().contains("---\ngrammar: tramlines-md\n"));
    assert!(grammar_text().contains("---\nextension: sokf\n"));
}

#[test]
fn an_extension_only_adds() {
    let mut g = tramlines_md::grammar::load_grammar().unwrap();
    let err = |g: &mut Grammar, text: &str| g.extend(text).unwrap_err().to_string();
    assert!(err(&mut g.clone(), "extends: tramlines-md\n").contains("no `extension` name"));
    assert!(err(&mut g.clone(), "extension: x\nextends: other\n").contains("not `tramlines-md`"));
    assert!(
        err(
            &mut g.clone(),
            "extension: x\nextends: tramlines-md\nschema:\n  keys:\n    line-limit:\n      required: false\n"
        )
        .contains("`schema.line-limit` changes the core grammar's `required`")
    );
    assert!(
        err(
            &mut g.clone(),
            "extension: x\nextends: tramlines-md\nmeta: {}\n"
        )
        .contains("`meta`")
    );
    g.extend("extension: x\nextends: tramlines-md\nschema:\n  keys:\n    extra: {type: boolean}\n")
        .unwrap();
    assert!(g.schema.key("extra").is_some());
}

#[test]
fn the_ref_type_takes_every_canonical_target() {
    let registry = embedded_formats(&load_grammar().unwrap(), &RegexCache::new()).unwrap();
    let types = registry.types_seen();
    let whole =
        |name: &str| regex::Regex::new(&format!("^{}$", types.get(name).unwrap().regex())).unwrap();
    let re = whole("ref");
    for ok in [
        "req-cli",
        "req-cli#CLI-1_AC-2",
        "flowers#",
        "#user",
        "a#4-x",
        "sokf-spec:sokf-format-spec#9",
        "sokf-spec:#user",
        "acme/common:glossary",
    ] {
        assert!(re.is_match(ok), "{ok}");
    }
    for bad in ["Req", "#", "a b", "/a.md", "", "a:", "a/b:c/d:e", ":x"] {
        assert!(!re.is_match(bad), "{bad}");
    }
    let store = whole("store");
    for ok in ["", "sokf-spec:", "acme/common:"] {
        assert!(store.is_match(ok), "{ok}");
    }
    for bad in ["sokf-spec", "a/b", "A:"] {
        assert!(!store.is_match(bad), "{bad}");
    }
    // The registry reserves `ref`, `store`, and `slug`, as it reserves the
    // engine's own built-in types.
    assert!(types.reserves("ref") && types.reserves("store") && types.reserves("slug"));
}
