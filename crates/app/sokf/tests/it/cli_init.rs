//! These tests run `sokf init` through the binary, end to end. They check the
//! text and JSON that `init` prints, the stderr message and exit code 2 of
//! each refusal, and that `validate` passes on the new store.

use std::fs;

use common::{Fixture, git, run_in};

use crate::common;

/// `empty` returns a fixture that holds an empty repository in a temporary
/// directory.
fn empty() -> Fixture {
    let f = Fixture::new("manifest");
    fs::remove_dir_all(f.0.join("knowledge")).unwrap();
    f
}

// sokf-tests: CLI-2_AC-11
#[test]
fn init_writes_the_store_and_reports_each_file() {
    let f = empty();
    let (stdout, stderr, code) = f.run(&["init", "--scope", "test"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        stdout,
        "created /knowledge/index.md\n\
         created /knowledge/manifest.sokf.yaml\n\
         created /knowledge/schemas/index.md\n"
    );
    assert_eq!(stderr, "");
    let (stdout, _, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(stdout, "0 errors, 0 warnings\n");
    // `init` accepts `--no-cache`, which changes nothing, and refuses because
    // the store exists.
    let (_, stderr, code) = f.run(&["--no-cache", "init", "--scope", "test"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: init: /knowledge is already a knowledge store: it holds manifest.sokf.yaml\n"
    );
}

// sokf-tests: CLI-2_AC-11
// sokf-tests: CLI-2_AC-12
#[test]
fn init_in_another_directory_registers_it_and_prints_json() {
    let f = empty();
    let (stdout, _, code) = f.run_here(&[
        "init",
        "docs/kb",
        "--name",
        "project-kb",
        "--scope",
        "test",
        "--description",
        "Knowledge for the project.",
        "--json",
    ]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        stdout,
        "{\"root\":\"/docs/kb\",\"created\":[\"/.sokf/config.toml\",\"/docs/kb/index.md\",\"/docs/kb/manifest.sokf.yaml\",\"/docs/kb/schemas/index.md\"],\"updated\":[],\"hand_written_indexes\":[]}\n"
    );
    assert_eq!(
        fs::read_to_string(f.0.join("docs/kb/manifest.sokf.yaml")).unwrap(),
        "sokf: \"0.6\"\nname: project-kb\nscope: test\ndescription: Knowledge for the project.\n"
    );
    // `init` for a second store edits the config and reports it as `updated`.
    let (stdout, _, code) = f.run_here(&["init", "/knowledge", "--scope", "test"]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        stdout,
        "created /knowledge/index.md\n\
         created /knowledge/manifest.sokf.yaml\n\
         created /knowledge/schemas/index.md\n\
         updated /.sokf/config.toml\n"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".sokf/config.toml")).unwrap(),
        "[knowledge]\nroots = [\"/docs/kb\", \"/knowledge\"]\n"
    );
    let (stdout, _, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{stdout}");
}

// sokf-tests: CLI-2_AC-13
// sokf-tests: CLI-2_AC-14
// sokf-tests: CLI-2_AC-15
// sokf-tests: CLI-2_AC-18
// sokf-tests: CLI-2_AC-19
// sokf-tests: CLI-2_AC-33
#[test]
fn every_refusal_exits_2_on_stderr_and_writes_nothing() {
    let f = empty();
    f.write("file.md", "");
    fs::create_dir_all(f.0.join("empty")).unwrap();
    for (args, message) in [
        (
            vec!["init", "../outside"],
            "error: init: ../outside lies outside the repository\n",
        ),
        (
            vec!["init", "file.md"],
            "error: init: /file.md is a file, not a directory\n",
        ),
        (
            vec!["init", "."],
            "error: init: / is the repository root, and a store there would hold every file of the repository. Pass a directory under the root.\n",
        ),
        (
            vec!["init", "empty", "--name", ""],
            "error: init: --name is empty\n",
        ),
        (
            vec!["init", "empty", "--name", "Project KB"],
            "error: init: --name is not a slug. A slug is lower-case letters and digits, joined by single hyphens. Pass --name with a slug.\n",
        ),
        (
            vec!["init", "empty"],
            "error: init: you passed no --scope, and the URL of the git remote `origin` has no owner to read. Pass --scope with the slug of the store's owner.\n",
        ),
    ] {
        let (stdout, stderr, code) = f.run_here(&args);
        assert_eq!(code, 2, "{args:?}: {stdout}{stderr}");
        assert_eq!(stderr, message, "{args:?}");
        assert_eq!(stdout, "");
    }
    f.write(".sokf/config.toml", "not toml [");
    let (_, stderr, code) = f.run_here(&["init", "empty", "--scope", "test"]);
    assert_eq!(code, 2);
    assert!(
        stderr.starts_with("error: init: /.sokf/config.toml: "),
        "{stderr}"
    );
    assert!(!f.0.join("empty/manifest.sokf.yaml").exists());
    assert!(!f.0.join("knowledge").exists());
    // Outside a repository, `init` exits with code 2 before it reads anything.
    let (_, stderr, code) = run_in(&std::env::temp_dir(), &["init"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("--repo-root"), "{stderr}");
}

/// `with_origin` returns an empty repository that is a git repository whose
/// remote `origin` has the URL `url`. The test needs `git` on the path.
fn with_origin(url: &str) -> Fixture {
    let f = empty();
    git(&f.0, &["init", "-q"]);
    git(&f.0, &["remote", "add", "origin", url]);
    f
}

// sokf-tests: CLI-2_AC-17
#[test]
fn init_without_scope_reads_the_owner_of_origin_in_lower_case() {
    let f = with_origin("https://github.com/NCoderz/sokf.git");
    let (stdout, stderr, code) = f.run(&["init"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        fs::read_to_string(f.0.join("knowledge/manifest.sokf.yaml")).unwrap(),
        "sokf: \"0.6\"\nname: knowledge\nscope: ncoderz\n"
    );
}

// sokf-tests: CLI-1_AC-19
#[test]
fn init_reads_the_origin_of_the_root_whatever_git_dir_names() {
    let a = with_origin("https://github.com/alpha/a.git");
    let b = with_origin("https://github.com/beta/b.git");
    // A commit hook of `b` exports its `GIT_DIR` to each program that it
    // runs, and only the binary under test inherits it here.
    let (stdout, stderr, code) = common::output(
        common::sokf()
            .env("GIT_DIR", b.0.join(".git"))
            .arg("--repo-root")
            .arg(a.root())
            .arg("init")
            .current_dir(&a.0),
    );
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        fs::read_to_string(a.0.join("knowledge/manifest.sokf.yaml")).unwrap(),
        "sokf: \"0.6\"\nname: knowledge\nscope: alpha\n"
    );
}

// sokf-tests: CLI-2_AC-18
// sokf-tests: CLI-2_AC-20
#[test]
fn a_scope_not_a_slug_or_a_taken_identity_exits_2() {
    let f = empty();
    let (stdout, stderr, code) = f.run(&["init", "/docs/a", "--scope", "test"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    for (args, message) in [
        (
            vec!["init", "/docs/b", "--scope", "Acme Inc"],
            "error: init: --scope `Acme Inc` is not a slug. A slug is lower-case letters and digits, joined by single hyphens.\n",
        ),
        (
            vec!["init", "/docs/b", "--name", "a", "--scope", "test"],
            "error: init: the store at /docs/a already has the identity `test/a`. Pass --name with another slug.\n",
        ),
    ] {
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {stdout}{stderr}");
        assert_eq!(stderr, message, "{args:?}");
        assert_eq!(stdout, "");
    }
    assert!(!f.0.join("docs/b").exists());
}

// sokf-tests: CLI-2_AC-30
// sokf-tests: CLI-2_AC-31
// sokf-tests: CLI-2_AC-32
// sokf-tests: CLI-4_AC-35
// sokf-tests: CONF-3_AC-8
#[test]
fn init_over_documents_writes_the_manifest_and_warns_of_a_hand_written_index() {
    let f = Fixture::new("adopt");
    let (stdout, stderr, code) = f.run(&["init", "/analysis", "--scope", "test"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        stdout,
        "created /.sokf/config.toml\n\
         created /analysis/manifest.sokf.yaml\n"
    );
    assert_eq!(
        stderr,
        "warning: init: /analysis/chuckie-egg/index.md is not a generated index, so `sokf validate --fix` doesn't overwrite it: move its content into a document\n"
    );
    let page = fs::read_to_string(f.0.join("analysis/chuckie-egg/index.md")).unwrap();
    let (stdout, _, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 1, "{stdout}");
    assert_eq!(
        stdout,
        "created /analysis/index.md\n\
         rewrote /analysis/old/index.md\n\
         /analysis/a.md: error: the document has no frontmatter (SOKF_FORMAT_17)\n\
         /analysis/chuckie-egg/b.md: error: the document has no frontmatter (SOKF_FORMAT_17)\n\
         /analysis/chuckie-egg/index.md: error: the file is not a generated index, so `--fix` doesn't overwrite it: move its content into a document, then run `--fix` (SOKF_FORMAT_97)\n\
         /analysis/old/c.md: error: the document has no frontmatter (SOKF_FORMAT_17)\n\
         \n\
         4 errors, 0 warnings\n"
    );
    assert_eq!(
        fs::read_to_string(f.0.join("analysis/chuckie-egg/index.md")).unwrap(),
        page
    );
    // A second `init` refuses the store that the first one made.
    let (_, stderr, code) = f.run(&["init", "/analysis", "--scope", "test"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: init: /analysis is already a knowledge store: it holds manifest.sokf.yaml\n"
    );
    // An exclude under the root has no effect, and `validate` warns of it.
    let config = fs::read_to_string(f.0.join(".sokf/config.toml")).unwrap();
    f.write(
        ".sokf/config.toml",
        &format!("{config}\n[repository]\nexclude = [\"/analysis/chuckie-egg/**\"]\n"),
    );
    let (stdout, _, _) = f.run(&["validate", "--warnings"]);
    assert!(
        stdout.contains("/.sokf/config.toml: warning: the `repository.exclude` glob `/analysis/chuckie-egg/**` lies under the knowledge root /analysis, so it has no effect: sokf checks every file under a root (CONF-3_AC-8)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("/analysis/chuckie-egg/b.md: error:"),
        "{stdout}"
    );
}

// The files of the sokf pack, copied into a repository that `sokf init`
// created, validate with no error, because the pack carries every format and
// type that its schemas name in its file of `/.sokf/formats.d/`, and the new
// repository has no `/.sokf/formats.yaml` to lend one.
// sokf-tests: CLI-10_AC-5
#[test]
fn the_sokf_pack_validates_in_a_repository_that_init_created() {
    let f = empty();
    let (out, err, code) = f.run(&["init", "--scope", "example"]);
    assert_eq!(code, 0, "{out}{err}");
    crate::common::take_method(&f);
    assert!(
        f.path(".sokf/formats.d/pakf/@sokf/sokf/formats.yaml")
            .is_file()
    );
    let (out, err, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!f.path(".sokf/formats.yaml").exists());
    // A second run fixes nothing and finds nothing.
    let (out, err, code) = f.run(&["validate", "--fix"]);
    assert_eq!((code, out.as_str()), (0, "0 errors, 0 warnings\n"), "{err}");
}
