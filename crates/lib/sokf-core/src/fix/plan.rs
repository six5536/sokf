//! Within `Scope`, `plan` composes every rewrite of a file into the file's
//! new text, lists every move, and lists every index to write. A refused
//! move becomes a finding.

use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
};

use super::{
    AnchorEdit, Edit, Rename,
    edits::{ResourceEdit, definition_edit, include_edit, rewrite},
};
use crate::{
    Compiled, Expected, Finding, LineEnding, Located, Locator, Model, RepoPath, Result, Scope,
    check::{self, Ctx, DocView, Placement, Repair},
    grammar::{SokfSchema, States},
    graph::{Target, resolve_to},
    include::{self, DefinitionEntry, render_definitions},
    index,
    markdown::{IncludeTarget, LinkTarget, sokf_label},
    model::{
        KeyIndex,
        keys::{self, Label},
    },
};

/// `Rewrite` is the new text of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    /// `path` is the path of the file after any move.
    pub path: RepoPath,
    /// `text` is the new text, with LF line endings.
    pub text: String,
    /// `eol` is the line ending to write the file with.
    pub eol: LineEnding,
}

/// `Move` is the move of a document from one path to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    /// `from` is the old path.
    pub from: RepoPath,
    /// `to` is the new path.
    pub to: RepoPath,
}

// sokf:begin fix-plan
/// `FixPlan` holds the changes that `--fix` makes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixPlan {
    /// `rewrites` lists the files to rewrite.
    pub rewrites: Vec<Rewrite>,
    /// `creates` lists the indexes to create, with LF line endings
    /// (SOKF_VALIDATION_36).
    pub creates: Vec<Rewrite>,
    /// `moves` lists the documents to move.
    pub moves: Vec<Move>,
    /// `refused` holds the findings for the moves that the plan refuses
    /// because their target exists.
    pub refused: Vec<Finding>,
}
// sokf:end fix-plan

/// `plan` computes the repairs that `--fix` makes within `scope`. When `only`
/// is given, it plans repairs only for the documents in `only`. The reason is
/// that an incremental run loads more than the documents to repair: it also
/// loads the documents that the plan reads (CACHE-3_AC-1). `plan` computes the expected indexes from the
/// planned moves, so they describe the tree that the plan leaves.
// sokf-implements: SOKF_VALIDATION_14
// sokf-implements: SOKF_VALIDATION_17
// sokf-implements: SOKF_VALIDATION_54
// sokf-implements: SOKF_VALIDATION_18
// sokf-implements: SOKF_VALIDATION_19
// sokf-implements: SOKF_VALIDATION_20
// sokf-implements: SOKF_VALIDATION_21
// sokf-implements: SOKF_VALIDATION_22
// sokf-implements: SOKF_VALIDATION_24
// sokf-implements: SOKF_VALIDATION_32
// sokf-implements: SOKF_VALIDATION_33
// sokf-implements: CLI-4_AC-5
// sokf-implements: CLI-4_AC-23
// sokf-implements: CLI-4_AC-34
pub fn plan(
    model: &Model,
    compiled: &Compiled,
    expected: &Expected,
    keys: &KeyIndex,
    scope: &Scope,
    only: Option<&std::collections::BTreeSet<RepoPath>>,
) -> Result<FixPlan> {
    let mut out = FixPlan::default();
    let mut remap: HashMap<RepoPath, RepoPath> = HashMap::new();
    let mut targets: BTreeSet<RepoPath> = BTreeSet::new();
    let planned = |path: &RepoPath| scope.contains(path) && only.is_none_or(|o| o.contains(path));

    // Plan the moves first, because they change the paths that definition
    // blocks link to.
    for (k, doc) in model.documents() {
        if !planned(&doc.path) {
            continue;
        }
        let Some((_, schema)) = compiled.for_doc(model, k, doc) else {
            continue;
        };
        let states = check::states_of(doc.frontmatter.as_ref(), schema);
        let Some(derived) = check::derived_dir(schema, &states) else {
            continue;
        };
        if check::placed(&doc.path, schema, &derived) {
            continue;
        }
        let Some(target) = derived.join(doc.path.file_name()) else {
            continue;
        };
        // A sidecar moves with its asset to the same directory, and both
        // keep their filenames. When either target exists, neither moves
        // (SOKF_VALIDATION_18, SOKF_VALIDATION_20).
        let mut pair: Vec<(RepoPath, RepoPath)> = vec![(doc.path.clone(), target)];
        if let Some((_, asset)) = model.asset_of(&doc.path)
            && let Some(asset_target) = derived.join(asset.path.file_name())
        {
            pair.push((asset.path.clone(), asset_target));
        }
        // sokf-implements: CLI-4_AC-33
        let occupied: Vec<&RepoPath> = pair
            .iter()
            .filter(|(_, to)| model.exists(to) || targets.contains(to))
            .map(|(_, to)| to)
            .collect();
        if !occupied.is_empty() {
            let names: Vec<String> = occupied.iter().map(ToString::to_string).collect();
            let what = if pair.len() > 1 {
                "the sidecar and its asset"
            } else {
                "the document"
            };
            out.refused.push(Finding::error(
                doc.path.clone(),
                None,
                "SOKF_VALIDATION_21",
                format!(
                    "cannot move {what} to {}: a file is already there",
                    names.join(", ")
                ),
            ));
            continue;
        }
        // sokf-implements: CLI-4_AC-24
        for (from, to) in pair {
            targets.insert(to.clone());
            remap.insert(from.clone(), to.clone());
            out.moves.push(Move { from, to });
        }
    }

    let ctx = |knowledge: Option<usize>| Ctx::new(model, compiled, expected, keys, knowledge);
    for (k, doc) in model.documents() {
        if only.is_some_and(|o| !o.contains(&doc.path)) {
            continue;
        }
        let schema = compiled.for_doc(model, k, doc).map(|(_, c)| c);
        let states = schema
            .map(|c| check::states_of(doc.frontmatter.as_ref(), c))
            .unwrap_or_default();
        let view = check::doc_view(doc);
        plan_file(
            &mut out,
            view,
            schema,
            &states,
            &ctx(Some(k)),
            doc.source.eol,
            scope,
            &remap,
        )?;
    }
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        for schema in &knowledge.schemas {
            // A schema that the run didn't read is a placeholder from the
            // cache store, with no text, so the plan never rewrites it.
            if !schema.loaded || only.is_some_and(|o| !o.contains(&schema.doc.path)) {
                continue;
            }
            let view = check::doc_view(&schema.doc);
            plan_file(
                &mut out,
                view,
                Some(&compiled.schema_document),
                &States::default(),
                &ctx(Some(k)),
                schema.doc.source.eol,
                scope,
                &remap,
            )?;
        }
    }
    for file in &model.additional {
        let schema = file.schema.and_then(|r| compiled.schema(r));
        let view = check::additional_view(file);
        let k = include::knowledge_of_additional(model, file);
        plan_file(
            &mut out,
            view,
            schema,
            &States::default(),
            &ctx(k),
            file.source.eol,
            scope,
            &remap,
        )?;
    }
    plan_indexes(&mut out, model, scope, only);
    out.rewrites.sort_by(|a, b| a.path.cmp(&b.path));
    out.creates.sort_by(|a, b| a.path.cmp(&b.path));
    out.moves.sort_by(|a, b| a.from.cmp(&b.from));
    Ok(out)
}

/// `reference_renames` returns each reference that `--fix` respells, as
/// written and as rewritten (SOKF_VALIDATION_24, SOKF_VALIDATION_32):
///
/// - `--fix` respells a fragment that refers to a key or a heading by its
///   text with its slug: `[sokf:#User]` to `[sokf:#user]`, `](#User)` to
///   `](#user)`, and `]: #User` to `]: #user`.
/// - `--fix` respells a `sokf:` label with its canonical store prefix.
/// - `--fix` respells a path link to a document or an asset of any store as
///   its canonical label: `](/k/a.md#x)` to `][sokf:a#x]`.
/// - `--fix` respells the `to` of a `links` entry as its canonical id.
/// - `--fix` respells an include marker with its canonical target, which
///   identifies an asset by its sidecar's id rather than by its path.
///
/// Only the spelling changes: each rewritten reference leads to the same
/// target as before.
fn reference_renames(view: DocView<'_>, ctx: &Ctx<'_>) -> Vec<Rename> {
    let mut out = Vec::new();
    for link in &view.body.links {
        match &link.target {
            LinkTarget::Host(_) => {
                let (Some(k), Some(written), Some(parts)) =
                    (ctx.knowledge, sokf_label(link), Label::of(&link.target))
                else {
                    continue;
                };
                let (Ok(resolved), slug) = keys::resolve_slugged(ctx.model, ctx.keys, k, parts)
                else {
                    continue;
                };
                let fragment = slug.as_deref().or(parts.fragment);
                if let Some(canonical) =
                    ctx.model
                        .canonical_label(k, resolved.knowledge, parts.id, fragment)
                    && canonical != written
                {
                    out.push((
                        Some(link.lines()),
                        format!("[{written}]"),
                        format!("[{canonical}]"),
                    ));
                }
            }
            LinkTarget::Repo { .. } | LinkTarget::Relative { .. } => {
                let Some((label, true)) = ctx.knowledge.and_then(|k| {
                    include::path_link_label(ctx.model, ctx.keys, k, view.path, link)
                }) else {
                    continue;
                };
                let Some(written) = &link.inline else {
                    continue;
                };
                out.push((Some(link.lines()), written.clone(), format!("][{label}]")));
            }
            LinkTarget::Anchor(fragment) => {
                if let Err(keys::Unresolved::KeyForm(slug)) =
                    keys::resolve_in(ctx.keys, view.path, view.body, fragment)
                {
                    out.push((
                        Some(link.lines()),
                        format!("](#{fragment})"),
                        format!("](#{slug})"),
                    ));
                    out.push((None, format!("]: #{fragment}"), format!("]: #{slug}")));
                }
            }
            _ => {}
        }
    }
    if let (Some(k), Some(core)) = (ctx.knowledge, view.core) {
        // An entry runs to the line before the next entry, and the last
        // entry runs to the end of the frontmatter.
        let mut starts: Vec<usize> = core.links.iter().map(|e| e.line).collect();
        starts.sort_unstable();
        let last_of = |line: usize| {
            starts
                .iter()
                .find(|&&next| next > line)
                .map_or(view.body_line.saturating_sub(1), |next| next - 1)
        };
        for entry in &core.links {
            if let Some(Target::Document { id, knowledge, .. }) =
                resolve_to(ctx.model, k, &entry.to)
                && let Some(canonical) = ctx.model.canonical_include(k, knowledge, &id, None)
                && canonical != entry.to
            {
                // The value can be written plain, in double quotes, or in
                // single quotes, so the plan tries each spelling.
                for written in [
                    entry.to.clone(),
                    format!("\"{}\"", entry.to),
                    format!("'{}'", entry.to),
                ] {
                    out.push((
                        Some((entry.line, last_of(entry.line))),
                        format!("to: {written}"),
                        format!("to: {canonical}"),
                    ));
                }
            }
        }
    }
    if let Some(k) = ctx.knowledge {
        for block in &view.segments.regions {
            let written = block.target.clone();
            let canonical = match &IncludeTarget::parse(&block.target) {
                IncludeTarget::Document {
                    store,
                    id,
                    fragment,
                } => ctx
                    .model
                    .document_in(k, store.as_deref(), id)
                    .and_then(|(target, _)| {
                        ctx.model
                            .canonical_include(k, target, id, fragment.as_deref())
                    }),
                IncludeTarget::File { path, region } => RepoPath::parse(path)
                    .and_then(|rp| check::asset_include(ctx.model, k, &rp, region.as_deref())),
            };
            if let Some(canonical) = canonical
                && canonical != written
            {
                out.push((
                    Some((block.open_line, block.open_line)),
                    format!("sokf:include {written} -->"),
                    format!("sokf:include {canonical} -->"),
                ));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `value_end` returns the number of the line after the value of the
/// top-level frontmatter key on line `line`: the value continues on each
/// following line that is indented or that is an item of a list, and on the
/// blank lines between them.
fn value_end(text: &str, line: usize) -> usize {
    let mut end = line + 1;
    for (i, l) in text.lines().enumerate().skip(line) {
        if l.starts_with([' ', '\t']) && !l.trim().is_empty() || l.starts_with("- ") || l == "-" {
            end = i + 2;
        } else if !l.trim().is_empty() {
            break;
        }
    }
    end
}

/// `plan_indexes` checks each index in `scope` and each index in a directory
/// that a move touches. When the text of such an index differs from its
/// expected text, `plan_indexes` adds the index to `out`. The plan rewrites
/// the index when it exists and creates it when it doesn't.
/// When `only` is given, it plans only the directories of the index paths in
/// `only` and the directories that the moves touch. When the run hasn't
/// loaded an index, it reads the index from disk (CACHE-1_AC-18).
fn plan_indexes(
    out: &mut FixPlan,
    model: &Model,
    scope: &Scope,
    only: Option<&std::collections::BTreeSet<RepoPath>>,
) {
    let touched = index::touched(model, &out.moves);
    let dirs: Option<std::collections::BTreeSet<RepoPath>> = only.map(|o| {
        o.iter()
            .filter(|p| p.file_name() == "index.md")
            .filter_map(RepoPath::parent)
            .chain(touched.iter().cloned())
            .collect()
    });
    let indexes = index::expected_for(model, &out.moves, dirs.as_ref());
    for (path, text) in indexes.expected {
        let wanted = scope.contains(&path) || path.parent().is_some_and(|d| touched.contains(&d));
        if !wanted {
            continue;
        }
        let held: Option<(Cow<'_, str>, LineEnding)> = match model.by_path(&path) {
            Some(Located::Index(_, file)) => {
                Some((Cow::Borrowed(&file.source.text), file.source.eol))
            }
            _ => model
                .tree
                .file(&path)
                .and_then(|_| crate::source::load(&model.tree.root, &path).ok())
                .map(|s| (Cow::Owned(s.text), s.eol)),
        };
        match held {
            Some((current, eol)) => {
                if current != text.as_str() {
                    out.rewrites.push(Rewrite { path, text, eol });
                }
            }
            // sokf-implements: CLI-12_AC-1
            None => out.creates.push(Rewrite {
                path,
                text,
                eol: LineEnding::Lf,
            }),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn plan_file(
    out: &mut FixPlan,
    view: DocView<'_>,
    schema: Option<&SokfSchema>,
    states: &States,
    ctx: &Ctx<'_>,
    eol: LineEnding,
    scope: &Scope,
    remap: &HashMap<RepoPath, RepoPath>,
) -> Result<()> {
    let expected = ctx.expected;
    let path = view.path;
    let in_scope = scope.contains(path);
    let mut includes: Vec<Edit> = Vec::new();
    let mut repairs: Vec<Repair> = Vec::new();
    let mut placements: Option<(Vec<Placement>, Vec<Rename>)> = None;
    // A sidecar's `resource` is its asset's path after the moves
    // (SOKF_VALIDATION_22). The edit replaces the whole value, which can run
    // over several lines, or inserts a missing `resource` after the `id`
    // line, or after the line that opens the frontmatter.
    let resource: Option<ResourceEdit> = if in_scope {
        ctx.model.asset_of(path).and_then(|(_, asset)| {
            let core = view.core?;
            let wanted = remap.get(&asset.path).unwrap_or(&asset.path);
            if core.resource.as_ref().and_then(|r| r.one()) == Some(wanted.as_str()) {
                return None;
            }
            Some(ResourceEdit {
                lines: core.line_of("resource").map(|l| l..value_end(view.text, l)),
                after: core.line_of("id").unwrap_or(1),
                value: wanted.as_str().to_string(),
            })
        })
    } else {
        None
    };
    if in_scope {
        for block in &view.segments.regions {
            if let Some(interior) = expected.include(path, block.open_line)
                && interior != block.content
            {
                includes.push(include_edit(block.open_line, block.close_line, interior));
            }
        }
        let renames = reference_renames(view, ctx);
        let checked =
            schema.map(|c| check::check_against(view, c, states, ctx, Locator::schema("")));
        if let Some(checked) = checked {
            repairs = checked.repairs;
            if checked.anchors_stale || !renames.is_empty() {
                placements = Some((checked.anchors, renames));
            }
        } else if !renames.is_empty() {
            placements = Some((Vec::new(), renames));
        }
    }
    let definition: Option<Edit> = expected
        .definitions
        .get(path)
        .and_then(|d| d.as_ref())
        .and_then(|entries| {
            let remapped: Vec<DefinitionEntry> = entries
                .iter()
                .map(|e| DefinitionEntry {
                    label: e.label.clone(),
                    path: remap
                        .get(&e.path)
                        .cloned()
                        .unwrap_or_else(|| e.path.clone()),
                    slug: e.slug.clone(),
                })
                .collect();
            let regenerated = entries.iter().any(|e| remap.contains_key(&e.path));
            if !in_scope && !regenerated {
                return None;
            }
            let rendered = render_definitions(&remapped);
            let current = view.segments.definition.as_ref();
            let differs = match current {
                None => !rendered.is_empty(),
                Some(d) => d.trailing || d.text.trim_end() != rendered.trim_end(),
            };
            differs.then(|| definition_edit(view.text, current.map(|d| d.line), &rendered))
        });
    if includes.is_empty()
        && repairs.is_empty()
        && definition.is_none()
        && resource.is_none()
        && placements.is_none()
    {
        return Ok(());
    }
    let anchors = placements.as_ref().map(|(placements, renames)| AnchorEdit {
        body: view.body,
        placements,
        text: view.text,
        renames,
    });
    let mut edits = includes;
    edits.extend(definition);
    edits.extend(resource.map(|r| r.edit()));
    let text = rewrite(view.text, view.segments, edits, &repairs, anchors.as_ref())?;
    if text == view.text {
        return Ok(());
    }
    out.rewrites.push(Rewrite {
        path: remap.get(path).cloned().unwrap_or_else(|| path.clone()),
        text,
        eol,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;
    use crate::{Cache, CacheMode, RepoRoot, api::load_repository, config, grammar};

    /// `fixture` copies the `indexes` fixture to a directory, named for
    /// `name`, under the temp directory, and returns its path.
    fn fixture(name: &str) -> std::path::PathBuf {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/indexes");
        let dir = crate::test_support::fresh_dir(&format!("plan-{name}"));
        copy(&source, &dir);
        dir
    }

    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn plan_on(dir: &Path, scope: &Scope) -> FixPlan {
        let root = RepoRoot::new(dir.to_path_buf());
        let config = config::load_config(&root).unwrap();
        let grammar = grammar::load_grammar().unwrap();
        let cache = Cache::open(&root, CacheMode::Off);
        let loaded = load_repository(&root, &config, &grammar, None, &cache).unwrap();
        plan(
            &loaded.model,
            &loaded.compiled,
            &loaded.expected,
            &loaded.keys,
            scope,
            None,
        )
        .unwrap()
    }

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn paths(rewrites: &[Rewrite]) -> Vec<&str> {
        rewrites.iter().map(|r| r.path.as_str()).collect()
    }

    // sokf-tests: SOKF_VALIDATION_33
    // sokf-tests: SOKF_VALIDATION_36
    // sokf-tests: CLI-4_AC-5
    // sokf-tests: SOKF_VALIDATION_55
    #[test]
    fn indexes_are_planned_in_scope_and_where_a_move_touches() {
        let dir = fixture("scope");
        // With everything in scope, the plan moves the document, then
        // creates some indexes and rewrites others to match the moved tree.
        let all = plan_on(&dir, &Scope::all());
        assert_eq!(all.moves.len(), 1);
        assert_eq!(
            paths(&all.creates),
            vec![
                "/knowledge/issues/closed/index.md",
                "/knowledge/none/index.md"
            ]
        );
        assert_eq!(
            paths(&all.rewrites),
            vec![
                "/knowledge/issues/index.md",
                "/knowledge/issues/open/index.md",
                "/knowledge/sub/index.md"
            ]
        );
        assert!(all.creates.iter().all(|c| c.eol == LineEnding::Lf));
        // When `PATHS` contains only the document, the plan repairs the
        // indexes that the move touches: the directory it leaves, the one it
        // creates, and the parent that gains the new directory. It repairs
        // nothing outside them.
        let mut scope = Scope::default();
        scope.add(p("/knowledge/issues/open/issue-001-move.md"));
        let narrowed = plan_on(&dir, &scope);
        assert_eq!(narrowed.moves.len(), 1);
        assert_eq!(
            paths(&narrowed.creates),
            vec!["/knowledge/issues/closed/index.md"]
        );
        assert_eq!(
            paths(&narrowed.rewrites),
            vec![
                "/knowledge/issues/index.md",
                "/knowledge/issues/open/index.md"
            ]
        );
        // When `PATHS` contains a directory, the plan repairs only that
        // directory's index.
        let mut scope = Scope::default();
        scope.dirs.insert(p("/knowledge/sub"));
        let sub = plan_on(&dir, &scope);
        assert!(sub.moves.is_empty() && sub.creates.is_empty());
        assert_eq!(paths(&sub.rewrites), vec!["/knowledge/sub/index.md"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_value_ends_before_the_next_key() {
        let text = "---\nresource: /k/a.png\ntitle: T\n---\n";
        assert_eq!(value_end(text, 2), 3);
        let text = "---\nresource:\n  - /a\n\n  - /b\n- /c\ntitle: T\n---\n";
        assert_eq!(value_end(text, 2), 7);
        let text = "---\nresource: |\n  /a\n\ntitle: T\n---\n";
        assert_eq!(value_end(text, 2), 4);
        let text = "---\nresource: /a\n---\n  indented body\n";
        assert_eq!(value_end(text, 2), 3);
    }

    // sokf-tests: SOKF_VALIDATION_20
    // sokf-tests: SOKF_VALIDATION_21
    #[test]
    fn a_refused_move_touches_no_index() {
        let dir = fixture("refused");
        fs::create_dir_all(dir.join("knowledge/issues/closed")).unwrap();
        fs::write(
            dir.join("knowledge/issues/closed/issue-001-move.md"),
            "occupied\n",
        )
        .unwrap();
        let mut scope = Scope::default();
        scope.add(p("/knowledge/issues/open/issue-001-move.md"));
        let out = plan_on(&dir, &scope);
        assert!(out.moves.is_empty());
        assert_eq!(out.refused.len(), 1);
        assert!(out.creates.is_empty() && out.rewrites.is_empty(), "{out:?}");
        fs::remove_dir_all(&dir).unwrap();
    }
}
