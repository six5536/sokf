//! This module reads the files that the checks need (documents, schemas,
//! additional files, and indexes) in full and admits them to the model.

use std::collections::BTreeSet;

use super::load::{CachedLoader, ensure_not_missed};
use crate::{
    Cache, Compiled, Model, RepoPath, RepoRoot, Result, Segments, include,
    markdown::{IncludeTarget, Link, LinkTarget},
    model::{CoreFields, KeyIndex, Loader, ParseMode, frontmatter::Resource},
};

/// `admit_for_check` reads in full what the checks of `paths` read: the
/// files among `paths`, the documents that they read, and the schemas that
/// their checks compile. It then compiles the schemas that it read. A model
/// built on a removed file that the hint left out describes a repository
/// that no longer exists, so `admit_for_check` stops the run in that case.
pub(super) fn admit_for_check(
    model: &mut Model,
    loader: &mut CachedLoader<'_>,
    cache: &Cache,
    compiled: &mut Compiled,
    paths: &BTreeSet<RepoPath>,
) -> Result<()> {
    admit_full(model, loader, cache, paths)?;
    admit_closure(model, loader, cache, paths);
    admit_schemas_for(model, loader, cache, paths)?;
    ensure_not_missed(cache)?;
    compiled.compile_more(model);
    Ok(())
}

/// `admit_full` reads in full each document and additional file among
/// `paths` that the model does not hold, and admits it to the model. It reads each
/// schema among `paths` in full too, through `admit_schema`.
pub(super) fn admit_full(
    model: &mut Model,
    loader: &mut CachedLoader<'_>,
    cache: &Cache,
    paths: &BTreeSet<RepoPath>,
) -> Result<()> {
    for path in paths {
        if cache.is_schema(path) {
            admit_schema(model, loader, cache, path)?;
            continue;
        }
        if model.holds(path) {
            continue;
        }
        // For a file outside the roots, the checks use the marker rows that
        // the store keeps and do not read the file (CACHE-1_AC-33).
        let marker_file = model.is_marker_file(path);
        if marker_file && !model.markers.contains_key(path) {
            let rows = cache.stored_markers(path);
            if !rows.is_empty() {
                model.markers.insert(path.clone(), rows);
            }
        }
        if let Some(&schema) = model.dispatch.get(path) {
            if model.additional.iter().any(|a| &a.path == path) {
                continue;
            }
            let source = loader.load(path)?;
            let parsed = loader.parse(&source, ParseMode::Plain);
            model.admit_additional(crate::model::AdditionalFile {
                path: path.clone(),
                source,
                body: parsed.body,
                segments: parsed.segments,
                schema,
            });
            continue;
        }
        if marker_file {
            continue;
        }
        let doc = loader.full_document(path)?;
        let k = cache.knowledge_of(path).unwrap_or(0);
        model.admit(k, doc);
    }
    Ok(())
}

/// `admit_schema` reads the schema at `path` in full, when the run has not
/// read it yet, and puts it in the model in place of the summary that the
/// store keeps.
// sokf-implements: CACHE-1_AC-25
fn admit_schema(
    model: &mut Model,
    loader: &mut CachedLoader<'_>,
    cache: &Cache,
    path: &RepoPath,
) -> Result<()> {
    let loaded = match model.by_path(path) {
        Some(crate::Located::Schema(_, s)) => s.loaded,
        _ => return Ok(()),
    };
    if loaded {
        return Ok(());
    }
    let doc = loader.full_document(path)?;
    let k = cache.knowledge_of(path).unwrap_or(0);
    model.admit_schema(k, crate::model::SchemaDocument::new(doc));
    Ok(())
}

/// `admit_schemas_for` reads in full the schemas that the checks of `paths`
/// compile: the schemas that claim each document's type, the schema that
/// governs each additional file, and each schema among `paths`. It reads no
/// other schema.
// sokf-implements: CACHE-1_AC-25
// sokf-implements: CACHE-1_AC-42
pub(super) fn admit_schemas_for(
    model: &mut Model,
    loader: &mut CachedLoader<'_>,
    cache: &Cache,
    paths: &BTreeSet<RepoPath>,
) -> Result<()> {
    let mut wanted: BTreeSet<RepoPath> = BTreeSet::new();
    for path in paths {
        match model.by_path(path) {
            Some(crate::Located::Document(k, doc)) => {
                if let Some(t) = doc.type_name() {
                    for r in model.claimants(k, t) {
                        wanted.insert(model.schema(r).doc.path.clone());
                    }
                }
            }
            Some(crate::Located::Schema(..)) => {
                wanted.insert(path.clone());
            }
            Some(crate::Located::Additional(file)) => {
                if let Some(r) = file.schema {
                    wanted.insert(model.schema(r).doc.path.clone());
                }
            }
            _ => {}
        }
    }
    for path in &wanted {
        admit_schema(model, loader, cache, path)?;
    }
    Ok(())
}

/// `admit_schemas_of_type` reads in full, in every store, the schemas that
/// claim `type_name`. A command that compiles the schema of one type calls
/// `admit_schemas_of_type`.
// sokf-implements: CACHE-1_AC-25
pub fn admit_schemas_of_type(
    model: &mut Model,
    root: &RepoRoot,
    cache: &Cache,
    type_name: &str,
) -> Result<()> {
    let wanted: Vec<RepoPath> = model
        .knowledges
        .iter()
        .flat_map(|k| {
            k.schemas_for(type_name)
                .iter()
                .map(|&i| k.schemas[i].doc.path.clone())
        })
        .collect();
    admit_paths(model, root, cache, &wanted)
}

/// `admit_schema_by_id` reads in full the schema with `id` in each store
/// that has one.
// sokf-implements: CACHE-1_AC-25
pub fn admit_schema_by_id(
    model: &mut Model,
    root: &RepoRoot,
    cache: &Cache,
    id: &str,
) -> Result<()> {
    let wanted: Vec<RepoPath> = model
        .knowledges
        .iter()
        .filter_map(|k| k.schema_by_id(id).map(|i| k.schemas[i].doc.path.clone()))
        .collect();
    admit_paths(model, root, cache, &wanted)
}

/// `admit_paths` reads in full each schema in `wanted`.
fn admit_paths(
    model: &mut Model,
    root: &RepoRoot,
    cache: &Cache,
    wanted: &[RepoPath],
) -> Result<()> {
    let mut loader = CachedLoader::new(root, cache);
    for path in wanted {
        admit_schema(model, &mut loader, cache, path)?;
    }
    Ok(())
}

/// `unit_paths` returns the path of every document, schema, and additional
/// file that the walk lists. A full run reads these files. An incremental run
/// calls `Model::is_unit` for each path instead and never builds the full set.
pub(super) fn unit_paths(model: &Model) -> BTreeSet<RepoPath> {
    let mut all = model.document_paths();
    all.extend(model.dispatch.keys().cloned());
    all
}

/// `admit_indexes` reads the index file of each directory in `dirs` when the
/// walk lists the index file and the model does not hold it. The check of the
/// directory needs its index file.
// sokf-implements: CACHE-1_AC-18
pub(super) fn admit_indexes(
    model: &mut Model,
    loader: &mut CachedLoader<'_>,
    cache: &Cache,
    dirs: &BTreeSet<RepoPath>,
) {
    for dir in dirs {
        let Some(path) = dir.join("index.md") else {
            continue;
        };
        if !cache.is_walked(&path) || model.by_path(&path).is_some() {
            continue;
        }
        let Ok(Some(index)) = loader.index(&path) else {
            continue;
        };
        let k = cache.knowledge_of(&path).unwrap_or(0);
        model.admit_index(k, index);
    }
}

/// `admit_closure` reads in full every document that the checks of `from`
/// look up. These are the targets of the labels, links, includes, images,
/// and paths in those files. The check of a file that includes a whole document
/// resolves the references of the included body again, so `admit_closure`
/// also reads the targets of each such included document (CACHE-3_AC-1). When
/// a file cannot be read, `admit_closure` skips it: the model keeps the
/// summary from the store, and the check reports the error.
// sokf-implements: CACHE-3_AC-2
pub(super) fn admit_closure(
    model: &mut Model,
    loader: &mut CachedLoader<'_>,
    cache: &Cache,
    from: &BTreeSet<RepoPath>,
) {
    let mut queue: Vec<RepoPath> = from.iter().cloned().collect();
    let mut seen: BTreeSet<RepoPath> = BTreeSet::new();
    while let Some(path) = queue.pop() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let (paths, includes) = references(model, &path);
        for p in paths {
            if cache.is_schema(&p) {
                let _ = admit_schema(model, loader, cache, &p);
                continue;
            }
            if model.holds(&p) || !cache.is_document(&p) {
                continue;
            }
            let Ok(doc) = loader.full_document(&p) else {
                continue;
            };
            let k = cache.knowledge_of(&p).unwrap_or(0);
            model.admit(k, doc);
        }
        queue.extend(includes);
    }
}

/// `references` returns the paths that the checks of a file might look up,
/// and, among them, the documents whose bodies the file includes.
fn references(model: &Model, path: &RepoPath) -> (BTreeSet<RepoPath>, BTreeSet<RepoPath>) {
    let mut paths: BTreeSet<RepoPath> = BTreeSet::new();
    let mut includes: BTreeSet<RepoPath> = BTreeSet::new();
    if let Some(rows) = model.markers.get(path) {
        marker_references(model, rows, &mut paths);
    }
    let (k, core, links, segments) = match model.by_path(path) {
        Some(crate::Located::Document(k, d)) => {
            (Some(k), Some(&d.core), &d.body.links, &d.segments)
        }
        Some(crate::Located::Schema(k, s)) => (
            Some(k),
            Some(&s.doc.core),
            &s.doc.body.links,
            &s.doc.segments,
        ),
        Some(crate::Located::Additional(f)) => (
            include::knowledge_of_additional(model, f),
            None,
            &f.body.links,
            &f.segments,
        ),
        _ => return (paths, includes),
    };
    let lookup = Lookup {
        model,
        knowledge: k,
    };
    lookup.link_references(path, links, &mut paths);
    if let Some(core) = core {
        lookup.frontmatter_references(core, &mut paths);
    }
    lookup.include_references(segments, &mut paths, &mut includes);
    (paths, includes)
}

/// A `Lookup` finds the files that the references of one file lead to.
struct Lookup<'a> {
    /// The model.
    model: &'a Model,
    /// The knowledge store of the file, if it lies in one.
    knowledge: Option<usize>,
}

impl Lookup<'_> {
    /// `by_id` adds to `paths` each document of the id `id` in the store
    /// `store`, or in the store of the file when `store` is `None`.
    fn by_id(&self, paths: &mut BTreeSet<RepoPath>, store: Option<&str>, id: &str) {
        let Some(k) = self.knowledge else { return };
        let kn = match store {
            None => Some(k),
            Some(s) => self.model.store(k, s).map(|h| h.knowledge),
        };
        if let Some(kn) = kn {
            paths.extend(self.model.ids(kn, id));
        }
    }

    /// `by_path` adds `p` to `paths`, and the sidecar of the asset at `p`
    /// when there is one.
    fn by_path(&self, paths: &mut BTreeSet<RepoPath>, p: RepoPath) {
        if let Some((_, asset)) = self.model.asset_at(&p) {
            paths.insert(asset.sidecar_path());
        }
        paths.insert(p);
    }

    /// `link_references` adds to `paths` the target of each of `links`, the
    /// links of the body of the file at `path`.
    fn link_references(&self, path: &RepoPath, links: &[Link], paths: &mut BTreeSet<RepoPath>) {
        for link in links {
            match &link.target {
                LinkTarget::Host(_) => {
                    if let Some(label) = crate::model::keys::Label::of(&link.target)
                        && let Some(id) = label.id
                    {
                        self.by_id(paths, label.store, id);
                    }
                }
                LinkTarget::Repo { path: p, .. } => self.by_path(paths, p.clone()),
                LinkTarget::Relative { target, .. } => {
                    if let Some(p) = path.resolve_relative(target) {
                        self.by_path(paths, p);
                    }
                }
                _ => {}
            }
        }
    }

    /// `frontmatter_references` adds to `paths` the target of each `links`
    /// entry of `core`, each of its resources, and the resource of each of
    /// its sources.
    fn frontmatter_references(&self, core: &CoreFields, paths: &mut BTreeSet<RepoPath>) {
        for entry in &core.links {
            if entry.to.starts_with('/') {
                if let Some(p) = RepoPath::parse(&entry.to) {
                    self.by_path(paths, p);
                }
            } else {
                let (store, id) = crate::model::split_store(&entry.to);
                self.by_id(paths, store, id);
            }
        }
        for r in core.resource.iter().flat_map(Resource::entries) {
            if let Some(p) = RepoPath::parse(r) {
                self.by_path(paths, p);
            }
        }
        for s in &core.sources {
            if let Some(p) = RepoPath::parse(&s.resource) {
                self.by_path(paths, p);
            }
        }
    }

    /// `include_references` adds to `paths` the target of each include
    /// marker of `segments`. It also adds to `includes` each document whose
    /// whole body a marker includes.
    fn include_references(
        &self,
        segments: &Segments,
        paths: &mut BTreeSet<RepoPath>,
        includes: &mut BTreeSet<RepoPath>,
    ) {
        for region in &segments.regions {
            match IncludeTarget::parse(&region.target) {
                IncludeTarget::Document {
                    store,
                    id,
                    fragment,
                } => {
                    let mut targets = BTreeSet::new();
                    self.by_id(&mut targets, store.as_deref(), &id);
                    if fragment.as_deref().is_none_or(str::is_empty) {
                        includes.extend(targets.iter().cloned());
                    }
                    paths.extend(targets);
                }
                IncludeTarget::File { path: p, .. } => {
                    if let Some(p) = RepoPath::parse(&p) {
                        self.by_path(paths, p);
                    }
                }
            }
        }
    }
}

/// `marker_references` adds to `paths` the path of each document that one of
/// `rows` refers to by id. It looks in the store that the reference
/// specifies, or in every store when the reference specifies none
/// (`SOKF_FORMAT_148`).
fn marker_references(
    model: &Model,
    rows: &[crate::markers::MarkerRow],
    paths: &mut BTreeSet<RepoPath>,
) {
    use crate::markers::{Reference, RowKind};
    for row in rows {
        let RowKind::Marker { target, .. } = &row.kind else {
            continue;
        };
        let Reference::Label(label) = Reference::parse(target) else {
            continue;
        };
        let Some(id) = label.id.filter(|i| !i.is_empty()) else {
            continue;
        };
        for k in 0..model.knowledges.len() {
            let named = label
                .store
                .is_none_or(|s| model.store(k, s).is_some_and(|h| h.knowledge == k));
            if named {
                paths.extend(model.ids(k, id));
            }
        }
    }
}

/// `needed_slugs` returns the slugs that the checks might resolve in the files
/// that the model holds. It returns three kinds:
///
/// - The key or the label fragment of each marker, as written and as each
///   id that it can name
/// - The fragment of each label in the links of the documents, schemas, and
///   additional files, as written and as each id that it can name
/// - The id of each key that this run captured
pub(super) fn needed_slugs(model: &Model, captured: &KeyIndex) -> BTreeSet<String> {
    use crate::markdown::{self, LinkTarget};
    let mut slugs = BTreeSet::new();
    for rows in model.markers.values() {
        for row in rows {
            let crate::markers::RowKind::Marker { target, .. } = &row.kind else {
                continue;
            };
            let key = match crate::markers::Reference::parse(target) {
                crate::markers::Reference::Key(key) => Some(key),
                crate::markers::Reference::Label(label) => label.fragment,
            };
            if let Some(key) = key.filter(|k| !k.is_empty()) {
                slugs.insert(key.to_string());
                slugs.extend(markdown::key_ids(key));
            }
        }
    }
    let links = model
        .documents_and_schemas()
        .map(|(_, d)| &d.body.links)
        .chain(model.additional.iter().map(|f| &f.body.links));
    for list in links {
        for link in list {
            if let LinkTarget::Host(_) = &link.target
                && let Some(label) = crate::model::keys::Label::of(&link.target)
                && let Some(f) = label.fragment.filter(|f| !f.is_empty())
            {
                slugs.insert(f.to_string());
                slugs.extend(markdown::key_ids(f));
            }
        }
    }
    for entries in captured.document.values() {
        for e in entries {
            slugs.insert(e.id.clone());
        }
    }
    slugs
}
