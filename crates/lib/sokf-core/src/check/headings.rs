//! This module tells whether a heading of a document is the heading of a
//! section, as the schema check matches it.

use tramlines_md::check::{HeadingMatch, MatchedBy};

use crate::{Document, grammar::Section};

/// `opens` reports whether the heading `h` of `doc` is the heading of the
/// section `j` of `sections`, the schema's sections, given `matches`, the
/// section that `check::match_headings` matches each heading to: the match
/// gives the heading to the section, and the heading stands at the
/// section's level. A heading that only stands in the place of a heading
/// bound to the frontmatter (`MatchedBy::Place`), which the check repairs,
/// and a heading at another level, which the check reports, open nothing.
pub(crate) fn opens(
    doc: &Document,
    sections: &[Section],
    matches: &[Option<HeadingMatch>],
    h: usize,
    j: usize,
) -> bool {
    let (Some(heading), Some(section)) = (doc.body.headings.get(h), sections.get(j)) else {
        return false;
    };
    let Some(found) = matches.get(h).copied().flatten() else {
        return false;
    };
    found.section == j && heading.level == section.level && found.by != MatchedBy::Place
}
