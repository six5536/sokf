//! The `config` module finds the repository root, loads `/.sokf/config.toml`,
//! the drop-in folders `/.sokf/roots.d/` and `/.sokf/formats.d/`, and
//! `/.sokf/formats.yaml`, and resolves the `PATHS` scope of a command.

mod d_folder;
mod file;
mod formats;
mod formats_d;
mod root;
mod roots_d;

pub use file::{
    Config, ConfigText, FormatConfig, MarkerCheck, config_path, load_config, read_lenient,
};
pub use formats::{FORMATS_DIR, RepoFormats, load_repo_formats};
pub use root::{PathArg, Scope, discover_root, resolve_paths};
pub use roots_d::registry_files_lenient;
