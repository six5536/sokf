//! End-to-end tests for the marker edges of `sokf graph`: the markers of
//! one file fold under a document, `--all` and a file or a key show each
//! marker, and the JSON carries the same view. Each test writes two files
//! with markers into a copy of the `clean` fixture, whose requirement
//! `req-cli` defines the keys `CLI-1_AC-1` and `CLI-1_AC-2`.

use common::{Fixture, run};

use crate::common;

// sokf:ignore-begin
/// The first file implements both keys and tests the document.
const LOAD: &str = "// sokf-implements: CLI-1_AC-1\nfn load() {}\n// sokf-implements: CLI-1_AC-2\nfn report() {}\n// sokf-tests: req-cli\nfn check() {}\n";
/// The second file implements the first key.
const PARSE: &str = "fn parse() {}\n// sokf-implements: CLI-1_AC-1\n";
// sokf:ignore-end

/// `fixture` returns a copy of `clean` with the two marker files.
fn fixture() -> Fixture {
    let f = Fixture::new("clean");
    f.write("src/load.rs", LOAD);
    f.write("src/parse.rs", PARSE);
    f
}

/// `graph` runs `sokf graph` on the fixture, with the cache or with
/// `--no-cache`, and returns its stdout. It fails the test unless the
/// command exits with code 0.
fn graph(f: &Fixture, cached: bool, args: &[&str]) -> String {
    let mut all = vec!["--repo-root", f.root(), "graph"];
    if !cached {
        all.push("--no-cache");
    }
    all.extend_from_slice(args);
    let (stdout, stderr, code) = run(&all);
    assert_eq!(code, 0, "{all:?}: {stderr}");
    stdout
}

/// `marker_lines` returns the lines of `text` that show a marker edge.
fn marker_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| {
            l.contains("/src/load.rs") || l.contains("/src/parse.rs") || l.contains("(line")
        })
        .filter(|l| !l.contains("defined-in"))
        .collect()
}

// Under a document, the markers of one file and one rel are one edge with
// their count, even a single marker. The view is the same with the cache
// and without it, which reads every file for markers.
// sokf-tests: CLI-7_AC-90
// sokf-tests: CLI-7_AC-91
// sokf-tests: CLI-7_AC-95
#[test]
fn a_document_shows_the_markers_of_each_file_as_one_edge() {
    let f = fixture();
    let text = graph(&f, true, &["req-cli"]);
    assert_eq!(
        marker_lines(&text),
        vec![
            "├── implemented-by → /src/load.rs (2 markers)",
            "├── tested-by → /src/load.rs (1 marker)",
            "└── implemented-by → /src/parse.rs (1 marker)",
        ],
        "{text}"
    );
    assert_eq!(graph(&f, false, &["req-cli"]), text);
}

// With `--all`, each marker is an edge, with the key that it cites after the
// rel and its line after the target.
// sokf-tests: CLI-7_AC-92
#[test]
fn all_shows_each_marker_with_its_key_and_line() {
    let f = fixture();
    let text = graph(&f, true, &["req-cli", "--all"]);
    assert_eq!(
        marker_lines(&text),
        vec![
            "├── implemented-by #cli-1_ac-1 → /src/load.rs (line 1)",
            "├── implemented-by #cli-1_ac-2 → /src/load.rs (line 3)",
            "├── tested-by → /src/load.rs (line 5)",
            "└── implemented-by #cli-1_ac-1 → /src/parse.rs (line 2)",
        ],
        "{text}"
    );
    assert_eq!(graph(&f, false, &["req-cli", "--all"]), text);
}

// A file shows each of its markers, with the document and the key that the
// marker cites, and its line.
// sokf-tests: CLI-7_AC-93
#[test]
fn a_file_shows_each_of_its_markers() {
    let f = fixture();
    let text = graph(&f, true, &["/src/load.rs"]);
    assert_eq!(
        marker_lines(&text),
        vec![
            "/src/load.rs",
            "├── implements → req-cli#cli-1_ac-1 (line 1)",
            "├── implements → req-cli#cli-1_ac-2 (line 3)",
            "└── tests → req-cli (line 5)",
        ],
        "{text}"
    );
    assert_eq!(graph(&f, false, &["/src/load.rs"]), text);
}

// A key shows the file and the line of each marker that cites it.
// sokf-tests: CLI-7_AC-94
#[test]
fn a_key_shows_each_marker_that_cites_it() {
    let f = fixture();
    let text = graph(&f, true, &["#CLI-1_AC-1"]);
    assert_eq!(
        marker_lines(&text),
        vec![
            "├── implemented-by → /src/load.rs (line 1)",
            "└── implemented-by → /src/parse.rs (line 2)",
        ],
        "{text}"
    );
    assert_eq!(graph(&f, false, &["#CLI-1_AC-1"]), text);
}

// In JSON, a folded marker edge carries the count of its markers, and no
// region and no line. An edge of one marker carries both.
// sokf-tests: CLI-7_AC-96
#[test]
fn json_counts_the_markers_of_a_folded_edge() {
    let f = fixture();
    let query = "edges[?kind=='marker' && !synthesised]";
    let folded = graph(&f, true, &["req-cli", "--json", "--query", query]);
    assert_eq!(
        folded.trim(),
        concat!(
            r#"[{"kind":"marker","from":"/src/load.rs","to":"/knowledge/req-cli.md","rel":"implements","markers":2},"#,
            r#"{"kind":"marker","from":"/src/load.rs","to":"/knowledge/req-cli.md","rel":"tests","markers":1},"#,
            r#"{"kind":"marker","from":"/src/parse.rs","to":"/knowledge/req-cli.md","rel":"implements","markers":1}]"#,
        )
    );
    let each = graph(&f, true, &["/src/parse.rs", "--json", "--query", query]);
    assert_eq!(
        each.trim(),
        r#"[{"kind":"marker","from":"/src/parse.rs","to":"/knowledge/req-cli.md","rel":"implements","region":"cli-1_ac-1","line":2}]"#
    );
}

// sokf:ignore-begin
/// A file that implements one key on two lines.
const TWICE: &str =
    "// sokf-implements: CLI-1_AC-2\nfn one() {}\n// sokf-implements: CLI-1_AC-2\nfn two() {}\n";
// sokf:ignore-end

// Each marker edge has its own inverse: two markers of one key in one file
// differ by their line, so the JSON of one of them holds it and its inverse,
// and not the inverse of the other.
// sokf-tests: CLI-7_AC-90
#[test]
fn each_marker_of_a_key_has_its_own_inverse() {
    let f = fixture();
    f.write("src/twice.rs", TWICE);
    let args = ["/src/twice.rs", "--edge", "line==`3`", "--json", "--query"];
    let lines = graph(
        &f,
        true,
        &[&args[..], &["edges[?kind=='marker'].[rel, line]"]].concat(),
    );
    assert_eq!(lines.trim(), r#"[["implemented-by",3],["implements",3]]"#);
}

// A directory outside the roots lists each file under it that holds a marker,
// because the file is an end of the marker's edge, with the cache and
// without it.
// sokf-tests: CLI-7_AC-35
#[test]
fn a_directory_lists_its_files_with_markers() {
    let f = fixture();
    let text = graph(&f, true, &["/src"]);
    assert!(
        text.contains("├── /src/load.rs\n") && text.contains("/src/parse.rs\n"),
        "{text}"
    );
    assert_eq!(graph(&f, false, &["/src"]), text);
}
