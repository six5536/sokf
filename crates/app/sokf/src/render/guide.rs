//! This module writes the text output of `sokf guide` (CLI-17_AC-16): for each
//! document, a line that names it, the schema's guidance, the guidance of the
//! frontmatter, and each section under its heading. Each part's text includes
//! the words that state each limit, as the scaffold writes them (CLI-17_AC-29).
//! The words and the meaning of each label come once, after the line that
//! names the document, and each block whose units carry a label says so.

use std::io::{self, Write};

use sokf_core::{BlockGuide, DocumentGuide, GuideLabel, GuideReport};

/// `text` writes the guide of each document of `report`, in order, with a blank
/// line between parts. A part that has no guidance prints nothing, except a
/// section, which prints its heading.
// sokf-implements: CLI-17_AC-16
pub fn text<W: Write>(w: &mut W, report: &GuideReport) -> io::Result<()> {
    let mut parts: Vec<Vec<String>> = Vec::new();
    for doc in &report.documents {
        parts.extend(document(doc));
    }
    let chunks: Vec<String> = parts.iter().map(|lines| lines.join("\n")).collect();
    if !chunks.is_empty() {
        writeln!(w, "{}", chunks.join("\n\n"))?;
    }
    Ok(())
}

/// `document` returns the parts of one document's guide, each as its lines.
fn document(doc: &DocumentGuide) -> Vec<Vec<String>> {
    let mut named = vec![doc.type_name.clone()];
    named.extend(
        doc.states
            .iter()
            .map(|(k, v)| format!("{k}={}", v.as_str().unwrap_or_default())),
    );
    let mut parts = vec![vec![format!("{} ({})", doc.path, named.join(", "))]];
    // sokf-implements: CLI-17_AC-35
    // sokf-implements: CLI-17_AC-36
    if !doc.labels.is_empty() {
        let mut lines = vec!["Labels:".to_string()];
        lines.extend(doc.labels.iter().map(|l| match &l.meaning {
            Some(m) => format!("  [{}]: {m}", l.words),
            None => format!("  [{}]", l.words),
        }));
        parts.push(lines);
    }
    if let Some(t) = &doc.text {
        parts.push(vec![t.clone()]);
    }
    let mut front: Vec<String> = doc
        .frontmatter
        .iter()
        .filter_map(|k| {
            let text = k.text.as_ref()?;
            Some(format!("  {}: {text}", k.key))
        })
        .collect();
    // sokf-implements: CLI-17_AC-33
    front.extend(
        doc.links
            .iter()
            .filter_map(|l| Some(format!("  links ({}): {}", l.rel, l.text.as_ref()?))),
    );
    if !front.is_empty() {
        front.insert(0, "Frontmatter:".to_string());
        parts.push(front);
    }
    if let Some(p) = &doc.preamble {
        let mut lines = vec!["Preamble:".to_string()];
        lines.extend(p.text.iter().map(|t| format!("  {t}")));
        blocks(&p.blocks, 1, &mut lines);
        if lines.len() > 1 {
            parts.push(lines);
        }
    }
    for s in &doc.sections {
        // sokf-implements: CLI-17_AC-14
        // sokf-implements: CLI-17_AC-28
        // A required section inside a parent that the document doesn't
        // hold is required only with the parent, so the mark names it.
        let absent_parent = s.parent.as_ref().and_then(|p| {
            doc.sections
                .iter()
                .find(|o| o.rule_id.as_ref() == Some(p) && !o.held)
        });
        let mark = match (s.held, s.optional, absent_parent) {
            (true, _, _) => String::new(),
            (false, true, _) => " (absent, optional)".to_string(),
            (false, false, Some(p)) => format!(" (absent, required under {})", p.heading),
            (false, false, None) => " (absent, required)".to_string(),
        };
        let mut lines = vec![format!(
            "{} {}{mark}",
            "#".repeat(usize::from(s.level)),
            s.heading
        )];
        lines.extend(s.text.iter().map(|t| format!("  {t}")));
        blocks(&s.blocks, 1, &mut lines);
        parts.push(lines);
    }
    parts
}

/// `blocks` adds a line for each block of `list` that has guidance or whose
/// units carry a label, as its kind, the mark of its labels, and its text, at
/// the indent `depth`, with the blocks inside it one step further in. A block
/// without either prints only its kind, and only when a block inside it
/// prints a line.
fn blocks(list: &[BlockGuide], depth: usize, out: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    for b in list {
        let mut inner = Vec::new();
        blocks(&b.blocks, depth + 1, &mut inner);
        let mark = label_mark(b);
        match &b.text {
            Some(t) => out.push(format!("{indent}{}{mark}: {t}", b.kind)),
            None if !inner.is_empty() || !mark.is_empty() => {
                out.push(format!("{indent}{}{mark}", b.kind));
            }
            None => {}
        }
        out.extend(inner);
    }
}

/// `label_mark` returns the mark that says whether the units of `b` require
/// a label or allow one, such as ` (label required)`, with each unit rule
/// that declares its own `label` named by its `rule-id`, or else by its line,
/// as in ``(label optional; required for `decision`)``. It is empty for a block
/// whose units carry no label.
// sokf-implements: CLI-17_AC-37
fn label_mark(b: &BlockGuide) -> String {
    let parts: Vec<String> = b
        .labels
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let kind = match l.label {
                GuideLabel::Required => "required",
                GuideLabel::Optional => "optional",
            };
            let rule = match (l.rule, &l.rule_id) {
                (None, _) => String::new(),
                (Some(_), Some(id)) => format!(" for `{id}`"),
                (Some(_), None) => format!(" for the rule at line {}", l.line),
            };
            let lead = if i == 0 { "label " } else { "" };
            format!("{lead}{kind}{rule}")
        })
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}
