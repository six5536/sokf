//! These tests run `init` through the core on temporary repositories. Each
//! test runs one scenario of CLI-2 and checks the tree and the result. The
//! refusal test also checks that a refusal leaves every byte of the tree
//! unchanged.

use std::path::PathBuf;

use common::TempTree;
use sokf_core::{CacheMode, Error, InitOptions, InitResult, RepoPath, init, load_config};

use crate::common;

fn p(s: &str) -> RepoPath {
    RepoPath::parse(s).unwrap()
}

fn run(
    tree: &TempTree,
    dir: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
) -> sokf_core::Result<InitResult> {
    init(
        &tree.root(),
        &InitOptions {
            dir: dir.map(PathBuf::from),
            cwd: tree.dir().to_path_buf(),
            name: name.map(str::to_string),
            scope: Some("test".into()),
            description: description.map(str::to_string),
        },
    )
}

/// `assert_clean` checks that the store validates with no errors or warnings,
/// and that `--fix` plans no fixes.
fn assert_clean(tree: &TempTree) {
    let report = tree.validate(false, CacheMode::Off);
    assert!(
        report.errors.is_empty() && report.warnings.is_empty(),
        "{report:?}"
    );
    let fixed = tree.validate(true, CacheMode::Off);
    assert!(fixed.fixes.is_empty(), "{fixed:?}");
}

// sokf-tests: CLI-2_AC-1
// sokf-tests: CLI-2_AC-3
// sokf-tests: CLI-2_AC-5
// sokf-tests: CLI-2_AC-6
// sokf-tests: CLI-2_AC-7
// sokf-tests: CLI-2_AC-8
// sokf-tests: CLI-2_AC-11
// sokf-tests: CLI-2_AC-22
// sokf-tests: CLI-2_AC-24
// sokf-tests: CLI_P-21
#[test]
fn an_empty_repository_gets_the_default_store_and_no_config() {
    let tree = TempTree::empty("init-default");
    let out = run(&tree, None, None, None).unwrap();
    assert_eq!(
        out,
        InitResult {
            root: p("/knowledge"),
            created: vec![
                p("/knowledge/index.md"),
                p("/knowledge/manifest.sokf.yaml"),
                p("/knowledge/schemas/index.md")
            ],
            updated: vec![],
            hand_written_indexes: vec![],
        }
    );
    assert_eq!(
        tree.read("knowledge/manifest.sokf.yaml"),
        "sokf: \"0.6\"\nname: knowledge\nscope: test\n"
    );
    assert_eq!(
        tree.read("knowledge/index.md"),
        "# knowledge\n\n- [schemas/](schemas/index.md)\n"
    );
    assert_eq!(tree.read("knowledge/schemas/index.md"), "# schemas\n");
    assert!(!tree.dir().join(".sokf").exists());
    assert_eq!(
        load_config(&tree.root()).unwrap().roots,
        vec![p("/knowledge")]
    );
    assert_clean(&tree);
}

// sokf-tests: CLI-2_AC-2
// sokf-tests: CLI-2_AC-4
// sokf-tests: CLI-2_AC-9
// sokf-tests: CLI-2_AC-16
// sokf-tests: CLI-2_AC-11
// sokf-tests: CLI-2_AC-21
// sokf-tests: CLI_P-21
#[test]
fn a_named_directory_with_no_config_writes_one() {
    let tree = TempTree::empty("init-named");
    let out = run(
        &tree,
        Some("docs/kb"),
        Some("project-kb"),
        Some("Knowledge for the project."),
    )
    .unwrap();
    assert_eq!(out.root, p("/docs/kb"));
    assert_eq!(
        out.created,
        vec![
            p("/.sokf/config.toml"),
            p("/docs/kb/index.md"),
            p("/docs/kb/manifest.sokf.yaml"),
            p("/docs/kb/schemas/index.md")
        ]
    );
    assert_eq!(out.updated, [] as [sokf_core::RepoPath; 0]);
    assert_eq!(
        tree.read("docs/kb/manifest.sokf.yaml"),
        "sokf: \"0.6\"\nname: project-kb\nscope: test\ndescription: Knowledge for the project.\n"
    );
    // The default root does not exist, so `init` leaves it out of the config.
    assert_eq!(
        tree.read(".sokf/config.toml"),
        "[knowledge]\nroots = [\"/docs/kb\"]\n"
    );
    assert_eq!(
        load_config(&tree.root()).unwrap().roots,
        vec![p("/docs/kb")]
    );
    assert_clean(&tree);
}

// sokf-tests: CLI-2_AC-9
// sokf-tests: CLI-2_AC-21
// sokf-tests: CLI_P-21
#[test]
fn a_second_store_beside_the_default_keeps_it() {
    let tree = TempTree::empty("init-second");
    run(&tree, None, None, None).unwrap();
    let out = run(&tree, Some("docs/kb"), None, None).unwrap();
    assert_eq!(out.created[0], p("/.sokf/config.toml"));
    assert_eq!(
        tree.read(".sokf/config.toml"),
        "[knowledge]\nroots = [\"/knowledge\", \"/docs/kb\"]\n"
    );
    assert_eq!(
        load_config(&tree.root()).unwrap().roots,
        vec![p("/knowledge"), p("/docs/kb")]
    );
    assert_clean(&tree);
}

// sokf-tests: CLI-2_AC-9
// sokf-tests: CLI-2_AC-11
// sokf-tests: CONF-5_AC-1
// sokf-tests: CLI_P-22
#[test]
fn a_commented_config_is_edited_in_place() {
    let tree = TempTree::empty("init-commented");
    tree.write("knowledge/.keep", "");
    tree.write(
        ".sokf/config.toml",
        "# Stores.\n[knowledge]\nroots = [\"/knowledge\"] # the default\n\n[repository]\nexclude = [\"/vendor/**\"]\n",
    );
    // `knowledge/` is a root without a manifest, and the store goes in a
    // second directory.
    let out = run(&tree, Some("/docs/kb/"), None, None).unwrap();
    assert_eq!(out.root, p("/docs/kb"));
    assert_eq!(out.updated, vec![p("/.sokf/config.toml")]);
    assert!(out.created.iter().all(|c| c.starts_with(&p("/docs/kb"))));
    assert_eq!(
        tree.read(".sokf/config.toml"),
        "# Stores.\n[knowledge]\nroots = [\"/knowledge\", \"/docs/kb\"] # the default\n\n[repository]\nexclude = [\"/vendor/**\"]\n"
    );
    assert_eq!(
        load_config(&tree.root()).unwrap().roots,
        vec![p("/knowledge"), p("/docs/kb")]
    );
}

// sokf-tests: CLI-2_AC-25
#[test]
fn a_directory_the_config_lists_already_leaves_it() {
    let tree = TempTree::empty("init-listed");
    tree.write(".sokf/config.toml", "[knowledge]\nroots = [\"/docs/kb\"]\n");
    let out = run(&tree, Some("docs/kb"), None, None).unwrap();
    assert_eq!(out.updated, [] as [sokf_core::RepoPath; 0]);
    assert_eq!(
        tree.read(".sokf/config.toml"),
        "[knowledge]\nroots = [\"/docs/kb\"]\n"
    );
    assert_clean(&tree);
}

// sokf-tests: CLI-2_AC-9
// sokf-tests: CONF-2_AC-4
#[test]
fn a_nested_store_is_registered_as_any_other() {
    let tree = TempTree::empty("init-nested");
    run(&tree, None, None, None).unwrap();
    let out = run(&tree, Some("knowledge/inner"), None, None).unwrap();
    assert_eq!(out.root, p("/knowledge/inner"));
    assert_eq!(
        load_config(&tree.root()).unwrap().roots,
        vec![p("/knowledge"), p("/knowledge/inner")]
    );
    // The outer index doesn't list the inner root until `--fix` adds the link.
    let report = tree.validate(false, CacheMode::Off);
    assert_eq!(report.errors.len(), 1, "{report:?}");
    assert_eq!(report.errors[0].path, p("/knowledge/index.md"));
    let fixed = tree.validate(true, CacheMode::Off);
    assert!(fixed.errors.is_empty(), "{fixed:?}");
    assert!(
        tree.read("knowledge/index.md")
            .contains("- [inner/](inner/index.md)\n")
    );
}

// sokf-tests: CLI-9_AC-1
// sokf-tests: CLI-2_AC-13
// sokf-tests: CLI-2_AC-14
// sokf-tests: CLI-2_AC-15
// sokf-tests: CLI-2_AC-30
// sokf-tests: CLI-2_AC-33
#[test]
fn every_refusal_writes_nothing() {
    let tree = TempTree::empty("init-refused");
    tree.write("file.md", "");
    tree.write(
        "store/manifest.sokf.yaml",
        "sokf: \"0.6\"\nname: s\nscope: test\n",
    );
    tree.write("empty/.keep", "");
    std::fs::remove_file(tree.dir().join("empty/.keep")).unwrap();
    let cases: Vec<(Option<&str>, Option<&str>, &str)> = vec![
        (Some("../outside"), None, "lies outside the repository"),
        (Some("file.md"), None, "is a file"),
        (Some("."), None, "is the repository root"),
        (Some("store"), None, "is already a knowledge store"),
        (Some("empty"), Some(""), "--name is empty"),
    ];
    for (dir, name, expected) in cases {
        let before = tree.files();
        let e = run(&tree, dir, name, None).unwrap_err();
        assert!(matches!(e, Error::Init(_)), "{dir:?}: {e}");
        assert!(e.to_string().contains(expected), "{dir:?}: {e}");
        assert_eq!(tree.files(), before, "{dir:?} wrote something");
    }
    tree.write(".sokf/config.toml", "not toml [");
    let before = tree.files();
    let e = run(&tree, Some("empty"), None, None).unwrap_err();
    assert!(
        e.to_string().starts_with("init: /.sokf/config.toml: "),
        "{e}"
    );
    assert_eq!(tree.files(), before);
    assert!(!tree.dir().join("empty/manifest.sokf.yaml").exists());
}

// sokf-tests: CLI-2_AC-10
#[test]
fn init_is_blind_to_the_knowledge_and_the_cache() {
    let tree = TempTree::empty("init-blind");
    run(&tree, None, None, None).unwrap();
    // Write a document that doesn't validate. Write a cache file with invalid contents.
    tree.write("knowledge/bad.md", "---\ntype: Nope\n---\n\n# Bad\n");
    tree.write(".sokf/cache/store", "not a cache\n");
    let cache = tree.cache_files();
    let out = run(&tree, Some("docs/kb"), None, None).unwrap();
    assert_eq!(out.root, p("/docs/kb"));
    // The broken document doesn't stop `init`, and `init` leaves the cache as it
    // was.
    assert_eq!(
        tree.read("knowledge/bad.md"),
        "---\ntype: Nope\n---\n\n# Bad\n"
    );
    assert_eq!(tree.cache_files(), cache);
}

// sokf-tests: CLI-2_AC-6
// sokf-tests: CLI-2_AC-31
// sokf-tests: CLI-2_AC-32
// sokf-tests: SOKF_VALIDATION_56
// sokf-tests: CLI-4_AC-35
#[test]
fn a_directory_of_documents_gets_a_manifest_and_keeps_every_file() {
    let tree = TempTree::fixture("adopt");
    let before = tree.files();
    let out = run(&tree, Some("analysis"), None, None).unwrap();
    assert_eq!(
        out,
        InitResult {
            root: p("/analysis"),
            created: vec![p("/.sokf/config.toml"), p("/analysis/manifest.sokf.yaml")],
            updated: vec![],
            hand_written_indexes: vec![p("/analysis/chuckie-egg/index.md")],
        }
    );
    // `init` writes no index, and changes no file that was there.
    let after: Vec<(String, String)> = tree
        .files()
        .into_iter()
        .filter(|(rel, _)| rel != ".sokf/config.toml" && rel != "analysis/manifest.sokf.yaml")
        .collect();
    assert_eq!(after, before);
    assert_eq!(
        load_config(&tree.root()).unwrap().roots,
        vec![p("/analysis")]
    );
    // `validate` lists the documents to convert and the indexes to write.
    // `--fix` writes the missing index and rewrites the stale one, and leaves
    // the hand-written one as it was.
    let report = tree.validate(false, CacheMode::Off);
    let errors: Vec<String> = report
        .errors
        .iter()
        .map(|f| format!("{}: {}", f.path, sokf_core::report_message(f)))
        .collect();
    assert!(
        errors.contains(&"/analysis/chuckie-egg/index.md: the file is not a generated index, so `--fix` doesn't overwrite it: move its content into a document, then run `--fix` (SOKF_FORMAT_97)".to_string()),
        "{errors:?}"
    );
    let hand_written = tree.read("analysis/chuckie-egg/index.md");
    let fixed = tree.validate(true, CacheMode::Off);
    let fixes: Vec<String> = fixed
        .fixes
        .iter()
        .map(|f| serde_json::to_string(f).unwrap())
        .collect();
    assert_eq!(
        fixes,
        vec![
            r#"{"op":"created","path":"/analysis/index.md"}"#,
            r#"{"op":"rewrote","path":"/analysis/old/index.md"}"#,
        ]
    );
    assert_eq!(tree.read("analysis/chuckie-egg/index.md"), hand_written);
    assert_eq!(tree.read("analysis/old/index.md"), "# old\n\n- [c](c.md)\n");
    assert!(
        fixed
            .errors
            .iter()
            .any(|f| f.path == p("/analysis/chuckie-egg/index.md")),
        "{fixed:?}"
    );
    let again = tree.validate(true, CacheMode::Off);
    assert!(again.fixes.is_empty(), "{again:?}");
}
