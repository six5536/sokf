//! The pack module copies a schema pack into a knowledge store and keeps it
//! current: it reads the pack's manifest, records the pack in the lock,
//! fetches it through the user's `git`, and merges each later upstream
//! change with the store's edits through `git merge-file` (design-pack).

mod eol;
mod git;
mod landing;
mod lock;
mod manifest;
mod mode;
mod order;
mod owner;
mod plan;
mod source;

pub use eol::{is_base, normalise};
pub use git::{TempDir, diff, merge_file, shallow_clone};
pub use landing::{INSTRUCTION_FILES, covers};
pub use lock::{
    LOCK_VERSION, Lock, LockedFile, LockedPack, PACKS_DIR, base_path, formats_of, fragments,
    hash_of, instructions_of, lock_path, lock_text, pack_dir, read_lock,
};
pub use manifest::{MANIFEST, PackManifest, is_slug, parse_manifest, requires_newer, tool_version};
pub use mode::{file_is_executable, set_executable};
pub use order::precedence;
pub use owner::{Ownership, Shadow, Supply, Takeover, owners};
pub use plan::{
    Fetched, FileAction, PackPlan, Request, locked_types, plan, schema_type, store_of, supply,
};
pub use source::{
    BUILTIN, EMBEDDED, EmbeddedPack, FILES, PackFiles, Source, clone_dir, destination,
    embedded_packs, find_embedded, read_dir,
};
