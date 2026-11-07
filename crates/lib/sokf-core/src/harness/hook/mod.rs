//! The `hook` module holds the hook of the `harness hook` command, the same
//! for every harness, and its guard.

mod guard;
mod prompt;

pub use guard::guard_key;
pub use prompt::{Answer, EventName, HookAnswer, HookCall, NoteOutput, hook, hook_call};
