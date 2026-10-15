//! The `source` module loads files from the repository. It records the line
//! ending of each file, computes the file key, the length, modification time,
//! and hash that identify a file's content for the cache, hashes text, and
//! splits the frontmatter from the body.

mod hash;
mod text;

pub use hash::{Fnv, fnv1a64};
pub use text::{FileKey, LineEnding, Source, Stat, load, load_text, split};
pub(crate) use text::{create_parent, read_optional};
