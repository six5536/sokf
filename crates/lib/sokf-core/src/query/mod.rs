//! This module parses and evaluates JMESPath expressions. `--query` uses
//! them as projections (CLI-4_AC-14), and `graph` uses them as predicates
//! (CLI-7_AC-43). The `jmespath` crate parses an expression. The `eval`
//! module walks the syntax tree over a `serde_json::Value`. An object
//! therefore keeps the key order that it was read or written with.

#[cfg(test)]
mod compliance;
mod eval;
mod expr;
mod functions;

pub use expr::Expr;
