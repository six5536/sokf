//! This module finds the headings that a link fragment can point to, and
//! gives each one the slug that a repository host gives it.

use std::collections::HashMap;

use tramlines_md::markdown::{Body, Heading};

/// `link_headings` returns every heading of `body` that a link fragment can
/// point to, in line order: the headings of the document, and each heading
/// inside a quote or a list item. The engine reads a heading inside a quote
/// or an item as part of that block, not as a heading of the document
/// (TLMD_173). A git host still gives such a heading an anchor, so a link to
/// it resolves (SOKF_FORMAT_71, SOKF_FORMAT_73).
///
/// Each heading carries the slug that GitHub gives it (SOKF_FORMAT_155). The
/// first heading with a slug keeps it. A later heading with the same slug
/// takes the slug, a hyphen, and a count, and the count rises past each
/// candidate that an earlier heading holds, so no two headings share a slug.
// sokf-implements: SOKF_FORMAT_155
pub fn link_headings(body: &Body) -> Vec<Heading> {
    suffix(in_line_order(body))
}

/// `suffixed_headings` returns the line and the slug of each heading whose
/// slug carries a suffix because an earlier heading holds its plain slug.
pub fn suffixed_headings(body: &Body) -> Vec<(usize, String)> {
    let plain = in_line_order(body);
    suffix(plain.clone())
        .into_iter()
        .zip(plain)
        .filter(|(h, p)| h.slug != p.slug)
        .map(|(h, _)| (h.line, h.slug))
        .collect()
}

/// `in_line_order` returns the headings of `body` and the headings inside
/// its quotes and list items, in line order, with their plain slugs.
fn in_line_order(body: &Body) -> Vec<Heading> {
    let mut out = body.headings.clone();
    out.extend(body.nested_headings());
    out.sort_by_key(|h| h.line);
    out
}

/// `suffix` gives each heading of `out`, in order, the slug that GitHub
/// gives it.
fn suffix(mut out: Vec<Heading>) -> Vec<Heading> {
    let mut taken: HashMap<String, usize> = HashMap::new();
    for heading in &mut out {
        let base = heading.slug.clone();
        let mut slug = base.clone();
        while taken.contains_key(&slug) {
            let count = taken.entry(base.clone()).or_default();
            *count += 1;
            slug = format!("{base}-{count}");
        }
        taken.insert(slug.clone(), 0);
        heading.slug = slug;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::{parse, segments};

    // sokf-tests: SOKF_FORMAT_73
    #[test]
    fn a_heading_inside_a_quote_or_an_item_can_be_named() {
        let text = "# Top\n\n> ## Inner\n>\n> Text.\n\n- ## In item\n";
        let body = parse(text, 1, &segments(text, 1));
        let slugs: Vec<(usize, String)> = link_headings(&body)
            .into_iter()
            .map(|h| (h.line, h.slug))
            .collect();
        assert_eq!(
            slugs,
            vec![
                (1, "top".to_string()),
                (3, "inner".to_string()),
                (7, "in-item".to_string())
            ]
        );
    }

    // sokf-tests: FORMAT_P-7
    proptest::proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        /// The slug of any heading is well formed, and the slug of a slug is
        /// the slug itself.
        #[test]
        fn a_heading_slug_is_idempotent_and_well_formed(heading in "\\PC{0,40}") {
            let once = crate::markdown::slug(&heading);
            proptest::prop_assert_eq!(crate::markdown::slug(&once), once.clone());
            for c in once.chars() {
                proptest::prop_assert!(c.is_alphanumeric() || c == '-' || c == '_');
            }
            proptest::prop_assert_eq!(once.to_lowercase(), once.clone());
        }
    }

    // sokf-tests: SOKF_FORMAT_155
    #[test]
    fn a_repeated_heading_takes_the_suffix_that_github_gives_it() {
        let text = "# Foo\n\n## Foo\n\n## Foo 1\n\n> ## Foo\n";
        let body = parse(text, 1, &segments(text, 1));
        let slugs: Vec<String> = link_headings(&body).into_iter().map(|h| h.slug).collect();
        assert_eq!(slugs, ["foo", "foo-1", "foo-1-1", "foo-2"]);
    }
}
