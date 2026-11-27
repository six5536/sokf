//! This module names a text for same-same.

use crate::RepoPath;

/// `name_of` returns the name under which same-same reads a text of the
/// file at `path`: its path from the repository's root, so that a pattern of
/// two parts, such as `.vscode/*.json`, reads by the file's folder too
/// (CLI-4_AC-113).
pub(crate) fn name_of(path: &RepoPath) -> String {
    path.as_str().trim_start_matches('/').to_string()
}
