//! The `it` test binary holds all the integration tests of the crate, so
//! the crate links once, not once for each test file. Each module is one
//! file of tests.

mod cli;
mod cli_d_folders;
mod cli_design;
mod cli_graph;
mod cli_graph_markers;
mod cli_harness;
mod cli_harness_multi;
mod cli_index;
mod cli_info;
mod cli_init;
mod cli_locate;
mod cli_new;
mod cli_no_packs;
mod cli_options;
mod cli_validate;
mod cli_verified;
mod common;
mod json_schemas;
