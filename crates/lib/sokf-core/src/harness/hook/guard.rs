//! This module guards the hook against noting the same errors twice. It
//! keeps the hash of the last errors that the hook noted to the agent, for
//! each harness and session, in a file under `/.sokf/cache/` (CACHE-2_AC-2),
//! through the loop guard of agent-harness-adapter.

use agent_harness_adapter_core::hook::LoopGuard;

use crate::{CacheMode, RepoRoot, cache::CACHE_DIR};

// sokf:begin guard-file
/// `guard_key` returns the key of the hook of the harness `harness` in the
/// session `session` in the loop guard, which keeps the hash of each key in
/// `/.sokf/cache/<key>.hash`: `harness-prompt-<harness>-<session>`, or
/// `harness-prompt-<harness>` when the harness gives no session. The loop guard
/// names the file by the key's hash instead when the key is longer than 64
/// characters or holds a character other than a letter, a digit, `-`, or `_`.
pub fn guard_key(harness: &str, session: Option<&str>) -> String {
    match session {
        Some(s) if !s.is_empty() => format!("harness-prompt-{harness}-{s}"),
        _ => format!("harness-prompt-{harness}"),
    }
}
// sokf:end guard-file

/// `Guard` is the loop guard of one harness and session in a repository, or no
/// guard when the cache is off.
pub struct Guard {
    guard: Option<LoopGuard>,
    key: String,
}

impl Guard {
    /// `new` returns the guard of the key `key` in `root`, which reads and writes
    /// nothing when `mode` is off.
    pub fn new(root: &RepoRoot, mode: CacheMode, key: String) -> Self {
        let guard = (mode == CacheMode::On).then(|| LoopGuard::new(root.as_path().join(CACHE_DIR)));
        Guard { guard, key }
    }

    // sokf:begin guard-write
    /// `should_note` reports whether the hook should note the errors `text` to
    /// the agent. It returns `false` when the guard file holds the hash of the
    /// same text, and otherwise records the hash and returns `true`. Like every
    /// write to the cache, the record ignores a failure, and it creates the cache
    /// directory with its `.gitignore`. When the cache is off, the guard reads
    /// and writes nothing and returns `true`, so the hook notes the errors at
    /// each prompt.
    // sokf-implements: CACHE-2_AC-2
    // sokf-implements: CACHE-2_AC-3
    // sokf-implements: CLI-4_AC-18
    // sokf-implements: CLI-1_AC-13
    pub fn should_note(&self, text: &str) -> bool {
        self.guard
            .as_ref()
            .is_none_or(|g| g.should_block(&self.key, text))
    }
    // sokf:end guard-write

    /// `clear` forgets the last errors, once no error remains, so that the guard
    /// file of a session that ends clean goes away.
    pub fn clear(&self) {
        if let Some(g) = &self.guard {
            g.clear(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // sokf-tests: CLI-4_AC-18
    // sokf-tests: CACHE-2_AC-2
    // sokf-tests: CACHE-2_AC-3
    #[test]
    fn notes_once_per_text_and_never_records_with_the_cache_off() {
        let dir = crate::test_support::fresh_dir("harness-guard");
        let root = RepoRoot::new(dir.clone());
        let name = guard_key("claude", Some("s1"));
        assert_eq!(name, "harness-prompt-claude-s1");
        let file = dir.join(".sokf/cache/harness-prompt-claude-s1.hash");
        // With the cache off, the hook always notes the errors and writes nothing.
        let off = Guard::new(&root, CacheMode::Off, name.clone());
        assert!(off.should_note("report"));
        assert!(!file.exists());
        let on = Guard::new(&root, CacheMode::On, name.clone());
        assert!(on.should_note("report"));
        let kept = fs::read_to_string(&file).unwrap();
        assert!(
            kept.starts_with("fnv1a64:") && kept.len() == "fnv1a64:".len() + 17,
            "{kept:?}"
        );
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/cache/.gitignore")).unwrap(),
            "*\n"
        );
        assert!(!on.should_note("report"));
        // Another session, or another harness, has a guard of its own.
        assert!(
            Guard::new(&root, CacheMode::On, guard_key("claude", Some("s2"))).should_note("report")
        );
        assert!(Guard::new(&root, CacheMode::On, guard_key("codex", None)).should_note("report"));
        // With the cache off, the guard reads nothing and leaves the file as
        // it is.
        assert!(off.should_note("report"));
        off.clear();
        assert_eq!(fs::read_to_string(&file).unwrap(), kept);
        assert!(on.should_note("other"));
        assert_ne!(fs::read_to_string(&file).unwrap(), kept);
        on.clear();
        assert!(!file.exists());
        assert!(on.should_note("other"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_session_keys_by_the_harness() {
        assert_eq!(guard_key("pi", None), "harness-prompt-pi");
        assert_eq!(guard_key("pi", Some("")), "harness-prompt-pi");
    }
}
