//! This module walks the repository. It collects the files and directories
//! under each knowledge root and the other files that a run reads. It also
//! finds the innermost knowledge root that holds a path.

mod collect;
mod tree;

pub use collect::collect;
pub(crate) use collect::{Keep, list_under_root, repo_glob, walk_outside};
pub use tree::{FileKind, Tree, WalkedDir, WalkedFile, innermost};
