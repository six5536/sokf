//! Tests for schema lowering.

use std::collections::BTreeMap;

use crate::{
    RepoPath, Severity,
    grammar::{
        CitedFrom, Dispatch, FmFormat, FormatSource, KeyScope as Scope, Lang, RegexCache, Shape,
        SokfSchema, States, Text, compile, glob_inside, load_grammar,
    },
    markdown::BlockKind,
    yaml,
};

fn compile_text(text: &str) -> std::result::Result<SokfSchema, Vec<String>> {
    let grammar = load_grammar().unwrap();
    let regexes = RegexCache::new();
    let registry = super::requirements();
    let yaml = yaml::parse(text, 1).unwrap();
    compile(
        &yaml,
        &grammar,
        &registry,
        &regexes,
        &RepoPath::parse("/k/schemas/s.md").unwrap(),
        "s",
    )
    .map(|(schema, _)| schema)
    .map_err(|f| f.into_iter().map(|f| f.message).collect())
}

/// `cited-by` lowers as the inverse of `links`: each entry keeps its
/// types, its rule, and the common keys resolved against the lifecycle.
// sokf-tests: EXT-9_AC-3
// sokf-tests: EXT-9_AC-4
#[test]
fn lowers_cited_by() {
    let c = compile_text(
        "line-limit: 0\nlifecycle:\n  - { key: scope, states: [public, internal] }\nfrontmatter:\n  type: {const: Contract}\ncited-by:\n  - from-type: Stories\n    keys: criteria\n    only: { scope: [public] }\n    severity: warn\n  - from-type: [Overview, Design]\nsections:\n  - heading: Criteria\n    level: 1\n    rule-id: criteria\n    guidance: d\n    blocks:\n      - kind: bullet-list\n        each: {pattern: '^(?P<id>C-\\d+): .+$', key: id, example: 'C-1: it works'}\n",
    )
    .unwrap();
    assert_eq!(c.cited_by.len(), 2);
    let first = &c.cited_by[0];
    assert_eq!(first.from, CitedFrom::Types(vec!["Stories".into()]));
    assert_eq!(first.keys.as_deref(), Some("criteria"));
    assert_eq!(first.common.severity, Severity::Warning);
    assert!(first.common.only.admits(&States(BTreeMap::from([(
        "scope".to_string(),
        vec!["public".to_string()]
    )]))));
    assert!(!first.common.only.admits(&States(BTreeMap::from([(
        "scope".to_string(),
        vec!["internal".to_string()]
    )]))));
    let second = &c.cited_by[1];
    assert_eq!(
        second.from,
        CitedFrom::Types(vec!["Overview".into(), "Design".into()])
    );
    assert_eq!(second.keys, None);
    assert_eq!(second.common.severity, Severity::Error);
}

/// The `only` of a `links`, a `plans-paths`, and a `cited-by` entry is a
/// condition, as a section's is: a list of maps admits the states that any
/// one of its maps admits.
#[test]
fn an_entry_takes_a_condition_of_several_maps() {
    let c = compile_text(
        "line-limit: 0\nlifecycle:\n  - { key: scope, states: [public, internal] }\n  - { key: kind, states: [api, tool] }\nfrontmatter:\n  type: {const: Contract}\nlinks:\n  - rel: relates-to\n    min: 0\n    only: [{ scope: public }, { kind: tool }]\n    guidance: d\nplans-paths:\n  - section: files\n    only: [{ scope: public }, { kind: tool }]\ncited-by:\n  - from-type: Stories\n    only: [{ scope: public }, { kind: tool }]\nsections:\n  - heading: Files\n    level: 1\n    rule-id: files\n    guidance: d\n    blocks:\n      - kind: bullet-list\n        example: '[the file](/src/main.rs)'\n",
    )
    .unwrap();
    let states = |scope: &str, kind: &str| {
        States(BTreeMap::from([
            ("scope".to_string(), vec![scope.to_string()]),
            ("kind".to_string(), vec![kind.to_string()]),
        ]))
    };
    for only in [
        &c.links[0].common.only,
        &c.plans_paths[0].common.only,
        &c.cited_by[0].common.only,
    ] {
        assert!(only.admits(&states("public", "api")));
        assert!(only.admits(&states("internal", "tool")));
        assert!(!only.admits(&states("internal", "api")));
    }
}

/// `cited-by` lowers an entry that counts `implements` markers. `markers`
/// lowers to the list of markers that can cite the type.
// sokf-tests: EXT-9_AC-6
// sokf-tests: EXT-9_AC-11
// sokf-tests: SOKF_SCHEMA_48
#[test]
fn lowers_from_marker_and_markers() {
    let schema = |cited: &str, markers: &str| {
        format!(
            "line-limit: 0\nfrontmatter:\n  type: {{const: Contract}}\n{markers}cited-by:\n{cited}sections:\n  - heading: Criteria\n    level: 1\n    guidance: d\n    blocks:\n      - kind: bullet-list\n        each: {{pattern: '^(?P<id>C-\\d+): .+$', key: id, key-of: criteria, example: 'C-1: it works'}}\n"
        )
    };
    let c = compile_text(&schema(
        "  - from-marker: implements\n    keys: criteria\n",
        "markers: [implements, tests]\n",
    ))
    .unwrap();
    assert_eq!(c.cited_by[0].from, CitedFrom::Marker("implements".into()));
    assert_eq!(
        c.markers,
        Some(vec!["implements".to_string(), "tests".to_string()])
    );
    let none = compile_text(&schema("  - from-marker: tests\n", "markers: []\n")).unwrap();
    assert_eq!(none.markers, Some(vec![]));
    let open = compile_text(&schema("  - from-type: Stories\n", "")).unwrap();
    assert_eq!(open.markers, None);
    // EXT-9_AC-11: a rel of `markers` that isn't lowercase kebab case
    // matches no marker, so the schema is refused, and the message gives
    // the kebab form when there is one.
    for (markers, says, hint) in [
        ("markers: [Implements]\n", "`Implements`", "`implements`"),
        ("markers: [implements, test_of]\n", "`test_of`", "`test-of`"),
        (
            "markers:\n  - implements\n  - TestOf\n",
            "`TestOf`",
            "`test-of`",
        ),
        ("markers: ['-a']\n", "`-a`", "joined by `-`"),
    ] {
        let errs = compile_text(&schema("  - from-type: Stories\n", markers)).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains(says)
                && e.contains("isn't lowercase kebab case")
                && e.contains(hint)),
            "{errs:?}"
        );
    }
    // SOKF_SCHEMA_48: an entry declares exactly one of `from-type` and `from-marker`.
    for cited in [
        "  - from-type: Stories\n    from-marker: tests\n",
        "  - keys: criteria\n",
    ] {
        let errs = compile_text(&schema(cited, "")).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.contains("from-type") && e.contains("from-marker")),
            "{errs:?}"
        );
    }
}

#[test]
fn lowers_the_grammar_guide_schema() {
    let c = compile_text(
        "line-limit: 120\nguidance: One bug or one feature, and what closed it.\nroot: /knowledge/issues\nlifecycle:\n  - { key: state, states: [open, closed], path: true }\n  - { key: kind, states: [bug, feature] }\nfrontmatter:\n  type:\n    const: Issue\n  id:\n    template: 'issue-${:seq}-${:slug}'\n  reviewers:\n    list: true\n    format: document-id\n    min: 1\n    optional: true\nlinks:\n  - rel: implements\n    to-type: [Feature]\n    guidance: d\n  - rel: relates-to\n    min: 0\n    guidance: d\nsections:\n  - heading: { from: title }\n    level: 1\n    guidance: The title.\n  - heading: Reproduction\n    level: 2\n    only: { kind: bug }\n    guidance: d\n    blocks:\n      - kind: code\n        lang: [rust, any]\n      - kind: table\n        columns: [Name, Type]\n        each: { pattern: '^`(?P<name>[a-z-]+)`', key: name, unique: repository, example: '`name` | string' }\n      - kind: group\n        repeat: true\n        blocks:\n          - { kind: prose, max: { hard: 1, when-at-limit: 'Write a second need as a group of its own.' }, each: { format: user-story } }\n  - heading: { template: '${name:word} flow', key: name, example: main flow }\n    repeat: true\n    level: 2\n    rule-id: flows\n    guidance: d\n  - heading: Conclusion\n    level: 2\n    only: { state: closed }\n    renamed-from: [Result]\n    guidance: d\n    blocks: []\nprohibited-patterns:\n  - pattern: '\\bTODO\\b'\n    message: A finished document contains no TODO.\nformats:\n  requirement-ears:\n    pattern: '.*'\n    example: x\nexamples:\n  - document: |\n      ---\n      type: Issue\n      ---\n",
    )
    .unwrap();
    assert_eq!(c.dispatch, Dispatch::Type("Issue".into()));
    assert_eq!(c.core.core.examples.len(), 1);
    assert_eq!(
        c.core.core.guidance.as_deref(),
        Some("One bug or one feature, and what closed it.")
    );
    assert_eq!(c.core.sections[0].guidance.as_deref(), Some("The title."));
    assert_eq!(c.core.core.line_limit, 120);
    assert_eq!(c.root.as_deref(), Some("/knowledge/issues"));
    assert_eq!(c.core.core.lifecycle.len(), 2);
    assert!(c.is_path("state") && !c.is_path("kind"));
    assert!(c.core.core.id_template().is_some());
    assert_eq!(c.core.core.frontmatter.len(), 3);
    let reviewers = &c.core.core.frontmatter[2].1;
    assert!(reviewers.list && reviewers.optional);
    assert_eq!(reviewers.format, Some(FmFormat::Host("document-id".into())));
    assert_eq!(reviewers.min, Some(1));
    assert_eq!(c.links[0].to_type, vec!["Feature"]);
    assert_eq!(c.links[0].min, 1);
    assert_eq!(c.links[1].min, 0);
    assert_eq!(c.core.sections.len(), 4);
    assert!(
        matches!(c.core.sections[0].heading.shape, Shape::Text(Text::From(ref k)) if k == "title")
    );
    assert_eq!(c.core.sections[0].label(), "<title>");
    assert_eq!(c.core.sections[1].label(), "Reproduction");
    assert_eq!(c.core.sections[2].label(), "${name:word} flow");
    assert_eq!(c.core.sections[2].heading.key.as_deref(), Some("name"));
    let repro = &c.core.sections[1];
    assert!(repro.common.only.is_gated());
    let bug = States(BTreeMap::from([(
        "kind".to_string(),
        vec!["bug".to_string()],
    )]));
    let feature = States(BTreeMap::from([(
        "kind".to_string(),
        vec!["feature".to_string()],
    )]));
    assert!(repro.common.only.admits(&bug));
    assert!(!repro.common.only.admits(&feature));
    assert!(!repro.common.only.admits(&States::default()));
    let blocks = repro.blocks.as_ref().unwrap();
    assert_eq!(
        blocks[0].lang,
        Some(Lang::Tags(vec!["rust".into(), "any".into()]))
    );
    assert_eq!(
        blocks[1].columns.as_ref().unwrap(),
        &vec!["Name".to_string(), "Type".to_string()]
    );
    assert_eq!(
        blocks[1].each.as_ref().unwrap().unique,
        Scope::Host("repository".into())
    );
    assert_eq!(blocks[2].kind, BlockKind::Group);
    assert_eq!(c.core.sections[2].rule_id.as_deref(), Some("flows"));
    assert_eq!(c.core.rule_ids(), vec!["flows"]);
    assert_eq!(c.core.sections[3].renamed_from, vec!["Result"]);
    assert_eq!(c.core.sections[3].blocks.as_ref().unwrap().len(), 0);
    assert_eq!(
        c.core.core.prohibited_patterns[0].pattern.as_str(),
        r"\bTODO\b"
    );
    assert_eq!(
        c.core.core.formats.entries["requirement-ears"].1,
        FormatSource::Schema("s".into())
    );
}

/// A schema's templates can use the types that the schema declares. Those
/// types also join the formats that the schema can use.
#[test]
fn a_schema_writes_its_templates_with_its_own_types() {
    let c = compile_text(
        "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  code: {template: '${:requirement-code}', example: LINK}\ntypes:\n  kind: [Performance, Capacity]\nsections:\n  - heading: { template: '${:int}.${:int} ${id:requirement-id} ${:kind}', key: id, example: 2.1 LINK-3 Performance }\n    level: 1\n    guidance: d\n    blocks: []\n",
    )
    .unwrap();
    assert!(
        c.core.sections[0]
            .heading
            .shape
            .matches("2.1 LINK-3 Capacity", |_| None)
    );
    assert!(
        !c.core.sections[0]
            .heading
            .shape
            .matches("2.1 LINK-3 Speed", |_| None)
    );
    assert!(c.core.core.formats.types_seen().get("kind").is_some());
    assert!(
        c.core
            .core
            .formats
            .types_seen()
            .get("criterion-id")
            .is_some()
    );
}

#[test]
fn additional_files_and_any_lang() {
    let c = compile_text("line-limit: 0\nadditional-files: '/README.md'\npreamble:\n  guidance: d\n  blocks:\n    - kind: code\n      lang: any\n").unwrap();
    assert_eq!(c.dispatch, Dispatch::Globs(vec!["/README.md".into()]));
    assert_eq!(
        c.core.preamble.as_ref().unwrap().blocks.as_ref().unwrap()[0].lang,
        Some(Lang::Any)
    );
    assert!(c.core.core.id_template().is_none());
    assert!(c.core.core.dimension("x").is_none());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the test lists one case for each refusal, in one table"
)]
fn semantic_refusals() {
    let cases: Vec<(&str, &str)> = vec![
        ("line-limit: 0\nsections: []\n", "neither"),
        (
            "line-limit: 0\nadditional-files: /x\nfrontmatter:\n  type:\n    const: X\nsections: []\n",
            "both",
        ),
        (
            "line-limit: 0\nadditional-files: x\nsections: []\n",
            "does not start with /",
        ),
        (
            "line-limit: 0\nadditional-files: '/['\nsections: []\n",
            "does not parse",
        ),
        (
            "line-limit: 0\nadditional-files: /x\nlifecycle:\n  - {key: s, states: [a]}\nsections: []\n",
            "together with `additional-files`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a], path: true}\nsections: []\n",
            "needs `root`",
        ),
        (
            "line-limit: 0\nroot: k\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a], path: true}\nsections: []\n",
            "is not a valid repository path",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a]}\n  - {key: s, states: [b]}\nsections: []\n",
            "declared twice",
        ),
        // sokf-tests: EXT-7_AC-2
        // sokf-tests: SOKF_SCHEMA_50
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a, b], archived: [c]}\nsections: []\n",
            "`archived` state `c` is not a state of the dimension `s`",
        ),
        // sokf-tests: EXT-2_AC-3
        (
            "line-limit: 0\narchived: true\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a]}\nsections: []\n",
            "`archived` together with `lifecycle`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    only: {s: a}\n    guidance: d\n",
            "`only` without a lifecycle",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a]}\nsections:\n  - heading: A\n    level: 1\n    only: {t: a}\n    guidance: d\n",
            "does not declare",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nlifecycle:\n  - {key: s, states: [a]}\nsections:\n  - heading: A\n    level: 1\n    only: {s: z}\n    guidance: d\n",
            "which is no state of `s`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n  - heading: B\n    level: 3\n    guidance: d\n",
            "skips a level",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: { pattern: '^(?P<k>a)$', key: k, example: a }\n    level: 1\n    guidance: d\n    for-each-key-of: nope\n",
            "`for-each-key-of` names no set `nope`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: { pattern: '^(?P<k>a)$', key: k, example: a }\n    level: 1\n    guidance: d\n    for-each-key-of: bare\n    blocks:\n      - kind: prose\n        rule-id: bare\n        each: {pattern: '^b$', example: b}\n",
            "`for-each-key-of` names set `bare`, which holds no key",
        ),
        // sokf-tests: EXT-9_AC-5
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\ncited-by:\n  - from-type: Stories\n    keys: nope\nsections: []\n",
            "`keys` names no set `nope`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\ncited-by:\n  - from-type: Stories\n    keys: bare\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        rule-id: bare\n        each: {pattern: '^b$', example: b}\n",
            "`keys` names set `bare`, which holds no key",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\ncited-by:\n  - keys: a\nsections: []\n",
            "from-type",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\ncited-by:\n  - from-type: Stories\n    min: 1\nsections: []\n",
            "min",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\ncited-by:\n  - from-type: Stories\n    only: {scope: [public]}\nsections: []\n",
            "`only` without a lifecycle",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: { template: 'A ${k:int}', key: k, key-derives-from: {rule: nope, separator: _}, example: A 1 }\n    level: 1\n    guidance: d\n",
            "`key-derives-from` names no set `nope`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        each: {template: '${k:word}: ${}', key: k, key-derives-from: {from: code, separator: _}, example: 'X_1: y'}\n",
            "`key-derives-from` names no frontmatter key `code`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  code: {pattern: '^[A-Z]+$', example: X}\nsections:\n  - heading: { template: 'A ${k:int}', key: k, key-derives-from: {rule: a, from: code, separator: _}, example: A 1 }\n    level: 1\n    guidance: d\n",
            "exactly one of",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  code: {pattern: '^[A-Z]+$', example: X}\nsections:\n  - heading: { template: 'A ${k:int}', key-derives-from: {from: code, separator: '-'}, example: A 1 }\n    level: 1\n    guidance: d\n",
            "requires",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    rule-id: a\n    guidance: d\n  - heading: B\n    level: 1\n    rule-id: a\n    guidance: d\n",
            "declared twice",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: code\n",
            "requires `lang`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: table\n",
            "requires `columns`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: group\n",
            "requires `blocks`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        each: {format: nope}\n",
            "`format` names no format `nope`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        each: {format: requirement-ears, key: id, example: x}\n",
            "captures a key itself",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        each: {pattern: 'x', key: k, example: x}\n",
            "`key` `k` names no hole or named group of the rule's shape",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: { pattern: '^x$', key: k, example: x }\n    level: 1\n    guidance: d\n",
            "`key` `k` names no hole or named group of the rule's shape",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  id: {template: 'x-${:seq}-${:seq}'}\nsections: []\n",
            "more than one",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  other: {template: 'x-${:seq}', seq-width: 2, example: x-01}\nsections: []\n",
            "`seq-width` applies to the `id` template alone",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections: []\nformats:\n  f: {example: e, pattern: '('}\n",
            "not a valid regex",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  s: {enum: [a]}\nlifecycle:\n  - {key: s, states: [a]}\nsections: []\n",
            "it is not repeated under `frontmatter`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  id: {template: 'x-${:seq}', enum: [x]}\nsections: []\n",
            "`id` takes `template` alone",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\n  id: {template: 'x-${:int}'}\nsections: []\n",
            "an id holds only the holes a new document is filled with",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: 'A ${x}'\n    level: 1\n    guidance: d\n",
            "holds a hole",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: { template: 'A ${x}' }\n    level: 1\n    guidance: d\n",
            "a rule that declares `template` carries an `example` of it",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        each: {text: a, pattern: b}\n",
            "a rule has one shape",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        each: {template: 'a ${x:nope}', example: a b}\n",
            "no type `nope`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\ntypes:\n  text: x\nsections: []\n",
            "reserved",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: A\n    level: 1\n    for-each-key-of: a\n    guidance: d\n",
            "the heading declares `key`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections:\n  - heading: { template: 'A ${x}', example: A b }\n    renamed-from: [B]\n    level: 1\n    guidance: d\n",
            "`renamed-from` beside a heading that varies",
        ),
    ];
    // A schema with no path-enforced dimension can declare a `root` (SOKF_SCHEMA_27).
    assert!(
        compile_text("line-limit: 0\nroot: /k\nfrontmatter:\n  type: {const: X}\nsections: []\n")
            .is_ok()
    );
    for (text, needle) in cases {
        let err = compile_text(text).unwrap_err();
        assert!(
            err.iter().any(|m| m.contains(needle)),
            "expected {needle:?} in {err:?} for:\n{text}"
        );
    }
}

// sokf-tests: EXT-2_AC-1
// sokf-tests: EXT-2_AC-2
#[test]
fn asset_files_dispatch_and_glob_rules() {
    // The schema is at `/k/schemas/s.md`, so its knowledge root is `/k`.
    let ok = compile_text(
        "line-limit: 0\nroot: /k/media\nlifecycle:\n  - {key: s, states: [a], path: true}\nfrontmatter:\n  type: {const: Image}\nasset-files: ['/k/media/**/*.png', '/k/media/**/*.jpg']\nsections: []\n",
    )
    .unwrap();
    assert_eq!(ok.dispatch, Dispatch::Type("Image".into()));
    assert_eq!(
        ok.asset_files,
        vec!["/k/media/**/*.png", "/k/media/**/*.jpg"]
    );
    assert!(ok.has_assets());
    let plain =
        compile_text("line-limit: 0\nfrontmatter:\n  type: {const: X}\nsections: []\n").unwrap();
    assert!(!plain.has_assets());
    let cases: Vec<(&str, &str)> = vec![
        (
            "line-limit: 0\nadditional-files: /README.md\nasset-files: '/k/*.png'\nsections: []\n",
            "`asset-files` together with `additional-files`",
        ),
        (
            "line-limit: 0\nasset-files: '/k/*.png'\nsections: []\n",
            "needs a `type` const",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nasset-files: 'k/*.png'\nsections: []\n",
            "does not start with /",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nasset-files: '/k/['\nsections: []\n",
            "does not parse",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nasset-files: '/docs/*.png'\nsections: []\n",
            "lies outside the knowledge root `/k`",
        ),
        (
            "line-limit: 0\nfrontmatter:\n  type: {const: X}\nasset-files: '/kb/*.png'\nsections: []\n",
            "lies outside the knowledge root `/k`",
        ),
        (
            "line-limit: 0\nroot: /k/media\nlifecycle:\n  - {key: s, states: [a], path: true}\nfrontmatter:\n  type: {const: X}\nasset-files: '/k/media/*.png'\nsections: []\n",
            "does not begin with `/k/media/**/`",
        ),
    ];
    for (text, needle) in cases {
        let messages = compile_text(text).unwrap_err();
        assert!(
            messages.iter().any(|m| m.contains(needle)),
            "{needle}: {messages:?}"
        );
    }
    assert!(glob_inside(
        "/k/media/**/*.png",
        &RepoPath::parse("/k").unwrap()
    ));
    assert!(glob_inside("/k/*.png", &RepoPath::parse("/k").unwrap()));
    assert!(!glob_inside("/kb/*.png", &RepoPath::parse("/k").unwrap()));
    assert!(!glob_inside("/*/x.png", &RepoPath::parse("/k").unwrap()));
}

/// `arb_id_template` returns the id template of a schema, with a lifecycle
/// hole or not and a slug or not, and its sequence width.
fn arb_id_template() -> impl proptest::strategy::Strategy<Value = (String, usize)> {
    use proptest::prelude::*;
    ("[a-z]{1,4}", 1usize..6, any::<bool>(), any::<bool>()).prop_map(
        |(prefix, width, with_slug, with_kind)| {
            let kind = if with_kind { "-${kind}" } else { "" };
            let slug = if with_slug { "-${:slug}" } else { "" };
            (format!("{prefix}-${{:seq}}{kind}{slug}"), width)
        },
    )
}

// sokf-tests: EXT_P-3
proptest::proptest! {
    #![proptest_config(crate::proptest_cases::cases(64))]

    /// The id that a schema's id template renders for a sequence number is
    /// a slug and matches the template, and reading the id with the template
    /// returns the number and the state.
    #[test]
    fn an_id_template_renders_an_id_it_reads_back(
        (template, width) in arb_id_template(),
        seq in 0u64..10_000,
        slug in "[a-z0-9]+(-[a-z0-9]+){0,2}",
        state in proptest::sample::select(vec!["feature", "bug", "chore"]),
    ) {
        let c = compile_text(&format!(
            "line-limit: 0\nlifecycle:\n  - {{ key: kind, states: [feature, bug, chore] }}\nfrontmatter:\n  type:\n    const: Issue\n  id:\n    template: '{template}'\n    seq-width: {width}\nsections:\n  - heading: {{ from: title }}\n    level: 1\n    guidance: The title.\n"
        ))
        .unwrap();
        let t = c.core.core.id_template().unwrap();
        let mut states = States::default();
        states.0.insert("kind".into(), vec![state.to_string()]);
        let rendered = t.render_with(seq, Some(&slug), &states);
        proptest::prop_assert_eq!(rendered.is_ok(), seq.to_string().len() <= width);
        if let Ok(id) = rendered {
            proptest::prop_assert!(crate::markdown::is_slug(&id), "{id}");
            proptest::prop_assert!(t.matches(&id));
            proptest::prop_assert_eq!(t.seq_of(&id), Some(seq));
            if t.has_keys() {
                proptest::prop_assert_eq!(t.keys_of(&id), vec![("kind".to_string(), state.to_string())]);
            }
        }
    }
}

/// A schema whose only findings are warnings compiles, and `compile`
/// returns the warnings with it; an error of the lowering beside a warning
/// drops the schema, and `compile` returns both.
// sokf-tests: EXT-1_AC-5
#[test]
fn a_schema_with_only_warnings_compiles_with_them() {
    let grammar = load_grammar().unwrap();
    let regexes = RegexCache::new();
    let registry = super::requirements();
    let path = RepoPath::parse("/k/schemas/s.md").unwrap();
    let warned = "line-limit: 0\ncurrent-links: ['^__Planned__: ']\nfrontmatter:\n  type: {const: Note}\nsections: []\n";
    let (_, warnings) = compile(
        &yaml::parse(warned, 1).unwrap(),
        &grammar,
        &registry,
        &regexes,
        &path,
        "s",
    )
    .unwrap();
    let keys: Vec<(Severity, &str)> = warnings
        .iter()
        .map(|f| (f.severity, f.criterion.as_str()))
        .collect();
    assert_eq!(keys, [(Severity::Warning, "SOKF_SCHEMA_110")]);
    let broken = "line-limit: 0\ncurrent-links: ['^__Planned__: ']\nsections: []\n";
    let findings = compile(
        &yaml::parse(broken, 1).unwrap(),
        &grammar,
        &registry,
        &regexes,
        &path,
        "s",
    )
    .unwrap_err();
    let keys: Vec<(Severity, &str)> = findings
        .iter()
        .map(|f| (f.severity, f.criterion.as_str()))
        .collect();
    assert_eq!(
        keys,
        [
            (Severity::Error, "SOKF_SCHEMA_3"),
            (Severity::Warning, "SOKF_SCHEMA_110")
        ],
        "{findings:?}"
    );
}
