//! The loaders walk the repository, sync the cache, and build the model
//! that a command works on, and they read every file through the cache. A
//! load for a check also computes the dirty set, the documents that the
//! check runs on.

use std::collections::BTreeSet;

use crate::{
    Cache, Compiled, Config, Document, Error, Expected, Model, RepoPath, RepoRoot, Result, Source,
    Tree,
    cache::DocRow,
    check, config,
    grammar::{self, FormatRegistry, Grammar, RegexCache},
    include,
    model::{self, KeyIndex, Loader, ParseMode, Query},
    source, walk,
};

use super::{
    admit::{admit_for_check, admit_full, admit_indexes, needed_slugs, unit_paths},
    dirty::{additional_rows, changed_queries, key_query, schema_rows, scope_name},
    markers::{is_checked_unit, scan_outside},
};

/// `Working` selects which documents a cached loader passes to the model
/// builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Working {
    /// Every document, read in full.
    All,
    /// The loader reads only the changed documents, and the store's rows
    /// stand in for the others until a check needs one.
    Changed,
}

/// `CachedLoader` reads files through the cache: on each read, it records
/// the file's source key in the cache.
#[derive(Debug)]
pub struct CachedLoader<'a> {
    root: &'a RepoRoot,
    cache: &'a Cache,
    working: Working,
}

impl<'a> CachedLoader<'a> {
    /// `new` makes a loader that reads under `root` through `cache` and
    /// passes every document to the model builder.
    pub fn new(root: &'a RepoRoot, cache: &'a Cache) -> Self {
        CachedLoader {
            root,
            cache,
            working: Working::All,
        }
    }

    /// `changed_only` makes a loader that passes only the changed documents
    /// to the model builder.
    pub fn changed_only(root: &'a RepoRoot, cache: &'a Cache) -> Self {
        CachedLoader {
            root,
            cache,
            working: Working::Changed,
        }
    }

    /// `stands_in` returns whether the store's row stands in for the file at
    /// `path`: the loader reads only the changed files, and the file is
    /// unchanged.
    fn stands_in(&self, path: &RepoPath) -> bool {
        self.working == Working::Changed && !self.cache.must_read(path)
    }
}

impl Loader for CachedLoader<'_> {
    // sokf-implements: CACHE-1_AC-2
    // sokf-implements: CACHE-1_AC-3
    fn load(&mut self, path: &RepoPath) -> Result<Source> {
        let source = source::load(self.root, path).inspect_err(|e| {
            if is_not_found(e) {
                self.cache.note_missing();
            }
        })?;
        self.cache.observe(path, &source.key);
        Ok(source)
    }

    /// `document` returns `None` for an unchanged document when the loader
    /// reads only the changed documents, because the store's row stands in
    /// for it.
    /// Otherwise it reads and parses the document, and records its row for
    /// the commit.
    // sokf-implements: CACHE-3_AC-1
    fn document(&mut self, path: &RepoPath) -> Result<Option<Document>> {
        if self.stands_in(path) {
            return Ok(None);
        }
        self.full_document(path).map(Some)
    }

    /// `schema` returns `None` for an unchanged schema when the loader reads
    /// only the changed files. A check that needs the compiled schema loads
    /// the schema into the model.
    // sokf-implements: CACHE-1_AC-25
    // sokf-implements: CACHE-1_AC-42
    fn schema(&mut self, path: &RepoPath) -> Result<Option<Document>> {
        if self.stands_in(path) {
            return Ok(None);
        }
        self.full_document(path).map(Some)
    }

    /// `additional` returns `None` for an unchanged additional file when the
    /// loader reads only the changed files. The file's check reads the file
    /// when the file is dirty.
    // sokf-implements: CACHE-1_AC-20
    fn additional(&mut self, path: &RepoPath) -> Result<Option<Source>> {
        if self.stands_in(path) {
            return Ok(None);
        }
        self.load(path).map(Some)
    }

    /// `index` returns `None` for an unchanged index file when the loader
    /// reads only the changed files. The check of its directory reads the
    /// index file when a change touched the directory.
    // sokf-implements: CACHE-1_AC-19
    fn index(&mut self, path: &RepoPath) -> Result<Option<crate::model::IndexFile>> {
        if self.stands_in(path) {
            return Ok(None);
        }
        let source = self.load(path)?;
        Ok(Some(crate::model::IndexFile {
            path: path.clone(),
            source,
        }))
    }

    fn full_document(&mut self, path: &RepoPath) -> Result<Document> {
        let source = self.load(path)?;
        let parsed = self.parse(&source, ParseMode::Document);
        let doc = Document::new(source, parsed);
        let knowledge = self.cache.knowledge_of(path).unwrap_or(0);
        self.cache.record_row(path, DocRow::of(knowledge, &doc));
        Ok(doc)
    }
}

// sokf:begin loaded
/// `Loaded` holds everything that a run derives from the tree before the run
/// checks any document, and records what the run checks.
pub struct Loaded {
    /// The model: the documents to check and the documents that their
    /// checks read, in full.
    pub model: Model,
    /// The compiled schemas.
    pub compiled: Compiled,
    /// The captured keys: this run's keys for the documents it checks, and
    /// the store's keys for the others.
    pub keys: KeyIndex,
    /// The expected generated content of the files that the run checks.
    pub expected: Expected,
    /// The documents that this run checks: every document in a full run,
    /// otherwise the dirty set.
    pub dirty: BTreeSet<RepoPath>,
    /// The directories whose index this run computes and checks: the
    /// directories that a change touched, or `None` for every directory.
    pub touched: Option<BTreeSet<RepoPath>>,
    /// Whether this run checks every document.
    pub full: bool,
}
// sokf:end loaded

/// `load_model` walks the repository, syncs the cache, and builds the model
/// through the cache, reading and parsing every document. The caller records
/// the results that it derives from the model, then commits the cache.
// sokf-implements: CACHE-1_AC-27
pub fn load_model(root: &RepoRoot, config: &Config, cache: &Cache) -> Result<Model> {
    let tree = walk_tree(root, config, cache)?;
    cache.sync(&tree);
    model::build(tree, &mut CachedLoader::new(root, cache))
}

/// `walk_tree` returns the file tree that this run works on. A hint is the
/// list of changed paths that a caller passes in. Without a hint,
/// `walk_tree` walks the repository. With a hint, it starts from the store's
/// rows and lists each hinted path again by the walk's rules.
// sokf-implements: CACHE-1_AC-23
fn walk_tree(root: &RepoRoot, config: &Config, cache: &Cache) -> Result<Tree> {
    cache.begin_run();
    match cache.hint() {
        Some(changed) => cache.tree_from_store(root, config, &changed),
        None => walk::collect(root, config),
    }
}

/// `load_working` walks the repository, syncs the cache, and builds the
/// model from only the changed documents, with the store's rows standing in
/// for the others. When the store has no rows to stand in for them,
/// `load_working` reads every document.
// sokf-implements: CACHE-3_AC-1
// sokf-implements: CACHE-3_AC-2
pub fn load_working(root: &RepoRoot, config: &Config, cache: &Cache) -> Result<Model> {
    let tree = walk_tree(root, config, cache)?;
    cache.sync(&tree);
    match cache.view() {
        Some(view) => model::build_with(
            tree,
            &mut CachedLoader::changed_only(root, cache),
            Some(view),
        ),
        None => model::build(tree, &mut CachedLoader::new(root, cache)),
    }
}

/// `load_with_keys` returns the model with every key captured, for a
/// lookup under `--no-cache`. Such a lookup runs no check.
pub fn load_with_keys(
    root: &RepoRoot,
    config: &Config,
    cache: &Cache,
) -> Result<(Model, KeyIndex)> {
    let (grammar, repo_formats) = load_formats(root)?;
    let model = load_model(root, config, cache)?;
    let compiled = grammar::compile_all(&model, grammar, repo_formats);
    let keys = check::capture_keys_of(&model, &compiled, None);
    Ok((model, keys))
}

/// `load_formats` loads the grammar and the repository's formats file.
pub(super) fn load_formats(root: &RepoRoot) -> Result<(Grammar, Option<FormatRegistry>)> {
    let grammar = grammar::load_grammar()?;
    let repo_formats = config::load_repo_formats(root, &grammar, &RegexCache::new())?;
    Ok((grammar, repo_formats))
}

/// When the cache is on, `load_checked` runs the check that comes before a
/// lookup. It checks the changes since the last check, as `validate` does,
/// commits the result, and drops the findings. The store then holds the
/// results of a check of the current tree, so the next command doesn't
/// repeat the check. With `--no-cache` it returns `None`, because the run
/// then uses a new, empty store that no later command reads.
// sokf-implements: CACHE-1_AC-34
// sokf-implements: CACHE-1_AC-35
// sokf-implements: CACHE-1_AC-36
pub fn load_checked(root: &RepoRoot, config: &Config, cache: &Cache) -> Result<Option<Loaded>> {
    if !cache.is_on() {
        return Ok(None);
    }
    let (grammar, repo_formats) = load_formats(root)?;
    let (loaded, _) = super::validate::check_and_keep(
        root,
        config,
        &grammar,
        repo_formats.as_ref(),
        cache,
        false,
        false,
    )?;
    Ok(Some(loaded))
}

/// `load_for_lookup` returns the model that a lookup reads. With the cache
/// on, this is the model of the check that `load_checked` ran: it holds the
/// changed documents, and the store's rows stand in for the others. With
/// `--no-cache`, `load_for_lookup` reads every document.
// sokf-implements: CACHE-1_AC-21
// sokf-implements: CACHE-1_AC-37
pub fn load_for_lookup(root: &RepoRoot, config: &Config, cache: &Cache) -> Result<Model> {
    match load_checked(root, config, cache)? {
        Some(loaded) => Ok(loaded.model),
        None => load_model(root, config, cache),
    }
}

/// `load_keys_for` returns the model that a lookup reads, with the keys that
/// a command resolves. With the cache on, the model is the one that
/// `load_checked` checked, and the keys are the store's keys for `slugs`.
/// Those keys are current, because `load_checked` commits its check before
/// it returns. With `--no-cache`,
/// `load_keys_for` captures every key.
// sokf-implements: CACHE-1_AC-21
pub fn load_keys_for(
    root: &RepoRoot,
    config: &Config,
    cache: &Cache,
    slugs: &BTreeSet<String>,
) -> Result<(Model, KeyIndex)> {
    let Some(loaded) = load_checked(root, config, cache)? else {
        return load_with_keys(root, config, cache);
    };
    Ok((loaded.model, cache.stored_keys_for(slugs, &BTreeSet::new())))
}

/// `load_repository` loads everything that a full run needs: it walks the
/// repository, loads every document, builds the model, compiles the
/// schemas, captures the keys, and expands the generated content.
pub fn load_repository(
    root: &RepoRoot,
    config: &Config,
    grammar: &Grammar,
    repo_formats: Option<&FormatRegistry>,
    cache: &Cache,
) -> Result<Loaded> {
    load_for_check(root, config, grammar, repo_formats, cache, true)
}

/// `file_hash` returns the FNV-1a hash of a repository file's bytes, or 0
/// when the file is absent.
fn file_hash(root: &RepoRoot, path: &RepoPath) -> u64 {
    std::fs::read(path.to_fs(root))
        .map(|bytes| source::fnv1a64(&bytes))
        .unwrap_or(0)
}

/// `load_for_check` walks the repository, syncs the cache, builds the
/// model, and compiles the schemas. Then it loads the documents to check
/// and captures their keys. A full run checks every document. Any other
/// run checks the dirty set. A reader of a fact is a document whose check
/// reads that fact. The changed documents and schemas are dirty. The
/// readers of each fact, schema, or identity that changed are dirty. The
/// readers of each schema that uses a format or type that changed in the
/// formats file are dirty. A pass compares the keys of the new dirty
/// documents with the store's keys, and the readers of each key that
/// differs become dirty. Passes repeat until one adds no new document. A
/// run is full when the caller asks for one or when the store is new.
// sokf-implements: CLI-4_AC-20
// sokf-implements: CACHE-1_AC-4
// sokf-implements: CACHE-1_AC-5
// sokf-implements: CACHE-1_AC-15
// sokf-implements: CACHE-1_AC-16
// sokf-implements: CACHE-1_AC-17
// sokf-implements: CACHE-1_AC-40
// sokf-implements: CACHE-1_AC-41
pub(super) fn load_for_check(
    root: &RepoRoot,
    config: &Config,
    grammar: &Grammar,
    repo_formats: Option<&FormatRegistry>,
    cache: &Cache,
    want_full: bool,
) -> Result<Loaded> {
    let mut model = if want_full {
        load_model(root, config, cache)?
    } else {
        load_working(root, config, cache)?
    };
    let mut loader = CachedLoader::new(root, cache);
    let identities_changed = cache.identities_changed(model.identity_rows());
    let full = want_full || cache.is_fresh();
    model.marker_check = config.markers;
    let scanned = scan_outside(&mut model, cache, root, full);
    let mut compiled = grammar::compile_all(&model, grammar.clone(), repo_formats.cloned());
    let formats_changed = cache.formats_changed(file_hash(root, &config::formats_path()), || {
        grammar::fingerprint(&compiled.registry)
    });
    let schemas_changed = cache.schemas_changed(schema_rows(&model, cache));
    let additional_changed = cache.additional_changed(additional_rows(&model));
    let mut touched: Option<BTreeSet<RepoPath>> = None;
    let (dirty, keys) = if full {
        let all = unit_paths(&model);
        admit_full(&mut model, &mut loader, cache, &all)?;
        ensure_not_missed(cache)?;
        let keys = check::capture_keys_of(&model, &compiled, None);
        model.plans = check::capture_plans_of(&model, &compiled, None);
        (all, keys)
    } else {
        let mut dirs: BTreeSet<RepoPath> = BTreeSet::new();
        let mut queries = changed_queries(&model, cache, root, &mut dirs);
        for (knowledge, key) in schemas_changed {
            queries.insert(Query::Schema { knowledge, key }.key());
        }
        if identities_changed {
            queries.insert(Query::Identities.key());
        }
        // After an edit to the formats file, query the schemas that use a
        // changed format or type. The documents that those schemas govern
        // read the schemas, so those documents become dirty too.
        // sokf-implements: CACHE-1_AC-17
        if let Some(names) = &formats_changed {
            for (k, knowledge) in model.knowledges.iter().enumerate() {
                for (i, schema) in knowledge.schemas.iter().enumerate() {
                    if schema.uses.iter().any(|u| names.contains(u)) {
                        queries.insert(
                            Query::Schema {
                                knowledge: k,
                                key: knowledge.schema_key(i),
                            }
                            .key(),
                        );
                    }
                }
            }
        }
        admit_indexes(&mut model, &mut loader, cache, &dirs);
        touched = Some(dirs);
        // Only the files whose content changed are dirty. The sync reads a
        // document again when its modification time lies within
        // `RACY_MARGIN` of the last sync. Such a document stays clean when
        // its content is the same.
        let changed = cache.changed_set();
        let mut dirty: BTreeSet<RepoPath> = model
            .documents_and_schemas()
            .map(|(_, d)| &d.path)
            .chain(model.additional.iter().map(|a| &a.path))
            .filter(|p| changed.contains(*p))
            .cloned()
            .collect();
        // A file whose markers changed is dirty, so its check runs.
        dirty.extend(scanned);
        // A file that a glob now matches is dirty, so its check runs. For a
        // file that a glob no longer matches, the loader clears its output
        // and its edges in the cache. The commit deletes every row of a file
        // that the walk no longer lists, so the loader records nothing for it.
        // sokf-implements: CACHE-1_AC-52
        for path in additional_changed {
            queries.insert(Query::Path(path.clone()).key());
            if model.is_additional(&path) {
                dirty.insert(path);
            } else if model.tree.file(&path).is_some() {
                cache.record_output(&path, Vec::new());
                cache.record_edges(&path, Vec::new());
            }
        }
        // The readers of the changed queries are dirty, except a reader that
        // the walk no longer lists as a unit, because it has no check.
        dirty.extend(
            cache
                .readers_of(&queries)
                .into_iter()
                .filter(|p| is_checked_unit(&model, cache, p)),
        );
        // When a format changed, was added, or was removed, every schema
        // document is dirty, because its own check reads the examples of
        // the whole registry.
        // sokf-implements: CACHE-1_AC-40
        if formats_changed
            .as_ref()
            .is_some_and(|names| names.iter().any(|n| n.starts_with("f:")))
        {
            dirty.extend(
                model
                    .knowledges
                    .iter()
                    .flat_map(|k| k.schemas.iter().map(|s| s.doc.path.clone()))
                    .filter(|p| is_checked_unit(&model, cache, p)),
            );
        }
        // Compare the keys of the dirty documents with the store's keys: a
        // key that differs from the store's makes its readers dirty. The
        // loop repeats until no new document is dirty.
        let mut captured = KeyIndex::default();
        let mut planned = crate::model::PlanIndex::default();
        let mut seen: BTreeSet<RepoPath> = BTreeSet::new();
        loop {
            let wave: BTreeSet<RepoPath> = dirty.difference(&seen).cloned().collect();
            if wave.is_empty() {
                break;
            }
            admit_for_check(&mut model, &mut loader, cache, &mut compiled, &wave)?;
            let fresh = check::capture_keys_of(&model, &compiled, Some(&wave));
            let mut moved: BTreeSet<String> = BTreeSet::new();
            for path in &wave {
                let old = cache.old_keys_of(path);
                let new: BTreeSet<(String, String, usize)> = fresh
                    .of_document(path)
                    .map(|e| {
                        (
                            scope_name(&e.scope),
                            crate::markdown::slug(&e.key),
                            e.knowledge,
                        )
                    })
                    .collect();
                for (scope, slug, k) in old.symmetric_difference(&new) {
                    moved.insert(key_query(scope, slug, *k));
                }
            }
            captured.extend(fresh.document.into_values().flatten());
            // A path that a document starts or stops planning makes the
            // readers of that path dirty (SOKF_SCHEMA_53).
            let plans = check::capture_plans_of(&model, &compiled, Some(&wave));
            for path in &wave {
                let new = plans.of(path);
                for target in cache.old_plans_of(path).symmetric_difference(&new) {
                    moved.insert(Query::Planned(target.clone()).key());
                }
                for target in &new {
                    planned.add(path, target);
                }
            }
            seen.extend(wave);
            let more: BTreeSet<RepoPath> = cache
                .readers_of(&moved)
                .into_iter()
                .filter(|p| is_checked_unit(&model, cache, p) && !dirty.contains(p))
                .collect();
            dirty.extend(more);
        }
        let stored_plans = cache.stored_plans(&dirty);
        for target in stored_plans.targets() {
            for planner in stored_plans.planners(&target) {
                planned.add(&planner, &target);
            }
        }
        model.plans = planned;
        let slugs = needed_slugs(&model, &captured);
        let stored = cache.stored_keys_for(&slugs, &dirty);
        captured.extend(stored.document.into_values().flatten());
        (dirty, captured)
    };
    let expected = if full {
        include::expected(&model, &keys)
    } else {
        include::expected_for(&model, &keys, Some(&dirty))
    };
    Ok(Loaded {
        model,
        compiled,
        keys,
        expected,
        dirty,
        touched,
        full,
    })
}

/// `is_not_found` returns whether `e` is the I/O error of a missing file.
pub(super) fn is_not_found(e: &crate::Error) -> bool {
    matches!(e, crate::Error::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound)
}

/// `ensure_not_missed` stops a run with an error when its hint left out a
/// removed file that the run read. `check_and_keep` and the fix loop of
/// `validate` catch the error and run again without the hint, so it never
/// reaches the caller.
pub(super) fn ensure_not_missed(cache: &Cache) -> Result<()> {
    if cache.missed() {
        return Err(Error::Internal("the hint left out a removed file".into()));
    }
    Ok(())
}
