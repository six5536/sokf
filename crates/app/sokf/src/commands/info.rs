//! This module runs the `sokf info` command.

use std::path::Path;

use sokf_core::{CacheMode, Expr, InfoLayer, info_formats, info_formats_json, info_grammar};

use crate::{
    cli::{InfoArgs, InfoCommand, Layer},
    output,
};

/// `info` prints the schema language or a format registry, as YAML or with
/// `--json` as one JSON object, or the value that `--query` selects from it,
/// which it compiles before it reads the repository. It finds the repository
/// root only when the command needs the repository: when the user gives a
/// schema, or when the command reads a layer other than the embedded one.
// sokf-implements: CLI-5_AC-2
// sokf-implements: CLI-5_AC-6
// sokf-implements: CLI-5_AC-40
pub fn info(repo_root: Option<&Path>, cache: CacheMode, args: InfoArgs) -> sokf_core::Result<u8> {
    let text = match args.what {
        InfoCommand::Grammar => info_grammar(),
        InfoCommand::Formats(f) => {
            let query = f.query.as_deref().map(Expr::compile).transpose()?;
            let layer = match f.layer {
                Some(Layer::Embedded) => InfoLayer::Embedded,
                Some(Layer::Repo) => InfoLayer::Repo,
                Some(Layer::FormatsD) => InfoLayer::FormatsD,
                Some(Layer::Effective) | None => InfoLayer::Effective,
            };
            let needs_root = f.schema.is_some() || layer != InfoLayer::Embedded;
            let root = if needs_root {
                Some(output::root(repo_root)?.1)
            } else {
                None
            };
            let schema = f.schema.as_deref();
            if f.json {
                let registry = info_formats_json(root.as_ref(), layer, schema, cache)?;
                output::json_text(&registry, query.as_ref())?
            } else {
                info_formats(root.as_ref(), layer, schema, cache)?
            }
        }
    };
    output::write_stdout(text.as_bytes())?;
    Ok(output::EXIT_OK)
}
