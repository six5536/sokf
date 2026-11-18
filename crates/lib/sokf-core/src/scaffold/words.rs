//! This module holds sokf's own words in a scaffold, in the language of the
//! schema (`SOKF_SCHEMA_101`): the words that state each limit before its
//! `when-at-limit`, the rule for writing a description, which `sokf new`
//! writes before the `description` key and `sokf guide` prints, and the words
//! and the meaning of each attribution label.

use tramlines_md::{
    check::{Counts, LimitFacts, LimitSubject, RuleName},
    markdown::BlockKind,
};

use super::front::guidance_comment;
use crate::{
    Model,
    config::{Texts, fill_template, placeholders},
    grammar::{Compiled, Limit, LimitKind, SokfSchema, squash},
    model::Manifest,
};

/// `Words` gives sokf's texts in one language: the language of the schema
/// of a scaffold or a guide.
#[derive(Debug, Clone)]
pub struct Words<'a> {
    texts: &'a Texts,
    language: String,
}

impl<'a> Words<'a> {
    /// `new` returns the words of `texts` in `language`.
    pub fn new(texts: &'a Texts, language: impl Into<String>) -> Self {
        Words {
            texts,
            language: language.into(),
        }
    }

    /// `description` returns the rule for writing a description, with each run
    /// of whitespace made one space, or `None` where the language has none.
    pub fn description(&self) -> Option<String> {
        let rule = squash(self.texts.words(&self.language, "description")?);
        (!rule.is_empty()).then_some(rule)
    }

    /// `label` returns the words of the label `key` in the language, or
    /// `None` where the language has none, and the engine then takes the key
    /// as the label's words (`SOKF_SCHEMA_106`).
    // sokf-implements: SOKF_SCHEMA_105
    // sokf-implements: SOKF_SCHEMA_106
    pub fn label(&self, key: &str) -> Option<String> {
        self.texts.label(&self.language, key)
    }

    /// `attribution` returns the meaning of the label `key`, with each run of
    /// whitespace made one space, or `None` where the language has none.
    pub fn attribution(&self, key: &str) -> Option<String> {
        self.texts.attribution(&self.language, key)
    }

    /// `limit_lead` returns the words before the `when-at-limit` of the limit
    /// that `facts` describe: the limit's template, or `limit.ban` for a hard
    /// limit of 0, filled with the limit's count and unit. It returns `None`
    /// where the language lacks the template, the word for the unit, or the
    /// `rule-unit` that the template needs, and the engine then writes the
    /// limit in the syntax of the schema (CLI-6_AC-122).
    // sokf-implements: CLI-6_AC-120
    // sokf-implements: CLI-6_AC-121
    // sokf-implements: CLI-6_AC-122
    // sokf-implements: SOKF_SCHEMA_102
    pub fn limit_lead(&self, facts: &LimitFacts<'_>) -> Option<String> {
        let name = match facts.subject {
            LimitSubject::Rule {
                name: Some(RuleName::Format(n) | RuleName::RuleId(n)),
                ..
            } => Some(n),
            _ => None,
        };
        self.lead(facts.kind, facts.count, unit_of(&facts.subject), name)
    }

    /// `link_lead` returns the words before the `when-at-limit` of `limit`,
    /// the limit of the `links` rule of `rel`, as `limit_lead` does for a
    /// limit of the engine's: the unit is a link, with the `rel` as the
    /// rule's name, such as "At most 1 `part-of` link. Never leave out or
    /// merge content to stay within the count. When at the limit:".
    // sokf-implements: EXT-8_AC-5
    pub fn link_lead(&self, limit: &Limit, rel: &str) -> Option<String> {
        self.lead(limit.kind, limit.count, "link", Some(rel))
    }

    /// `lead` fills the template of a limit of `kind` and `count`, which
    /// counts `unit`, with the name `name` of its rule where it has one.
    fn lead(
        &self,
        kind: LimitKind,
        count: usize,
        unit: &str,
        name: Option<&str>,
    ) -> Option<String> {
        let lang = self.language.as_str();
        let template = match (kind, count) {
            (LimitKind::Hard, 0) => "limit.ban",
            (LimitKind::Hard, _) => "limit.hard",
            (LimitKind::Soft, _) => "limit.soft",
        };
        let template = self.texts.words(lang, template)?;
        let shown = count.to_string();
        // A template without `{unit}` needs no word for the unit.
        if !placeholders(template).contains(&"unit") {
            return Some(fill_template(template, &[("count", &shown)]));
        }
        // The word for the unit in the form for the count: a ban, of 0,
        // takes the form `other`. `None` where the language lacks it.
        let noun = || {
            let (one, other) = self.texts.forms(lang, unit)?;
            Some(if count == 1 { one } else { other })
        };
        let unit = match name {
            Some(name) => {
                let rule_unit = self.texts.words(lang, "rule-unit")?;
                // A `rule-unit` without `{unit}` needs no word for the unit.
                let noun = if placeholders(rule_unit).contains(&"unit") {
                    noun()?
                } else {
                    ""
                };
                fill_template(rule_unit, &[("name", name), ("unit", noun)])
            }
            None => noun()?.to_string(),
        };
        Some(fill_template(
            template,
            &[("count", &shown), ("unit", &unit)],
        ))
    }
}

/// `unit_of` returns the unit that a limit on `subject` counts, as the
/// engine counts it: the unit of the block for a block's own limit and for
/// a rule of its units, such as a fence of a code block, a line for a line
/// rule, and a value for a frontmatter key.
fn unit_of(subject: &LimitSubject<'_>) -> &'static str {
    let block = match subject {
        LimitSubject::Key(_) => return "value",
        LimitSubject::Rule {
            counts: Counts::Lines,
            ..
        } => return "line",
        LimitSubject::Rule { block, .. } | LimitSubject::Block(block) => *block,
    };
    match block {
        BlockKind::Prose | BlockKind::Blockquote => "paragraph",
        BlockKind::BulletList | BlockKind::NumberedList => "item",
        BlockKind::Table => "row",
        // A code block's own limit counts its fences, and a line rule its
        // lines.
        BlockKind::Code => "fence",
        BlockKind::Image => "image",
        BlockKind::References => "definition",
        BlockKind::Group => "repetition",
        _ => "unit",
    }
}

/// `language` returns the language of `schema`, the schema of `type_name`
/// in the store `knowledge`: its `language`, else the `primary-language`
/// of the store that holds the schema, else `en`. The schema of a schema
/// document is the grammar's, so the language comes from `fallback`, the
/// store of the document.
// sokf-implements: SOKF_SCHEMA_101
pub fn language(
    model: &Model,
    compiled: &Compiled,
    schema: &SokfSchema,
    type_name: &str,
    knowledge: Option<usize>,
    fallback: Option<usize>,
) -> String {
    let home = knowledge
        .filter(|_| type_name != "Schema")
        .and_then(|k| compiled.for_type(model, k, type_name))
        .map(|(r, _)| r.knowledge)
        .or(fallback);
    let manifest = home.and_then(|k| model.knowledges[k].manifest.as_ref());
    language_of(schema, manifest)
}

/// `language_of` returns the language of `schema`: its `language`, else
/// the `primary-language` of `manifest`, the manifest of the store that
/// holds the schema, else `en`.
fn language_of(schema: &SokfSchema, manifest: Option<&Manifest>) -> String {
    let given = |l: &Option<String>| l.clone().filter(|l| !l.is_empty());
    given(&schema.core.core.language)
        .or_else(|| manifest.and_then(|m| given(&m.primary_language)))
        .unwrap_or_else(|| "en".to_string())
}

/// `with_description` returns `document`, a scaffold, with `rule`, the rule
/// for writing a description, at the start of the guidance line before its
/// `description` key, or on a guidance line of its own where the key has
/// none. A scaffold without the key comes back as it was.
// sokf-implements: CLI-6_AC-118
// sokf-implements: CLI-6_AC-119
pub fn with_description(document: &str, rule: &str) -> String {
    let lines: Vec<&str> = document.split_inclusive('\n').collect();
    // The frontmatter runs from the first line, `---`, to the next `---`.
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return document.to_string();
    }
    let Some(end) = lines.iter().skip(1).position(|l| l.trim_end() == "---") else {
        return document.to_string();
    };
    let Some(at) = lines[1..=end]
        .iter()
        .position(|l| l.starts_with("description:"))
        .map(|i| i + 1)
    else {
        return document.to_string();
    };
    // The engine writes the schema's guidance for the key on the line
    // before it.
    let own = guidance_comment("");
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
    match out[at - 1].strip_prefix(&own) {
        Some(rest) => out[at - 1] = format!("{} {rest}", guidance_comment(rule)),
        None => out.insert(at, format!("{}\n", guidance_comment(rule))),
    }
    out.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(kind: LimitKind, count: usize, subject: LimitSubject<'_>) -> LimitFacts<'_> {
        LimitFacts::new(kind, count, subject, None)
    }

    // sokf-tests: CLI-6_AC-120
    // sokf-tests: CLI-6_AC-121
    #[test]
    fn a_limit_is_worded_with_its_count_and_its_unit() {
        let texts = Texts::english();
        let words = Words::new(&texts, "en");
        let lead = |f: LimitFacts<'_>| words.limit_lead(&f);
        assert_eq!(
            lead(facts(
                LimitKind::Hard,
                9,
                LimitSubject::Block(BlockKind::BulletList)
            ))
            .as_deref(),
            Some(
                "At most 9 items. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
        assert_eq!(
            lead(facts(
                LimitKind::Soft,
                1,
                LimitSubject::Block(BlockKind::Prose)
            ))
            .as_deref(),
            Some(
                "Usually up to 1 paragraph. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
        let rule = |counts, name| LimitSubject::Rule {
            block: BlockKind::Code,
            counts,
            name,
        };
        assert_eq!(
            lead(facts(
                LimitKind::Hard,
                9,
                rule(Counts::Lines, Some(RuleName::Format("node")))
            ))
            .as_deref(),
            Some(
                "At most 9 `node` lines. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
        assert_eq!(
            lead(facts(
                LimitKind::Hard,
                0,
                rule(Counts::Lines, Some(RuleName::RuleId("todo")))
            ))
            .as_deref(),
            Some("No `todo` lines. Instead:")
        );
        let list = LimitSubject::Rule {
            block: BlockKind::NumberedList,
            counts: Counts::Units,
            name: None,
        };
        assert_eq!(
            lead(facts(LimitKind::Hard, 1, list)).as_deref(),
            Some(
                "At most 1 item. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
        assert_eq!(
            lead(facts(LimitKind::Soft, 3, LimitSubject::Key("tags"))).as_deref(),
            Some(
                "Usually up to 3 values. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
        assert_eq!(
            lead(facts(
                LimitKind::Hard,
                1,
                LimitSubject::Block(BlockKind::Code)
            ))
            .as_deref(),
            Some(
                "At most 1 fence. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
        assert_eq!(
            lead(facts(
                LimitKind::Hard,
                2,
                LimitSubject::Block(BlockKind::Group)
            ))
            .as_deref(),
            Some(
                "At most 2 repetitions. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
    }

    // Each unit that a limit can count is one that a file of texts can word.
    #[test]
    fn each_unit_is_a_unit_of_the_texts() {
        let kinds = [
            BlockKind::Prose,
            BlockKind::BulletList,
            BlockKind::NumberedList,
            BlockKind::Table,
            BlockKind::Code,
            BlockKind::Blockquote,
            BlockKind::Image,
            BlockKind::Region,
            BlockKind::Html,
            BlockKind::ThematicBreak,
            BlockKind::References,
            BlockKind::Group,
            BlockKind::Heading,
        ];
        let mut subjects = vec![LimitSubject::Key("tags")];
        for block in kinds {
            subjects.push(LimitSubject::Block(block));
            for counts in [Counts::Units, Counts::Lines] {
                subjects.push(LimitSubject::Rule {
                    block,
                    counts,
                    name: None,
                });
            }
        }
        for subject in subjects {
            assert!(
                crate::config::UNITS.contains(&unit_of(&subject)),
                "{subject:?}"
            );
        }
    }

    // sokf-tests: CLI-6_AC-122
    // sokf-tests: SOKF_SCHEMA_102
    #[test]
    fn a_language_without_the_words_gives_none() {
        let texts = Texts::english();
        let f = facts(LimitKind::Hard, 3, LimitSubject::Block(BlockKind::Table));
        assert_eq!(Words::new(&texts, "de").limit_lead(&f), None);
        assert_eq!(Words::new(&texts, "de").description(), None);
        assert_eq!(
            Words::new(&texts, "en-GB").limit_lead(&f).as_deref(),
            Some(
                "At most 3 rows. Never leave out or merge content to stay within the count. When at the limit:"
            )
        );
    }

    // A template or a `rule-unit` without `{unit}` needs no unit of the
    // language, so a ban can be worded without one.
    // sokf-tests: CLI-6_AC-120
    // sokf-tests: CLI-6_AC-121
    #[test]
    fn a_template_without_its_unit_needs_no_forms() {
        let texts = Texts::default().with(
            "de: { limit: { ban: 'Keine. Stattdessen:', hard: 'Höchstens {count}.', \
             soft: 'Meist {count} {unit}.' }, rule-unit: '`{name}`' }\n",
        );
        let words = Words::new(&texts, "de");
        let table = LimitSubject::Block(BlockKind::Table);
        let lead = |kind, count, subject| words.limit_lead(&facts(kind, count, subject));
        assert_eq!(
            lead(LimitKind::Hard, 0, table).as_deref(),
            Some("Keine. Stattdessen:")
        );
        assert_eq!(
            lead(LimitKind::Hard, 4, table).as_deref(),
            Some("Höchstens 4.")
        );
        let tasks = LimitSubject::Rule {
            block: BlockKind::BulletList,
            counts: Counts::Units,
            name: Some(RuleName::RuleId("tasks")),
        };
        assert_eq!(
            lead(LimitKind::Soft, 9, tasks).as_deref(),
            Some("Meist 9 `tasks`.")
        );
        // A template with `{unit}` still needs the unit's forms.
        assert_eq!(lead(LimitKind::Soft, 9, table), None);
        let part_of = Limit {
            kind: LimitKind::Hard,
            count: 0,
            when_at_limit: String::new(),
        };
        assert_eq!(
            words.link_lead(&part_of, "part-of").as_deref(),
            Some("Keine. Stattdessen:")
        );
    }

    // sokf-tests: CLI-6_AC-118
    // sokf-tests: CLI-6_AC-119
    #[test]
    fn the_rule_for_a_description_opens_the_line_before_the_key() {
        let own =
            "---\ntype: Note\n# sokf:guidance The note says why.\ndescription: Why.\n---\n\nBody\n";
        assert_eq!(
            with_description(own, "The rule."),
            "---\ntype: Note\n# sokf:guidance The rule. The note says why.\ndescription: Why.\n---\n\nBody\n"
        );
        let bare = "---\ntype: Note\ndescription: Why.\n---\n";
        assert_eq!(
            with_description(bare, "The rule."),
            "---\ntype: Note\n# sokf:guidance The rule.\ndescription: Why.\n---\n"
        );
        // Without the key, and in the body, nothing changes.
        for text in [
            "---\ntype: Note\n---\n\ndescription: x\n",
            "# Title\n",
            "---\ntype: Note\n",
        ] {
            assert_eq!(with_description(text, "The rule."), text);
        }
    }
}
