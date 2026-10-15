//! This file holds the entry point of the `sokf` command-line tool.
// When the nightly coverage job sets `coverage_nightly` through cargo-llvm-cov,
// this line turns on the attribute that leaves untestable glue code out of the
// coverage report. Normal builds and tests use the stable toolchain, which
// ignores this line.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![warn(missing_docs)]

mod cli;
mod commands;
mod man;
mod output;
#[cfg(test)]
mod proptest_cases;
mod render;

use std::process::ExitCode;

use clap::{CommandFactory, Parser, error::ErrorKind};
use sokf_core::CacheMode;

use crate::cli::{Cli, Command, CompletionsArgs};

// sokf-implements: CLI-1_AC-11
// sokf-implements: CLI-1_AC-12
fn main() -> ExitCode {
    // `finish` exits with the code that `run` returns. On an error, it prints
    // `error: ` and the message and exits with code 2, unless the program that
    // reads the output closed the pipe. Such a program is `head`, or a pager
    // that the user quit early. A closed pipe isn't a failure of sokf, so sokf
    // prints nothing and exits with success, as ripgrep and other tools do.
    // Rust sets `SIGPIPE` to `SIG_IGN`, so a write to the closed pipe fails
    // with an `EPIPE` error and the `SIGPIPE` signal doesn't kill the process.
    agent_harness_adapter_core::cli::finish(run())
}

// sokf:begin run
fn run() -> sokf_core::Result<u8> {
    let cli = Cli::parse();
    if cli.version {
        // The output is `NAME VERSION`, where `NAME` is the name of the tool
        // and `VERSION` is its version, such as `sokf 1.2.3`. This form follows
        // the usual convention of command-line tools. So a bug report that
        // quotes the output identifies the tool and its version.
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(output::EXIT_OK);
    }
    let Some(command) = cli.command else {
        Cli::command().print_help().ok();
        return Ok(output::EXIT_OK);
    };
    // clap refuses a global option after the command beside the exclusive
    // `--json-schema`, but not one before it, which it parses on `sokf` itself.
    // This check refuses both, so that the position of a global option
    // changes nothing (CLI-1_AC-1).
    // sokf-implements: CLI-1_AC-16
    if let Some(schema) = command.json_schema() {
        if cli.repo_root.is_some() || cli.no_cache {
            Cli::command()
                .error(
                    ErrorKind::ArgumentConflict,
                    "the argument '--json-schema' cannot be used with one or more of the other specified arguments",
                )
                .exit();
        }
        return output::print_schema(schema);
    }
    let repo_root = cli.repo_root.as_deref();
    let cache = if cli.no_cache {
        CacheMode::Off
    } else {
        CacheMode::On
    };
    match command {
        // `init` reads no knowledge, so the cache mode does not apply to it.
        Command::Init(args) => commands::init(repo_root, args),
        Command::Harness(args) => commands::harness(repo_root, cache, args),
        Command::Validate(args) => commands::validate(repo_root, cache, args),
        Command::Info(args) => commands::info(repo_root, cache, args),
        Command::New(args) => commands::new(repo_root, cache, args),
        Command::Graph(args) => commands::graph(repo_root, cache, args),
        Command::Locate(args) => commands::locate(repo_root, cache, args),
        Command::Completions(args) => run_completions(args),
        Command::Man => run_man(),
    }
}
// sokf:end run

/// `run_completions` writes the shell completion script for `args.shell` to stdout.
fn run_completions(args: CompletionsArgs) -> sokf_core::Result<u8> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    // The script goes into a buffer, not straight to stdout. `generate` returns
    // `()` and calls `expect()` on each write, so a closed pipe makes it panic.
    // `panic = "abort"` then ends the process before `main` can see the error
    // and treat a broken pipe as success. `output::write_stdout` writes the
    // finished buffer and returns any write error, including a broken pipe, to
    // the caller.
    let mut buf = Vec::new();
    clap_complete::generate(args.shell, &mut cmd, name, &mut buf);
    output::write_stdout(&buf)?;
    Ok(output::EXIT_OK)
}

/// `run_man` writes the man page, in roff format, to stdout. The release
/// archives include this page.
fn run_man() -> sokf_core::Result<u8> {
    let buf = man::render().map_err(output::stdout_err)?;
    output::write_stdout(&buf)?;
    Ok(output::EXIT_OK)
}
