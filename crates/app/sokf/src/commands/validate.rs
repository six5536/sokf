//! This module runs the `sokf validate` command.

use std::path::Path;

use sokf_core::{CacheMode, Expr, ValidateOptions};

use crate::{cli::ValidateArgs, output, render};

/// `validate` runs the validator and writes its report, or the projection
/// that `--query` selects from it, to standard output.
// sokf-implements: CLI-4_AC-14
// sokf-implements: CLI-4_AC-3
// sokf-implements: CLI-4_AC-12
pub fn validate(
    repo_root: Option<&Path>,
    cache: CacheMode,
    args: ValidateArgs,
) -> sokf_core::Result<u8> {
    let (cwd, root) = output::root(repo_root)?;
    let opts = ValidateOptions {
        paths: args.paths,
        cwd,
        fix: args.fix,
        cache,
        query: args.query.as_deref().map(Expr::compile).transpose()?,
        changed: args.changed,
    };
    let (report, projection) = sokf_core::validate(&root, &opts)?;
    let buf = if let Some(projection) = &projection {
        output::json_line(projection)?
    } else if args.json {
        // `validate` serialises the report into a buffer before it writes
        // anything, because callers pipe the output into `jq`. A failure
        // part-way through a write would leave a partial JSON object on
        // standard output.
        output::json_line(&report)?
    } else {
        let mut buf = Vec::new();
        render::text(&mut buf, &report, args.warnings).map_err(output::stdout_err)?;
        buf
    };
    output::write_stdout(&buf)?;
    // sokf-implements: CLI-1_AC-9
    // sokf-implements: CLI-1_AC-10
    Ok(if report.has_errors() {
        output::EXIT_ERRORS
    } else {
        output::EXIT_OK
    })
}
