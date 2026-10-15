//! This module writes the files of a new store for `sokf init`, and edits
//! the config to register a root. It also derives the document that
//! `sokf new` creates from a scaffold.

mod config;
mod fill;
#[cfg(test)]
mod fixtures;
mod front;
mod gate;
mod place;
mod seed;
mod seq;
mod store;

mod write;

pub use config::{ConfigChange, register};
pub use fill::{Identity, bind, fill, identity};
pub use gate::gate;
pub use place::{Site, asset, place, slugify};
pub use seed::{seed, states};
pub use seq::{next_seq, render_id, schema_for};
pub use store::{StoreFiles, store};
pub use tramlines_md::derive::{Refusal, derived_title, no_template};
pub use tramlines_md::grammar::IdTemplate;
pub use write::{create_new, write};

pub use front::SokfScaffold;
