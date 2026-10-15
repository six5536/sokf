//! This module holds `SokfScaffold`, the host that writes the frontmatter
//! of a schema's scaffold when the derivation asks for it. The host first
//! writes `type`, `id`, and `title`, which are the core fields of the SOKF
//! format specification. It writes the edges last: the edges are the
//! entries of the frontmatter's `links` field.

use tramlines_core::frontmatter::Host as FrontmatterHost;
use tramlines_md::{
    check::{Host, ScaffoldFront, Subject},
    markdown::Dialect,
};

use crate::{
    grammar::{Dispatch, SokfSchema, States},
    markdown::SOKF,
    model::frontmatter::CORE_KEYS,
};

/// `SokfScaffold` is the host for the derivation of one schema's scaffold:
/// it tells the derivation the dialect, the reserved keys, and the
/// frontmatter to write. A scaffold has no files or regions around it, so
/// the host reports every image and region as unknown.
pub struct SokfScaffold<'a> {
    schema: &'a SokfSchema,
}

impl<'a> SokfScaffold<'a> {
    /// `new` returns the host for the derivation of `schema`.
    pub fn new(schema: &'a SokfSchema) -> Self {
        SokfScaffold { schema }
    }
}

/// `slugified` returns a type name as a slug: lower case, with a hyphen
/// before each capital letter after the first. For an empty name,
/// `slugified` returns `document`.
fn slugified(type_name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in type_name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(ch.to_lowercase());
    }
    if out.is_empty() {
        "document".into()
    } else {
        out
    }
}

/// `guidance_comment` returns a single YAML comment line, marked
/// `sokf:guidance`, that holds the whole text. Each run of whitespace in
/// the text becomes one space.
fn guidance_comment(text: &str) -> String {
    let one: Vec<&str> = text.split_whitespace().collect();
    format!("# {} {}", SOKF.spell("guidance"), one.join(" "))
}

impl FrontmatterHost for SokfScaffold<'_> {
    fn scalar_format(&self, _: &str, _: &str) -> Option<Result<(), String>> {
        None
    }

    fn reserved_keys(&self) -> &[&str] {
        CORE_KEYS
    }
}

impl Host for SokfScaffold<'_> {
    fn dialect(&self) -> Dialect {
        SOKF
    }

    fn image(&self, _: &str) -> Subject {
        Subject::Unknown
    }

    fn region(&self, _: &str) -> Subject {
        Subject::Unknown
    }

    /// `scaffold_front` writes `type`, `id`, and `title` first. The type is
    /// the schema's type name. The id is the schema's id template rendered
    /// with the number 1 and, when the template has a slug, the slug
    /// `example`. If the template fails to render, the id is the type's
    /// slug. For a schema with no id template, the id is `<type>-example`.
    /// The title is the slug `example` written as words, or the whole id
    /// when the template has no slug. `sokf new` fills in the same title
    /// when no title is given. An additional file has no frontmatter, so
    /// `scaffold_front` returns `None` for its schema.
    fn scaffold_front(&self, states: &States) -> Option<ScaffoldFront> {
        let Dispatch::Type(type_name) = &self.schema.dispatch else {
            return None;
        };
        let template = self.schema.core.core.id_template();
        let id = match template {
            Some(t) => t
                .render_with(1, t.has_slug().then_some("example"), states)
                .unwrap_or_else(|_| slugified(type_name)),
            None => format!("{}-example", slugified(type_name)),
        };
        let slug = match template {
            Some(t) if t.has_slug() => "example".to_string(),
            _ => id.clone(),
        };
        let title = crate::scaffold::derived_title(&slug);
        // The scaffold writes each optional edge once, just as it writes an
        // optional key. The written edge shows the author each link that the
        // schema allows.
        // Validation reports the edge's placeholder target until the
        // author replaces it, so the author either fills in the edge or
        // deletes it.
        // sokf-implements: EXT-8_AC-1
        let mut trailing = Vec::new();
        let edges: Vec<_> = self
            .schema
            .links
            .iter()
            .filter(|r| r.common.only.admits(states))
            .collect();
        if !edges.is_empty() {
            trailing.push("links:".to_string());
            for rule in edges {
                for _ in 0..rule.min.max(1) {
                    if let Some(g) = &rule.guidance {
                        trailing.push(format!("  {}", guidance_comment(g)));
                    }
                    trailing.push(format!("  - rel: {}", rule.rel));
                    trailing.push("    to: target-id".to_string());
                }
            }
        }
        Some(ScaffoldFront {
            leading: vec![
                ("type".into(), Some(type_name.clone())),
                ("id".into(), Some(id)),
                ("title".into(), Some(title)),
            ],
            trailing,
        })
    }
}
