//! This module finds each part that agent-harness-adapter reports `edited` with
//! its warning that the record doesn't tell an edit from an earlier version of
//! sokf's content, such as a part whose record an older sokf wrote, so that the
//! report gives that warning in sokf's words (CLI-3_AC-113) and `validate` warns
//! at the part (CLI-4_AC-118). The adapter gives each warning as text, and lets
//! a tool give neither its command nor its words for it, so the module finds the
//! warning by the adapter's words for each part that the adapter reports
//! `edited`.

use agent_harness_adapter_core::{InstallResult, State};

/// `OLDER` is the adapter's reason for a record that an older version wrote,
/// the one reason for which sokf's content is the later one.
const OLDER: &str = "its record is from an older version";

/// `CAUSES` maps each reason of the adapter why its record can't tell an edit
/// from what it wrote to sokf's words for it.
const CAUSES: [(&str, &str); 4] = [
    (OLDER, "an older sokf recorded it"),
    (
        "its record is from a later version",
        "a later sokf recorded it",
    ),
    (
        "its record was made under other rules",
        "another version of sokf recorded it",
    ),
    (
        "its record doesn't fit the part",
        "the record of the harnesses doesn't fit it",
    ),
];

/// `Unvouched` is a part that the adapter reports `edited`, and whose record
/// doesn't tell an edit from an earlier version of sokf's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unvouched {
    /// The harness, for example `claude`.
    pub harness: String,
    /// The part, for example `skills`.
    pub part: String,
    /// The part's path, relative to the root of the scope, as the adapter
    /// gives it.
    pub path: String,
    /// Why the record can't tell, in the adapter's words, for example `its
    /// record is from an older version`.
    pub reason: String,
    /// The position of the adapter's warning in the result's warnings.
    pub warning: usize,
}

impl Unvouched {
    /// `cause` returns sokf's words for the reason, or the adapter's reason as
    /// it stands where sokf doesn't know it.
    pub fn cause(&self) -> &str {
        CAUSES
            .iter()
            .find(|(reason, _)| *reason == self.reason)
            .map_or(self.reason.as_str(), |(_, cause)| cause)
    }

    /// `older` reports whether an older sokf recorded the part, so that sokf's
    /// content is the later one, which a forced install writes.
    pub fn older(&self) -> bool {
        self.reason == OLDER
    }

    /// `words` returns the warning in sokf's words, with the part's path as
    /// the report shows it, `shown`, and without the adapter's remedy, which
    /// names no command of sokf's (CLI-3_AC-113).
    pub fn words(&self, shown: &str) -> String {
        format!(
            "{}: {} in {shown} may not be an edit, as {}",
            self.harness,
            self.part,
            self.cause()
        )
    }
}

/// `unvouched` returns each part of `result` that the adapter reports
/// `edited` and leaves as it found it, with its warning that the part may not
/// be an edit, in the order of the report. The adapter words that warning
/// `<harness>: <part> in <path> may not be an edit, as <reason>; <remedy>`.
pub(crate) fn unvouched(result: &InstallResult) -> Vec<Unvouched> {
    let mut out = Vec::new();
    for h in &result.harnesses {
        for p in &h.parts {
            if p.state != State::Edited || p.action.is_some() {
                continue;
            }
            let prefix = format!(
                "{}: {} in {} may not be an edit, as ",
                h.harness, p.part, p.path
            );
            let found = result
                .warnings
                .iter()
                .enumerate()
                .find_map(|(i, w)| w.strip_prefix(&prefix).map(|rest| (i, rest)));
            if let Some((warning, rest)) = found {
                let reason = rest.split_once("; ").map_or(rest, |(reason, _)| reason);
                out.push(Unvouched {
                    harness: h.harness.clone(),
                    part: p.part.clone(),
                    path: p.path.clone(),
                    reason: reason.to_string(),
                    warning,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use agent_harness_adapter_core::Scope;

    use super::*;
    use crate::harness::SokfTool;

    fn status(dir: &Path) -> InstallResult {
        let tool = SokfTool::new(
            Some(dir.to_path_buf()),
            None,
            std::collections::BTreeMap::default(),
        );
        agent_harness_adapter_core::status(&tool, ["claude"], Scope::Project).unwrap()
    }

    // sokf-tests: CLI-3_AC-113
    #[test]
    fn finds_the_edited_part_whose_record_is_of_an_older_sokf() {
        let dir = crate::test_support::fresh_dir("harness-unvouched");
        fs::create_dir_all(dir.join(".claude/skills/sokf")).unwrap();
        fs::write(
            dir.join(".claude/skills/sokf/SKILL.md"),
            "---\nname: sokf\n---\nold\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        // A record of sokf 0.8, whose value the adapter can't use.
        fs::write(
            dir.join(".sokf/harness.toml"),
            "[claude]\nskills = \"fnv1a64:0000000000000001\"\n",
        )
        .unwrap();
        let result = status(&dir);
        let found = unvouched(&result);
        assert_eq!(found.len(), 1, "{result:?}");
        let u = &found[0];
        assert_eq!(
            (u.harness.as_str(), u.part.as_str(), u.path.as_str()),
            ("claude", "skills", ".claude/skills")
        );
        assert_eq!(u.reason, "its record is from an older version");
        assert_eq!(
            u.words("/.claude/skills"),
            "claude: skills in /.claude/skills may not be an edit, as an older sokf recorded it"
        );
        assert!(u.older());
        // A reason that sokf doesn't know passes through as it stands.
        let other = Unvouched {
            reason: "the moon is full".into(),
            ..u.clone()
        };
        assert_eq!(other.cause(), "the moon is full");
        assert!(!other.older());
        assert!(result.warnings[u.warning].starts_with("claude: skills in .claude/skills "));
        // An edited part without a record is an edit: the adapter doesn't warn.
        fs::remove_file(dir.join(".sokf/harness.toml")).unwrap();
        assert_eq!(unvouched(&status(&dir)), []);
        fs::remove_dir_all(&dir).unwrap();
    }
}
