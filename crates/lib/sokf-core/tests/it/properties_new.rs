//! These property tests check how `sokf new` gates a template, fills its
//! identity, writes a skeleton, and refuses bad input.

use std::{collections::BTreeMap, fmt::Write as _};

use proptest::prelude::*;
use sokf_core::{
    grammar::{Only, States},
    markdown::is_only_marker,
    scaffold,
};

/// One part of a generated template: its gate, its kind, and its position.
#[derive(Debug, Clone)]
struct Part {
    only: Option<Only>,
    kind: PartKind,
    n: usize,
}

#[derive(Debug, Clone, Copy)]
enum PartKind {
    Key,
    Section,
    Block,
    Fence,
}

fn only(state: Option<&str>, kind: Option<&str>) -> Option<Only> {
    let mut map = BTreeMap::new();
    if let Some(s) = state {
        map.insert("state".to_string(), vec![s.to_string()]);
    }
    if let Some(k) = kind {
        map.insert("kind".to_string(), vec![k.to_string()]);
    }
    if map.is_empty() {
        None
    } else {
        Some(Only(map))
    }
}

fn marker_text(o: &Only) -> String {
    o.0.iter()
        .map(|(k, v)| format!("{k}={}", v.join(",")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn arb_only() -> impl Strategy<Value = Option<Only>> {
    (
        prop::option::of(prop::sample::select(vec!["open", "closed"])),
        prop::option::of(prop::sample::select(vec!["bug", "feature"])),
    )
        .prop_map(|(s, k)| only(s, k))
}

fn arb_states() -> impl Strategy<Value = States> {
    (
        prop::sample::select(vec!["open", "closed"]),
        prop::sample::select(vec!["bug", "feature"]),
    )
        .prop_map(|(s, k)| {
            States(BTreeMap::from([
                ("state".to_string(), vec![s.to_string()]),
                ("kind".to_string(), vec![k.to_string()]),
            ]))
        })
}

/// `arb_parts` generates keys, then blocks and fences, then sections. A block
/// after a section belongs to that section, so only this order keeps the
/// parts independent.
fn arb_parts() -> impl Strategy<Value = Vec<Part>> {
    let key = arb_only().prop_map(|o| (o, PartKind::Key));
    let block = (
        arb_only(),
        prop::sample::select(vec![PartKind::Block, PartKind::Fence]),
    );
    let section = arb_only().prop_map(|o| (o, PartKind::Section));
    (
        prop::collection::vec(key, 0..3),
        prop::collection::vec(block, 0..3),
        prop::collection::vec(section, 1..4),
    )
        .prop_map(|(k, b, s)| {
            k.into_iter()
                .chain(b)
                .chain(s)
                .enumerate()
                .map(|(n, (only, kind))| Part { only, kind, n })
                .collect()
        })
}

/// `render` writes the template with its markers. Given `states`, it instead
/// writes the expected result of gating the template, without markers.
fn render(parts: &[Part], states: Option<&States>) -> String {
    let admitted = |p: &Part| states.is_none_or(|s| p.only.as_ref().is_none_or(|o| o.admits(s)));
    let mut fm = String::from("---\ntype: Issue\nid: x\n");
    let mut body = String::from("\n# Title\n\nIntro.\n\n");
    for p in parts {
        let marker = p.only.as_ref().map(marker_text);
        match p.kind {
            PartKind::Key => {
                if !admitted(p) {
                    continue;
                }
                match (&marker, states) {
                    (Some(m), None) => {
                        let _ = write!(fm, "k{}: # sokf:only {m}\n  - deep\n", p.n);
                    }
                    _ => {
                        let _ = write!(fm, "k{}:\n  - deep\n", p.n);
                    }
                }
            }
            PartKind::Section | PartKind::Block => {
                if !admitted(p) {
                    continue;
                }
                if let (Some(m), None) = (&marker, states) {
                    let _ = writeln!(body, "<!-- sokf:only {m} -->");
                }
                if matches!(p.kind, PartKind::Section) {
                    let _ = write!(body, "## S{}\n\nPara {}.\n\n### Sub\n\nMore.\n\n", p.n, p.n);
                } else {
                    // A code fence is always a block of its own, whereas
                    // adjacent paragraphs merge into one prose block and
                    // adjacent lists into one loose list.
                    let _ = write!(body, "```\nblock {}\n```\n\n", p.n);
                }
            }
            PartKind::Fence => {
                // A marker inside a fence is text and is never gated.
                body.push_str("```\n<!-- sokf:only state=closed -->\ntext\n```\n\n");
            }
        }
    }
    format!("{fm}---\n{body}")
}

// sokf-tests: EXT_P-4
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn gate_is_the_copying_rule(parts in arb_parts(), states in arb_states()) {
        let template = render(&parts, None);
        let gated = scaffold::gate(&template, &states);
        let expected = render(&parts, Some(&states));
        prop_assert_eq!(&gated, &expected);
        let mut in_fence = false;
        for line in gated.lines() {
            if line.starts_with("```") {
                in_fence = !in_fence;
            }
            prop_assert!(in_fence || !is_only_marker(line), "{line}");
        }
    }
}

/// One frontmatter line of a generated template.
#[derive(Debug, Clone)]
enum FmLine {
    Other(usize, bool),
    Id,
    Title,
    State(&'static str),
    Verified,
    Generated,
}

fn arb_frontmatter() -> impl Strategy<Value = Vec<FmLine>> {
    let line = prop_oneof![
        (0usize..5, any::<bool>()).prop_map(|(n, deep)| FmLine::Other(n, deep)),
        Just(FmLine::Title),
        Just(FmLine::State("state")),
        Just(FmLine::State("kind")),
        Just(FmLine::Verified),
        Just(FmLine::Generated),
    ];
    prop::collection::vec(line, 0..7).prop_map(|mut v| {
        v.insert(0, FmLine::Id);
        v
    })
}

fn render_frontmatter(lines: &[FmLine], filled: Option<&scaffold::Identity>) -> String {
    let mut out = String::from("---\ntype: Issue\n");
    let mut seen_title = false;
    let mut seen = Vec::new();
    for l in lines {
        match l {
            FmLine::Other(n, deep) => {
                let _ = writeln!(out, "other{n}: value {n}");
                if *deep {
                    out.push_str("  - nested\n\n  - more\n");
                }
            }
            FmLine::Id => match filled {
                Some(i) => {
                    let _ = writeln!(out, "id: {}", i.id);
                    if !lines.iter().any(|l| matches!(l, FmLine::Title)) {
                        let _ = writeln!(out, "title: {}", i.title);
                    }
                    for key in ["state", "kind"] {
                        if !lines
                            .iter()
                            .any(|l| matches!(l, FmLine::State(k) if *k == key))
                        {
                            let _ = writeln!(out, "{key}: {}", i.states.0[key][0]);
                        }
                    }
                }
                None => out.push_str("id: issue-001-example\n"),
            },
            FmLine::Title => {
                if seen_title {
                    continue;
                }
                seen_title = true;
                match filled {
                    Some(i) => {
                        let _ = writeln!(out, "title: {}", i.title);
                    }
                    None => out.push_str("title: Example\n"),
                }
            }
            FmLine::State(key) => {
                if seen.contains(key) {
                    continue;
                }
                seen.push(*key);
                match filled {
                    Some(i) => {
                        let _ = writeln!(out, "{key}: {}", i.states.0[*key][0]);
                    }
                    None => {
                        let _ = writeln!(out, "{key}: placeholder");
                    }
                }
            }
            FmLine::Verified => {
                if filled.is_none() {
                    out.push_str("verified:\n  - by: human:a\n    at: 2026-01-01T00:00:00Z\n");
                }
            }
            FmLine::Generated => {
                if filled.is_none() {
                    out.push_str("generated: { by: x, at: y }\n");
                }
            }
        }
    }
    out.push_str("---\n\n# Example\n");
    out
}

// sokf-tests: CLI_P-23
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn fill_touches_only_the_identity(lines in arb_frontmatter(), states in arb_states(), n in 1u64..999) {
        let schema = issue_schema();
        let template = render_frontmatter(&lines, None);
        let identity = scaffold::identity(&format!("issue-{n:03}-x"), Some("x"), None, states, None);
        let filled = scaffold::fill(&template, &schema, &identity);
        prop_assert_eq!(&filled, &render_frontmatter(&lines, Some(&identity)));
    }
}

/// `issue_schema` returns the schema that the property tests use. Its
/// lifecycle has two dimensions, `state` and `kind`.
fn issue_schema() -> sokf_core::grammar::SokfSchema {
    use sokf_core::grammar::LifecycleDimension;
    let mut c = sokf_core::grammar::Schema::demand_nothing();
    c.core.lifecycle = vec![
        LifecycleDimension {
            key: "state".into(),
            states: vec!["open".into(), "closed".into()],
            guidance: None,
            line: 1,
            extensions: BTreeMap::default(),
        },
        LifecycleDimension {
            key: "kind".into(),
            states: vec!["bug".into(), "feature".into()],
            guidance: None,
            line: 2,
            extensions: BTreeMap::default(),
        },
    ];
    sokf_core::grammar::SokfSchema {
        paths: vec!["state".into()],
        ..sokf_core::grammar::SokfSchema::of_schema_document(c)
    }
}

use common::{TempTree, new_options, not_missing, on};
use sokf_core::{CacheMode, NewOptions};

use crate::common;

/// `arb_invocation` generates a legal invocation of `new` on the fixture: a
/// type, its states, and its slug or id.
fn arb_invocation() -> impl Strategy<Value = NewOptionsSpec> {
    prop_oneof![
        (
            prop::sample::select(vec!["open", "closed"]),
            prop::sample::select(vec!["bug", "feature"]),
            "[a-z]{1,6}(-[a-z]{1,6}){0,2}",
        )
            .prop_map(|(s, k, slug)| NewOptionsSpec::Issue(s, k, slug)),
        "[a-z]{2,8}".prop_map(NewOptionsSpec::Note),
        prop::sample::select(vec!["draft", "published"]).prop_map(NewOptionsSpec::Image),
    ]
}

#[derive(Debug, Clone)]
enum NewOptionsSpec {
    Issue(&'static str, &'static str, String),
    Note(String),
    Image(&'static str),
}

fn options_of(tree: &TempTree, spec: &NewOptionsSpec) -> NewOptions {
    match spec {
        NewOptionsSpec::Issue(s, k, slug) => {
            let mut o = new_options(tree.dir(), "Issue");
            o.slug = Some(slug.clone());
            o.states = vec![
                ("state".to_string(), s.to_string()),
                ("kind".to_string(), k.to_string()),
            ];
            o
        }
        NewOptionsSpec::Note(id) => {
            let mut o = new_options(tree.dir(), "Note");
            o.id = Some(format!("note-{id}"));
            let mut title = id.clone();
            if let Some(first) = title.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            o.title = Some(title);
            o
        }
        NewOptionsSpec::Image(state) => {
            let mut o = new_options(tree.dir(), "Image");
            o.id = Some(format!("pic-{state}"));
            o.states = vec![("state".to_string(), state.to_string())];
            let file = match *state {
                "draft" => "knowledge/media/draft/flowers.png",
                _ => "knowledge/media/published/moved.png",
            };
            o.for_asset = Some(file.into());
            o
        }
    }
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    /// The check of an untouched skeleton reports only the parts that its
    /// writer has still to write.
    // sokf-tests: CLI_P-24
    #[test]
    fn the_skeleton_passes_its_templates_checks(spec in arb_invocation()) {
        let tree = TempTree::fixture("new");
        let mut o = options_of(&tree, &spec);
        o.write = true;
        let r = sokf_core::new(&tree.root(), &o).unwrap();
        let report = tree.validate(true, CacheMode::Off);
        let left = not_missing(&report, r.path.as_str());
        prop_assert!(left.is_empty(), "{left:?}");
    }
}

// sokf-tests: CLI-9_AC-1
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn nothing_on_refusal(spec in arb_invocation(), which in 0usize..4) {
        let tree = TempTree::fixture("new");
        let before = tree.files();
        let mut o = options_of(&tree, &spec);
        o.write = true;
        match which {
            0 => o.states.push(("phase".into(), "x".into())),
            1 => o.type_name = "Nope".into(),
            2 => o.dir = Some("src".into()),
            _ => {
                o.id = Some("Not Slug".into());
                o.slug = None;
            }
        }
        prop_assert!(sokf_core::new(&tree.root(), &o).is_err());
        prop_assert_eq!(tree.files(), before);
    }
}

/// A generated section records the state it is gated on, the kind of block
/// it holds, whether a pattern constrains the block's text, and, if none
/// does, whether the block has an `example`. A pattern must come with an
/// example. Otherwise, the grammar refuses the schema before sokf tries to
/// derive a scaffold from it.
#[derive(Debug, Clone)]
struct Shape {
    gate: Option<&'static str>,
    kind: &'static str,
    shaped: bool,
    example: bool,
}

fn arb_shape() -> impl Strategy<Value = Shape> {
    (
        prop::sample::select(vec![None, Some("draft"), Some("done")]),
        prop::sample::select(vec![
            "prose",
            "bullet-list",
            "numbered-list",
            "blockquote",
            "code",
            "table",
        ]),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(gate, kind, shaped, example)| Shape {
            gate,
            kind,
            shaped,
            example,
        })
}

/// `schema_of` writes a schema with one section for each shape, and the
/// schema declares no `templates`.
fn schema_of(shapes: &[Shape]) -> String {
    let mut s = String::from(
        "---\ntype: Schema\nid: schema-gen\ndescription: A schema from which sokf derives a template.\n---\n\n````yaml\nline-limit: 0\nlifecycle:\n  - { key: state, states: [draft, done] }\nfrontmatter:\n  type:\n    const: Gen\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n",
    );
    for (i, shape) in shapes.iter().enumerate() {
        let _ = write!(s, "  - heading: S{i}\n    level: 2\n");
        if let Some(g) = shape.gate {
            let _ = writeln!(s, "    only: {{ state: {g} }}");
        }
        let _ = write!(s, "    blocks:\n      - kind: {}\n", shape.kind);
        if shape.kind == "code" {
            s.push_str("        lang: text\n");
        }
        if shape.kind == "table" {
            s.push_str("        columns: [Name, Meaning]\n");
        }
        let text = if shape.kind == "table" {
            "ONE | what it means"
        } else {
            "ONE thing"
        };
        if shape.shaped {
            let _ = write!(
                s,
                "        each:\n          pattern: '^ONE'\n          example: {text}\n"
            );
        } else if shape.example {
            let _ = writeln!(s, "        example: {text}");
        }
    }
    s.push_str("````\n");
    s
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    /// Deriving a scaffold either succeeds or fails with a reason. When it
    /// fails, the schema's report gives that reason. When it succeeds, the
    /// scaffolds satisfy the schema, their markers agree with its gates, and
    /// `sokf new` writes a document at each state that lacks only what its
    /// writer writes.
    #[test]
    fn a_derivation_passes_its_own_schema_or_refuses_by_name(
        shapes in prop::collection::vec(arb_shape(), 1..4)
    ) {
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/gen.md", &schema_of(&shapes));
        let report = tree.validate(false, sokf_core::CacheMode::Off);
        let mine: Vec<&str> = on(&report.errors, "gen.md")
            .map(|f| f.message.as_str())
            .collect();
        // A block with no example is the one thing that stops the
        // derivation, and the report says so. It reports nothing else.
        let refused = shapes.iter().any(|s| !s.shaped && !s.example);
        if refused {
            prop_assert!(
                mine.iter().all(|m| m.starts_with("the schema derives no scaffold:")),
                "{mine:?}"
            );
            return Ok(());
        }
        prop_assert!(mine.is_empty(), "{mine:?}");
        // The skeleton that `sokf new` writes reports only the parts that
        // its writer has still to write.
        for state in ["draft", "done"] {
            let mut o = new_options(tree.dir(), "Gen");
            o.id = Some(format!("gen-{state}"));
            o.states = vec![("state".into(), state.to_string())];
            o.write = true;
            let made = sokf_core::new(&tree.root(), &o);
            prop_assert!(made.is_ok(), "{:?}", made.as_ref().err());
            let after = tree.validate(true, sokf_core::CacheMode::Off);
            if let Ok(made) = made {
                let left = not_missing(&after, made.path.as_str());
                prop_assert!(left.is_empty(), "{left:?}\n{}", common::json(&after));
            }
        }
    }
}

/// `scaffolding` returns whether `line` of a derived body is scaffolding that
/// the schema determines: a blank line, a marker, a fence line, a table's
/// header or rule, or a heading.
fn scaffolding(line: &str) -> bool {
    let t = line.trim();
    t.is_empty()
        || t.starts_with("<!--")
        || t.starts_with("```")
        || t.starts_with('#')
        || t == "| Name | Meaning |"
        || t == "| --- | --- |"
}

proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    /// A derivation invents nothing: in the document that `sokf new` creates,
    /// and in the example of each section that its guide gives, every line
    /// after the frontmatter either is scaffolding that the schema determines
    /// or contains text from one of the schema's `example` values.
    #[test]
    fn a_derivation_invents_nothing(
        shapes in prop::collection::vec(arb_shape(), 1..4)
    ) {
        prop_assume!(!shapes.iter().any(|s| !s.shaped && !s.example));
        let tree = TempTree::fixture("clean");
        tree.write("knowledge/schemas/gen.md", &schema_of(&shapes));
        let stated = ["ONE thing", "ONE | what it means"];
        for state in ["draft", "done"] {
            let mut o = new_options(tree.dir(), "Gen");
            o.id = Some(format!("gen-{state}"));
            o.states = vec![("state".into(), state.to_string())];
            let made = sokf_core::new(&tree.root(), &o).unwrap();
            // The body of the skeleton, and the examples of the sections that
            // the guide gives, which the scaffold writes.
            let skeleton = made.document.splitn(3, "---\n").nth(2).unwrap_or_default();
            let sections: Vec<String> = made
                .guide
                .iter()
                .flat_map(|g| g.sections.iter().filter_map(|s| s.example.clone()))
                .collect();
            let body = format!("{skeleton}\n{}", sections.join("\n\n"));
            for line in body.lines() {
                prop_assert!(
                    scaffolding(line) || stated.iter().any(|s| line.contains(s)),
                    "`{}` in\n{}",
                    line,
                    body
                );
            }
        }
    }
}
