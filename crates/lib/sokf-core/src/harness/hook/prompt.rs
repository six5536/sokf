//! This module holds the hook, the same for every harness. When the user
//! submits a prompt, the hook runs `validate`, and while errors remain it adds
//! a short note to the agent's context, once for each distinct set of errors
//! in each session, in the form that the harness that ran it reads. It never
//! blocks the agent and never changes a file.

use std::sync::Arc;

use agent_harness_adapter_core::{
    Harness, harness,
    hook::{Event, HookInput},
};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use super::guard::{Guard, guard_key};
use crate::{
    CacheMode, Config, Error, RepoRoot, Report, Result, ValidateOptions, config,
    harness::HOOK_EVENTS, report,
};

/// `Answer` is the hook's answer, the same for every harness, which
/// agent-harness-adapter writes in the harness's form. With `Allow`, the hook adds
/// nothing. With `Context`, the harness adds the note to the agent's context.
pub use agent_harness_adapter_core::hook::Answer;

/// A `HookAnswer` is the answer that `sokf harness hook claude prompt-submit` prints on
/// stdout, and that Codex and Factory Droid read in the same form: an empty object adds
/// nothing, and `hookSpecificOutput` adds the note to the agent's context. Every other
/// harness reads the answer in its own form.
// `HookAnswer` defines the JSON that goes over stdout, which the adapter's Claude Code harness
// writes: its schema is the one published as `harness-hook-answer.schema.json`, and the test
// `each_answer_as_json` checks that the two agree.
// sokf-implements: CLI-3_AC-82
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
#[schemars(title = "sokf harness hook answer")]
pub enum HookAnswer {
    /// While errors remain, the note goes into the agent's context.
    Note {
        /// The output for the event.
        #[serde(rename = "hookSpecificOutput")]
        output: NoteOutput,
    },
    /// When no error remains, or the hook has already noted the same errors, the hook
    /// adds nothing.
    // `Allow` is a struct variant with no fields, because a unit variant would serialise as
    // `null`, not `{}`. It is the only object in the schema that allows no properties,
    // because the answer is always empty.
    #[schemars(extend("maxProperties" = 0))]
    Allow {},
}

/// A `NoteOutput` holds the note and the event that it answers.
#[derive(Serialize, JsonSchema)]
pub struct NoteOutput {
    /// The event, always `UserPromptSubmit`.
    #[serde(rename = "hookEventName")]
    pub event: EventName,
    /// The note: `sokf:`, the number of errors, and `Fix before finishing the work.`
    #[serde(rename = "additionalContext")]
    pub note: String,
}

/// An `EventName` is the name of the event that the answer is for. Its only value is
/// `UserPromptSubmit`.
#[derive(Serialize, JsonSchema)]
pub enum EventName {
    /// The user submits a prompt.
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit,
}

/// `OLD_EVENTS` lists each event of a hook that an earlier sokf installed, so that a hook line
/// written by an earlier sokf keeps parsing until the next install.
const OLD_EVENTS: &[Event] = &[Event::Stop];

/// A `HookCall` is the harness that runs a hook and the event that the hook answers.
#[derive(Debug, Clone)]
pub struct HookCall {
    /// The harness, one of agent-harness-adapter's.
    pub harness: Arc<dyn Harness>,
    /// The event.
    pub event: Event,
}

/// `hook_call` returns the hook that `sokf harness hook <HARNESS> <EVENT>` runs. It refuses,
/// with `Error::Harness`, a harness that the adapter doesn't know or that runs no hooks, and an
/// event that sokf has no hook for. It takes `stop`, the event of an earlier sokf's hook.
// sokf-implements: CLI-3_AC-31
pub fn hook_call(harness: &str, event: &str) -> Result<HookCall> {
    let h = harness::find(harness)
        .ok_or_else(|| Error::Harness(format!("no harness named `{harness}`")))?;
    if h.hook_events().is_empty() {
        return Err(Error::Harness(format!("harness `{harness}` runs no hooks")));
    }
    let parsed = event.parse::<Event>().ok();
    let known = |e: &Event| HOOK_EVENTS.contains(e) || OLD_EVENTS.contains(e);
    match parsed {
        Some(e) if known(&e) && h.hook_events().contains(&e) => Ok(HookCall {
            harness: h,
            event: e,
        }),
        _ => Err(Error::Harness(format!(
            "sokf has no hook for the event `{event}`"
        ))),
    }
}

/// `holds_knowledge` reports whether the repository `root` has `/.sokf/config.toml` or one of
/// the default knowledge roots.
fn holds_knowledge(root: &RepoRoot) -> bool {
    config::config_path().to_fs(root).is_file()
        || Config::default()
            .roots
            .iter()
            .any(|r| r.to_fs(root).is_dir())
}

/// `note` returns the note for `n` errors.
// sokf-implements: CLI-3_AC-82
fn note(n: usize) -> String {
    let errors = tramlines_core::text::count(n, "error", "errors");
    format!("sokf: {errors}. Fix before finishing the work.")
}

/// `hook` runs the hook `call` with the harness's `input`.
// sokf-implements: CLI-3_AC-51
pub fn hook(root: &RepoRoot, call: &HookCall, input: &str, cache: CacheMode) -> Result<Answer> {
    let nothing = Answer::Allow { stderr: None };
    // A hook that an earlier sokf installed at the stop runs until the next install, and
    // lets the agent stop.
    // sokf-implements: CLI-3_AC-84
    if call.event == Event::Stop {
        return Ok(nothing);
    }
    // A harness that can't add text to the agent's context at the event gets nothing, before
    // the hook runs the check.
    // sokf-implements: CLI-3_AC-83
    let probe = Answer::Context {
        text: String::new(),
    };
    if call.harness.answer(call.event, &probe).is_err() {
        return Ok(nothing);
    }
    // A hook installed at the user scope runs in every repository. A repository with no
    // configuration and no default knowledge root holds no SOKF knowledge.
    // sokf-implements: CLI-3_AC-68
    if !holds_knowledge(root) {
        return Ok(nothing);
    }
    // The input is the harness's event object. The hook reads only the session, so it
    // treats an input that doesn't parse as empty.
    // sokf-implements: CLI-3_AC-57
    // sokf-implements: CLI-3_AC-69
    let input = HookInput::parse(call.harness.as_ref(), call.event, input)
        .unwrap_or_else(|_| HookInput::new(call.harness.id(), call.event, Value::Null));
    let guard = Guard::new(
        root,
        cache,
        guard_key(call.harness.id(), input.session_id.as_deref()),
    );
    let opts = ValidateOptions {
        paths: vec![],
        cwd: root.as_path().to_path_buf(),
        fix: false,
        cache,
        query: None,
        changed: vec![],
        // The hook decides on the errors alone, so the warnings at the harnesses need no
        // home directory.
        home: None,
    };
    let (report, _) = crate::validate(root, &opts)?;
    // sokf-implements: CLI-3_AC-25
    if !report.has_errors() {
        guard.clear();
        return Ok(nothing);
    }
    // The guard hashes the errors alone.
    let n = report.errors.len();
    let errors = Report {
        errors: report.errors,
        ..Report::default()
    };
    // sokf-implements: CLI-3_AC-28
    // sokf-implements: CLI-3_AC-85
    if !guard.should_note(&report::text(&errors, false)) {
        return Ok(nothing);
    }
    Ok(Answer::Context { text: note(n) })
}

#[cfg(test)]
mod tests {
    use agent_harness_adapter_core::hook::emit;

    use super::*;

    fn json(id: &str, answer: Answer) -> String {
        let h = harness::find(id).unwrap();
        let mut out = Vec::new();
        let code = emit(
            h.as_ref(),
            Event::PromptSubmit,
            Ok::<_, Error>(answer),
            &mut out,
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(code, 0);
        String::from_utf8(out).unwrap().trim_end().to_string()
    }

    // sokf-tests: CLI-3_AC-25
    // sokf-tests: CLI-3_AC-82
    #[test]
    fn each_answer_as_json() {
        assert_eq!(json("claude", Answer::Allow { stderr: None }), "{}");
        assert_eq!(
            json(
                "claude",
                Answer::Context {
                    text: "sokf: 1 error. Fix before finishing the work.".into()
                }
            ),
            "{\"hookSpecificOutput\":{\"hookEventName\":\"UserPromptSubmit\",\"additionalContext\":\"sokf: 1 error. Fix before finishing the work.\"}}"
        );
        // The answers agree with `HookAnswer`, whose schema is published, for each
        // harness that reads Claude Code's answers.
        let schema_form = |a: &HookAnswer| serde_json::to_string(a).unwrap();
        for id in ["claude", "codex", "factory"] {
            assert_eq!(
                json(id, Answer::Allow { stderr: None }),
                schema_form(&HookAnswer::Allow {})
            );
            assert_eq!(
                json(id, Answer::Context { text: "n".into() }),
                schema_form(&HookAnswer::Note {
                    output: NoteOutput {
                        event: EventName::UserPromptSubmit,
                        note: "n".into()
                    }
                })
            );
        }
    }

    // sokf-tests: CLI-3_AC-31
    #[test]
    fn a_hook_call_names_a_harness_that_runs_hooks_and_a_hook_event() {
        for id in [
            "claude", "codex", "factory", "gemini", "copilot", "cursor", "pi", "opencode", "kilo",
            "qwen", "devin",
        ] {
            let call = hook_call(id, "prompt-submit").unwrap();
            assert_eq!((call.harness.id(), call.event), (id, Event::PromptSubmit));
            // The event of an earlier sokf's hook still parses.
            let call = hook_call(id, "stop").unwrap();
            assert_eq!((call.harness.id(), call.event), (id, Event::Stop));
        }
        let e = |h: &str, ev: &str| hook_call(h, ev).unwrap_err().to_string();
        assert_eq!(
            e("nope", "prompt-submit"),
            "harness: no harness named `nope`"
        );
        assert_eq!(
            e("agents", "prompt-submit"),
            "harness: harness `agents` runs no hooks"
        );
        assert_eq!(
            e("claude", "pre-tool"),
            "harness: sokf has no hook for the event `pre-tool`"
        );
        assert_eq!(
            e("claude", "start"),
            "harness: sokf has no hook for the event `start`"
        );
    }

    // sokf-tests: CLI-3_AC-82
    #[test]
    fn the_note_counts_the_errors() {
        assert_eq!(note(1), "sokf: 1 error. Fix before finishing the work.");
        assert_eq!(note(3), "sokf: 3 errors. Fix before finishing the work.");
    }
}
