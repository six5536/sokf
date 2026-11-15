//! End-to-end tests of a repository that an older sokf set up with packs:
//! sokf offers no command of the packs, refuses a `[[pack]]` entry of the
//! configuration, and warns at each file that the packs left.

use crate::common::{project, tree};

// The command `pack` is gone, so clap refuses it as a command that it
// doesn't know, and the help names no such command.
#[test]
fn sokf_offers_no_command_of_the_packs() {
    let f = project();
    for args in [&["pack", "sync"][..], &["pack", "status", "--check"]] {
        let (stdout, stderr, code) = f.run_here(args);
        assert_eq!((stdout.as_str(), code), ("", 2), "{args:?}: {stderr}");
        assert!(
            stderr.contains("unrecognized subcommand 'pack'"),
            "{stderr}"
        );
    }
    let (stdout, _, code) = f.run(&["--help"]);
    assert_eq!(code, 0);
    assert!(!stdout.contains("  pack "), "{stdout}");
}

// Every command that reads the configuration stops with exit code 2 on a
// `[[pack]]` entry, says that packs are no longer part of sokf, and writes
// no file.
// sokf-tests: CONF-1_AC-6
#[test]
fn a_pack_entry_of_the_configuration_stops_each_command() {
    let f = project();
    let config = std::fs::read_to_string(f.path(".sokf/config.toml")).unwrap_or_default();
    f.write(
        ".sokf/config.toml",
        &format!("{config}\n[[pack]]\nname = \"@sokf/sokf\"\nsource = \"@sokf/sokf\"\n"),
    );
    let before = tree(&f);
    for args in [
        &["validate"][..],
        &["validate", "--fix"],
        &["graph"],
        &["harness", "install", "claude"],
        &["harness", "status", "claude"],
        &["init", "/docs", "--scope", "test"],
    ] {
        let (stdout, stderr, code) = f.run(args);
        assert_eq!((stdout.as_str(), code), ("", 2), "{args:?}: {stderr}");
        assert!(
            stderr.contains("packs are no longer part of sokf: remove each `[[pack]]` entry"),
            "{args:?}: {stderr}"
        );
        assert!(!stderr.contains("pakf"), "{args:?}: {stderr}");
    }
    assert_eq!(tree(&f), before);
}

// The lock and the folder of the copies that an older sokf wrote each get a
// warning, which `validate` lists without `--warnings`, and the run still
// exits with code 0.
// sokf-tests: CLI-4_AC-104
// sokf-tests: CLI-4_AC-43
#[test]
fn validate_warns_at_each_file_that_the_packs_left() {
    let f = project();
    f.write(".sokf/packs.lock", "lock-version = 4\n");
    f.write(
        ".sokf/packs/@sokf/sokf/base/knowledge/schemas/issue.md",
        "# Issue\n",
    );
    let (out, err, code) = f.run(&["validate"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("/.sokf/packs.lock: warning: /.sokf/packs.lock is the lock of the packs"),
        "{out}"
    );
    assert!(
        out.contains("/.sokf/packs: warning: /.sokf/packs is the folder of the packs' copies"),
        "{out}"
    );
    assert!(out.ends_with("0 errors, 2 of 2 warnings listed\n"), "{out}");
    std::fs::remove_file(f.path(".sokf/packs.lock")).unwrap();
    std::fs::remove_dir_all(f.path(".sokf/packs")).unwrap();
    let (out, _, code) = f.run(&["validate"]);
    assert_eq!((code, out.as_str()), (0, "0 errors, 0 warnings\n"));
}
