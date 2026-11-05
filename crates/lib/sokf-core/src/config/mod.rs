//! The `config` module finds the repository root, loads `/.sokf/config.toml`
//! and `/.sokf/formats.yaml`, and resolves the `PATHS` scope of a command.

mod file;
mod formats;
mod root;

pub use file::{Config, ConfigText, MarkerCheck, config_path, load_config, read_lenient};
pub use formats::{RepoFormats, load_repo_formats};
pub use root::{PathArg, Scope, discover_root, resolve_paths};
