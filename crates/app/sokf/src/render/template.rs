//! This module parses and renders the template that `--format` sets for the
//! text form of `graph`. The Text form section of design-graph describes
//! the template.

use serde_json::Value;

/// `Template` holds a parsed template: its literal text and its `{key}`
/// placeholders, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Text(String),
    Key(String),
}

impl Template {
    /// `parse` reads a template from `s`, in which `{` opens a placeholder and
    /// `}` closes it. It returns an error message when a brace is unmatched
    /// or a placeholder contains a `{`.
    // sokf-implements: CLI-7_AC-21
    pub fn parse(s: &str) -> Result<Template, String> {
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut key: Option<String> = None;
        for c in s.chars() {
            match (c, &mut key) {
                ('{', None) => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    key = Some(String::new());
                }
                ('{', Some(_)) => return Err("a `{` is inside a placeholder".into()),
                ('}', Some(k)) => {
                    parts.push(Part::Key(std::mem::take(k)));
                    key = None;
                }
                ('}', None) => return Err("a `}` has no `{` before it".into()),
                (c, Some(k)) => k.push(c),
                (c, None) => text.push(c),
            }
        }
        if key.is_some() {
            return Err("a `{` has no `}` after it".into());
        }
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(Template { parts })
    }

    /// `render` returns the line for a node object. A document's line is the
    /// template with each placeholder replaced by the value of its key, or by
    /// nothing when the document lacks the key. When the document has no
    /// `id`, `{id}` is replaced by the document's path. Any other node's line
    /// is its path or target.
    pub fn render(&self, node: &Value) -> String {
        if node.get("kind").and_then(Value::as_str) != Some("document") {
            return path_or_target(node);
        }
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Text(t) => out.push_str(t),
                Part::Key(k) => match node.get(k) {
                    Some(v) => out.push_str(&value(v)),
                    None if k == "id" => out.push_str(&path_or_target(node)),
                    None => {}
                },
            }
        }
        out
    }
}

/// `value` returns a string unchanged, a list as its elements joined with
/// `, `, and any other value as JSON.
fn value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(value).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

/// `path_or_target` returns the `path` of a node, or the `target` of a
/// missing node, or an empty string when the node has neither.
pub fn path_or_target(node: &Value) -> String {
    node.get("path")
        .or_else(|| node.get("target"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::*;

    #[test]
    fn renders_keys_by_the_value_rules() {
        let node = json!({
            "kind": "document", "path": "/k/a.md", "id": "a", "title": "A",
            "tags": ["x", ["y", "z"]], "n": 1, "b": true, "m": {"k": "v"}, "nothing": null
        });
        let render = |t: &str| Template::parse(t).unwrap().render(&node);
        assert_eq!(render("{id}"), "a");
        assert_eq!(render("{id}  {title}"), "a  A");
        assert_eq!(render("{tags}"), "x, y, z");
        assert_eq!(render("{n}/{b}/{m}/{nothing}"), "1/true/{\"k\":\"v\"}/null");
        assert_eq!(render("[{absent}]"), "[]");
        assert_eq!(render("plain"), "plain");
        assert_eq!(render(""), "");
        // A broken document without an `id` renders `{id}` as its path.
        let broken =
            json!({"kind": "document", "path": "/k/b.md", "broken": true, "reason": "no id"});
        assert_eq!(Template::parse("{id}").unwrap().render(&broken), "/k/b.md");
        // Any other node renders as its path or target, whatever the template holds.
        let file = json!({"kind": "file", "path": "/src/a.rs"});
        assert_eq!(Template::parse("{id}").unwrap().render(&file), "/src/a.rs");
        let missing = json!({"kind": "missing", "target": "planer"});
        assert_eq!(Template::parse("{id}").unwrap().render(&missing), "planer");
        assert_eq!(path_or_target(&json!({"kind": "x"})), "");
    }

    #[test]
    fn an_unmatched_brace_does_not_parse() {
        assert!(Template::parse("{id").is_err());
        assert!(Template::parse("id}").is_err());
        assert!(Template::parse("{{id}}").is_err());
        assert!(Template::parse("{a}{b}").is_ok());
    }

    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn a_template_round_trips(text in "[a-zA-Z0-9 :,/._-]{0,20}", key in "[a-z_][a-z0-9_]{0,8}", value in "[a-zA-Z0-9 ]{0,12}") {
            prop_assume!(key != "kind");
            let node = json!({"kind": "document", "path": "/k/a.md", (key.clone()): value.clone()});
            prop_assert_eq!(Template::parse(&text).unwrap().render(&node), text.clone());
            let with_key = format!("{text}{{{key}}}{text}");
            prop_assert_eq!(Template::parse(&with_key).unwrap().render(&node), format!("{text}{value}{text}"));
            let open = format!("{text}{{{key}");
            let close = format!("{text}}}{key}");
            prop_assert!(Template::parse(&open).is_err());
            prop_assert!(Template::parse(&close).is_err());
        }
    }
}
