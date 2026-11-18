//! This module runs the `sokf guide` command.

use std::path::Path;

use sokf_core::{CacheMode, Expr, GuideOptions};

use crate::{cli::GuideArgs, output, render};

/// `guide` prints the guide of each document that the arguments name: as text,
/// as one JSON object with `--json`, or as the value that `--query` selects
/// from that object. It compiles the query before it reads the repository. It
/// renders the whole output before it writes any of it, so a refusal prints
/// nothing to stdout.
// sokf-implements: CLI-17_AC-17
// sokf-implements: CLI-17_AC-18
// sokf-implements: CLI-17_AC-20
// sokf-implements: CLI-17_AC-22
// sokf-implements: CLI-17_AC-23
pub fn guide(repo_root: Option<&Path>, cache: CacheMode, args: GuideArgs) -> sokf_core::Result<u8> {
    let query = args.query.as_deref().map(Expr::compile).transpose()?;
    let (cwd, root) = output::root(repo_root)?;
    let opts = GuideOptions {
        documents: args.documents,
        states: args.state,
        cwd,
        cache,
    };
    let report = sokf_core::guide(&root, &opts)?;
    let buf = if args.json {
        output::json_text(&report, query.as_ref())?.into_bytes()
    } else {
        let mut buf = Vec::new();
        render::guide(&mut buf, &report).map_err(output::stdout_err)?;
        buf
    };
    output::write_stdout(&buf)?;
    Ok(output::EXIT_OK)
}
