//! This module implements `sokf validate`. It loads the configuration,
//! walks the tree, loads and builds the model, compiles the schemas, and
//! works out the expected content, then checks the documents and the
//! expected indexes. With `--fix`, it plans the fixes, applies them, and
//! runs all of that again. Last, it finishes the report and filters it to
//! the paths that the caller gave.

use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
};

use crate::{
    Cache, CacheMode, Config, Error, Expr, Finding, RepoPath, RepoRoot, Report, Result,
    cache::CitedFact,
    check, config, fix,
    grammar::{FormatRegistry, Grammar},
    include, index,
    model::{KeyIndex, Query},
};

use super::{
    admit::{admit_closure, admit_for_check, admit_full, admit_schemas_for, needed_slugs},
    load::{CachedLoader, Loaded, ensure_not_missed, load_for_check, load_formats},
    markers::is_checked_unit,
};

/// The options of `sokf validate`.
#[derive(Debug, Clone)]
pub struct ValidateOptions {
    /// `PATHS`: the paths that the report is filtered to.
    pub paths: Vec<PathBuf>,
    /// The current directory, which `PATHS` resolve from.
    pub cwd: PathBuf,
    /// `--fix`: fix what the tool can fix, then check again.
    pub fix: bool,
    /// The cache mode, which `--no-cache` sets to `Off`.
    pub cache: CacheMode,
    /// `--query`: the expression that projects the report.
    pub query: Option<Expr>,
    /// `--changed`: the files that the caller knows have changed, each a
    /// path from the current directory or from the repository root. The
    /// cache reads the file metadata of only these files (CLI-4_AC-21).
    pub changed: Vec<PathBuf>,
}

/// `changed_paths` resolves the paths of `--changed`, each a path from the
/// current directory or from the repository root, whether the file still
/// exists or has been deleted. A path outside the repository is an error.
// sokf-implements: CLI-4_AC-22
fn changed_paths(root: &RepoRoot, cwd: &Path, paths: &[PathBuf]) -> Result<Vec<RepoPath>> {
    paths
        .iter()
        .map(|path| {
            let shown = path.display().to_string();
            path.to_str()
                .and_then(|s| crate::PathArg(s).resolve_new(root, cwd))
                .ok_or(Error::Path(shown))
        })
        .collect()
}

/// `check_repository` loads the knowledge for a check and checks it. It
/// checks the documents of the dirty set, then the readers of each
/// citation that changed. It also runs the checks that run every time,
/// whatever documents are dirty. It stores what the run derived in the
/// cache, and returns every finding: those of the documents it checked, and
/// the cache's stored findings for the documents it didn't check.
// sokf-implements: CACHE-1_AC-14
// sokf-implements: CACHE-1_AC-11
// sokf-implements: CLI-4_AC-20
pub fn check_repository(
    root: &RepoRoot,
    config: &Config,
    grammar: &Grammar,
    repo_formats: Option<&FormatRegistry>,
    cache: &Cache,
    want_full: bool,
) -> Result<(Loaded, Vec<Finding>)> {
    check_and_keep(root, config, grammar, repo_formats, cache, want_full, true)
}

/// `check_and_keep` runs the check of `check_repository` and commits what
/// it derived to the cache. With `report`, it returns the findings of the
/// check and the cache's stored findings for the units it didn't check.
/// Without `report`, it returns no findings. A lookup, such as `graph` or
/// `locate`, calls it this way: the lookup first runs a check to bring the
/// cache up to date, and it reports no finding from that check
/// (CACHE-1_AC-35).
pub(crate) fn check_and_keep(
    root: &RepoRoot,
    config: &Config,
    grammar: &Grammar,
    repo_formats: Option<&FormatRegistry>,
    cache: &Cache,
    want_full: bool,
    report: bool,
) -> Result<(Loaded, Vec<Finding>)> {
    let run = || {
        check_once(
            root,
            config,
            grammar,
            repo_formats,
            cache,
            want_full,
            report,
        )
    };
    let result = run();
    if hint_missed(root, cache, result.as_ref().err()) {
        return run();
    }
    result
}

/// `hint_missed` returns whether the hint of this run left out a removed
/// file that the run read: the run took the file from the store and found
/// it gone. The cache's loader records such a read, and `e`, the error
/// that ended the run, can show one too. In that case `hint_missed` drops
/// the hint, so that the run that follows walks the repository. The run
/// that follows syncs again, which clears the state of this run.
// sokf-implements: CACHE-1_AC-53
fn hint_missed(root: &RepoRoot, cache: &Cache, e: Option<&Error>) -> bool {
    if cache.hint().is_none() || cache.is_fresh() {
        return false;
    }
    let read_removed = e.is_some_and(|e| {
        super::load::is_not_found(e)
            && matches!(e, Error::Io { path, .. } if RepoPath::from_fs(root, path).is_some())
    });
    let missed = cache.missed() || read_removed;
    if missed {
        cache.drop_hint();
    }
    missed
}

/// `check_once` runs one check for `check_and_keep`.
fn check_once(
    root: &RepoRoot,
    config: &Config,
    grammar: &Grammar,
    repo_formats: Option<&FormatRegistry>,
    cache: &Cache,
    want_full: bool,
    report: bool,
) -> Result<(Loaded, Vec<Finding>)> {
    let mut loaded = load_for_check(root, config, grammar, repo_formats, cache, want_full)?;
    // `indexes` holds the expected index of each touched directory, worked
    // out from the tree as it stands, with no moves applied.
    let indexes = index::expected_for(&loaded.model, &[], loaded.touched.as_ref());
    let stored_citations = |units: &BTreeSet<RepoPath>, checked: &BTreeSet<RepoPath>| {
        let mut rows = Vec::new();
        for unit in units {
            for c in cache.stored_citations_of(unit, checked) {
                rows.push((unit.clone(), c));
            }
        }
        rows
    };
    let (units, citations) = if loaded.full {
        (None, Vec::new())
    } else {
        // The units to check are the dirty documents and the indexes of the
        // touched directories.
        let mut units = loaded.dirty.clone();
        units.extend(indexes.expected.keys().cloned());
        (Some(units), stored_citations(&loaded.dirty, &loaded.dirty))
    };
    let mut out = check::run_units(
        &loaded.model,
        &loaded.compiled,
        &loaded.expected,
        &loaded.keys,
        &indexes,
        units.as_ref(),
        citations,
        true,
    );
    if !loaded.full {
        // The citations of some targets changed. This block checks the
        // documents that read those citations, unless this run has checked
        // them already. A
        // citation to the same target that differs only in its fragment,
        // relationship, or marker line still counts as a change.
        let mut moved: BTreeSet<String> = BTreeSet::new();
        for (source, cited) in &out.citations {
            let old = cache.old_citations(source);
            let new: BTreeSet<CitedFact> = cited
                .iter()
                .map(|(target, c)| CitedFact::of(target, c))
                .collect();
            for fact in old.symmetric_difference(&new) {
                moved.insert(Query::Citations(fact.target.clone()).key());
            }
        }
        let checked: BTreeSet<RepoPath> = out.documents.keys().cloned().collect();
        let round2: BTreeSet<RepoPath> = cache
            .readers_of(&moved)
            .into_iter()
            .filter(|p| is_checked_unit(&loaded.model, cache, p) && !checked.contains(p))
            .collect();
        if !round2.is_empty() {
            admit_for_check(
                &mut loaded.model,
                &mut CachedLoader::new(root, cache),
                cache,
                &mut loaded.compiled,
                &round2,
            )?;
            // The block captures the keys of the documents checked again, and
            // adds them to `loaded.keys`. Their coverage check reads these keys, and the
            // store records each document's keys from `loaded.keys`.
            // `loaded.keys` may already hold a key of a document, because the
            // store returned the key for a slug that a reference needs. Then
            // the stored key stays, and the captured key isn't added a second
            // time.
            let own = check::capture_keys_of(&loaded.model, &loaded.compiled, Some(&round2));
            let held: HashSet<(RepoPath, String)> = own
                .document
                .keys()
                .filter(|k| loaded.keys.document.contains_key(*k))
                .cloned()
                .collect();
            loaded.keys.extend(
                own.document
                    .into_iter()
                    .filter(|(k, _)| !held.contains(k))
                    .flat_map(|(_, entries)| entries),
            );
            let slugs = needed_slugs(&loaded.model, &KeyIndex::default());
            let stored = cache.stored_keys_for(&slugs, &checked);
            for entries in stored.document.into_values() {
                for e in entries {
                    if !loaded
                        .keys
                        .document
                        .contains_key(&(e.path.clone(), crate::markdown::slug(&e.key)))
                    {
                        loaded.keys.add(e);
                    }
                }
            }
            let more = include::expected_for(&loaded.model, &loaded.keys, Some(&round2));
            loaded.expected.includes.extend(more.includes);
            loaded.expected.definitions.extend(more.definitions);
            // The citations of the documents checked again are the store's
            // citations, plus the citations that the first round found. The
            // store's citations leave out those from the documents that this
            // run checked, because the run found those citations itself.
            let excluding: BTreeSet<RepoPath> = checked.union(&round2).cloned().collect();
            let mut citations = stored_citations(&round2, &excluding);
            for cited in out.citations.values() {
                citations.extend(
                    cited
                        .iter()
                        .filter(|(target, _)| round2.contains(target))
                        .cloned(),
                );
            }
            let second = check::run_units(
                &loaded.model,
                &loaded.compiled,
                &loaded.expected,
                &loaded.keys,
                &indexes,
                Some(&round2),
                citations,
                false,
            );
            out.documents.extend(second.documents);
            out.citations.extend(second.citations);
            loaded.dirty.extend(round2);
        }
    }
    // Store what this run derived, for the next run to use: the findings of
    // each unit, the unit's edges for `graph`, and the queries that the run
    // made for the unit. Those queries include the ones made while the run
    // worked out the unit's edges.
    let checked: BTreeSet<RepoPath> = out.documents.keys().cloned().collect();
    for (path, findings) in &out.documents {
        cache.record_output(path, findings.clone());
        if loaded.model.holds(path) || loaded.model.is_additional(path) {
            loaded.model.watch(path);
            cache.record_edges(path, crate::graph::edges_of(&loaded.model, path));
            loaded.model.unwatch();
        }
        cache.record_deps(path, &loaded.model.reads_of(path));
    }
    for (source, cited) in &out.citations {
        let rows = cited
            .iter()
            .map(|(target, c)| crate::cache::CitationRow {
                target: target.clone(),
                slug: c.slug.clone(),
                source: c.source.clone(),
                source_type: c.source_type.clone(),
                rel: c.rel.clone(),
                line: c.line,
            })
            .collect();
        cache.record_citations(source, rows);
    }
    cache.record_keys_of(&loaded.keys, &checked);
    cache.record_plans_of(&loaded.model.plans, &checked);
    for queries in loaded.model.reads().values() {
        for q in queries {
            if let Query::Exists(p) = q
                && !cache.is_walked(p)
            {
                cache.record_probe(p, loaded.model.tree.exists(p));
            }
        }
    }
    ensure_not_missed(cache)?;
    cache.commit();
    if !report {
        return Ok((loaded, Vec::new()));
    }
    let mut findings = out.findings();
    if !loaded.full {
        findings.extend(cache.stored_outputs(&checked));
    }
    Ok((loaded, findings))
}

/// `prepare_fix` loads in full every document that a fix might rewrite: the
/// documents with a finding, and the readers of each document that a fix
/// moves. It loads those readers because their definition blocks contain the
/// old path. It then loads the documents that these documents read. Last, it
/// works out the keys and the expected content that the plan reads. `prepare_fix`
/// returns the documents that the plan covers, or `None` for a full run.
// sokf-implements: SOKF_VALIDATION_19
fn prepare_fix(
    root: &RepoRoot,
    cache: &Cache,
    loaded: &mut Loaded,
    findings: &[Finding],
) -> Result<Option<BTreeSet<RepoPath>>> {
    if loaded.full {
        return Ok(None);
    }
    let mut fix_set: BTreeSet<RepoPath> = findings
        .iter()
        .filter(|f| loaded.model.is_unit(&f.path))
        .map(|f| f.path.clone())
        .collect();
    // The plan also covers every index that has a finding, because the plan
    // reads those indexes.
    let indexes: BTreeSet<RepoPath> = findings
        .iter()
        .filter(|f| f.path.file_name() == "index.md" && !loaded.model.is_unit(&f.path))
        .map(|f| f.path.clone())
        .collect();
    // A move changes the path of a document, and the readers of that
    // document refer to the old path, so the loop collects the queries that
    // find those readers.
    let mut moved: BTreeSet<String> = BTreeSet::new();
    for f in findings {
        if !f.message.contains("its lifecycle states place it in") || !loaded.model.is_unit(&f.path)
        {
            continue;
        }
        moved.insert(Query::Path(f.path.clone()).key());
        moved.insert(Query::Body(f.path.clone()).key());
        if let Some(old) = cache.old_row(&f.path)
            && let Some(id) = old.id
        {
            moved.insert(
                Query::Doc {
                    knowledge: old.knowledge,
                    id,
                }
                .key(),
            );
        }
        if let Some((_, asset)) = loaded.model.asset_of(&f.path) {
            moved.insert(Query::Path(asset.path.clone()).key());
        }
    }
    fix_set.extend(
        cache
            .readers_of(&moved)
            .into_iter()
            .filter(|p| loaded.model.is_unit(p)),
    );
    let mut loader = CachedLoader::new(root, cache);
    admit_full(&mut loaded.model, &mut loader, cache, &fix_set)?;
    // The plan runs over every document and additional file that the model
    // holds so far. These include each file that the run read again, because
    // its modification time was too close to the last sync to trust
    // (`RACY_MARGIN`). The plan also loads the files that each of them
    // reads, and reads those files without rewriting them.
    let held: BTreeSet<RepoPath> = loaded
        .model
        .documents()
        .map(|(_, d)| d.path.clone())
        .chain(loaded.model.additional.iter().map(|a| a.path.clone()))
        .collect();
    admit_closure(&mut loaded.model, &mut loader, cache, &held);
    admit_schemas_for(&mut loaded.model, &mut loader, cache, &held)?;
    ensure_not_missed(cache)?;
    loaded.compiled.compile_more(&loaded.model);
    // The plan captures the keys of every document held in full, and takes
    // the keys of the rest from the store by slug.
    let mut keys = check::capture_keys_of(&loaded.model, &loaded.compiled, Some(&held));
    let slugs = needed_slugs(&loaded.model, &keys);
    keys.extend(
        cache
            .stored_keys_for(&slugs, &held)
            .document
            .into_values()
            .flatten(),
    );
    loaded.expected = include::expected_for(&loaded.model, &keys, Some(&held));
    loaded.keys = keys;
    let mut over = held;
    over.extend(indexes);
    Ok(Some(over))
}

/// `plan_fix` loads what a fix needs and plans the fix.
fn plan_fix(
    root: &RepoRoot,
    cache: &Cache,
    loaded: &mut Loaded,
    findings: &[Finding],
    scope: &config::Scope,
) -> Result<fix::FixPlan> {
    let over = prepare_fix(root, cache, loaded, findings)?;
    fix::plan(
        &loaded.model,
        &loaded.compiled,
        &loaded.expected,
        &loaded.keys,
        scope,
        over.as_ref(),
    )
}

/// `plan_is_stale` returns whether a plan would write where the tree of
/// the run no longer matches the disk: it moves a file that is gone,
/// rewrites a file that is gone, creates a file that the tree lists, or
/// writes into a directory that the tree lists and that is gone. Only a run that
/// took the tree from the store, with a hint, can make such a plan.
// sokf-implements: CACHE-1_AC-54
// sokf-implements: CACHE-1_AC-55
fn plan_is_stale(root: &RepoRoot, model: &crate::Model, plan: &fix::FixPlan) -> bool {
    let moved_to: BTreeSet<&RepoPath> = plan.moves.iter().map(|m| &m.to).collect();
    let listed_dir = |d: &RepoPath| model.tree.dirs.binary_search_by(|x| x.path.cmp(d)).is_ok();
    let gone_dir = |d: &RepoPath| listed_dir(d) && !d.to_fs(root).is_dir();
    plan.moves
        .iter()
        .any(|m| !m.from.to_fs(root).exists() || m.to.parent().is_some_and(|d| gone_dir(&d)))
        || plan
            .rewrites
            .iter()
            .any(|r| !moved_to.contains(&r.path) && !r.path.to_fs(root).exists())
        || plan.creates.iter().any(|c| {
            model.tree.file(&c.path).is_some() || c.path.parent().is_some_and(|d| gone_dir(&d))
        })
}

/// `FIX_PASSES` bounds the passes of one fix, and so the depth of a chain of
/// includes that one run repairs.
const FIX_PASSES: usize = 10;

/// `validate` runs `sokf validate` and returns the report, and the
/// report's projection when `opts.query` is given.
// sokf-implements: CLI-4_AC-14
// sokf-implements: SOKF_VALIDATION_39
// sokf-implements: CLI-4_AC-2
// sokf-implements: CLI-4_AC-5
// sokf-implements: CLI-4_AC-1
// sokf-implements: CLI-4_AC-4
// sokf-implements: CLI-4_AC-22
pub fn validate(
    root: &RepoRoot,
    opts: &ValidateOptions,
) -> Result<(Report, Option<serde_json::Value>)> {
    let config = config::load_config(root)?;
    let (grammar, repo_formats) = load_formats(root)?;
    let mut scope = config::resolve_paths(root, &opts.cwd, &opts.paths)?;
    let cache = Cache::open(root, opts.cache);
    if !opts.changed.is_empty() {
        if opts.cache == CacheMode::Off {
            return Err(Error::Internal(
                "`--changed` needs the cache, so don't use it with `--no-cache`".into(),
            ));
        }
        cache.set_hint(changed_paths(root, &opts.cwd, &opts.changed)?);
    }
    let check = || {
        check_repository(
            root,
            &config,
            &grammar,
            repo_formats.as_ref(),
            &cache,
            false,
        )
    };
    let mut report = Report::default();
    // With `--fix`, the fix plan covers the documents with findings, which
    // `prepare_fix` loads in full after the first check. After the fix,
    // `validate` checks again, and that second check syncs the cache with
    // the files that the fix wrote.
    let (mut loaded, mut findings) = check()?;
    // A pass of the fix expands each include from its source as the source
    // was before the pass, so a source that the pass repairs leaves its
    // includes stale. The fix runs again while it leaves a stale include, and
    // each pass expands from what the last one wrote. A chain of includes
    // takes one pass for each link, up to `FIX_PASSES` (SOKF_VALIDATION_15,
    // SOKF_VALIDATION_37).
    let mut passes = 0;
    while opts.fix && passes < FIX_PASSES {
        passes += 1;
        // The plan can rest on a removed file that the hint left out: a
        // document with a stored finding, or a directory that the store
        // lists. The run then checks the whole repository again before it
        // plans the fix, so that the fix never writes into what was removed.
        let mut plan = plan_fix(root, &cache, &mut loaded, &findings, &scope);
        let stale = matches!(&plan, Ok(p) if cache.hint().is_some()
            && !cache.is_fresh()
            && plan_is_stale(root, &loaded.model, p));
        if stale {
            cache.drop_hint();
        }
        if stale || hint_missed(root, &cache, plan.as_ref().err()) {
            (loaded, findings) = check()?;
            plan = plan_fix(root, &cache, &mut loaded, &findings, &scope);
        }
        let plan = plan?;
        for m in &plan.moves {
            scope.add(m.to.clone());
        }
        let (ops, refused) = fix::apply(root, plan)?;
        // The loop adds the files that the fix wrote to the hint, so that the
        // next check reads their file metadata.
        if let Some(mut hint) = cache.hint() {
            for op in &ops {
                match op {
                    crate::FixOp::Moved { from, to } => {
                        hint.push(from.clone());
                        hint.push(to.clone());
                    }
                    crate::FixOp::Created { path } | crate::FixOp::Rewrote { path } => {
                        hint.push(path.clone());
                    }
                }
            }
            cache.set_hint(hint);
        }
        let wrote = !ops.is_empty();
        report.fixes.extend(ops);
        report.extend(refused);
        (loaded, findings) = check()?;
        if !wrote || !findings.iter().any(|f| f.criterion == "SOKF_FORMAT_85") {
            break;
        }
    }
    report.extend(findings);
    let report = report.finish().filter(&scope);
    let projection = opts.query.as_ref().map(|q| report.project(q)).transpose()?;
    Ok((report, projection))
}
