use super::*;

#[test]
fn a_tag_is_looked_up_without_case_and_then_without_its_last_subtags() {
    assert_eq!(lookup("de-CH-1996"), ["de-ch-1996", "de-ch", "de"]);
    assert_eq!(lookup("EN"), ["en"]);
    // A single-letter subtag only introduces an extension.
    assert_eq!(lookup("en-a-bbb"), ["en-a-bbb", "en"]);
    // The whole tag is tried first, whatever its last subtag.
    assert_eq!(lookup("de-x-a"), ["de-x-a", "de"]);
}

// sokf-tests: CONF-12_AC-6
#[test]
fn a_tag_that_ends_in_a_single_letter_takes_its_own_entry() {
    let t =
        Texts::default().with("de-x-a: { description: Eigen. }\nde: { description: Allgemein. }\n");
    assert_eq!(t.words("de-X-A", "description"), Some("Eigen."));
    assert_eq!(t.words("de-x-b", "description"), Some("Allgemein."));
}

// A unit's form other than `one` and `other`, such as `few`, is
// ignored, as the tramlines command ignores it.
#[test]
fn a_further_form_of_a_unit_is_ignored() {
    let path = RepoPath::parse("/.sokf/texts.yaml").unwrap();
    let parsed = parse(
        &path,
        "pl: { units: { item: { one: punkt, few: punkty, other: punktów } } }\n",
    )
    .unwrap();
    assert_eq!(
        parsed
            .iter()
            .map(|e| (&*e.entry, &e.text))
            .collect::<Vec<_>>(),
        [(
            "units.item",
            &Text::Forms {
                one: "punkt".into(),
                other: "punktów".into()
            }
        )]
    );
}

#[test]
fn a_template_is_filled_in_one_pass() {
    assert_eq!(
        fill_template(
            "At most {count} {unit}.",
            &[("count", "3"), ("unit", "{count}")]
        ),
        "At most 3 {count}."
    );
    assert_eq!(
        fill_template("{x} {unit} {", &[("unit", "items")]),
        "{x} items {"
    );
    assert_eq!(
        placeholders("a {count} {not here} {} {unit}{"),
        ["count", "unit"]
    );
}

#[test]
fn sokf_english_holds_every_entry() {
    let t = Texts::english();
    for entry in [
        "description",
        "limit.hard",
        "limit.soft",
        "limit.ban",
        "rule-unit",
    ] {
        assert!(t.words("en", entry).is_some(), "{entry}");
    }
    for unit in UNITS {
        assert!(t.forms("en-GB", unit).is_some(), "{unit}");
    }
    assert!(t.words("de", "description").is_none());
    // The English words of each label are its key (D-10 of issue 265).
    for label in LABELS {
        assert_eq!(t.label("en", label).as_deref(), Some(label));
        assert!(
            t.words("en", &format!("attribution.{label}")).is_some(),
            "{label}"
        );
    }
    assert_eq!(t.label("de", "human"), None);
}

// The words of a label come in the shape of the `tramlines` command's
// `labels`, with each run of whitespace made one space, and two labels
// with the same words in one language, whose own or else whose key,
// are refused.
// sokf-tests: CONF-12_AC-9
// sokf-tests: CONF-12_AC-10
#[test]
fn the_labels_take_words_that_a_label_can_hold() {
    let t = Texts::english().with("de: { labels: { human: ' Ein  Mensch ', agent: KI } }\n");
    assert_eq!(t.label("de-AT", "human").as_deref(), Some("Ein Mensch"));
    assert_eq!(t.label("de", "agent").as_deref(), Some("KI"));
    assert!(t.check_labels(&BTreeMap::new(), &BTreeMap::new()).is_ok());
    let path = RepoPath::parse("/.sokf/texts.yaml").unwrap();
    for (text, says) in [
        ("de: { labels: { human: '' } }\n", "are blank"),
        ("de: { labels: { human: 'a]' } }\n", "hold a square bracket"),
        ("de: { labels: { agent: '^1' } }\n", "begin with `^`"),
        (
            "de: { labels: { robot: x } }\n",
            "`labels.robot` is no label",
        ),
        ("de: { attribution: { human: [x] } }\n", "no string"),
    ] {
        let e = parse(&path, text).unwrap_err().to_string();
        assert!(e.contains(says), "{text:?} -> {e}");
    }
    let mut sources = BTreeMap::new();
    sources.insert(
        ("de-ch".to_string(), "labels.human".to_string()),
        path.clone(),
    );
    let clash = Texts::english()
        .with("de-CH: { labels: { human: KI } }\nde: { labels: { agent: KI, human: M } }\n");
    let mut written = BTreeMap::new();
    written.insert("de-ch".to_string(), "de-CH".to_string());
    let e = clash
        .check_labels(&sources, &written)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("in `de-CH`, the labels `human` and `agent` both take the words `KI`"),
        "{e}"
    );
    // A label without words of its own takes its key, and the words of
    // one label can't be the key of the other.
    for (text, says) in [
        (
            "fr: { labels: { human: agent } }\n",
            "both take the words `agent`",
        ),
        (
            "de: { labels: { human: agent, agent: KI } }\n",
            "in `de`, the label `human` takes the words `agent`, the key of `agent`, so a \
             document could not tell the two labels apart",
        ),
        (
            "de: { labels: { human: Mensch, agent: human } }\n",
            "in `de`, the label `agent` takes the words `human`, the key of `human`, so a \
             document could not tell the two labels apart",
        ),
    ] {
        let e = Texts::english()
            .with(text)
            .check_labels(&BTreeMap::new(), &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(e.contains(says), "{text:?} -> {e}");
    }
}

// The English rule for a description is the format specification's
// definition of `description`, word for word (D-17 of issue 260).
#[test]
fn the_english_rule_for_a_description_is_the_definition_of_the_format() {
    let spec = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../sokf-spec/sokf-format-spec.md"),
    )
    .unwrap();
    let definition = spec
        .lines()
        .find_map(|l| l.strip_prefix("- `description` - The text that the index"))
        .map(|rest| format!("The text that the index{rest}"))
        .unwrap();
    assert_eq!(
        Texts::english().words("en", "description"),
        Some(definition.as_str())
    );
}

#[test]
fn a_file_of_another_shape_is_refused() {
    let path = RepoPath::parse("/.sokf/texts.yaml").unwrap();
    for (text, says) in [
        ("- de\n", "no map from language tags"),
        ("d_e: {}\n", "no language tag"),
        ("de: { title: x }\n", "`title` is no text of sokf's"),
        ("de: { limit: { hard: 'Höchstens {cnt}.' } }\n", "`{cnt}`"),
        (
            "de: { description: 'Der {unit}' }\n",
            "takes no placeholder",
        ),
        (
            "de: { units: { item: { one: Punkt } } }\n",
            "no map of the strings",
        ),
        (
            "de: { units: { word: { one: a, other: b } } }\n",
            "no unit of sokf's",
        ),
        ("de: { description: [x] }\n", "no string"),
        (
            "de: { limit: { hard: 'Höchstens {unit}.' } }\n",
            "holds no `{count}`",
        ),
        ("de: { rule-unit: '{unit}' }\n", "holds no `{name}`"),
        ("de: {}\nDE: {}\n", "gives the language of line 1 again"),
        ("de: [\n", ""),
    ] {
        let e = parse(&path, text).unwrap_err().to_string();
        assert!(
            e.starts_with("/.sokf/texts.yaml: ") && e.contains(says),
            "{text:?} -> {e}"
        );
    }
    assert!(parse(&path, "").unwrap().is_empty());
    assert!(parse(&path, "de: { limit: { ban: 'Keine {unit}.' } }\n").is_ok());
}

// The error on a label whose words are the other label's key names the file
// that gave those words, not the file that gave the other label's.
// sokf-tests: CONF-12_AC-10
#[test]
fn a_clash_names_the_file_of_the_faulty_entry() {
    let own = RepoPath::parse("/.sokf/texts.yaml").unwrap();
    let pack = RepoPath::parse("/.sokf/texts.d/pack.yaml").unwrap();
    let mut sources = BTreeMap::new();
    sources.insert(("de".to_string(), "labels.human".to_string()), own);
    sources.insert(("de".to_string(), "labels.agent".to_string()), pack);
    let e = Texts::english()
        .with("de: { labels: { human: agent, agent: KI } }\n")
        .check_labels(&sources, &BTreeMap::new())
        .unwrap_err()
        .to_string();
    assert!(e.starts_with("/.sokf/texts.yaml: "), "{e}");
}
