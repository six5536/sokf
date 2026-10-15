//! This module guards the stop hook against blocking twice on the same
//! report. It keeps the hash of the last report that the hook blocked on, for
//! each harness and session, in a file under `/.sokf/cache/` (CACHE-2_AC-2),
//! through the loop guard of agent-harness-adapter.

use agent_harness_adapter_core::hook::LoopGuard;

use crate::{CacheMode, RepoRoot, cache::CACHE_DIR};

// sokf:begin guard-file
/// `guard_key` returns the key of the stop hook of the harness `harness` in the
/// session `session` in the loop guard, which keeps the hash of each key in
/// `/.sokf/cache/<key>.hash`: `harness-stop-<harness>-<session>`, or
/// `harness-stop-<harness>` when the harness gives no session. The loop guard
/// names the file by the key's hash instead when the key is longer than 64
/// characters or holds a character other than a letter, a digit, `-`, or `_`.
pub fn guard_key(harness: &str, session: Option<&str>) -> String {
    match session {
        Some(s) if !s.is_empty() => format!("harness-stop-{harness}-{s}"),
        _ => format!("harness-stop-{harness}"),
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
    /// `should_block` reports whether the hook should block on the report
    /// `text`. It returns `false` when the guard file holds the hash of the same
    /// text, and otherwise records the hash and returns `true`. Like every write
    /// to the cache, the record ignores a failure, and it creates the cache
    /// directory with its `.gitignore`. When the cache is off, the guard reads
    /// and writes nothing and returns `true`, so the hook blocks on every report
    /// unless the agent already continues because of a stop hook.
    // sokf-implements: CACHE-2_AC-2
    // sokf-implements: CACHE-2_AC-3
    // sokf-implements: CLI-4_AC-18
    // sokf-implements: CLI-1_AC-13
    pub fn should_block(&self, text: &str) -> bool {
        self.guard
            .as_ref()
            .is_none_or(|g| g.should_block(&self.key, text))
    }
    // sokf:end guard-write

    /// `clear` forgets the last report, once no error remains, so that the guard
    /// file of a session that ends clean goes away.
    pub fn clear(&self) {
        if let Some(g) = &self.guard {
            g.clear(&self.key);
        }
    }

    /// `is_on` reports whether the guard reads and writes its file.
    pub fn is_on(&self) -> bool {
        self.guard.is_some()
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
    fn blocks_once_per_text_and_never_records_with_the_cache_off() {
        let dir = crate::test_support::fresh_dir("harness-guard");
        let root = RepoRoot::new(dir.clone());
        let key = guard_key("claude", Some("s1"));
        assert_eq!(key, "harness-stop-claude-s1");
        let file = dir.join(".sokf/cache/harness-stop-claude-s1.hash");
        // With the cache off, the hook always blocks and writes nothing.
        let off = Guard::new(&root, CacheMode::Off, key.clone());
        assert!(!off.is_on());
        assert!(off.should_block("report"));
        assert!(!file.exists());
        let on = Guard::new(&root, CacheMode::On, key.clone());
        assert!(on.should_block("report"));
        let kept = fs::read_to_string(&file).unwrap();
        assert!(
            kept.starts_with("fnv1a64:") && kept.len() == "fnv1a64:".len() + 17,
            "{kept:?}"
        );
        assert_eq!(
            fs::read_to_string(dir.join(".sokf/cache/.gitignore")).unwrap(),
            "*\n"
        );
        assert!(!on.should_block("report"));
        // Another session, or another harness, has a guard of its own.
        assert!(
            Guard::new(&root, CacheMode::On, guard_key("claude", Some("s2")))
                .should_block("report")
        );
        assert!(Guard::new(&root, CacheMode::On, guard_key("codex", None)).should_block("report"));
        // With the cache off, the guard reads nothing and leaves the file as
        // it is.
        assert!(off.should_block("report"));
        off.clear();
        assert_eq!(fs::read_to_string(&file).unwrap(), kept);
        assert!(on.should_block("other"));
        assert_ne!(fs::read_to_string(&file).unwrap(), kept);
        on.clear();
        assert!(!file.exists());
        assert!(on.should_block("other"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_session_keys_by_the_harness() {
        assert_eq!(guard_key("pi", None), "harness-stop-pi");
        assert_eq!(guard_key("pi", Some("")), "harness-stop-pi");
    }
}
