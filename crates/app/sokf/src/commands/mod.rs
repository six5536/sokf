//! This module holds one handler for each command. A handler finds the
//! repository root only if it reads the repository (CLI-1_AC-5, CLI-5_AC-2,
//! CLI-5_AC-6).

mod graph;
mod harness;
mod info;
mod init;
mod locate;
mod new;
mod validate;

pub use graph::graph;
pub use harness::harness;
pub use info::info;
pub use init::init;
pub use locate::locate;
pub use new::new;
pub use validate::validate;
