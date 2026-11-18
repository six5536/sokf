//! This module holds `SokfScaffold`, the host that writes the frontmatter
//! of a schema's scaffold when the derivation asks for it. The host first
//! writes `type`, `id`, and `title`, which are the core fields of the SOKF
//! format specification. It writes the edges last: the edges are the
//! entries of the frontmatter's `links` field. It words each limit and each
//! label in the language of the schema, from sokf's texts.

use tramlines_core::frontmatter::Host as FrontmatterHost;
use tramlines_md::{
    check::{Host, LabelFacts, LimitFacts, ScaffoldFront, Subject},
    markdown::Dialect,
};

use super::Words;
use crate::{
    grammar::{Dispatch, LimitKind, LinkRule, SokfSchema, States, squash},
    markdown::SOKF,
    model::frontmatter::CORE_KEYS,
};

/// `SokfScaffold` is the host for the derivation of one schema's scaffold:
/// it tells the derivation the dialect, the reserved keys, the frontmatter
/// to write, and the words before each limit's `when-at-limit`. A scaffold
/// has no files or regions around it, so the host reports every image and
/// region as unknown.
pub struct SokfScaffold<'a> {
    schema: &'a SokfSchema,
    words: Option<&'a Words<'a>>,
}

impl<'a> SokfScaffold<'a> {
    /// `new` returns the host for the derivation of `schema`, which gives
    /// no words for a limit, so that the engine writes each limit in the
    /// syntax of the schema.
    pub fn new(schema: &'a SokfSchema) -> Self {
        SokfScaffold {
            schema,
            words: None,
        }
    }

    /// `with_words` returns the host for the derivation of `schema`, which
    /// words each limit with `words`, sokf's texts in the language of the
    /// schema.
    pub fn with_words(schema: &'a SokfSchema, words: &'a Words<'a>) -> Self {
        SokfScaffold {
            schema,
            words: Some(words),
        }
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
pub(super) fn guidance_comment(text: &str) -> String {
    let one: Vec<&str> = text.split_whitespace().collect();
    format!("# {} {}", SOKF.spell("guidance"), one.join(" "))
}

/// `link_text` returns the words of the guidance line before the entries of a
/// `links` rule, which the scaffold writes and `sokf guide` prints. The words
/// are the rule's guidance, then the words that `words` give for its limit,
/// then the limit's `when-at-limit`, with each run of whitespace made one
/// space. Where `words` give no words for the limit, the line states the limit
/// in the syntax of the schema, such as `` `hard: 1`, `when-at-limit`: ``, as
/// the engine does for a limit that its host gives no words for. `link_text`
/// returns `None` where the rule has neither guidance nor a limit.
// sokf-implements: EXT-8_AC-5
pub fn link_text(rule: &LinkRule, words: Option<&Words<'_>>) -> Option<String> {
    let limit = rule.max.iter().flat_map(|l| {
        let lead = words
            .and_then(|w| w.link_lead(l, &rule.rel))
            .filter(|w| !squash(w).is_empty())
            .unwrap_or_else(|| {
                let kind = match l.kind {
                    LimitKind::Soft => "soft",
                    LimitKind::Hard => "hard",
                };
                format!("`{kind}: {}`, `when-at-limit`:", l.count)
            });
        [lead, l.when_at_limit.clone()]
    });
    let words: Vec<String> = rule
        .guidance
        .iter()
        .cloned()
        .chain(limit)
        .map(|t| squash(&t))
        .filter(|t| !t.is_empty())
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
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

    /// `limit_lead` words the limit that `facts` describe from sokf's texts
    /// in the language of the schema, or gives `None`, so that the engine
    /// writes the limit in the syntax of the schema.
    fn limit_lead(&self, facts: &LimitFacts<'_>) -> Option<String> {
        self.words?.limit_lead(facts)
    }

    /// `label_words` gives the words of the label that `facts` describe from
    /// sokf's texts in the language of the schema, or `None`, so that the
    /// engine writes the label's key.
    // sokf-implements: SOKF_SCHEMA_107
    fn label_words(&self, facts: &LabelFacts<'_>) -> Option<String> {
        self.words?.label(facts.key)
    }

    /// `scaffold_label` names `agent` as the label of each unit of a scaffold
    /// that carries one, because an agent writes a new document. Every schema
    /// with labels declares it (`SOKF_SCHEMA_104`).
    // sokf-implements: SOKF_SCHEMA_108
    // sokf-implements: CLI-6_AC-124
    // sokf-implements: CLI-6_AC-125
    fn scaffold_label(&self, _labels: &[String]) -> Option<String> {
        Some("agent".to_string())
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
        // A rule that a hard limit of 0 bans has no edge to write.
        // sokf-implements: EXT-8_AC-1
        // sokf-implements: EXT-8_AC-6
        let mut trailing = Vec::new();
        let edges: Vec<_> = self.schema.written_links(states).collect();
        if !edges.is_empty() {
            trailing.push("links:".to_string());
            for rule in edges {
                for _ in 0..rule.min.max(1) {
                    if let Some(g) = link_text(rule, self.words) {
                        trailing.push(format!("  {}", guidance_comment(&g)));
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
