//! This module writes the files of a new store for `sokf init`, and edits
//! the config to register a root. It also derives the skeleton that
//! `sokf new` creates a document from.

mod config;
mod elsewhere;
mod fill;
#[cfg(test)]
mod fixtures;
mod front;
mod gate;
mod place;
mod seed;
mod seq;
mod store;
mod words;
mod write;

pub use config::{ConfigChange, register};
pub use fill::{Identity, bind, fill, identity, title};
pub use front::{SokfScaffold, link_example, link_text};
pub use gate::gate;
pub use place::{Site, asset, place, slugify};
pub use seed::{Refused, given_states, skeleton, states};
pub use seq::{Where, given_seq, next_seq, render_id, schema_for};
pub use store::{StoreFiles, store};
pub use tramlines_md::{
    derive::{Refusal, derived_title, no_template},
    grammar::IdTemplate,
};
pub use words::{Words, language};
pub use write::{create_new, write};
