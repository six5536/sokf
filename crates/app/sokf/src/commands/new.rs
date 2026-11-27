//! The `sokf new` command.

use std::{io::Write, path::Path};

use sokf_core::{CacheMode, GuideReport, NewOptions};

use crate::{cli::NewArgs, output, render};

/// `new` derives the skeleton of a document from a schema and prints it, and
/// prints the guide of the document on stderr. With `--write`, it prints the
/// `created` line in place of the document, and an `updated` line for each
/// index that it wrote, and the guide on stderr. With `--id-only`, it prints
/// only the id. With `--json`, it prints one JSON object, which holds the
/// guide, and with `--write` too, the object takes the place of the `created`
/// line.
// sokf-implements: CLI-6_AC-10
// sokf-implements: CLI-6_AC-11
// sokf-implements: CLI-6_AC-12
// sokf-implements: CLI-6_AC-13
// sokf-implements: CLI-6_AC-72
// sokf-implements: CLI-6_AC-127
// sokf-implements: CLI-6_AC-129
pub fn new(repo_root: Option<&Path>, cache: CacheMode, args: NewArgs) -> sokf_core::Result<u8> {
    let (cwd, root) = output::root(repo_root)?;
    let opts = NewOptions {
        type_name: output::required(args.type_name, "TYPE")?,
        slug: args.slug,
        id: args.id,
        title: args.title,
        seq: args.seq,
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
        for index in &result.indexes {
            writeln!(buf, "updated {index}").map_err(output::stdout_err)?;
        }
    } else {
        buf.extend_from_slice(result.document.as_bytes());
    }
    output::write_stdout(&buf)?;
    // The guide goes on stderr, so that stdout holds the document alone, and
    // the writer reads each part's guidance and example before writing it.
    if !args.json
        && !args.id_only
        && let Some(guide) = &result.guide
    {
        let mut text =
            b"\nThe guide of the new document: the guidance and the example of each part.\n\n"
                .to_vec();
        let report = GuideReport {
            documents: vec![guide.clone()],
        };
        render::guide(&mut text, &report).map_err(output::stdout_err)?;
        // The command has done its work by now, so a stderr that can't be
        // written to costs only the guide.
        let _ = std::io::stderr().write_all(&text);
    }
    if let Some(e) = &result.index_error {
        eprintln!("warning: new: {e}, so an index may be stale: run `sokf validate --fix`");
    }
    // sokf-implements: CLI-6_AC-55
    Ok(output::EXIT_OK)
}
