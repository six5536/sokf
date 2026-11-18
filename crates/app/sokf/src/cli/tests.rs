//! The unit tests of the argument parser check each command's arguments
//! and the errors that clap reports.

use super::*;

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(args).unwrap()
}

#[test]
fn bare_invocation_has_no_command() {
    let cli = parse(&["sokf"]);
    assert!(cli.command.is_none());
    assert!(!cli.version);
    assert!(cli.repo_root.is_none());
}

#[test]
fn version_flag_has_both_spellings() {
    assert!(parse(&["sokf", "-V"]).version);
    assert!(parse(&["sokf", "--version"]).version);
    // The flag is global, so clap accepts it after a subcommand too.
    assert!(parse(&["sokf", "validate", "--version"]).version);
}

// sokf-tests: CLI-1_AC-1
#[test]
fn repo_root_is_global() {
    assert_eq!(
        parse(&["sokf", "--repo-root", "/r", "validate"])
            .repo_root
            .as_deref(),
        Some(std::path::Path::new("/r"))
    );
    assert_eq!(
        parse(&["sokf", "validate", "--repo-root", "/r"])
            .repo_root
            .as_deref(),
        Some(std::path::Path::new("/r"))
    );
}

// sokf-tests: CLI-1_AC-2
#[test]
fn no_cache_is_global() {
    assert!(!parse(&["sokf", "validate"]).no_cache);
    assert!(parse(&["sokf", "--no-cache", "validate"]).no_cache);
    assert!(parse(&["sokf", "validate", "--no-cache"]).no_cache);
    assert!(parse(&["sokf", "graph", "--no-cache"]).no_cache);
}

#[test]
fn init_takes_a_directory_a_name_a_description_and_json() {
    let Some(Command::Init(args)) = parse(&["sokf", "init"]).command else {
        panic!("expected init");
    };
    assert!(args.dir.is_none() && args.name.is_none() && args.description.is_none());
    assert!(!args.json);
    let Some(Command::Init(args)) = parse(&[
        "sokf",
        "init",
        "docs/kb",
        "--name",
        "project-kb",
        "--description",
        "Knowledge.",
        "--json",
        "--no-cache",
    ])
    .command
    else {
        panic!("expected init");
    };
    assert_eq!(args.dir.as_deref(), Some(std::path::Path::new("docs/kb")));
    assert_eq!(args.name.as_deref(), Some("project-kb"));
    assert_eq!(args.description.as_deref(), Some("Knowledge."));
    assert!(args.json);
    assert!(Cli::try_parse_from(["sokf", "init", "a", "b"]).is_err());
}

#[test]
fn harness_has_install_status_diff_and_the_hidden_hook() {
    let Some(Command::Harness(HarnessArgs {
        what: HarnessCommand::Install(args),
    })) = parse(&[
        "sokf",
        "harness",
        "install",
        "claude",
        "codex",
        "--scope",
        "user",
        "--force",
        "skills",
        "--force",
        "hooks,instructions",
        "--json",
        "--no-cache",
    ])
    .command
    else {
        panic!("expected harness install");
    };
    assert_eq!(args.names, ["claude", "codex"]);
    assert_eq!(args.scope, HarnessScopeArg::User);
    assert_eq!(args.force, ["skills", "hooks", "instructions"]);
    assert!(args.json);
    let Some(Command::Harness(HarnessArgs {
        what: HarnessCommand::Install(args),
    })) = parse(&["sokf", "harness", "install", "claude"]).command
    else {
        panic!("expected harness install");
    };
    assert!(args.force.is_empty() && !args.json);
    assert_eq!(args.scope, HarnessScopeArg::Project);
    let Some(Command::Harness(HarnessArgs {
        what: HarnessCommand::Diff(args),
    })) = parse(&["sokf", "harness", "diff", "claude", "--scope", "user"]).command
    else {
        panic!("expected harness diff");
    };
    assert_eq!(args.names, ["claude"]);
    assert_eq!(args.scope, HarnessScopeArg::User);
    let Some(Command::Harness(HarnessArgs {
        what: HarnessCommand::Status(args),
    })) = parse(&[
        "sokf", "harness", "status", "all", "--scope", "local", "--json",
    ])
    .command
    else {
        panic!("expected harness status");
    };
    assert_eq!(args.names, ["all"]);
    assert_eq!(args.scope, HarnessScopeArg::Local);
    assert!(args.json);
    let Some(Command::Harness(HarnessArgs {
        what: HarnessCommand::Hook(args),
    })) = parse(&["sokf", "harness", "hook", "claude", "stop"]).command
    else {
        panic!("expected harness hook");
    };
    assert_eq!((args.name.as_str(), args.hook.as_str()), ("claude", "stop"));
    for args in [
        vec!["sokf", "harness"],
        vec!["sokf", "harness", "install"],
        vec!["sokf", "harness", "status", "claude", "--force"],
        // `--force` takes a part, and `install` has no `--without`.
        vec!["sokf", "harness", "install", "claude", "--force"],
        vec!["sokf", "harness", "install", "claude", "--without", "hooks"],
        vec!["sokf", "harness", "diff"],
        vec!["sokf", "harness", "diff", "claude", "--json"],
        vec!["sokf", "harness", "hook", "claude"],
        vec!["sokf", "harness", "status", "claude", "--scope", "global"],
    ] {
        assert!(Cli::try_parse_from(&args).is_err(), "{args:?}");
    }
}

#[test]
fn validate_takes_paths_and_flags() {
    let Some(Command::Validate(args)) = parse(&["sokf", "validate"]).command else {
        panic!("expected validate");
    };
    assert_eq!(args.paths, [] as [std::path::PathBuf; 0]);
    assert!(!args.fix && !args.json && !args.warnings && args.query.is_none());

    let Some(Command::Validate(args)) = parse(&[
        "sokf",
        "validate",
        "a.md",
        "docs",
        "--fix",
        "--json",
        "--query",
        "errors",
        "--warnings",
    ])
    .command
    else {
        panic!("expected validate");
    };
    assert_eq!(args.paths.len(), 2);
    assert!(args.fix && args.json && args.warnings);
    assert_eq!(args.query.as_deref(), Some("errors"));
    // `--query` requires `--json` (CLI-4_AC-16).
    assert!(Cli::try_parse_from(["sokf", "validate", "--query", "errors"]).is_err());
}

#[test]
fn graph_takes_starts_bounds_and_one_form() {
    let Some(Command::Graph(args)) = parse(&["sokf", "graph"]).command else {
        panic!("expected graph");
    };
    assert_eq!(args.start, [] as [std::string::String; 0]);
    assert_eq!(args.depth, None);
    assert!(!args.all && !args.archived);
    assert!(args.selector.is_empty() && args.edge.is_empty() && args.to.is_empty());
    assert!(args.format.is_none() && !args.json && args.query.is_none() && !args.mermaid);
    let Some(Command::Graph(args)) = parse(&[
        "sokf",
        "graph",
        "planner",
        "knowledge/core",
        "--depth",
        "3",
        "-l",
        "a",
        "--selector",
        "b",
        "--edge",
        "c",
        "--to",
        "d",
        "--to",
        "e",
        "--json",
        "--query",
        "nodes",
        "--all",
        "--archived",
    ])
    .command
    else {
        panic!("expected graph");
    };
    assert_eq!(args.start, vec!["planner", "knowledge/core"]);
    assert_eq!(args.depth, Some(3));
    assert!(args.all && args.archived);
    assert_eq!(args.selector, vec!["a", "b"]);
    assert_eq!(args.edge, vec!["c"]);
    assert_eq!(args.to, vec!["d", "e"]);
    assert!(args.json);
    assert_eq!(args.query.as_deref(), Some("nodes"));
    let Some(Command::Graph(args)) = parse(&["sokf", "graph", "--format", "{id} {title}"]).command
    else {
        panic!("expected graph");
    };
    assert_eq!(args.format.as_deref(), Some("{id} {title}"));
    assert!(parse(&["sokf", "graph", "--mermaid"]).command.is_some());
    // Clap refuses each of these: more than one output form (CLI-7_AC-23),
    // `--query` without `--json` (CLI-7_AC-24), and a depth that is not a
    // number.
    for args in [
        vec!["sokf", "graph", "--json", "--mermaid"],
        vec!["sokf", "graph", "--format", "{id}", "--json"],
        vec!["sokf", "graph", "--format", "{id}", "--mermaid"],
        vec!["sokf", "graph", "--query", "nodes"],
        vec!["sokf", "graph", "--query", "nodes", "--mermaid"],
        vec!["sokf", "graph", "--query", "nodes", "--format", "{id}"],
        vec!["sokf", "graph", "--depth", "x"],
    ] {
        assert!(Cli::try_parse_from(&args).is_err(), "{args:?}");
    }
}

#[test]
fn locate_takes_one_or_more_ids() {
    let Some(Command::Locate(args)) = parse(&["sokf", "locate", "a", "b"]).command else {
        panic!("expected locate");
    };
    assert_eq!(args.ids, vec!["a", "b"]);
    assert!(!args.asset && !args.json);
    assert!(Cli::try_parse_from(["sokf", "locate"]).is_err());
    // `--asset` and `--json` exclude each other, as the `locate` interface of CLI-8 says.
    let Some(Command::Locate(args)) = parse(&["sokf", "locate", "--asset", "a"]).command else {
        panic!("expected locate");
    };
    assert!(args.asset);
    let Some(Command::Locate(args)) = parse(&["sokf", "locate", "--json", "a"]).command else {
        panic!("expected locate");
    };
    assert!(args.json);
    assert!(Cli::try_parse_from(["sokf", "locate", "--asset", "--json", "a"]).is_err());
}

#[test]
fn guide_takes_documents_states_json_and_a_query() {
    let Some(Command::Guide(args)) = parse(&[
        "sokf",
        "guide",
        "a",
        "/b.md",
        "--state",
        "state=open",
        "--state",
        "kind=bug",
    ])
    .command
    else {
        panic!("expected guide");
    };
    assert_eq!(args.documents, vec!["a", "/b.md"]);
    assert_eq!(
        args.state,
        vec![
            ("state".to_string(), "open".to_string()),
            ("kind".to_string(), "bug".to_string())
        ]
    );
    assert!(!args.json && args.query.is_none());
    let Some(Command::Guide(args)) =
        parse(&["sokf", "guide", "--json", "--query", "documents", "a"]).command
    else {
        panic!("expected guide");
    };
    assert!(args.json);
    assert_eq!(args.query.as_deref(), Some("documents"));
    for args in [
        vec!["sokf", "guide"],
        vec!["sokf", "guide", "--query", "documents", "a"],
        vec!["sokf", "guide", "--state", "state", "a"],
    ] {
        assert!(Cli::try_parse_from(&args).is_err(), "{args:?}");
    }
}

#[test]
fn info_has_grammar_and_formats() {
    assert!(matches!(
        parse(&["sokf", "info", "grammar"]).command,
        Some(Command::Info(InfoArgs {
            what: InfoCommand::Grammar
        }))
    ));
    let Some(Command::Info(InfoArgs {
        what: InfoCommand::Formats(args),
    })) = parse(&["sokf", "info", "formats", "--layer", "repo"]).command
    else {
        panic!("expected info formats");
    };
    assert_eq!(args.layer, Some(Layer::Repo));
    assert!(args.schema.is_none());
    // --layer and --schema exclude each other (CLI-5_AC-9).
    assert!(
        Cli::try_parse_from([
            "sokf", "info", "formats", "--layer", "repo", "--schema", "x"
        ])
        .is_err()
    );
    assert!(Cli::try_parse_from(["sokf", "info"]).is_err());
}

#[test]
fn new_takes_a_type_its_options_and_refuses_the_exclusive_pairs() {
    let Some(Command::New(args)) = parse(&[
        "sokf",
        "new",
        "Issue",
        "--slug",
        "retry",
        "--state",
        "kind=bug",
        "--state",
        "state=open",
        "--title",
        "T",
        "--write",
    ])
    .command
    else {
        panic!("expected new");
    };
    assert_eq!(args.type_name.as_deref(), Some("Issue"));
    assert_eq!(args.slug.as_deref(), Some("retry"));
    assert_eq!(
        args.state,
        vec![
            ("kind".to_string(), "bug".to_string()),
            ("state".to_string(), "open".to_string())
        ]
    );
    assert!(args.write && !args.json && !args.id_only);
    let Some(Command::New(args)) = parse(&["sokf", "new", "N", "--write", "--json"]).command else {
        panic!("expected new");
    };
    assert!(args.write && args.json);
    assert!(Cli::try_parse_from(["sokf", "new"]).is_err());
    for bad in [
        vec!["sokf", "new", "N", "--slug", "a", "--id", "b"],
        vec!["sokf", "new", "N", "--dir", "a", "--for", "b"],
        vec!["sokf", "new", "N", "--write", "--id-only"],
        vec!["sokf", "new", "N", "--json", "--id-only"],
        vec!["sokf", "new", "N", "--state", "nokey"],
    ] {
        assert!(Cli::try_parse_from(bad.clone()).is_err(), "{bad:?}");
    }
}

#[test]
fn completions_requires_a_known_shell() {
    let Some(Command::Completions(args)) = parse(&["sokf", "completions", "bash"]).command else {
        panic!("expected completions");
    };
    assert_eq!(args.shell, clap_complete::Shell::Bash);
    assert!(Cli::try_parse_from(["sokf", "completions", "cmd.exe"]).is_err());
    assert!(Cli::try_parse_from(["sokf", "completions"]).is_err());
}

#[test]
fn man_is_parsed_though_hidden_from_help() {
    assert!(matches!(
        parse(&["sokf", "man"]).command,
        Some(Command::Man)
    ));
}

#[test]
fn unknown_commands_and_flags_are_rejected() {
    assert!(Cli::try_parse_from(["sokf", "frobnicate"]).is_err());
    assert!(Cli::try_parse_from(["sokf", "--definitely-not-a-flag"]).is_err());
}
