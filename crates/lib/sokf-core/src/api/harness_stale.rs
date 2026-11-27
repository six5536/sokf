//! This module warns, in `sokf validate`, at each installed part of a
//! harness that is stale, absent, or may not be an edit, so that an agent
//! learns after an upgrade of sokf that its skill or its instructions are out
//! of date (CLI-4).

use std::path::PathBuf;

use agent_harness_adapter_core::{Scope, State};

use super::harness::{HarnessDirs, installed_status, shown, tool};
use crate::{
    Finding, RepoPath, RepoRoot,
    harness::unvouched,
    report::{ABSENT_PART, STALE_PART, UNVOUCHED_PART},
};

/// `stale_parts` returns a warning for each part of a harness installed at
/// the project or the local scope of `root` that is stale, a part that
/// nobody edited and that differs from what `harness install` writes; for
/// each part that is absent, a part whose file, block, or entries are
/// missing and that the configuration doesn't decline; and for each part that
/// is edited and whose record doesn't tell an edit from an earlier version of
/// sokf's content, such as a record of an older sokf. The home directory
/// `home` holds the user's settings, which a harness at the project scope can
/// read, as `harness install` reads them. Any other edited part gets no
/// warning, because the edit is the user's choice. A harness whose state can't
/// be read gets no warning either, because a check of the knowledge doesn't
/// fail on the harness.
// sokf-implements: CLI-4_AC-40
pub(crate) fn stale_parts(root: &RepoRoot, home: Option<PathBuf>) -> Vec<Finding> {
    let dirs = HarnessDirs {
        repo: Some(root.clone()),
        home,
    };
    // The project and the local scopes read the same configuration, so one
    // tool serves both.
    let Ok(tool) = tool(&dirs, Scope::Project) else {
        return Vec::new();
    };
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
        let doubtful = unvouched(&found);
        for h in &found.harnesses {
            for p in &h.parts {
                let Some(path) = RepoPath::parse(&shown(scope, &p.path)) else {
                    continue;
                };
                let install = format!("sokf harness install {}{local}", h.harness);
                let (key, message) = match p.state {
                    State::Stale => (
                        STALE_PART,
                        format!(
                            "the {} part of the {} harness is stale: nobody edited it, and it differs from what sokf now writes; run `{install}` to refresh it",
                            p.part, h.harness
                        ),
                    ),
                    // sokf-implements: CLI-4_AC-42
                    State::Absent => (
                        ABSENT_PART,
                        format!(
                            "the {} part of the {} harness is absent; run `{install}` to write it",
                            p.part, h.harness
                        ),
                    ),
                    // sokf-implements: CLI-4_AC-118
                    State::Edited => {
                        let Some(u) = doubtful
                            .iter()
                            .find(|u| u.harness == h.harness && u.part == p.part)
                        else {
                            continue;
                        };
                        // Only a record of an older sokf names the forced install: a
                        // record of a later sokf, or under other rules, can hold a
                        // later sokf's content, which the install would replace with
                        // this one's older content.
                        let check = format!("check `sokf harness diff {}{local}`", h.harness);
                        let next = if u.older() {
                            format!(
                                "{check}, then run `{install} --force {}` unless the difference holds an edit of the project's",
                                p.part
                            )
                        } else {
                            check
                        };
                        (
                            UNVOUCHED_PART,
                            format!(
                                "the {} part of the {} harness may not be an edit, as {}; {next}",
                                p.part,
                                h.harness,
                                u.cause()
                            ),
                        )
                    }
                    _ => continue,
                };
                out.push(Finding::warning(path, None, key, message));
            }
        }
    }
    out
}
