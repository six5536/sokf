//! This module defines `Knowledge` and `Model`, which index the parsed tree.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{
    AdditionalFile, Asset, Document, Fallback, IndexFile, Manifest, Query, Recorder,
    SchemaDocument, SchemaRef,
};
use crate::{FileKind, Finding, RepoPath, Tree};

// sokf:begin knowledge
/// A `Knowledge` is one knowledge store.
#[derive(Debug)]
pub struct Knowledge {
    /// The root directory.
    pub root: RepoPath,
    /// The manifest, when the manifest file exists.
    pub manifest: Option<Manifest>,
    /// The path of the manifest.
    pub manifest_path: RepoPath,
    /// The schemas, by path.
    pub schemas: Vec<SchemaDocument>,
    /// The documents, by path.
    pub documents: Vec<Document>,
    /// The index files, by path.
    pub indexes: Vec<IndexFile>,
    /// The assets, by path (FORMAT-1_AC-1).
    pub assets: Vec<Asset>,
    /// The documents, by `id`. An id that more than one document has is a
    /// duplicate.
    pub by_id: HashMap<String, Vec<usize>>,
    /// The documents, by path.
    pub by_path: HashMap<RepoPath, usize>,
    /// The documents, by `type`.
    pub by_type: HashMap<String, Vec<usize>>,
    /// The schemas, by the type that each one dispatches on.
    pub schemas_by_type: HashMap<String, Vec<usize>>,
    /// The assets, by path.
    pub asset_by_path: HashMap<RepoPath, usize>,
    /// The assets, by the path of their sidecar.
    pub asset_of_sidecar: HashMap<RepoPath, usize>,
}
// sokf:end knowledge

impl Knowledge {
    /// `insert_document` adds a document to the store and indexes it by id,
    /// type, and path. When the document is the sidecar of an asset, it
    /// also pairs the asset with it.
    pub(super) fn insert_document(&mut self, doc: Document) {
        let index = self.documents.len();
        if let Some(id) = doc.id() {
            self.by_id.entry(id.to_string()).or_default().push(index);
        }
        if let Some(t) = doc.type_name() {
            self.by_type.entry(t.to_string()).or_default().push(index);
        }
        self.by_path.insert(doc.path.clone(), index);
        if let Some(&a) = self.asset_of_sidecar.get(&doc.path) {
            self.assets[a].sidecar = Some(index);
        }
        self.documents.push(doc);
    }

    /// `document` returns the document with `id`, when exactly one document
    /// has it. A schema is a document of type `Schema`, so `document`
    /// resolves the id of a schema too.
    pub fn document(&self, id: &str) -> Option<&Document> {
        if self.ids(id).len() != 1 {
            return None;
        }
        match self.by_id.get(id).map(Vec::as_slice) {
            Some([one]) => Some(&self.documents[*one]),
            _ => self.schema_by_id(id).map(|i| &self.schemas[i].doc),
        }
    }

    /// `ids` returns the path of every document and schema that has `id`.
    pub fn ids(&self, id: &str) -> Vec<&RepoPath> {
        let mut paths: Vec<&RepoPath> = self
            .by_id
            .get(id)
            .into_iter()
            .flatten()
            .map(|&i| &self.documents[i].path)
            .collect();
        paths.extend(
            self.schemas
                .iter()
                .filter(|s| s.doc.id() == Some(id))
                .map(|s| &s.doc.path),
        );
        paths
    }

    /// `documents_of` returns the documents of type `type_name`.
    pub fn documents_of(&self, type_name: &str) -> impl Iterator<Item = &Document> {
        self.by_type
            .get(type_name)
            .into_iter()
            .flatten()
            .map(|&i| &self.documents[i])
    }

    /// `schemas_for` returns the indexes of the schemas that dispatch on
    /// `type_name`.
    pub fn schemas_for(&self, type_name: &str) -> &[usize] {
        self.schemas_by_type
            .get(type_name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// `is_document` reports whether `path` is a document of this knowledge
    /// store, schemas included.
    pub fn is_document(&self, path: &RepoPath) -> bool {
        self.by_path.contains_key(path) || self.schemas.iter().any(|s| &s.doc.path == path)
    }

    /// `asset_of` returns the asset that the document at `sidecar`
    /// describes, if there is one.
    pub fn asset_of(&self, sidecar: &RepoPath) -> Option<&Asset> {
        self.asset_of_sidecar.get(sidecar).map(|&i| &self.assets[i])
    }

    /// `asset_at` returns the asset at `path`, if there is one.
    pub fn asset_at(&self, path: &RepoPath) -> Option<&Asset> {
        self.asset_by_path.get(path).map(|&i| &self.assets[i])
    }

    /// `schema_key` returns the key of schema `i`. The key is the value of
    /// the schema's `type` const. A schema that has no such const
    /// dispatches only by its globs, and its key is its path (CACHE-1_AC-15).
    pub fn schema_key(&self, i: usize) -> String {
        let s = &self.schemas[i];
        s.type_const
            .clone()
            .unwrap_or_else(|| s.doc.path.to_string())
    }

    /// `schema_by_id` returns the index of the schema with `id`, when exactly
    /// one schema has it.
    pub fn schema_by_id(&self, id: &str) -> Option<usize> {
        let mut found = self
            .schemas
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id() == id);
        let first = found.next()?;
        found.next().is_none().then_some(first.0)
    }
}

// sokf:begin model
/// A `Model` holds every knowledge store, every additional file, and the tree.
#[derive(Debug)]
pub struct Model {
    /// The knowledge stores, in configuration order.
    pub knowledges: Vec<Knowledge>,
    /// The additional files that the run has loaded, by path.
    pub additional: Vec<AdditionalFile>,
    /// Each additional file that a schema's globs match, by path, whether the
    /// run has loaded it or not. The value is the schema whose glob matches
    /// the file, or none when the globs of more than one schema match it
    /// (CACHE-1_AC-15).
    pub dispatch: BTreeMap<RepoPath, Option<SchemaRef>>,
    /// The walked tree.
    pub tree: Tree,
    /// The findings of the build, such as files that fail to load and globs
    /// that clash.
    pub problems: Vec<Finding>,
    /// The reads of the run. The recorder keeps each query that the check of
    /// a file makes, under the path of that file (CACHE-1_AC-4).
    pub reads: Recorder,
    /// The cache. It supplies facts about the documents that the run did not
    /// load (CACHE-1_AC-21).
    pub fallback: Option<Box<dyn Fallback>>,
    /// The markers of each file outside the knowledge roots that the run has
    /// loaded, by path. The markers come from a scan of the file, or from the
    /// rows that the cache keeps for it.
    pub markers: BTreeMap<RepoPath, Vec<crate::markers::MarkerRow>>,
    /// What the run does with markers. It is `Off` until a check sets it.
    pub marker_check: crate::MarkerCheck,
    /// `plans` holds the files that the documents plan (SOKF_SCHEMA_53).
    pub plans: super::PlanIndex,
}
// sokf:end model

/// A `Located` is what a path is in the model, together with the index of
/// the knowledge store, for a file of a knowledge store.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Located<'a> {
    /// A document of a knowledge store.
    Document(usize, &'a Document),
    /// A schema of a knowledge store.
    Schema(usize, &'a SchemaDocument),
    /// An additional file.
    Additional(&'a AdditionalFile),
    /// An index file of a knowledge store.
    Index(usize, &'a IndexFile),
    /// An asset of a knowledge store.
    Asset(usize, &'a Asset),
}

impl Model {
    /// `watch` records every later read against `reader`.
    pub fn watch(&self, reader: &RepoPath) {
        self.reads.borrow_mut().watch(reader);
    }

    /// `unwatch` stops recording reads against any reader.
    pub fn unwatch(&self) {
        self.reads.borrow_mut().unwatch();
    }

    /// `note` records `query` against the watched reader.
    pub fn note(&self, query: Query) {
        self.reads.borrow_mut().note(query);
    }

    /// `reads_of` returns the queries that `reader` made.
    pub fn reads_of(&self, reader: &RepoPath) -> BTreeSet<Query> {
        self.reads.borrow().of(reader)
    }

    /// `reads` returns the queries of every reader.
    pub fn reads(&self) -> BTreeMap<RepoPath, BTreeSet<Query>> {
        self.reads.borrow().all().clone()
    }

    /// `document` returns the document with `id` in `knowledge`, when the run
    /// has loaded it. Before the checks run, the run loads every document
    /// that a check looks up. If only the cache knows a document, the run
    /// failed to load it, so a debug build stops with an assertion failure.
    pub fn document(&self, knowledge: usize, id: &str) -> Option<&Document> {
        self.note(Query::Doc {
            knowledge,
            id: id.to_string(),
        });
        let found = self.knowledges.get(knowledge)?.document(id);
        debug_assert!(
            found.is_some() || self.ids(knowledge, id).len() != 1 || cfg!(not(debug_assertions)),
            "the run has not loaded the document `{id}` of knowledge store {knowledge}, and only the cache knows it"
        );
        found
    }

    /// `ids` returns the path of every document and schema of `knowledge`
    /// that has `id`. It takes the paths from the documents that the run has
    /// loaded. It then adds each path that the cache knows, unless the run
    /// has loaded it.
    pub fn ids(&self, knowledge: usize, id: &str) -> Vec<RepoPath> {
        self.note(Query::Doc {
            knowledge,
            id: id.to_string(),
        });
        let mut paths: Vec<RepoPath> = self
            .knowledges
            .get(knowledge)
            .map(|k| k.ids(id).into_iter().cloned().collect())
            .unwrap_or_default();
        if let Some(fallback) = &self.fallback {
            for p in fallback.ids(knowledge, id) {
                if !self.holds(&p) && !paths.contains(&p) {
                    paths.push(p);
                }
            }
            paths.sort();
        }
        paths
    }

    /// `documents_of` returns the documents of type `type_name` in
    /// `knowledge` that the run has loaded.
    pub fn documents_of(&self, knowledge: usize, type_name: &str) -> Vec<&Document> {
        self.note(Query::Type {
            knowledge,
            type_name: type_name.to_string(),
        });
        self.knowledges
            .get(knowledge)
            .map(|k| k.documents_of(type_name).collect())
            .unwrap_or_default()
    }

    /// `ids_of_type` returns the path and id of every document of type
    /// `type_name` in `knowledge`. It takes them from the documents that the
    /// run has loaded. For the paths that the run has not loaded, it adds the
    /// cache's rows.
    pub fn ids_of_type(&self, knowledge: usize, type_name: &str) -> Vec<(RepoPath, String)> {
        let mut out: Vec<(RepoPath, String)> = self
            .documents_of(knowledge, type_name)
            .into_iter()
            .filter_map(|d| d.id().map(|id| (d.path.clone(), id.to_string())))
            .collect();
        if let Some(fallback) = &self.fallback {
            for (p, id) in fallback.ids_of_type(knowledge, type_name) {
                if !self.holds(&p) {
                    out.push((p, id));
                }
            }
            out.sort();
        }
        out
    }

    /// `path_of_id` returns the path of the document or schema with `id` in
    /// `knowledge`, when exactly one document or schema has it. It counts the
    /// documents that the run has loaded and the ones that the cache knows
    /// (CACHE-1_AC-21).
    pub fn path_of_id(&self, knowledge: usize, id: &str) -> Option<RepoPath> {
        match self.ids(knowledge, id).as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        }
    }

    /// `id_of` returns the knowledge store and id of the document or schema
    /// at `path`. It takes them from the document when the run has loaded
    /// it, and from the cache's row for the path otherwise.
    pub fn id_of(&self, path: &RepoPath) -> Option<(usize, String)> {
        self.note(Query::Path(path.clone()));
        if let Some(found) = self.id_at(path) {
            return Some(found);
        }
        let k = self.knowledge_of(path)?;
        let id = self.fallback.as_ref()?.fields(path)?.id?;
        Some((k, id))
    }

    /// `broken_in` returns the paths of the documents of `knowledge` whose
    /// frontmatter doesn't parse. It takes them from the documents that the
    /// run has loaded. For the documents that the run has not loaded, it adds
    /// the paths that the cache lists as broken.
    pub fn broken_in(&self, knowledge: usize) -> Vec<RepoPath> {
        let mut out: Vec<RepoPath> = self
            .knowledges
            .get(knowledge)
            .map(|k| {
                k.documents
                    .iter()
                    .filter(|d| !d.core.present)
                    .map(|d| d.path.clone())
                    .collect()
            })
            .unwrap_or_default();
        if let Some(fallback) = &self.fallback {
            for p in fallback.broken_in(knowledge) {
                if !self.holds(&p) {
                    out.push(p);
                }
            }
        }
        out.sort();
        out
    }

    /// `holds` reports whether the run has loaded a document or schema at
    /// `path`.
    pub fn holds(&self, path: &RepoPath) -> bool {
        self.knowledges.iter().any(|k| k.is_document(path))
    }

    /// `is_document` reports whether `path` is a document of any knowledge
    /// store, whether the run has loaded it or only the cache knows it.
    pub fn is_document(&self, path: &RepoPath) -> bool {
        self.holds(path) || self.fallback.as_ref().is_some_and(|f| f.is_document(path))
    }

    /// `admit` adds to the model a document that the build did not load. The
    /// run has since read the document in full. Such a document is one that the
    /// run checks, or one that a check looks up.
    pub fn admit(&mut self, knowledge: usize, doc: Document) {
        let Some(k) = self.knowledges.get_mut(knowledge) else {
            return;
        };
        if k.by_path.contains_key(&doc.path) {
            return;
        }
        k.insert_document(doc);
    }

    /// `admit_schema` replaces the cache's summary of a schema with the
    /// schema read in full. It keeps the same index, so every reference to
    /// the schema stays valid (CACHE-1_AC-25).
    pub fn admit_schema(&mut self, knowledge: usize, schema: SchemaDocument) {
        let Some(k) = self.knowledges.get_mut(knowledge) else {
            return;
        };
        if let Some(slot) = k.schemas.iter_mut().find(|s| s.doc.path == schema.doc.path) {
            *slot = schema;
        }
    }

    /// `admit_index` adds to the model an index file that the build did not
    /// load. The run reads the index file of a directory when a change
    /// touches the directory, so that the check of the directory can compare
    /// it with the expected index (CACHE-1_AC-18).
    pub fn admit_index(&mut self, knowledge: usize, index: IndexFile) {
        let Some(k) = self.knowledges.get_mut(knowledge) else {
            return;
        };
        if k.indexes.iter().any(|i| i.path == index.path) {
            return;
        }
        k.indexes.push(index);
    }

    /// `exists` reports whether `path` is a file or a directory of the
    /// repository, and records the question as a query (CACHE-1_AC-26).
    pub fn exists(&self, path: &RepoPath) -> bool {
        self.note(Query::Exists(path.clone()));
        self.tree.exists(path)
    }

    /// `loaded_document` returns the knowledge and the document at `path`,
    /// when `path` is a document or a schema that the run loaded.
    pub(crate) fn loaded_document(&self, path: &RepoPath) -> Option<(usize, &Document)> {
        match self.by_path(path)? {
            Located::Document(k, doc) => Some((k, doc)),
            Located::Schema(k, s) if s.loaded => Some((k, &s.doc)),
            _ => None,
        }
    }

    /// `by_path` returns what `path` is in the model.
    pub fn by_path(&self, path: &RepoPath) -> Option<Located<'_>> {
        self.note(Query::Path(path.clone()));
        for (k, knowledge) in self.knowledges.iter().enumerate() {
            if let Some(&i) = knowledge.by_path.get(path) {
                return Some(Located::Document(k, &knowledge.documents[i]));
            }
            if let Some(s) = knowledge.schemas.iter().find(|s| &s.doc.path == path) {
                return Some(Located::Schema(k, s));
            }
            if let Some(i) = knowledge.indexes.iter().find(|i| &i.path == path) {
                return Some(Located::Index(k, i));
            }
            if let Some(a) = knowledge.asset_at(path) {
                return Some(Located::Asset(k, a));
            }
        }
        self.additional
            .iter()
            .find(|a| &a.path == path)
            .map(Located::Additional)
    }

    /// `is_document_of` reports whether `path` is a document of `knowledge`.
    pub fn is_document_of(&self, knowledge: usize, path: &RepoPath) -> bool {
        self.note(Query::Path(path.clone()));
        self.knowledges
            .get(knowledge)
            .is_some_and(|k| k.is_document(path))
            || (self.knowledge_of(path) == Some(knowledge)
                && self.fallback.as_ref().is_some_and(|f| f.is_document(path)))
    }

    /// `is_materialized` reports whether `path` is a document of any
    /// knowledge store or an additional file. An include block that refers to
    /// a file by path can't include such a file.
    pub fn is_materialized(&self, path: &RepoPath) -> bool {
        self.note(Query::Path(path.clone()));
        self.is_document(path) || self.dispatch.contains_key(path)
    }

    /// `is_additional` reports whether a schema's globs match `path` as an
    /// additional file, whether the run has loaded it or not.
    pub fn is_additional(&self, path: &RepoPath) -> bool {
        self.dispatch.contains_key(path)
    }

    /// `admit_additional` adds to the model an additional file that the
    /// build did not load. The run has since read the file in full, because a
    /// check needs it (CACHE-1_AC-20).
    pub fn admit_additional(&mut self, file: AdditionalFile) {
        if self.additional.iter().any(|a| a.path == file.path) {
            return;
        }
        self.additional.push(file);
    }

    /// `is_materialized_in` reports whether `path` is a document of
    /// `knowledge` or an additional file that its schemas match. An include
    /// block in a document of `knowledge` can't include such a file by path
    /// (SOKF_FORMAT_89). `is_materialized_in` treats an additional file of
    /// another knowledge store as an ordinary repository file, which such an
    /// include block can include. With no knowledge store given, the files of
    /// every knowledge store count. To reject an include block that refers by
    /// path to a document or an asset of any knowledge store, a caller checks
    /// the path separately, through `id_at` and `asset_at`.
    // sokf-implements: SOKF_FORMAT_89
    pub fn is_materialized_in(&self, knowledge: Option<usize>, path: &RepoPath) -> bool {
        match knowledge {
            None => self.is_materialized(path),
            Some(k) => {
                self.is_document_of(k, path)
                    || self
                        .dispatch
                        .get(path)
                        .is_some_and(|s| s.is_some_and(|s| s.knowledge == k))
            }
        }
    }

    /// `asset_of` returns the asset that the document at `sidecar`
    /// describes, with its knowledge store.
    pub fn asset_of(&self, sidecar: &RepoPath) -> Option<(usize, &Asset)> {
        self.note(Query::Path(sidecar.clone()));
        self.knowledges
            .iter()
            .enumerate()
            .find_map(|(k, kn)| kn.asset_of(sidecar).map(|a| (k, a)))
    }

    /// `asset_at` returns the asset at `path`, with its knowledge store.
    pub fn asset_at(&self, path: &RepoPath) -> Option<(usize, &Asset)> {
        self.note(Query::Path(path.clone()));
        self.knowledges
            .iter()
            .enumerate()
            .find_map(|(k, kn)| kn.asset_at(path).map(|a| (k, a)))
    }

    /// `content_path` returns the path that the id of a document refers to
    /// (FORMAT-2_AC-1): the path of the asset for a sidecar, and the path of
    /// the document otherwise. The resolution of an id doesn't change: an id
    /// still resolves to its document. Only code that needs the content calls
    /// `content_path`. For example, the code that builds the definition of a
    /// label without a fragment calls it.
    pub fn content_path(&self, document: &RepoPath) -> RepoPath {
        self.asset_of(document)
            .map(|(_, a)| a.path.clone())
            .unwrap_or_else(|| document.clone())
    }

    /// `schema` returns the schema at `r`.
    pub fn schema(&self, r: SchemaRef) -> &SchemaDocument {
        &self.knowledges[r.knowledge].schemas[r.index]
    }

    /// `schema_query` returns the query that stands for a read of the schema
    /// at `r`. The query uses the key that `schema_key` returns. The key is
    /// the value of the schema's `type` const, or, for a schema that
    /// dispatches only by its globs, the schema's path (CACHE-1_AC-15).
    pub fn schema_query(&self, r: SchemaRef) -> Query {
        Query::Schema {
            knowledge: r.knowledge,
            key: self.knowledges[r.knowledge].schema_key(r.index),
        }
    }

    /// `knowledge_of` returns the knowledge store whose root contains `path`,
    /// the innermost one when roots nest.
    pub fn knowledge_of(&self, path: &RepoPath) -> Option<usize> {
        crate::walk::innermost(self.knowledges.iter().map(|k| &k.root), path)
    }

    /// `is_unit` reports whether `path` is a unit. A unit is a file that a
    /// check produces output for: a document, a schema document, or an
    /// additional file matched by a schema's globs (CACHE-1_AC-24). It searches the
    /// walk's sorted list, so it does not build a set of all paths.
    pub fn is_unit(&self, path: &RepoPath) -> bool {
        self.tree
            .file(path)
            .is_some_and(|f| matches!(f.kind, FileKind::Document | FileKind::Schema))
            || self.dispatch.contains_key(path)
    }

    /// `is_marker_file` reports whether `path` is a file that the run reads
    /// for markers: a file of the walk outside every knowledge root, when the
    /// run checks markers (SOKF_VALIDATION_50).
    pub fn is_marker_file(&self, path: &RepoPath) -> bool {
        self.marker_check != crate::MarkerCheck::Off
            && self
                .tree
                .file(path)
                .is_some_and(|f| f.root.is_none() && f.kind == FileKind::Candidate)
    }

    /// `document_paths` returns the path of every document and schema
    /// document of every knowledge store that the walk lists, whether the run
    /// has loaded it or not. These are the files that a full run reads.
    pub fn document_paths(&self) -> BTreeSet<RepoPath> {
        self.tree
            .files
            .iter()
            .filter(|f| matches!(f.kind, FileKind::Document | FileKind::Schema))
            .map(|f| f.path.clone())
            .collect()
    }

    /// `documents` returns every document of every knowledge store, with the
    /// index of its knowledge store.
    pub fn documents(&self) -> impl Iterator<Item = (usize, &Document)> {
        self.knowledges
            .iter()
            .enumerate()
            .flat_map(|(k, kn)| kn.documents.iter().map(move |d| (k, d)))
    }

    /// `documents_and_schemas` returns every document and every schema
    /// document of every knowledge store, with the index of its knowledge
    /// store.
    pub fn documents_and_schemas(&self) -> impl Iterator<Item = (usize, &Document)> {
        self.knowledges.iter().enumerate().flat_map(|(k, kn)| {
            kn.documents
                .iter()
                .chain(kn.schemas.iter().map(|s| &s.doc))
                .map(move |d| (k, d))
        })
    }
}
