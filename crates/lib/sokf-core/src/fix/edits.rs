//! This module builds the edits that SOKF makes to a file: they rewrite the
//! interior of an include, the definition block, and the `resource` field of
//! a sidecar. SOKF applies them together with the engine's edits, in one pass
//! (SOKF_VALIDATION_14, SOKF_VALIDATION_22).

use std::ops::Range;

use tramlines_md::edit::{AnchorEdit, Edit, apply, line_edits};

use crate::{Result, check::Repair, markdown::Segments};

/// `include_edit` regenerates the interior of an include block: it replaces
/// the lines between the block's markers with `interior` (SOKF_FORMAT_85).
pub fn include_edit(open: usize, close: usize, interior: &str) -> Edit {
    let new: Vec<String> = interior.lines().map(str::to_string).collect();
    Edit::replace(open + 1..close, &new)
}

/// `definition_edit` regenerates the definition block as `tail`, and an
/// empty `tail` removes the block. The edit replaces the block, from its
/// `marker` line to the end of the text, with one blank line and `tail`. The
/// replaced lines include the blank lines before the marker. When the text
/// has no marker, the edit replaces the blank lines at the end of the text,
/// so `tail` follows the last line that isn't blank (SOKF_VALIDATION_43).
pub fn definition_edit(text: &str, marker: Option<usize>, tail: &str) -> Edit {
    let lines: Vec<&str> = text.lines().collect();
    let end = lines.len() + 1;
    let mut start = marker.unwrap_or(end);
    while start > 1 && lines[start - 2].trim().is_empty() {
        start -= 1;
    }
    let mut new: Vec<String> = Vec::new();
    if !tail.is_empty() {
        if start > 1 {
            new.push(String::new());
        }
        new.extend(tail.lines().map(str::to_string));
    }
    Edit::replace(start..end, &new)
}

/// A `ResourceEdit` sets the `resource` field of a sidecar to the path of
/// its asset (SOKF_VALIDATION_22). It replaces the lines of the `resource`
/// key and its value when the frontmatter has them, and otherwise inserts a
/// `resource` line after the `id` line, or after the line that opens the
/// frontmatter. No other line of the frontmatter changes (SOKF_VALIDATION_23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEdit {
    /// `lines` are the numbers of the `resource` line and of the lines that
    /// continue its value, as with a list, when the frontmatter has them.
    pub lines: Option<Range<usize>>,
    /// `after` is the number of the line that a missing `resource` line
    /// follows.
    pub after: usize,
    /// `value` is the path of the asset.
    pub value: String,
}

impl ResourceEdit {
    /// `edit` returns the edit that writes the `resource` line.
    // sokf-implements: FORMAT-1_AC-5
    pub fn edit(&self) -> Edit {
        let line = [format!("resource: {}", self.value)];
        match &self.lines {
            Some(lines) => Edit::replace(lines.clone(), &line),
            None => Edit::replace(self.after + 1..self.after + 1, &line),
        }
    }
}

// sokf:begin rewrite
/// `rewrite` applies three sets of edits to `text` in one pass: SOKF's
/// `edits`, the engine's edits for `repairs`, and, when `anchors` is given,
/// the engine's edits for the anchors and renames.
// sokf-implements: SOKF_VALIDATION_14
// sokf-implements: SOKF_VALIDATION_25
// sokf-implements: SOKF_VALIDATION_26
// sokf-implements: SOKF_VALIDATION_27
// sokf-implements: SOKF_VALIDATION_28
// sokf-implements: SOKF_VALIDATION_30
pub fn rewrite(
    text: &str,
    segments: &Segments,
    mut edits: Vec<Edit>,
    repairs: &[Repair],
    anchors: Option<&AnchorEdit<'_>>,
) -> Result<String> {
    edits.extend(line_edits(text, segments, repairs, anchors));
    Ok(apply(text, &edits)?)
}
// sokf:end rewrite

#[cfg(test)]
mod tests {
    use super::*;

    type IncludeEdit = (usize, usize, String);
    type DefinitionEdit = (Option<usize>, String);

    /// `rewrite_text` builds the edits as the planner does and applies them
    /// in one pass.
    fn rewrite_text(
        text: &str,
        includes: &[IncludeEdit],
        definition: Option<&DefinitionEdit>,
        repairs: &[Repair],
        resource: Option<&ResourceEdit>,
    ) -> String {
        let segments = crate::markdown::segments(text, 1);
        let mut edits: Vec<Edit> = includes
            .iter()
            .map(|(open, close, interior)| include_edit(*open, *close, interior))
            .collect();
        edits.extend(definition.map(|(marker, tail)| definition_edit(text, *marker, tail)));
        edits.extend(resource.map(ResourceEdit::edit));
        rewrite(text, &segments, edits, repairs, None).unwrap()
    }

    #[test]
    fn rewrites_headings_includes_and_the_definition_block() {
        let text = "---\nid: a\n---\n\nTitle\n=====\n\n## Old\n\n<!-- sokf:include x -->\n\nstale\n\n<!-- /sokf:include -->\n\ntail\n\n<!-- sokf:links -->\n\n[sokf:x]: /old.md\n";
        let repairs = vec![
            Repair::heading(5, 6, 1, "Title".into()),
            Repair::heading(8, 8, 2, "New".into()),
        ];
        let includes = vec![(10, 14, "\nfresh\n\n".to_string())];
        let definition = (
            Some(18),
            "<!-- sokf:links -->\n\n[sokf:x]: /new.md\n".to_string(),
        );
        let out = rewrite_text(text, &includes, Some(&definition), &repairs, None);
        assert_eq!(
            out,
            "---\nid: a\n---\n\n# Title\n\n## New\n\n<!-- sokf:include x -->\n\nfresh\n\n<!-- /sokf:include -->\n\ntail\n\n<!-- sokf:links -->\n\n[sokf:x]: /new.md\n"
        );
        // A second pass with the same edits, at the shifted line numbers,
        // changes nothing.
        let again = rewrite_text(
            &out,
            &[(9, 13, "\nfresh\n\n".to_string())],
            Some(&(Some(17), definition.1.clone())),
            &[],
            None,
        );
        assert_eq!(again, out);
    }

    #[test]
    fn adds_and_removes_the_definition_block() {
        let added = rewrite_text(
            "# T\n\nbody\n\n\n",
            &[],
            Some(&(None, "<!-- sokf:links -->\n\n[sokf:a]: /a.md\n".into())),
            &[],
            None,
        );
        assert_eq!(
            added,
            "# T\n\nbody\n\n<!-- sokf:links -->\n\n[sokf:a]: /a.md\n"
        );
        let removed = rewrite_text(
            "# T\n\nbody\n\n<!-- sokf:links -->\n\n[sokf:a]: /a.md\n",
            &[],
            Some(&(Some(5), String::new())),
            &[],
            None,
        );
        assert_eq!(removed, "# T\n\nbody\n");
        assert_eq!(rewrite_text("", &[], None, &[], None), "");
        assert_eq!(
            rewrite_text(
                "",
                &[],
                Some(&(None, "<!-- sokf:links -->\n\n[sokf:a]: /a.md\n".into())),
                &[],
                None
            ),
            "<!-- sokf:links -->\n\n[sokf:a]: /a.md\n"
        );
    }

    // sokf-tests: FORMAT-1_AC-5
    #[test]
    fn the_resource_lines_are_replaced_or_a_line_is_inserted() {
        let stale =
            "---\ntype: Image\nid: flowers\nresource: /old/flowers.png\ntitle: T\n---\n\nbody\n";
        let edit = ResourceEdit {
            lines: Some(4..5),
            after: 3,
            value: "/k/media/flowers.png".into(),
        };
        assert_eq!(
            rewrite_text(stale, &[], None, &[], Some(&edit)),
            "---\ntype: Image\nid: flowers\nresource: /k/media/flowers.png\ntitle: T\n---\n\nbody\n"
        );
        let missing = "---\ntype: Image\nid: flowers\ntitle: T\n---\n\nbody\n";
        let edit = ResourceEdit {
            lines: None,
            after: 3,
            value: "/k/media/flowers.png".into(),
        };
        assert_eq!(
            rewrite_text(missing, &[], None, &[], Some(&edit)),
            "---\ntype: Image\nid: flowers\nresource: /k/media/flowers.png\ntitle: T\n---\n\nbody\n"
        );
        // A value of several lines is replaced whole.
        let listed = "---\ntype: Image\nresource:\n  - /a.png\n  - /b.png\ntitle: T\n---\n";
        let edit = ResourceEdit {
            lines: Some(3..6),
            after: 1,
            value: "/k/media/flowers.png".into(),
        };
        assert_eq!(
            rewrite_text(listed, &[], None, &[], Some(&edit)),
            "---\ntype: Image\nresource: /k/media/flowers.png\ntitle: T\n---\n"
        );
    }

    #[test]
    fn an_empty_interior_is_one_blank_line() {
        let out = rewrite_text(
            "<!-- sokf:include x -->\nold\n<!-- /sokf:include -->\n",
            &[(1, 3, "\n".into())],
            None,
            &[],
            None,
        );
        assert_eq!(out, "<!-- sokf:include x -->\n\n<!-- /sokf:include -->\n");
    }
}
