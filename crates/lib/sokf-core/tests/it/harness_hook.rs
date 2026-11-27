//! These tests run the hook over the fixtures. When the knowledge is clean, the
//! hook adds nothing. When it is broken, the hook adds a note to the agent's
//! context once for each set of errors, and never changes a file. A failure of
//! the tool never blocks the agent.

use common::TempTree;
use sokf_core::{CacheMode, Error, harness};

use crate::common;

fn prompt(tree: &TempTree, cache: CacheMode) -> sokf_core::Result<harness::Answer> {
    prompt_as(
        tree,
        "claude",
        "{\"hook_event_name\":\"UserPromptSubmit\"}",
        cache,
    )
}

fn prompt_as(
    tree: &TempTree,
    harness: &str,
    input: &str,
    cache: CacheMode,
) -> sokf_core::Result<harness::Answer> {
    let call = harness::hook_call(harness, "prompt-submit")?;
    harness::hook(&tree.root(), &call, input, cache)
}

const NOTHING: harness::Answer = harness::Answer::Allow { stderr: None };

// sokf-tests: CLI-3_AC-25
#[test]
fn a_clean_knowledge_gets_no_note() {
    let tree = TempTree::fixture("clean");
    let before = tree.files();
    assert_eq!(prompt(&tree, CacheMode::On).unwrap(), NOTHING);
    assert_eq!(tree.files(), before);
}

// sokf-tests: CLI-3_AC-51
#[test]
fn the_hook_repairs_nothing() {
    let tree = TempTree::fixture("fixable");
    let before = tree.files();
    let answer = prompt(&tree, CacheMode::Off).unwrap();
    assert_eq!(tree.files(), before, "the hook changed a file");
    // The fixable fixture's errors stay, for `--fix` to repair when the agent chooses.
    assert!(
        matches!(answer, harness::Answer::Context { .. }),
        "{answer:?}"
    );
}

// sokf-tests: CACHE-2_AC-2
// sokf-tests: CACHE-2_AC-3
// sokf-tests: CLI-3_AC-28
// sokf-tests: CLI-3_AC-82
// sokf-tests: CLI-3_AC-85
// sokf-tests: CLI_P-20
#[test]
fn a_broken_knowledge_gets_one_note_per_set_of_errors() {
    let tree = TempTree::fixture("broken");
    let n = tree.validate_all(false, CacheMode::Off).errors.len();
    let harness::Answer::Context { text } = prompt(&tree, CacheMode::On).unwrap() else {
        panic!("expected a note");
    };
    assert_eq!(
        text,
        format!("sokf: {n} errors. Fix before finishing the work.")
    );
    // The guard file holds the loop guard's hash of the errors that the hook
    // noted, and a newline.
    let guard = tree.dir().join(".sokf/cache/harness-prompt-claude.hash");
    let kept = std::fs::read_to_string(&guard).unwrap();
    assert!(
        kept.ends_with('\n') && kept.lines().count() == 1 && !kept.contains("error"),
        "{kept:?}"
    );
    // While the errors stay the same, the hook adds nothing.
    assert_eq!(prompt(&tree, CacheMode::On).unwrap(), NOTHING);
    // When the errors change, the hook notes them again, and the guard file
    // holds the new hash.
    tree.write(
        "knowledge/extra.md",
        "---\ntype: Nope\nid: extra\n---\n\n# Extra\n",
    );
    assert!(matches!(
        prompt(&tree, CacheMode::On).unwrap(),
        harness::Answer::Context { .. }
    ));
    let renewed = std::fs::read_to_string(&guard).unwrap();
    assert_ne!(renewed, kept);
    // With the cache off, there is no guard: the hook notes the errors at each
    // prompt, and neither reads nor writes the guard file.
    for _ in 0..2 {
        assert!(matches!(
            prompt(&tree, CacheMode::Off).unwrap(),
            harness::Answer::Context { .. }
        ));
    }
    assert_eq!(std::fs::read_to_string(&guard).unwrap(), renewed);
}

// sokf-tests: CACHE-2_AC-2
// sokf-tests: CLI-3_AC-28
#[test]
fn each_harness_and_session_has_its_own_guard() {
    let tree = TempTree::fixture("broken");
    let notes = |harness: &str, input: &str| {
        matches!(
            prompt_as(&tree, harness, input, CacheMode::On).unwrap(),
            harness::Answer::Context { .. }
        )
    };
    let session = "{\"session_id\":\"s1\"}";
    assert!(notes("claude", session));
    assert!(!notes("claude", session));
    // Another session of the same harness, and another harness, get the note once more.
    assert!(notes("claude", "{\"session_id\":\"s2\"}"));
    assert!(notes("codex", session));
    assert!(notes("gemini", session));
    assert!(notes("pi", "{}"));
    assert!(!notes("pi", "{}"));
    for file in [
        "harness-prompt-claude-s1",
        "harness-prompt-claude-s2",
        "harness-prompt-codex-s1",
        "harness-prompt-pi",
    ] {
        assert!(
            tree.dir()
                .join(format!(".sokf/cache/{file}.hash"))
                .is_file(),
            "{file}"
        );
    }
    // Once the knowledge is clean, the hook clears the session's guard.
    let clean = TempTree::fixture("clean");
    let call = harness::hook_call("claude", "prompt-submit").unwrap();
    std::fs::create_dir_all(clean.dir().join(".sokf/cache")).unwrap();
    clean.write(".sokf/cache/harness-prompt-claude-s1.hash", "0\n");
    assert_eq!(
        harness::hook(&clean.root(), &call, session, CacheMode::On).unwrap(),
        NOTHING
    );
    assert!(
        !clean
            .dir()
            .join(".sokf/cache/harness-prompt-claude-s1.hash")
            .exists()
    );
}

// The adapter writes each answer in its harness's form, and its own tests check those forms.
// This test checks that every harness that runs the hook takes sokf's answer: the note, or,
// where the harness can't add text to the agent's context, nothing.
// sokf-tests: CLI-3_AC-82
// sokf-tests: CLI-3_AC-83
// sokf-tests: CLI-3_AC-69
#[test]
fn every_harness_that_runs_the_hook_takes_the_answer() {
    use agent_harness_adapter_core::{harness::builtin, hook::emit};
    let tree = TempTree::fixture("broken");
    let mut noted = Vec::new();
    for h in builtin() {
        let Ok(call) = harness::hook_call(h.id(), "prompt-submit") else {
            continue;
        };
        let answer = harness::hook(&tree.root(), &call, "", CacheMode::Off).unwrap();
        match &answer {
            harness::Answer::Context { text } => {
                assert!(text.starts_with("sokf: "), "{}: {text}", h.id());
                noted.push(h.id().to_string());
            }
            harness::Answer::Allow { stderr: None } => {}
            other => panic!("{}: {other:?}", h.id()),
        }
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = emit(
            h.as_ref(),
            call.event,
            Ok::<_, Error>(answer),
            &mut out,
            &mut err,
        )
        .unwrap();
        assert!(
            err.is_empty(),
            "{}: {}",
            h.id(),
            String::from_utf8_lossy(&err)
        );
        assert_eq!(code, 0, "{}", h.id());
    }
    // Copilot and Cursor can't add text to the agent's context at the prompt.
    assert_eq!(
        noted,
        [
            "claude", "codex", "factory", "gemini", "pi", "opencode", "kilo", "qwen", "devin"
        ]
    );
}

// sokf-tests: CLI-3_AC-84
#[test]
fn a_stop_hook_of_an_earlier_sokf_lets_the_agent_stop() {
    let tree = TempTree::fixture("broken");
    let before = tree.files();
    let call = harness::hook_call("claude", "stop").unwrap();
    assert_eq!(
        harness::hook(&tree.root(), &call, "{}", CacheMode::On).unwrap(),
        NOTHING
    );
    assert_eq!(tree.files(), before);
}

// sokf-tests: CLI-3_AC-68
#[test]
fn a_repository_without_knowledge_gets_no_note() {
    let tree = TempTree::empty("hook-no-knowledge");
    let before = tree.files();
    assert_eq!(prompt(&tree, CacheMode::On).unwrap(), NOTHING);
    assert_eq!(tree.files(), before);
    assert!(!tree.dir().join(".sokf").exists());
}

// sokf-tests: CLI-3_AC-31
// sokf-tests: CLI-3_AC-30
#[test]
fn an_unknown_name_is_a_usage_error_and_a_broken_config_propagates() {
    let tree = TempTree::fixture("clean");
    let e = harness::hook_call("cline", "prompt-submit").unwrap_err();
    assert_eq!(e.to_string(), "harness: no harness named `cline`");
    let e = harness::hook_call("claude", "start").unwrap_err();
    assert_eq!(
        e.to_string(),
        "harness: sokf has no hook for the event `start`"
    );
    // The hook still runs when its input is not JSON.
    assert!(prompt_as(&tree, "claude", "not json", CacheMode::Off).is_ok());
    assert!(prompt_as(&tree, "copilot", "not json", CacheMode::Off).is_ok());
    tree.write(".sokf/config.toml", "[knowledge]\nroots = [\"/missing\"]\n");
    let e = prompt(&tree, CacheMode::Off).unwrap_err();
    assert!(matches!(e, Error::Config { .. }), "{e}");
}
