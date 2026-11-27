//! The tests of the comparison through same-same: what sokf reads from each
//! answer, for the pairs that a formatter's change of layout and a person's
//! edit give.

use std::fs;

use same_same::Named;

use super::*;
use crate::{
    RepoRoot,
    config::{FormatConfig, FormatRule},
    include::{definition_pair, index_pair},
    test_support::{fresh_dir, write},
};

fn path(p: &str) -> RepoPath {
    RepoPath::parse(p).unwrap()
}

/// `pair` returns the pair of an include of the whole file `name`.
fn pair(name: &str, expected: &str, actual: &str) -> Pair {
    Pair {
        path: path("/k/a.md"),
        name: name.into(),
        fragment: false,
        expected: expected.into(),
        actual: actual.into(),
        source: Some(Source {
            path: path(&format!("/src/{name}")),
            range: None,
            lines: (1..=u32::try_from(expected.lines().count()).unwrap()).collect(),
        }),
    }
}

/// `settle` returns the layout of `pairs` in a repository with no formatter.
fn settle(config: &FormatConfig, pairs: &[Pair]) -> Layout {
    let dir = fresh_dir("compare");
    let layout = Session::new(&RepoRoot::new(dir.clone()), false).settle(config, pairs);
    fs::remove_dir_all(&dir).unwrap();
    layout
}

/// `current` returns whether the pair is current with the default `[format]`.
fn current(p: &Pair) -> bool {
    settle(&FormatConfig::default(), std::slice::from_ref(p)).same(p)
}

// A declaration file, a `package.json` that a formatter re-laid or sorted,
// and a list of imports that it sorted are current, with no formatter.
// sokf-tests: CLI-4_AC-44
// sokf-tests: CLI-4_AC-45
// sokf-tests: CLI-4_AC-109
// sokf-tests: CLI-4_AC-113
#[test]
fn a_re_laid_text_of_a_known_format_is_current() {
    for name in [
        "index.d.ts",
        "index.d.mts",
        "index.d.cts",
        "styles.d.css.ts",
    ] {
        assert!(
            current(&pair(
                name,
                "export declare function f(a: string): number;\n",
                "export declare function f(\n  a: string,\n): number;\n",
            )),
            "{name}"
        );
    }
    assert!(current(&pair(
        "package.json",
        "{\"version\": \"1.0.0\", \"name\": \"x\"}\n",
        "{\n  \"name\": \"x\",\n  \"version\": \"1.0.0\"\n}\n",
    )));
    assert!(current(&pair(
        "tsconfig.json",
        "{\"compilerOptions\": {\"strict\": true,},}\n",
        "{ \"compilerOptions\": { \"strict\": true } }\n",
    )));
    assert!(current(&pair(
        "main.ts",
        "import { b } from \"./b\";\nimport { a } from \"./a\";\n\nexport const c = a + b;\n",
        "import { a } from \"./a\";\nimport { b } from \"./b\";\n\nexport const c = a + b;\n",
    )));
    assert!(current(&pair(
        "view.tsx",
        "export const v = <div className=\"p-2 flex\" />;\n",
        "export const v = <div className=\"flex p-2\" />;\n",
    )));
    assert!(current(&pair(
        "notes.txt",
        "\u{feff}one\r\ntwo\r\n",
        "one\ntwo\n"
    )));
}

// A reorder of the items of a list is stale, with the line of the source
// where the first item that differs stands.
// sokf-tests: CLI-4_AC-110
#[test]
fn a_change_of_meaning_is_stale_with_its_line() {
    let p = pair(
        "data.json",
        "{\n  \"a\": [\n    1,\n    2\n  ]\n}\n",
        "{\"a\": [2, 1]}\n",
    );
    let layout = settle(&FormatConfig::default(), std::slice::from_ref(&p));
    assert!(!layout.same(&p));
    assert_eq!(layout.line(&p), Some(3));
    assert_eq!(layout.warning(&p), None);
    // The line of a region is the line of its file.
    let mut region = p.clone();
    region.fragment = true;
    region.source.as_mut().unwrap().lines = vec![7, 8, 9, 10, 11, 12];
    let layout = settle(&FormatConfig::default(), std::slice::from_ref(&region));
    assert_eq!(layout.line(&region), Some(9));
    // A format that same-same doesn't know compares by its bytes, which give
    // the line too.
    let rust = pair(
        "main.rs",
        "fn a() {}\nfn b() {}\n",
        "fn a() {}\nfn c() {}\n",
    );
    let layout = settle(&FormatConfig::default(), std::slice::from_ref(&rust));
    assert!(!layout.same(&rust));
    assert_eq!(layout.line(&rust), Some(2));
    assert_eq!(
        stale_message("/src/data.json", Some(&path("/src/data.json")), Some(9)),
        "`sokf:include /src/data.json` is stale: `/src/data.json` differs from line 9"
    );
}

// A region compares as the range of its file's lines against the code of the
// fence, so the indentation that the region takes from its place is layout.
// sokf-tests: CLI-4_AC-111
#[test]
fn a_region_without_its_indentation_is_current() {
    let dir = fresh_dir("compare-region");
    let file = "server:\n  tls:\n    port: 80\n    host: a\n";
    write(&dir, "src/app.yaml", file);
    let p = Pair {
        path: path("/k/a.md"),
        name: "app.yaml".into(),
        fragment: true,
        expected: "    port: 80\n    host: a\n".into(),
        actual: "port: 80\nhost: a\n".into(),
        source: Some(Source {
            path: path("/src/app.yaml"),
            range: Some(3..5),
            lines: vec![3, 4],
        }),
    };
    let session = Session::new(&RepoRoot::new(dir.clone()), false);
    let layout = session.settle(&FormatConfig::default(), std::slice::from_ref(&p));
    assert!(layout.same(&p));
    let mut edited = p.clone();
    edited.actual = "port: 81\nhost: a\n".into();
    let layout = session.settle(&FormatConfig::default(), std::slice::from_ref(&edited));
    assert!(!layout.same(&edited));
    assert_eq!(layout.line(&edited), Some(3));
    fs::remove_dir_all(&dir).unwrap();
}

// How a YAML 1.1 reader takes a value is a difference, unless `format.same`
// names it.
// sokf-tests: CLI-4_AC-114
// sokf-tests: CONF-10_AC-22
#[test]
fn a_yaml_1_1_value_is_stale_unless_format_same_names_it() {
    let p = pair("app.yaml", "enabled: no\n", "enabled: \"no\"\n");
    assert!(!current(&p));
    let mut config = FormatConfig::default();
    config.same.insert(Named::Yaml11Value);
    assert!(settle(&config, std::slice::from_ref(&p)).same(&p));
}

// A definition block whose destination a person changed is stale, and one
// that a formatter re-laid is current; an index compares as a whole
// Markdown file.
// sokf-tests: CLI-4_AC-112
// sokf-tests: CLI-4_AC-46
#[test]
fn a_definition_block_counts_its_destinations() {
    let doc = path("/k/a.md");
    let rendered = "<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n[sokf:c]: /k/c.md";
    let relaid = definition_pair(
        &doc,
        rendered,
        "<!-- sokf:links -->\n\n[sokf:b]:   /k/b.md\n[sokf:c]:  /k/c.md",
    );
    let edited = definition_pair(
        &doc,
        rendered,
        "<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n[sokf:c]: /k/d.md",
    );
    let label = definition_pair(
        &doc,
        rendered,
        "<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n[sokf:x]: /k/c.md",
    );
    let layout = settle(
        &FormatConfig::default(),
        &[relaid.clone(), edited.clone(), label.clone()],
    );
    assert_eq!(
        definition_pair(&path("/README"), "a", "b").name,
        "README.md"
    );
    assert!(layout.same(&relaid));
    assert!(!layout.same(&edited));
    assert!(!layout.same(&label));
    let index = path("/k/index.md");
    let expected = "# K\n\n- [B][sokf:b] - The b.\n\n<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n";
    let wrapped = index_pair(
        &index,
        expected,
        "# K\n\n-   [B][sokf:b] -\n    The b.\n\n<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n",
    );
    assert!(current(&wrapped));
}

// A format that same-same doesn't know compares byte for byte without a
// warning where no formatter is configured or detected.
// sokf-tests: CLI-4_AC-48
#[test]
fn without_a_formatter_an_unknown_format_compares_byte_for_byte() {
    let p = pair("site.css", "a{color:red}\n", "a {\n  color: red;\n}\n");
    let layout = settle(&FormatConfig::default(), std::slice::from_ref(&p));
    assert!(!layout.same(&p));
    assert_eq!(layout.warning(&p), None);
    assert!(layout.unsettled().is_empty());
}

// A command that is missing compares the pair byte for byte with a warning
// at the configuration file, and leaves the file unsettled.
// sokf-tests: CLI-4_AC-49
// sokf-tests: CACHE-2_AC-9
#[test]
fn a_command_that_is_missing_warns_and_leaves_the_pair_unsettled() {
    let config = FormatConfig {
        rule: Some(FormatRule::Command {
            argv: vec!["./missing-formatter".into(), "{files}".into()],
            config: None,
        }),
        ..FormatConfig::default()
    };
    let p = pair("site.css", "a{color:red}\n", "a {\n  color: red;\n}\n");
    let layout = settle(&config, std::slice::from_ref(&p));
    assert!(!layout.same(&p));
    let warning = layout.warning(&p).expect("the warning");
    assert_eq!(warning.path.as_str(), "/.sokf/config.toml");
    assert!(
        warning.message.contains("doesn't start"),
        "{}",
        warning.message
    );
    assert!(layout.unsettled().contains(&path("/k/a.md")));
    // A pair of a known format needs no formatter.
    let json = pair("a.json", "{\"a\": 1}\n", "{ \"a\": 1 }\n");
    let layout = settle(&config, std::slice::from_ref(&json));
    assert!(layout.same(&json));
    assert!(layout.unsettled().is_empty());
}

// A pass with no pair compares nothing, so it writes nothing to
// same-same's cache, also after the cache asked for the identity.
// sokf-tests: CACHE-3_AC-13
#[test]
fn a_pass_with_no_pair_writes_nothing() {
    let dir = fresh_dir("compare-none");
    let session = Session::new(&RepoRoot::new(dir.clone()), true);
    assert_eq!(session.identity(), "none");
    let layout = session.settle(&FormatConfig::default(), &[]);
    assert!(layout.unsettled().is_empty());
    drop(session);
    let folder = dir.join(".sokf/cache/same-same");
    assert!(!folder.exists() || fs::read_dir(&folder).unwrap().next().is_none());
    fs::remove_dir_all(&dir).unwrap();
}

// Under the cache, same-same keeps its cache in `/.sokf/cache/same-same/`.
// sokf-tests: CACHE-2_AC-10
#[test]
fn the_cache_lies_in_its_folder() {
    let dir = fresh_dir("compare-cache");
    let session = Session::new(&RepoRoot::new(dir.clone()), true);
    let p = pair("a.json", "{\"a\": 1}\n", "{ \"a\": 1 }\n");
    assert!(session.settle(&FormatConfig::default(), &[p]).holds(&pair(
        "a.json",
        "{\"a\": 1}\n",
        "{ \"a\": 1 }\n"
    )));
    drop(session);
    let folder = dir.join(".sokf/cache/same-same");
    assert!(folder.is_dir());
    assert!(fs::read_dir(&folder).unwrap().next().is_some());
    assert_eq!(
        fs::read_to_string(dir.join(".sokf/cache/.gitignore")).unwrap(),
        "*\n"
    );
    fs::remove_dir_all(&dir).unwrap();
}

// The identity of the formatter names the formatter that same-same detects
// with its version, or the formatter of `[format]` with its settings.
// sokf-tests: CACHE-2_AC-11
#[test]
fn the_identity_names_the_formatter() {
    let dir = fresh_dir("compare-identity");
    let identity =
        |dir: &std::path::Path| Session::new(&RepoRoot::new(dir.to_path_buf()), false).identity();
    assert_eq!(identity(&dir), "none");
    write(&dir, ".oxfmtrc.json", "{}\n");
    write(&dir, "node_modules/.bin/oxfmt", "");
    write(
        &dir,
        "node_modules/oxfmt/package.json",
        "{\"name\": \"oxfmt\", \"version\": \"0.77.0\"}\n",
    );
    assert_eq!(identity(&dir), "none", "no detection unless asked");
    write(&dir, ".sokf/config.toml", "[format]\ndetect = true\n");
    let detected = identity(&dir);
    assert!(detected.starts_with("n:oxfmt"), "{detected}");
    write(
        &dir,
        ".sokf/config.toml",
        "[format]\nformatter = \"prettier\"\nconfig = \".prettierrc.json\"\n",
    );
    write(&dir, ".prettierrc.json", "{\"semi\": false}\n");
    let one = identity(&dir);
    assert!(one.starts_with("n:prettier"), "{one}");
    write(&dir, ".prettierrc.json", "{\"semi\": true}\n");
    assert_ne!(identity(&dir), one, "other settings, another identity");
    fs::remove_dir_all(&dir).unwrap();
}

// A pair reads under the path of its file from the root, so a pattern of two
// parts, such as `.vscode/*.json`, reads JSON with comments.
// sokf-tests: CLI-4_AC-113
#[test]
fn a_pair_reads_under_the_path_of_its_file() {
    let source = "{\n  // the width\n  \"editor.rulers\": [80],\n}\n";
    let copy = "{\n  // the width\n  \"editor.rulers\": [80]\n}\n";
    assert!(current(&pair(".vscode/settings.json", source, copy)));
    assert!(!current(&pair("settings.json", source, copy)));
    assert_eq!(
        name_of(&path("/src/.vscode/settings.json")),
        "src/.vscode/settings.json"
    );
    assert_eq!(
        index_pair(&path("/k/index.md"), "a", "b").name,
        "k/index.md"
    );
}

// The labels of an index are read as written: a reference's label before the
// definition block, and a definition's label after its marker, so a line of
// a wrapped entry that opens with a link reads as no definition.
// sokf-tests: CLI-4_AC-117
#[test]
fn an_index_reads_its_labels_as_written() {
    use crate::include::index_labels_as_written;
    let expected = "# K\n\n- [B][sokf:b] - See [the guide](x) and [CLI-1]: the CLI.\n\n<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n";
    let wrapped = "# K\n\n- [B][sokf:b] - See\n  [the guide](x) and [CLI-1]: the CLI.\n\n<!-- sokf:links -->\n\n[sokf:b]: /k/b.md\n";
    assert!(index_labels_as_written(expected, wrapped));
    assert!(!index_labels_as_written(
        expected,
        &expected.replace("][sokf:b]", "][SOKF:B]")
    ));
    assert!(!index_labels_as_written(
        expected,
        &expected.replace("[B][sokf:b]", "[B](/k/b.md)")
    ));
    assert!(!index_labels_as_written(
        expected,
        &expected.replace("[sokf:b]: ", "[Sokf:b]: ")
    ));
}
