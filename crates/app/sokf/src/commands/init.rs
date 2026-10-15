//! This module runs the `sokf init` command.

use std::{io::Write, path::Path};

use sokf_core::InitOptions;

use crate::{cli::InitArgs, output};

/// `init` creates a knowledge store and prints the path of each file it wrote.
/// It prints one `created <path>` line for each new file, then one
/// `updated <path>` line for each file it edited. With `--json`, it prints the
/// result as one JSON object instead.
// sokf-implements: CLI-2_AC-11
// sokf-implements: CLI-2_AC-12
pub fn init(repo_root: Option<&Path>, args: InitArgs) -> sokf_core::Result<u8> {
    let (cwd, root) = output::root(repo_root)?;
    let opts = InitOptions {
        dir: args.dir,
        cwd,
        name: args.name,
        scope: args.scope,
        description: args.description,
    };
    let result = sokf_core::init(&root, &opts)?;
    let mut buf = Vec::new();
    if args.json {
        buf = output::json_line(&result)?;
    } else {
        for path in &result.created {
            writeln!(buf, "created {path}").map_err(output::stdout_err)?;
        }
        for path in &result.updated {
            writeln!(buf, "updated {path}").map_err(output::stdout_err)?;
        }
    }
    output::write_stdout(&buf)?;
    // sokf-implements: CLI-2_AC-27
    Ok(output::EXIT_OK)
}
