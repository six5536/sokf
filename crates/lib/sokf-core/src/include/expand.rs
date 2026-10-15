//! This module computes the expected content of every include block and
//! definition block (SOKF_VALIDATION_15). It expands each file after the
//! files that it includes, so each include block holds the expected content of
//! its target, never the target's text on disk. `sokf validate --fix` writes
//! this content into each block, and validation compares each block with it.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::{fence, graph, region};
use crate::{
    Body, Model, RepoPath, Segments,
    markdown::{self, IncludeTarget, Link, sokf_label},
    model::{KeyIndex, Query, keys},
    source,
};

/// A `DefinitionEntry` is one line of a definition block.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionEntry {
    /// `label` holds the label with its `sokf:` prefix.
    pub label: String,
    /// `path` holds the path of the target.
    pub path: RepoPath,
    /// `slug` holds the `#slug` of the target when the fragment identifies a
    /// heading or a key that a heading captures. Otherwise it is `None`.
    pub slug: Option<String>,
}

/// An `Expected` holds what every generated block should hold.
#[derive(Debug, Clone, Default)]
pub struct Expected {
    /// This map holds the interior of each include block, keyed by file and
    /// by the line of its open marker. The value is `None` when the interior
    /// can't be regenerated.
    pub includes: HashMap<(RepoPath, usize), Option<String>>,
    /// This map holds the definition entries of each file. The list is empty
    /// when the file needs no block, and the value is `None` when a label
    /// doesn't resolve.
    pub definitions: HashMap<RepoPath, Option<Vec<DefinitionEntry>>>,
    /// `cycles` lists each include cycle as the files on it.
    pub cycles: Vec<Vec<RepoPath>>,
    /// `on_cycle` holds every file that lies on an include cycle.
    pub on_cycle: HashSet<RepoPath>,
}

impl Expected {
    /// `include` returns the expected interior of the include block that
    /// opens at `line` in `path`, when the interior can be regenerated.
    pub fn include(&self, path: &RepoPath, line: usize) -> Option<&str> {
        self.includes
            .get(&(path.clone(), line))
            .and_then(|i| i.as_deref())
    }
}

/// `interior` returns the text that goes between the markers of an include
/// block that holds `body`. The `body` has no trailing newline.
fn interior(body: &str) -> String {
    if body.is_empty() {
        "\n".to_string()
    } else {
        format!("\n{body}\n\n")
    }
}

/// `render_definitions` returns the text of the definition block for
/// `entries`, or an empty string when there are no entries.
// sokf-implements: SOKF_FORMAT_79
// sokf-implements: SOKF_FORMAT_80
// sokf-implements: SOKF_FORMAT_81
pub fn render_definitions(entries: &[DefinitionEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = String::from("<!-- sokf:links -->\n\n");
    for e in entries {
        out.push_str(&format!("[{}]: {}", e.label, e.path));
        if let Some(slug) = &e.slug {
            out.push('#');
            out.push_str(slug);
        }
        out.push('\n');
    }
    out
}

/// `expected` computes the expected content of every generated block.
// sokf-implements: SOKF_FORMAT_85
pub fn expected(model: &Model, keys: &KeyIndex) -> Expected {
    expected_for(model, keys, None)
}

/// `expected_for` computes the expected content of the generated blocks of
/// the files in `only`, or of every file when `only` is `None`. It expands the
/// include targets of those files too. It reads the body of a summarised
/// document from the disk when a file that depends on it needs the body
/// (CACHE-3_AC-1).
// sokf-implements: SOKF_FORMAT_78
// sokf-implements: SOKF_FORMAT_85
// sokf-implements: SOKF_VALIDATION_15
// sokf-implements: CACHE-3_AC-1
pub fn expected_for(model: &Model, keys: &KeyIndex, only: Option<&BTreeSet<RepoPath>>) -> Expected {
    // The nodes to expand are the wanted files and every file that they
    // include, directly or through another file.
    let (g, needed) = graph::graph_for(model, only);
    let o = graph::order(&g);
    let mut out = Expected {
        cycles: o.cycles.clone(),
        on_cycle: o.cycles.iter().flatten().cloned().collect(),
        ..Expected::default()
    };
    // This map holds the expanded body of each document by path, or `None`
    // when the body can't be computed.
    let mut expansions: HashMap<RepoPath, Option<String>> = HashMap::new();
    let cycle_nodes: Vec<usize> = (0..g.nodes.len())
        .filter(|i| out.on_cycle.contains(&g.nodes[*i]))
        .collect();
    for &node in o.sorted.iter().chain(cycle_nodes.iter()) {
        if needed.as_ref().is_some_and(|n| !n.contains(&node)) {
            continue;
        }
        let path = &g.nodes[node];
        model.watch(path);
        let (knowledge, segments, body, body_text, body_line, is_document) =
            match model.by_path(path) {
                Some(crate::Located::Document(k, doc)) => parts_of(k, doc),
                Some(crate::Located::Schema(k, schema)) => parts_of(k, &schema.doc),
                Some(crate::Located::Additional(file)) => (
                    graph::knowledge_of_additional(model, file),
                    &file.segments,
                    &file.body,
                    file.source.text.as_str(),
                    1,
                    false,
                ),
                _ => {
                    model.unwatch();
                    continue;
                }
            };
        let mut all_computable = true;
        for block in &segments.regions {
            let expected = match &IncludeTarget::parse(&block.target) {
                IncludeTarget::Document {
                    store,
                    id,
                    fragment,
                } => {
                    let target = knowledge.and_then(|k| model.document_in(k, store.as_deref(), id));
                    match target {
                        None => None,
                        Some((target_k, target))
                            if graph::includes_body(model, &target.path, fragment.as_deref()) =>
                        {
                            Some(target)
                                .filter(|t| !out.on_cycle.contains(&t.path))
                                .and_then(|t| {
                                    model.note(Query::Body(t.path.clone()));
                                    expansions.get(&t.path).cloned().flatten()
                                })
                                .map(|body| match knowledge {
                                    Some(k) if k != target_k => {
                                        requalify(model, keys, target_k, k, &body)
                                    }
                                    _ => body,
                                })
                                .map(|body| interior(&body))
                        }
                        // Otherwise the block holds the content of the
                        // sidecar's asset, whole or one region of it. An
                        // include block with a path holds the content of a
                        // file in the same way (FORMAT-2_AC-5). A plain
                        // document has no asset, so its block has no
                        // expected interior.
                        // sokf-implements: FORMAT-2_AC-5
                        Some((_, target)) => model
                            .asset_of(&target.path)
                            .and_then(|(_, a)| asset_interior(model, &a.path, fragment.as_deref())),
                    }
                }
                IncludeTarget::File { path, region } => {
                    file_interior(model, knowledge, path, region.as_deref())
                }
            };
            if expected.is_none() {
                all_computable = false;
            }
            out.includes
                .insert((path.clone(), block.open_line), expected);
        }
        if is_document {
            let expansion = if all_computable && !out.on_cycle.contains(path) {
                Some(expand(body_text, body_line, segments, body, path, &out))
            } else {
                None
            };
            expansions.insert(path.clone(), expansion);
        }
        model.unwatch();
    }

    // Each definition block lists every resolved `sokf:` label in the body
    // and in each include interior.
    let documents = model
        .documents_and_schemas()
        .map(|(k, d)| (Some(k), &d.path, &d.body.links, &d.segments));
    let additional = model.additional.iter().map(|f| {
        (
            graph::knowledge_of_additional(model, f),
            &f.path,
            &f.body.links,
            &f.segments,
        )
    });
    for (k, path, links, segments) in documents.chain(additional) {
        let Some(k) = k else { continue };
        if only.is_some_and(|o| !o.contains(path)) {
            continue;
        }
        model.watch(path);
        let entries = definition_entries(model, keys, k, path, links, segments, &out);
        model.unwatch();
        out.definitions.insert(path.clone(), entries);
    }
    out
}

/// `Parts` holds what the expansion of one file reads: its knowledge, its
/// segments, its body, the text of its body, the line where its body starts,
/// and whether the file is a document.
type Parts<'a> = (Option<usize>, &'a Segments, &'a Body, &'a str, usize, bool);

/// `parts_of` returns the parts of a document of the knowledge `k`.
fn parts_of(k: usize, doc: &crate::Document) -> Parts<'_> {
    (
        Some(k),
        &doc.segments,
        &doc.body,
        doc.body_text(),
        doc.body_line,
        true,
    )
}

/// `file_interior` returns the interior of an include block whose target is
/// a path. It returns an interior only when both of these hold:
///
/// - The path resolves to a repository file that is neither a document of
///   any store nor an additional file of the including file's `knowledge`
///   (SOKF_FORMAT_88, SOKF_FORMAT_89).
/// - The region can be extracted from that file.
///
/// Otherwise it returns `None`. A path that points to an asset still gets an
/// interior. Validation reports a finding for that path, but the path and the
/// id of the asset's sidecar document refer to the same content. So in one
/// pass `--fix` replaces the path with the id and writes this interior.
fn file_interior(
    model: &Model,
    knowledge: Option<usize>,
    path: &str,
    region: Option<&str>,
) -> Option<String> {
    let rp = RepoPath::parse(path)?;
    if model.id_at(&rp).is_some() || model.is_materialized_in(knowledge, &rp) {
        return None;
    }
    asset_interior(model, &rp, region)
}

/// `asset_interior` returns the interior for the file at `path`, whole or one
/// region of it, in a fenced code block.
fn asset_interior(model: &Model, path: &RepoPath, region: Option<&str>) -> Option<String> {
    if !model.exists(path) {
        return None;
    }
    model.note(Query::Body(path.clone()));
    let text = source::load_text(&model.tree.root, path).ok()?;
    let extraction = region::extract(&text, region).ok()??;
    Some(interior(&fence::fenced(&extraction, path)))
}

/// `expand` returns the expanded body of a document. It leaves out the
/// definition block, the generated anchors, and the include markers. It
/// replaces each include block with its expected interior. When the expected
/// interior can't be computed, it keeps the interior on disk instead. Then it
/// trims the blank lines at both ends. An anchor belongs to the document that
/// captures its key, so a copy carries no anchors (SOKF_FORMAT_83).
fn expand(
    body_text: &str,
    body_line: usize,
    segments: &Segments,
    body: &Body,
    path: &RepoPath,
    out: &Expected,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut skip_until: Option<usize> = None;
    let mut after_anchors = false;
    for (i, line) in body_text.lines().enumerate() {
        let file_line = body_line + i;
        if let Some(until) = skip_until {
            if file_line <= until {
                continue;
            }
            skip_until = None;
        }
        // The loop drops a line that holds only anchors, and the blank line
        // after it.
        if body.anchors.iter().any(|a| a.line == file_line && a.alone) {
            after_anchors = true;
            continue;
        }
        if std::mem::take(&mut after_anchors) && line.trim().is_empty() {
            continue;
        }
        let line = &*body.clean_line(file_line, line);
        if segments
            .definition
            .as_ref()
            .is_some_and(|d| file_line >= d.line)
        {
            break;
        }
        if let Some(block) = segments.regions.iter().find(|b| b.open_line == file_line) {
            let content = out.include(path, block.open_line).unwrap_or(&block.content);
            lines.extend(content.lines().map(str::to_string));
            skip_until = Some(block.close_line);
            continue;
        }
        lines.push(line.to_string());
    }
    let start = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(0, |i| i + 1);
    if start >= end {
        return String::new();
    }
    lines[start..end].join("\n")
}

/// `definition_entries` returns the definition entries of one file: the
/// labels that its own body cites, and the labels inside each include
/// interior. It lists each entry as `--fix` writes it in the same pass: a
/// fragment written as the text of a key becomes the key's slug, a label
/// takes its canonical store prefix, and a path link to a document or an
/// asset of any store becomes its canonical label (SOKF_VALIDATION_24,
/// SOKF_VALIDATION_32). It returns `None` when a label doesn't resolve.
// sokf-implements: SOKF_FORMAT_80
// sokf-implements: SOKF_FORMAT_81
// sokf-implements: FORMAT-2_AC-2
// sokf-implements: FORMAT-2_AC-3
fn definition_entries(
    model: &Model,
    keys: &KeyIndex,
    knowledge: usize,
    path: &RepoPath,
    links: &[Link],
    segments: &Segments,
    out: &Expected,
) -> Option<Vec<DefinitionEntry>> {
    let mut labels: Vec<String> = Vec::new();
    for link in links {
        if let Some(label) = sokf_label(link) {
            labels.push(label);
        } else if let Some((label, true)) = path_link_label(model, keys, knowledge, path, link) {
            labels.push(label);
        }
    }
    for block in &segments.regions {
        let content = out.include(path, block.open_line).unwrap_or(&block.content);
        let inner_segments = Segments::default();
        let body = markdown::parse(content, 1, &inner_segments);
        labels.extend(body.links.iter().filter_map(sokf_label));
    }
    labels.sort();
    labels.dedup();
    let mut entries = Vec::new();
    for label in labels {
        let parts = markdown::label_parts(&label)?;
        let (resolved, slug) = keys::resolve_slugged(model, keys, knowledge, parts);
        let resolved = resolved.ok()?;
        let parts = parts.with_fragment(slug.as_deref().or(parts.fragment));
        let label = model
            .canonical_label(knowledge, resolved.knowledge, parts.id, parts.fragment)
            .unwrap_or_else(|| label.clone());
        // A bare label refers to the content, which for a sidecar is its
        // asset (FORMAT-2_AC-2). A label with any fragment, even an empty
        // one, refers to the file itself.
        let path = if parts.fragment.is_none() {
            model.content_path(&resolved.path)
        } else {
            resolved.path
        };
        entries.push(DefinitionEntry {
            label,
            path,
            slug: resolved.slug,
        });
    }
    entries.sort();
    entries.dedup();
    Some(entries)
}

/// `path_link_label` returns the canonical label of a path link in `path`,
/// when the link points to a document or an asset of any store. When the
/// fragment is the text of a key, the label uses the key's slug instead. It
/// also returns whether `--fix` writes the label. `--fix` writes the label of
/// an inline link when the label resolves (SOKF_VALIDATION_24). It returns `None` for a
/// link to any other file, and for an image link whose target is not an
/// asset.
pub fn path_link_label(
    model: &Model,
    keys: &KeyIndex,
    knowledge: usize,
    path: &RepoPath,
    link: &Link,
) -> Option<(String, bool)> {
    let (target, fragment) = match &link.target {
        markdown::LinkTarget::Repo { path, fragment } => (path.clone(), fragment.as_deref()),
        markdown::LinkTarget::Relative { target, fragment } => {
            (path.resolve_relative(target)?, fragment.as_deref())
        }
        _ => return None,
    };
    // An image refers to an asset, so a link to a document written as an
    // image gets no label (SOKF_FORMAT_67).
    if link.is_image && model.asset_at(&target).is_none() {
        return None;
    }
    let label = model.path_label(knowledge, &target, fragment)?;
    let parts = markdown::label_parts(&label)?;
    let (label, resolves) = match keys::resolve_slugged(model, keys, knowledge, parts) {
        (Ok(_), None) => (label, true),
        (Ok(r), Some(slug)) => {
            match model.canonical_label(knowledge, r.knowledge, parts.id, Some(&slug)) {
                Some(slugged) => (slugged, true),
                None => (label, false),
            }
        }
        (Err(_), _) => (label, false),
    };
    Some((label, resolves && link.inline.is_some()))
}

/// `requalify` returns `body`, the expanded body of a document of the store
/// `from`, with each `sokf:` label rewritten the way a document of the store
/// `to` would write it. The rewrite makes an included copy mean what its
/// source means (format specification §9.3).
fn requalify(model: &Model, keys: &KeyIndex, from: usize, to: usize, body: &str) -> String {
    let parsed = markdown::parse(body, 1, &Segments::default());
    // The loop respells each label only on the lines of its link and never
    // in a code span, so an example of a label written as code stays as
    // written.
    let mut renames: Vec<((usize, usize), String, String)> = Vec::new();
    for link in &parsed.links {
        let (Some(label), Some(parts)) = (sokf_label(link), keys::Label::of(&link.target)) else {
            continue;
        };
        let Ok(resolved) = keys::resolve(model, keys, from, parts) else {
            continue;
        };
        if let Some(new) = model.canonical_label(to, resolved.knowledge, parts.id, parts.fragment)
            && new != label
        {
            renames.push((link.lines(), format!("[{label}]"), format!("[{new}]")));
        }
    }
    renames.sort();
    renames.dedup();
    let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
    for ((first, last), old, new) in renames {
        for line in first..=last {
            let Some(text) = lines.get_mut(line - 1) else {
                continue;
            };
            let at: Vec<usize> = text
                .match_indices(old.as_str())
                .map(|(pos, _)| pos)
                .filter(|&pos| !parsed.in_code_span(line, pos))
                .collect();
            for pos in at.into_iter().rev() {
                text.replace_range(pos..pos + old.len(), &new);
            }
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn interiors_and_definition_blocks_render() {
        assert_eq!(interior(""), "\n");
        assert_eq!(interior("a\nb"), "\na\nb\n\n");
        assert_eq!(render_definitions(&[]), "");
        let entries = vec![
            DefinitionEntry {
                label: "sokf:#K".into(),
                path: RepoPath::parse("/k/a.md").unwrap(),
                slug: None,
            },
            DefinitionEntry {
                label: "sokf:b#h".into(),
                path: RepoPath::parse("/k/b.md").unwrap(),
                slug: Some("h".into()),
            },
        ];
        assert_eq!(
            render_definitions(&entries),
            "<!-- sokf:links -->\n\n[sokf:#K]: /k/a.md\n[sokf:b#h]: /k/b.md#h\n"
        );
    }

    #[test]
    fn expand_removes_markers_and_the_definition_block() {
        let body = "\n# T\n\n<!-- sokf:include x -->\n\nold\n\n<!-- /sokf:include -->\n\ntail\n\n<!-- sokf:links -->\n\n[sokf:x]: /k/x.md\n";
        let segments = markdown::segments(body, 5);
        let path = RepoPath::parse("/k/a.md").unwrap();
        let mut out = Expected::default();
        out.includes
            .insert((path.clone(), 8), Some("\nnew\n\n".into()));
        assert_eq!(
            expand(body, 5, &segments, &Body::default(), &path, &out),
            "# T\n\n\nnew\n\n\ntail"
        );
        let none = Expected::default();
        assert_eq!(
            expand(body, 5, &segments, &Body::default(), &path, &none),
            "# T\n\n\nold\n\n\ntail"
        );
        assert_eq!(
            expand(
                "\n\n",
                1,
                &Segments::default(),
                &Body::default(),
                &path,
                &none
            ),
            ""
        );
    }

    // sokf-tests: FORMAT_P-9
    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn the_definition_block_lists_each_label_once_in_ascending_order(labels in prop::collection::vec("[a-z]{1,4}(#[a-z]{1,3})?", 0..6)) {
            let entries: Vec<DefinitionEntry> = {
                let mut v: Vec<DefinitionEntry> = labels.iter().map(|l| DefinitionEntry {
                    label: format!("sokf:{l}"),
                    path: RepoPath::parse("/k/t.md").unwrap(),
                    slug: l.split_once('#').map(|(_, s)| s.to_string()),
                }).collect();
                v.sort();
                v.dedup();
                v
            };
            let text = render_definitions(&entries);
            if entries.is_empty() {
                prop_assert_eq!(text, "");
            } else {
                let segments = markdown::segments(&text, 1);
                let def = segments.definition.unwrap();
                prop_assert!(!def.trailing);
                let got: Vec<String> = def.entries.iter().map(|(l, _)| l.clone()).collect();
                let want: Vec<String> = entries.iter().map(|e| e.label.clone()).collect();
                prop_assert_eq!(got.clone(), want);
                let mut sorted = got.clone();
                sorted.sort();
                sorted.dedup();
                prop_assert_eq!(got, sorted);
                for ((_, dest), e) in def.entries.iter().zip(&entries) {
                    let want = match &e.slug { Some(s) => format!("/k/t.md#{s}"), None => "/k/t.md".into() };
                    prop_assert_eq!(dest, &want);
                }
            }
        }
    }

    // sokf-tests: FORMAT_P-8
    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn an_expanded_body_has_no_marker_and_is_a_fixed_point(inner in "[a-z ]{0,20}", outer in "[a-z ]{0,20}") {
            let body = format!("{outer}\n\n<!-- sokf:include x -->\n\n{inner}\n\n<!-- /sokf:include -->\n\n<!-- sokf:links -->\n\n[sokf:x]: /k/x.md\n");
            let segments = markdown::segments(&body, 1);
            let path = RepoPath::parse("/k/a.md").unwrap();
            let mut out = Expected::default();
            out.includes.insert((path.clone(), 3), Some(interior(inner.trim())));
            let once = expand(&body, 1, &segments, &Body::default(), &path, &out);
            prop_assert!(!once.contains("sokf:include") && !once.contains("sokf:links"));
            let again = expand(&once, 1, &markdown::segments(&once, 1), &Body::default(), &path, &Expected::default());
            prop_assert_eq!(again, once);
        }
    }
}
