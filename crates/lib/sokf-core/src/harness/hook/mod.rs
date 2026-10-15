//! The `hook` module holds the hook of the `harness hook` command, the same
//! for every harness, and its loop guard.

mod guard;
mod stop;

pub use guard::guard_key;
pub use stop::{Answer, Decision, HookAnswer, HookCall, hook, hook_call};
