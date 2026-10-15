//! This module reads `archived`, the key with which a schema declares which
//! documents of its type are archived. It reads `archived` from a schema
//! before sokf compiles the schema, in the same way that it reads the
//! globs. As a result, the graph can leave out archived documents without
//! compiling a schema or reading a document (CLI-7_AC-67).

use std::collections::{BTreeMap, BTreeSet};

use super::{Document, Model};
use crate::{RepoPath, Spanned, Yaml};

/// `Archived` says which documents of a type are archived.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Archived {
    /// No document is archived, because the schema declares no `archived`.
    #[default]
    None,
    /// Every document is archived, because the schema has no lifecycle and
    /// declares `archived: true`.
    All,
    /// A document is archived when it is in an archived state of any
    /// dimension. Each entry holds the key of a dimension and its archived
    /// states, in lifecycle order.
    States(Vec<(String, Vec<String>)>),
}

impl Archived {
    /// `of_schema` returns the `archived` of the schema `yaml`: `All` for
    /// `archived: true` on a schema with no lifecycle, `States` for the
    /// `archived` lists of its lifecycle dimensions, or `None` when it
    /// declares neither.
    pub fn of_schema(yaml: &Spanned) -> Archived {
        // The schema grammar rejects `archived: true` beside a lifecycle, so
        // for such a schema no document is archived.
        if yaml.get("archived").and_then(Spanned::as_bool) == Some(true) {
            let lifecycle = yaml
                .get("lifecycle")
                .and_then(Spanned::as_seq)
                .is_some_and(|d| !d.is_empty());
            return if lifecycle {
                Archived::None
            } else {
                Archived::All
            };
        }
        let dimensions: Vec<(String, Vec<String>)> = yaml
            .get("lifecycle")
            .and_then(Spanned::as_seq)
            .unwrap_or(&[])
            .iter()
            .filter_map(|d| {
                let key = d.get("key")?.as_str()?.to_string();
                let states = d.get("archived")?.as_str_or_list()?;
                Some((key, states))
            })
            .collect();
        if dimensions.is_empty() {
            Archived::None
        } else {
            Archived::States(dimensions)
        }
    }

    /// `encode` returns the text that the store keeps: an empty string for
    /// `None`, `*` for `All`, or one line for each dimension, with the
    /// dimension's key and its states separated by tabs.
    pub fn encode(&self) -> String {
        match self {
            Archived::None => String::new(),
            Archived::All => "*".to_string(),
            Archived::States(dimensions) => dimensions
                .iter()
                .map(|(key, states)| format!("{key}\t{}", states.join("\t")))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// `decode` returns the `Archived` from which `encode` wrote `text`.
    pub fn decode(text: &str) -> Archived {
        match text {
            "" => Archived::None,
            "*" => Archived::All,
            _ => Archived::States(
                text.lines()
                    .filter_map(|line| {
                        let mut parts = line.split('\t');
                        let key = parts.next()?.to_string();
                        Some((key, parts.map(str::to_string).collect()))
                    })
                    .collect(),
            ),
        }
    }

    /// `holds` says whether a document is archived, where `values` holds
    /// the scalar values of the document's frontmatter. For `All`, every
    /// document is archived. For `States`, a document is archived when its
    /// value for the key of any dimension is one of that dimension's
    /// archived states.
    // sokf-implements: EXT-2_AC-4
    // sokf-implements: EXT-7_AC-3
    pub fn holds(&self, values: &[(String, String)]) -> bool {
        match self {
            Archived::None => false,
            Archived::All => true,
            Archived::States(dimensions) => dimensions.iter().any(|(key, states)| {
                values
                    .iter()
                    .find(|(k, _)| k == key)
                    .is_some_and(|(_, v)| states.contains(v))
            }),
        }
    }
}

/// `scalars` returns the scalar values of a document's frontmatter that the
/// store keeps, so that the graph can find a lifecycle state without
/// reading the file. It returns each top-level key whose value is a
/// string, an integer, or a boolean, with the value as text. It leaves out
/// `id`, `type`, `title`, and `description`, which the store keeps as
/// columns.
pub fn scalars(doc: &Document) -> Vec<(String, String)> {
    let Some(entries) = doc.frontmatter.as_ref().and_then(Spanned::as_map) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|(k, _)| !matches!(k.value.as_str(), "id" | "type" | "title" | "description"))
        .filter_map(|(k, v)| {
            let text = match &v.value {
                Yaml::Str(s) => s.clone(),
                Yaml::Int(i) => i.to_string(),
                Yaml::Bool(b) => b.to_string(),
                _ => return None,
            };
            Some((k.value.clone(), text))
        })
        .collect()
}

/// `encode_scalars` returns the text that the store keeps for `scalars`:
/// one line for each key. Each line holds the key and its value separated
/// by a tab. It writes each line break in a value as a space.
pub fn encode_scalars(scalars: &[(String, String)]) -> String {
    scalars
        .iter()
        .map(|(k, v)| format!("{k}\t{}", v.replace(['\n', '\r'], " ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `decode_scalars` returns the scalars from which `encode_scalars` wrote
/// `text`.
pub fn decode_scalars(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

impl Model {
    /// `archive_rules` returns the `archived` of each type, in each store,
    /// whose schema declares one.
    fn archive_rules(&self) -> BTreeMap<(usize, String), Archived> {
        let mut rules = BTreeMap::new();
        for (k, knowledge) in self.knowledges.iter().enumerate() {
            for schema in &knowledge.schemas {
                if let Some(t) = &schema.type_const
                    && schema.archived != Archived::None
                {
                    rules.insert((k, t.clone()), schema.archived.clone());
                }
            }
        }
        rules
    }

    /// `archived_documents` returns the archived documents of every store.
    /// For each document that this run loaded, it reads the frontmatter to
    /// decide whether the document is archived. For every other document, it
    /// reads the store's rows.
    // sokf-implements: CLI-7_AC-67
    pub fn archived_documents(&self) -> BTreeSet<RepoPath> {
        let rules = self.archive_rules();
        let mut out = BTreeSet::new();
        if rules.is_empty() {
            return out;
        }
        let mut held = BTreeSet::new();
        for (k, doc) in self.documents() {
            held.insert(doc.path.clone());
            let Some(rule) = doc.type_name().and_then(|t| rules.get(&(k, t.to_string()))) else {
                continue;
            };
            if rule.holds(&scalars(doc)) {
                out.insert(doc.path.clone());
            }
        }
        if let Some(fallback) = &self.fallback {
            for ((k, type_name), rule) in &rules {
                for (path, values) in fallback.scalars_of_type(*k, type_name) {
                    if held.contains(&path) {
                        continue;
                    }
                    if rule.holds(&values) {
                        out.insert(path);
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(text: &str) -> Spanned {
        crate::yaml::parse(text, 1).unwrap()
    }

    // sokf-tests: EXT-2_AC-4
    // sokf-tests: EXT-7_AC-3
    #[test]
    fn archived_reads_encodes_and_holds() {
        let states = Archived::of_schema(&schema(
            "lifecycle:\n  - key: state\n    states: [open, closed]\n    archived: [closed]\n  - key: kind\n    states: [a]\n",
        ));
        assert_eq!(
            states,
            Archived::States(vec![("state".into(), vec!["closed".into()])])
        );
        assert_eq!(Archived::decode(&states.encode()), states);
        let state = |v: &str| [("state".to_string(), v.to_string())];
        assert!(states.holds(&state("closed")));
        assert!(!states.holds(&state("open")));
        assert!(!states.holds(&[]));
        let all = Archived::of_schema(&schema("archived: true\n"));
        assert_eq!(all, Archived::All);
        assert_eq!(Archived::decode(&all.encode()), all);
        assert!(all.holds(&[]));
        // An empty lifecycle counts as no lifecycle, so with `archived: true`
        // every document is archived.
        assert_eq!(
            Archived::of_schema(&schema("lifecycle: []\narchived: true\n")),
            Archived::All
        );
        let none = Archived::of_schema(&schema("archived: false\n"));
        assert_eq!(none, Archived::None);
        assert_eq!(Archived::decode(&none.encode()), none);
        assert!(!none.holds(&state("x")));
    }

    #[test]
    fn scalars_keep_the_values_the_columns_do_not() {
        let source = crate::Source::from_text(
            RepoPath::parse("/k/a.md").unwrap(),
            "---\ntype: A\nid: a\ntitle: T\nstate: closed\nseq: 3\ndone: true\ntags: [x]\n---\n\n# T\n",
        );
        let parsed = super::super::parse_source(&source, super::super::ParseMode::Document);
        let doc = Document::new(source, parsed);
        let values = scalars(&doc);
        assert_eq!(
            values,
            vec![
                ("state".to_string(), "closed".to_string()),
                ("seq".to_string(), "3".to_string()),
                ("done".to_string(), "true".to_string()),
            ]
        );
        assert_eq!(decode_scalars(&encode_scalars(&values)), values);
    }
}
