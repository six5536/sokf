//! End-to-end tests of the detection of the project's formatter: no command
//! starts a program of the repository unless `/.sokf/config.toml` sets
//! `detect = true` under `[format]`, the read-only commands and the check
//! that comes before a lookup included.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt as _};

use common::Fixture;

use crate::common;

/// `fixture` returns the clean fixture with an include of a CSS file, whose
/// copy a formatter re-laid, and a fake oxfmt, which the configuration file
/// at the root reveals and which logs each start to `oxfmt.log`.
fn fixture() -> Fixture {
    let f = Fixture::new("clean");
    let overview = fs::read_to_string(f.path("knowledge/overview.md")).unwrap();
    f.write(
        "knowledge/overview.md",
        &overview.replace(
            "<!-- sokf:include req-cli -->",
            "<!-- sokf:include /src/site.css -->\n<!-- /sokf:include -->\n\n<!-- sokf:include req-cli -->",
        ),
    );
    f.write("src/site.css", "a { color: red; }\n");
    let (report, _, code) = f.run(&["validate", "--fix"]);
    assert_eq!(code, 0, "{report}");
    let overview = fs::read_to_string(f.path("knowledge/overview.md")).unwrap();
    f.write(
        "knowledge/overview.md",
        &overview.replace("a { color: red; }\n", "a {\n  color: red;\n}\n"),
    );
    f.write(".oxfmtrc.json", "{}\n");
    let log = f.path("oxfmt.log");
    f.write(
        "node_modules/.bin/oxfmt",
        &format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", log.display()),
    );
    fs::set_permissions(
        f.path("node_modules/.bin/oxfmt"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let _ = fs::remove_dir_all(f.path(".sokf/cache"));
    f
}

// Without `format.detect`, no command starts the formatter that the
// repository reveals: neither `validate`, with or without `--fix`, nor a
// lookup, whose check before it compares the re-laid include. The finding
// states only what differs.
// sokf-tests: CONF-10_AC-26
#[test]
fn no_command_starts_a_formatter_without_detection() {
    let f = fixture();
    for args in [
        &["graph", "overview"][..],
        &["locate", "overview"],
        &["guide", "overview"],
        &["info", "grammar"],
        &["new", "Decision", "--id", "decision-pick-one"],
    ] {
        let (_, stderr, code) = f.run(args);
        assert_eq!(code, 0, "{args:?}: {stderr}");
        let _ = fs::remove_dir_all(f.path(".sokf/cache"));
    }
    let (report, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{report}");
    assert!(
        report.contains("is stale: `/src/site.css` differs from line 1 (SOKF_FORMAT_85)\n"),
        "{report}"
    );
    let (report, _, _) = f.run(&["--no-cache", "validate"]);
    assert!(report.contains("SOKF_FORMAT_85"), "{report}");
    assert!(
        !f.path("oxfmt.log").exists(),
        "a program of the repository ran: {}",
        fs::read_to_string(f.path("oxfmt.log")).unwrap_or_default()
    );
}

// With `detect = true`, the check starts the formatter that the repository
// reveals.
// sokf-tests: CONF-10_AC-2
#[test]
fn detection_starts_the_formatter_where_the_project_turns_it_on() {
    let f = fixture();
    f.write(".sokf/config.toml", "[format]\ndetect = true\n");
    let (report, _, _) = f.run(&["validate"]);
    assert!(report.contains("SOKF_FORMAT_85"), "{report}");
    assert!(!report.contains("detect = true"), "{report}");
    assert!(
        f.path("oxfmt.log").exists(),
        "the detected formatter starts"
    );
}
