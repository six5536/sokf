//! This module computes the expected content of every include block and
//! definition block (`SOKF_VALIDATION_15`). It expands each file after the
//! files that it includes, so each include block holds the expected content of
//! its target, never the target's text on disk. `sokf validate --fix` writes
//! this content into each block, and validation compares each block with it.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Write as _,
};

use super::{fence, graph, region};
use crate::{
    Body, Model, RepoPath, Segments,
    compare::{Layout, Pair, Source},
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
    /// This map holds the source of each include block that holds a file
    /// in a fence, keyed as `includes` is. An include block of a document's
    /// body has no entry.
    pub fences: HashMap<(RepoPath, usize), Fenced>,
    /// This map holds the path from the root, as same-same reads it, of the
    /// document whose body each include block of a body holds, keyed as
    /// `includes` is.
    pub bodies: HashMap<(RepoPath, usize), String>,
    /// This map holds the definition entries of each file. The list is empty
    /// when the file needs no block, and the value is `None` when a label
    /// doesn't resolve.
    pub definitions: HashMap<RepoPath, Option<Vec<DefinitionEntry>>>,
    /// `cycles` lists each include cycle as the files on it.
    pub cycles: Vec<Vec<RepoPath>>,
    /// `on_cycle` holds every file that lies on an include cycle.
    pub on_cycle: HashSet<RepoPath>,
    /// `layout` records which blocks that differ from their expected text
    /// same-same finds the same (CLI-4_AC-45).
    pub layout: Layout,
}

/// A `Fenced` is the source of an include block that holds a file, whole or
/// one region of it, in a fence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fenced {
    /// The included file.
    pub path: RepoPath,
    /// Whether the block includes a region of the file.
    pub region: bool,
    /// The lines of the file that the code of the fence holds.
    pub lines: Vec<u32>,
    /// Those lines as one run of whole lines, where they are one.
    pub range: Option<std::ops::Range<u32>>,
}

impl Expected {
    /// `include` returns the expected interior of the include block that
    /// opens at `line` in `path`, when the interior can be regenerated.
    pub fn include(&self, path: &RepoPath, line: usize) -> Option<&str> {
        self.includes
            .get(&(path.clone(), line))
            .and_then(|i| i.as_deref())
    }

    /// `include_pair` returns the pair of texts that same-same compares
    /// for the include block that opens at `line` in `path` and holds
    /// `content`, when the block differs from its expected interior. An
    /// include of a file compares the code inside the fence under the path
    /// of its source, a region of it as a part of a file, and an
    /// include of a body compares the whole interior under the path of the
    /// included document (CLI-4_AC-113). It returns `None` when the
    /// block has no expected interior, holds it, or holds a fence of another
    /// form.
    // sokf-implements: CLI-4_AC-47
    // sokf-implements: CLI-4_AC-113
    pub fn include_pair(&self, path: &RepoPath, line: usize, content: &str) -> Option<Pair> {
        let expected = self.include(path, line)?;
        if expected == content {
            return None;
        }
        let key = (path.clone(), line);
        let Some(fenced) = self.fences.get(&key) else {
            return Some(Pair {
                path: path.clone(),
                name: self.bodies.get(&key)?.clone(),
                fragment: false,
                expected: expected.into(),
                actual: content.into(),
                source: None,
            });
        };
        let (open, close) = fence_ends(expected)?;
        let code = |text: &str| -> Option<String> {
            let inner = text.strip_prefix(open)?.strip_suffix(close)?;
            Some(inner.to_string())
        };
        Some(Pair {
            path: path.clone(),
            name: crate::compare::name_of(&fenced.path),
            fragment: fenced.region,
            expected: code(expected)?,
            actual: code(content)?,
            source: Some(Source {
                path: fenced.path.clone(),
                range: fenced.range.clone().filter(|_| fenced.region),
                lines: fenced.lines.clone(),
            }),
        })
    }

    /// `include_current` returns whether the include block that opens at
    /// `line` in `path` and holds `content` is current: whether it holds its
    /// expected interior, or same-same finds the two the same. It returns
    /// `None` when the block has no expected interior.
    // sokf-implements: CLI-4_AC-44
    // sokf-implements: CLI-4_AC-45
    pub fn include_current(&self, path: &RepoPath, line: usize, content: &str) -> Option<bool> {
        let expected = self.include(path, line)?;
        Some(
            expected == content
                || self
                    .include_pair(path, line, content)
                    .is_some_and(|p| self.layout.same(&p)),
        )
    }

    /// `include_stale` returns the warning that the formatter failed on the
    /// texts of the stale include block that opens at `line` in `path`,
    /// holds `content`, and whose target is written as `written`, when it
    /// did, and the message of its finding.
    pub fn include_stale(
        &self,
        path: &RepoPath,
        line: usize,
        content: &str,
        written: &str,
    ) -> (Option<crate::Finding>, String) {
        let Some(p) = self.include_pair(path, line, content) else {
            return (None, crate::compare::stale_message(written, None, None));
        };
        (
            self.layout.warning(&p),
            crate::compare::stale_message(
                written,
                p.source.as_ref().map(|s| &s.path),
                self.layout.line(&p),
            ),
        )
    }
}

/// `fence_ends` returns the line that opens the fence of an include
/// interior, after the interior's first blank line, and the text that
/// closes it, from the fence to the interior's end.
fn fence_ends(interior: &str) -> Option<(&str, &str)> {
    let rest = interior.strip_prefix('\n')?;
    let ticks = rest.len() - rest.trim_start_matches('`').len();
    let open_len = 1 + rest.find('\n')? + 1;
    let close_len = ticks + 2;
    (ticks >= 3 && open_len + close_len <= interior.len()).then(|| {
        (
            &interior[..open_len],
            &interior[interior.len() - close_len..],
        )
    })
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
        let _ = write!(out, "[{}]: {}", e.label, e.path);
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
// sokf-implements: SOKF_FORMAT_85
// sokf-implements: SOKF_VALIDATION_15
// sokf-implements: CACHE-3_AC-1
pub fn expected_for(model: &Model, keys: &KeyIndex, only: Option<&BTreeSet<RepoPath>>) -> Expected {
    // The nodes to expand are the wanted files and every file that they
    // include, directly or through another file.
    let (g, needed) = graph::graph_for(model, only);
    let o = graph::order(&g);
    let mut expander = Expander {
        model,
        keys,
        out: Expected {
            cycles: o.cycles.clone(),
            on_cycle: o.cycles.iter().flatten().cloned().collect(),
            ..Expected::default()
        },
        expansions: HashMap::new(),
    };
    let cycle_nodes: Vec<usize> = (0..g.nodes.len())
        .filter(|i| expander.out.on_cycle.contains(&g.nodes[*i]))
        .collect();
    for &node in o.sorted.iter().chain(cycle_nodes.iter()) {
        if needed.as_ref().is_some_and(|n| !n.contains(&node)) {
            continue;
        }
        expander.expand_file(&g.nodes[node]);
    }
    expander.add_definitions(only);
    expander.out
}

/// An `Expander` holds what `expected_for` builds as it expands the files,
/// each after the files that it includes.
struct Expander<'a> {
    /// The model.
    model: &'a Model,
    /// The captured keys.
    keys: &'a KeyIndex,
    /// The expected content so far.
    out: Expected,
    /// This map holds the expanded body of each document by path, or `None`
    /// when the body can't be computed.
    expansions: HashMap<RepoPath, Option<String>>,
}

impl Expander<'_> {
    /// `expand_file` computes the expected interior of each include block of
    /// the file at `path`, and the file's expanded body when the file is a
    /// document.
    fn expand_file(&mut self, path: &RepoPath) {
        let model = self.model;
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
                    return;
                }
            };
        let mut all_computable = true;
        for block in &segments.regions {
            let expected = self.block_interior(knowledge, &block.target);
            if expected.is_none() {
                all_computable = false;
            }
            let key = (path.clone(), block.open_line);
            let expected = expected.map(|(interior, kind)| {
                match kind {
                    Kind::File(fenced) => self.out.fences.insert(key.clone(), fenced).map(drop),
                    Kind::Body(name) => self.out.bodies.insert(key.clone(), name).map(drop),
                };
                interior
            });
            self.out.includes.insert(key, expected);
        }
        if is_document {
            let expansion = if all_computable && !self.out.on_cycle.contains(path) {
                Some(expand(
                    body_text, body_line, segments, body, path, &self.out,
                ))
            } else {
                None
            };
            self.expansions.insert(path.clone(), expansion);
        }
        model.unwatch();
    }

    /// `block_interior` returns the expected interior of an include block
    /// whose target is `target`, in a file of the knowledge `knowledge`, or
    /// `None` when the interior can't be computed. With the interior, it
    /// returns the source of the file that a fence holds, or the file name
    /// of the document whose body it holds.
    fn block_interior(&self, knowledge: Option<usize>, target: &str) -> Option<(String, Kind)> {
        let model = self.model;
        match &IncludeTarget::parse(target) {
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
                        let name = crate::compare::name_of(&target.path);
                        Some(target)
                            .filter(|t| !self.out.on_cycle.contains(&t.path))
                            .and_then(|t| {
                                model.note(Query::Body(t.path.clone()));
                                self.expansions.get(&t.path).cloned().flatten()
                            })
                            .map(|body| match knowledge {
                                Some(k) if k != target_k => {
                                    requalify(model, self.keys, target_k, k, &body)
                                }
                                _ => body,
                            })
                            .map(|body| (interior(&body), Kind::Body(name)))
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
        }
    }

    /// `add_definitions` computes the definition entries of each document,
    /// schema, and additional file in `only`, or of every file when `only`
    /// is `None`.
    // sokf-implements: SOKF_FORMAT_78
    fn add_definitions(&mut self, only: Option<&BTreeSet<RepoPath>>) {
        let model = self.model;
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
            let entries = definition_entries(model, self.keys, k, path, links, segments, &self.out);
            model.unwatch();
            self.out.definitions.insert(path.clone(), entries);
        }
    }
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
///   (`SOKF_FORMAT_88`, `SOKF_FORMAT_89`).
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
) -> Option<(String, Kind)> {
    let rp = RepoPath::parse(path)?;
    if model.id_at(&rp).is_some() || model.is_materialized_in(knowledge, &rp) {
        return None;
    }
    asset_interior(model, &rp, region)
}

/// `asset_interior` returns the interior for the file at `path`, whole or one
/// region of it, in a fenced code block, with where the code lies in the
/// file.
fn asset_interior(model: &Model, path: &RepoPath, region: Option<&str>) -> Option<(String, Kind)> {
    if !model.exists(path) {
        return None;
    }
    model.note(Query::Body(path.clone()));
    let text = source::load_text(&model.tree.root, path).ok()?;
    let extraction = region::extract_lines(&text, region).ok()??;
    let fenced = Fenced {
        path: path.clone(),
        region: region.is_some(),
        range: extraction.range(),
        lines: extraction.lines,
    };
    Some((
        interior(&fence::fenced(&extraction.text, path)),
        Kind::File(fenced),
    ))
}

/// A `Kind` says what an include block holds: a file in a fence, or the
/// body of the document of a path from the root.
enum Kind {
    File(Fenced),
    Body(String),
}

/// `expand` returns the expanded body of a document. It leaves out the
/// definition block, the generated anchors, and the include markers. It
/// replaces each include block with its expected interior. When the expected
/// interior can't be computed, it keeps the interior on disk instead. Then it
/// trims the blank lines at both ends. An anchor belongs to the document that
/// captures its key, so a copy carries no anchors (`SOKF_FORMAT_83`).
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
/// asset of any store becomes its canonical label (`SOKF_VALIDATION_24`,
/// `SOKF_VALIDATION_32`). It returns `None` when a label doesn't resolve.
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
/// an inline link when the label resolves (`SOKF_VALIDATION_24`). It returns `None` for a
/// link to any other file, for a link that ends with `/`, which names only a
/// directory (`SOKF_FORMAT_157`), and for an image link whose target is not an
/// asset.
pub fn path_link_label(
    model: &Model,
    keys: &KeyIndex,
    knowledge: usize,
    path: &RepoPath,
    link: &Link,
) -> Option<(String, bool)> {
    let (target, fragment) = match &link.target {
        markdown::LinkTarget::Repo {
            path,
            fragment,
            dir: false,
        } => (path.clone(), fragment.as_deref()),
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
mod tests;
