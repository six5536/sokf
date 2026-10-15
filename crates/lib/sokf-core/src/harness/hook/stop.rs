//! This module holds the stop hook, the same for every harness. The hook runs
//! `validate --fix` and makes the agent continue on the errors that remain,
//! once for each distinct report in each session, in the form that the harness
//! that ran it reads.

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
/// agent-harness-adapter writes in the harness's form. With `Allow`, the agent can
/// stop, and `stderr` holds a report for the user that lists the same errors
/// that the hook has already made the agent continue on once. With `Continue`,
/// the agent continues with the reason as its input: the fixes that
/// `validate --fix` made, followed by the errors.
pub use agent_harness_adapter_core::hook::Answer;

/// A `HookAnswer` is the answer that `sokf harness hook claude stop` prints on stdout, and
/// that Codex and Factory Droid read in the same form: an empty object lets the agent stop,
/// and a block makes the agent continue (CLI-3_AC-25, CLI-3_AC-26). Every other harness
/// reads the answer in its own form.
// `HookAnswer` defines the JSON that goes over stdout, which the adapter's Claude Code harness
// writes: its schema is the one published as `harness-hook-answer.schema.json`, and the test
// `each_answer_as_json` checks that the two agree.
// sokf-implements: CLI-3_AC-26
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
#[schemars(title = "sokf harness hook answer")]
pub enum HookAnswer {
    /// When errors remain, the agent continues with the reason as its input.
    Block {
        /// The decision is always `block`.
        decision: Decision,
        /// The reason starts with `sokf:` and the number of remaining errors. It then
        /// lists the fixes that `validate --fix` made, and the errors as `validate`
        /// reports them.
        reason: String,
    },
    /// When no error remains, or the hook has already blocked on the same report, the agent
    /// stops.
    // `Allow` is a struct variant with no fields, because a unit variant would serialise as
    // `null`, not `{}`. It is the only object in the schema that allows no properties,
    // because the answer is always empty.
    #[schemars(extend("maxProperties" = 0))]
    Allow {},
}

/// A `Decision` tells Claude Code what to do with the agent. Its only value, `block`, makes
/// the agent continue.
#[derive(Serialize, JsonSchema)]
pub enum Decision {
    /// The agent continues.
    #[serde(rename = "block")]
    Block,
}

/// `OLD_EVENTS` maps each hook name that an earlier sokf installed to its event, so that a
/// hook line written before the harness names its events keeps working until the next
/// install. The earlier `stop` is the event's name today too.
const OLD_EVENTS: &[(&str, Event)] = &[("stop", Event::Stop)];

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
/// event that sokf has no hook for.
// sokf-implements: CLI-3_AC-31
pub fn hook_call(harness: &str, event: &str) -> Result<HookCall> {
    let h = harness::find(harness)
        .ok_or_else(|| Error::Harness(format!("no harness named `{harness}`")))?;
    if h.hook_events().is_empty() {
        return Err(Error::Harness(format!("harness `{harness}` runs no hooks")));
    }
    let parsed = event.parse::<Event>().ok().or_else(|| {
        OLD_EVENTS
            .iter()
            .find(|(name, _)| *name == event)
            .map(|(_, e)| *e)
    });
    match parsed {
        Some(e) if HOOK_EVENTS.contains(&e) && h.hook_events().contains(&e) => Ok(HookCall {
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

// sokf-implements: CLI-3_AC-27
fn reason(report: &Report, text: &str) -> String {
    let n = report.errors.len();
    let noun = if n == 1 {
        "error remains"
    } else {
        "errors remain"
    };
    format!(
        "sokf: {n} {noun} in the knowledge after `sokf validate --fix`. Fix them before you finish.\n{text}"
    )
}

/// `hook` runs the hook `call` with the harness's `input`.
// sokf-implements: CLI-3_AC-51
pub fn hook(root: &RepoRoot, call: &HookCall, input: &str, cache: CacheMode) -> Result<Answer> {
    // The input is the harness's event object. The hook reads only the session and whether
    // the agent already continues because of a stop hook, so it treats an input that
    // doesn't parse as empty.
    // sokf-implements: CLI-3_AC-57
    // sokf-implements: CLI-3_AC-69
    let input = HookInput::parse(call.harness.as_ref(), call.event, input)
        .unwrap_or_else(|_| HookInput::new(call.harness.id(), call.event, Value::Null));
    let guard = Guard::new(
        root,
        cache,
        guard_key(call.harness.id(), input.session_id.as_deref()),
    );
    // A hook installed at the user scope runs in every repository. A repository with no
    // configuration and no default knowledge root holds no SOKF knowledge, so the agent stops.
    // sokf-implements: CLI-3_AC-68
    if !holds_knowledge(root) {
        return Ok(Answer::Allow { stderr: None });
    }
    let opts = ValidateOptions {
        paths: vec![],
        cwd: root.as_path().to_path_buf(),
        fix: true,
        cache,
        query: None,
        changed: vec![],
    };
    let (report, _) = crate::validate(root, &opts)?;
    // sokf-implements: CLI-3_AC-25
    if !report.has_errors() {
        guard.clear();
        return Ok(Answer::Allow { stderr: None });
    }
    let text = report::text(&report, false);
    // The guard hashes only the errors. A second run makes no fixes, so its
    // report differs from the first only in the fixes, and the hook should
    // treat it as the same report.
    let mut errors_only = report.clone();
    errors_only.fixes.clear();
    // sokf-implements: CLI-3_AC-28
    // sokf-implements: CLI-3_AC-29
    // sokf-implements: CLI-3_AC-63
    let block = if guard.is_on() {
        guard.should_block(&report::text(&errors_only, false))
    } else {
        !input.continuing
    };
    if !block {
        return Ok(Answer::Allow { stderr: Some(text) });
    }
    let answer = Answer::Continue {
        reason: reason(&report, &text),
    };
    // A harness that can't make the agent continue at this event lets it stop, with the
    // report for the user.
    // sokf-implements: CLI-3_AC-64
    if call.harness.answer(call.event, &answer).is_err() {
        return Ok(Answer::Allow { stderr: Some(text) });
    }
    Ok(answer)
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
            Event::Stop,
            Ok::<_, Error>(answer),
            &mut out,
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(code, 0);
        String::from_utf8(out).unwrap().trim_end().to_string()
    }

    // sokf-tests: CLI-3_AC-25
    // sokf-tests: CLI-3_AC-26
    #[test]
    fn each_answer_as_json() {
        assert_eq!(json("claude", Answer::Allow { stderr: None }), "{}");
        assert_eq!(
            json(
                "claude",
                Answer::Allow {
                    stderr: Some("x".into())
                }
            ),
            "{}"
        );
        assert_eq!(
            json(
                "claude",
                Answer::Continue {
                    reason: "sokf: 1 error remains".into()
                }
            ),
            "{\"decision\":\"block\",\"reason\":\"sokf: 1 error remains\"}"
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
                json(id, Answer::Continue { reason: "r".into() }),
                schema_form(&HookAnswer::Block {
                    decision: Decision::Block,
                    reason: "r".into()
                })
            );
        }
    }

    // sokf-tests: CLI-3_AC-31
    #[test]
    fn a_hook_call_names_a_harness_that_runs_hooks_and_the_stop_event() {
        for id in [
            "claude", "codex", "factory", "gemini", "copilot", "cursor", "pi", "opencode", "kilo",
            "qwen", "devin",
        ] {
            let call = hook_call(id, "stop").unwrap();
            assert_eq!((call.harness.id(), call.event), (id, Event::Stop));
        }
        let e = |h: &str, ev: &str| hook_call(h, ev).unwrap_err().to_string();
        assert_eq!(e("nope", "stop"), "harness: no harness named `nope`");
        assert_eq!(
            e("agents", "stop"),
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

    #[test]
    fn the_reason_counts_the_errors() {
        let mut report = Report::default();
        report.errors.push(crate::Finding::error(
            crate::RepoPath::parse("/k/a.md").unwrap(),
            None,
            "SOKF_FORMAT_1",
            "bad",
        ));
        let r = reason(&report, "REPORT");
        assert!(
            r.starts_with("sokf: 1 error remains in the knowledge"),
            "{r}"
        );
        assert!(r.ends_with("\nREPORT"), "{r}");
        report.errors.push(report.errors[0].clone());
        assert!(reason(&report, "").starts_with("sokf: 2 errors remain"));
    }
}
