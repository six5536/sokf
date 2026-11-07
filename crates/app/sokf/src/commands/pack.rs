//! `sokf pack` runs its subcommands: `sync`, `add`, `status`, `accept`,
//! `list`, `remove`, `diff`, and `export` (CLI-13).

use std::path::Path;

use sokf_core::{
    PackAcceptOptions, PackAddOptions, PackDiffOptions, PackSyncOptions, PackSyncReport,
    discover_root,
};

use crate::{
    cli::{PackArgs, PackCommand},
    output,
};

/// `pack` runs the `pack` subcommand that `args` selects. Only `sync` and
/// `add` use the network. `sync`, `add`, and `remove` exit with code 1 when
/// a merge conflicts, `status --check` exits with code 1 when a file of a
/// pack differs from its base, and every subcommand exits with code 2 when
/// it refuses to run.
// sokf-implements: CLI-13_AC-10
pub fn pack(repo_root: Option<&Path>, args: PackArgs) -> sokf_core::Result<u8> {
    let (text, code) = match args.what {
        PackCommand::Sync(a) => {
            let (_, root) = output::root(repo_root)?;
            let report = sokf_core::pack_sync(
                &root,
                &PackSyncOptions {
                    name: a.name,
                    keep_existing: a.keep_existing,
                    home: std::env::home_dir(),
                },
            )?;
            synced(&report)
        }
        PackCommand::Add(a) => {
            let (cwd, root) = output::root(repo_root)?;
            let report = sokf_core::pack_add(
                &root,
                &PackAddOptions {
                    source: a.source,
                    git_ref: a.git_ref,
                    path: a.path,
                    store: a.store,
                    keep_existing: a.keep_existing,
                    cwd,
                    home: std::env::home_dir(),
                },
            )?;
            synced(&report)
        }
        PackCommand::Status(a) => {
            let (_, root) = output::root(repo_root)?;
            let status = sokf_core::pack_status(&root)?;
            // A stale pack doesn't fail the check, because an upgrade of
            // sokf alone shouldn't fail a project's checks.
            // sokf-implements: CLI-13_AC-49
            // sokf-implements: CLI-13_AC-50
            let code = if a.check && status.drifted() {
                output::EXIT_ERRORS
            } else {
                output::EXIT_OK
            };
            (status.to_text(), code)
        }
        PackCommand::Accept(a) => {
            let (cwd, root) = output::root(repo_root)?;
            let report = sokf_core::pack_accept(
                &root,
                &PackAcceptOptions {
                    paths: a.paths,
                    note: a.note,
                    withdraw: a.withdraw,
                    cwd,
                },
            )?;
            (report.to_text(), output::EXIT_OK)
        }
        PackCommand::List => {
            // Outside a repository, the list marks no pack.
            let root = discover_root(&output::cwd()?, repo_root).ok();
            let packs = sokf_core::pack_list(root.as_ref())?;
            (sokf_core::pack_list_text(&packs), output::EXIT_OK)
        }
        PackCommand::Remove(a) => {
            let (_, root) = output::root(repo_root)?;
            let report = sokf_core::pack_remove(&root, &a.name, std::env::home_dir())?;
            let code = if report.conflicted() {
                output::EXIT_ERRORS
            } else {
                output::EXIT_OK
            };
            (report.to_text(), code)
        }
        PackCommand::Diff(a) => {
            let (cwd, root) = output::root(repo_root)?;
            let patch = sokf_core::pack_diff(
                &root,
                &PackDiffOptions {
                    name: a.name,
                    accepted: a.accepted,
                    new: a.new,
                    cwd,
                    home: std::env::home_dir(),
                },
            )?;
            (patch, output::EXIT_OK)
        }
        PackCommand::Export(a) => {
            let dir = output::cwd()?.join(&a.dir);
            let written = sokf_core::pack_export(&a.name, &dir)?;
            let text = written
                .iter()
                .map(|rel| format!("wrote {}\n", dir.join(rel).display()))
                .collect();
            (text, output::EXIT_OK)
        }
    };
    output::write_stdout(text.as_bytes())?;
    Ok(code)
}

/// `synced` returns the text of the report of `sync` or `add`, and exits
/// with code 1 when a merge conflicts.
fn synced(report: &PackSyncReport) -> (String, u8) {
    let code = if report.conflicted() {
        output::EXIT_ERRORS
    } else {
        output::EXIT_OK
    };
    (report.to_text(), code)
}
