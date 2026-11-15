//! These tests cover the configuration file `/.sokf/config.toml`
//! (contract-config): the roots that it lists, and the trees that sokf
//! leaves out when it walks the repository.

use common::{TempTree, said};
use sokf_core::CacheMode;

use crate::common;

/// `LFS_POINTER` is the text of the Git LFS pointer that the tests write. A
/// clone holds such a pointer file in place of an object that Git has not
/// fetched.
const LFS_POINTER: &str = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12345\n";

/// `clean_with_links` returns the `clean` fixture, with a sentence that
/// cites `links` added to the context paragraph of its decision.
fn clean_with_links(links: &str) -> TempTree {
    let tree = TempTree::fixture("clean");
    let path = "knowledge/decisions/decision-single-binary.md";
    let text = tree.read(path).replace(
        "users installed the wrong one.",
        &format!("users installed the wrong one. See {links}."),
    );
    tree.write(path, &text);
    tree
}

/// `errors_on` validates the tree and returns the message of every error on
/// `path`. It drops the criterion key in parentheses that a message ends
/// with.
fn errors_on(tree: &TempTree, path: &str) -> Vec<String> {
    tree.validate(false, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| f.path.as_str() == path)
        .map(|f| said(&f.message))
        .collect()
}

// A schema in each of two roots matches `/README.md`, so the file matches two
// schemas, and validation reports the error on each of the two schemas.
// sokf-tests: CONF-2_AC-5
#[test]
fn schemas_of_two_roots_that_match_one_file_are_each_an_error() {
    let tree = TempTree::fixture("clean");
    tree.add_other_store();
    tree.write(
        "other/schemas/readme.md",
        "---\ntype: Schema\nid: schema-other-readme\ndescription: The readme again.\n---\n\n````yaml\nline-limit: 0\nadditional-files: /README.md\nsections: []\n````\n",
    );
    let expected = "/README.md matches the schemas schema-other-readme and schema-readme, but a file can match only one schema";
    for schema in ["/knowledge/schemas/readme.md", "/other/schemas/readme.md"] {
        let errors = errors_on(&tree, schema);
        assert!(
            errors.iter().any(|m| m == expected),
            "{schema}: {errors:#?}"
        );
    }
}

// A path link into a tree that `repository.exclude` lists resolves if its
// file exists, and a link to a file that doesn't exist is an error.
// sokf-tests: CONF-3_AC-3
#[test]
fn a_path_link_resolves_into_an_excluded_tree() {
    let links = "[the notes](/vendor/notes.md) and [nothing](/vendor/gone.md)";
    let tree = clean_with_links(links);
    tree.write(
        ".sokf/config.toml",
        "[repository]\nexclude = [\"/vendor/**\"]\n",
    );
    tree.write("vendor/notes.md", "# Notes\n");
    let errors = errors_on(&tree, "/knowledge/decisions/decision-single-binary.md");
    assert!(
        !errors.iter().any(|m| m.contains("/vendor/notes.md")),
        "{errors:#?}"
    );
    assert!(
        errors.iter().any(|m| m.contains("/vendor/gone.md")),
        "{errors:#?}"
    );
}

// A Git LFS pointer stands in place of an object that Git has not fetched.
// A path link to the pointer resolves, as a link to any file that exists
// does.
// sokf-tests: CONF-3_AC-4
#[test]
fn a_git_lfs_pointer_is_a_file_that_exists() {
    let tree = clean_with_links("[the photo](/media/photo.png)");
    tree.write(
        ".gitattributes",
        "*.png filter=lfs diff=lfs merge=lfs -text\n",
    );
    tree.write("media/photo.png", LFS_POINTER);
    let errors = errors_on(&tree, "/knowledge/decisions/decision-single-binary.md");
    assert!(errors.is_empty(), "{errors:#?}");
    // After the test removes the file, the same link is an error.
    std::fs::remove_file(tree.dir().join("media/photo.png")).unwrap();
    let errors = errors_on(&tree, "/knowledge/decisions/decision-single-binary.md");
    assert!(
        errors.iter().any(|m| m.contains("/media/photo.png")),
        "{errors:#?}"
    );
}

/// `read` returns, as one string, the configuration that `load_config` reads.
/// When `load_config` fails, `read` returns its error.
fn read(tree: &TempTree) -> String {
    match sokf_core::load_config(&tree.root()) {
        Ok(config) => format!("{config:?}"),
        Err(e) => format!("error: {e}"),
    }
}

// For every fixture, both with and without its `/knowledge` directory, sokf
// reads an absent `/.sokf/config.toml` the same as an empty one: it returns the
// same configuration for both, or fails on both with the same error.
// sokf-tests: CONF_P-1
// sokf-tests: CONF-1_AC-1
#[test]
fn an_absent_configuration_reads_as_one_with_no_tables() {
    let fixtures = [
        "assets",
        "broken",
        "clean",
        "fixable",
        "graph",
        "harness",
        "indexes",
        "manifest",
        "new",
        "no-manifest",
        "two-roots",
    ];
    for name in fixtures {
        let tree = TempTree::fixture(name);
        for without_knowledge in [false, true] {
            if without_knowledge {
                let _ = std::fs::remove_dir_all(tree.dir().join("knowledge"));
            }
            let _ = std::fs::remove_file(tree.dir().join(".sokf/config.toml"));
            let absent = read(&tree);
            tree.write(".sokf/config.toml", "");
            assert_eq!(
                read(&tree),
                absent,
                "{name}, without knowledge: {without_knowledge}"
            );
        }
    }
}
