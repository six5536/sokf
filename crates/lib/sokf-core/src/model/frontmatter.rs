//! This module parses a document's frontmatter into `CoreFields`, as the
//! format specification defines them. The module parses each key on its
//! own: it reports a finding for a bad key and goes on with the rest of the
//! document.

use std::{collections::HashMap, sync::OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{Finding, RepoPath, Spanned, Yaml, YamlError, markdown::is_url};

/// The `status` values (SOKF_FORMAT_28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// `draft`.
    Draft,
    /// `stable`, the default.
    Stable,
    /// `deprecated`.
    Deprecated,
}

/// `resource` (SOKF_FORMAT_29) is what a document describes: a `/` path or a URL,
/// or a list of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    /// `One` is a resource written as one string, the form that a sidecar's
    /// `resource` takes (SOKF_FORMAT_31).
    One(String),
    /// `List` is a resource written as a list.
    List(Vec<String>),
}

impl Resource {
    /// `entries` returns every entry, in the order written.
    pub fn entries(&self) -> &[String] {
        match self {
            Resource::One(r) => std::slice::from_ref(r),
            Resource::List(rs) => rs,
        }
    }

    /// `one` returns the string when the resource is written as one string,
    /// and `None` when it is written as a list.
    pub fn one(&self) -> Option<&str> {
        match self {
            Resource::One(r) => Some(r),
            Resource::List(_) => None,
        }
    }
}

/// One `sources` entry (SOKF_FORMAT_37, SOKF_FORMAT_38).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    /// `resource`: a path relative to the repository root, or a URL.
    pub resource: String,
    /// `id`: the footnote key.
    pub id: Option<String>,
    /// `title`.
    pub title: Option<String>,
    /// The entry's line.
    pub line: usize,
}

/// One `links` entry (SOKF_FORMAT_50, SOKF_FORMAT_51).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkEntry {
    /// `rel`.
    pub rel: String,
    /// `to`: an id or a `/` path.
    pub to: String,
    /// `note`.
    pub note: Option<String>,
    /// The entry's line.
    pub line: usize,
}

/// One `verified` entry (SOKF_FORMAT_43, SOKF_FORMAT_44).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedEntry {
    /// `by`: a `human:` or `process:` actor.
    pub by: String,
    /// `at`: an ISO 8601 datetime.
    pub at: String,
    /// The entry's line.
    pub line: usize,
}

/// The core frontmatter fields of the format specification, each optional, plus
/// the line of each key and the findings that the parser records as it goes on
/// past each bad key.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CoreFields {
    /// `type`.
    pub type_: Option<String>,
    /// `id`.
    pub id: Option<String>,
    /// `title`.
    pub title: Option<String>,
    /// `description`.
    pub description: Option<String>,
    /// `tags`.
    pub tags: Option<Vec<String>>,
    /// `resource`.
    pub resource: Option<Resource>,
    /// `status`.
    pub status: Option<Status>,
    /// `sources`: the entries that parsed as valid maps. The parser drops any
    /// other entry.
    pub sources: Vec<SourceEntry>,
    /// `links`: the entries that parsed as valid maps. The parser drops any
    /// other entry.
    pub links: Vec<LinkEntry>,
    /// `verified`.
    pub verified: Vec<VerifiedEntry>,
    /// Keys that are not core, as written.
    pub extra: Vec<(String, Spanned)>,
    /// The line of every present key.
    pub lines: HashMap<String, usize>,
    /// The findings about the shape of the fields.
    pub problems: Vec<Finding>,
    /// Whether the frontmatter is present and a map.
    pub present: bool,
}

/// The core keys of the format specification.
pub const CORE_KEYS: &[&str] = &[
    "type",
    "id",
    "title",
    "description",
    "tags",
    "resource",
    "status",
    "sources",
    "links",
    "verified",
];

/// The stamped keys of documents (SOKF_FORMAT_24): keys that sokf writes
/// itself.
pub const STAMPED_DOCUMENT_KEYS: &[&str] = &["generated"];

impl CoreFields {
    /// `extra` returns the value of a non-core key.
    pub fn extra(&self, key: &str) -> Option<&Spanned> {
        self.extra.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// `line_of` returns the line of `key`, or `None` when the key is absent.
    pub fn line_of(&self, key: &str) -> Option<usize> {
        self.lines.get(key).copied()
    }

    /// `fail` records an error about the frontmatter of the document at
    /// `path`.
    fn fail(
        &mut self,
        path: &RepoPath,
        line: Option<usize>,
        criterion: &'static str,
        msg: impl Into<String>,
    ) {
        self.problems
            .push(Finding::error(path.clone(), line, criterion, msg));
    }
}

/// `iso_datetime` returns the pattern of an ISO 8601 date or datetime,
/// compiled once.
fn iso_datetime() -> &'static Regex {
    static ISO: OnceLock<Regex> = OnceLock::new();
    ISO.get_or_init(|| {
        Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?)?$")
            .expect("literal")
    })
}

/// `core_fields` parses the frontmatter into core fields. `yaml` is the parsed
/// frontmatter, the parse error, or `None` when the document has no frontmatter.
// sokf-implements: SOKF_FORMAT_17
// sokf-implements: SOKF_FORMAT_19
// sokf-implements: SOKF_FORMAT_26
// sokf-implements: SOKF_FORMAT_27
// sokf-implements: SOKF_FORMAT_28
// sokf-implements: SOKF_FORMAT_29
// sokf-implements: SOKF_FORMAT_32
// sokf-implements: SOKF_FORMAT_37
// sokf-implements: SOKF_FORMAT_38
// sokf-implements: SOKF_FORMAT_42
// sokf-implements: SOKF_FORMAT_43
// sokf-implements: SOKF_FORMAT_44
// sokf-implements: SOKF_FORMAT_50
// sokf-implements: SOKF_FORMAT_51
pub fn core_fields(yaml: Option<Result<&Spanned, &YamlError>>, path: &RepoPath) -> CoreFields {
    let mut core = CoreFields::default();
    let node = match yaml {
        None => {
            core.fail(
                path,
                None,
                "SOKF_FORMAT_17",
                "the document has no frontmatter",
            );
            return core;
        }
        Some(Err(e)) => {
            core.fail(
                path,
                Some(e.line()),
                "SOKF_FORMAT_18",
                format!("frontmatter: {e}"),
            );
            return core;
        }
        Some(Ok(node)) => node,
    };
    let Some(entries) = node.as_map() else {
        core.fail(
            path,
            Some(node.line),
            "SOKF_FORMAT_18",
            format!("the frontmatter must be a map, not a {}", node.type_name()),
        );
        return core;
    };
    core.present = true;
    for (key, value) in entries {
        core.lines.insert(key.value.clone(), key.line);
        let line = Some(value.line);
        match key.value.as_str() {
            "type" => match value.as_str() {
                Some(s) if !s.trim().is_empty() => core.type_ = Some(s.to_string()),
                Some(_) => core.fail(path, line, "SOKF_FORMAT_19", "`type` is empty"),
                None => core.fail(path, line, "SOKF_FORMAT_19", "`type` must be a string"),
            },
            "id" => {
                // A scalar that isn't a string still gives the document its
                // id, so that the other checks can name the document.
                if let Some(s) = value.scalar_text() {
                    core.id = Some(s);
                }
                if value.as_str().is_none() {
                    core.fail(path, line, "SOKF_FORMAT_32", "`id` must be a string");
                }
            }
            "title" => match value.as_str() {
                Some(s) => core.title = Some(s.to_string()),
                None => core.fail(path, line, "SOKF_FORMAT_26", "`title` must be a string"),
            },
            "description" => match value.as_str() {
                Some(s) => core.description = Some(s.to_string()),
                None => core.fail(
                    path,
                    line,
                    "SOKF_FORMAT_26",
                    "`description` must be a string",
                ),
            },
            "tags" => match value.as_seq().map(|items| {
                items
                    .iter()
                    .map(|i| i.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()
            }) {
                Some(Some(tags)) => core.tags = Some(tags),
                _ => core.fail(
                    path,
                    line,
                    "SOKF_FORMAT_27",
                    "`tags` must be a list of strings",
                ),
            },
            "resource" => match resource(value) {
                Some(r) => core.resource = Some(r),
                None => core.fail(
                    path,
                    line,
                    "SOKF_FORMAT_29",
                    "`resource` must be a `/` path or a URL, or a list of them",
                ),
            },
            "status" => match value.as_str() {
                Some("draft") => core.status = Some(Status::Draft),
                Some("stable") => core.status = Some(Status::Stable),
                Some("deprecated") => core.status = Some(Status::Deprecated),
                _ => core.fail(
                    path,
                    line,
                    "SOKF_FORMAT_28",
                    "`status` must be `draft`, `stable`, or `deprecated`",
                ),
            },
            "sources" => sources(&mut core, value, path),
            "links" => links(&mut core, value, path),
            "verified" => verified(&mut core, value, path),
            _ => core.extra.push((key.value.clone(), value.clone())),
        }
    }
    core
}

/// `resource` reads `value` as a `resource`, and returns `None` when it isn't
/// one. A `resource` is a string, or a non-empty list of strings, and each
/// string is a `/` path or a URL.
fn resource(value: &Spanned) -> Option<Resource> {
    let valid = |s: &str| RepoPath::parse(s).is_some() || is_url(s);
    if let Some(s) = value.as_str() {
        return valid(s).then(|| Resource::One(s.to_string()));
    }
    let entries = value
        .as_seq()?
        .iter()
        .map(|i| i.as_str().filter(|s| valid(s)).map(str::to_string))
        .collect::<Option<Vec<_>>>()?;
    (!entries.is_empty()).then_some(Resource::List(entries))
}

/// `EntryRules` says what a list of entry maps, `sources` or `links`,
/// allows, and which criterion each finding about it cites.
struct EntryRules {
    /// The frontmatter key, such as `sources`.
    field: &'static str,
    /// The keys that an entry can have.
    allowed: &'static [&'static str],
    /// The allowed keys, as a finding names them.
    allowed_text: &'static str,
    /// The criterion of a value that isn't a list of maps.
    shape: &'static str,
    /// The criterion of a key that an entry can't have.
    unknown: &'static str,
    /// The criterion of a value that isn't a string, by its key.
    not_string: fn(&str) -> &'static str,
}

/// `each_entry` checks that `value` is a list of maps that have only the
/// keys that `rules` allows, each with a string value. It calls `add` for
/// each map, in order, with whether the map passed. It reports each
/// problem as it finds it, so the findings of one entry come before those
/// of the next.
fn each_entry(
    core: &mut CoreFields,
    value: &Spanned,
    path: &RepoPath,
    rules: &EntryRules,
    mut add: impl FnMut(&mut CoreFields, &Spanned, bool),
) {
    let field = rules.field;
    let Some(items) = value.as_seq() else {
        core.fail(
            path,
            Some(value.line),
            rules.shape,
            format!("`{field}` must be a list of maps"),
        );
        return;
    };
    for item in items {
        let Some(entries) = item.as_map() else {
            core.fail(
                path,
                Some(item.line),
                rules.shape,
                format!("each `{field}` entry must be a map"),
            );
            continue;
        };
        let mut ok = true;
        for (k, v) in entries {
            if !rules.allowed.contains(&k.value.as_str()) {
                core.fail(
                    path,
                    Some(k.line),
                    rules.unknown,
                    format!(
                        "a `{field}` entry has `{}`. Only {} are allowed",
                        k.value, rules.allowed_text
                    ),
                );
                ok = false;
            } else if v.as_str().is_none() {
                core.fail(
                    path,
                    Some(v.line),
                    (rules.not_string)(&k.value),
                    format!("`{field}[].{}` must be a string", k.value),
                );
                ok = false;
            }
        }
        add(core, item, ok);
    }
}

/// `text_of` returns the string value of `key` in an entry map.
fn text_of(item: &Spanned, key: &str) -> Option<String> {
    item.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn sources(core: &mut CoreFields, value: &Spanned, path: &RepoPath) {
    let rules = EntryRules {
        field: "sources",
        allowed: &["resource", "id", "title"],
        allowed_text: "`resource`, `id`, and `title`",
        shape: "SOKF_FORMAT_38",
        unknown: "SOKF_FORMAT_38",
        not_string: |k| {
            if k == "resource" {
                "SOKF_FORMAT_37"
            } else {
                "SOKF_FORMAT_38"
            }
        },
    };
    each_entry(core, value, path, &rules, |core, item, ok| {
        let Some(resource) = text_of(item, "resource") else {
            core.fail(
                path,
                Some(item.line),
                "SOKF_FORMAT_37",
                "a `sources` entry has no `resource`",
            );
            return;
        };
        if ok {
            core.sources.push(SourceEntry {
                resource,
                id: text_of(item, "id"),
                title: text_of(item, "title"),
                line: item.line,
            });
        }
    });
}

fn links(core: &mut CoreFields, value: &Spanned, path: &RepoPath) {
    let rules = EntryRules {
        field: "links",
        allowed: &["rel", "to", "note"],
        allowed_text: "`rel`, `to`, and `note`",
        shape: "SOKF_FORMAT_50",
        unknown: "SOKF_FORMAT_51",
        not_string: |k| {
            if k == "note" {
                "SOKF_FORMAT_51"
            } else {
                "SOKF_FORMAT_50"
            }
        },
    };
    each_entry(core, value, path, &rules, |core, item, ok| {
        let (Some(rel), Some(to)) = (text_of(item, "rel"), text_of(item, "to")) else {
            core.fail(
                path,
                Some(item.line),
                "SOKF_FORMAT_50",
                "a `links` entry needs `rel` and `to`",
            );
            return;
        };
        if ok {
            core.links.push(LinkEntry {
                rel,
                to,
                note: text_of(item, "note"),
                line: item.line,
            });
        }
    });
}

fn verified(core: &mut CoreFields, value: &Spanned, path: &RepoPath) {
    let items: Vec<&Spanned> = match &value.value {
        Yaml::Seq(items) => items.iter().collect(),
        Yaml::Map(_) => vec![value],
        _ => {
            core.fail(
                path,
                Some(value.line),
                "SOKF_FORMAT_42",
                "`verified` must be a list of `{ by, at }` maps",
            );
            return;
        }
    };
    for item in items {
        let line = Some(item.line);
        let by = item.get("by").and_then(|b| b.as_str());
        let at = item.get("at").and_then(|a| a.scalar_text());
        let (Some(by), Some(at)) = (by, at) else {
            let key = if by.is_none() {
                "SOKF_FORMAT_43"
            } else {
                "SOKF_FORMAT_44"
            };
            core.fail(path, line, key, "a `verified` entry needs `by` and `at`");
            continue;
        };
        if !(by.starts_with("human:") || by.starts_with("process:")) {
            core.fail(
                path,
                line,
                "SOKF_FORMAT_43",
                "`verified[].by` must be a `human:` or `process:` actor",
            );
            continue;
        }
        if !iso_datetime().is_match(&at) {
            core.fail(
                path,
                line,
                "SOKF_FORMAT_44",
                "`verified[].at` must be an ISO 8601 datetime",
            );
            continue;
        }
        core.verified.push(VerifiedEntry {
            by: by.to_string(),
            at,
            line: item.line,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml;

    fn fields(text: &str) -> CoreFields {
        let parsed = yaml::parse(text, 2);
        core_fields(Some(parsed.as_ref()), &RepoPath::parse("/k/a.md").unwrap())
    }

    #[test]
    fn parses_every_core_field() {
        let c = fields(
            "type: Subsystem\nid: subsystem-logging\ntitle: Logging\ndescription: d\nresource: /crates/x.rs\ntags: [core, logging]\nstatus: draft\nsources:\n  - id: log4rs\n    resource: https://crates.io/log4rs\n    title: docs\nlinks:\n  - rel: depends-on\n    to: config\n    note: n\nverified:\n  - { by: human:rs, at: 2026-08-04T09:00:00Z }\nextra: 1\n",
        );
        assert!(c.problems.is_empty(), "{:?}", c.problems);
        assert!(c.present);
        assert_eq!(c.type_.as_deref(), Some("Subsystem"));
        assert_eq!(c.id.as_deref(), Some("subsystem-logging"));
        assert_eq!(
            c.tags.as_ref().unwrap(),
            &vec!["core".to_string(), "logging".to_string()]
        );
        assert_eq!(c.status, Some(Status::Draft));
        assert_eq!(c.sources[0].id.as_deref(), Some("log4rs"));
        assert_eq!(c.links[0].to, "config");
        assert_eq!(c.verified[0].by, "human:rs");
        assert_eq!(c.extra("extra").unwrap().as_int(), Some(1));
        assert_eq!(c.line_of("id"), Some(3));
        assert!(c.extra("nope").is_none());
        assert_eq!(c.resource, Some(Resource::One("/crates/x.rs".into())));
        let bare = fields("verified: { by: process:x, at: 2026-01-01 }\n");
        assert_eq!(bare.verified.len(), 1);
    }

    #[test]
    fn each_bad_shape_is_one_finding_naming_the_field() {
        let c = fields(
            "type: [x]\nid: 3\ntitle: 1\ndescription: []\ntags: x\nresource: relative\nstatus: nope\nsources:\n  - resource: /x\n    extra: 1\n  - id: only\n  - 3\nlinks:\n  - rel: x\n  - to: 1\n    rel: r\n  - rel: r\n    to: t\n    other: 1\nverified:\n  - { by: agent/1, at: 2026-01-01 }\n  - { by: human:x, at: soon }\n  - { by: human:x }\n",
        );
        let messages: Vec<&str> = c.problems.iter().map(|f| f.message.as_str()).collect();
        for needle in [
            "`type` must be a string",
            "`id` must be a string",
            "`title` must be a string",
            "`description` must be a string",
            "`tags` must be a list of strings",
            "`resource` must be a `/` path or a URL, or a list of them",
            "`status` must be",
            "has `extra`",
            "no `resource`",
            "each `sources` entry must be a map",
            "needs `rel` and `to`",
            "`links[].to` must be a string",
            "has `other`",
            "`human:` or `process:`",
            "ISO 8601",
            "needs `by` and `at`",
        ] {
            assert!(
                messages.iter().any(|m| m.contains(needle)),
                "missing {needle:?} in {messages:?}"
            );
        }
        assert_eq!(c.id.as_deref(), Some("3"));
        assert!(c.sources.is_empty());
        assert!(c.links.is_empty());
        assert!(c.verified.is_empty());
        let e = fields("type: ''\nsources: 1\nlinks: 1\nverified: 1\n");
        assert!(
            e.problems
                .iter()
                .any(|f| f.message.contains("`type` is empty"))
        );
        assert!(
            e.problems
                .iter()
                .any(|f| f.message.contains("`sources` must be a list"))
        );
        assert!(
            e.problems
                .iter()
                .any(|f| f.message.contains("`links` must be a list"))
        );
        assert!(
            e.problems
                .iter()
                .any(|f| f.message.contains("`verified` must be a list"))
        );
    }

    #[test]
    fn a_resource_is_one_or_a_list() {
        let list = fields("resource: [/a, https://x.y]\n");
        let r = list.resource.unwrap();
        assert_eq!(r.entries(), ["/a", "https://x.y"]);
        assert_eq!(r.one(), None);
        assert_eq!(Resource::One("/a".into()).entries(), ["/a"]);
        for bad in [
            "resource: []\n",
            "resource: [/a, rel]\n",
            "resource: [[/a]]\n",
        ] {
            assert!(fields(bad).resource.is_none(), "{bad}");
            assert_eq!(fields(bad).problems.len(), 1, "{bad}");
        }
    }

    #[test]
    fn missing_and_broken_frontmatter() {
        let path = RepoPath::parse("/k/a.md").unwrap();
        let none = core_fields(None, &path);
        assert!(!none.present);
        assert!(none.problems[0].message.contains("no frontmatter"));
        let err = YamlError::Anchor { line: 3 };
        let bad = core_fields(Some(Err(&err)), &path);
        assert_eq!(bad.problems[0].line, Some(3));
        let list = fields("- a\n");
        assert!(list.problems[0].message.contains("must be a map"));
    }

    // sokf-tests: FORMAT_P-10
    proptest::proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        /// sokf reads frontmatter through the engine's YAML reader, which
        /// refuses each anchor, alias, and tag with an error at the line of
        /// that anchor, alias, or tag.
        #[test]
        fn anchors_aliases_and_tags_are_errors_on_their_line(
            before in 0usize..4,
            name in "[a-z][a-z0-9]{0,5}",
        ) {
            let pad: String = (0..before).map(|i| format!("k{i}: v\n")).collect();
            let line = before + 1;
            proptest::prop_assert_eq!(
                yaml::parse(&format!("{pad}a: &{name} 1\n"), 1).unwrap_err(),
                YamlError::Anchor { line }
            );
            proptest::prop_assert_eq!(
                yaml::parse(&format!("{pad}a: *{name}\n"), 1).unwrap_err(),
                YamlError::Alias { line }
            );
            proptest::prop_assert_eq!(
                yaml::parse(&format!("{pad}a: !{name} 1\n"), 1).unwrap_err(),
                YamlError::Tag { line }
            );
        }

        /// A plain scalar takes its type from the YAML 1.2 core schema, and
        /// quoting makes any scalar a string.
        #[test]
        fn plain_scalars_follow_the_core_schema(
            n in proptest::prelude::any::<i64>(),
            b in proptest::prelude::any::<bool>(),
            s in "[a-z][a-z ]{0,10}[a-z]",
        ) {
            let get = |text: String| yaml::parse(&text, 1).unwrap().get("v").cloned().unwrap();
            proptest::prop_assert_eq!(get(format!("v: {n}\n")).as_int(), Some(n));
            proptest::prop_assert_eq!(get(format!("v: {b}\n")).as_bool(), Some(b));
            let plain = get(format!("v: {s}\n"));
            proptest::prop_assert_eq!(plain.as_str(), Some(s.as_str()));
            let quoted = get(format!("v: \"{n}\"\n"));
            let text = n.to_string();
            proptest::prop_assert_eq!(quoted.as_str(), Some(text.as_str()));
        }
    }
}
