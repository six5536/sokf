//! `sokf harness` runs its three sub-commands: `install`, `status`, and the hidden `hook`.

use std::{
    io::{self, Read},
    path::Path,
};

use agent_harness_adapter_core::hook::emit;
use sokf_core::{
    CacheMode, Error, HarnessDirs, HarnessOptions, HarnessReport, HarnessScope, discover_root,
    harness,
};

use crate::{
    cli::{HarnessArgs, HarnessCommand, HarnessScopeArg},
    output,
};

/// `harness` runs the `harness` sub-command that `args` selects.
// sokf-implements: CLI-3_AC-9
pub fn harness(
    repo_root: Option<&Path>,
    cache: CacheMode,
    args: HarnessArgs,
) -> sokf_core::Result<u8> {
    match args.what {
        HarnessCommand::Install(args) => {
            let scope = scope(args.scope);
            let dirs = dirs(repo_root, scope)?;
            let mut opts = HarnessOptions::new(names(args.names)?, scope).force(args.force);
            if !args.without.is_empty() {
                opts = opts.without(args.without);
            }
            let result = sokf_core::harness_install(&dirs, &opts)?;
            print(&result, args.json)
        }
        HarnessCommand::Status(args) => {
            let scope = scope(args.scope);
            let dirs = dirs(repo_root, scope)?;
            let result = sokf_core::harness_status(&dirs, &names(args.names)?, scope)?;
            print(&result, args.json)
        }
        HarnessCommand::Hook(args) => {
            hook(&output::cwd()?, repo_root, cache, &args.name, &args.hook)
        }
    }
}

/// `scope` returns the adapter's scope of `--scope`.
fn scope(arg: HarnessScopeArg) -> HarnessScope {
    match arg {
        HarnessScopeArg::Project => HarnessScope::Project,
        HarnessScopeArg::User => HarnessScope::User,
        HarnessScopeArg::Local => HarnessScope::Local,
    }
}

/// `names` returns the harness names, which clap requires unless `--json-schema` is
/// present, which clap has then already checked.
fn names(names: Vec<String>) -> sokf_core::Result<Vec<String>> {
    if names.is_empty() {
        return Err(Error::Internal("clap requires HARNESS".into()));
    }
    Ok(names)
}

/// `dirs` returns the repository and the home directory. The project and local scopes need
/// the repository; the user scope reads it only when there is one.
// sokf-implements: CLI-3_AC-66
fn dirs(repo_root: Option<&Path>, scope: HarnessScope) -> sokf_core::Result<HarnessDirs> {
    let repo = match scope {
        HarnessScope::User => discover_root(&output::cwd()?, repo_root).ok(),
        _ => Some(output::root(repo_root)?.1),
    };
    Ok(HarnessDirs {
        repo,
        home: std::env::home_dir(),
    })
}

/// `print` writes the report to stdout, as text lines or, with `json`, as one JSON object.
// sokf-implements: CLI-3_AC-11
// sokf-implements: CLI-3_AC-19
// sokf-implements: CLI-3_AC-20
fn print(result: &HarnessReport, json: bool) -> sokf_core::Result<u8> {
    let buf = if json {
        output::json_line(result)?
    } else {
        result.to_text().into_bytes()
    };
    output::write_stdout(&buf)?;
    Ok(output::EXIT_OK)
}

/// `hook` answers a hook of the harness `name` for the event `event`, writes the answer in
/// the harness's form to stdout, and exits with the harness's code. When the harness or the
/// event is unknown, the command exits with code 2. Outside a repository, it lets the agent
/// stop. On any other failure, `hook` writes the error to stderr and nothing to stdout and
/// exits with code 1, so that the failure never blocks the harness.
// sokf-implements: CLI-3_AC-29
fn hook(
    cwd: &Path,
    repo_root: Option<&Path>,
    cache: CacheMode,
    name: &str,
    event: &str,
) -> sokf_core::Result<u8> {
    // sokf-implements: CLI-3_AC-31
    let call = harness::hook_call(name, event)?;
    let mut input = String::new();
    let answer = match std::io::stdin().read_to_string(&mut input) {
        Err(e) => Err(Error::io("<stdin>", e)),
        // sokf-implements: CLI-3_AC-68
        Ok(_) => match discover_root(cwd, repo_root) {
            Err(Error::NoRepository) => Ok(harness::Answer::Allow { stderr: None }),
            root => root.and_then(|root| harness::hook(&root, &call, &input, cache)),
        },
    };
    // `emit` writes an answer's report to stderr and its form to stdout, and returns the
    // harness's exit code. On any other failure, it writes the error to stderr and returns 1.
    // sokf-implements: CLI-3_AC-25
    // sokf-implements: CLI-3_AC-26
    // sokf-implements: CLI-3_AC-30
    emit(
        call.harness.as_ref(),
        call.event,
        answer,
        &mut io::stdout().lock(),
        &mut io::stderr(),
    )
    .map_err(output::stdout_err)
}
