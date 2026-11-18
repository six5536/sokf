//! Each test validates one fixture repository and checks its report, often
//! against a JSON snapshot.

use common::{TempTree, error_lines, json, new_options};
use sokf_core::{CacheMode, InfoLayer, NewOptions, info_formats, new};

use crate::common;

// sokf-tests: CLI-10_AC-1
#[test]
fn a_clean_store_has_no_findings() {
    let tree = TempTree::fixture("clean");
    let report = tree.validate(false, CacheMode::Off);
    assert!(report.errors.is_empty(), "{}", json(&report));
    assert_eq!(report.warnings, [] as [sokf_core::Finding; 0]);
    assert_eq!(report.fixes, [] as [sokf_core::FixOp; 0]);
}

// sokf-tests: SOKF_VALIDATION_1
// sokf-tests: SOKF_VALIDATION_45
// sokf-tests: SOKF_VALIDATION_46
#[test]
fn the_broken_store_reports_every_step() {
    let tree = TempTree::fixture("broken");
    let report = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!(json(&report));
}

// sokf-tests: CLI-10_AC-2
#[test]
fn manifest_problems_are_reported() {
    let tree = TempTree::fixture("manifest");
    let report = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!(json(&report));
    let tree = TempTree::fixture("no-manifest");
    let report = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!(json(&report));
}

// sokf-tests: CLI-10_AC-2
// sokf-tests: CLI-10_AC-6
#[test]
fn an_earlier_version_says_what_changed() {
    let tree = TempTree::fixture("clean");
    let manifest = tree.read("knowledge/manifest.sokf.yaml");
    let declare = |version: &str| {
        tree.write(
            "knowledge/manifest.sokf.yaml",
            &manifest.replace("sokf: \"0.7\"", &format!("sokf: \"{version}\"")),
        );
        error_lines(&tree.validate(false, CacheMode::Off))
    };
    assert_eq!(
        declare("0.6"),
        [
            "/knowledge/manifest.sokf.yaml:Some(1): the manifest declares sokf `0.6`. This tool implements `0.7`. Version 0.7 changes the schemas: each `max` is a map with its `when-at-limit`, and sokf supplies no line format. Fix the findings on the store's schemas, and then declare `sokf: \"0.7\"`. The changelog lists every change: https://github.com/six5536/sokf/blob/main/sokf-spec/changelog.md#07"
        ]
    );
    for other in ["0.8", "1.0", "0.7.1", "seven"] {
        assert_eq!(
            declare(other),
            [format!(
                "/knowledge/manifest.sokf.yaml:Some(1): the manifest declares sokf `{other}`. This tool implements `0.7`"
            )],
            "{other}"
        );
    }
}

// sokf-tests: SOKF_FORMAT_96
// sokf-tests: SOKF_FORMAT_103
// sokf-tests: SOKF_SCHEMA_5
// sokf-tests: CLI-4_AC-28
#[test]
fn a_broken_region_is_reported_at_its_marker() {
    let tree = TempTree::fixture("clean");
    let text = tree.read("src/main.rs");
    let end = "// sokf:end cli\n";
    assert!(text.contains(end));
    tree.write("src/main.rs", &text.replace(end, ""));
    let report = tree.validate(false, CacheMode::Off);
    let errors = error_lines(&report);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(
        errors[0].starts_with("/src/main.rs:Some(3): region `cli` begins at line 3 but never ends"),
        "{errors:#?}"
    );
}

// sokf-tests: CLI-4_AC-28
// sokf-tests: CACHE-1_AC-14
#[test]
fn a_broken_region_is_reported_from_the_store_until_it_is_repaired() {
    use common::{
        age,
        incremental::{edit, run},
    };
    let tree = TempTree::fixture("clean");
    let text = tree.read("src/main.rs");
    age(tree.dir(), 10);
    run(&tree);
    edit(&tree, "src/main.rs", &text.replace("// sokf:end cli\n", ""));
    // `said` drops the key, which `run` might or might not print.
    let broken = "/src/main.rs:Some(3): region `cli` begins at line 3 but never ends";
    let said = |findings: Vec<String>| -> Vec<String> {
        findings.iter().map(|f| common::said(f)).collect()
    };
    let (_, _, findings, _) = run(&tree);
    assert_eq!(said(findings), vec![broken.to_string()]);
    // Nothing has changed, so the run reports the finding from the store.
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(said(findings), vec![broken.to_string()]);
    // After the region is repaired, the run checks the readme again and the
    // finding goes away.
    edit(&tree, "src/main.rs", &text);
    let (dirty, _, findings, _) = run(&tree);
    assert!(dirty.contains("/README.md"), "{dirty:?}");
    assert!(findings.is_empty(), "{findings:?}");
}

// sokf-tests: CLI-4_AC-27
#[test]
fn indexes_are_checked_per_directory() {
    let tree = TempTree::fixture("indexes");
    let report = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!(json(&report));
}

// sokf-tests: SOKF_VALIDATION_2
// sokf-tests: CLI_P-6
#[test]
fn a_crlf_copy_reports_the_same() {
    let tree = TempTree::fixture("broken");
    let lf = tree.validate(false, CacheMode::Off);
    for (rel, text) in tree.files() {
        tree.write(&rel, &text.replace('\n', "\r\n"));
    }
    let crlf = tree.validate(false, CacheMode::Off);
    assert_eq!(lf.errors, crlf.errors);
    assert_eq!(lf.warnings, crlf.warnings);
}

// sokf-tests: CONF-2_AC-2
#[test]
fn two_roots_are_two_knowledges() {
    let tree = TempTree::fixture("clean");
    tree.add_other_store();
    tree.write(
        "other/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: Note\n    level: 1\n    blocks:\n      - kind: prose\n        example: Milk, bread, and the good coffee.\n````\n",
    );
    // Seen from the first root, a document in the other root is an ordinary
    // repository file: a link by path resolves, but a link by id does not.
    tree.write(
        "other/note.md",
        "---\ntype: Note\nid: req-cli\n---\n\n# Note\n\nSee [the requirement](/knowledge/req-cli.md) and [by id][sokf:overview].\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!(json(&report));
}

#[test]
fn examples_are_checked_as_documents_and_may_not_carry_markers() {
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\ndescription: A note.\n---\n\n````yaml\nline-limit: 0\nlifecycle:\n  - { key: state, states: [draft, done] }\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: Note\n    level: 1\n    blocks:\n      - kind: prose\n        example: Milk, bread, and the good coffee.\n  - heading: Outcome\n    level: 1\n    guidance: The outcome.\n    only: { state: done }\nexamples:\n  - document: |\n      ---\n      type: Note\n      id: n2\n      state: done\n      ---\n\n      # Note\n\n      Text.\n\n      # Outcome\n\n      Done.\n  - document: |\n      ---\n      type: Note\n      id: n3\n      state: draft\n      ---\n\n      # Wrong\n  - document: |\n      ---\n      type: Note\n      id: n4\n      state: draft\n      ---\n\n      # Note\n\n      <!-- sokf:only state=done -->\n      Text.\n````\n",
    );
    let report = tree.validate_paths(&["knowledge/schemas/note.md"], false);
    let messages: Vec<String> = report
        .errors
        .iter()
        .map(sokf_core::report_message)
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("example 2 fails: ") && m.ends_with(" (TL_69)")),
        "{messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.starts_with(
            "example 3 fails: line 9: [TLMD_10] a `sokf:only` marker belongs in a scaffold"
        ) && m.ends_with(" (TL_69)")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.starts_with("example 1")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.starts_with("template")),
        "{messages:?}"
    );
}

// sokf-tests: CLI-4_AC-2
#[test]
fn paths_narrow_the_report() {
    let tree = TempTree::fixture("broken");
    let full = tree.validate(false, CacheMode::Off);
    let narrowed = tree.validate_paths(&["docs"], false);
    assert!(
        narrowed
            .errors
            .iter()
            .all(|f| f.path.relative().starts_with("docs/"))
    );
    assert_ne!(narrowed.errors, [] as [sokf_core::Finding; 0]);
    assert!(narrowed.errors.len() < full.errors.len());
    let one = tree.validate_paths(&["knowledge/other/orphan.md"], false);
    assert_eq!(one.errors.len(), 1, "{}", json(&one));
}

// sokf-tests: CLI-4_AC-1
#[test]
fn a_run_with_paths_checks_every_file() {
    use sokf_core::{
        Cache, Report, ValidateOptions, check_repository, grammar::RegexCache, load_config,
        load_grammar, load_repo_formats, validate,
    };
    let tree = TempTree::fixture("broken");
    common::age(tree.dir(), 10);
    let root = tree.root();
    let narrowed = validate(
        &root,
        &ValidateOptions {
            paths: vec!["docs".into()],
            cwd: tree.dir().to_path_buf(),
            fix: false,
            cache: CacheMode::On,
            query: None,
            changed: vec![],
            home: None,
        },
    )
    .unwrap()
    .0;
    // The next run has nothing to check, because the run with `PATHS`
    // checked and stored every file, including the files outside `PATHS`.
    let cache = Cache::open(&root, CacheMode::On);
    let grammar = load_grammar().unwrap();
    let formats = load_repo_formats(&root, &grammar, &RegexCache::new()).unwrap();
    let (loaded, findings) = check_repository(
        &root,
        &load_config(&root).unwrap(),
        &grammar,
        &formats,
        &cache,
        false,
    )
    .unwrap();
    assert!(!loaded.full);
    assert!(loaded.dirty.is_empty(), "{:?}", loaded.dirty);
    let mut stored = Report::default();
    stored.extend(findings);
    let stored = stored.finish();
    let cold = tree.validate_all(false, CacheMode::Off);
    // Compare only the path and the message, because this test is not about
    // the line of a finding that the store returns.
    let said = |r: &Report| -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = r
            .errors
            .iter()
            .map(|f| (f.path.to_string(), f.message.clone()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(said(&stored), said(&cold));
    assert!(narrowed.errors.len() < cold.errors.len());
}

// sokf-tests: CLI_P-1
// sokf-tests: CACHE-1_AC-10
#[test]
fn a_cached_second_run_reports_the_same() {
    let tree = TempTree::fixture("broken");
    let first = tree.validate(false, CacheMode::On);
    assert!(tree.dir().join(".sokf/cache/.gitignore").exists());
    let second = tree.validate(false, CacheMode::On);
    let uncached = tree.validate(false, CacheMode::Off);
    assert_eq!(first, second);
    assert_eq!(first, uncached);
}

/// `id_only` runs `sokf new --id-only` through the core library.
fn id_only(tree: &TempTree, type_name: &str, slug: Option<&str>) -> sokf_core::Result<String> {
    new(
        &tree.root(),
        &NewOptions {
            slug: slug.map(str::to_string),
            id_only: true,
            ..new_options(tree.dir(), type_name)
        },
    )
    .map(|r| r.id)
}

// sokf-tests: CLI-6_AC-4
// sokf-tests: CLI-6_AC-13
// sokf-tests: CLI-6_AC-17
// sokf-tests: CLI-6_AC-18
// sokf-tests: CLI-6_AC-47
// sokf-tests: CLI-6_AC-50
#[test]
fn the_id_alone_on_a_valid_an_empty_and_a_broken_type() {
    let tree = TempTree::fixture("clean");
    assert_eq!(
        id_only(&tree, "Issue", Some("retry")).unwrap(),
        "issue-002-retry"
    );
    let e = id_only(&tree, "Issue", None).unwrap_err().to_string();
    assert!(e.contains("`--slug` is required"), "{e}");
    // An empty type has no document yet.
    tree.write(
        "knowledge/schemas/task.md",
        "---\ntype: Schema\nid: schema-task\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Task\n  id:\n    template: 'task-${:seq}'\n    seq-width: 4\nsections: []\n````\n",
    );
    assert_eq!(id_only(&tree, "Task", None).unwrap(), "task-0001");
    let e = id_only(&tree, "Task", Some("x")).unwrap_err().to_string();
    assert!(e.contains("no `slug` placeholder"), "{e}");
    // A broken type has a document whose frontmatter does not parse.
    tree.write(
        "knowledge/task-bad.md",
        "---\ntype: Task\nid: &a task-0001\n---\n\n# T\n",
    );
    let e = id_only(&tree, "Task", None).unwrap_err().to_string();
    assert!(e.contains("does not parse"), "{e}");
    assert!(id_only(&tree, "Nope", None).is_err());
    let e = id_only(&tree, "Decision", None).unwrap_err().to_string();
    assert!(e.contains("no id template"), "{e}");
}

// sokf-tests: CLI-5_AC-4
// sokf-tests: CLI-5_AC-5
// sokf-tests: CLI-5_AC-6
// sokf-tests: CLI-5_AC-7
// sokf-tests: CLI-5_AC-10
#[test]
fn info_formats_per_layer_and_per_schema() {
    let tree = TempTree::fixture("clean");
    let embedded = info_formats(None, InfoLayer::Embedded, None, CacheMode::Off).unwrap();
    insta::assert_snapshot!(embedded);
    assert!(info_formats(None, InfoLayer::Effective, None, CacheMode::Off).is_err());
    assert!(info_formats(Some(&tree.root()), InfoLayer::Repo, None, CacheMode::Off).is_err());
    let effective = info_formats(
        Some(&tree.root()),
        InfoLayer::Effective,
        None,
        CacheMode::Off,
    )
    .unwrap();
    assert_eq!(effective, embedded);
    // The repository declares a loose `requirement-ears`, which
    // `schema-requirement` declares again in its own formats.
    tree.write(
        ".sokf/formats.yaml",
        "registry:\n  requirement-ears:\n    key: id\n    pattern: '^(?P<id>\\S+).*$'\n    example: loose\n  extra:\n    pattern: '^e$'\n    example: e\n",
    );
    let repo = info_formats(Some(&tree.root()), InfoLayer::Repo, None, CacheMode::Off).unwrap();
    assert!(repo.contains("source: repo") && !repo.contains("user-story"));
    let effective = info_formats(
        Some(&tree.root()),
        InfoLayer::Effective,
        None,
        CacheMode::Off,
    )
    .unwrap();
    assert!(
        !effective.contains("user-story")
            && effective.contains("extra")
            && effective.contains("example: loose")
    );
    // The view of the schema lays the schema's own formats on the
    // repository's.
    let schema = info_formats(
        Some(&tree.root()),
        InfoLayer::Effective,
        Some("schema-requirement"),
        CacheMode::Off,
    )
    .unwrap();
    assert!(
        schema.contains("user-story")
            && schema.contains("source: schema-requirement")
            && schema.contains("extra")
            && !schema.contains("example: loose"),
        "{schema}"
    );
    assert!(
        info_formats(
            Some(&tree.root()),
            InfoLayer::Effective,
            Some("nope"),
            CacheMode::Off
        )
        .is_err()
    );
    tree.write(
        ".sokf/formats.yaml",
        "registry:\n  bad:\n    pattern: '^b$'\n",
    );
    assert!(info_formats(Some(&tree.root()), InfoLayer::Repo, None, CacheMode::Off).is_err());
}

// sokf-tests: SOKF_FORMAT_85
// sokf-tests: SOKF_VALIDATION_15
#[test]
fn a_schema_is_an_include_source() {
    let tree = TempTree::fixture("clean");
    let text = tree.read("knowledge/overview.md");
    tree.write(
        "knowledge/overview.md",
        &text.replace(
            "<!-- /sokf:include -->",
            "<!-- /sokf:include -->\n\n<!-- sokf:include schema-issue -->\n\n<!-- /sokf:include -->",
        ),
    );
    let fixed = tree.validate(true, CacheMode::Off);
    assert!(fixed.errors.is_empty(), "{}", json(&fixed));
    assert!(
        tree.read("knowledge/overview.md")
            .contains("root: /knowledge/issues")
    );
}

const SPEC_SCHEMA: &str = "---\ntype: Schema\nid: schema-spec\ndescription: A spec.\n---\n\n````yaml\nline-limit: 0\nlifecycle:\n  - { key: mode, states: [a, b] }\nfrontmatter:\n  type:\n    const: Spec\nsections:\n  - heading: { from: title }\n    level: 1\n    guidance: d\n    blocks: []\n  - heading: Items\n    level: 2\n    guidance: d\n    blocks:\n      - kind: group\n        repeat: true\n        rule-id: items\n        blocks:\n          - kind: prose\n            example: 'One line of prose.'\n            only: { mode: a }\n            max: { hard: 1, when-at-limit: 'Put the rest in the item''s list.' }\n          - kind: bullet-list\n            each: { pattern: '^(?P<k>[A-Z]+-[0-9]+)', key: k, example: 'X-1 one' }\n  - heading: Details\n    level: 2\n    guidance: d\n    blocks:\n      - kind: bullet-list\n        each: { pattern: '^(?P<k>[A-Z]+-[0-9]+\\.[0-9]+)', key: k, key-derives-from: { rule: items, separator: '.' }, example: 'X-1.1 detail' }\n````\n";

#[test]
fn a_group_rule_id_names_its_captures_and_opens_with_its_first_admitted_block() {
    let tree = TempTree::fixture("clean");
    tree.write("knowledge/schemas/spec.md", SPEC_SCHEMA);
    tree.write(
        "knowledge/spec-a.md",
        "---\ntype: Spec\nid: spec-a\ntitle: A\ndescription: A.\nmode: a\n---\n\n# A\n\n## Items\n\nIntro.\n\n- X-1 one\n\nMore.\n\n- X-2 two\n\n## Details\n\n- X-1.1 detail\n- X-2.1 detail\n",
    );
    tree.write(
        "knowledge/spec-b.md",
        "---\ntype: Spec\nid: spec-b\ntitle: B\ndescription: B.\nmode: b\n---\n\n# B\n\n## Items\n\n- Y-1 one\n\n## Details\n\n- Y-1.1 detail\n- Z-1.1 stray\n",
    );
    let report = tree.validate(false, CacheMode::Off);
    // The added files make two indexes stale, and the document check reports
    // the only other finding.
    let errors: Vec<_> = report
        .errors
        .iter()
        .filter(|f| f.path.file_name() != "index.md")
        .collect();
    assert_eq!(errors.len(), 1, "{}", json(&report));
    assert!(errors[0].message.contains("Z-1.1"), "{}", errors[0].message);
    assert_eq!(errors[0].path.relative(), "knowledge/spec-b.md");
}

#[test]
fn const_on_a_list_key_applies_to_every_element() {
    let tree = TempTree::fixture("clean");
    let schema = tree.read("knowledge/schemas/issue.md");
    tree.write(
        "knowledge/schemas/issue.md",
        &schema.replace(
            "  id:\n    template: \"issue-${:seq}-${:slug}\"\n",
            "  id:\n    template: \"issue-${:seq}-${:slug}\"\n  labels:\n    list: true\n    const: bug\n    optional: true\n",
        ),
    );
    let issue = tree.read("knowledge/issues/open/issue-001-retry.md");
    tree.write(
        "knowledge/issues/open/issue-001-retry.md",
        &issue.replace("kind: bug\n", "kind: bug\nlabels: [bug, feature]\n"),
    );
    let report = tree.validate(false, CacheMode::Off);
    let messages: Vec<&str> = report.errors.iter().map(|f| f.message.as_str()).collect();
    assert_eq!(messages.len(), 1, "{}", json(&report));
    assert!(
        messages[0].contains("`labels` must be `bug`; it is `feature`"),
        "{}",
        messages[0]
    );
}

// sokf-tests: FORMAT-1_AC-3
// sokf-tests: FORMAT-1_AC-4
// sokf-tests: FORMAT-1_AC-5
// sokf-tests: FORMAT-2_AC-3
// sokf-tests: FORMAT-2_AC-6
// sokf-tests: FORMAT-2_AC-7
#[test]
fn assets_and_sidecars_are_checked() {
    let tree = TempTree::fixture("assets");
    let report = tree.validate(false, CacheMode::Off);
    insta::assert_snapshot!(json(&report));
    // FORMAT-1_AC-6: replacing the content of an asset changes no finding,
    // unless an include refers to the asset, because validation then reads it
    // like any other file.
    for rel in [
        "knowledge/media/draft/flowers.png",
        "knowledge/media/draft/sunset.png",
        "knowledge/media/draft/orphan.png",
        "knowledge/media/draft/stale.png",
    ] {
        tree.write(rel, "other bytes\n");
    }
    let again = tree.validate(false, CacheMode::Off);
    assert_eq!(again.errors, report.errors);
}

/// `readme_link_errors` writes `link` into the prose of the clean fixture's
/// readme, at line 3, validates with and without the cache, and returns the
/// errors of the readme, the same in both runs.
fn readme_link_errors(tree: &TempTree, link: &str) -> Vec<String> {
    let text = tree.read("README.md");
    let mut lines: Vec<&str> = text.split_inclusive('\n').collect();
    let prose = format!("A fixture repository for the {link}.\n");
    lines[2] = &prose;
    tree.write("README.md", &lines.concat());
    let errors = |mode| {
        let report = tree.validate(false, mode);
        error_lines(&report)
            .into_iter()
            .filter(|e| e.starts_with("/README.md:"))
            .collect::<Vec<_>>()
    };
    let off = errors(CacheMode::Off);
    assert_eq!(errors(CacheMode::On), off);
    off
}

// sokf-tests: SOKF_FORMAT_157
#[test]
fn a_directory_link_may_end_with_a_slash() {
    let tree = TempTree::fixture("clean");
    for link in ["[sources](/src/)", "[sources](/src)", "[sources](/src//)"] {
        assert_eq!(
            readme_link_errors(&tree, link),
            Vec::<String>::new(),
            "{link}"
        );
    }
    // `--fix` keeps the slash.
    tree.validate(true, CacheMode::On);
    assert!(tree.read("README.md").contains("[sources](/src//)"));
}

// sokf-tests: SOKF_FORMAT_158
#[test]
fn a_link_with_a_trailing_slash_to_a_file_fails() {
    let tree = TempTree::fixture("clean");
    let errors = readme_link_errors(&tree, "[usage](/src/main.rs/)");
    assert_eq!(
        errors.iter().map(|e| common::said(e)).collect::<Vec<_>>(),
        ["/README.md:Some(3): link `/src/main.rs/` names a file, not a directory"],
        "{errors:#?}"
    );
    // A link with the slash to a document names no document, so it gets no
    // `sokf:` label, and `--fix` leaves it as written.
    let errors = readme_link_errors(&tree, "[overview](/knowledge/overview.md/)");
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(
        errors[0].contains("link `/knowledge/overview.md/` names a file, not a directory"),
        "{errors:#?}"
    );
    tree.validate(true, CacheMode::On);
    assert!(
        tree.read("README.md")
            .contains("[overview](/knowledge/overview.md/)")
    );
}

// sokf-tests: SOKF_FORMAT_159
#[test]
fn an_invalid_repository_path_says_so() {
    let tree = TempTree::fixture("clean");
    for link in ["/src/./main.rs", "/src//main.rs", "/src/../src/", "/src/./"] {
        let errors = readme_link_errors(&tree, &format!("[usage]({link})"));
        assert_eq!(
            errors.iter().map(|e| common::said(e)).collect::<Vec<_>>(),
            [format!(
                "/README.md:Some(3): link `{link}` is not a valid repository path"
            )],
            "{errors:#?}"
        );
    }
}

// sokf-tests: SOKF_SCHEMA_32
#[test]
fn two_documents_of_a_type_with_one_number_are_both_reported() {
    // Two branches that each created an issue merged into one tree.
    let retry = "knowledge/issues/open/issue-001-retry.md";
    let tree = TempTree::fixture("clean");
    tree.write(
        "knowledge/issues/open/issue-001-other.md",
        &tree
            .read(retry)
            .replace("issue-001-retry", "issue-001-other")
            .replace("Retry fails twice", "Other"),
    );
    let report = tree.validate(false, CacheMode::Off);
    let clashes: Vec<String> = report
        .errors
        .iter()
        .filter(|f| f.criterion == "SOKF_SCHEMA_32")
        .map(|f| format!("{}: {}", f.path, f.message))
        .collect();
    assert_eq!(
        clashes,
        [
            "/knowledge/issues/open/issue-001-other.md: /knowledge/issues/open/issue-001-retry.md also uses sequence number 1",
            "/knowledge/issues/open/issue-001-retry.md: /knowledge/issues/open/issue-001-other.md also uses sequence number 1",
        ],
        "{}",
        json(&report)
    );
}
