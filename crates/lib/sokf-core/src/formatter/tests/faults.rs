//! The tests of the failures of the formatter: a text that fails or hangs
//! alone, a failure of the environment that the samples show, a language
//! with no sample, and the forms that a failed batch keeps.

use std::fs;

use super::*;

// A text on which the formatter lasts longer than the limit alone, after
// its sample passed, is a failure that the store keeps, and the texts after
// it are unsettled. The batch, the samples, and the text alone run, and a
// later run starts no formatter, because the pair's other text fails on it
// for good.
// sokf-tests: CONF-10_AC-8
// sokf-tests: CLI-4_AC-73
// sokf-tests: CLI-4_AC-74
// sokf-tests: CLI-4_AC-91
#[cfg(unix)]
#[test]
fn a_text_that_hangs_alone_is_a_lasting_failure() {
    let (root, mut config) = fake("slow");
    config.timeout = std::time::Duration::from_secs(1);
    let log = root.as_path().join("runs.log");
    let layout = Session::new(&root, true).settle(&config, &[pair("json", "SLOW  x", "SLOW x")]);
    let warning = layout
        .warning("json", "SLOW  x", "SLOW x")
        .expect("a warning");
    assert!(
        warning.message.contains("failed on a text"),
        "{}",
        warning.message
    );
    assert!(
        layout.unsettled().is_empty(),
        "the failure settles the pair"
    );
    assert_eq!(runs(&log), 3, "the batch, the samples, the first text");
    let start = std::time::Instant::now();
    let again = Session::new(&root, true).settle(&config, &[pair("json", "SLOW  x", "SLOW x")]);
    assert!(again.warning("json", "SLOW  x", "SLOW x").is_some());
    assert_eq!(runs(&log), 3);
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    // A text that the hang left untried is unsettled.
    let layout = Session::new(&root, true).settle(
        &config,
        &[
            pair("json", "SLOW  y", "SLOW  z"),
            pair("json", "c  d", "c d"),
        ],
    );
    assert_eq!(
        layout.unsettled(),
        &std::collections::BTreeSet::from([at()])
    );
    assert!(!layout.same("json", "c  d", "c d"));
    let warning = layout.warning("json", "c  d", "c d").expect("a warning");
    assert!(
        warning.message.contains("longer than 1 second on"),
        "{}",
        warning.message
    );
}

// A stored hang keeps the limit that it reached: a later run with a limit
// no longer than that one reuses the failure and starts no formatter, and a
// run with a longer limit formats the text again and keeps its form.
// sokf-tests: CLI-4_AC-90
// sokf-tests: CLI-4_AC-91
#[cfg(unix)]
#[test]
fn a_longer_limit_formats_a_stored_hang_again() {
    let (root, mut config) = fake("longer");
    let log = root.as_path().join("runs.log");
    let late = [pair("json", "LATE  x", "LATE x")];
    config.timeout = std::time::Duration::from_secs(1);
    let layout = Session::new(&root, true).settle(&config, &late);
    assert!(layout.warning("json", "LATE  x", "LATE x").is_some());
    assert!(
        layout.unsettled().is_empty(),
        "the hang is a lasting failure"
    );
    assert_eq!(runs(&log), 3, "the batch, the samples, the first text");
    for limit in [1000, 500] {
        config.timeout = std::time::Duration::from_millis(limit);
        let again = Session::new(&root, true).settle(&config, &late);
        assert!(again.warning("json", "LATE  x", "LATE x").is_some());
        assert_eq!(runs(&log), 3, "a limit of {limit} ms reuses the hang");
    }
    config.timeout = std::time::Duration::from_secs(4);
    let longer = Session::new(&root, true).settle(&config, &late);
    assert!(longer.same("json", "LATE  x", "LATE x"));
    assert!(longer.warning("json", "LATE  x", "LATE x").is_none());
    assert_eq!(runs(&log), 4, "one batch formats both texts");
    // The form replaces the hang, so a shorter limit reuses the form.
    config.timeout = std::time::Duration::from_secs(1);
    let after = Session::new(&root, true).settle(&config, &late);
    assert!(after.same("json", "LATE  x", "LATE x"));
    assert_eq!(runs(&log), 4);
}

// A hang stops the formatting of the rest of the command even when it is
// the last text that runs alone, so a later pass of the same command starts
// no formatter, and its texts are unsettled for the hang.
// sokf-tests: CLI-4_AC-74
#[cfg(unix)]
#[test]
fn a_hang_of_the_last_text_stops_the_command() {
    let (root, mut config) = fake("last");
    config.timeout = std::time::Duration::from_secs(1);
    let log = root.as_path().join("runs.log");
    let session = Session::new(&root, true);
    let layout = session.settle(&config, &[pair("json", "a  b", "LATE  x")]);
    assert!(layout.warning("json", "a  b", "LATE  x").is_some());
    assert_eq!(runs(&log), 4, "the batch, the samples, each text alone");
    let later = session.settle(&config, &[pair("json", "c  d", "c d")]);
    assert_eq!(runs(&log), 4, "the hang stopped the command");
    let warning = later.warning("json", "c  d", "c d").expect("a warning");
    assert!(
        warning.message.contains("longer than 1 second on"),
        "{}",
        warning.message
    );
}

// A pair whose two texts both fail, such as a region that doesn't parse on
// its own, is a failure on those texts. After the batch and the samples,
// each text runs alone until one fails for good, even the text of a batch
// of one, because a first run can fail for a cold start.
// sokf-tests: CLI-4_AC-49
// sokf-tests: CLI-4_AC-76
#[cfg(unix)]
#[test]
fn a_pair_whose_texts_both_fail_is_a_failure_on_its_texts() {
    let (root, config) = fake("both");
    let session = Session::new(&root, true);
    let layout = session.settle(&config, &[pair("json", "BAD  1", "BAD 1")]);
    assert!(layout.unsettled().is_empty());
    let warning = layout
        .warning("json", "BAD  1", "BAD 1")
        .expect("a warning");
    assert!(
        warning.message.contains("failed on a text"),
        "{}",
        warning.message
    );
    let log = root.as_path().join("runs.log");
    // The second text needs no run alone, because the first one fails for
    // good.
    assert_eq!(
        runs(&log),
        3,
        "the batch, the samples, the first text alone"
    );
    Session::new(&root, true).settle(&config, &[pair("json", "a  b", "a b")]);
    assert_eq!(runs(&log), 4);
    Session::new(&root, true).settle(&config, &[pair("json", "a  b", "BAD  3")]);
    assert_eq!(runs(&log), 7, "the batch, the samples, the text alone");
    // The failures last: a pair with a failed text starts no formatter.
    Session::new(&root, true).settle(&config, &[pair("json", "BAD  1", "BAD  4")]);
    assert_eq!(runs(&log), 7);
}

// A formatter whose environment fails it, as dprint offline with no plugins
// in its cache, fails on its sample too: the run stores no failure, leaves
// the file unsettled, and formats nothing more, and once the environment
// works, the texts compare by their form.
// sokf-tests: CLI-4_AC-70
// sokf-tests: CLI-4_AC-71
#[cfg(unix)]
#[test]
fn a_failed_sample_stores_no_failure() {
    let (root, config) = fake("environment");
    let log = root.as_path().join("runs.log");
    write(root.as_path(), "broken", "");
    let session = Session::new(&root, true);
    let pairs = [pair("json", "a  b", "a b"), pair("md", "c  d", "c d")];
    let layout = session.settle(&config, &pairs);
    assert!(!layout.same("json", "a  b", "a b"));
    let warning = layout.warning("json", "a  b", "a b").expect("a warning");
    assert!(
        warning.message.contains("its sample"),
        "{}",
        warning.message
    );
    assert_eq!(
        layout.unsettled(),
        &std::collections::BTreeSet::from([at()])
    );
    assert_eq!(runs(&log), 2, "the batch and the samples");
    // The session formats nothing more in this command.
    let later = session.settle(&config, &[pair("json", "e  f", "e f")]);
    assert!(!later.unsettled().is_empty());
    assert_eq!(runs(&log), 2);
    fs::remove_file(root.as_path().join("broken")).unwrap();
    let layout = Session::new(&root, true).settle(&config, &pairs);
    assert!(layout.same("json", "a  b", "a b") && layout.same("md", "c  d", "c d"));
    assert!(layout.unsettled().is_empty());
}

// After a failed batch, the forms of the texts that the formatter formatted,
// which are the texts that it changed, stay, though the sample of another
// language fails.
// sokf-tests: CLI-4_AC-75
// sokf-tests: CLI-4_AC-70
#[cfg(unix)]
#[test]
fn the_forms_of_a_failed_batch_stay() {
    let (root, config) = fake("partial");
    write(root.as_path(), "broken-md", "");
    let pairs = [pair("json", "a  b", "a\tb"), pair("md", "c  d", "c d")];
    let layout = Session::new(&root, true).settle(&config, &pairs);
    assert!(layout.same("json", "a  b", "a\tb"));
    assert!(!layout.same("md", "c  d", "c d"));
    assert!(layout.warning("md", "c  d", "c d").is_some());
    fs::remove_file(root.as_path().join("broken-md")).unwrap();
    let log = root.as_path().join("runs.log");
    let before = runs(&log);
    let layout = Session::new(&root, true).settle(&config, &pairs);
    assert!(layout.same("md", "c  d", "c d"));
    assert_eq!(runs(&log), before + 1, "the JSON texts come from the store");
}

// A failure in a language that has no sample may be the environment's, so
// the store keeps none, and each run formats the text again.
// sokf-tests: CLI-4_AC-72
#[cfg(unix)]
#[test]
fn a_failure_in_a_language_with_no_sample_is_unsettled() {
    let (root, config) = fake("no-sample");
    let log = root.as_path().join("runs.log");
    for i in 1..=2 {
        let layout = Session::new(&root, true).settle(&config, &[pair("rs", "BAD  x", "fn  a")]);
        assert_eq!(
            layout.unsettled(),
            &std::collections::BTreeSet::from([at()])
        );
        let warning = layout.warning("rs", "BAD  x", "fn  a").expect("a warning");
        assert!(warning.message.contains("no sample"), "{}", warning.message);
        // The first run formats `fn  a` in the batch and runs `BAD  x`
        // alone, and the next runs `BAD  x` in a batch and alone, as no
        // store holds it.
        assert_eq!(runs(&log), [2, 4][i - 1]);
    }
}

// A batch of one text that fails on its first run, such as a formatter
// that fetches its plugins on its first start, runs the text again after
// the samples, and stores its form.
// sokf-tests: CLI-4_AC-71
// sokf-tests: CLI-4_AC-73
#[cfg(unix)]
#[test]
fn a_batch_of_one_text_runs_it_again_after_the_samples() {
    let (root, config) = fake("cold");
    let log = root.as_path().join("runs.log");
    Session::new(&root, true).settle(&config, &[pair("json", "c  d", "c d")]);
    assert_eq!(runs(&log), 1);
    // The next run of the formatter fails and removes the flag, as a cold
    // start that warms the formatter's cache would.
    let script = fs::read_to_string(root.as_path().join("fmt.sh")).unwrap();
    let cold = root.as_path().join("cold");
    write(root.as_path(), "cold", "");
    write(
        root.as_path(),
        "fmt.sh",
        &script.replacen(
            "status=0\n",
            &format!(
                "status=0\nif [ -e '{c}' ]; then rm '{c}'; exit 1; fi\n",
                c = cold.display()
            ),
            1,
        ),
    );
    // `c  d` comes from the store, so the batch holds `a  b` alone.
    let layout = Session::new(&root, true).settle(&config, &[pair("json", "c  d", "a  b")]);
    assert!(layout.warning("json", "c  d", "a  b").is_none());
    assert_eq!(runs(&log), 4, "the batch, the samples, the text alone");
    let again = Session::new(&root, true).settle(&config, &[pair("json", "a  b", "a b")]);
    assert!(again.same("json", "a  b", "a b"));
    assert_eq!(runs(&log), 5, "the form of `a  b` lasts");
}

// A pair that fails for good and a pair left unsettled in one pass each get
// the warning that says why.
// sokf-tests: CLI-4_AC-49
#[cfg(unix)]
#[test]
fn each_pair_gets_the_warning_of_its_own_failure() {
    let (root, config) = fake("mixed");
    let layout = Session::new(&root, true).settle(
        &config,
        &[
            pair("json", "BAD  1", "BAD 1"),
            pair("rs", "BAD  2", "BAD 2"),
        ],
    );
    let failed = layout
        .warning("json", "BAD  1", "BAD 1")
        .expect("a warning");
    assert!(
        failed.message.contains("failed on a text that"),
        "{}",
        failed.message
    );
    let unsettled = layout.warning("rs", "BAD  2", "BAD 2").expect("a warning");
    assert!(
        unsettled.message.contains("no sample"),
        "{}",
        unsettled.message
    );
}

// A limit of many years runs the formatter without overflowing the clock.
// sokf-tests: CONF-10_AC-8
#[cfg(unix)]
#[test]
fn a_huge_limit_runs_the_formatter() {
    let (root, mut config) = fake("huge");
    config.timeout = std::time::Duration::from_secs(i64::MAX.unsigned_abs());
    let layout = Session::new(&root, true).settle(&config, &[pair("json", "a  b", "a b")]);
    assert!(layout.same("json", "a  b", "a b"));
}

// A session that a failed sample stopped still reads the store, so a later
// pass of the same command compares the pairs whose forms the store holds.
// sokf-tests: CLI-4_AC-70
#[cfg(unix)]
#[test]
fn a_stopped_session_reads_the_store() {
    let (root, config) = fake("stopped");
    let known = [pair("json", "a  b", "a\tb")];
    assert!(
        Session::new(&root, true)
            .settle(&config, &known)
            .same("json", "a  b", "a\tb")
    );
    write(root.as_path(), "broken", "");
    let session = Session::new(&root, true);
    let first = session.settle(&config, &[pair("json", "c  d", "c d")]);
    assert!(!first.unsettled().is_empty());
    let later = session.settle(&config, &known);
    assert!(later.same("json", "a  b", "a\tb"));
    assert!(later.unsettled().is_empty());
}

// Each unsettled text keeps the reason of its own fault, and a hang stops
// the formatting of the rest of the command, so one command lasts the
// limit at most twice.
// sokf-tests: CLI-4_AC-72
// sokf-tests: CLI-4_AC-74
#[cfg(unix)]
#[test]
fn each_unsettled_text_keeps_its_reason() {
    let (root, mut config) = fake("reasons");
    config.timeout = std::time::Duration::from_secs(1);
    let log = root.as_path().join("runs.log");
    let session = Session::new(&root, true);
    let layout = session.settle(
        &config,
        &[
            pair("rs", "BAD  r", "fn  r"),
            pair("json", "SLOW  s", "SLOW  t"),
            pair("json", "c  d", "e  f"),
        ],
    );
    let warning = |ext: &str, a: &str, b: &str| layout.warning(ext, a, b).unwrap().message;
    assert!(warning("rs", "BAD  r", "fn  r").contains("no sample"));
    assert!(warning("json", "c  d", "e  f").contains("longer than 1 second on"));
    let before = runs(&log);
    let later = session.settle(&config, &[pair("json", "g  h", "g h")]);
    assert_eq!(runs(&log), before, "the hang stopped the command");
    assert!(!later.unsettled().is_empty());
}

// A run after the batch that doesn't start leaves the other texts
// unsettled, and the forms of the batch stay.
// sokf-tests: CLI-4_AC-75
#[cfg(unix)]
#[test]
fn a_run_after_the_batch_that_does_not_start_keeps_the_forms() {
    let (root, config) = fake("vanishes");
    // The batch removes the formatter, so no later run starts.
    let script = fs::read_to_string(root.as_path().join("fmt.sh")).unwrap();
    write(
        root.as_path(),
        "fmt.sh",
        &script.replacen("exit $status\n", "rm \"$0\"\nexit $status\n", 1),
    );
    let layout = Session::new(&root, true).settle(
        &config,
        &[
            pair("json", "a  b", "a\tb"),
            pair("json", "BAD  1", "BAD 1"),
        ],
    );
    assert!(layout.same("json", "a  b", "a\tb"));
    let warning = layout
        .warning("json", "BAD  1", "BAD 1")
        .expect("a warning");
    assert!(
        warning.message.contains("couldn't run the formatter"),
        "{}",
        warning.message
    );
}

// A text in a language with no sample that lasts longer than the limit
// alone is unsettled as a hang, which stops the command.
// sokf-tests: CLI-4_AC-72
// sokf-tests: CLI-4_AC-74
#[cfg(unix)]
#[test]
fn a_hang_in_a_language_with_no_sample_stops_the_command() {
    let (root, mut config) = fake("hang-no-sample");
    config.timeout = std::time::Duration::from_secs(1);
    let log = root.as_path().join("runs.log");
    let session = Session::new(&root, true);
    let layout = session.settle(&config, &[pair("rs", "SLOW  r", "SLOW  s")]);
    let warning = layout
        .warning("rs", "SLOW  r", "SLOW  s")
        .expect("a warning");
    assert!(
        warning.message.contains("longer than 1 second on"),
        "{}",
        warning.message
    );
    let before = runs(&log);
    session.settle(&config, &[pair("json", "g  h", "g h")]);
    assert_eq!(runs(&log), before, "the hang stopped the command");
}

// A configured command whose `--version` lasts longer than the limit
// counts as a formatter that doesn't start, so the command spends the limit
// on it once.
// sokf-tests: CONF-10_AC-8
#[cfg(unix)]
#[test]
fn a_version_that_hangs_stops_the_command() {
    let (root, mut config) = fake("slow-version");
    config.timeout = std::time::Duration::from_secs(1);
    let script = fs::read_to_string(root.as_path().join("fmt.sh")).unwrap();
    write(
        root.as_path(),
        "fmt.sh",
        &script.replacen("then cat", "then sleep 5; cat", 1),
    );
    let start = std::time::Instant::now();
    let layout = Session::new(&root, true).settle(&config, &[pair("json", "a  b", "a b")]);
    assert!(start.elapsed() < std::time::Duration::from_secs(3));
    let warning = layout.warning("json", "a  b", "a b").expect("a warning");
    assert!(
        warning.message.contains("doesn't start"),
        "{}",
        warning.message
    );
    assert_eq!(runs(&root.as_path().join("runs.log")), 0);
}
