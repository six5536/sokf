//! The `sokf new` command.

use std::{io::Write, path::Path};

use sokf_core::{CacheMode, NewOptions};

use crate::{cli::NewArgs, output};

/// `new` derives a document from a schema and prints it. With `--write`, it
/// prints the `created` line instead. With `--id-only`, it prints only the id.
/// With `--json`, it prints one JSON object.
// sokf-implements: CLI-6_AC-10
// sokf-implements: CLI-6_AC-11
// sokf-implements: CLI-6_AC-12
// sokf-implements: CLI-6_AC-13
pub fn new(repo_root: Option<&Path>, cache: CacheMode, args: NewArgs) -> sokf_core::Result<u8> {
    let (cwd, root) = output::root(repo_root)?;
    let opts = NewOptions {
        type_name: output::required(args.type_name, "TYPE")?,
        slug: args.slug,
        id: args.id,
        title: args.title,
        states: args.state,
        dir: args.dir,
        for_asset: args.for_asset,
        cwd,
        write: args.write,
        id_only: args.id_only,
        cache,
    };
    let result = sokf_core::new(&root, &opts)?;
    let mut buf = Vec::new();
    if args.json {
        buf = output::json_line(&result)?;
    } else if args.id_only {
        writeln!(buf, "{}", result.id).map_err(output::stdout_err)?;
    } else if args.write {
        writeln!(buf, "created {}", result.path).map_err(output::stdout_err)?;
    } else {
        buf.extend_from_slice(result.document.as_bytes());
    }
    output::write_stdout(&buf)?;
    // sokf-implements: CLI-6_AC-55
    Ok(output::EXIT_OK)
}
