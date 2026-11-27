//! This module holds the wire types of `sokf guide --json` (CLI-17_AC-17),
//! which `sokf new --json` also gives for the document that it creates.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;

use crate::RepoPath;

// sokf:begin guide-report
/// `GuideReport` is the report of `sokf guide --json`: the guide of each
/// document that an argument names, in the order of the arguments
/// (CLI-17_AC-17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(title = "sokf guide report")]
pub struct GuideReport {
    /// `documents` holds one guide for each argument, in the order of the
    /// arguments.
    pub documents: Vec<DocumentGuide>,
}

/// `DocumentGuide` holds the guidance that a document's schema gives for
/// the parts that the document's lifecycle states admit, in the schema's
/// order (CLI-17_AC-6), with each part's example (CLI-17_AC-38). Each `text`
/// holds the words of the guidance line that the scaffold of the schema
/// writes before the same part at the same states, and each `example` the
/// text that the scaffold writes for the part, with the document's own
/// frontmatter values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DocumentGuide {
    /// `path` is the path of the document.
    pub path: RepoPath,
    /// `id` is the `id` of the document. It is absent when the frontmatter
    /// has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub id: Option<String>,
    /// `type` is the `type` of the document.
    #[serde(rename = "type")]
    pub type_name: String,
    /// `schema` is the path of the schema that governs the document. It is
    /// absent for a schema, which the grammar of the schema language
    /// governs. Each `line` of the guide is a line of this file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "RepoPath")]
    pub schema: Option<RepoPath>,
    /// `states` maps each lifecycle key, in the order of the lifecycle, to
    /// the state at which the guide shows the document: the document's own
    /// state, or the state that `--state` gives.
    #[schemars(with = "BTreeMap<String, String>")]
    pub states: serde_json::Map<String, serde_json::Value>,
    /// `guidance` is the schema's top-level `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `bans` holds the message of each ban of the whole body, in order.
    pub bans: Vec<String>,
    /// `text` holds the words of the guidance line that opens the body: the
    /// schema's guidance, then the message of each ban of the whole body. It
    /// is absent when the schema has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `labels` holds each attribution label that the schema declares, in
    /// the schema's order, once for the whole document (CLI-17_AC-35). It is
    /// empty for a schema without labels.
    pub labels: Vec<LabelGuide>,
    /// `frontmatter` holds each frontmatter key that the schema declares and
    /// the states admit, in the schema's order, then each lifecycle key.
    pub frontmatter: Vec<KeyGuide>,
    /// `links` holds each `links` rule that the states admit, in the schema's
    /// order, except a rule whose hard limit of 0 bans its links.
    pub links: Vec<LinkGuide>,
    /// `preamble` is the part of the body before the first heading, where
    /// the schema declares one that the states admit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "PreambleGuide")]
    pub preamble: Option<PreambleGuide>,
    /// `sections` holds each section that the states admit, in the schema's
    /// order, with each section that declares `parent` after its parent.
    pub sections: Vec<SectionGuide>,
    /// `frontmatter_text` is the YAML of the document's frontmatter, from which
    /// the text output leaves out each example of a key that it holds
    /// already. The JSON report leaves this field out.
    #[serde(skip)]
    #[schemars(skip)]
    pub frontmatter_text: String,
    /// `body_text` is the document's body, from which the text output leaves out
    /// each example of a block that it holds already. The JSON report leaves
    /// this field out.
    #[serde(skip)]
    #[schemars(skip)]
    pub body_text: String,
}

/// `LabelGuide` holds one attribution label of the schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct LabelGuide {
    /// `key` is the label's key, `human` or `agent`.
    pub key: String,
    /// `words` are the words that a document writes between the label's
    /// square brackets: sokf's words for the label in the language of the
    /// schema, or else the key.
    pub words: String,
    /// `meaning` is sokf's sentence that says what the label means, in the
    /// language of the schema. It is absent where sokf's texts hold none in
    /// that language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub meaning: Option<String>,
}

/// `KeyGuide` holds the guidance of one frontmatter key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct KeyGuide {
    /// `key` is the key.
    pub key: String,
    /// `lifecycle` is true when the key is a lifecycle key rather than a
    /// declared one.
    pub lifecycle: bool,
    /// `optional` is true when the key can be absent.
    pub optional: bool,
    /// `guidance` is the key's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `limit` is the limit on the elements of the key's list. It is absent
    /// when the key has no `max`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GuideLimit")]
    pub limit: Option<GuideLimit>,
    /// `text` holds the words of the guidance line before the key: sokf's own
    /// text for the key, which is the rule for writing a `description`, then
    /// the key's guidance, then the words that state its limit and the limit's
    /// `when-at-limit`. It is absent when the key has none of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `example` is the line that the scaffold of the schema writes for the
    /// key, with the document's own values, such as `priority: medium`. It is
    /// absent where the scaffold writes none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub example: Option<String>,
    /// `line` is the line of the key's rule in the schema.
    pub line: usize,
}

/// `LinkGuide` holds the guidance of one `links` rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct LinkGuide {
    /// `rel` is the relationship of the links that the rule governs.
    pub rel: String,
    /// `optional` is true when the rule's `min` is 0.
    pub optional: bool,
    /// `limit` is the limit on the links of the rule. It is absent when the
    /// rule has no `max`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GuideLimit")]
    pub limit: Option<GuideLimit>,
    /// `text` holds the words of the guidance line before the link: the
    /// rule's guidance, then the words that state its limit and the limit's
    /// `when-at-limit`. It is absent when the rule has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `example` is the `links` key with one entry of the rule, which names
    /// its target by the placeholder `target-id`.
    pub example: String,
}

/// `PreambleGuide` holds the guidance of the preamble.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct PreambleGuide {
    /// `guidance` is the preamble's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `text` holds the words of the guidance line before the preamble.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `example` is the text that the scaffold of the schema writes for the
    /// preamble. It is absent where the scaffold writes none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub example: Option<String>,
    /// `blocks` holds each block that the states admit, in order.
    pub blocks: Vec<BlockGuide>,
    /// `line` is the line of the preamble in the schema.
    pub line: usize,
}

/// `SectionGuide` holds the guidance of one section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SectionGuide {
    /// `heading` is the heading as a finding of `validate` names it: its
    /// text, its template or pattern, or `<key>` for a heading that takes
    /// the value of the frontmatter key `key`.
    pub heading: String,
    /// `level` is the level of the heading.
    pub level: u8,
    /// `rule_id` is the section's `rule-id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `parent` is the `rule-id` of the section that this one lies inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub parent: Option<String>,
    /// `optional` is true when the section can be absent at the states.
    pub optional: bool,
    /// `repeat` is true when the heading can occur more than once.
    pub repeat: bool,
    /// `held` is true when tramlines-md's match of a heading to its section
    /// gives the section a heading at its level whose reading, as
    /// tramlines-md normalises it, has the section's heading shape, or one
    /// of its `renamed-from` texts (CLI-17_AC-15).
    pub held: bool,
    /// `guidance` is the section's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `text` holds the words of the guidance line before the heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `example` is the text that the scaffold of the schema writes for the
    /// section: each occurrence, from its heading, with the sections inside
    /// it and the section's `examples`. It is absent where the scaffold
    /// writes none. A section inside a repeating one gives the text of its
    /// first occurrence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub example: Option<String>,
    /// `blocks` holds each block of the section's body that the states
    /// admit, in order.
    pub blocks: Vec<BlockGuide>,
    /// `line` is the line of the section's rule in the schema.
    pub line: usize,
}

/// `BlockGuide` holds the guidance of one block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct BlockGuide {
    /// `kind` is the kind of the block, as the grammar names it, such as
    /// `prose` or `bullet-list`.
    pub kind: String,
    /// `rule_id` is the block's `rule-id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `pos` is the block's place in the list that declares it, from 1.
    pub pos: usize,
    /// `optional` is true when the block can be absent at the states.
    pub optional: bool,
    /// `guidance` is the block's own `guidance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub guidance: Option<String>,
    /// `limits` holds the block's own limit, then the limit of each of its
    /// unit rules and line rules, in the schema's order.
    pub limits: Vec<RuleLimit>,
    /// `labels` holds the block's own `label`, then the `label` of each of its
    /// unit rules that declares one, in the schema's order (CLI-17_AC-37).
    pub labels: Vec<RuleLabel>,
    /// `bans` holds the message of each ban of the block's unit rules, of
    /// the formats that they bind, and of their line rules, each once, in
    /// order.
    pub bans: Vec<String>,
    /// `text` holds the words of the guidance line before the block: its
    /// guidance, then the words that state each limit and the limit's
    /// `when-at-limit`, then the message of each ban. It is absent when the
    /// block has none of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    /// `example` is the text that the scaffold of the schema writes for the
    /// block, without the indent of an item that holds it. It is absent where
    /// the scaffold writes none. A block that the scaffold writes more than
    /// once gives its first occurrence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub example: Option<String>,
    /// `blocks` holds the blocks of a group, and the blocks inside the items
    /// of a list, that the states admit, in order.
    pub blocks: Vec<BlockGuide>,
    /// `line` is the line of the block's rule in the schema.
    pub line: usize,
}

/// `RuleLimit` is one limit of a block: its own, or one of a unit rule's or
/// a line rule's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RuleLimit {
    /// `rule` is the place of the unit rule or the line rule whose limit this
    /// is, among the unit rules of the schema, counted from 0. It is absent for
    /// the block's own limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub rule: Option<usize>,
    /// `format` is the name of the format that the unit rule or the line
    /// rule binds. It is absent for the block's own limit and for a rule
    /// that binds no format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub format: Option<String>,
    /// `rule_id` is the `rule-id` of the unit rule or the line rule. It is
    /// absent for the block's own limit and for a rule without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `counts` is what the unit rule or the line rule counts: the units of
    /// the block, or the content lines of a code block or an HTML block. It
    /// is absent for the block's own limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GuideCounts")]
    pub counts: Option<GuideCounts>,
    /// `line` is the line of the rule in the schema.
    pub line: usize,
    /// The limit.
    #[serde(flatten)]
    pub limit: GuideLimit,
}

/// `RuleLabel` says whether the units of a block, or the units of one of its
/// unit rules, carry a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RuleLabel {
    /// `rule` is the place of the unit rule whose `label` this is, among the
    /// unit rules of the schema, counted from 0. It is absent for the block's
    /// own `label`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub rule: Option<usize>,
    /// `rule_id` is the `rule-id` of the unit rule. It is absent for the
    /// block's own `label` and for a rule without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rule_id: Option<String>,
    /// `line` is the line of the rule in the schema.
    pub line: usize,
    /// `label` is `required` where each unit carries a label, and `optional`
    /// where a unit carries one if its writer gives one.
    pub label: GuideLabel,
}

/// `GuideLabel` says whether a unit carries a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuideLabel {
    /// Every unit carries a label.
    Required,
    /// A unit carries a label where its writer gives one.
    Optional,
}

/// `GuideLimit` is a limit on a part: its kind, the number of units, lines, or
/// elements that it states, and what the writer does at the limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct GuideLimit {
    /// `kind` is `soft` for the usual length, which nothing checks, or
    /// `hard` for the most that a part can hold.
    pub kind: GuideLimitKind,
    /// `count` is the number that the limit states.
    pub count: usize,
    /// `when_at_limit` is the schema's text that says what the writer does at
    /// the limit.
    pub when_at_limit: String,
}

/// `GuideCounts` is what the limit of a unit rule or a line rule counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuideCounts {
    /// The units of the block: its paragraphs, items, rows, or repetitions.
    Units,
    /// The content lines of a code block or an HTML block.
    Lines,
}

/// `GuideLimitKind` is the kind of a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuideLimitKind {
    /// The usual length, which nothing checks.
    Soft,
    /// The most that a part can hold.
    Hard,
}
// sokf:end guide-report
