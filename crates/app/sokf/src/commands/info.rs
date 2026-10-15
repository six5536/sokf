//! This module runs the `sokf info` command.

use std::path::Path;

use sokf_core::{CacheMode, InfoLayer, info_formats, info_grammar};

use crate::{
    cli::{InfoArgs, InfoCommand, Layer},
    output,
};

/// `info` prints the schema language or a format registry. It finds the
/// repository root only when the command needs the repository: when the user
/// gives a schema, or when the command reads a layer other than the embedded
/// one.
// sokf-implements: CLI-5_AC-2
// sokf-implements: CLI-5_AC-6
pub fn info(repo_root: Option<&Path>, cache: CacheMode, args: InfoArgs) -> sokf_core::Result<u8> {
    let text = match args.what {
        InfoCommand::Grammar => info_grammar().to_string(),
        InfoCommand::Formats(f) => {
            let layer = match f.layer {
                Some(Layer::Embedded) => InfoLayer::Embedded,
                Some(Layer::Repo) => InfoLayer::Repo,
                Some(Layer::Effective) | None => InfoLayer::Effective,
            };
            let needs_root = f.schema.is_some() || layer != InfoLayer::Embedded;
            let root = if needs_root {
                Some(output::root(repo_root)?.1)
            } else {
                None
            };
            info_formats(root.as_ref(), layer, f.schema.as_deref(), cache)?
        }
    };
    output::write_stdout(text.as_bytes())?;
    Ok(output::EXIT_OK)
}
