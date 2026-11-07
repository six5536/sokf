//! This module implements `sokf pack accept`, which records that a project
//! keeps an edit or a deletion of a pack's file on purpose, so that `pack
//! status --check` passes on the file (CLI-13). It writes only the lock and
//! uses no network.

use std::path::PathBuf;

use serde::Serialize;

use super::pack::{is_edited, write_lock};
use crate::{
    Error, RepoPath, RepoRoot, Result, check,
    config::PathArg,
    pack::{self, Accepted},
};

/// `PackAcceptOptions` holds the options of `pack accept`.
#[derive(Debug, Clone, Default)]
pub struct PackAcceptOptions {
    /// The files to accept, or whose acceptance to withdraw, as the user
    /// wrote them.
    pub paths: Vec<String>,
    /// Why the project keeps the edits.
    pub note: Option<String>,
    /// Whether to withdraw the acceptances instead.
    pub withdraw: bool,
    /// The directory from which a relative path is read.
    pub cwd: PathBuf,
}

/// A `PackAccepted` is the report of `pack accept`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PackAccepted {
    /// Each file whose edit the command accepted.
    pub accepted: Vec<String>,
    /// Each file whose acceptance the command withdrew.
    pub withdrawn: Vec<String>,
}

impl PackAccepted {
    /// `to_text` returns a line for each file.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for p in &self.accepted {
            out.push_str(&format!("accepted {p}\n"));
        }
        for p in &self.withdrawn {
            out.push_str(&format!("withdrew the acceptance of {p}\n"));
        }
        if out.is_empty() {
            out.push_str("no acceptance changed\n");
        }
        out
    }
}

/// `pack_accept` records in the lock that the project keeps each file of
/// `options.paths` as it stands, with the note, or withdraws the acceptance
/// of each. It refuses, before it writes, a missing note, a path that no
/// pack in the lock owns, a file that holds its base, and a file that holds
/// a conflict.
// sokf-implements: CLI-13_AC-85
// sokf-implements: CLI-13_AC-86
// sokf-implements: CLI-13_AC-87
// sokf-implements: CLI-13_AC-88
// sokf-implements: CLI-13_AC-89
// sokf-implements: CLI-13_AC-90
pub fn pack_accept(root: &RepoRoot, options: &PackAcceptOptions) -> Result<PackAccepted> {
    let note = options.note.as_deref().map(str::trim).unwrap_or_default();
    if !options.withdraw && note.is_empty() {
        return Err(Error::Pack(
            "an acceptance needs a note; pass --note with the reason that the project keeps the edit"
                .into(),
        ));
    }
    let mut lock = pack::read_lock(root)?;
    let mut refused = Vec::new();
    let mut targets: Vec<(RepoPath, String, String, Option<Accepted>)> = Vec::new();
    for arg in &options.paths {
        let Some(dest) = PathArg(arg).resolve_new(root, &options.cwd) else {
            refused.push(format!("{arg}: not a path in the repository"));
            continue;
        };
        let Some((name, rel)) = lock
            .owner_of(dest.as_str())
            .map(|(n, r)| (n.to_string(), r.to_string()))
        else {
            refused.push(format!("{dest}: no pack in the lock owns the file"));
            continue;
        };
        // A file that two arguments name is accepted once.
        if targets.iter().any(|(d, ..)| *d == dest) {
            continue;
        }
        if options.withdraw {
            targets.push((dest, name, rel, None));
            continue;
        }
        if !is_edited(root, &rel, &lock.packs[&name].files[&rel]) {
            refused.push(format!(
                "{dest}: the file holds the pack's version, so there is no edit to accept"
            ));
            continue;
        }
        let path = dest.to_fs(root);
        let bytes = match std::fs::read(&path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            // A file that exists and can't be read is no deletion.
            Err(e) => {
                refused.push(format!("{dest}: the file can't be read: {e}"));
                continue;
            }
        };
        if let Some(b) = &bytes
            && !check::conflict_lines(&dest, &String::from_utf8_lossy(b)).is_empty()
        {
            refused.push(format!(
                "{dest}: the file holds a conflict; resolve it, then accept it"
            ));
            continue;
        }
        let bit = bytes.is_some() && pack::carries_mode(&rel) && pack::file_is_executable(&path);
        targets.push((
            dest,
            name,
            rel,
            Some(Accepted::of(bytes.as_deref(), bit, note)),
        ));
    }
    if !refused.is_empty() {
        return Err(Error::Pack(refused.join("; ")));
    }
    let mut report = PackAccepted::default();
    let mut changed = false;
    for (dest, name, rel, accepted) in targets {
        let file = lock
            .packs
            .get_mut(&name)
            .and_then(|p| p.files.get_mut(&rel))
            .ok_or_else(|| Error::Internal(format!("no record of {dest}")))?;
        match accepted {
            None if file.accepted.take().is_some() => {
                report.withdrawn.push(dest.to_string());
                changed = true;
            }
            None => {}
            Some(a) => {
                changed |= file.accepted.as_ref() != Some(&a);
                file.accepted = Some(a);
                report.accepted.push(dest.to_string());
            }
        }
    }
    if changed {
        write_lock(root, &lock)?;
    }
    Ok(report)
}
