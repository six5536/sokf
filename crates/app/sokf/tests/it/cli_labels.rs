//! These tests run `sokf new`, `sokf validate`, and `sokf guide` on the
//! fixture `guide` with a schema of memos that declares the attribution
//! labels, and check the words of each label in the language of the schema,
//! the label of a scaffold, the repair of a label, the refusals, and the
//! labels of a guide.

use common::Fixture;

use crate::{common, json_schemas::valid};

/// `MEMO` is a schema whose prose requires a label, whose list allows one,
/// with a rule `point` that requires one and a rule `note` whose example
/// carries one, and whose closing prose allows one that its example doesn't
/// carry. `LANGUAGE` stands for the schema's `language` line, `LABELS` for
/// its `labels`, and `HUMAN` for the words of the label `human` in its
/// examples.
const MEMO: &str = "---\ntype: Schema\nid: schema-memo\ntitle: Memo Schema\ndescription: A memo.\n---\n\n````yaml\nline-limit: 0\nLANGUAGElabels: LABELS\nfrontmatter:\n  type:\n    const: Memo\n  id:\n    template: \"memo-${:slug}\"\ntypes:\n  point-id: 'P-[1-9][0-9]*'\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        label: required\n        max: { hard: 1, when-at-limit: 'Write a second memo.' }\n        example: '[HUMAN] The kettle goes back to the shop.'\n      - kind: bullet-list\n        label: optional\n        rules:\n          - template: '${id:point-id} ${}.'\n            key: id\n            rule-id: point\n            label: required\n            example: 'P-1 [HUMAN] The kettle leaks.'\n          - pattern: '^Note '\n            rule-id: note\n            example: '[HUMAN] Note that the shop opens at nine.'\n      - kind: prose\n        optional: true\n        label: optional\n        example: 'The memo ends here.'\n````\n";

/// `GERMAN` gives the labels German words and meanings.
const GERMAN: &str = "de:\n  labels: { human: Mensch, agent: KI }\n  attribution:\n    human: Ein Mensch sagte den ganzen Text.\n    agent: Eine KI schlug den Text vor.\n";

/// `memo` returns the fixture `guide` with the memo schema, its `language`
/// line `language`, such as `"language: de\n"`, or none, and the words
/// `human` of the label `human` in its examples.
fn memo(language: &str, human: &str) -> Fixture {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO
            .replace("LANGUAGE", language)
            .replace("LABELS", "[human, agent]")
            .replace("HUMAN", human),
    );
    fix(&f);
    f
}

/// `fix` runs `sokf validate --fix` on `f` twice, checks that the second run
/// changes no file, and returns what the first run prints.
fn fix(f: &Fixture) -> String {
    let (first, _, _) = f.run(&["validate", "--fix"]);
    let (second, _, _) = f.run(&["validate", "--fix"]);
    assert!(
        !second.lines().any(|l| {
            ["rewrote ", "created ", "moved ", "updated "]
                .iter()
                .any(|op| l.starts_with(op))
        }),
        "the second run fixed more: {second}"
    );
    first
}

/// `scaffold` returns the scaffold of a new memo on `f`, as the guide that
/// `sokf new Memo` prints gives it.
fn scaffold(f: &Fixture) -> String {
    f.scaffold(&["Memo", "--slug", "kettle"])
}

/// `doc` returns a memo whose units carry the labels `first`, for the prose,
/// and `point`, for the item `P-1`.
fn doc(first: &str, point: &str) -> String {
    format!(
        "---\ntype: Memo\nid: memo-kettle\ntitle: Kettle\n---\n\n# Kettle\n\n[{first}] The kettle goes back to the shop.\n\n- <a id=\"p-1\"></a>P-1 [{point}] The kettle leaks.\n- Note that the shop opens at nine.\n"
    )
}

/// `errors` returns the lines of `sokf validate` on `f` that report an error
/// in the memo.
fn errors(f: &Fixture) -> Vec<String> {
    let (out, _, _) = f.run(&["validate"]);
    out.lines()
        .filter(|l| l.starts_with("/knowledge/memo-kettle.md") && l.contains("error:"))
        .map(str::to_string)
        .collect()
}

// A scaffold, which the guide of a new document gives as its examples,
// labels `[agent]` each unit whose rule requires a label, after
// the key of a unit that begins with one, and each unit whose rule allows a
// label and whose example carries one, and leaves unlabelled a unit whose
// example carries none. The new document holds no unit with a label to
// check.
// sokf-tests: CLI-6_AC-124
// sokf-tests: CLI-6_AC-125
// sokf-tests: SOKF_SCHEMA_108
#[test]
fn a_scaffold_labels_its_units_agent() {
    let f = memo("", "human");
    let out = scaffold(&f);
    for line in [
        "\n[agent] The kettle goes back to the shop.\n",
        "\n- P-1 [agent] The kettle leaks.\n",
        "\n- [agent] Note that the shop opens at nine.\n",
        "\nThe memo ends here.\n",
    ] {
        assert!(out.contains(line), "{line:?} is not in {out}");
    }
    let (out, err, code) = f.run(&["new", "Memo", "--slug", "kettle", "--write"]);
    assert_eq!(code, 0, "{out}{err}");
    let labels: Vec<String> = errors(&f)
        .into_iter()
        .filter(|l| {
            ["TLMD_249", "TLMD_250", "TLMD_251"]
                .iter()
                .any(|k| l.contains(k))
        })
        .collect();
    assert_eq!(labels, Vec::<String>::new());
}

// In a schema in German, the scaffold writes the German words of
// `/.sokf/texts.yaml`, the check accepts them and refuses a label written as
// its key, and `--fix` rewrites that label in the German words.
// sokf-tests: SOKF_SCHEMA_105
// sokf-tests: SOKF_SCHEMA_107
// sokf-tests: CLI-4_AC-108
// sokf-tests: CLI-6_AC-124
// sokf-tests: CONF-12_AC-1
// sokf-tests: CONF-12_AC-7
#[test]
fn a_label_takes_the_words_of_the_schemas_language() {
    let f = Fixture::new("guide");
    f.write(".sokf/texts.yaml", GERMAN);
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO
            .replace("LANGUAGE", "language: de\n")
            .replace("LABELS", "[human, agent]")
            .replace("HUMAN", "Mensch"),
    );
    let out = scaffold(&f);
    assert!(
        out.contains("\n[KI] The kettle goes back to the shop.\n"),
        "{out}"
    );
    f.write("knowledge/memo-kettle.md", &doc("Mensch", "KI"));
    fix(&f);
    assert_eq!(errors(&f), Vec::<String>::new());
    f.write("knowledge/memo-kettle.md", &doc("human", "agent"));
    let found = errors(&f);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().all(|l| l.contains("TLMD_251")), "{found:?}");
    let out = fix(&f);
    assert!(out.contains("rewrote /knowledge/memo-kettle.md"), "{out}");
    let text = std::fs::read_to_string(f.path("knowledge/memo-kettle.md")).unwrap();
    assert_eq!(text, doc("Mensch", "KI"));
    assert_eq!(errors(&f), Vec::<String>::new());
}

// A language without words for the labels takes their keys, in the check and
// in the scaffold.
// sokf-tests: SOKF_SCHEMA_106
#[test]
fn a_language_without_words_takes_the_keys() {
    let f = memo("language: fr\n", "human");
    let out = scaffold(&f);
    assert!(
        out.contains("\n[agent] The kettle goes back to the shop.\n"),
        "{out}"
    );
    f.write("knowledge/memo-kettle.md", &doc("human", "agent"));
    fix(&f);
    assert_eq!(errors(&f), Vec::<String>::new());
}

// A schema whose `labels` holds a key other than `human` and `agent`, or
// lacks one of them, breaks.
// sokf-tests: SOKF_SCHEMA_104
#[test]
fn a_label_other_than_human_and_agent_is_refused() {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO
            .replace("LANGUAGE", "")
            .replace("LABELS", "[human, robot]")
            .replace("HUMAN", "human"),
    );
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("/knowledge/schemas/memo.md:10: error: `labels` holds `robot`, which is no label of sokf's; sokf's labels are `human` and `agent` (SOKF_SCHEMA_104)"),
        "{out}"
    );
    // A schema without `agent` would label what an agent writes `[human]`.
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO
            .replace("LANGUAGE", "")
            .replace("LABELS", "[human]")
            .replace("HUMAN", "human"),
    );
    let (out, _, _) = f.run(&["validate"]);
    assert!(
        out.contains("/knowledge/schemas/memo.md:10: error: `labels` holds no `agent`; a schema declares both of sokf's labels, `human` and `agent` (SOKF_SCHEMA_104)"),
        "{out}"
    );
}

// The guide states the words and the meaning of each label once, marks each
// block whose units carry a label, and gives both in its JSON. In a
// language whose texts hold words and no meaning, it prints the words alone.
// sokf-tests: CLI-17_AC-35
// sokf-tests: CLI-17_AC-36
// sokf-tests: CLI-17_AC-37
#[test]
fn the_guide_states_each_label_once() {
    let f = memo("", "human");
    f.write("knowledge/memo-kettle.md", &doc("human", "agent"));
    let (text, err, code) = f.run(&["guide", "memo-kettle"]);
    assert_eq!(code, 0, "{err}");
    for line in [
        "\n\nLabels:\n  [human]: A person said all of the text that the label covers, or confirmed all of it in so many words.\n  [agent]: An agent proposed the text that the label covers, or some of it.\n\n",
        "\n  prose (label required): At most 1 paragraph.",
        "\n  bullet-list (label optional; required for `point`)\n",
        "\n  prose (optional; label optional)\n",
    ] {
        assert!(text.contains(line), "{line:?} is not in {text}");
    }
    assert_eq!(text.matches("Labels:").count(), 1, "{text}");
    let (json, err, code) = f.run(&["guide", "memo-kettle", "--json"]);
    assert_eq!(code, 0, "{err}");
    let value = valid("guide-report.schema.json", &json);
    let doc = &value["documents"][0];
    assert_eq!(doc["labels"][1]["key"], "agent", "{doc}");
    assert_eq!(doc["labels"][1]["words"], "agent", "{doc}");
    let list = &doc["sections"][0]["blocks"][1]["labels"];
    assert_eq!(list[0].get("rule"), None, "{list}");
    assert_eq!(list[0]["label"], "optional", "{list}");
    assert_eq!(list[1]["rule_id"], "point", "{list}");
    assert_eq!(list[1]["label"], "required", "{list}");

    f.write(
        ".sokf/texts.yaml",
        "de:\n  labels: { human: Mensch, agent: KI }\n",
    );
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO
            .replace("LANGUAGE", "language: de\n")
            .replace("LABELS", "[human, agent]")
            .replace("HUMAN", "Mensch"),
    );
    let (text, err, code) = f.run(&["guide", "memo-kettle"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("\n\nLabels:\n  [Mensch]\n  [KI]\n\n"),
        "{text}"
    );
}

// The words of a label that no label could hold, a label that sokf doesn't
// know, and two labels with the same words stop the check with an error
// that names the file.
// sokf-tests: CONF-12_AC-4
// sokf-tests: CONF-12_AC-9
// sokf-tests: CONF-12_AC-10
#[test]
fn a_label_that_no_document_could_write_stops_the_check() {
    let f = memo("", "human");
    for (texts, says) in [
        (
            "de:\n  labels: { human: '[Mensch' }\n",
            "the words of `labels.human` hold a square bracket",
        ),
        (
            "de:\n  labels: { agent: '^KI' }\n",
            "the words of `labels.agent` begin with `^`",
        ),
        (
            "de:\n  labels: { human: ' ' }\n",
            "the words of `labels.human` are blank",
        ),
        (
            "de:\n  attribution: { robot: Ein Roboter. }\n",
            "`attribution.robot` is no label of sokf's",
        ),
        (
            "de:\n  labels: { human: agent }\n",
            "in `de`, the labels `human` and `agent` both take the words `agent`",
        ),
    ] {
        f.write(".sokf/texts.yaml", texts);
        let (out, err, code) = f.run(&["validate"]);
        assert_eq!(code, 2, "{texts:?}: {out}{err}");
        assert!(
            err.contains("/.sokf/texts.yaml") && err.contains(says),
            "{texts:?}: {err}"
        );
    }
}

// A cached run checks the labels again when the words of a label change in
// `/.sokf/texts.yaml`, and when the store's `primary-language` changes.
// sokf-tests: CACHE-1_AC-99
// sokf-tests: CACHE-1_AC-100
#[test]
fn a_change_of_the_words_reaches_a_cached_run() {
    let f = memo("", "human");
    f.write("knowledge/memo-kettle.md", &doc("Mensch", "KI"));
    f.write(".sokf/texts.yaml", GERMAN);
    let manifest =
        "sokf: \"0.7\"\nname: guide-fixture\nscope: test\ndescription: A store for sokf guide.\n";
    f.write(
        "knowledge/manifest.sokf.yaml",
        &format!("{manifest}primary-language: de\n"),
    );
    fix(&f);
    assert_eq!(errors(&f), Vec::<String>::new());
    f.write(
        ".sokf/texts.yaml",
        &GERMAN.replace("human: Mensch", "human: Person"),
    );
    let found = errors(&f);
    assert!(
        found.len() == 1 && found[0].contains("`[Mensch]`"),
        "{found:?}"
    );
    f.write(".sokf/texts.yaml", GERMAN);
    assert_eq!(errors(&f), Vec::<String>::new());
    f.write("knowledge/manifest.sokf.yaml", manifest);
    let found = errors(&f);
    assert_eq!(found.len(), 2, "{found:?}");
}
