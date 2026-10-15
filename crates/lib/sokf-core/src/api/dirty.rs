//! This module computes the dirty set and builds the rows that the store
//! keeps. The dirty set is the set of queries whose answers changed since the
//! last run.

use std::collections::BTreeSet;

use crate::{Cache, Model, RepoPath, RepoRoot, model::Query};

/// `additional_rows` returns a row for the store for each additional file
/// of the model, with the path of the schema that governs the file.
pub(super) fn additional_rows(model: &Model) -> Vec<crate::cache::AdditionalRow> {
    model
        .dispatch
        .iter()
        .map(|(path, schema)| crate::cache::AdditionalRow {
            path: path.clone(),
            schema: schema.map(|r| model.schema(r).doc.path.clone()),
        })
        .collect()
}

/// `scope_name` returns the name that the store gives to the scope of a
/// key: `document` for a document scope, or the name of the host.
pub(super) fn scope_name(scope: &crate::grammar::KeyScope) -> String {
    match scope {
        crate::grammar::KeyScope::Document => "document".to_string(),
        crate::grammar::KeyScope::Host(h) => h.clone(),
    }
}

/// `key_query` returns the query that a key of `scope` and `slug` in the
/// knowledge `knowledge` answers. A key of any other scope answers the
/// query of a key in a document.
pub(super) fn key_query(scope: &str, slug: &str, knowledge: usize) -> String {
    match scope {
        "repository" => Query::KeyRepository(slug.to_string()).key(),
        "knowledge" => Query::KeyKnowledge {
            knowledge,
            slug: slug.to_string(),
        }
        .key(),
        _ => Query::KeyDocument {
            knowledge,
            slug: slug.to_string(),
        }
        .key(),
    }
}

/// `schema_rows` returns a row for the store for each schema file of the
/// model, with the key that the schema claims.
pub(super) fn schema_rows(model: &Model, cache: &Cache) -> Vec<crate::cache::SchemaRow> {
    let mut rows = Vec::new();
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        for (i, schema) in knowledge.schemas.iter().enumerate() {
            rows.push(crate::cache::SchemaRow {
                knowledge: k,
                path: schema.doc.path.clone(),
                key: knowledge.schema_key(i),
                hash: cache.hash_of(&schema.doc.path).unwrap_or(0),
                globs: schema.globs.join("\n"),
                asset_globs: schema.asset_globs.join("\n"),
                uses: schema.uses.join("\n"),
                line: schema.doc.body_line,
                archived: schema.archived.encode(),
            });
        }
    }
    rows
}

/// `changed_queries` returns the queries whose answers changed since the
/// last run. It works them out from the files that the last sync found
/// changed. For each changed document and schema, it compares the hash of
/// the parts that other documents can see with the hash in the store's row.
/// It also adds the queries of the added or removed files, of the changed
/// asset pairings, and of the probed paths whose existence changed.
///
/// It adds to `dirs` each directory whose index the run must compute again:
///
/// - The parent of each file that was added, removed, or changed
/// - Each directory that was added, and the parent of each directory that
///   was added or removed
/// - The directory of each document that an id moved to or from
/// - The directory of each asset or sidecar whose pairing changed
// sokf-implements: CACHE-1_AC-4
// sokf-implements: CACHE-1_AC-5
// sokf-implements: CACHE-1_AC-18
pub(super) fn changed_queries(
    model: &Model,
    cache: &Cache,
    root: &RepoRoot,
    dirs: &mut BTreeSet<RepoPath>,
) -> BTreeSet<String> {
    let mut q: BTreeSet<String> = BTreeSet::new();
    let mut ids: BTreeSet<(usize, String)> = BTreeSet::new();
    let changed = cache.changed_set();
    let touch = |dirs: &mut BTreeSet<RepoPath>, path: &RepoPath| {
        if let Some(parent) = path.parent() {
            dirs.insert(parent);
        }
    };
    for (k, doc) in model.documents_and_schemas() {
        if !changed.contains(&doc.path) {
            continue;
        }
        touch(dirs, &doc.path);
        q.insert(Query::Body(doc.path.clone()).key());
        match cache.old_row(&doc.path) {
            Some(old) => {
                // A change to a scalar value, such as a lifecycle state, can
                // archive the document, which a reader checks
                // (SOKF_SCHEMA_54).
                let scalars =
                    crate::model::archive::encode_scalars(&crate::model::archive::scalars(doc));
                if old.visible != doc.visible_hash() || old.scalars != scalars {
                    doc_and_type(&mut q, k, old.id.as_deref(), old.type_name.as_deref());
                    doc_and_type(&mut q, k, doc.id(), doc.type_name());
                    q.insert(Query::Path(doc.path.clone()).key());
                    if old.id != doc.core.id {
                        ids.extend(old.id.iter().map(|id| (k, id.clone())));
                        ids.extend(doc.id().map(|id| (k, id.to_string())));
                    }
                }
            }
            None => {
                doc_and_type(&mut q, k, doc.id(), doc.type_name());
                q.insert(Query::Path(doc.path.clone()).key());
                q.insert(Query::Exists(doc.path.clone()).key());
                ids.extend(doc.id().map(|id| (k, id.to_string())));
            }
        }
    }
    for path in cache.removed() {
        touch(dirs, &path);
        file_queries(&mut q, &path, true);
        if let Some(old) = cache.old_row(&path) {
            doc_and_type(
                &mut q,
                old.knowledge,
                old.id.as_deref(),
                old.type_name.as_deref(),
            );
            ids.extend(old.id.map(|id| (old.knowledge, id)));
        }
        for target in cache.old_citation_targets(&path) {
            q.insert(Query::Citations(target).key());
        }
        for (scope, slug, k) in cache.old_keys_of(&path) {
            q.insert(key_query(&scope, &slug, k));
        }
        for target in cache.old_plans_of(&path) {
            q.insert(Query::Planned(target).key());
        }
    }
    for path in &changed {
        let file = model.tree.file(path);
        if file
            .is_some_and(|f| matches!(f.kind, crate::FileKind::Document | crate::FileKind::Schema))
        {
            continue;
        }
        // The run computes the index of the parent again only for a file in
        // a root, because only a directory in a root has an index.
        if file.is_some_and(|f| f.root.is_some()) {
            touch(dirs, path);
        }
        file_queries(&mut q, path, cache.is_new(path));
    }
    // When the stat of a candidate file, such as an included source or an
    // asset, is new or has changed, what its readers took from it has
    // changed too. The run marks their queries without reading the file.
    // sokf-implements: CACHE-1_AC-26
    // sokf-implements: CACHE-1_AC-43
    for (path, new) in cache.pending_candidates() {
        file_queries(&mut q, &path, new);
    }
    let rows = asset_rows(model);
    for path in cache.assets_changed(&rows) {
        touch(dirs, &path);
        q.insert(Query::Path(path).key());
    }
    cache.record_assets(rows);
    for path in cache.probes_changed(root) {
        q.insert(Query::Exists(path).key());
    }
    // A directory that was added needs an index, and a directory that was
    // added or removed changes the listing of its parent. When an id moves
    // from one document to another, the index of each directory that holds a
    // document with that id changes. While two documents have the same id,
    // the index lists each of them as a plain file. When only one document
    // has the id again, the index lists that document as a document again.
    let (added, removed) = cache.dirs_changed();
    for dir in added {
        touch(dirs, &dir);
        dirs.insert(dir);
    }
    for dir in removed {
        touch(dirs, &dir);
    }
    for (k, id) in ids {
        for path in model.ids(k, &id) {
            touch(dirs, &path);
        }
    }
    q
}

/// `file_queries` adds to `q` the queries that a file's body and path
/// answer, and the query of its existence when `exists` is set.
fn file_queries(q: &mut BTreeSet<String>, path: &RepoPath, exists: bool) {
    q.insert(Query::Body(path.clone()).key());
    q.insert(Query::Path(path.clone()).key());
    if exists {
        q.insert(Query::Exists(path.clone()).key());
    }
}

/// `doc_and_type` adds to `q` the queries that the id and the type of a
/// document answer.
fn doc_and_type(q: &mut BTreeSet<String>, k: usize, id: Option<&str>, type_name: Option<&str>) {
    if let Some(id) = id {
        q.insert(
            Query::Doc {
                knowledge: k,
                id: id.to_string(),
            }
            .key(),
        );
    }
    if let Some(t) = type_name {
        q.insert(
            Query::Type {
                knowledge: k,
                type_name: t.to_string(),
            }
            .key(),
        );
    }
}

/// `asset_rows` returns the asset pairings of the model as rows for the
/// store. A row records the sidecar of an asset whenever the sidecar is a
/// document, whether the run holds the sidecar or only the store does. As a
/// result, two runs write the same row for the pairing, whether or not they
/// hold the sidecar.
fn asset_rows(model: &Model) -> Vec<crate::cache::AssetRow> {
    let mut rows = Vec::new();
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        for asset in &knowledge.assets {
            rows.push(crate::cache::AssetRow {
                knowledge: k,
                asset: asset.path.clone(),
                sidecar: asset.has_sidecar.then(|| asset.sidecar_path()),
            });
        }
    }
    rows
}
