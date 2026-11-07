//! This module warns, in `sokf validate`, at each installed part of a
//! harness that is stale, so that an agent learns after an upgrade of sokf
//! that its skill or its instructions are out of date (CLI-4).

use std::path::PathBuf;

use agent_harness_adapter_core::{Item, Scope, State};

use super::{
    harness::{HarnessDirs, installed_status, shown, tool},
    harness_packs::conflicted_fragments,
};
use crate::{
    Finding, RepoPath, RepoRoot,
    report::{ABSENT_PART, STALE_PART},
};

/// `stale_parts` returns a warning for each part of a harness installed at
/// the project or the local scope of `root` that is stale, a part that
/// nobody edited and that differs from what `harness install` writes, and
/// for each part that is absent: a part whose file, block, or entries are
/// missing and that the configuration doesn't decline. The
/// home directory `home` holds the user's settings, which a harness at the
/// project scope can read, as `harness install` reads them. An edited part
/// gets no warning, because the edit is the user's choice. A harness whose
/// state can't be read gets no warning either, because a check of the
/// knowledge doesn't fail on the harness. While a pack's fragment holds a
/// conflict, the warning at an instructions part says to resolve it first,
/// because an install would copy the conflict into the block.
// sokf-implements: CLI-4_AC-40
pub(crate) fn stale_parts(root: &RepoRoot, home: Option<PathBuf>) -> Vec<Finding> {
    let dirs = HarnessDirs {
        repo: Some(root.clone()),
        home,
    };
    // The project and the local scopes read the same configuration and
    // fragments, so one tool serves both.
    let Ok(tool) = tool(&dirs, Scope::Project) else {
        return Vec::new();
    };
    let mut conflicted: Option<Vec<String>> = None;
    let mut out = Vec::new();
    for scope in [Scope::Project, Scope::Local] {
        let Ok(Some(found)) = installed_status(&tool, scope) else {
            continue;
        };
        let local = if scope == Scope::Local {
            " --scope local"
        } else {
            ""
        };
        for h in &found.harnesses {
            for p in &h.parts {
                let Some(path) = RepoPath::parse(&shown(scope, &p.path)) else {
                    continue;
                };
                let (key, mut message, verb) = match p.state {
                    State::Stale => (
                        STALE_PART,
                        format!(
                            "the {} part of the {} harness is stale: nobody edited it, and it differs from what sokf now writes; ",
                            p.part, h.harness
                        ),
                        "refresh",
                    ),
                    // sokf-implements: CLI-4_AC-42
                    State::Absent => (
                        ABSENT_PART,
                        format!(
                            "the {} part of the {} harness is absent; ",
                            p.part, h.harness
                        ),
                        "write",
                    ),
                    _ => continue,
                };
                let conflicts = if p.part == Item::Instructions.as_str() {
                    conflicted
                        .get_or_insert_with(|| conflicted_fragments(root).unwrap_or_default())
                        .as_slice()
                } else {
                    &[]
                };
                if !conflicts.is_empty() {
                    message.push_str(&format!(
                        "resolve the conflict in {} first, then ",
                        conflicts.join(", ")
                    ));
                }
                message.push_str(&format!(
                    "run `sokf harness install {}{local}` to {verb} it",
                    h.harness
                ));
                out.push(Finding::warning(path, None, key, message));
            }
        }
    }
    out
}
