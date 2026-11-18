//! These tests check the format registry: its shapes, variants, bans, layers,
//! and patches.
use std::fmt::Write as _;

use proptest::prelude::*;
use regex::Regex;

use super::requirements;
use crate::{
    Finding, RepoPath,
    grammar::{
        Captured, FormatMatch, FormatRegistry, FormatSource, RegexCache, embedded_formats,
        load_grammar, refused,
    },
    yaml,
};

fn key(k: &str, start: usize) -> FormatMatch {
    FormatMatch::Ok(Some(Captured {
        key: k.into(),
        start,
    }))
}

#[test]
fn the_requirement_formats_check_stories_and_ears() {
    let r = requirements();
    let story = r.get("user-story").unwrap();
    assert_eq!(
        story.check("CLI-1: AS A developer, I WANT a thing, SO THAT I win."),
        key("CLI-1", 0)
    );
    assert!(matches!(story.check("CLI-1: I want"), FormatMatch::Fail(_)));
    assert!(matches!(
        story.check("CLI-1: AS A dev, I WANT x, SO THAT it SHALL work."),
        FormatMatch::Refused(m) if m.contains("`SHALL` is refused:") && m.contains(story.first_example())
    ));
    let ears = r.get("requirement-ears").unwrap();
    assert_eq!(
        ears.check("CLI-2_AC-1 [ubiquitous]: the system SHALL log every command invocation."),
        key("CLI-2_AC-1", 0)
    );
    // Every variant fails at the column after the `[`, so a tag that no
    // variant carries fails there.
    assert!(matches!(
        ears.check("CLI-2_AC-1 [bogus]: the system SHALL log"),
        FormatMatch::Fail(m) if m.contains("matches none of the forms") && m.contains("`event`") && m.contains("`complex-event`")
    ));
    // The check reports a failing line against the variant that matches the
    // most of it.
    let event = &ears.variants.iter().find(|(n, _)| n == "event").unwrap().1;
    assert!(matches!(
        ears.check("CLI-2_AC-1 [event]: the system SHALL log"),
        FormatMatch::Fail(m) if m.contains("the `event` form") && m.contains("column 21") && m.contains(&event.example)
    ));
    assert!(matches!(
        ears.check("CLI-2_AC-1 [ubiquitous]: the system SHALL log and SHALL sing."),
        FormatMatch::Refused(m) if m.contains("`SHALL log and SHALL` is refused:")
    ));
    assert_eq!(r.entries["requirement-ears"].1, FormatSource::Embedded);
}

/// The tool embeds no format, so a project takes its formats from its
/// schemas and from the layers of its repository.
#[test]
fn the_embedded_formats_hold_no_format() {
    let r = embedded_formats(&load_grammar().unwrap(), &RegexCache::new()).unwrap();
    assert!(r.entries.is_empty());
}

/// The requirement formats refuse the weak, vague, passive, and compound
/// sentences they exist to keep out, and a form that does not match its tag.
#[test]
fn the_requirement_formats_refuse_what_they_ban() {
    let r = requirements();
    // A line fails when it misses its shape (`Fail`) or matches a ban
    // (`Refused`).
    let fails = |name: &str, line: &str| {
        matches!(
            r.entries[name].0.check(line),
            FormatMatch::Fail(_) | FormatMatch::Refused(_)
        )
    };
    let ok = |name: &str, line: &str| matches!(r.entries[name].0.check(line), FormatMatch::Ok(_));
    let ears = "requirement-ears";
    assert!(ok(
        ears,
        "A-1_AC-1 [event]: WHEN a user posts, the API SHALL return 201."
    ));
    assert!(ok(
        ears,
        "A-1_AC-1 [ubiquitous]: the API SHALL NOT write a file."
    ));
    for bad in [
        // The event form takes no THEN; the unwanted form needs it.
        "A-1_AC-1 [event]: WHEN a user posts THEN the API SHALL return 201.",
        "A-1_AC-1 [unwanted]: IF the code is not known, the API SHALL return 404.",
        // The tag does not match the form.
        "A-1_AC-1 [state]: WHEN a user posts, the API SHALL return 201.",
        // These lines fail for an upper-case THE, a missing period, a weak
        // modal, a vague word, the passive voice, and a compound sentence.
        "A-1_AC-1 [ubiquitous]: THE API SHALL return 201.",
        "A-1_AC-1 [ubiquitous]: the API SHALL return 201",
        "A-1_AC-1 [ubiquitous]: the API SHALL return 201 if it can.",
        "A-1_AC-1 [ubiquitous]: the API SHALL respond quickly.",
        "A-1_AC-1 [ubiquitous]: the file SHALL be written.",
        "A-1_AC-1 [ubiquitous]: the API SHALL log and SHALL count.",
    ] {
        assert!(fails(ears, bad), "{bad}");
    }
    let prop = "requirement-property";
    assert!(ok(
        prop,
        "A_P-1 [round-trip]: FOR ANY URL, the API SHALL return it."
    ));
    assert!(fails(
        prop,
        "A_P-1 [shape]: FOR ANY URL, the API SHALL return it."
    ));
    assert!(fails(prop, "A_P-1 [invariant]: the API SHALL return it."));
    let evo = "requirement-evolution";
    assert!(ok(evo, "Deprecation: An old version works for 12 months."));
    assert!(fails(
        evo,
        "Deprecation: An old version SHALL work for 12 months."
    ));
    assert!(fails(
        evo,
        "Deprecation: An old version may be removed eventually."
    ));
    assert!(fails(
        evo,
        "Deprecation: An old version is removed after a year."
    ));
    assert!(fails(evo, "Sunset: An old version works for 12 months."));
    let sum = "requirement-summary";
    assert!(ok(
        sum,
        "`GET /{code}` sends a visitor to the address in `/v1.2`."
    ));
    assert!(ok(sum, "The settings screen lets an owner change a link."));
    assert!(ok(sum, "A visit to `GET /{code}` sends the visitor on."));
    assert!(fails(sum, "- sends a visitor to the address."));
    assert!(fails(sum, "`GET /{code}` sends a visitor"));
    assert!(fails(
        sum,
        "`GET /{code}` sends a visitor. It logs the visit."
    ));
    assert!(fails(
        sum,
        "`GET /{code}` sends a visitor, and the API SHALL log it."
    ));
    let chan = "requirement-channel";
    assert!(ok(chan, "None."));
    assert!(ok(chan, "Exit code: 0, 1 or 2."));
    assert!(ok(
        chan,
        "`stdout`: the report, as text or as the JSON below."
    ));
    assert!(fails(chan, "The report on stdout."));
    assert!(fails(chan, "stdout: the report"));
    assert!(fails(chan, "stdout: the API SHALL print the report."));
    assert!(fails(chan, "stdout: the report, which may be JSON."));
    assert!(fails(chan, "None"));
}

/// Every example in the requirement formats conforms to the entry that
/// carries it. The reason is that sokf shows the example to the author of a
/// failing line, and a scaffold writes it into a new document.
#[test]
fn every_example_of_the_requirement_formats_conforms_to_its_own_entry() {
    let r = requirements();
    for (name, (format, _)) in &r.entries {
        for line in format.examples() {
            assert!(
                matches!(format.check(line), FormatMatch::Ok(_)),
                "{name}: {:?}",
                format.check(line)
            );
        }
    }
}

#[test]
fn to_yaml_prints_sources_and_round_trips() {
    let r = requirements();
    let text = r.to_yaml(true);
    assert!(text.starts_with("types:\n"), "{text}");
    assert!(text.contains("source: built-in"), "{text}");
    assert!(text.contains("source: embedded"), "{text}");
    let parsed = yaml::parse(&text, 1).unwrap();
    let registry = parsed.get("registry").unwrap();
    assert_eq!(registry.as_map().unwrap().len(), 6);
    let ears = registry.get("requirement-ears").unwrap();
    assert_eq!(ears.get("source").unwrap().as_str(), Some("embedded"));
    assert!(ears.get("variants").unwrap().get("event").is_some());
}

fn registry_from(
    text: &str,
    source: FormatSource,
) -> std::result::Result<FormatRegistry, Vec<Finding>> {
    let grammar = load_grammar().unwrap();
    let regexes = RegexCache::new();
    let root = yaml::parse(text, 1).unwrap();
    FormatRegistry::from_yaml(root.get("registry").unwrap(), source, &grammar, &regexes)
}

#[test]
fn from_yaml_validates_against_the_grammar() {
    let ok = registry_from(
        "registry:\n  simple:\n    pattern: '^x$'\n    example: x\n",
        FormatSource::Repo,
    )
    .unwrap();
    assert_eq!(ok.get("simple").unwrap().check("x"), FormatMatch::Ok(None));
    assert_eq!(ok.entries["simple"].1, FormatSource::Repo);
    let bad = registry_from(
        "registry:\n  simple:\n    pattern: '('\n    example: x\n  other:\n    nope: 1\n",
        FormatSource::Schema("s".into()),
    )
    .unwrap_err();
    assert!(
        bad.iter().any(|f| f.message.contains("format `simple`")),
        "{bad:?}"
    );
    assert!(
        bad.iter().any(|f| f.message.contains("format `other`")),
        "{bad:?}"
    );
    let bad = registry_from("registry: [1]\n", FormatSource::Repo).unwrap_err();
    assert_eq!(bad.len(), 1);
    // A format has either one shape with its example, or variants that each
    // have their own.
    for (bad, key, says) in [
        (
            "registry:\n  v:\n    example: x\n    variants:\n      a:\n        pattern: '^a$'\n        example: a\n",
            "TL_110",
            "no `example` of its own",
        ),
        (
            "registry:\n  v:\n    pattern: '^a$'\n",
            "TL_40",
            "a format with one shape carries an `example`",
        ),
        (
            "registry:\n  v:\n    key: id\n    template: 'a ${n}'\n    example: a b\n",
            "TL_31",
            "`key` `id` names no hole or named group of the shape",
        ),
        (
            "registry:\n  v:\n    template: 'a ${n:nope}'\n    example: a b\n",
            "TL_120",
            "no type `nope`",
        ),
        (
            "registry:\n  v:\n    text: a\n    pattern: b\n    example: a\n",
            "TL_22",
            "exactly one of",
        ),
    ] {
        let found = registry_from(bad, FormatSource::Repo).unwrap_err();
        assert!(
            found
                .iter()
                .any(|f| f.criterion == key && f.message.contains(says)),
            "{bad}: {found:?}"
        );
    }
}

/// The check applies a ban after the shape, and the failure shows the
/// example of the variant the line matched, not the example of another
/// variant.
#[test]
fn a_ban_shows_the_example_of_the_variant_the_line_took() {
    let r = registry_from(
        "registry:\n  v:\n    prohibited-patterns: [{pattern: '\\bnever\\b', message: The line contains no never.}]\n    variants:\n      a:\n        template: 'a ${}'\n        example: a one\n      b:\n        template: 'b ${}'\n        example: b two\n",
        FormatSource::Repo,
    )
    .unwrap();
    assert_eq!(
        r.get("v").unwrap().check("b never"),
        FormatMatch::Refused(
            "`never` is refused: The line contains no never. A line of this kind reads b two"
                .into()
        )
    );
}

/// A layer's types apply to every format in the layers farther from it,
/// and the name of a built-in type is reserved.
#[test]
fn a_type_means_one_thing_to_every_format() {
    let grammar = load_grammar().unwrap();
    let regexes = RegexCache::new();
    let base = registry_from(
        "registry:\n  f:\n    key: id\n    template: '${id:code}: ${}'\n    example: 'AB: x'\n",
        FormatSource::Embedded,
    );
    // A type the layer does not declare is unknown to its templates.
    assert!(base.is_err());
    let root = yaml::parse(
        "types:\n  code: '[A-Z]+'\nregistry:\n  f:\n    key: id\n    template: '${id:code}: ${}'\n    example: 'AB: x'\n",
        1,
    )
    .unwrap();
    let base = FormatRegistry::from_yaml_at(
        root.get("registry").unwrap(),
        root.get("types"),
        FormatSource::Embedded,
        &grammar,
        &regexes,
        &RepoPath::root(),
        None,
    )
    .unwrap();
    assert!(matches!(
        base.get("f").unwrap().check("AB9: x"),
        FormatMatch::Fail(_)
    ));
    let over = yaml::parse("types:\n  code: '[A-Z0-9]+'\n", 1).unwrap();
    let repo = FormatRegistry::from_yaml_at(
        &yaml::parse("{}", 1).unwrap(),
        over.get("types"),
        FormatSource::Repo,
        &grammar,
        &regexes,
        &RepoPath::root(),
        Some(&base),
    )
    .unwrap();
    let (seen, broken) = base.layered(&repo);
    assert!(broken.is_empty(), "{broken:?}");
    assert_eq!(seen.get("f").unwrap().check("AB9: x"), key("AB9", 0));
    assert_eq!(seen.types_seen().entries["code"].1, FormatSource::Repo);
    let reserved = yaml::parse("types:\n  slug: x\n", 1).unwrap();
    let e = FormatRegistry::from_yaml_at(
        &yaml::parse("{}", 1).unwrap(),
        reserved.get("types"),
        FormatSource::Repo,
        &grammar,
        &regexes,
        &RepoPath::root(),
        None,
    )
    .unwrap_err();
    assert!(e[0].message.contains("reserved"), "{e:?}");
}

fn arb_registry(source: FormatSource) -> impl Strategy<Value = FormatRegistry> {
    prop::collection::btree_set(prop::sample::select(vec!["a", "b", "c", "d"]), 0..4).prop_map(
        move |names| {
            let grammar = load_grammar().unwrap();
            let regexes = RegexCache::new();
            let text = names.iter().fold(String::new(), |mut out, n| {
                let _ = write!(
                    out,
                    "  {n}:\n    pattern: '^{n}-{0}$'\n    example: {n}-{0}\n",
                    source.label()
                );
                out
            });
            let text = if text.is_empty() {
                " {}".to_string()
            } else {
                format!("\n{text}")
            };
            let root = yaml::parse(&format!("registry:{text}"), 1).unwrap();
            let registry = root.get("registry").cloned().unwrap();
            FormatRegistry::from_yaml(&registry, source.clone(), &grammar, &regexes).unwrap()
        },
    )
}

proptest! {
    #![proptest_config(crate::proptest_cases::cases(256))]

    #[test]
    fn a_name_resolves_to_the_nearest_layer(
        embedded in arb_registry(FormatSource::Embedded),
        repo in arb_registry(FormatSource::Repo),
        schema in arb_registry(FormatSource::Schema("s".into())),
    ) {
        let effective = embedded.layered(&repo).0.layered(&schema).0;
        for name in ["a", "b", "c", "d"] {
            let expected = if schema.get(name).is_some() {
                Some(FormatSource::Schema("s".into()))
            } else if repo.get(name).is_some() {
                Some(FormatSource::Repo)
            } else if embedded.get(name).is_some() {
                Some(FormatSource::Embedded)
            } else {
                None
            };
            let got = effective.entries.get(name).map(|(f, s)| {
                prop_assert!(f.first_example().ends_with(&*s.label()));
                Ok(s.clone())
            }).transpose()?;
            prop_assert_eq!(got, expected);
        }
    }
}

/// This helper builds a registry of one variant format and one plain
/// format, and puts `word` in the example of variant `a` and in the example
/// of the plain format.
fn exampled(word: &str) -> FormatRegistry {
    let text = format!(
        "registry:\n  v:\n    variants:\n      a:\n        template: '[a] ${{:text}}'\n        example: \"[a] {word}\"\n      b:\n        template: '[b] ${{:int}}'\n        example: \"[b] 1\"\n  p:\n    pattern: '^{word}$'\n    example: {word}\n",
    );
    registry_from(&text, FormatSource::Repo).unwrap()
}

proptest! {
    #![proptest_config(crate::proptest_cases::cases(256))]

    /// A shape failure gives the place where the shape fails and that
    /// shape's example. It gives nothing else from the schema's text.
    #[test]
    fn a_shape_failure_shows_the_example(word in "[a-z]{3,8}") {
        let r = exampled(&word);
        let FormatMatch::Fail(said) = r.get("v").unwrap().check("[b] NOPE") else {
            return Err(TestCaseError::fail("the variant shape accepted a bad line"));
        };
        prop_assert_eq!(
            &said,
            "does not match the `b` form: it fails at column 5: expected an `int` (`\\d+`), found `NOPE`; a line of this kind reads [b] 1"
        );
        let FormatMatch::Fail(said) = r.get("p").unwrap().check("NOPE") else {
            return Err(TestCaseError::fail("the plain shape accepted a bad line"));
        };
        prop_assert_eq!(
            &said,
            &format!("does not match `^{word}$`; a line of this kind reads {word}")
        );
    }
}

proptest! {
    #![proptest_config(crate::proptest_cases::cases(256))]

    /// A ban quotes what it matched: the span is a match of the ban's
    /// pattern, taken from the line, and a ban on a rule shows the
    /// rule's example beside it.
    #[test]
    fn a_ban_quotes_what_it_matched(
        line in "[a-zA-Z ]{0,24}",
        ban in prop::sample::select(vec![r"[A-Z]{2,}", r"\bSHALL\b", r"a+", r"x"]),
        example in prop::option::of("[a-z]{1,6}"),
    ) {
        let pattern = Regex::new(ban).unwrap();
        let rule = tramlines_core::grammar::Prohibition {
            pattern: pattern.clone(),
            message: "The line holds no such text.".to_string(),
        };
        let said = refused(&line, std::slice::from_ref(&rule), example.as_deref());
        match pattern.find(&line) {
            None => prop_assert!(said.is_none()),
            Some(m) => {
                let said = said.unwrap();
                let quoted = format!("`{}` is refused: The line holds no such text.", m.as_str());
                prop_assert!(said.starts_with(&quoted), "{said}");
                prop_assert!(line.contains(m.as_str()) && pattern.is_match(m.as_str()));
                if let Some(e) = &example {
                    let tail = format!(" A line of this kind reads {e}");
                    prop_assert!(said.ends_with(&tail), "{}", said);
                } else { prop_assert_eq!(&said, &quoted) }
            }
        }
    }
}
