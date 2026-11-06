//! This module keeps `sokf harness install` and `sokf pack sync` off each
//! other's files (CLI-3, CLI-13): it refuses an install whose part would
//! write a file that the pack lock records, and lists the paths that each
//! installed harness writes, on which no file of a pack lands.

use std::collections::BTreeMap;

use agent_harness_adapter_core::{Item, Scope};

use super::harness::{HarnessDirs, HarnessOptions, harness_ids, refusal, shown, tool};
use crate::{Error, Result, harness::SokfTool, pack};

/// `refuse_pack_files` refuses an install at the project or the local scope
/// that would write a part to a file that the pack lock records, and names
/// the pack, because `pack sync` and `harness install` would then overwrite
/// each other's file.
// sokf-implements: CLI-3_AC-81
pub(super) fn refuse_pack_files(
    dirs: &HarnessDirs,
    tool: &SokfTool,
    opts: &HarnessOptions,
) -> Result<()> {
    let Some(repo) = &dirs.repo else {
        return Ok(());
    };
    if opts.scope == Scope::User {
        return Ok(());
    }
    let lock = pack::read_lock(repo)?;
    if lock.packs.is_empty() {
        return Ok(());
    }
    let found = agent_harness_adapter_core::status(tool, harness_ids(&opts.harnesses), opts.scope)
        .map_err(refusal(opts.scope))?;
    for h in &found.harnesses {
        for p in &h.parts {
            let written = part_path(opts.scope, p);
            let owned = lock.packs.iter().find_map(|(name, pack)| {
                pack.files
                    .values()
                    .find(|f| pack::covers(&written, &f.path))
                    .map(|f| (name, &f.path))
            });
            if let Some((pack, path)) = owned {
                return Err(Error::Harness(format!(
                    "{path}: the {} part of {} would write a file of the pack {pack}; remove the file from the pack, or decline the part in /.sokf/config.toml",
                    p.part, h.harness
                )));
            }
        }
    }
    Ok(())
}

/// `part_path` returns what the part `p` writes at `scope`, from the root of
/// the scope: its file, or for the skills part, the directory of sokf's own
/// skill, with a trailing `/`, because the skills part writes only that
/// directory of the harness's directory of skills.
fn part_path(scope: Scope, p: &agent_harness_adapter_core::PartResult) -> String {
    let path = shown(scope, &p.path);
    if p.part == Item::Skills.as_str() {
        format!("{}/sokf/", path.trim_end_matches('/'))
    } else {
        path
    }
}

/// `written_paths` returns each path of the repository that a part of a
/// harness installed at the project or the local scope writes, from the
/// repository root, with what writes it. A path that ends with `/` stands
/// for every file under it.
pub(crate) fn written_paths(dirs: &HarnessDirs) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if dirs.repo.is_none() {
        return Ok(out);
    }
    for scope in [Scope::Project, Scope::Local] {
        let tool = tool(dirs, scope)?;
        let installed =
            agent_harness_adapter_core::installed(&tool, scope).map_err(refusal(scope))?;
        if installed.is_empty() {
            continue;
        }
        let found =
            agent_harness_adapter_core::status(&tool, &installed, scope).map_err(refusal(scope))?;
        for h in found.harnesses {
            for p in &h.parts {
                let path = part_path(scope, p);
                if path.starts_with('/') {
                    out.insert(
                        path,
                        format!("the {} part of the {} harness", p.part, h.harness),
                    );
                }
            }
        }
    }
    Ok(out)
}
