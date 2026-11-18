//! These tests run `sokf new`, `sokf guide`, and `sokf validate` on the
//! fixture `guide` with a schema of memos, and check sokf's own texts in the
//! language of the schema: the rule for a description, the words for each
//! limit, the files of texts that give them, and `primary-language`.

use common::Fixture;

use crate::{common, json_schemas::valid};

/// `MEMO` is a schema with guidance for `description`, a limit on a
/// frontmatter list, a block's own limit, and the limits of two unit rules,
/// one of which bans its units. `LANGUAGE` stands for the schema's
/// `language` line.
const MEMO: &str = "---\ntype: Schema\nid: schema-memo\ntitle: Memo Schema\ndescription: A memo.\n---\n\n````yaml\nline-limit: 0\nLANGUAGEfrontmatter:\n  type:\n    const: Memo\n  id:\n    template: \"memo-${:slug}\"\n  description:\n    guidance: The description names the decision.\n    example: The kettle goes back to the shop.\n  tags:\n    list: true\n    optional: true\n    max: { soft: 2, when-at-limit: 'Keep the tags that a reader searches for.' }\n    example: kitchen\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: bullet-list\n        max: { hard: 3, when-at-limit: 'Write a second memo.' }\n        rules:\n          - rule-id: todo\n            pattern: '^TODO'\n            min: 0\n            max: { hard: 0, when-at-limit: 'File an issue.' }\n            example: TODO ask\n          - rule-id: point\n            pattern: '^[A-Z]'\n            max: { hard: 1, when-at-limit: 'Join the points.' }\n            example: The kettle leaks.\n````\n";

/// `RULE` is the start of sokf's English rule for a description.
const RULE: &str = "The text that the index shows beside the title, for a reader deciding whether to open the document.";

/// `memo` returns the fixture `guide` with the memo schema, its `language`
/// line `language`, such as `"language: en\n"`, or none.
fn memo(language: &str) -> Fixture {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO.replace("LANGUAGE", language),
    );
    f
}

/// `scaffold` returns what `sokf new Memo` prints on `f`.
fn scaffold(f: &Fixture) -> String {
    let (out, err, code) = f.run(&["new", "Memo", "--slug", "kettle"]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

/// `with_manifest` adds `line` to the manifest of the fixture's store.
fn with_manifest(f: &Fixture, line: &str) {
    f.write(
        "knowledge/manifest.sokf.yaml",
        &format!("sokf: \"0.7\"\nname: guide-fixture\nscope: test\ndescription: A store for sokf guide.\n{line}"),
    );
}

// A store without `primary-language` is in English, so the scaffold holds
// sokf's English rule for a description before the schema's guidance, on
// the line before the key, and English words before each limit's
// `when-at-limit`: a ban for a hard limit of 0, and the `rule-id` of a
// named rule.
// sokf-tests: CLI-6_AC-118
// sokf-tests: CLI-6_AC-119
// sokf-tests: CLI-6_AC-120
// sokf-tests: CLI-6_AC-121
// sokf-tests: SOKF_SCHEMA_101
// sokf-tests: CONF-12_AC-1
#[test]
fn a_store_without_a_language_takes_sokfs_english() {
    let f = memo("");
    let out = scaffold(&f);
    assert!(
        out.contains(&format!(
            "\n# sokf:guidance {RULE} It tells that reader what they need beyond the title, in terms they already know, usually in one sentence. The description names the decision.\ndescription: "
        )),
        "{out}"
    );
    for words in [
        "# sokf:guidance Usually up to 2 values. Never leave out or merge content to stay within the count. When at the limit: Keep the tags that a reader searches for.\ntags:",
        "<!-- sokf:guidance At most 3 items. Never leave out or merge content to stay within the count. When at the limit: Write a second memo. No `todo` items. Instead: File an issue. At most 1 `point` item. Never leave out or merge content to stay within the count. When at the limit: Join the points. -->",
    ] {
        assert!(out.contains(words), "{words:?} is not in {out}");
    }
}

// The guide prints the rule for a description before the schema's
// guidance for the key, and the same words for each limit as the
// scaffold, and its JSON gives a rule's limit its `rule-id` and what it
// counts.
// sokf-tests: CLI-17_AC-7
// sokf-tests: CLI-17_AC-31
// sokf-tests: CLI-17_AC-32
#[test]
fn the_guide_says_what_the_scaffold_says() {
    let f = memo("");
    let (out, err, code) = f.run(&["new", "Memo", "--slug", "kettle", "--write"]);
    assert_eq!(code, 0, "{out}{err}");
    let (text, err, code) = f.run(&["guide", "memo-kettle"]);
    assert_eq!(code, 0, "{err}");
    let doc = std::fs::read_to_string(f.path("knowledge/memo-kettle.md")).unwrap();
    for line in doc.lines() {
        let line = line.trim();
        let words = line.strip_prefix("# sokf:guidance ").or_else(|| {
            line.strip_prefix("<!-- sokf:guidance ")
                .and_then(|l| l.strip_suffix(" -->"))
        });
        if let Some(words) = words {
            assert!(text.contains(words), "{words:?} is not in {text}");
        }
    }
    assert!(text.contains(&format!("\n  description: {RULE}")), "{text}");
    let (json, err, code) = f.run(&["guide", "memo-kettle", "--json"]);
    assert_eq!(code, 0, "{err}");
    let value = valid("guide-report.schema.json", &json);
    let limits = &value["documents"][0]["sections"][0]["blocks"][0]["limits"];
    assert_eq!(limits[0].get("counts"), None, "{limits}");
    assert_eq!(limits[1]["rule_id"], "todo", "{limits}");
    assert_eq!(limits[1]["counts"], "units", "{limits}");
    assert_eq!(limits[1]["count"], 0, "{limits}");
}

// A store in German without German texts gets no rule for a description
// and each limit in the syntax of the schema; a schema in English still
// takes the English texts; and the German texts of `/.sokf/texts.yaml`
// reach a schema in Swiss German.
// sokf-tests: CLI-6_AC-122
// sokf-tests: CLI-6_AC-123
// sokf-tests: SOKF_SCHEMA_101
// sokf-tests: SOKF_SCHEMA_102
// sokf-tests: CONF-12_AC-1
// sokf-tests: CONF-12_AC-6
#[test]
fn a_language_without_texts_takes_the_syntax_of_the_schema() {
    let f = memo("");
    with_manifest(&f, "primary-language: de\n");
    let out = scaffold(&f);
    assert!(
        !out.contains(RULE)
            && out.contains("# sokf:guidance The description names the decision.\ndescription: ")
            && out.contains("`hard: 3`, `when-at-limit`: Write a second memo.")
            && out.contains("`hard: 0`, `when-at-limit`: File an issue."),
        "{out}"
    );
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO.replace("LANGUAGE", "language: en\n"),
    );
    assert!(scaffold(&f).contains(RULE));
    f.write(
        "knowledge/schemas/memo.md",
        &MEMO.replace("LANGUAGE", "language: de-CH\n"),
    );
    f.write(
        ".sokf/texts.yaml",
        "DE:\n  description: Der Text neben dem Titel im Index.\n  limit:\n    hard: 'Höchstens {count} {unit}. An der Grenze:'\n  units:\n    item: { one: Punkt, other: Punkte }\n",
    );
    let out = scaffold(&f);
    assert!(
        out.contains("# sokf:guidance Der Text neben dem Titel im Index. The description names the decision.\n")
            && out.contains("Höchstens 3 Punkte. An der Grenze: Write a second memo.")
            // German has no ban, and no word for a value.
            && out.contains("`hard: 0`, `when-at-limit`: File an issue.")
            && out.contains("`soft: 2`, `when-at-limit`: Keep the tags"),
        "{out}"
    );
}

// A schema that a store inherits keeps the language of the store that
// holds it, because sokf's words stand beside the schema's own guidance.
// sokf-tests: SOKF_SCHEMA_101
#[test]
fn an_inherited_schema_keeps_the_language_of_its_own_store() {
    let f = memo("");
    f.write(
        ".sokf/config.toml",
        "[knowledge]\nroots = [\"/knowledge\", \"/other\"]\n",
    );
    let other = |language: &str| {
        f.write(
            "other/manifest.sokf.yaml",
            &format!("sokf: \"0.7\"\nname: other\nscope: test\nextends: test/guide-fixture\nprimary-language: {language}\n"),
        );
    };
    let new_other = || {
        let (out, err, code) = f.run(&["new", "other:Memo", "--slug", "kettle"]);
        assert_eq!(code, 0, "{out}{err}");
        out
    };
    other("de");
    assert!(new_other().contains(RULE));
    other("en");
    with_manifest(&f, "primary-language: de\n");
    assert!(!new_other().contains(RULE));
}

/// `ONE` and `TWO` are files of texts of two packs, each in a folder of its
/// own.
const ONE: &str = ".sokf/texts.d/pakf/@acme/one/texts.yaml";
const TWO: &str = ".sokf/texts.d/pakf/@acme/two/texts.yaml";

// No file of `/.sokf/texts.d/` takes precedence over another: two files
// that give one entry the same text count once, two that differ stop
// `new` until `/.sokf/texts.yaml` gives the entry, and the project's file
// replaces an entry of sokf's English.
// sokf-tests: CONF-12_AC-1
// sokf-tests: CONF-12_AC-2
// sokf-tests: CONF-12_AC-3
#[test]
fn the_files_of_texts_d_take_no_precedence() {
    let f = memo("");
    let hard = |w: &str| {
        format!("en:\n  limit:\n    hard: '{w} {{count}} {{unit}}. When at the limit:'\n")
    };
    f.write(ONE, &hard("Up to"));
    f.write(TWO, &hard("Up to"));
    assert!(scaffold(&f).contains("Up to 3 items. When at the limit: Write"));
    f.write(TWO, &hard("No more than"));
    let (out, err, code) = f.run(&["new", "Memo", "--slug", "kettle"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        out.is_empty()
            && err.contains(&format!("error: /{TWO}: "))
            && err.contains(&format!(
                "the text `limit.hard` of `en` is given differently in `/{ONE}` and `/{TWO}`"
            )),
        "{err}"
    );
    let (_, err, code) = f.run(&["guide", "note-kettle"]);
    assert!(
        code == 2 && err.contains("the text `limit.hard` of `en` is given differently"),
        "{err}"
    );
    f.write(".sokf/texts.yaml", &hard("Never more than"));
    assert!(scaffold(&f).contains("Never more than 3 items. When at the limit: Write"));
}

// A file of texts of another shape stops `new`, `guide`, and `validate` with
// an error that names the file, and `info`, which checks nothing, doesn't
// read it.
// sokf-tests: CONF-12_AC-4
// sokf-tests: CONF-12_AC-5
// sokf-tests: CONF-12_AC-7
// sokf-tests: CONF-12_AC-8
#[test]
fn a_bad_file_of_texts_stops_each_command_that_reads_it() {
    let f = memo("");
    for (text, says) in [
        ("en: [x]\n", "line 1: `en` is no map"),
        (
            "en:\n  limit:\n    soft: 'About {cnt}.'\n",
            "line 3: `limit.soft` holds `{cnt}`",
        ),
        ("en_GB: {}\n", "`en_GB` is no language tag"),
        (
            "en:\n  limit:\n    hard: 'At most {unit}.'\n",
            "line 3: `limit.hard` holds no `{count}`",
        ),
        (
            "en: {}\nEN: {}\n",
            "line 2: `EN` gives the language of line 1 again",
        ),
    ] {
        f.write(ONE, text);
        for args in [
            &["new", "Memo", "--slug", "kettle"][..],
            &["guide", "note-kettle"],
            &["validate"],
        ] {
            let (out, err, code) = f.run(args);
            assert_eq!(code, 2, "{text:?} {args:?}: {out}{err}");
            assert!(
                err.contains(&format!("error: /{ONE}: ")) && err.contains(says),
                "{text:?}: {err}"
            );
        }
        let (_, _, code) = f.run(&["info", "grammar"]);
        assert_eq!(code, 0, "{text:?}");
    }
}

// A `primary-language` that isn't a language tag is an error on its line.
// sokf-tests: SOKF_FORMAT_251
#[test]
fn a_primary_language_that_is_no_tag_is_an_error() {
    let f = Fixture::new("guide");
    with_manifest(&f, "primary-language: de_CH\n");
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("/knowledge/manifest.sokf.yaml:5: error: the manifest's `primary-language` is `de_CH`, which is no language tag in the syntax of BCP 47")
            && out.contains("(SOKF_FORMAT_251)"),
        "{out}"
    );
    with_manifest(&f, "primary-language: [de]\n");
    let (out, _, code) = f.run(&["validate"]);
    assert!(
        code == 1 && out.contains("the manifest's `primary-language` is no single value"),
        "{out}"
    );
    with_manifest(&f, "primary-language: de-CH\n");
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{out}");
}
