//! This module forms the ids that a key's text, as a writer gives it in a
//! marker, a fragment, or a reference, can name, as tramlines forms the id
//! of a key.

use tramlines_md::markdown::{plain_text, slug};

/// `key_ids` returns the ids that a key given as `text` can have, in the
/// order to try them: the slug of the text itself, which is the id of a key
/// that a line of a fence captures, then the slug of its plain text,
/// without the markers of emphasis, code, and links, which is the id of a
/// key that a heading or a unit of prose captures, as `Capture::id` forms
/// them (`TLMD_101`). The slug of the text comes first, so that a reference
/// that found a key by it still finds that key. The id of a key that sokf
/// records comes from the anchor that the key holds.
pub fn key_ids(text: &str) -> Vec<String> {
    let written = slug(text);
    let plain = slug(&plain_text(text));
    if plain == written {
        vec![written]
    } else {
        vec![written, plain]
    }
}

#[cfg(test)]
mod tests {
    use super::key_ids;

    #[test]
    fn the_ids_of_a_key_read_past_its_markup_and_keep_its_text() {
        assert_eq!(key_ids("CLI-1_AC-2"), ["cli-1_ac-2"]);
        assert_eq!(key_ids("_Short_"), ["_short_", "short"]);
        assert_eq!(key_ids("*Short*"), ["short"]);
        assert_eq!(key_ids("`Short code`"), ["short-code"]);
        assert_eq!(key_ids("Option<T>"), ["optiont", "option"]);
    }
}
