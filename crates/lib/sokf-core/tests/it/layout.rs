//! The comparison by meaning: a generated block that a formatter re-laid
//! stays current, `--fix` leaves it and writes a stale one as its source
//! holds it, and a run that changes nothing starts no formatter. same-same
//! compares JSON, Markdown, YAML, and TypeScript by their meaning, with no
//! formatter, and CSS through the formatter of `[format]`. A small fake
//! formatter stands for the project's: it removes the whitespace of a file,
//! fails on a file that holds `BAD`, hangs on one that holds `SLOW`, and
//! logs each run.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt as _};

use sokf_core::{CacheMode, Report, report_text};

use crate::common::{
    TempTree,
    incremental::{run, set},
};

/// The JSON source that the overview includes, as no formatter left it.
const SOURCE: &str = "{\"a\": [1, 2], \"b\": {\"c\": true}}\n";

/// The CSS source that the overview includes, a format that same-same
/// compares through the formatter.
const STYLE: &str = "a { color: red; }\n";

/// The overview, which includes the requirement's body and the sources.
const OVERVIEW: &str = "---\ntype: Overview\nid: overview\ntitle: Overview\ndescription: Reads the requirement.\nlinks:\n  - rel: references\n    to: req-cli\n---\n\n# Overview\n\nThe CLI requirement is [CLI-1][sokf:#cli-1] in [the criteria][sokf:req-cli#acceptance-criteria], governed by [its schema][sokf:schema-requirement].\n\n<!-- sokf:include req-cli -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/data.json -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/site.css -->\n<!-- /sokf:include -->\n";

/// `tree` returns the clean fixture with the overview, the sources, and a
/// fake formatter, which `[format]` names when `configured` holds, after
/// one `--fix` that fills the blocks.
fn tree(configured: bool) -> TempTree {
    let tree = TempTree::clean_with(&[
        ("knowledge/overview.md", OVERVIEW),
        ("src/data.json", SOURCE),
        ("src/site.css", STYLE),
    ]);
    let log = tree.dir().join("runs.log");
    let script = format!(
        "#!/bin/sh\necho run >> '{log}'\nstatus=0\nfor f in \"$@\"; do\n  if grep -q BAD \"$f\"; then status=1; continue; fi\n  if grep -q SLOW \"$f\"; then sleep 5; fi\n  tr -d ' \\n\\t' < \"$f\" > \"$f.tmp\" && mv \"$f.tmp\" \"$f\"\ndone\nexit $status\n",
        log = log.display(),
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

/// The copy of the JSON source that the project's formatter re-lays.
const RELAID: &str = "{\n  \"a\": [1,\n    2],\n  \"b\": {\n    \"c\": true\n  }\n}\n";

/// The copy of the CSS source that the project's formatter re-lays.
const RELAID_STYLE: &str = "a {\n  color: red;\n}\n";

// A block that a formatter re-laid is current, and only the pair of a
// format that same-same doesn't know starts the formatter, once. The second
// run starts no formatter, and `--fix` leaves the re-laid blocks as they
// are.
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
    relay(&tree, "knowledge/overview.md", STYLE, RELAID_STYLE);
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
    assert_eq!(runs(&tree), base + 1, "one run formats both texts of CSS");
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
    // Without the cache, the run compares again and finds the same.
    let cold = tree.validate(false, CacheMode::Off);
    assert!(stale(&cold).is_empty(), "{:?}", stale(&cold));
}

// Under `--no-cache`, the comparison writes nothing under
// `/.sokf/cache/same-same/`, and the store of an older sokf,
// `/.sokf/cache/formatted.sqlite`, stays as it was.
// sokf-tests: CACHE-2_AC-8
// sokf-tests: CACHE-2_AC-10
#[test]
fn the_cache_of_same_same_follows_the_mode() {
    let tree = tree(true);
    let older = tree.dir().join(".sokf/cache/formatted.sqlite");
    fs::write(&older, "an older store").unwrap();
    let folder = tree.dir().join(".sokf/cache/same-same");
    let _ = fs::remove_dir_all(&folder);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    relay(&tree, "knowledge/overview.md", STYLE, RELAID_STYLE);
    let report = tree.validate(false, CacheMode::Off);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
    assert!(!folder.exists(), "no file under --no-cache");
    let report = tree.validate(false, CacheMode::On);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
    assert!(folder.is_dir());
    assert_eq!(fs::read_to_string(&older).unwrap(), "an older store");
}

// A block whose meaning differs is stale, with the line of the source that
// differs, and `--fix` writes the source's text as it stands, with no run
// of the formatter.
// sokf-tests: CLI-4_AC-45
// sokf-tests: CLI-4_AC-50
// sokf-tests: CLI-4_AC-110
// sokf-tests: CLI-4_AC-115
#[test]
fn a_block_whose_meaning_differs_is_stale() {
    let tree = tree(true);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    tree.write(
        "src/data.json",
        "{\"a\": [1,\n  3], \"b\": {\"c\": true}}\n",
    );
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(
        stale(&report)[0].contains("`/src/data.json` differs from line 2"),
        "{:?}",
        stale(&report)
    );
    let base = runs(&tree);
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    assert!(
        tree.read("knowledge/overview.md")
            .contains("```json\n{\"a\": [1,\n  3], \"b\": {\"c\": true}}\n```"),
        "{}",
        tree.read("knowledge/overview.md")
    );
    assert_eq!(runs(&tree), base, "`--fix` runs no formatter");
    let second = tree.validate(true, CacheMode::On);
    assert!(second.fixes.is_empty(), "{:?}", second.fixes);
}

// Without a formatter, a re-laid block of JSON is current, and one of CSS
// is stale, with no warning.
// sokf-tests: CLI-4_AC-48
#[test]
fn without_a_formatter_an_unknown_format_compares_byte_for_byte() {
    let tree = tree(false);
    relay(&tree, "knowledge/overview.md", SOURCE, RELAID);
    relay(&tree, "knowledge/overview.md", STYLE, RELAID_STYLE);
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(
        stale(&report)[0].contains("site.css"),
        "{:?}",
        stale(&report)
    );
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(runs(&tree), 0);
}

// A text that the command fails on compares byte for byte, and the run
// reports one warning, which the text report lists without `--warnings`.
// sokf-tests: CLI-4_AC-49
// sokf-tests: CLI-4_AC-43
// sokf-tests: CLI-4_AC-2
#[test]
fn a_failure_of_the_formatter_warns_once() {
    let tree = tree(true);
    let bad = "a { content: \"BAD\"; }\n";
    tree.write("src/site.css", bad);
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    let again = tree.validate(true, CacheMode::On);
    assert!(again.fixes.is_empty(), "{:?}", again.fixes);
    relay(
        &tree,
        "knowledge/overview.md",
        bad,
        "a {\n  content: \"BAD\";\n}\n",
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
        text.contains("warning: the formatter fmt.sh fails on `/src/site.css` (CLI-4_AC-49)"),
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

// A command that the repository names but that isn't installed makes the
// run compare byte for byte and warn, and the next run checks the overview
// again, until the command starts.
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
    relay(&tree, "knowledge/overview.md", STYLE, RELAID_STYLE);
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
    // Once the command is installed, the next run checks the overview
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

// A run whose command doesn't start keeps the findings of every document
// that needed no formatter, so the next run checks the overview alone, and
// checks it again until the command starts.
// sokf-tests: CACHE-2_AC-9
#[test]
fn a_formatter_that_does_not_start_keeps_the_other_documents() {
    let tree = tree(false);
    relay(&tree, "knowledge/overview.md", STYLE, RELAID_STYLE);
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

// A command that runs past the limit of `format.timeout` is stopped, and
// the pair compares byte for byte with a warning. same-same keeps nothing
// of a command, so the next run tries it again, as the command can have
// run past its limit only on a busy machine.
// sokf-tests: CONF-10_AC-8
#[test]
fn a_command_that_hangs_is_stopped_at_its_limit() {
    let tree = tree(true);
    tree.write(
        ".sokf/config.toml",
        "[format]\ncommand = [\"./fmt.sh\", \"{files}\"]\ntimeout = 1\n",
    );
    let slow = "a { content: \"SLOW\"; }\n";
    tree.write("src/site.css", slow);
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    let again = tree.validate(true, CacheMode::On);
    assert!(again.fixes.is_empty(), "{:?}", again.fixes);
    relay(
        &tree,
        "knowledge/overview.md",
        slow,
        "a {\n  content: \"SLOW\";\n}\n",
    );
    let start = std::time::Instant::now();
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
    assert!(
        report.warnings.iter().any(|w| w.criterion == "CLI-4_AC-49"),
        "{:?}",
        report.warnings
    );
    let base = runs(&tree);
    let again = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&again).len(), 1, "{:?}", stale(&again));
    assert!(runs(&tree) > base, "the next run tries the command again");
}

// An index that a formatter re-laid is current with no formatter at all,
// because same-same compares it as Markdown.
// sokf-tests: CLI-4_AC-46
#[test]
fn a_re_laid_index_needs_no_formatter() {
    let tree = tree(false);
    relay(
        &tree,
        "knowledge/index.md",
        "[sokf:overview]: /knowledge/overview.md",
        "[sokf:overview]:  /knowledge/overview.md",
    );
    let report = tree.validate(false, CacheMode::On);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
}

// The detected formatter doesn't start for a declaration file that it
// re-laid, nor for a sorted `package.json`, a sorted list of imports, or a
// `tsconfig.json` without its trailing comma.
// sokf-tests: CLI-4_AC-44
// sokf-tests: CLI-4_AC-109
// sokf-tests: CLI-4_AC-113
// sokf-tests: CONF-10_AC-2
#[test]
fn a_re_laid_include_of_a_known_format_starts_no_formatter() {
    let declaration = "export declare function f(a: string): number;\n";
    let package = "{\"version\": \"1.0.0\", \"name\": \"x\"}\n";
    let imports =
        "import { b } from \"./b\";\nimport { a } from \"./a\";\n\nexport const c = a + b;\n";
    let tsconfig = "{\"compilerOptions\": {\"strict\": true,},}\n";
    let doc = "---\ntype: Overview\nid: sources\ntitle: Sources\ndescription: Shows the sources.\n---\n\n# Sources\n\nThe sources, as the formatter leaves them.\n\n<!-- sokf:include /src/index.d.ts -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/package.json -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/main.ts -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/tsconfig.json -->\n<!-- /sokf:include -->\n";
    let tree = TempTree::clean_with(&[
        ("knowledge/sources.md", doc),
        ("src/index.d.ts", declaration),
        ("src/package.json", package),
        ("src/main.ts", imports),
        ("src/tsconfig.json", tsconfig),
    ]);
    // A fake oxfmt, which the configuration file at the root reveals, logs
    // each start.
    let log = tree.dir().join("oxfmt.log");
    tree.write(".oxfmtrc.json", "{}\n");
    tree.write(".sokf/config.toml", "[format]\ndetect = true\n");
    tree.write(
        "node_modules/.bin/oxfmt",
        &format!("#!/bin/sh\necho run >> '{}'\nexit 1\n", log.display()),
    );
    fs::set_permissions(
        tree.dir().join("node_modules/.bin/oxfmt"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let report = tree.validate(true, CacheMode::On);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    for (from, to) in [
        (
            declaration,
            "export declare function f(\n  a: string,\n): number;\n",
        ),
        (
            package,
            "{\n  \"name\": \"x\",\n  \"version\": \"1.0.0\"\n}\n",
        ),
        (
            imports,
            "import { a } from \"./a\";\nimport { b } from \"./b\";\n\nexport const c = a + b;\n",
        ),
        (tsconfig, "{ \"compilerOptions\": { \"strict\": true } }\n"),
    ] {
        relay(&tree, "knowledge/sources.md", from, to);
    }
    let report = tree.validate(false, CacheMode::On);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(!log.exists(), "no run of the formatter");
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.fixes.is_empty(), "{:?}", fixed.fixes);
}

/// The YAML file whose region an include holds.
const SETTINGS: &str =
    "server:\n  # sokf:begin flags\n  enabled: no\n  port: 80\n  # sokf:end flags\n";

/// The document that includes the region of the YAML file.
const FLAGS: &str = "---\ntype: Overview\nid: flags\ntitle: Flags\ndescription: Shows the flags.\n---\n\n# Flags\n\nThe flags of the server.\n\n<!-- sokf:include /src/settings.yaml#flags -->\n<!-- /sokf:include -->\n";

// A region of YAML whose copy lost the region's indentation is current, and
// one whose copy quotes a value that a YAML 1.1 reader takes otherwise is
// stale, unless `format.same` names `yaml11-value`.
// sokf-tests: CLI-4_AC-111
// sokf-tests: CLI-4_AC-114
// sokf-tests: CONF-10_AC-22
#[test]
fn a_region_of_yaml_compares_by_its_meaning() {
    let tree = TempTree::clean_with(&[
        ("knowledge/flags.md", FLAGS),
        ("src/settings.yaml", SETTINGS),
    ]);
    let report = tree.validate(true, CacheMode::On);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    relay(
        &tree,
        "knowledge/flags.md",
        "  enabled: no\n  port: 80\n",
        "enabled: no\nport: 80\n",
    );
    let report = tree.validate(false, CacheMode::On);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
    relay(
        &tree,
        "knowledge/flags.md",
        "enabled: no",
        "enabled: \"no\"",
    );
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    tree.write(".sokf/config.toml", "[format]\nsame = [\"yaml11-value\"]\n");
    let report = tree.validate(false, CacheMode::On);
    assert!(stale(&report).is_empty(), "{:?}", stale(&report));
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.fixes.is_empty(), "{:?}", fixed.fixes);
    tree.write(".sokf/config.toml", "");
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    assert!(
        tree.read("knowledge/flags.md")
            .contains("```yaml\n  enabled: no\n  port: 80\n```"),
        "{}",
        tree.read("knowledge/flags.md")
    );
    let again = tree.validate(true, CacheMode::On);
    assert!(again.fixes.is_empty(), "{:?}", again.fixes);
}

// A definition block whose destination or label a person edited is
// stale, also where the label differs only in its case, which Markdown
// reads as the same label.
// sokf-tests: CLI-4_AC-112
#[test]
fn an_edited_destination_is_stale() {
    let tree = tree(false);
    relay(
        &tree,
        "knowledge/overview.md",
        "[sokf:schema-requirement]: ",
        "[sokf:Schema-Requirement]: ",
    );
    let report = tree.validate(false, CacheMode::On);
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.criterion == "SOKF_FORMAT_80"),
        "{:?}",
        report.errors
    );
    let fixed = tree.validate(true, CacheMode::On);
    assert!(fixed.errors.is_empty(), "{:?}", fixed.errors);
    assert!(
        tree.read("knowledge/overview.md")
            .contains("[sokf:schema-requirement]: ")
    );
    let again = tree.validate(true, CacheMode::On);
    assert!(again.fixes.is_empty(), "{:?}", again.fixes);
    relay(
        &tree,
        "knowledge/overview.md",
        "[sokf:#cli-1]: /knowledge/req-cli.md#cli-1",
        "[sokf:#cli-1]: /knowledge/req-cli.md#cli-2",
    );
    let report = tree.validate(false, CacheMode::On);
    assert!(
        report
            .errors
            .iter()
            .any(|f| f.criterion == "SOKF_FORMAT_80"),
        "{:?}",
        report.errors
    );
}

// Without `format.detect`, sokf looks for no formatter, so a re-laid include
// of CSS is stale with no run of the formatter that the repository's
// configuration file reveals, and its finding states only what differs;
// with `detect = true`, same-same starts that formatter.
// sokf-tests: CONF-10_AC-2
// sokf-tests: CONF-10_AC-26
#[test]
fn detection_is_off_unless_the_project_turns_it_on() {
    let tree = tree(false);
    let log = tree.dir().join("oxfmt.log");
    tree.write(".oxfmtrc.json", "{}\n");
    tree.write(
        "node_modules/.bin/oxfmt",
        &format!("#!/bin/sh\necho run >> '{}'\nexit 1\n", log.display()),
    );
    fs::set_permissions(
        tree.dir().join("node_modules/.bin/oxfmt"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    relay(&tree, "knowledge/overview.md", STYLE, RELAID_STYLE);
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(
        stale(&report)[0].ends_with("`/src/site.css` differs from line 1"),
        "{:?}",
        stale(&report)
    );
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(!log.exists(), "no formatter starts without `format.detect`");
    tree.write(".sokf/config.toml", "[format]\ndetect = true\n");
    let report = tree.validate(false, CacheMode::On);
    assert_eq!(stale(&report).len(), 1, "{:?}", stale(&report));
    assert!(
        !stale(&report)[0].contains("detect = true"),
        "{:?}",
        stale(&report)
    );
    assert!(log.exists(), "the detected formatter starts");
}
