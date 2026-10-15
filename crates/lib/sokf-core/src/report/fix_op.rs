//! `FixOp` records one change that `--fix` made. It is plain data that
//! passes through the cache.

use serde::{Deserialize, Serialize};

use crate::RepoPath;

/// `FixOp` is one operation that `validate --fix` performed (CLI-4_AC-26).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum FixOp {
    /// `Moved` records a document that `--fix` moved to its derived directory.
    Moved {
        /// `from` is the path of the document before the move.
        from: RepoPath,
        /// `to` is the path of the document after the move.
        to: RepoPath,
    },
    /// `Created` records an index that `--fix` wrote where none existed
    /// (SOKF_VALIDATION_33).
    Created {
        /// `path` is the path of the index that `--fix` wrote.
        path: RepoPath,
    },
    /// `Rewrote` records a file whose bytes `--fix` changed.
    Rewrote {
        /// `path` is the path of the file that `--fix` rewrote.
        path: RepoPath,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    #[test]
    fn fix_ops_serialize_with_an_op_tag() {
        let moved = FixOp::Moved {
            from: p("/a.md"),
            to: p("/b.md"),
        };
        assert_eq!(
            serde_json::to_string(&moved).unwrap(),
            r#"{"op":"moved","from":"/a.md","to":"/b.md"}"#
        );
        let rewrote = FixOp::Rewrote { path: p("/a.md") };
        assert_eq!(
            serde_json::to_string(&rewrote).unwrap(),
            r#"{"op":"rewrote","path":"/a.md"}"#
        );
        let created = FixOp::Created {
            path: p("/k/index.md"),
        };
        assert_eq!(
            serde_json::to_string(&created).unwrap(),
            r#"{"op":"created","path":"/k/index.md"}"#
        );
    }
}
