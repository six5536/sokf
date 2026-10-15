//! The `harness` module integrates sokf with agent harnesses (CLI-3) through
//! agent-harness-adapter. It holds sokf's integration, sokf as a adapter `Tool`, and
//! the hook. The adapter holds the harnesses, the part kinds, the states, the
//! record, and `install` and `status`.

mod hook;
mod integration;
mod tool;

/// `DEFAULT_HOOK_PROGRAM` is the program that the hook line runs. A
/// repository replaces it with `harness.<name>.command` (CONF-4_AC-2,
/// CONF-4_AC-3).
pub const DEFAULT_HOOK_PROGRAM: &str = "sokf";

pub use hook::{Answer, Decision, HookAnswer, HookCall, guard_key, hook, hook_call};
pub use integration::{
    ALLOWED_COMMAND, HOOK_COMMAND_MARK, HOOK_EVENTS, hook_command, integration, skill,
};
pub use tool::{HEADER, SokfTool, record_path};
