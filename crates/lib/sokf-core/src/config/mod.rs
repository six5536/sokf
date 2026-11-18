//! The `config` module finds the repository root, loads `/.sokf/config.toml`,
//! `/.sokf/formats.yaml`, `/.sokf/texts.yaml`, and the drop-in folders
//! `/.sokf/roots.d/`, `/.sokf/excludes.d/`, `/.sokf/formats.d/`, and
//! `/.sokf/texts.d/`, and resolves the `PATHS` scope of a command.

mod d_folder;
mod excludes_d;
mod file;
mod formats;
mod formats_d;
mod root;
mod roots_d;
mod texts;

pub use excludes_d::{AddedExclude, excludes_files_lenient};
pub use file::{
    Config, ConfigText, FormatConfig, MarkerCheck, config_path, load_config, read_lenient,
};
pub use formats::{FORMATS_DIR, RepoFormats, load_repo_formats};
pub use root::{PathArg, Scope, discover_root, resolve_paths};
pub use roots_d::registry_files_lenient;
#[cfg(test)]
pub(crate) use texts::UNITS;
pub(crate) use texts::{LABELS, fill_template, placeholders};
pub use texts::{Texts, load_texts, texts_files_lenient};
