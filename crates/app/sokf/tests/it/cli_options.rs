//! End-to-end tests of behaviour every command shares: read-only commands
//! write no file, and every command stops when the tool rejects its
//! configuration file (contract-cli §2, contract-config §1).

use common::Fixture;

use crate::common;

// `graph`, `locate`, `info`, and `new` without `--write` change no file in
// the tree. They write only the cache under `/.sokf`.
// sokf-tests: CLI-9_AC-3
#[test]
fn the_commands_that_only_read_write_no_file() {
    let f = Fixture::new("clean");
    let before = f.files();
    for args in [
        vec!["graph"],
        vec!["graph", "--json"],
        vec!["graph", "--mermaid"],
        vec!["locate", "overview"],
        vec!["info", "grammar"],
        vec!["info", "formats"],
        vec!["new", "Issue", "--slug", "retry-logic"],
        vec!["new", "Issue", "--slug", "retry-logic", "--json"],
        vec!["new", "Issue", "--slug", "retry-logic", "--id-only"],
    ] {
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!(code, 0, "`sokf {}` -> {stdout}{stderr}", args.join(" "));
        assert_eq!(f.files(), before, "`sokf {}`", args.join(" "));
    }
}

// When the tool rejects a configuration file, the command exits with code 2
// and prints `error: <message>` on stderr, and the tree stays as it was.
// sokf-tests: CONF-1_AC-3
// sokf-tests: CONF-1_AC-4
// sokf-tests: CONF-2_AC-6
// sokf-tests: CONF-3_AC-5
// sokf-tests: CLI-9_AC-1
#[test]
fn a_refused_configuration_exits_2_and_writes_nothing() {
    for (text, needle) in [
        ("not toml [", "/.sokf/config.toml: "),
        ("[knowledge]\nrootz = []\n", "unknown field `rootz`"),
        ("[other]\nx = 1\n", "unknown field `other`"),
        (
            "[knowledge]\nroots = [\"/missing\"]\n",
            "knowledge root `/missing` is not a directory",
        ),
        (
            "[repository]\nexclude = [\"vendor/**\"]\n",
            "repository.exclude glob `vendor/**` is not /-rooted",
        ),
    ] {
        let f = Fixture::new("clean");
        f.write(".sokf/config.toml", text);
        let before = f.files();
        for args in [vec!["validate", "--fix"], vec!["graph"]] {
            let (stdout, stderr, code) = f.run(&args);
            assert_eq!(code, 2, "{text:?}: {stderr}");
            assert_eq!(stdout, "", "{text:?}");
            assert!(
                stderr.starts_with("error: /.sokf/config.toml: "),
                "{text:?}: {stderr}"
            );
            assert!(stderr.contains(needle), "{text:?}: {stderr}");
            assert_eq!(f.files(), before, "{text:?}");
        }
    }
}
