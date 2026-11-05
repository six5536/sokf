//! `sokf pack` runs its subcommands: `sync`, `status`, `list`, `remove`,
//! `diff`, and `export` (CLI-13).

use std::path::Path;

use sokf_core::{PackSyncOptions, discover_root};

use crate::{
    cli::{PackArgs, PackCommand},
    output,
};

/// `pack` runs the `pack` subcommand that `args` selects. Only `sync` uses
/// the network. `sync` exits with code 1 when a merge conflicts, and every
/// subcommand exits with code 2 when it refuses to run.
// sokf-implements: CLI-13_AC-10
pub fn pack(repo_root: Option<&Path>, args: PackArgs) -> sokf_core::Result<u8> {
    let (text, code) = match args.what {
        PackCommand::Sync(a) => {
            let (cwd, root) = output::root(repo_root)?;
            let report = sokf_core::pack_sync(
                &root,
                &PackSyncOptions {
                    source: a.source,
                    git_ref: a.git_ref,
                    path: a.path,
                    store: a.store,
                    adopt: a.adopt,
                    replace: a.replace,
                    cwd,
                    home: std::env::home_dir(),
                },
            )?;
            let code = if report.conflicted() {
                output::EXIT_ERRORS
            } else {
                output::EXIT_OK
            };
            (report.to_text(), code)
        }
        PackCommand::Status => {
            let (_, root) = output::root(repo_root)?;
            (sokf_core::pack_status(&root)?.to_text(), output::EXIT_OK)
        }
        PackCommand::List => {
            // Outside a repository, the list marks no pack.
            let root = discover_root(&output::cwd()?, repo_root).ok();
            let packs = sokf_core::pack_list(root.as_ref())?;
            (sokf_core::pack_list_text(&packs), output::EXIT_OK)
        }
        PackCommand::Remove(a) => {
            let (_, root) = output::root(repo_root)?;
            (
                sokf_core::pack_remove(&root, &a.name, std::env::home_dir())?.to_text(),
                output::EXIT_OK,
            )
        }
        PackCommand::Diff(a) => {
            let (_, root) = output::root(repo_root)?;
            (sokf_core::pack_diff(&root, &a.name)?, output::EXIT_OK)
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
