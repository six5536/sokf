//! These tests cover references across knowledge stores. A store's identity
//! is `<scope>/<name>`. A reference into another store puts the store in front
//! of the form that the reference takes inside that store. Each target has one
//! spelling, the shortest. `--fix` writes that spelling wherever the rewrite
//! changes the spelling and not the target.

use common::{TempTree, said};
use sokf_core::CacheMode;

use crate::common;

/// `NOTE_SCHEMA` is the schema of a note: one heading and any number of
/// paragraphs.
const NOTE_SCHEMA: &str = "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nlinks:\n  - rel: references\n    min: 0\nsections:\n  - heading: Note\n    level: 1\n    blocks:\n      - kind: prose\n        repeat: true\n        example: Milk, bread, and the good coffee.\n````\n";

/// `two_stores` returns the `clean` fixture, the store `test/fixture`, with a
/// second store, `test/other`, that holds a note. The note's frontmatter ends
/// with `links`, and its body is `body`.
fn two_stores(links: &str, body: &str) -> TempTree {
    let tree = TempTree::fixture("clean");
    tree.add_other_store();
    tree.write("other/schemas/note.md", NOTE_SCHEMA);
    tree.write(
        "other/note.md",
        &format!("---\ntype: Note\nid: note\n{links}---\n\n# Note\n\n{body}\n"),
    );
    tree
}

fn errors_on(tree: &TempTree, fix: bool, path: &str) -> Vec<String> {
    tree.validate(fix, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| f.path.as_str() == path)
        .map(|f| said(&f.message))
        .collect()
}

// Each spelling longer than the shortest produces a finding. The finding
// gives the shortest spelling, and `--fix` writes that spelling. The test covers a store prefix
// that is longer than it needs to be, this store's own prefix, a bare key
// that lives in another store, a path to a document in either store, and the
// path in a `links` entry. A reference to a store that is not a knowledge
// root stays as written.
// sokf-tests: SOKF_FORMAT_60
// sokf-tests: SOKF_FORMAT_64
// sokf-tests: SOKF_FORMAT_65
// sokf-tests: SOKF_VALIDATION_24
// sokf-tests: CONF-2_AC-3
#[test]
fn every_spelling_but_the_shortest_is_respelled() {
    let body = "Read [the criteria][sokf:fixture:req-cli#acceptance-criteria], [CLI-1][sokf:#cli-1], [the overview][sokf:test/fixture:overview], [this note][sokf:other:note], [the story](/knowledge/req-cli.md#cli-1) and [again](note.md).\n\nA key is written `[sokf:#cli-1]`, as [the\nstory][sokf:#cli-1] is.";
    let tree = two_stores(
        "links:\n  - rel: references\n    to: /knowledge/overview.md\n",
        body,
    );
    let before = errors_on(&tree, false, "/other/note.md");
    for expected in [
        "write `[sokf:#cli-1]` as `[sokf:fixture:#cli-1]`",
        "write `[sokf:test/fixture:overview]` as `[sokf:fixture:overview]`",
        "write `[sokf:other:note]` as `[sokf:note]`",
        "the link refers to the document `/knowledge/req-cli.md` by path. Refer to it by id as `[text][sokf:fixture:req-cli#cli-1]`",
        "the link refers to the document `/other/note.md` by path. Refer to it by id as `[text][sokf:note]`",
        "write the `links` entry `to: /knowledge/overview.md` as `to: fixture:overview`",
    ] {
        assert!(
            before.iter().any(|m| m == expected),
            "{expected}\n{before:#?}"
        );
    }
    let after = errors_on(&tree, true, "/other/note.md");
    assert!(after.is_empty(), "{after:?}");
    let note = tree.read("other/note.md");
    for written in [
        "to: fixture:overview",
        "Read [the criteria][sokf:fixture:req-cli#acceptance-criteria], [CLI-1][sokf:fixture:#cli-1], [the overview][sokf:fixture:overview], [this note][sokf:note], [the story][sokf:fixture:req-cli#cli-1] and [again][sokf:note].",
        "[sokf:fixture:#cli-1]: /knowledge/req-cli.md#cli-1",
        // A label written as code is an example, not a link, and stays as
        // written. A label that a wrapped link carries on its next line is
        // respelled.
        "A key is written `[sokf:#cli-1]`, as [the\nstory][sokf:fixture:#cli-1] is.",
        "[sokf:fixture:overview]: /knowledge/overview.md",
    ] {
        assert!(note.contains(written), "{written}\n{note}");
    }
}

// A reference that uses a store's former identity still resolves, and
// `--fix` respells it with the current identity. A reference to a name that
// is neither a current nor a former identity is a finding, and stays as
// written.
// sokf-tests: SOKF_FORMAT_64
// sokf-tests: SOKF_VALIDATION_24
#[test]
fn a_former_identity_resolves_and_is_respelled() {
    let tree = two_stores(
        "",
        "See [the overview][sokf:legacy:overview], [whole][sokf:test/legacy:overview].",
    );
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: fixture\nscope: test\nrenamed-from: [test/legacy]\ndescription: A clean fixture store.\n",
    );
    assert!(
        errors_on(&tree, false, "/other/note.md")
            .contains(&"write `[sokf:legacy:overview]` as `[sokf:fixture:overview]`".to_string())
    );
    assert_eq!(
        errors_on(&tree, true, "/other/note.md"),
        [] as [std::string::String; 0]
    );
    assert!(
        tree.read("other/note.md")
            .contains("See [the overview][sokf:fixture:overview], [whole][sokf:fixture:overview].")
    );
    let gone = two_stores(
        "links:\n  - rel: references\n    to: nowhere:x\n",
        "See [gone][sokf:nowhere:x].",
    );
    let got = errors_on(&gone, true, "/other/note.md");
    for expected in [
        "`[sokf:nowhere:x]` refers to the store `nowhere`, which is not a knowledge root of this repository",
        "`links` entry `to: nowhere:x` refers to the store `nowhere`, which is not a knowledge root of this repository",
    ] {
        assert!(got.iter().any(|m| m == expected), "{expected}\n{got:#?}");
    }
    assert!(gone.read("other/note.md").contains("[sokf:nowhere:x]"));
}

// When two roots share an identity, current or former, validation reports a
// finding on each root's manifest. A manifest must name its owner in `scope`, and its
// `name` and `renamed-from` entries must be slugs.
// sokf-tests: SOKF_FORMAT_10
// sokf-tests: SOKF_FORMAT_11
// sokf-tests: SOKF_FORMAT_12
// sokf-tests: CLI-4_AC-29
#[test]
fn an_identity_two_roots_hold_is_a_finding_on_each() {
    let tree = two_stores("", "Milk.");
    tree.write(
        "other/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: other\nscope: test\nrenamed-from: [test/fixture]\n",
    );
    assert_eq!(
        errors_on(&tree, false, "/other/manifest.sokf.yaml"),
        vec![
            "/knowledge also uses the identity `test/fixture`. A store's identity, current or former, must be unique among the roots"
        ]
    );
    assert_eq!(
        errors_on(&tree, false, "/knowledge/manifest.sokf.yaml"),
        vec![
            "/other also uses the identity `test/fixture`. A store's identity, current or former, must be unique among the roots"
        ]
    );
    tree.write(
        "other/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: Other\nrenamed-from: [other]\n",
    );
    assert_eq!(
        errors_on(&tree, false, "/other/manifest.sokf.yaml"),
        vec![
            "the manifest declares no `scope`. The scope names the store's owner as a slug: lower-case letters and digits joined by single hyphens. The store's identity is `<scope>/<name>`",
            "the manifest's `name` `Other` is not a slug. A slug is lower-case letters and digits joined by single hyphens",
            "the manifest's `renamed-from` entry `other` is not of the form `<scope>/<name>`, where both parts are slugs",
        ]
    );
}

/// `page_including` returns two stores. The store `test/other` holds an
/// asset, a page schema with includes that must match `^/`, and a page that
/// includes `target`.
fn page_including(target: &str) -> TempTree {
    let tree = two_stores("", "Milk.");
    tree.write(
        "other/schemas/data.md",
        "---\ntype: Schema\nid: schema-data\n---\n\n````yaml\nline-limit: 0\nasset-files: /other/data/*.csv\nfrontmatter:\n  type:\n    const: Data\nsections:\n  - heading: Rates\n    level: 1\n    blocks:\n      - kind: prose\n        example: The daily rates.\n````\n",
    );
    tree.write("other/data/rates.csv", "day,rate\n1,2\n");
    tree.write(
        "other/data/rates.csv.md",
        "---\ntype: Data\nid: rates\nresource: /other/data/rates.csv\n---\n\n# Rates\n\nThe daily rates.\n",
    );
    tree.write(
        "other/schemas/page.md",
        "---\ntype: Schema\nid: schema-page\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Page\nsections:\n  - heading: Page\n    level: 1\n    blocks:\n      - kind: include\n        source-pattern: '^/'\n        example: |\n          <!-- sokf:include /other/data/rates.csv -->\n          <!-- /sokf:include -->\n````\n",
    );
    tree.write(
        "other/page.md",
        &format!(
            "---\ntype: Page\nid: page\n---\n\n# Page\n\n<!-- sokf:include {target} -->\n<!-- /sokf:include -->\n"
        ),
    );
    tree
}

// When a page includes an asset by its sidecar's id, validation matches the
// include's `source-pattern` against the asset's path, so `^/` accepts a
// file and an asset and refuses a document. `--fix` respells an asset's path
// as its id, which still passes the same pattern.
// sokf-tests: EXT-4_AC-1
// sokf-tests: SOKF_VALIDATION_24
#[test]
fn a_path_pattern_reads_an_asset_include_by_its_assets_path() {
    assert_eq!(
        errors_on(&page_including("rates"), true, "/other/page.md"),
        [] as [std::string::String; 0]
    );
    let respelled = page_including("/other/data/rates.csv");
    assert_eq!(
        errors_on(&respelled, true, "/other/page.md"),
        [] as [std::string::String; 0]
    );
    assert!(
        respelled
            .read("other/page.md")
            .contains("<!-- sokf:include rates -->")
    );
    let got = errors_on(&page_including("note"), true, "/other/page.md");
    assert!(
        got.iter().any(|m| m.starts_with(
            "`sokf:include note` names a document; this rule takes what matches `^/`, a file or an asset by its path"
        )),
        "{got:?}"
    );
}

// A qualified label makes an edge between documents of the two stores. The
// commands accept every form of a reference, including a key alone.
// sokf-tests: CLI-8_AC-9
// sokf-tests: CLI-8_AC-11
#[test]
fn a_qualified_reference_is_an_edge_and_the_commands_read_it() {
    let tree = two_stores("", "See [the overview][sokf:fixture:overview].");
    tree.validate(true, CacheMode::Off);
    let model = sokf_core::load_model(
        &tree.root(),
        &sokf_core::load_config(&tree.root()).unwrap(),
        &sokf_core::Cache::open(&tree.root(), CacheMode::Off),
    )
    .unwrap();
    let graph = sokf_core::graph::build(&model);
    let path = |p: &str| sokf_core::NodeKey::Path(sokf_core::RepoPath::parse(p).unwrap());
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.from == path("/other/note.md") && e.to == path("/knowledge/overview.md")),
        "{:?}",
        graph.edges
    );
    let found = |reference: &str| {
        sokf_core::locate(&tree.root(), &[reference.into()], false, CacheMode::Off)
            .map(|l| l[0].path.clone())
    };
    let req = tree.dir().join("knowledge/req-cli.md");
    for reference in [
        "req-cli",
        "fixture:req-cli",
        "sokf:fixture:#cli-1",
        "#cli-1_ac-2",
    ] {
        assert_eq!(found(reference).unwrap(), req, "{reference}");
    }
    assert!(found("other:#cli-1").is_err());
    // When a key identifies a document in each of two stores, the error
    // lists each qualified form.
    tree.write(
        "other/schemas/requirement.md",
        &tree.read("knowledge/schemas/requirement.md"),
    );
    tree.write(
        "other/req-two.md",
        &tree
            .read("knowledge/req-cli.md")
            .replace("id: req-cli", "id: req-two"),
    );
    assert_eq!(
        found("#cli-1").unwrap_err().to_string(),
        "locate: `#cli-1` identifies more than one document: /knowledge/req-cli.md, /other/req-two.md. Write `fixture:#cli-1` or `other:#cli-1`"
    );
    assert_eq!(
        found("other:#cli-1").unwrap(),
        tree.dir().join("other/req-two.md")
    );
}

// A page includes another store's documents and assets as it includes its
// own store's. It includes an asset by its sidecar's qualified id, and
// `--fix` replaces an asset's path with that id. It includes a document by
// its qualified id. The include shows the document's body, with each label
// respelled as this store writes it. An include of a
// document by its path is a finding, and `--fix` does not rewrite it, because
// an include by path shows different text from an include by id.
// sokf-tests: SOKF_FORMAT_64
// sokf-tests: SOKF_FORMAT_85
// sokf-tests: SOKF_FORMAT_88
// sokf-tests: SOKF_VALIDATION_24
#[test]
fn an_include_reaches_another_store_by_its_qualified_id() {
    let tree = TempTree::fixture("clean");
    tree.add_other_store();
    tree.write(
        "other/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: Note\n    level: 1\n    blocks:\n      - kind: prose\n        example: Milk, bread, and the good coffee.\n````\n",
    );
    tree.write(
        "other/schemas/data.md",
        "---\ntype: Schema\nid: schema-data\n---\n\n````yaml\nline-limit: 0\nasset-files: /other/data/*.csv\nfrontmatter:\n  type:\n    const: Data\nsections:\n  - heading: Rates\n    level: 1\n    blocks:\n      - kind: prose\n        example: The daily rates.\n````\n",
    );
    tree.write(
        "other/note.md",
        "---\ntype: Note\nid: note\n---\n\n# Note\n\nMilk at [the day's rate][sokf:rates], written `[sokf:rates]`.\n\n<!-- sokf:links -->\n\n[sokf:rates]: /other/data/rates.csv\n",
    );
    tree.write("other/data/rates.csv", "day,rate\n1,2\n");
    tree.write(
        "other/data/rates.csv.md",
        "---\ntype: Data\nid: rates\nresource: /other/data/rates.csv\n---\n\n# Rates\n\nThe daily rates.\n",
    );
    tree.write(
        "knowledge/schemas/page.md",
        "---\ntype: Schema\nid: schema-page\ndescription: A page of includes.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Page\nsections:\n  - heading: Page\n    level: 1\n    blocks:\n      - kind: include\n        repeat: true\n        example: |\n          <!-- sokf:include other:note -->\n          <!-- /sokf:include -->\n````\n",
    );
    tree.write(
        "knowledge/page.md",
        "---\ntype: Page\nid: page\n---\n\n# Page\n\n<!-- sokf:include other:note -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /other/data/rates.csv -->\n<!-- /sokf:include -->\n",
    );
    tree.write(
        "knowledge/page-path.md",
        "---\ntype: Page\nid: page-path\n---\n\n# Page\n\n<!-- sokf:include /other/note.md -->\n<!-- /sokf:include -->\n",
    );
    let report = tree.validate(true, CacheMode::Off);
    let on = |file: &str| -> Vec<String> {
        report
            .errors
            .iter()
            .filter(|f| f.path.as_str() == format!("/knowledge/{file}"))
            .map(sokf_core::report_message)
            .collect()
    };
    assert!(on("page.md").is_empty(), "{:?}", on("page.md"));
    let page = tree.read("knowledge/page.md");
    assert!(
        page.contains("<!-- sokf:include other:rates -->\n\n```csv\nday,rate\n1,2\n```"),
        "{page}"
    );
    assert!(
        page.contains("Milk at [the day's rate][sokf:other:rates], written `[sokf:rates]`."),
        "{page}"
    );
    assert!(
        page.contains("[sokf:other:rates]: /other/data/rates.csv"),
        "{page}"
    );
    assert_eq!(
        on("page-path.md"),
        vec![
            "`sokf:include /other/note.md` refers to a document by path. Include its body by id as `sokf:include other:note`, or show its source in a code block (SOKF_FORMAT_88)"
        ]
    );
    assert!(
        tree.read("knowledge/page-path.md")
            .contains("<!-- sokf:include /other/note.md -->")
    );
    assert!(
        tree.validate(false, CacheMode::Off)
            .errors
            .iter()
            .all(|f| f.path.as_str() != "/knowledge/page.md")
    );
}

// `--fix` respells a reference whatever form it is written in: a quoted
// `to`, an inline link with a title, and a path with a fragment. When the
// fragment of a path is the key's text, `--fix` writes the fragment as the
// key's slug. Validation reports a link that takes its path from a reference
// definition, with a message that leaves the repair to the writer. `--fix`
// leaves that link and the definition block as they are. A second
// `--fix` rewrites nothing.
// sokf-tests: SOKF_VALIDATION_24
// sokf-tests: SOKF_VALIDATION_32
// sokf-tests: SOKF_VALIDATION_37
#[test]
fn fix_respells_what_is_written_and_nothing_else() {
    let tree = two_stores(
        "links:\n  - rel: references\n    to: \"/knowledge/overview.md\"\nsources:\n  - id: fn\n    resource: /knowledge/overview.md\n",
        "The [overview](/knowledge/overview.md \"Overview\"), [the story](/knowledge/req-cli.md#CLI-1) and [the req][d].[^fn] ![Not an image](/knowledge/overview.md)\n\n[d]: /knowledge/req-cli.md\n\n[^fn]: Read [the\n    overview](/knowledge/overview.md).",
    );
    let before = errors_on(&tree, false, "/other/note.md");
    assert!(
        before.iter().any(|m| m
            == "the link refers to the document `/knowledge/req-cli.md` by path. Refer to it by id as `[text][sokf:fixture:req-cli#cli-1]`"),
        "{before:#?}"
    );
    let after = errors_on(&tree, true, "/other/note.md");
    assert!(
        after.iter().any(|m| m
            == "the link refers to the document `/knowledge/req-cli.md` by path. Refer to it by id as `[text][sokf:fixture:req-cli]`"),
        "{after:#?}"
    );
    let note = tree.read("other/note.md");
    for written in [
        "    to: fixture:overview\n",
        // An image that points to a document is not an image of an asset,
        // and keeps its path.
        "The [overview][sokf:fixture:overview], [the story][sokf:fixture:req-cli#cli-1] and [the req][d].[^fn] ![Not an image](/knowledge/overview.md)",
        // A link inside a footnote, which is not a unit of any block, is
        // respelled even when it wraps onto its next line.
        "[^fn]: Read [the\n    overview][sokf:fixture:overview].",
        "[sokf:fixture:overview]: /knowledge/overview.md\n[sokf:fixture:req-cli#cli-1]: /knowledge/req-cli.md#cli-1\n",
    ] {
        assert!(note.contains(written), "{written}\n{note}");
    }
    assert!(!note.contains("[sokf:fixture:req-cli]:"), "{note}");
    assert_eq!(
        tree.validate(true, CacheMode::Off).fixes,
        [] as [sokf_core::FixOp; 0]
    );
}
