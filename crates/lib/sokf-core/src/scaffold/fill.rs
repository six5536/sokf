//! This module fills a scaffold for `sokf new`. It writes the identity of the
//! new document into the frontmatter, and drops the keys that a new document
//! must not have. It also rewrites each heading bound to a frontmatter key to
//! the value of that key.

use super::gate::continuation_end;
use tramlines_md::derive::derived_title;

use crate::{
    Compiled, Error, Locator, Model, RepoPath, Result, Source,
    check::{Ctx, check_against, doc_view},
    fix::{AnchorEdit, rewrite},
    grammar::{Dispatch, SokfSchema, States},
    include::Expected,
    model::{Document, KeyIndex, ParseMode, parse_source},
    source::split,
    yaml,
};

/// An `Identity` is what `fill` writes into the frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The id of the document.
    pub id: String,
    /// The title of the document.
    pub title: String,
    /// The state of the document in each lifecycle dimension.
    pub states: States,
    /// The path of the asset, when the document is a sidecar.
    pub resource: Option<RepoPath>,
}

/// `identity` returns the identity of a new document. The id is the `id`
/// argument, unchanged. The title is the one given with `--title`. Without
/// `--title`, `identity` derives the title from the words of the slug, or
/// from the words of the id when there is no slug.
// sokf-implements: CLI-6_AC-6
// sokf-implements: CLI-6_AC-28
pub fn identity(
    id: &str,
    slug: Option<&str>,
    title: Option<&str>,
    states: States,
    resource: Option<RepoPath>,
) -> Identity {
    let title = title
        .map(str::to_string)
        .unwrap_or_else(|| derived_title(slug.unwrap_or(id)));
    Identity {
        id: id.to_string(),
        title,
        states,
        resource,
    }
}

/// `fill` edits the frontmatter of `document` line by line. The `id`,
/// `title`, `resource`, and lifecycle keys take their values from the
/// identity. A `title`, `resource`, or lifecycle key that is absent is
/// inserted after `id`. `fill` drops the `verified` and `generated` keys,
/// with their continuation lines. Every other line stays as it is, so that
/// the comments, the quoting, and the order of the scaffold stay the same,
/// byte for byte. `fill` writes a value without quotes when YAML parses it
/// back as the same text, and in double quotes otherwise. A document without
/// frontmatter gets a new frontmatter block that holds the `id`, the
/// `title`, and the `resource` when there is one.
// sokf-implements: CLI-6_AC-14
// sokf-implements: CLI_P-23
pub fn fill(document: &str, schema: &SokfSchema, identity: &Identity) -> String {
    let lines: Vec<&str> = document.lines().collect();
    let source = Source::from_text(RepoPath::root(), document);
    let close = match split(&source).frontmatter {
        Some((yaml, _)) => 1 + yaml.lines().count(),
        None => {
            let mut fm = format!(
                "---\nid: {}\ntitle: {}\n",
                scalar(&identity.id),
                scalar(&identity.title)
            );
            if let Some(r) = &identity.resource {
                fm.push_str(&format!("resource: {r}\n"));
            }
            fm.push_str("---\n");
            return fm + document;
        }
    };
    let mut out: Vec<Option<String>> = lines.iter().map(|l| Some((*l).to_string())).collect();
    let mut id_line = None;
    let mut seen_title = false;
    let mut seen_resource = false;
    let mut seen_states: Vec<&str> = Vec::new();
    let mut i = 1;
    while i < close.min(lines.len()) {
        let line = lines[i];
        let Some(key) = top_key(line) else {
            i += 1;
            continue;
        };
        let value = |v: &str| Some(format!("{key}: {}", scalar(v)));
        match key {
            "id" => {
                out[i] = value(&identity.id);
                id_line = Some(i);
            }
            "title" => {
                out[i] = value(&identity.title);
                seen_title = true;
            }
            "resource" if identity.resource.is_some() => {
                out[i] = identity.resource.as_ref().map(|r| format!("resource: {r}"));
                seen_resource = true;
            }
            "verified" | "generated" => {
                let end = continuation_end(&lines, i, close);
                out[i..end].fill(None);
                i = end;
                continue;
            }
            k => {
                if let Some(state) = identity.states.0.get(k).and_then(|s| s.first())
                    && schema.core.core.lifecycle.iter().any(|d| d.key == k)
                {
                    out[i] = value(state);
                    seen_states.push(k);
                }
            }
        }
        i += 1;
    }
    // sokf-implements: CLI-6_AC-53
    let mut inserted = Vec::new();
    if !seen_title {
        inserted.push(format!("title: {}", scalar(&identity.title)));
    }
    if let Some(r) = &identity.resource
        && !seen_resource
    {
        inserted.push(format!("resource: {r}"));
    }
    for dim in &schema.core.core.lifecycle {
        if !seen_states.contains(&dim.key.as_str())
            && let Some(state) = identity.states.0.get(&dim.key).and_then(|s| s.first())
        {
            inserted.push(format!("{}: {}", dim.key, scalar(state)));
        }
    }
    let at = id_line.map_or(close, |i| i + 1);
    let mut text = Vec::new();
    for (i, slot) in out.into_iter().enumerate() {
        if i == at {
            text.append(&mut inserted);
        }
        if let Some(line) = slot {
            text.push(line);
        }
    }
    if at >= lines.len() {
        text.extend(inserted);
    }
    let mut text = text.join("\n");
    if document.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// `top_key` returns the key of a top-level frontmatter line. Such a line is
/// not indented and not a comment, and holds `key:` followed by a space or
/// the end of the line.
fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('#') {
        return None;
    }
    let (key, rest) = line.split_once(':')?;
    if key.is_empty() || !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    Some(key)
}

/// `scalar` writes a value as a YAML scalar: without quotes when YAML parses
/// it back as the same text, and in double quotes otherwise. An empty value,
/// or one that holds `#`, is always written in double quotes.
// sokf-implements: CLI-6_AC-54
fn scalar(value: &str) -> String {
    let plain = yaml::parse(&format!("k: {value}\n"), 1)
        .ok()
        .and_then(|y| y.get("k").and_then(|v| v.scalar_text()))
        .is_some_and(|read| read == value)
        && !value.is_empty()
        && !value.contains('#');
    if plain {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// `bind` checks the filled text against the schema as a scaffold, which
/// allows placeholders, and rewrites the bound headings, paragraphs, and
/// items of the text to the values of their keys. When the `title` does not
/// match a pattern in the schema, `bind` returns an error that cites the
/// schema of the document's type. `bind` ignores
/// every other finding, because each one reports a placeholder that the
/// author fills in later.
// sokf-implements: CLI-6_AC-8
// sokf-implements: CLI-6_AC-36
// sokf-implements: CLI-6_AC-42
// sokf-implements: CLI-6_AC-56
pub fn bind(
    document: &str,
    path: &RepoPath,
    schema: &SokfSchema,
    states: &States,
    model: &Model,
    compiled: &Compiled,
    knowledge: Option<usize>,
) -> Result<String> {
    // The findings cite the schema that a title given with `--title` fails.
    let schema_id = match (&schema.dispatch, knowledge) {
        (Dispatch::Type(type_name), Some(k)) => compiled
            .for_type(model, k, type_name)
            .map(|(r, _)| model.schema(r).id())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let source = Source::from_text(path.clone(), document);
    let parsed = parse_source(&source, ParseMode::Document);
    let doc = Document::new(source, parsed);
    let expected = Expected::default();
    let keys = KeyIndex::default();
    let ctx = Ctx {
        model,
        compiled,
        expected: &expected,
        keys: &keys,
        knowledge,
        scaffold: true,
    };
    let out = check_against(
        doc_view(&doc),
        schema,
        states,
        &ctx,
        Locator::schema(schema_id),
    );
    if let Some(f) = out
        .findings
        .iter()
        .find(|f| f.message.starts_with("`title` `"))
    {
        return Err(Error::New(format!(
            "the title doesn't match the schema: {}. Give another title with `--title`",
            crate::report::message(f)
        )));
    }
    if out.repairs.is_empty() && !out.anchors_stale {
        return Ok(document.to_string());
    }
    // The rewrite places the anchor of each key, so that the new document
    // passes validation without `sokf validate --fix` (SOKF_FORMAT_83,
    // SOKF_FORMAT_84).
    let anchors = AnchorEdit {
        body: &doc.body,
        placements: &out.anchors,
        text: &doc.source.text,
        renames: &[],
    };
    rewrite(
        &doc.source.text,
        &doc.segments,
        Vec::new(),
        &out.repairs,
        Some(&anchors),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::fixtures::{schema, states};

    // sokf-tests: CLI-6_AC-6
    // sokf-tests: CLI-6_AC-28
    #[test]
    fn the_title_derives_from_the_slug_or_the_id() {
        let i = identity(
            "issue-004-retry",
            Some("retry"),
            None,
            States::default(),
            None,
        );
        assert_eq!(i.title, "Retry");
        let i = identity("note-a", None, Some("Given"), States::default(), None);
        assert_eq!(i.title, "Given");
    }

    // sokf-tests: CLI_P-23
    #[test]
    fn fill_edits_the_identity_lines_and_drops_the_rest() {
        let template = "---\ntype: Issue # the const\nid: issue-001-example\ntitle: Example\ndescription: One line.\nstate: closed\nkind: bug\nverified:\n  - by: human:a\n    at: 2026-01-01T00:00:00Z\ngenerated: { by: x, at: y }\ntags: [t]\n---\n\n# Example\n";
        let ident = identity(
            "issue-004-retry-logic",
            Some("retry-logic"),
            None,
            states(&[("state", "open"), ("kind", "feature")]),
            None,
        );
        assert_eq!(
            fill(template, &schema(), &ident),
            "---\ntype: Issue # the const\nid: issue-004-retry-logic\ntitle: Retry logic\ndescription: One line.\nstate: open\nkind: feature\ntags: [t]\n---\n\n# Example\n"
        );
    }

    // sokf-tests: CLI-6_AC-14
    // sokf-tests: CLI_P-23
    #[test]
    fn fill_inserts_what_is_absent_and_quotes_what_needs_it() {
        let template = "---\ntype: Note\nid: n\n---\n\n# N\n";
        let ident = identity(
            "note-a",
            None,
            Some("A: colon \"and\" quotes"),
            states(&[("state", "open"), ("kind", "bug")]),
            Some(RepoPath::parse("/knowledge/media/a.png").unwrap()),
        );
        assert_eq!(
            fill(template, &schema(), &ident),
            "---\ntype: Note\nid: note-a\ntitle: \"A: colon \\\"and\\\" quotes\"\nresource: /knowledge/media/a.png\nstate: open\nkind: bug\n---\n\n# N\n"
        );
        assert_eq!(scalar("plain words"), "plain words");
        assert_eq!(scalar("a # comment"), "\"a # comment\"");
        assert_eq!(scalar(""), "\"\"");
        assert_eq!(scalar("[list]"), "\"[list]\"");
        let bare = "# N\n";
        let ident = identity("note-a", None, None, States::default(), None);
        assert_eq!(
            fill(bare, &schema(), &ident),
            "---\nid: note-a\ntitle: Note a\n---\n# N\n"
        );
    }
}
