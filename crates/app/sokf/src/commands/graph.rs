//! This module runs the `sokf graph` command.

use std::path::Path;

use sokf_core::{CacheMode, Expr, GraphOptions, GraphQuery, graph::bounded};

use crate::{
    cli::GraphArgs,
    output,
    render::{Template, mermaid, tree},
};

/// `graph` parses the format template and compiles the selector, edge, target, and query
/// expressions, then runs the graph query on the knowledge root. It prints the query's
/// projection when the query has one, and otherwise prints the graph as JSON, as Mermaid,
/// or as a tree whose lines follow the format template. If the format template doesn't
/// parse, `graph` prints the error and returns the failure exit code.
// sokf-implements: CLI-7_AC-27
// sokf-implements: CLI-7_AC-12
// sokf-implements: CLI-7_AC-21
// sokf-implements: CLI-7_AC-65
pub fn graph(repo_root: Option<&Path>, cache: CacheMode, args: GraphArgs) -> sokf_core::Result<u8> {
    let template = match Template::parse(args.format.as_deref().unwrap_or("{id}")) {
        Ok(t) => t,
        Err(message) => {
            eprintln!("error: format: {message}");
            return Ok(output::EXIT_FAILURE);
        }
    };
    let compile = |sources: Vec<String>| -> sokf_core::Result<Vec<Expr>> {
        sources.iter().map(|s| Expr::compile(s)).collect()
    };
    let query = GraphQuery {
        start: args.start,
        depth: args.depth.unwrap_or(1),
        layout_edges: args.depth.is_some(),
        selector: compile(args.selector)?,
        edge: compile(args.edge)?,
        to: compile(args.to)?,
        query: args.query.as_deref().map(Expr::compile).transpose()?,
        all: args.all,
        archived: args.archived,
    };
    let (cwd, root) = output::root(repo_root)?;
    let opts = GraphOptions { query, cwd, cache };
    let (graph, result) = sokf_core::graph(&root, &opts)?;
    let mut buf = Vec::new();
    if let Some(projection) = &result.projection {
        buf = output::json_line(projection)?;
    } else if args.json {
        buf = output::json_line(&bounded(&graph, &result))?;
    } else if args.mermaid {
        mermaid::text(&mut buf, &graph, &result).map_err(output::stdout_err)?;
    } else {
        tree::text(&mut buf, &graph, &result, &template).map_err(output::stdout_err)?;
    }
    output::write_stdout(&buf)?;
    Ok(output::EXIT_OK)
}
