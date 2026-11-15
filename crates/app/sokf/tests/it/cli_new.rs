//! These tests run `sokf new` from end to end and check the document, the
//! `created` line, the id, the JSON object, and the exit code of every
//! refusal.

use common::{Fixture, git, run};

use crate::common;

// sokf-tests: CLI-6_AC-10
// sokf-tests: CLI-6_AC-11
// sokf-tests: CLI-6_AC-12
// sokf-tests: CLI-6_AC-13
#[test]
fn the_document_the_created_line_the_id_and_the_json() {
    let f = Fixture::new("new");
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "x"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.starts_with("---\ntype: Issue\nid: issue-002-x\ntitle: X\nstate: open\nkind: bug\n"),
        "{stdout}"
    );
    assert!(
        stdout.ends_with("1. Move a document with `--fix`.\n"),
        "{stdout}"
    );
    let (stdout, _, code) = f.run(&["new", "Issue", "--slug", "x", "--id-only"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "issue-002-x\n");
    let (stdout, _, code) = f.run(&["new", "Issue", "--slug", "x", "--json", "--no-cache"]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["id"], "issue-002-x");
    assert_eq!(value["path"], "/knowledge/issues/open/bug/issue-002-x.md");
    assert!(value["document"].as_str().unwrap().starts_with("---\n"));
    assert!(value.get("created").is_none());
    let (stdout, _, code) = f.run(&[
        "new",
        "Issue",
        "--slug",
        "x",
        "--state",
        "kind=feature",
        "--write",
    ]);
    assert_eq!(code, 0);
    // The index of each directory that holds the document lists it.
    assert_eq!(
        stdout,
        "created /knowledge/issues/open/feature/issue-002-x.md\n\
         updated /knowledge/issues/open/feature/index.md\n\
         updated /knowledge/issues/open/index.md\n"
    );
    assert!(
        f.0.join("knowledge/issues/open/feature/issue-002-x.md")
            .exists()
    );
    let (stdout, _, code) = f.run(&["new", "Schema", "--id", "schema-task", "--write"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "created /knowledge/schemas/schema-task.md\nupdated /knowledge/schemas/index.md\n"
    );
}

// sokf-tests: CLI-6_AC-18
// sokf-tests: CLI-6_AC-19
// sokf-tests: CLI-6_AC-20
#[test]
fn every_refusal_exits_2_and_writes_nothing() {
    let f = Fixture::new("new");
    let before = f.files();
    for (args, needle) in [
        (
            vec!["new", "Nope", "--id", "x"],
            "error: new: `Nope` has no schema",
        ),
        (vec!["new", "Issue"], "`--slug` is required"),
        (
            vec!["new", "Issue", "--slug", "x", "--state", "kind=huge"],
            "not a state of `kind`",
        ),
        (
            vec!["new", "Note", "--id", "note-two-words"],
            "so give the title with `--title`, such as `--title \"Groceries\"`",
        ),
        (
            vec!["new", "Note", "--id", "n", "--title", "Two words"],
            "doesn't match the schema",
        ),
        (
            vec!["new", "Note", "--id", "n", "--dir", "/src"],
            "outside the knowledge",
        ),
        (
            vec!["new", "Image", "--id", "x"],
            "so give `--for <PATH>` with the asset",
        ),
        (
            vec![
                "new",
                "Image",
                "--id",
                "x",
                "--for",
                "/knowledge/media/published/moved.png",
            ],
            "but the requested states put sidecars in",
        ),
        (
            vec!["new", "Note", "--id", "n", "--id-only"],
            "so give the id with `--id`",
        ),
    ] {
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {stderr}");
        assert!(stdout.is_empty(), "{args:?}: {stdout}");
        assert!(stderr.starts_with("error: new: "), "{args:?}: {stderr}");
        assert!(stderr.contains(needle), "{args:?}: {stderr}");
    }
    // Giving both options of an exclusive pair, or a `--state` value with no
    // `=`, is a usage error.
    for args in [
        vec!["new", "Issue", "--slug", "a", "--id", "b"],
        vec!["new", "Note", "--id", "n", "--dir", "a", "--for", "b"],
        vec!["new", "Issue", "--slug", "a", "--write", "--id-only"],
        vec!["new", "Issue", "--slug", "a", "--json", "--id-only"],
        vec!["new", "Issue", "--slug", "a", "--state", "nokey"],
    ] {
        let (_, stderr, code) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {stderr}");
    }
    assert_eq!(f.files(), before);
    // The first write succeeds. `sokf new` refuses a second write to the same
    // file.
    let write = [
        "--repo-root",
        f.root(),
        "new",
        "Note",
        "--id",
        "note-twice",
        "--title",
        "Twice",
        "--write",
    ];
    assert_eq!(run(&write).2, 0);
    let (_, stderr, code) = run(&write);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("exists, so sokf writes nothing"),
        "{stderr}"
    );
}

// sokf-tests: CLI-6_AC-9
#[test]
fn write_puts_lf_text_at_the_path_the_states_derive() {
    let f = Fixture::new("new");
    let (stdout, stderr, code) = f.run(&[
        "new",
        "Issue",
        "--slug",
        "lf",
        "--state",
        "state=closed",
        "--write",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout.lines().next(),
        Some("created /knowledge/issues/closed/bug/issue-002-lf.md")
    );
    let bytes = std::fs::read(f.0.join("knowledge/issues/closed/bug/issue-002-lf.md")).unwrap();
    assert_ne!(bytes, [] as [u8; 0]);
    assert!(
        !bytes.contains(&b'\r'),
        "the file has a carriage return (CR) byte"
    );
    assert!(bytes.ends_with(b"\n"));
}

/// `refused` runs `sokf new` with `args` on `f`, checks that the command
/// exits with 2, prints nothing on stdout, and writes nothing, then returns
/// the command's stderr.
fn refused(f: &Fixture, args: &[&str]) -> String {
    let before = f.files();
    let mut full = vec!["--repo-root", f.root(), "new"];
    full.extend_from_slice(args);
    let (stdout, stderr, code) = run(&full);
    assert_eq!(code, 2, "{args:?}: {stderr}");
    assert!(stdout.is_empty(), "{args:?}: {stdout}");
    assert_eq!(f.files(), before, "{args:?}");
    stderr
}

// sokf-tests: CLI-6_AC-16
#[test]
fn a_schema_that_derives_no_scaffold_exits_2_with_one_line_per_rule() {
    let f = Fixture::new("new");
    f.write(
        "knowledge/schemas/cascade.md",
        "---\ntype: Schema\nid: schema-cascade\ntitle: Cascade Schema\ndescription: Two rules short of a scaffold.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Cascade\n  id:\n    template: \"cascade-${:slug}\"\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n  - heading: Summary\n    level: 2\n    guidance: What the reader takes away.\n  - heading: Notes\n    level: 2\n    blocks:\n      - kind: bullet-list\n````\n",
    );
    let stderr = refused(&f, &["Cascade", "--slug", "thing", "--write"]);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(
        lines[0], "error: new: the schema of `Cascade` derives no scaffold:",
        "{stderr}"
    );
    // `stderr` has one line for each rule that stops the scaffold, and each
    // line gives that rule's line number.
    let at: Vec<&str> = lines
        .iter()
        .filter_map(|l| l.strip_prefix("  /knowledge/schemas/cascade.md:"))
        .collect();
    assert_eq!(at.len(), 2, "{stderr}");
    for rest in at {
        let line: usize = rest.split(':').next().unwrap().parse().unwrap();
        assert!(line > 8, "{stderr}");
    }
}

/// `two_roots` adds the store `other`, in `/other`, to the fixture `new`,
/// with copies of the Issue and Note schemas, so that both stores declare
/// both types. The first store is named `new-fixture`.
fn two_roots(f: &Fixture) {
    f.write(
        ".sokf/config.toml",
        "[knowledge]\nroots = [\"/knowledge\", \"/other\"]\n",
    );
    f.write(
        "other/manifest.sokf.yaml",
        "sokf: \"0.6\"\nname: other\nscope: test\n",
    );
    f.write("other/index.md", "# other\n");
    f.write("other/schemas/index.md", "# schemas\n");
    let read = |p: &str| std::fs::read_to_string(f.0.join(p)).unwrap();
    f.write(
        "other/schemas/issue.md",
        &read("knowledge/schemas/issue.md").replace("/knowledge/issues", "/other/issues"),
    );
    f.write("other/schemas/note.md", &read("knowledge/schemas/note.md"));
    f.write("other/notes/index.md", "# notes\n");
}

/// `created` runs `sokf new` with `args` and `--write` on `f`, checks that
/// the command succeeds, and returns the path that it reports.
fn created(f: &Fixture, args: &[&str]) -> String {
    let mut full = vec!["--repo-root", f.root(), "new"];
    full.extend_from_slice(args);
    full.push("--write");
    let (stdout, stderr, code) = run(&full);
    assert_eq!(code, 0, "{args:?}: {stderr}");
    stdout
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("created "))
        .unwrap_or_else(|| panic!("{stdout}"))
        .to_string()
}

// sokf-tests: CLI-6_AC-17
// sokf-tests: CLI-6_AC-37
// sokf-tests: CLI-6_AC-63
#[test]
fn a_type_with_no_schema_or_a_schema_in_two_roots_exits_2() {
    let f = Fixture::new("new");
    let stderr = refused(&f, &["Nope", "--id", "x"]);
    assert!(stderr.contains("`Nope` has no schema"), "{stderr}");
    two_roots(&f);
    let stderr = refused(&f, &["Issue", "--slug", "x"]);
    assert!(
        stderr.contains("more than one root declares `Issue`: /knowledge, /other"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Write `new-fixture:Issue` or `other:Issue`"),
        "{stderr}"
    );
}

// sokf-tests: CLI-6_AC-60
#[test]
fn a_qualified_type_takes_the_schema_of_its_store() {
    let f = Fixture::new("new");
    two_roots(&f);
    // The Issue schema of `other` puts its documents under `/other/issues`,
    // so the path shows which schema the command used.
    let path = created(
        &f,
        &["other:Issue", "--slug", "retry", "--state", "kind=bug"],
    );
    assert!(path.starts_with("/other/issues/open/"), "{path}");
    let path = created(
        &f,
        &[
            "test/new-fixture:Issue",
            "--slug",
            "retry",
            "--state",
            "kind=bug",
        ],
    );
    assert!(path.starts_with("/knowledge/issues/open/"), "{path}");
    let stderr = refused(&f, &["nowhere:Issue", "--slug", "x"]);
    assert!(stderr.contains("no store is named `nowhere`"), "{stderr}");
}

// sokf-tests: CLI-6_AC-61
#[test]
fn the_directory_chooses_the_store_of_an_unqualified_type() {
    let f = Fixture::new("new");
    two_roots(&f);
    let path = created(
        &f,
        &[
            "Note",
            "--title",
            "Groceries",
            "--id",
            "note-a",
            "--dir",
            "/other/notes",
        ],
    );
    assert_eq!(path, "/other/notes/note-a.md");
}

// sokf-tests: CLI-6_AC-62
#[test]
fn a_qualified_type_and_a_directory_in_another_store_exit_2() {
    let f = Fixture::new("new");
    two_roots(&f);
    let stderr = refused(
        &f,
        &[
            "new-fixture:Note",
            "--title",
            "Groceries",
            "--id",
            "note-a",
            "--dir",
            "/other/notes",
        ],
    );
    assert!(
        stderr.contains("`new-fixture:Note` names the store in /knowledge, but the path of `--dir` lies in /other"),
        "{stderr}"
    );
}

// sokf-tests: CLI-6_AC-21
#[test]
fn a_sequence_number_wider_than_the_template_exits_2() {
    let f = Fixture::new("new");
    f.write(
        "knowledge/issues/open/bug/issue-999-last.md",
        "---\ntype: Issue\nid: issue-999-last\ntitle: Last\nstate: open\nkind: bug\n---\n\n# Last\n\nThe last issue.\n\n## Reproduction\n\n1. Run it.\n",
    );
    let stderr = refused(&f, &["Issue", "--slug", "x", "--write"]);
    assert!(
        stderr.contains("the next number 1000 needs more than 3 digits"),
        "{stderr}"
    );
}

// sokf-tests: CLI-6_AC-64
// sokf-tests: CLI-6_AC-65
#[test]
fn a_type_with_a_root_goes_in_the_root_and_refuses_a_directory_outside_it() {
    let f = Fixture::new("new");
    let schema = std::fs::read_to_string(f.0.join("knowledge/schemas/note.md")).unwrap();
    f.write(
        "knowledge/schemas/note.md",
        &schema.replacen(
            "line-limit: 40\n",
            "line-limit: 40\nroot: /knowledge/notes\n",
            1,
        ),
    );
    let path = created(&f, &["Note", "--title", "Groceries", "--id", "note-a"]);
    assert_eq!(path, "/knowledge/notes/note-a.md");
    let stderr = refused(
        &f,
        &[
            "Note",
            "--title",
            "Groceries",
            "--id",
            "note-b",
            "--dir",
            "/knowledge",
        ],
    );
    assert!(
        stderr.contains("/knowledge lies outside /knowledge/notes, the root of the type's schema"),
        "{stderr}"
    );
}

/// `titled` writes the schema of the type `Titled`, whose id template is
/// `titled-${:slug}` and whose frontmatter `title` has the rule `rule`.
fn titled(f: &Fixture, rule: &str) {
    f.write(
        "knowledge/schemas/titled.md",
        &format!(
            "---\ntype: Schema\nid: schema-titled\ntitle: Titled Schema\ndescription: A type with a rule for its title.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Titled\n  id:\n    template: \"titled-${{:slug}}\"\n  title:\n{rule}sections:\n  - heading: {{ from: title }}\n    level: 1\n    blocks: []\n````\n"
        ),
    );
}

// sokf-tests: CLI-6_AC-6
// sokf-tests: CLI-6_AC-42
// sokf-tests: CLI-6_AC-66
// sokf-tests: CLI-6_AC-67
#[test]
fn a_type_whose_schema_sets_a_rule_for_the_title_needs_title() {
    let f = Fixture::new("new");
    // A template with literal text or a typed placeholder, a pattern, and a
    // list of choices each refuse every derived title, even one that would
    // pass, and the error gives the example.
    for (rule, example) in [
        (
            "    template: \"Add ${}\"\n    example: Add a portal\n",
            "Add a portal",
        ),
        (
            "    pattern: \"^Add \"\n    example: Add a portal\n",
            "Add a portal",
        ),
        ("    enum: [Add one, Add two]\n", "Add one"),
        (
            "    template: \"${:word}\"\n    example: Portal\n",
            "Portal",
        ),
    ] {
        titled(&f, rule);
        let stderr = refused(&f, &["Titled", "--slug", "add-portal"]);
        assert!(
            stderr.contains(&format!("`--title \"{example}\"`")),
            "{rule}: {stderr}"
        );
        let (stdout, stderr, code) = f.run(&["new", "Titled", "--slug", "add-portal", "--id-only"]);
        assert_eq!(
            (stdout.as_str(), code),
            ("titled-add-portal\n", 0),
            "{stderr}"
        );
    }
    // A title that `--title` gives passes or fails the rule.
    titled(&f, "    template: \"Add ${}\"\n    example: Add a portal\n");
    let (stdout, stderr, code) =
        f.run(&["new", "Titled", "--slug", "p", "--title", "Add a portal"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("title: Add a portal\n"), "{stdout}");
    let stderr = refused(&f, &["Titled", "--slug", "p", "--title", "Remove a portal"]);
    assert!(stderr.contains("doesn't match the schema"), "{stderr}");
    // A template of an untyped placeholder alone sets no rule.
    titled(&f, "    template: \"${}\"\n    example: Add a portal\n");
    let (stdout, stderr, code) = f.run(&["new", "Titled", "--slug", "spending-portal"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("title: Spending portal\n"), "{stdout}");
    // A `const` is the title.
    titled(&f, "    const: \"Titled: The one\"\n");
    let (stdout, stderr, code) = f.run(&["new", "Titled", "--slug", "x"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("title: \"Titled: The one\"\n"), "{stdout}");
    assert!(stdout.contains("# Titled: The one\n"), "{stdout}");
}

// sokf-tests: CLI-6_AC-68
#[test]
fn the_help_of_title_says_that_a_rule_needs_it() {
    let (stdout, _, code) = run(&["new", "--help"]);
    assert_eq!(code, 0);
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("--title"))
        .unwrap_or_else(|| panic!("{stdout}"));
    assert!(
        line.contains("A type whose schema sets a rule for `title`, such as a template, needs it."),
        "{line}"
    );
}

// sokf-tests: CLI-6_AC-4
// sokf-tests: CLI-6_AC-69
#[test]
fn seq_gives_the_number_and_leaves_the_numbers_below_it_free() {
    let f = Fixture::new("new");
    let (stdout, stderr, code) =
        f.run(&["new", "Issue", "--slug", "x", "--seq", "42", "--id-only"]);
    assert_eq!((stdout.as_str(), code), ("issue-042-x\n", 0), "{stderr}");
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "x", "--seq", "42", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["id"], "issue-042-x");
    assert!(
        value["document"]
            .as_str()
            .unwrap()
            .contains("\nid: issue-042-x\n"),
        "{stdout}"
    );
    let path = created(&f, &["Issue", "--slug", "x", "--seq", "42"]);
    assert_eq!(path, "/knowledge/issues/open/bug/issue-042-x.md");
    // Without `--seq`, the number is one more than the highest, and a number
    // below it that no document has is still free.
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "y", "--id-only"]);
    assert_eq!((stdout.as_str(), code), ("issue-043-y\n", 0), "{stderr}");
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "y", "--seq", "2", "--id-only"]);
    assert_eq!((stdout.as_str(), code), ("issue-002-y\n", 0), "{stderr}");
}

// sokf-tests: CLI-6_AC-70
// sokf-tests: CLI-6_AC-71
#[test]
fn seq_refuses_a_number_in_use_and_a_number_too_wide() {
    let f = Fixture::new("new");
    let stderr = refused(&f, &["Issue", "--slug", "x", "--seq", "1", "--write"]);
    assert!(
        stderr.contains("`issue-001-first` in /knowledge/issues/open/bug/issue-001-first.md already has the number 1"),
        "{stderr}"
    );
    // The number is in use whatever the other placeholders and states of the
    // document that has it.
    f.write(
        "knowledge/issues/open/feature/issue-005-other.md",
        "---\ntype: Issue\nid: issue-005-other\ntitle: Other\nstate: open\nkind: feature\n---\n\n# Other\n\nAnother issue.\n",
    );
    let stderr = refused(&f, &["Issue", "--slug", "x", "--seq", "5", "--id-only"]);
    assert!(stderr.contains("`issue-005-other`"), "{stderr}");
    let stderr = refused(&f, &["Issue", "--slug", "x", "--seq", "1000", "--id-only"]);
    assert!(
        stderr.contains("the number 1000 needs more than 3 digits"),
        "{stderr}"
    );
    // A document whose number is unknown might have the given one.
    f.write(
        "knowledge/issues/open/bug/issue-odd.md",
        "---\ntype: Issue\nid: issue-odd\ntitle: Odd\nstate: open\nkind: bug\n---\n\n# Odd\n\nAn odd issue.\n\n## Reproduction\n\n1. Run it.\n",
    );
    let stderr = refused(&f, &["Issue", "--slug", "x", "--seq", "7", "--id-only"]);
    assert!(
        stderr.contains("has an id that does not match the id template"),
        "{stderr}"
    );
}

// sokf-tests: CLI-6_AC-18
#[test]
fn seq_refuses_a_type_with_no_seq_placeholder() {
    let f = Fixture::new("new");
    titled(&f, "    template: \"${}\"\n    example: Add a portal\n");
    let stderr = refused(&f, &["Titled", "--slug", "x", "--seq", "3", "--id-only"]);
    assert!(
        stderr.contains(
            "`--seq 3` is given, but the id template of `Titled` has no `seq` placeholder"
        ),
        "{stderr}"
    );
    let stderr = refused(
        &f,
        &["Note", "--id", "n", "--title", "Groceries", "--seq", "3"],
    );
    assert!(
        stderr.contains("`--seq 3` is given, but `Note` has no id template"),
        "{stderr}"
    );
}

// sokf-tests: CLI-6_AC-72
// sokf-tests: CLI-6_AC-57
#[test]
fn write_and_json_write_the_document_and_print_the_object() {
    let f = Fixture::new("new");
    let (stdout, stderr, code) = f.run(&["new", "Issue", "--slug", "x", "--write", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["id"], "issue-002-x");
    assert_eq!(value["path"], "/knowledge/issues/open/bug/issue-002-x.md");
    assert!(value.get("created").is_none());
    let path = value["path"].as_str().unwrap().trim_start_matches('/');
    assert_eq!(
        std::fs::read_to_string(f.0.join(path)).unwrap(),
        value["document"].as_str().unwrap()
    );
    // A refusal writes nothing and prints no object.
    let (stdout, stderr, code) = f.run(&[
        "new", "Issue", "--slug", "x", "--seq", "2", "--write", "--json",
    ]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    // `--id-only` still refuses either of the other two.
    let before = f.files();
    for args in [
        vec!["new", "Issue", "--slug", "y", "--write", "--id-only"],
        vec!["new", "Issue", "--slug", "y", "--json", "--id-only"],
        vec![
            "new",
            "Issue",
            "--slug",
            "y",
            "--write",
            "--json",
            "--id-only",
        ],
    ] {
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {stderr}");
        assert!(stdout.is_empty(), "{args:?}: {stdout}");
    }
    assert_eq!(f.files(), before);
}

// sokf-tests: CLI-6_AC-73
#[test]
fn write_leaves_no_index_stale() {
    let f = crate::common::project();
    crate::common::take_method(&f);
    let (out, err, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 0, "{out}{err}");
    // A second run fixes nothing.
    let (out, err, code) = f.run(&["validate", "--fix"]);
    assert_eq!((code, out.as_str()), (0, "0 errors, 0 warnings\n"), "{err}");
    // The issue lands in `issues/open/`, a directory that doesn't exist yet,
    // so the command creates its index and rewrites the index above it.
    let (out, err, code) = f.run(&[
        "new",
        "Issue",
        "--slug",
        "first",
        "--title",
        "Add the first issue",
        "--state",
        "kind=feature",
        "--state",
        "state=open",
        "--write",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("updated /knowledge/issues/open/index.md\n"),
        "{out}"
    );
    assert!(
        out.contains("updated /knowledge/issues/index.md\n"),
        "{out}"
    );
    let (out, _, _) = f.run(&["validate"]);
    assert!(!out.contains("index.md"), "{out}");
}

// A commit hook exports the `GIT_DIR` of its own repository, and the scan of
// a `new` that it runs still reads the branches of the repository at the
// root.
// sokf-tests: CLI-1_AC-19
#[test]
fn a_number_on_another_branch_counts_whatever_git_dir_names() {
    let f = Fixture::new("new");
    git(&f.0, &["init", "-q"]);
    git(&f.0, &["add", "-A"]);
    git(&f.0, &["commit", "-q", "-m", "start"]);
    git(&f.0, &["switch", "-q", "-c", "other"]);
    let first = std::fs::read_to_string(f.path("knowledge/issues/open/bug/issue-001-first.md"))
        .unwrap()
        .replace("issue-001-first", "issue-002-elsewhere");
    f.write("knowledge/issues/open/bug/issue-002-elsewhere.md", &first);
    git(&f.0, &["add", "-A"]);
    git(&f.0, &["commit", "-q", "-m", "elsewhere"]);
    git(&f.0, &["switch", "-q", "main"]);
    // The hook's repository has no branch, so a scan of it gives no number.
    let hook = Fixture::new("manifest");
    git(&hook.0, &["init", "-q"]);
    // Only the binary under test inherits the variables, with the setting of
    // the pathspecs that a `git --glob-pathspecs commit` exports to its hook.
    let (stdout, stderr, code) = common::output(
        common::sokf()
            .env("GIT_DIR", hook.0.join(".git"))
            .env("GIT_GLOB_PATHSPECS", "1")
            .arg("--repo-root")
            .arg(f.root())
            .args(["new", "Issue", "--slug", "here", "--id-only"]),
    );
    assert_eq!((stdout.as_str(), code), ("issue-003-here\n", 0), "{stderr}");
}
