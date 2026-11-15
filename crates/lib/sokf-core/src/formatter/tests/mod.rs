//! The tests of the formatter: a small fake formatter checks the batch, the
//! store, and the failures, and each formatter that sokf detects checks that
//! sokf's settings give a text and the form that the formatter's own
//! defaults gave it the same form. A test of a detected
//! formatter that isn't installed says so and passes.

// Only the tests that run a shell script, which are Unix-only, touch files.
#[cfg(unix)]
use std::fs;
use std::path::Path;

use super::*;
use crate::test_support::{fresh_dir, write};

// Every test of the failures runs a fake formatter that is a shell script.
#[cfg(unix)]
mod faults;

/// `pair` returns the pair of `expected` and `actual` in the language `ext`,
/// which the file `/k/a.md` holds.
fn pair(ext: &str, expected: &str, actual: &str) -> Pair {
    Pair {
        path: at(),
        ext: ext.into(),
        expected: expected.into(),
        actual: actual.into(),
    }
}

/// `at` returns the path of the file that holds the pairs of the tests.
fn at() -> crate::RepoPath {
    crate::RepoPath::parse("/k/a.md").unwrap()
}

/// `runs` returns the number of runs of the fake formatter that `log`
/// records.
#[cfg(unix)]
fn runs(log: &Path) -> usize {
    fs::read_to_string(log).map_or(0, |t| t.lines().filter(|l| l.starts_with("run")).count())
}

/// `fake` writes a fake formatter into a new repository named by `tag`, and
/// returns the repository and the `[format]` that runs it. The formatter
/// collapses each run of whitespace in a file to one space, fails on a file
/// that holds `BAD`, sleeps 5 seconds on each file that holds `SLOW` and 2
/// seconds once in a run with a file that holds `LATE`, fails on every file
/// while the file `broken` exists, and on every Markdown file while
/// `broken-md` exists, answers `--version` with the text of `version.txt`,
/// and logs each run to `runs.log`.
#[cfg(unix)]
fn fake(tag: &str) -> (RepoRoot, FormatConfig) {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = fresh_dir(&format!("formatter-{tag}"));
    let log = dir.join("runs.log");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = --version ]; then cat '{v}'; exit 0; fi\necho \"run $# $(env | grep -c '^GIT_')\" >> '{log}'\nif [ -e '{d}/broken' ]; then exit 1; fi\nstatus=0\nfor f in \"$@\"; do\n  case \"$f\" in --*) continue;; esac\n  case \"$f\" in *.md) if [ -e '{d}/broken-md' ]; then status=1; continue; fi;; esac\n  if grep -q BAD \"$f\"; then status=1; continue; fi\n  if grep -q SLOW \"$f\"; then sleep 5; fi\n  if grep -q LATE \"$f\" && [ -z \"$late\" ]; then late=1; sleep 2; fi\n  tr -s ' \\n\\t' ' ' < \"$f\" | sed 's/ $//' > \"$f.tmp\" && mv \"$f.tmp\" \"$f\"\ndone\nexit $status\n",
        v = dir.join("version.txt").display(),
        d = dir.display(),
        log = log.display(),
    );
    write(&dir, "fmt.sh", &script);
    write(&dir, "version.txt", "1.0\n");
    fs::set_permissions(dir.join("fmt.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    let config = FormatConfig {
        command: Some(vec![
            "./fmt.sh".into(),
            "--settings={settings}".into(),
            "{files}".into(),
        ]),
        settings: Some(r#"{"width":1}"#.into()),
        ..FormatConfig::default()
    };
    (RepoRoot::new(dir), config)
}

// The pairs of one pass go to the formatter in one run, and a pass with no
// pair starts no formatter.
// sokf-tests: CACHE-2_AC-5
// sokf-tests: CACHE-3_AC-13
// sokf-tests: CLI-4_AC-45
// sokf-tests: CONF-10_AC-1
#[cfg(unix)]
#[test]
fn one_run_formats_every_text_of_a_pass() {
    let (root, config) = fake("batch");
    let log = root.as_path().join("runs.log");
    let session = Session::new(&root, true);
    assert!(session.settle(&config, &[]).same.is_empty());
    assert_eq!(runs(&log), 0);
    let pairs = [
        pair("json", "{\"a\":  1}\n", "{\"a\": 1}"),
        pair("md", "a\n\nb\n", "a b"),
        pair("json", "{\"a\": 1}\n", "{\"a\": 2}\n"),
    ];
    let layout = session.settle(&config, &pairs);
    assert_eq!(runs(&log), 1);
    assert!(layout.same("json", "{\"a\":  1}\n", "{\"a\": 1}"));
    assert!(layout.same("md", "a\n\nb\n", "a b"));
    assert!(!layout.same("json", "{\"a\": 1}\n", "{\"a\": 2}\n"));
    assert!(
        layout
            .warning("json", "{\"a\":  1}\n", "{\"a\": 1}")
            .is_none()
    );
    // The run got the files as arguments, and no variable of git's.
    let line = fs::read_to_string(&log).unwrap();
    assert!(line.starts_with("run 7 0"), "{line}");
    // The session holds what it formatted, so the same pairs start nothing.
    session.settle(&config, &pairs);
    assert_eq!(runs(&log), 1);
}

// A later run reuses the store's forms, and a new version of the formatter
// formats each text again.
// sokf-tests: CACHE-2_AC-6
// sokf-tests: CACHE-2_AC-7
#[cfg(unix)]
#[test]
fn the_store_keeps_each_form_for_one_version() {
    let (root, config) = fake("store");
    let log = root.as_path().join("runs.log");
    let pairs = [pair("json", "{\"a\":  1}\n", "{\"a\":\n 1}\n")];
    assert!(Session::new(&root, true).settle(&config, &pairs).same(
        "json",
        &pairs[0].expected,
        &pairs[0].actual
    ));
    assert_eq!(runs(&log), 1);
    assert!(
        root.as_path()
            .join(".sokf/cache")
            .join(FORMATTED_FILE)
            .is_file()
    );
    assert!(Session::new(&root, true).settle(&config, &pairs).same(
        "json",
        &pairs[0].expected,
        &pairs[0].actual
    ));
    assert_eq!(runs(&log), 1);
    write(root.as_path(), "version.txt", "2.0\n");
    Session::new(&root, true).settle(&config, &pairs);
    assert_eq!(runs(&log), 2);
}

// Without the cache, the forms stay in memory.
// sokf-tests: CACHE-2_AC-8
#[cfg(unix)]
#[test]
fn without_the_cache_no_file_is_written() {
    let (root, config) = fake("memory");
    let pairs = [pair("json", "a  b", "a b")];
    assert!(
        Session::new(&root, false)
            .settle(&config, &pairs)
            .same("json", "a  b", "a b")
    );
    assert!(!root.as_path().join(".sokf/cache").exists());
}

// A text that the formatter fails on compares byte for byte, with one
// warning at the file that names the formatter, and the texts that it
// formats still compare by their form.
// sokf-tests: CLI-4_AC-49
#[cfg(unix)]
#[test]
fn a_failure_on_a_text_compares_it_byte_for_byte() {
    let (root, config) = fake("failure");
    let session = Session::new(&root, true);
    let pairs = [pair("json", "BAD  x", "BAD x"), pair("json", "a  b", "a b")];
    let layout = session.settle(&config, &pairs);
    assert!(!layout.same("json", "BAD  x", "BAD x"));
    assert!(layout.same("json", "a  b", "a b"));
    let warning = layout
        .warning("json", "BAD  x", "BAD x")
        .expect("a warning");
    assert_eq!(warning.path.as_str(), "/.sokf/config.toml");
    assert_eq!(warning.criterion, WARNING);
    assert!(
        warning.message.contains("failed on a text"),
        "{}",
        warning.message
    );
    assert!(layout.unsettled().is_empty());
}

// A formatter that doesn't start makes every pair compare byte for byte,
// with a warning, and leaves the file of each pair unsettled.
// sokf-tests: CLI-4_AC-49
// sokf-tests: CACHE-2_AC-9
#[test]
fn a_formatter_that_does_not_start_compares_byte_for_byte() {
    let dir = fresh_dir("formatter-missing");
    let root = RepoRoot::new(dir);
    let config = FormatConfig {
        command: Some(vec!["./no-such-formatter".into(), "{files}".into()]),
        ..FormatConfig::default()
    };
    let session = Session::new(&root, true);
    let layout = session.settle(&config, &[pair("json", "a  b", "a b")]);
    assert!(!layout.same("json", "a  b", "a b"));
    let warning = layout.warning("json", "a  b", "a b").expect("a warning");
    assert!(
        warning.message.contains("doesn't start"),
        "{}",
        warning.message
    );
    assert_eq!(
        layout.unsettled(),
        &std::collections::BTreeSet::from([at()])
    );
}

// No formatter means byte comparison, with no warning.
// sokf-tests: CLI-4_AC-48
#[test]
fn no_formatter_compares_byte_for_byte() {
    let root = RepoRoot::new(fresh_dir("formatter-none"));
    let session = Session::new(&root, true);
    let layout = session.settle(&FormatConfig::default(), &[pair("json", "a  b", "a b")]);
    assert!(!layout.same("json", "a  b", "a b"));
    assert!(layout.warning("json", "a  b", "a b").is_none());
    assert!(layout.same("json", "a b", "a b"));
    assert!(layout.unsettled().is_empty());
}

// The command of the formatter reads no input and gets no variable that
// points git at a repository.
// sokf-tests: CONF-10_AC-7
#[test]
fn the_command_clears_the_variables_of_git() {
    let cmd = run::command(Path::new("fmt"));
    let envs: Vec<_> = cmd.get_envs().collect();
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            assert!(envs.contains(&(key.as_os_str(), None)), "{key:?}");
        }
    }
}

/// A `Sample` is a text in a language, as its source holds it.
struct Sample {
    ext: &'static str,
    text: &'static str,
}

/// `SAMPLES` lists the texts that each detected formatter is tested on: a
/// JSON Schema whose source no formatter touched, a region of YAML that its
/// place indents, Markdown with wrapped prose and a fence, and TypeScript.
const SAMPLES: [Sample; 4] = [
    Sample {
        ext: "json",
        text: "{\n  \"type\": \"object\",\n  \"required\": [\n    \"errors\",\n    \"warnings\"\n  ],\n  \"properties\": {\n    \"errors\": {\n      \"description\": \"Every error, ordered by path, then by line, which is a long line that a formatter with its default width wraps.\",\n      \"type\": \"array\"\n    }\n  }\n}\n",
    },
    Sample {
        ext: "yaml",
        text: "  heading:\n    template: '${n:int}. ${title}'\n    key:   n\n    examples: [ \"8. Links\",\n      \"9. Assets\" ]\n",
    },
    Sample {
        ext: "md",
        text: "# Title\n\nA paragraph that a writer wrapped\nby hand, over two lines, and that runs on for long enough that a formatter with a narrow width wraps it again at another place.\n\n* one\n* two\n\n```json\n{\"a\": [1,\n2]}\n```\n",
    },
    Sample {
        ext: "ts",
        text: "const report = { errors: [], warnings: ['a long warning that runs past the default width of a formatter', 'b'] }\n",
    },
];

/// `TIMEOUT` is the limit of the runs of a real formatter in the tests.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// `form` returns the form of `outcome`, which a test expects the formatter
/// to have given.
fn form(outcome: &run::Outcome) -> String {
    match outcome {
        run::Outcome::Form(form) => form.clone(),
        other => panic!("no form: {other:?}"),
    }
}

/// `tag` returns the tag of the fence of a sample in the language `ext`.
fn tag(ext: &str) -> &str {
    match ext {
        "ts" => "typescript",
        other => other,
    }
}

/// `as_in_markdown` returns the text that the project's formatter, run with
/// `formatter` at `program`, leaves of the sample: for Markdown the whole
/// text, and for another language the code of a fence that holds it in a
/// Markdown file, as an include holds it.
fn as_in_markdown(formatter: &Formatter, program: &Path, sample: &Sample) -> String {
    if sample.ext == "md" {
        let batch = run::format(
            formatter,
            program,
            &[("md", sample.text)],
            &[vec![]],
            TIMEOUT,
        )
        .expect("it starts");
        return form(&batch[0]);
    }
    let open = format!("```{}\n", tag(sample.ext));
    let doc = format!("# Include\n\n{open}{}```\n", sample.text);
    let batch =
        run::format(formatter, program, &[("md", &doc)], &[vec![]], TIMEOUT).expect("it starts");
    let form = form(&batch[0]);
    let start = form.find(&open).expect("the fence stays") + open.len();
    let end = start + form[start..].find("```").expect("the fence closes");
    form[start..end].to_string()
}

/// `check_preset` checks the preset `name` against the formatter that is
/// installed: it formats a Markdown file that holds each sample with the
/// formatter's defaults and with a narrow width, as a project's formatter
/// would, and checks that sokf's settings give each result and the sample
/// the same form. It checks the samples whose extensions `exts` lists, or
/// every sample that the formatter formats.
fn check_preset(
    name: &str,
    config_file: &str,
    (config, defaults, narrow): (&str, &str, &str),
    exts: Option<&[&str]>,
) {
    let dir = fresh_dir(&format!("preset-real-{name}"));
    write(&dir, config_file, config);
    let root = RepoRoot::new(dir);
    let formatter = find(&root, &FormatConfig::default()).expect("the preset");
    assert_eq!(formatter.program, name);
    let Some(program) = run::program(&root, &formatter.program) else {
        eprintln!("skipped: {name} is not installed");
        return;
    };
    let samples: Vec<&Sample> = SAMPLES
        .iter()
        .filter(|s| formatter.formats(s.ext) && exts.is_none_or(|e| e.contains(&s.ext)))
        .collect();
    // Each sample of a language that the test checks is a fixed point of
    // the formatter under sokf's settings, so that a failure on it is the
    // environment's (D-9 of issue-175).
    let exts: Vec<&str> = sample::extensions()
        .filter(|e| formatter.formats(e) && exts.is_none_or(|x| x.contains(e)))
        .collect();
    let texts: Vec<(&str, &str)> = exts
        .iter()
        .map(|e| (*e, sample::sample(e).unwrap_or_default()))
        .collect();
    let batch = run::format(
        &formatter,
        &program,
        &texts,
        &vec![vec![]; texts.len()],
        TIMEOUT,
    )
    .expect("it starts");
    for ((ext, text), outcome) in texts.iter().zip(&batch) {
        assert_eq!(&form(outcome), text, "{name} changed the sample of {ext}");
    }
    let session = Session::new(&root, false);
    for project in [defaults, narrow] {
        let as_project = Formatter {
            settings: project.to_string(),
            ..formatter.clone()
        };
        let pairs: Vec<Pair> = samples
            .iter()
            .map(|s| pair(s.ext, s.text, &as_in_markdown(&as_project, &program, s)))
            .collect();
        let layout = session.settle(&FormatConfig::default(), &pairs);
        for p in &pairs {
            assert!(
                layout.same(&p.ext, &p.expected, &p.actual),
                "{name}, {project}: {} and {} differ",
                p.expected,
                p.actual
            );
        }
    }
}

// sokf-tests: CONF-10_AC-2
#[test]
fn the_settings_of_oxfmt_undo_its_layout() {
    check_preset(
        "oxfmt",
        ".oxfmtrc.json",
        (
            "{}",
            "{}",
            r#"{"printWidth": 40, "proseWrap": "always", "singleQuote": true, "objectWrap": "preserve"}"#,
        ),
        None,
    );
}

// sokf-tests: CONF-10_AC-2
#[test]
fn the_settings_of_prettier_undo_its_layout() {
    check_preset(
        "prettier",
        ".prettierrc.json",
        (
            "{}",
            "{}",
            r#"{"printWidth": 40, "proseWrap": "always", "singleQuote": true, "objectWrap": "preserve"}"#,
        ),
        None,
    );
}

// sokf-tests: CONF-10_AC-2
#[test]
fn the_settings_of_biome_undo_its_layout() {
    check_preset(
        "biome",
        "biome.json",
        (
            "{}",
            r#"{"vcs": {"enabled": false}, "files": {"ignoreUnknown": true}}"#,
            r#"{"vcs": {"enabled": false}, "files": {"ignoreUnknown": true}, "formatter": {"lineWidth": 40}, "javascript": {"formatter": {"quoteStyle": "single"}}}"#,
        ),
        None,
    );
}

// The configuration loads the plugins of JSON and Markdown alone, so the
// test checks those two languages. dprint's Markdown plugin takes the
// indentation off a fence of a language that no plugin formats, which a
// formatter of that language alone can't undo.
// sokf-tests: CONF-10_AC-2
#[test]
#[ignore = "long: dprint fetches its plugins over the network"]
fn the_settings_of_dprint_undo_its_layout() {
    let plugins = r#"["npm:@dprint/json@0.26.0", "npm:@dprint/markdown@0.29.0"]"#;
    check_preset(
        "dprint",
        "dprint.json",
        (
            &format!(r#"{{"plugins": {plugins}}}"#),
            &format!(r#"{{"plugins": {plugins}}}"#),
            &format!(
                r#"{{"lineWidth": 40, "markdown": {{"textWrap": "always"}}, "plugins": {plugins}}}"#
            ),
        ),
        Some(&["json", "md"]),
    );
}

// The store of the cache records the formatter that the repository reveals,
// with the version of its package, so that a new one checks every document
// again.
// sokf-tests: CACHE-1_AC-6
#[cfg(unix)]
#[test]
fn the_fingerprint_follows_the_formatter_and_its_version() {
    let dir = fresh_dir("formatter-fingerprint");
    let root = RepoRoot::new(dir.clone());
    assert_eq!(fingerprint(&root), 0);
    write(&dir, ".prettierrc", "{}");
    write(
        &dir,
        "node_modules/prettier/package.json",
        r#"{"name": "prettier", "version": "3.0.0"}"#,
    );
    write(&dir, "node_modules/prettier/bin/prettier.cjs", "");
    fs::create_dir_all(dir.join("node_modules/.bin")).unwrap();
    std::os::unix::fs::symlink(
        "../prettier/bin/prettier.cjs",
        dir.join("node_modules/.bin/prettier"),
    )
    .unwrap();
    let first = fingerprint(&root);
    assert_ne!(first, 0);
    write(
        &dir,
        "node_modules/prettier/package.json",
        r#"{"name": "prettier", "version": "3.1.0"}"#,
    );
    assert_ne!(fingerprint(&root), first);
}

// pnpm installs a program as a script, and the version then comes from the
// package under `node_modules` at the root.
// sokf-tests: CACHE-1_AC-6
#[test]
fn the_version_of_a_package_comes_from_node_modules() {
    let dir = fresh_dir("formatter-pnpm");
    let root = RepoRoot::new(dir.clone());
    write(
        &dir,
        "node_modules/.bin/prettier",
        "#!/bin/sh\nexec node ../prettier/bin/prettier.cjs \"$@\"\n",
    );
    write(
        &dir,
        "node_modules/prettier/package.json",
        r#"{"name": "prettier", "version": "3.2.0"}"#,
    );
    let program = dir.join("node_modules/.bin/prettier");
    assert_eq!(
        run::package_version(&root, &program, "prettier").as_deref(),
        Some("3.2.0")
    );
    assert_eq!(run::package_version(&root, &program, "oxfmt"), None);
}

// A report gathers the outputs of several runs, and keeps one warning that
// the formatter failed, the one that says why it doesn't start first.
// sokf-tests: CLI-4_AC-49
#[test]
fn a_report_keeps_one_warning_of_the_formatter() {
    let at = crate::RepoPath::parse("/.prettierrc").unwrap();
    let warn = |m: &str| Finding::warning(at.clone(), None, WARNING, m);
    let mut report = crate::Report::default();
    report.extend(vec![
        warn("the formatter Prettier failed on a text"),
        warn("the formatter Prettier doesn't start: it is missing"),
    ]);
    let report = report.finish();
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].message.contains("doesn't start"));
}
