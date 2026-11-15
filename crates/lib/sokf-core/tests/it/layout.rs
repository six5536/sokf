//! The comparison by layout: a generated block that the project's formatter
//! re-laid stays current, `--fix` leaves it and writes a stale one as its
//! source holds it, and a run that changes nothing starts no formatter. A
//! small fake formatter stands for the project's: it removes the whitespace
//! of a file, fails on a file that holds `BAD`, hangs on one that holds
//! `SLOW`, fails on every file while the file `broken` exists, and logs each
//! run.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt as _};

use sokf_core::{CacheMode, Report, report_text};

use crate::common::{
    TempTree,
    incremental::{run, set},
};

/// The source that the overview includes, as no formatter left it.
const SOURCE: &str = "{\"a\": [1, 2], \"b\": {\"c\": true}}\n";

/// The overview, which includes the requirement's body and the source.
const OVERVIEW: &str = "---\ntype: Overview\nid: overview\ntitle: Overview\ndescription: Reads the requirement.\nlinks:\n  - rel: references\n    to: req-cli\n---\n\n# Overview\n\nThe CLI requirement is [CLI-1][sokf:#cli-1] in [the criteria][sokf:req-cli#acceptance-criteria], governed by [its schema][sokf:schema-requirement].\n\n<!-- sokf:include req-cli -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/data.json -->\n<!-- /sokf:include -->\n";

/// `tree` returns the clean fixture with the overview, the source, and a
/// fake formatter that `[format]` names, after one `--fix` that fills the
/// blocks.
fn tree(configured: bool) -> TempTree {
    let tree = TempTree::clean_with(&[
        ("knowledge/overview.md", OVERVIEW),
        ("src/data.json", SOURCE),
    ]);
    let log = tree.dir().join("runs.log");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 1.0; exit 0; fi\necho run >> '{log}'\nif [ -e '{broken}' ]; then exit 1; fi\nstatus=0\nfor f in \"$@\"; do\n  if grep -q BAD \"$f\"; then status=1; continue; fi\n  if grep -q SLOW \"$f\"; then sleep 5; fi\n  tr -d ' \\n\\t' < \"$f\" > \"$f.tmp\" && mv \"$f.tmp\" \"$f\"\ndone\nexit $status\n",
        log = log.display(),
        broken = tree.dir().join("broken").display(),
    );
    tree.write("fmt.sh", &script);
    fs::set_permissions(tree.dir().join("fmt.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    if configured {
        tree.write(
            ".sokf/config.toml",
            "[format]\ncommand = [\"./fmt.sh\", \"{files}\"]\n",
        );
    }
    let report = tree.validate(true, CacheMode::On);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    tree
}

/// `runs` returns the number of runs of the fake formatter.
fn runs(tree: &TempTree) -> usize {
    fs::read_to_string(tree.dir().join("runs.log")).map_or(0, |t| t.lines().count())
}

/// `relay` replaces `from` with `to` in the file at `rel`, as a formatter
/// that changes only the layout would.
fn relay(tree: &TempTree, rel: &str, from: &str, to: &str) {
    let text = tree.read(rel);
    assert!(text.contains(from), "{rel} holds no {from:?}");
    tree.write(rel, &text.replacen(from, to, 1));
}

/// `stale` returns the errors of `report` that name a stale block.
fn stale(report: &Report) -> Vec<String> {
    report
        .errors
        .iter()
        .filter(|f| {
            matches!(
                f.criterion.as_str(),
                "SOKF_FORMAT_85" | "SOKF_FORMAT_80" | "SOKF_FORMAT_97"
            )
        })
        .map(|f| format!("{}:{:?}: {}", f.path, f.line, f.message))
        .collect()
}

/// The copy of the source that the project's formatter re-lays.
const RELAID: &str = "{\n  \"a\": [1,\n    2],\n  \"b\": {\n    \"c\": true\n  }\n}\n";

// A block that the formatter re-laid is current, and the second run starts
// no formatter. `--fix` leaves the re-laid blocks as they are.
// sokf-tests: CLI-4_AC-44
// sokf-tests: CLI-4_AC-45
// sokf-tests: CLI-4_AC-46
// sokf-tests: CLI-4_AC-47
// sokf-tests: CLI-4_AC-51
// sokf-tests: CACHE-3_AC-13
// sokf-tests: CONF-10_AC-1
#[test]
fn a_block_that_the_formatter_re_laid_is_current() {
    let tree = tree(true);
    let base = runs(&tree);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    relay(
        &tree,
        "knowledge/overview.md",
        "- CLI-1_AC-1 [event]: WHEN",
        "-   CLI-1_AC-1 [event]:\n    WHEN",
    );
    relay(
        &tree,
        "knowledge/overview.md",
        "[sokf:#cli-1]: /knowledge/req-cli.md#cli-1",
        "[sokf:#cli-1]:   /knowledge/req-cli.md#cli-1",
    );
    relay(
        &tree,
        "knowledge/index.md",
        "[sokf:overview]: /knowledge/overview.md",
        "[sokf:overview]:  /knowledge/overview.md",
    );
    // An entry wrapped over two lines leaves the generated form.
    relay(
        &tree,
        "knowledge/index.md",
        "- [Overview][sokf:overview] - Reads",
        "- [Overview][sokf:overview] -\n  Reads",
    );
    let report = tree.validate(false, CacheMode::On);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
    assert_eq!(runs(&tree), base + 1, "one run formats every text");
    let again = tree.validate(false, CacheMode::On);
    assert!(stale(&again).is_empty(), "{:?}", stale(&again));
    assert_eq!(
        runs(&tree),
        base + 1,
        "a run that changes nothing starts no formatter"
    );
    let before = tree.read("knowledge/overview.md");
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.fixes.is_empty(), "{:?}", fixed.fixes);
    assert_eq!(tree.read("knowledge/overview.md"), before);
    // Without the cache, the run formats again and finds the same.
    let cold = tree.validate(false, CacheMode::Off);
    assert!(stale(&cold).is_empty(), "{:?}", stale(&cold));
}

// A block whose meaning differs is stale, and `--fix` writes the source's
// text as it stands.
// sokf-tests: CLI-4_AC-45
// sokf-tests: CLI-4_AC-50
#[test]
fn a_block_whose_meaning_differs_is_stale() {
    let tree = tree(true);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    tree.write("src/data.json", "{\"a\": [1, 3], \"b\": {\"c\": true}}\n");
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    assert!(
        tree.read("knowledge/overview.md")
            .contains("```json\n{\"a\": [1, 3], \"b\": {\"c\": true}}\n```"),
        "{}",
        tree.read("knowledge/overview.md")
    );
    let second = tree.validate(true, CacheMode::On);
    assert!(second.fixes.is_empty(), "{:?}", second.fixes);
}

// Without a formatter, a re-laid block is stale.
// sokf-tests: CLI-4_AC-48
#[test]
fn without_a_formatter_the_comparison_is_byte_for_byte() {
    let tree = tree(false);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(runs(&tree), 0);
}

// A text that the formatter fails on compares byte for byte, and the run
// reports one warning, which the text report lists without `--warnings`.
// sokf-tests: CLI-4_AC-49
// sokf-tests: CLI-4_AC-43
// sokf-tests: CLI-4_AC-2
#[test]
fn a_failure_of_the_formatter_warns_once() {
    let tree = tree(true);
    let bad = "{\"a\": \"BAD\"}\n";
    tree.write("src/data.json", bad);
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    relay(
        &tree,
        "knowledge/overview.md",
        bad,
        "{\n  \"a\": \"BAD\"\n}\n",
    );
    relay(
        &tree,
        "knowledge/overview.md",
        "[sokf:#cli-1]: /knowledge/req-cli.md#cli-1",
        "[sokf:#cli-1]:   /knowledge/req-cli.md#cli-1",
    );
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    let warnings: Vec<_> = report
        .warnings
        .iter()
        .filter(|w| w.criterion == "CLI-4_AC-49")
        .collect();
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert_eq!(warnings[0].path.as_str(), "/.sokf/config.toml");
    let text = report_text(&report, false);
    assert!(
        text.contains("warning: the formatter ./fmt.sh failed on a text"),
        "{text}"
    );
    // A run on other paths still reports the warning.
    let scoped = tree.validate_paths(&["knowledge/req-cli.md"], false);
    assert!(
        scoped.warnings.iter().any(|w| w.criterion == "CLI-4_AC-49"),
        "{:?}",
        scoped.warnings
    );
}

// A formatter that the repository names but that isn't installed makes the
// run compare byte for byte and warn, and the next run checks the overview
// again, until the formatter starts.
// sokf-tests: CLI-4_AC-49
// sokf-tests: CACHE-2_AC-9
#[test]
fn a_formatter_that_does_not_start_compares_byte_for_byte() {
    let tree = tree(false);
    tree.write(
        ".sokf/config.toml",
        "[format]\ncommand = [\"./later.sh\", \"{files}\"]\n",
    );
    let first = tree.validate(false, CacheMode::On);
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    for _ in 0..2 {
        let report = tree.validate(false, CacheMode::On);
        assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
        let warning = report
            .warnings
            .iter()
            .find(|w| w.criterion == "CLI-4_AC-49")
            .expect("the warning");
        assert!(
            warning.message.contains("doesn't start"),
            "{}",
            warning.message
        );
    }
    // Once the formatter is installed, the next run checks the overview
    // again, though the overview didn't change.
    fs::copy(tree.dir().join("fmt.sh"), tree.dir().join("later.sh")).unwrap();
    let report = tree.validate(false, CacheMode::On);
    assert!(
        stale(&report).is_empty(),
        "{:?} {:?}",
        stale(&report),
        report.warnings
    );
}

// A run whose formatter doesn't start keeps the findings of every document
// that needed no formatter, so the next run checks the overview alone, and
// checks it again until the formatter starts.
// sokf-tests: CACHE-2_AC-9
#[test]
fn a_formatter_that_does_not_start_keeps_the_other_documents() {
    let tree = tree(false);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    // The new configuration rebuilds the store, so the first run checks
    // every document.
    tree.write(
        ".sokf/config.toml",
        "[format]\ncommand = [\"./later.sh\", \"{files}\"]\n",
    );
    let (_, full, _, _) = run(&tree);
    assert!(full);
    for _ in 0..2 {
        let (dirty, full, lines, _) = run(&tree);
        assert!(!full, "the first run kept its findings");
        assert_eq!(dirty, set(&["/knowledge/overview.md"]), "{lines:?}");
    }
    fs::copy(tree.dir().join("fmt.sh"), tree.dir().join("later.sh")).unwrap();
    let (dirty, _, lines, _) = run(&tree);
    assert_eq!(dirty, set(&["/knowledge/overview.md"]));
    assert!(
        !lines.iter().any(|l| l.contains("SOKF_FORMAT_85")),
        "{lines:?}"
    );
    let (dirty, _, _, _) = run(&tree);
    assert!(dirty.is_empty(), "the overview is settled: {dirty:?}");
}

// A formatter that fails on every text for a reason of its environment, as
// dprint does offline with no plugins in its cache, fails on its sample
// too, so the run records no failure of the texts, and the first run after
// the environment works again compares them by their form.
// sokf-tests: CLI-4_AC-70
// sokf-tests: CLI-4_AC-71
#[test]
fn a_failure_of_the_environment_lasts_no_longer_than_the_environment() {
    let tree = tree(true);
    let base = runs(&tree);
    tree.write("broken", "");
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    for i in 1..=2 {
        let report = tree.validate(false, CacheMode::On);
        assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
        let warning = report
            .warnings
            .iter()
            .find(|w| w.criterion == "CLI-4_AC-49")
            .expect("the warning");
        assert!(warning.message.contains("sample"), "{}", warning.message);
        // The batch fails, then the run of the samples fails, and no text
        // runs alone.
        assert_eq!(runs(&tree), base + 2 * i, "each run tries again");
    }
    fs::remove_file(tree.dir().join("broken")).unwrap();
    let report = tree.validate(false, CacheMode::On);
    assert!(
        stale(&report).is_empty(),
        "{:?} {:?}",
        stale(&report),
        report.warnings
    );
}

// A text on which the formatter hangs costs the limit in one run alone:
// that run records its failure, and the next run compares it byte for byte
// with no run of the formatter.
// sokf-tests: CLI-4_AC-73
// sokf-tests: CLI-4_AC-74
// sokf-tests: CLI-4_AC-91
// sokf-tests: CONF-10_AC-8
// sokf-tests: CONF-10_AC-20
#[test]
fn a_text_on_which_the_formatter_hangs_is_a_lasting_failure() {
    let tree = tree(true);
    tree.write(
        ".sokf/config.toml",
        "[format]\ncommand = [\"./fmt.sh\", \"{files}\"]\ntimeout = 1\n",
    );
    let slow = "{\"a\": \"SLOW\"}\n";
    tree.write("src/data.json", slow);
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    relay(
        &tree,
        "knowledge/overview.md",
        slow,
        "{\n  \"a\": \"SLOW\"\n}\n",
    );
    let start = std::time::Instant::now();
    let first = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&first).len(), 1, "{:?}", stale(&first));
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
    let base = runs(&tree);
    let start = std::time::Instant::now();
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(
        report.warnings.iter().any(|w| w.criterion == "CLI-4_AC-49"),
        "{:?}",
        report.warnings
    );
    assert_eq!(runs(&tree), base, "the failure lasts");
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
}

// An index that the formatter re-laid while it didn't start is checked
// again in the next run, so it is current once the formatter starts,
// though nothing in its directory changed.
// sokf-tests: CACHE-2_AC-9
#[test]
fn an_unsettled_index_is_checked_again() {
    let tree = tree(false);
    relay(
        &tree,
        "knowledge/index.md",
        "[sokf:overview]: /knowledge/overview.md",
        "[sokf:overview]:  /knowledge/overview.md",
    );
    tree.write(
        ".sokf/config.toml",
        "[format]\ncommand = [\"./later.sh\", \"{files}\"]\n",
    );
    let index_errors = |report: &Report| {
        report
            .errors
            .iter()
            .filter(|f| f.path.as_str() == "/knowledge/index.md")
            .count()
    };
    for _ in 0..2 {
        let report = tree.validate(false, CacheMode::On);
        assert_eq!(index_errors(&report), 1);
        assert!(
            report.warnings.iter().any(|w| w.criterion == "CLI-4_AC-49"),
            "the index reports why it compares byte for byte: {:?}",
            report.warnings
        );
    }
    fs::copy(tree.dir().join("fmt.sh"), tree.dir().join("later.sh")).unwrap();
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(index_errors(&report), 0, "{:?}", report.errors);
}
