//! End-to-end tests of the drop-in folders of `/.sokf/`: sokf reads each
//! file of `roots.d` and of `formats.d` at any depth, and the files of
//! `formats.d` form one layer of formats, in which no file takes precedence
//! over another.

use crate::common::{Fixture, project};

/// `LINE_SCHEMA` is a schema whose one paragraph has the format
/// `shared-line`, which no layer below `formats.d` declares.
const LINE_SCHEMA: &str = "---\ntype: Schema\nid: schema-line\ntitle: Line schema\ndescription: A line.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Line\nsections:\n  - heading: { from: title }\n    level: 1\n    guidance: The line.\n    blocks:\n      - kind: prose\n        each:\n          format: shared-line\n          example: A line.\n````\n";

/// `ONE` and `TWO` are the files of two tools in `formats.d`, each in a
/// folder of its own.
const ONE: &str = ".sokf/formats.d/pakf/@acme/one/formats.yaml";
const TWO: &str = ".sokf/formats.d/pakf/@acme/two/formats.yaml";

/// `ANY_LINE` declares `shared-line` as any line.
const ANY_LINE: &str = "registry:\n  shared-line:\n    pattern: '.*'\n    example: A line.\n";

/// `with_line` returns the fixture `clean` with the line schema and a
/// document of it whose paragraph is `line`.
fn with_line(line: &str) -> Fixture {
    let f = Fixture::new("clean");
    f.write("knowledge/schemas/line.md", LINE_SCHEMA);
    f.write(
        "knowledge/line.md",
        &format!("---\ntype: Line\nid: line\ntitle: Line\n---\n\n# Line\n\n{line}\n"),
    );
    f
}

/// `validate_clean` runs `validate --fix` twice, and the second run must
/// find nothing and fix nothing.
fn validate_clean(f: &Fixture) {
    let (out, err, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 0, "{out}{err}");
    let (out, err, code) = f.run(&["validate", "--fix"]);
    assert_eq!((code, out.as_str()), (0, "0 errors, 0 warnings\n"), "{err}");
}

/// `entry` returns the lines of the entry `name` of the registry that
/// `info formats` prints with `args`, up to the next entry.
fn entry(f: &Fixture, args: &[&str], name: &str) -> String {
    let mut all = vec!["info", "formats"];
    all.extend_from_slice(args);
    let (out, err, code) = f.run(&all);
    assert_eq!(code, 0, "{err}");
    let head = format!("  {name}:");
    let mut lines = out.lines().skip_while(|l| *l != head);
    let Some(first) = lines.next() else {
        return String::new();
    };
    let body = lines.take_while(|l| l.starts_with("    "));
    std::iter::once(first)
        .chain(body)
        .collect::<Vec<_>>()
        .join("\n")
}

// sokf-tests: CONF-2_AC-48
#[test]
fn a_store_that_a_nested_file_of_roots_d_lists_is_checked() {
    let f = Fixture::new("clean");
    f.write(
        "vendor/kb/manifest.sokf.yaml",
        "sokf: \"0.6\"\nname: kb\nscope: vendor\n",
    );
    f.write(
        ".sokf/roots.d/pakf/@six5536/rust/roots.toml",
        "roots = [\"/vendor/kb\"]\n",
    );
    validate_clean(&f);
    // `--fix` writes the index of the store, so sokf read the store.
    assert!(f.path("vendor/kb/index.md").is_file());
}

// sokf-tests: CONF-11_AC-1
// sokf-tests: CLI-5_AC-3
// sokf-tests: CLI-5_AC-4
// sokf-tests: CLI-5_AC-42
// sokf-tests: CLI-5_AC-43
#[test]
fn the_files_of_formats_d_form_a_layer_below_the_repository() {
    let f = with_line("A line.");
    // No layer declares `shared-line` yet.
    let (_, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1);
    let (_, err, code) = f.run(&["info", "formats", "--layer", "formats.d"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("/.sokf/formats.d/"), "{err}");
    f.write(ONE, ANY_LINE);
    validate_clean(&f);
    let effective = entry(&f, &[], "shared-line");
    assert!(
        effective.contains("source: /.sokf/formats.d/pakf/@acme/one/formats.yaml"),
        "{effective}"
    );
    // The layer alone holds the formats of the folder and no embedded one.
    let layer = entry(&f, &["--layer", "formats.d"], "shared-line");
    assert!(layer.contains("@acme/one"), "{layer}");
    assert_eq!(entry(&f, &["--layer", "formats.d"], "user-story"), "");
    // `/.sokf/formats.yaml` lies above the folder.
    f.write(".sokf/formats.yaml", ANY_LINE);
    validate_clean(&f);
    assert!(entry(&f, &[], "shared-line").contains("source: repo"));
}

// sokf-tests: CONF-11_AC-2
// sokf-tests: CONF-11_AC-3
#[test]
fn a_format_that_two_files_of_formats_d_declare_stops_the_run() {
    let f = with_line("A line.");
    f.write(ONE, ANY_LINE);
    f.write(
        TWO,
        "registry:\n  shared-line:\n    pattern: '[A-Z].*'\n    example: Another line.\n",
    );
    for args in [&["validate"][..], &["info", "formats"]] {
        let (out, err, code) = f.run(args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert!(
            err.contains("`shared-line`")
                && err.contains("/.sokf/formats.d/pakf/@acme/one/formats.yaml")
                && err.contains("/.sokf/formats.d/pakf/@acme/two/formats.yaml")
                && err.contains("/.sokf/formats.yaml"),
            "{args:?}: {err}"
        );
    }
    // A patch in `/.sokf/formats.yaml` decides no clash.
    f.write(
        ".sokf/formats.yaml",
        "registry:\n  shared-line:\n    patch: true\n    example: A patched line.\n",
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    // A declaration in full decides it.
    f.write(".sokf/formats.yaml", ANY_LINE);
    validate_clean(&f);
    assert!(entry(&f, &[], "shared-line").contains("source: repo"));
}

// sokf-tests: CONF-11_AC-2
// sokf-tests: CONF-11_AC-3
#[test]
fn a_type_that_two_files_of_formats_d_declare_stops_the_run() {
    let f = with_line("A line.");
    f.write(ONE, &format!("types:\n  colour: [red]\n{ANY_LINE}"));
    f.write(TWO, "types:\n  colour: [blue]\nregistry: {}\n");
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("type `colour`") && err.contains("@acme/one") && err.contains("@acme/two"),
        "{err}"
    );
    f.write(
        ".sokf/formats.yaml",
        "types:\n  colour: [red, blue]\nregistry: {}\n",
    );
    validate_clean(&f);
}

// sokf-tests: CONF-11_AC-4
#[test]
fn a_patch_of_formats_d_merges_onto_the_layers_below() {
    let f = with_line("A line.");
    f.write(
        ONE,
        &format!(
            "{ANY_LINE}  user-story:\n    patch: true\n    example: 'ST-1: AS A writer, I WANT a patch, SO THAT the folder changes a format.'\n"
        ),
    );
    validate_clean(&f);
    let story = entry(&f, &["--layer", "formats.d"], "user-story");
    assert!(story.contains("the folder changes a format"), "{story}");
    // The template comes from the embedded entry that the patch merges onto.
    assert!(story.contains("I WANT ${}"), "{story}");
}

// sokf-tests: CONF-11_AC-5
#[test]
fn a_type_of_one_file_of_formats_d_reaches_the_formats_of_another() {
    // The file that redefines the embedded type sorts first, so a merge in
    // the order of the paths would compile the later file's format against
    // the embedded type.
    let f = with_line("THE line.");
    f.write(
        ".sokf/formats.d/a/formats.yaml",
        "types:\n  indefinite-article: [A, AN, THE]\nregistry: {}\n",
    );
    f.write(
        ".sokf/formats.d/z/formats.yaml",
        "registry:\n  shared-line:\n    template: '${:indefinite-article} line.'\n    example: A line.\n",
    );
    validate_clean(&f);
}

// sokf-tests: CONF-11_AC-6
#[test]
fn a_file_of_formats_d_that_does_not_parse_stops_the_run() {
    let f = with_line("A line.");
    for bad in [
        "registry: [",
        "other: 1\n",
        "registry:\n  shared-line:\n    pattern: '('\n    example: x\n",
    ] {
        f.write(ONE, bad);
        let (out, err, code) = f.run(&["validate"]);
        assert_eq!(code, 2, "{bad:?}: {out}{err}");
        assert!(
            err.contains("/.sokf/formats.d/pakf/@acme/one/formats.yaml"),
            "{bad:?}: {err}"
        );
    }
}

// sokf-tests: CONF-11_AC-1
#[test]
fn a_change_of_a_file_of_formats_d_reaches_a_cached_check() {
    let f = with_line("a line.");
    f.write(ONE, ANY_LINE);
    validate_clean(&f);
    f.write(
        ONE,
        "registry:\n  shared-line:\n    pattern: '[A-Z].*'\n    example: A line.\n",
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("/knowledge/line.md"), "{out}");
}

// sokf-tests: CONF-11_AC-1
// sokf-tests: CONF-11_AC-4
#[test]
fn the_layer_formats_d_lies_above_the_embedded_formats() {
    let f = project();
    validate_clean(&f);
    let line = entry(&f, &[], "requirement-evolution");
    assert!(line.contains("source: embedded"), "{line}");
    // A patch of the folder merges onto the embedded entry.
    f.write(
        ONE,
        "registry:\n  requirement-evolution:\n    patch: true\n    example: 'Versioning: The file changes with each release.'\n",
    );
    validate_clean(&f);
    let line = entry(&f, &[], "requirement-evolution");
    assert!(line.contains("@acme/one/formats.yaml"), "{line}");
    assert!(
        line.contains("The file changes with each release.") && line.contains("evolution-kind"),
        "{line}"
    );
    // An entry in full of the folder replaces the embedded one.
    f.write(
        ONE,
        "registry:\n  requirement-evolution:\n    pattern: '.*'\n    example: A line.\n",
    );
    validate_clean(&f);
    let line = entry(&f, &[], "requirement-evolution");
    assert!(
        line.contains("@acme/one/formats.yaml") && line.contains("pattern: \".*\""),
        "{line}"
    );
}

// sokf-tests: CONF-11_AC-8
#[test]
fn a_format_of_formats_d_uses_a_new_type_of_another_file() {
    let f = with_line("Red line.");
    f.write(
        ".sokf/formats.d/a/formats.yaml",
        "types:\n  colour: [A, Red]\nregistry: {}\n",
    );
    f.write(
        ".sokf/formats.d/z/formats.yaml",
        "registry:\n  shared-line:\n    template: '${:colour} line.'\n    example: Red line.\n",
    );
    validate_clean(&f);
}

// sokf-tests: CONF-11_AC-2
// sokf-tests: CONF-11_AC-3
#[test]
fn a_clash_names_the_first_file_and_is_decided_before_any_file_compiles() {
    let f = with_line("A line.");
    // `b` declares the format alike with `a`, and `c` differs from both.
    for dir in ["a", "b"] {
        f.write(&format!(".sokf/formats.d/{dir}/formats.yaml"), ANY_LINE);
    }
    f.write(
        ".sokf/formats.d/c/formats.yaml",
        "registry:\n  shared-line:\n    pattern: '[A-Z].*'\n    example: A line.\n",
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("`/.sokf/formats.d/a/formats.yaml` and `/.sokf/formats.d/c/formats.yaml`")
            && !err.contains("/.sokf/formats.d/b/"),
        "{err}"
    );
    // The later file's type would break the format of another file, and the
    // project's declaration in full of the type settles the clash.
    f.write(
        ".sokf/formats.d/a/formats.yaml",
        "types:\n  colour: [A, Red]\nregistry: {}\n",
    );
    f.write(
        ".sokf/formats.d/b/formats.yaml",
        "registry:\n  shared-line:\n    template: '${:colour} line.'\n    example: A line.\n",
    );
    f.write(
        ".sokf/formats.d/c/formats.yaml",
        "types:\n  colour: '[0-9]+'\nregistry: {}\n",
    );
    let (_, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("type `colour`"), "{err}");
    f.write(
        ".sokf/formats.yaml",
        "types:\n  colour: [A, Red]\nregistry: {}\n",
    );
    validate_clean(&f);
}

// sokf-tests: CONF-11_AC-9
#[test]
fn alike_declarations_of_formats_d_count_once() {
    let f = with_line("Red line.");
    // The two files differ in quoting, in the order of the keys, and in
    // their comments, and declare the same format, type, and patch.
    f.write(
        ONE,
        "types:\n  colour: [A, Red]\nregistry:\n  shared-line:\n    template: '${:colour} line.'\n    example: Red line.\n  user-story:\n    patch: true\n    example: 'ST-1: AS A writer, I WANT a patch, SO THAT the folder changes a format.'\n",
    );
    f.write(
        TWO,
        "# The same formats as one.\nregistry:\n  user-story: { example: \"ST-1: AS A writer, I WANT a patch, SO THAT the folder changes a format.\", patch: true }\n  shared-line:\n    example: \"Red line.\" # The line.\n    template: \"${:colour} line.\"\ntypes:\n  colour:\n    - \"A\"\n    - Red\n",
    );
    validate_clean(&f);
    // The first file in the order of the paths stands for the name.
    for (args, name) in [
        (&[][..], "shared-line"),
        (&["--layer", "formats.d"][..], "user-story"),
    ] {
        let line = entry(&f, args, name);
        assert!(
            line.contains("source: /.sokf/formats.d/pakf/@acme/one/formats.yaml"),
            "{line}"
        );
    }
    let (out, err, code) = f.run(&[
        "info",
        "formats",
        "--json",
        "--query",
        "types.colour.source",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("@acme/one"), "{out}");
}

// sokf-tests: CONF-11_AC-2
#[test]
fn the_order_of_the_variants_of_a_format_counts() {
    let f = with_line("A line.");
    let variant = |name: &str| {
        let text = if name == "short" {
            "A line."
        } else {
            "A long line."
        };
        format!("      {name}:\n        text: {text}\n        example: {text}\n")
    };
    let variants = |first: &str, second: &str| {
        format!(
            "registry:\n  shared-line:\n    variants:\n{}{}",
            variant(first),
            variant(second)
        )
    };
    f.write(ONE, &variants("short", "long"));
    f.write(TWO, &variants("short", "long"));
    validate_clean(&f);
    f.write(TWO, &variants("long", "short"));
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("format `shared-line`")
            && err.contains("@acme/one")
            && err.contains("@acme/two"),
        "{err}"
    );
}

// sokf-tests: CONF-11_AC-2
// sokf-tests: CONF-11_AC-9
#[test]
fn each_file_that_differs_from_the_first_declaration_is_named() {
    let f = with_line("A line.");
    let other = "registry:\n  shared-line:\n    pattern: '[A-Z].*'\n    example: A line.\n";
    f.write(".sokf/formats.d/a/formats.yaml", ANY_LINE);
    f.write(".sokf/formats.d/b/formats.yaml", other);
    f.write(".sokf/formats.d/c/formats.yaml", other);
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    for later in ["b", "c"] {
        assert!(
            err.contains(&format!(
                "`/.sokf/formats.d/a/formats.yaml` and `/.sokf/formats.d/{later}/formats.yaml`"
            )),
            "{later}: {err}"
        );
    }
    // A pair alike is one declaration, onto which a patch of the project
    // merges.
    f.write(".sokf/formats.d/c/formats.yaml", ANY_LINE);
    f.write(".sokf/formats.d/b/formats.yaml", ANY_LINE);
    f.write(
        ".sokf/formats.yaml",
        "registry:\n  shared-line:\n    patch: true\n    example: A patched line.\n",
    );
    validate_clean(&f);
    let line = entry(&f, &[], "shared-line");
    assert!(
        line.contains("A patched line.") && line.contains("source: repo"),
        "{line}"
    );
}

// sokf-tests: CONF-11_AC-9
#[test]
fn two_patches_that_list_the_held_variants_in_another_order_are_alike() {
    let f = with_line("A line.");
    let ubiquitous = "      ubiquitous:\n        example: 'PLUMB-4_AC-1 [ubiquitous]: the plumber SHALL queue a message.'\n";
    let event = "      event:\n        example: 'PLUMB-1_AC-1 [event]: WHEN a program writes, the plumber SHALL deliver the message.'\n";
    let patch = |first: &str, second: &str| {
        format!("{ANY_LINE}  requirement-ears:\n    patch: true\n    variants:\n{first}{second}")
    };
    f.write(ONE, &patch(ubiquitous, event));
    f.write(TWO, &patch(event, ubiquitous));
    validate_clean(&f);
}

/// `BIG` is a type that compiles alone and in a template with one hole of
/// it, while a template with two holes of it exceeds the size limit of a
/// regex.
const BIG: &str = "'\\w{1,200}'";

// sokf-tests: CONF-11_AC-3
// sokf-tests: CONF-11_AC-10
#[test]
fn a_format_of_formats_d_compiles_against_the_type_that_settles_a_clash() {
    let f = with_line("A line.");
    // `b`'s format compiles against `b`'s type and the project's, and not
    // against `a`'s, which comes first.
    f.write(
        ".sokf/formats.d/a/formats.yaml",
        &format!("types:\n  colour: {BIG}\nregistry: {{}}\n"),
    );
    f.write(
        ".sokf/formats.d/b/formats.yaml",
        "types:\n  colour: '[A-Z]?'\nregistry:\n  shared-line:\n    template: '${:colour}${:colour} line.${:mark}'\n    example: A line.\n",
    );
    // The format also uses a type of another file, so that `with_packs`
    // compiles it again.
    f.write(
        ".sokf/formats.d/c/formats.yaml",
        "types:\n  mark: '!?'\nregistry: {}\n",
    );
    let (_, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("type `colour`"), "{err}");
    // The declaration of `/.sokf/formats.yaml` is the one that the format
    // compiles against.
    f.write(
        ".sokf/formats.yaml",
        &format!("types:\n  colour: {BIG}\nregistry: {{}}\n"),
    );
    let (_, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.starts_with("error: /.sokf/formats.d/b/formats.yaml: ")
            && err.contains("`shared-line`"),
        "{err}"
    );
    f.write(
        ".sokf/formats.yaml",
        "types:\n  colour: '[A-Z]?'\nregistry: {}\n",
    );
    validate_clean(&f);
    let (out, err, code) = f.run(&[
        "info",
        "formats",
        "--json",
        "--query",
        "types.colour.source",
    ]);
    assert_eq!((code, out.trim()), (0, "\"repo\""), "{err}");
    // The layer holds no declaration of the type that the project settles.
    let (out, err, code) = f.run(&[
        "info",
        "formats",
        "--layer",
        "formats.d",
        "--json",
        "--query",
        "types.colour",
    ]);
    assert_eq!((code, out.trim()), (0, "null"), "{err}");
}

// sokf-tests: CONF-11_AC-5
// sokf-tests: CONF-11_AC-11
#[test]
fn an_embedded_format_that_a_type_of_formats_d_breaks_stops_the_run() {
    // `requirement-evolution` of the embedded formats uses the types
    // `evolution-kind` and `sentence`, and two holes of `BIG` exceed the
    // size limit of a regex.
    let f = project();
    validate_clean(&f);
    let types = format!("types:\n  evolution-kind: {BIG}\n  sentence: {BIG}\n");
    f.write(ONE, &format!("{types}registry: {{}}\n"));
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.starts_with("error: /.sokf/formats.d/pakf/@acme/one/formats.yaml: ")
            && err.contains("`requirement-evolution`")
            && err.contains("the layer `embedded`")
            && err.contains("`evolution-kind`"),
        "{err}"
    );
    // A patch of the folder that takes a type out of the format settles it
    // too.
    f.write(
        ONE,
        &format!(
            "{types}registry:\n  requirement-evolution:\n    patch: true\n    template: '${{:evolution-kind}}: A sentence.'\n    example: 'Versioning: A sentence.'\n"
        ),
    );
    validate_clean(&f);
    f.write(ONE, &format!("{types}registry: {{}}\n"));
    // The project settles it by declaring the format in full, which no
    // schema then sees below the folder.
    f.write(
        ".sokf/formats.yaml",
        "registry:\n  requirement-evolution:\n    pattern: '.*'\n    example: 'Versioning: A sentence.'\n",
    );
    validate_clean(&f);
    // Or by a patch that takes a type out of the format.
    f.write(
        ".sokf/formats.yaml",
        "registry:\n  requirement-evolution:\n    patch: true\n    template: '${:evolution-kind}: A sentence.'\n    example: 'Versioning: A sentence.'\n",
    );
    validate_clean(&f);
    // Or by declaring a type, so that no schema sees the folder's.
    f.write(
        ".sokf/formats.yaml",
        "types:\n  sentence: '[A-Z].*\\.'\nregistry: {}\n",
    );
    validate_clean(&f);
    // A type of the folder against which the format compiles passes.
    std::fs::remove_file(f.path(".sokf/formats.yaml")).unwrap();
    f.write(
        ONE,
        &format!("types:\n  evolution-kind: [Versioning, Deprecation]\n  sentence: {BIG}\nregistry: {{}}\n"),
    );
    validate_clean(&f);
}

// sokf-tests: CONF-11_AC-11
#[test]
fn a_format_below_compiles_against_the_types_that_a_schema_sees() {
    let f = project();
    validate_clean(&f);
    // The error names each type of the folder that the format uses.
    f.write(
        ONE,
        &format!("types:\n  evolution-kind: {BIG}\nregistry: {{}}\n"),
    );
    f.write(TWO, "types:\n  sentence: '\\w{1,199}'\nregistry: {}\n");
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.starts_with("error: /.sokf/formats.d/pakf/@acme/one/formats.yaml: ")
            && err.contains("`requirement-evolution`")
            && err.contains(
                "the type `evolution-kind` of this file and the type `sentence` of `/.sokf/formats.d/pakf/@acme/two/formats.yaml`"
            ),
        "{err}"
    );
    // A type of `/.sokf/formats.yaml` takes part, so the format compiles.
    std::fs::remove_file(f.path(TWO)).unwrap();
    f.write(
        ".sokf/formats.yaml",
        "types:\n  sentence: [x]\nregistry: {}\n",
    );
    validate_clean(&f);
    // The variant `named` of `requirement-summary` uses the two types that
    // the folder changes, and its variant `described` uses one, so only
    // that format fails, and `requirement-evolution` beside it compiles.
    std::fs::remove_file(f.path(".sokf/formats.yaml")).unwrap();
    f.write(
        ONE,
        &format!("types:\n  code-name: {BIG}\n  predicate: {BIG}\nregistry: {{}}\n"),
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("`requirement-summary`") && !err.contains("`requirement-evolution`"),
        "{err}"
    );
    // A type of `/.sokf/formats.yaml` that takes part is named too.
    f.write(
        ONE,
        "types:\n  evolution-kind: '\\w{1,199}'\nregistry: {}\n",
    );
    f.write(
        ".sokf/formats.yaml",
        "types:\n  sentence: '\\w{1,201}'\nregistry: {}\n",
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("`requirement-evolution`")
            && err.contains(
                "the type `evolution-kind` of this file and the type `sentence` of `/.sokf/formats.yaml`"
            )
            && !err.contains("format `requirement-evolution`: "),
        "{err}"
    );
    f.write(
        ".sokf/formats.yaml",
        "types:\n  sentence: [x]\nregistry: {}\n",
    );
    validate_clean(&f);
}
