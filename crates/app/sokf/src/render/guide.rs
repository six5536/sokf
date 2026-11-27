//! This module writes the text output of `sokf guide` (CLI-17_AC-16), which
//! `sokf new` also prints for the document that it creates: for each
//! document, a line that names it, the schema's guidance, the guidance of the
//! frontmatter, and each section under its heading. Each part's text includes
//! the words that state each limit, as the scaffold writes them (CLI-17_AC-29),
//! and the part's example below it (CLI-17_AC-38), unless the document holds
//! the example already or an example that the guide prints holds it
//! (CLI-17_AC-40). The words and the meaning of each label come once, after
//! the line that names the document, and each block whose units carry a label
//! says so.

use std::io::{self, Write};

use sokf_core::{BlockGuide, DocumentGuide, GuideLabel, GuideReport};

/// `text` writes the guide of each document of `report`, in order, with a blank
/// line between parts. A part that has neither guidance nor an example to
/// print prints nothing, except a section, which prints its heading.
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
    // A key's example is held where the frontmatter holds its lines as
    // written, and the example of the body's parts where the body holds it.
    let body_text = doc.body_text.as_str();
    let held_key = |e: &&str| holds_as_written(&doc.frontmatter_text, e);
    let held = |e: &&str| holds(body_text, e);
    let mut front: Vec<String> = Vec::new();
    for k in &doc.frontmatter {
        let example = k.example.as_deref().filter(|e| !held_key(e));
        if k.text.is_none() && example.is_none() {
            continue;
        }
        let name = format!("{}{}", k.key, if k.optional { " (optional)" } else { "" });
        front.push(match &k.text {
            Some(text) => format!("  {name}: {text}"),
            None => format!("  {name}:"),
        });
        front.extend(example_lines(example, 2));
    }
    // sokf-implements: CLI-17_AC-33
    // sokf-implements: CLI-17_AC-41
    for l in &doc.links {
        let optional = if l.optional { ", optional" } else { "" };
        let name = format!("links ({}{optional})", l.rel);
        front.push(match &l.text {
            Some(text) => format!("  {name}: {text}"),
            None => format!("  {name}:"),
        });
        front.extend(example_lines(Some(&l.example), 2));
    }
    if !front.is_empty() {
        front.insert(0, "Frontmatter:".to_string());
        parts.push(front);
    }
    if let Some(p) = &doc.preamble {
        let mut lines = vec!["Preamble:".to_string()];
        lines.extend(p.text.iter().map(|t| format!("  {t}")));
        let example = p.example.as_deref().filter(|e| !held(e));
        lines.extend(example_lines(example, 1));
        blocks(body_text, &p.blocks, 1, example.is_some(), &mut lines);
        if lines.len() > 1 {
            parts.push(lines);
        }
    }
    // The rule-ids of the sections whose example the guide prints, or whose
    // parent's example holds theirs: each holds the example of each section
    // inside it.
    let mut covering: Vec<&str> = Vec::new();
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
        let covered = s.parent.as_deref().is_some_and(|p| covering.contains(&p));
        // A section that the document holds shows its parts through the
        // examples of its blocks, unless it repeats, because its other
        // occurrences are in no block's example, and the check's match tells
        // whether the document holds it, where the same text can stand under
        // another parent. An example that is the heading alone says nothing
        // that the guide's heading doesn't.
        let example = s
            .example
            .as_deref()
            .filter(|e| !covered && !heading_alone(e, s.level))
            .filter(|_| s.repeat || !s.held);
        if (covered || example.is_some())
            && let Some(id) = &s.rule_id
        {
            covering.push(id);
        }
        lines.extend(example_lines(example, 1));
        blocks(
            body_text,
            &s.blocks,
            1,
            covered || example.is_some(),
            &mut lines,
        );
        parts.push(lines);
    }
    parts
}

/// `blocks` adds a line for each block of `list` that has guidance, an
/// example to print, or units that carry a label, or that is optional, as its
/// kind, its marks, and its text, at the indent `depth`, with its example
/// below it and the blocks inside it one step further in. A block without any
/// of them prints only its kind, and only when a block inside it prints a
/// line. Where `covered`, an example that the guide prints above holds the
/// example of each block, so none is printed again.
fn blocks(body: &str, list: &[BlockGuide], depth: usize, covered: bool, out: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    for b in list {
        let example = b.example.as_deref().filter(|e| !covered && !holds(body, e));
        let mut inner = Vec::new();
        blocks(
            body,
            &b.blocks,
            depth + 1,
            covered || example.is_some(),
            &mut inner,
        );
        let mark = marks(b);
        match &b.text {
            Some(t) => out.push(format!("{indent}{}{mark}: {t}", b.kind)),
            None if !inner.is_empty() || !mark.is_empty() || example.is_some() => {
                out.push(format!("{indent}{}{mark}", b.kind));
            }
            None => {}
        }
        out.extend(example_lines(example, depth + 1));
        out.extend(inner);
    }
}

/// `heading_alone` reports whether `example` is only a heading at `level`,
/// which says nothing that the guide's heading of the section doesn't, whether
/// the schema fixes the heading's text or a value or a template gives it.
fn heading_alone(example: &str, level: u8) -> bool {
    let mut lines = example.lines().filter(|l| !l.trim().is_empty());
    let marks = "#".repeat(usize::from(level)) + " ";
    lines.next().is_some_and(|l| l.starts_with(&marks)) && lines.next().is_none()
}

/// `example_lines` returns the lines that print `example` below its part at
/// the indent `depth`: `Example:`, then each line of the example one step
/// further in, with an empty line left empty. It returns no line for no
/// example.
// sokf-implements: CLI-17_AC-38
fn example_lines(example: Option<&str>, depth: usize) -> Vec<String> {
    let Some(example) = example else {
        return Vec::new();
    };
    let indent = "  ".repeat(depth);
    let mut lines = vec![format!("{indent}Example:")];
    lines.extend(example.lines().map(|l| {
        if l.is_empty() {
            String::new()
        } else {
            format!("{indent}  {l}")
        }
    }));
    lines
}

/// `holds_as_written` reports whether `text` holds each line of `example` as
/// written, indent and all, in a run, so that a key nested under another
/// key holds no example of a key at the top.
// sokf-implements: CLI-17_AC-40
fn holds_as_written(text: &str, example: &str) -> bool {
    let want: Vec<&str> = example.lines().collect();
    let have: Vec<&str> = text.lines().collect();
    !want.is_empty() && have.windows(want.len()).any(|w| w == want.as_slice())
}

/// `holds` reports whether `document` holds each line of `text` already, in
/// a run, each but for its indent, as a block of an item is written under
/// the item, so that the guide leaves out an example that the writer has in
/// front of them, such as the line of the id.
// sokf-implements: CLI-17_AC-40
fn holds(document: &str, text: &str) -> bool {
    let want: Vec<&str> = text.lines().map(str::trim_start).collect();
    let have: Vec<&str> = document.lines().map(str::trim_start).collect();
    !want.is_empty() && have.windows(want.len()).any(|w| w == want.as_slice())
}

/// `marks` returns the marks of `b` after its kind: `optional` where the
/// block can be absent, then whether the units of `b` require a label or
/// allow one, such as ` (label required)`, with each unit rule that declares
/// its own `label` named by its `rule-id`, or else by its line, as in
/// ``(label optional; required for `decision`)``. It is empty for a required
/// block whose units carry no label.
// sokf-implements: CLI-17_AC-37
// sokf-implements: CLI-17_AC-41
fn marks(b: &BlockGuide) -> String {
    let labels: Vec<String> = b
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
    let mut parts = Vec::new();
    if b.optional {
        parts.push("optional".to_string());
    }
    if !labels.is_empty() {
        parts.push(labels.join("; "));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A key's example is held where the frontmatter holds it as written, so
    // a key nested under another holds none, and a block's example where
    // the body holds it, indent aside, as a block of an item is written.
    // sokf-tests: CLI-17_AC-40
    #[test]
    fn an_example_is_held_by_its_own_part_of_the_document() {
        let front = "type: Note\nreviewer:\n  owner: ada";
        assert!(holds_as_written(front, "type: Note"));
        assert!(!holds_as_written(front, "owner: ada"));
        assert!(holds("- a\n  - b\n", "- b"));
        assert!(heading_alone("## XYZ", 2) && heading_alone("## Step 1\n", 2));
        assert!(!heading_alone("## Notes\n\nThe kettle is full.", 2));
        assert!(!heading_alone("### Notes", 2));
    }
}
