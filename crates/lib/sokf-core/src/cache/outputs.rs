//! This module saves in the cache the results of a run's checks, so that the
//! next run can read them back. It saves these results:
//!
//! - the output of each document's check
//! - the queries of each check
//! - the citations that each check resolved
//! - whether each path probed outside the walk exists
//! - the asset pairings
//! - the key that each schema claims
//! - the hash and fingerprint of the formats file
//! - the current and former identity of each knowledge store

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::params;

use super::{Cache, conn::Conn, sync::SyncState};
use crate::{
    Finding, RepoPath,
    check::Citation,
    model::{IdentityRow, KeyEntry, KeyIndex, Query},
};

/// `CitationRow` is one citation as the cache keeps it, from `source` to
/// `target`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CitationRow {
    /// The cited document.
    pub target: RepoPath,
    /// The cited key or heading, or `None` for the whole document.
    pub slug: Option<String>,
    /// The citing document.
    pub source: RepoPath,
    /// The type of the citing document, or empty for a marker.
    pub source_type: String,
    /// The relationship of a marker, or `None` for a body link.
    pub rel: Option<String>,
    /// The line of the citation.
    pub line: usize,
}

/// `CitedFact` holds the parts of one citation that the coverage of its
/// target depends on: when a fact changes, the run checks the target again.
/// The fact leaves out the line of a body link, because no coverage finding
/// reads it, but keeps the line of a marker, because the run reports a
/// refused marker at its line.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CitedFact {
    /// The cited document.
    pub target: RepoPath,
    /// The cited key or heading, or `None` for the whole document.
    pub slug: Option<String>,
    /// The type of the citing document, or empty for a marker.
    pub source_type: String,
    /// The relationship of a marker, or `None` for a body link.
    pub rel: Option<String>,
    /// The line of a marker, or `None` for a body link.
    pub line: Option<usize>,
}

impl CitedFact {
    /// `of` returns the parts of citation `c` of `target` that coverage
    /// depends on.
    pub fn of(target: &RepoPath, c: &Citation) -> Self {
        CitedFact {
            target: target.clone(),
            slug: c.slug.clone(),
            source_type: c.source_type.clone(),
            rel: c.rel.clone(),
            line: c.rel.as_ref().map(|_| c.line),
        }
    }
}

/// `AssetRow` is one asset pairing as the cache keeps it, so that a run can
/// find the pairings that changed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetRow {
    /// The knowledge store that holds the asset.
    pub knowledge: usize,
    /// The asset.
    pub asset: RepoPath,
    /// The sidecar, when the pairing has one.
    pub sidecar: Option<RepoPath>,
}

/// `AdditionalRow` is one additional file as the cache keeps it, with the
/// path of the schema whose glob governs the file, or none when no single
/// schema does.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdditionalRow {
    /// The file.
    pub path: RepoPath,
    /// The governing schema's path.
    pub schema: Option<RepoPath>,
}

/// `SchemaRow` is one schema file as the cache keeps it. The row holds the
/// key that the schema claims and the hash of the schema file. The key is
/// either a type const or the path of the schema file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaRow {
    /// The knowledge store that holds the schema.
    pub knowledge: usize,
    /// The schema file.
    pub path: RepoPath,
    /// The key it claims.
    pub key: String,
    /// The hash of the file.
    pub hash: u64,
    /// The `additional-files` globs, one per line.
    pub globs: String,
    /// The `asset-files` globs, one per line.
    pub asset_globs: String,
    /// The names from the registry that the schema uses, one per line.
    pub uses: String,
    /// The line where the schema's body starts.
    pub line: usize,
    /// Which documents of the schema's type are archived, encoded by
    /// `Archived::encode`.
    pub archived: String,
}

impl Cache {
    /// `identities_changed` reports whether the current and former
    /// identities of the knowledge stores differ from the ones that the
    /// cache recorded. The commit writes the new rows (CACHE-1_AC-11).
    // sokf-implements: CACHE-1_AC-16
    pub fn identities_changed(&self, rows: Vec<IdentityRow>) -> bool {
        let old = self.conn().rows(
            "SELECT root, identity, former FROM stores ORDER BY root",
            [],
            |r| {
                Ok(IdentityRow {
                    knowledge: r.get::<_, i64>(0)? as usize,
                    identity: r.get(1)?,
                    former: r.get(2)?,
                })
            },
        );
        let changed = old != rows;
        self.state.borrow_mut().identities = Some(rows);
        changed
    }

    /// `formats_changed` returns the registry entries that an edit of the
    /// formats file changed, or `None` when the file has the same hash as in
    /// the last run. When the hash differs, `fingerprint` computes one hash for
    /// each format and type of the current registry, and
    /// `formats_changed` compares them with the `formats` rows of the cache.
    /// The names whose hashes differ, and the names that were added or
    /// removed, make up the returned set. The commit writes the new hash and
    /// fingerprint (CACHE-1_AC-11).
    // sokf-implements: CACHE-1_AC-17
    pub fn formats_changed(
        &self,
        hash: u64,
        fingerprint: impl FnOnce() -> Vec<(String, u64)>,
    ) -> Option<BTreeSet<String>> {
        let stored = self.conn().meta("formats_hash");
        self.state.borrow_mut().formats_hash = Some(hash);
        if stored == Some(hash) {
            return None;
        }
        let mut old: BTreeMap<String, u64> = self
            .conn()
            .rows("SELECT name, hash FROM formats", [], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
            })
            .into_iter()
            .collect();
        let new = fingerprint();
        let mut changed: BTreeSet<String> = BTreeSet::new();
        for (name, hash) in &new {
            if old.remove(name) != Some(*hash) {
                changed.insert(name.clone());
            }
        }
        changed.extend(old.into_keys());
        self.state.borrow_mut().formats = Some(new);
        Some(changed)
    }

    /// `schemas_changed` returns, as `(knowledge store, key)` pairs, the
    /// keys whose schema file differs from the cached one. A key's schema
    /// file is the file that claims the key. It differs when the schema
    /// changed, was added or removed, or moved to another key. The commit
    /// writes the new rows (CACHE-1_AC-11).
    // sokf-implements: CACHE-1_AC-15
    pub fn schemas_changed(&self, rows: Vec<SchemaRow>) -> BTreeSet<(usize, String)> {
        let old: BTreeSet<SchemaRow> = self
            .conn()
            .rows(
                "SELECT store, path, key, hash, globs, asset_globs, uses, line, archived \
                 FROM schemas",
                [],
                |r| {
                    let Some(path) = RepoPath::parse(&r.get::<_, String>(1)?) else {
                        return Ok(None);
                    };
                    Ok(Some(SchemaRow {
                        knowledge: r.get::<_, i64>(0)? as usize,
                        path,
                        key: r.get(2)?,
                        hash: r.get::<_, i64>(3)? as u64,
                        globs: r.get(4)?,
                        asset_globs: r.get(5)?,
                        uses: r.get(6)?,
                        line: r.get::<_, i64>(7)? as usize,
                        archived: r.get(8)?,
                    }))
                },
            )
            .into_iter()
            .flatten()
            .collect();
        let new: BTreeSet<SchemaRow> = rows.iter().cloned().collect();
        let changed = old
            .symmetric_difference(&new)
            .map(|r| (r.knowledge, r.key.clone()))
            .collect();
        self.state.borrow_mut().schemas = Some(rows);
        changed
    }

    /// `additional_changed` returns the additional files whose governing
    /// schema differs from the cached one: a glob started
    /// or stopped matching the file, or another schema now governs it.
    /// The commit writes the new rows (CACHE-1_AC-11).
    // sokf-implements: CACHE-1_AC-15
    // sokf-implements: CACHE-1_AC-20
    pub fn additional_changed(&self, rows: Vec<AdditionalRow>) -> BTreeSet<RepoPath> {
        let old: BTreeSet<AdditionalRow> = self
            .conn()
            .rows("SELECT path, schema FROM additional", [], |r| {
                let schema = RepoPath::parse(&r.get::<_, String>(1)?);
                Ok(RepoPath::parse(&r.get::<_, String>(0)?)
                    .map(|path| AdditionalRow { path, schema }))
            })
            .into_iter()
            .flatten()
            .collect();
        let new: BTreeSet<AdditionalRow> = rows.iter().cloned().collect();
        let changed = old
            .symmetric_difference(&new)
            .map(|r| r.path.clone())
            .collect();
        self.state.borrow_mut().additional = Some(rows);
        changed
    }

    /// `readers_of` returns every reader of any of `queries`, as the cache
    /// recorded them. The `queries` table interns each query key and each
    /// reader's path, and the `deps` table pairs their ids.
    pub fn readers_of(&self, queries: &BTreeSet<String>) -> BTreeSet<RepoPath> {
        let mut readers = BTreeSet::new();
        let Ok(mut stmt) = self.conn().prepare(
            "SELECT r.key FROM deps d JOIN queries r ON r.id = d.reader \
             WHERE d.query = (SELECT id FROM queries WHERE key = ?1)",
        ) else {
            return readers;
        };
        for q in queries {
            let Ok(rows) = stmt.query_map([q.as_str()], |r| r.get::<_, String>(0)) else {
                continue;
            };
            for reader in rows.flatten() {
                if let Some(p) = RepoPath::parse(&reader) {
                    readers.insert(p);
                }
            }
        }
        readers
    }

    /// `record_output` keeps the output of one check for the commit to
    /// write.
    pub fn record_output(&self, path: &RepoPath, findings: Vec<Finding>) {
        self.state
            .borrow_mut()
            .outputs
            .insert(path.clone(), findings);
    }

    /// `record_deps` keeps the queries that one check made for the commit
    /// to write.
    pub fn record_deps(&self, path: &RepoPath, queries: &BTreeSet<Query>) {
        self.state
            .borrow_mut()
            .deps
            .insert(path.clone(), queries.iter().map(Query::key).collect());
    }

    /// `record_citations` keeps the citations that one document resolved
    /// for the commit to write.
    pub fn record_citations(&self, source: &RepoPath, rows: Vec<CitationRow>) {
        self.state
            .borrow_mut()
            .citations
            .insert(source.clone(), rows);
    }

    /// `record_keys_of` keeps the keys that each document in `checked`
    /// declares in this run. The commit writes them in place of the
    /// document's rows in the cache.
    pub fn record_keys_of(&self, keys: &KeyIndex, checked: &BTreeSet<RepoPath>) {
        let mut rows: BTreeMap<RepoPath, Vec<KeyEntry>> =
            checked.iter().map(|p| (p.clone(), Vec::new())).collect();
        for entries in keys.document.values() {
            for e in entries {
                if let Some(list) = rows.get_mut(&e.path) {
                    list.push(e.clone());
                }
            }
        }
        for list in rows.values_mut() {
            list.sort_by(|a, b| (a.line, &a.key).cmp(&(b.line, &b.key)));
        }
        self.state.borrow_mut().keys_of.extend(rows);
    }

    /// `record_probe` records whether a path outside the walk exists, so
    /// that the commit can write it. The next sync checks the path with
    /// stat again.
    pub fn record_probe(&self, path: &RepoPath, exists: bool) {
        self.state.borrow_mut().probes.insert(path.clone(), exists);
    }

    /// `record_assets` keeps the asset pairings of this run for the commit
    /// to write.
    pub fn record_assets(&self, rows: Vec<AssetRow>) {
        self.state.borrow_mut().assets = Some(rows);
    }

    /// `assets_changed` returns the paths whose pairing differs from the
    /// cached one. Such a path is an asset or a sidecar that gained, lost,
    /// or changed its partner.
    pub fn assets_changed(&self, rows: &[AssetRow]) -> BTreeSet<RepoPath> {
        let old: BTreeSet<AssetRow> = self
            .conn()
            .rows("SELECT store, asset, sidecar FROM assets", [], |r| {
                let knowledge = r.get::<_, i64>(0)? as usize;
                let sidecar = r.get::<_, Option<String>>(2)?;
                Ok(
                    RepoPath::parse(&r.get::<_, String>(1)?).map(|asset| AssetRow {
                        knowledge,
                        asset,
                        sidecar: sidecar.and_then(|s| RepoPath::parse(&s)),
                    }),
                )
            })
            .into_iter()
            .flatten()
            .collect();
        let new: BTreeSet<AssetRow> = rows.iter().cloned().collect();
        let mut changed = BTreeSet::new();
        for row in old.symmetric_difference(&new) {
            changed.insert(row.asset.clone());
            if let Some(s) = &row.sidecar {
                changed.insert(s.clone());
            }
        }
        changed
    }

    /// `probes_changed` returns the probed paths whose existence changed
    /// after the cache recorded it.
    pub fn probes_changed(&self, root: &crate::RepoRoot) -> BTreeSet<RepoPath> {
        self.conn()
            .rows("SELECT path, exists_ FROM probes", [], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .into_iter()
            .filter_map(|(path, answered)| {
                RepoPath::parse(&path).filter(|p| p.to_fs(root).exists() != (answered != 0))
            })
            .collect()
    }

    /// `stored_citations_of` returns the citations of `target` that the
    /// cache holds, except those from removed files and from the sources in
    /// `excluding`. This run resolves the citations of those sources again.
    pub fn stored_citations_of(
        &self,
        target: &RepoPath,
        excluding: &BTreeSet<RepoPath>,
    ) -> Vec<Citation> {
        let removed = self.removed_set();
        self.conn()
            .rows(
                "SELECT slug, source, source_type, rel, line FROM citations WHERE target = ?1",
                [target.as_str()],
                |r| {
                    let Some(source) = RepoPath::parse(&r.get::<_, String>(1)?) else {
                        return Ok(None);
                    };
                    Ok(Some(Citation {
                        slug: r.get(0)?,
                        source,
                        source_type: r.get(2)?,
                        rel: r.get(3)?,
                        line: r.get::<_, i64>(4)? as usize,
                    }))
                },
            )
            .into_iter()
            .flatten()
            .filter(|c| !excluding.contains(&c.source) && !removed.contains(&c.source))
            .collect()
    }

    /// `old_citation_targets` returns the targets that `source` cited, as
    /// the cache recorded them.
    pub fn old_citation_targets(&self, source: &RepoPath) -> BTreeSet<RepoPath> {
        self.old_citations(source)
            .into_iter()
            .map(|c| c.target)
            .collect()
    }

    /// `old_citations` returns what `source` cited, as the cache recorded
    /// it.
    pub fn old_citations(&self, source: &RepoPath) -> BTreeSet<CitedFact> {
        self.conn()
            .rows(
                "SELECT target, slug, source_type, rel, line FROM citations WHERE source = ?1",
                [source.as_str()],
                |r| {
                    let Some(target) = RepoPath::parse(&r.get::<_, String>(0)?) else {
                        return Ok(None);
                    };
                    let rel: Option<String> = r.get(3)?;
                    let line = r.get::<_, i64>(4)? as usize;
                    Ok(Some(CitedFact {
                        target,
                        slug: r.get(1)?,
                        source_type: r.get(2)?,
                        line: rel.as_ref().map(|_| line),
                        rel,
                    }))
                },
            )
            .into_iter()
            .flatten()
            .collect()
    }

    /// `old_keys_of` returns the keys that the document at `path` declares,
    /// as the cache recorded them, each as `(scope, slug, knowledge)`.
    pub fn old_keys_of(&self, path: &RepoPath) -> BTreeSet<(String, String, usize)> {
        self.conn()
            .rows(
                "SELECT scope, key, knowledge FROM keys WHERE path = ?1",
                [path.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        crate::markdown::slug(&r.get::<_, String>(1)?),
                        r.get::<_, i64>(2)? as usize,
                    ))
                },
            )
            .into_iter()
            .collect()
    }

    /// `stored_outputs` returns the stored findings of every document
    /// except those in `except`, whose checks this run ran (CLI-4_AC-20).
    pub fn stored_outputs(&self, except: &BTreeSet<RepoPath>) -> Vec<Finding> {
        self.conn()
            .rows(
                "SELECT path, findings FROM outputs WHERE has_findings = 1",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .into_iter()
            .filter(|(path, _)| RepoPath::parse(path).is_some_and(|p| !except.contains(&p)))
            .filter_map(|(_, json)| serde_json::from_slice::<Vec<Finding>>(&json).ok())
            .flatten()
            .collect()
    }

    /// `commit_outputs` writes what the run recorded, inside the commit's
    /// transaction. It writes the outputs, queries, citations, edges, keys,
    /// and probes, and it replaces all the rows of each checked document.
    /// It also writes the asset pairings, the schemas, the fingerprint of
    /// the formats file, the additional files, the identities of the
    /// knowledge stores, and the hash of the formats file.
    // sokf-implements: CACHE-1_AC-14
    pub(super) fn commit_outputs(&self, conn: &Conn, state: &SyncState) -> rusqlite::Result<()> {
        let mut gone: Vec<&str> = state.removed.iter().map(RepoPath::as_str).collect();
        // When a directory is removed, the commit also deletes the rows of
        // its index. If the directory never had an index, the deletes match
        // no rows.
        let gone_indexes: Vec<String> = state
            .dirs_removed
            .iter()
            .map(|d| format!("{d}/index.md"))
            .collect();
        gone.extend(gone_indexes.iter().map(String::as_str));
        for path in gone {
            for sql in [
                "DELETE FROM outputs WHERE path = ?1",
                "DELETE FROM deps WHERE reader = (SELECT id FROM queries WHERE key = ?1)",
                "DELETE FROM citations WHERE source = ?1",
                "DELETE FROM edges WHERE source = ?1",
                "DELETE FROM keys WHERE path = ?1",
                "DELETE FROM probes WHERE path = ?1",
            ] {
                conn.execute(sql, [path])?;
            }
        }
        Self::commit_edges(conn, state)?;
        let mut put_output = conn.prepare(
            "INSERT OR REPLACE INTO outputs (path, findings, has_findings) VALUES (?1, ?2, ?3)",
        )?;
        for (path, findings) in &state.outputs {
            let json = serde_json::to_vec(findings).unwrap_or_default();
            put_output.execute(params![path.as_str(), json, !findings.is_empty()])?;
        }
        // The `queries` table interns each reader's path and each query key
        // once, so the `deps` table stores each dependency as a pair of
        // integer ids.
        let mut add_key = conn.prepare("INSERT OR IGNORE INTO queries (key) VALUES (?1)")?;
        let mut id_of = conn.prepare("SELECT id FROM queries WHERE key = ?1")?;
        let mut intern = |key: &str| -> rusqlite::Result<i64> {
            add_key.execute([key])?;
            id_of.query_row([key], |r| r.get::<_, i64>(0))
        };
        let mut del_deps = conn.prepare("DELETE FROM deps WHERE reader = ?1")?;
        let mut put_dep =
            conn.prepare("INSERT OR IGNORE INTO deps (reader, query) VALUES (?1, ?2)")?;
        for (reader, queries) in &state.deps {
            let rid = intern(reader.as_str())?;
            del_deps.execute([rid])?;
            for q in queries {
                let qid = intern(q)?;
                put_dep.execute([rid, qid])?;
            }
        }
        let mut del_cit = conn.prepare("DELETE FROM citations WHERE source = ?1")?;
        let mut put_cit = conn.prepare(
            "INSERT INTO citations (target, slug, source, source_type, rel, line) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for (source, rows) in &state.citations {
            del_cit.execute([source.as_str()])?;
            for c in rows {
                put_cit.execute(params![
                    c.target.as_str(),
                    c.slug,
                    c.source.as_str(),
                    c.source_type,
                    c.rel,
                    c.line as i64
                ])?;
            }
        }
        let mut del_keys = conn.prepare("DELETE FROM keys WHERE path = ?1")?;
        let mut put_key = conn.prepare(
            "INSERT INTO keys (path, key, slug, line, heading_slug, scope, knowledge, rule_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for (path, entries) in &state.keys_of {
            del_keys.execute([path.as_str()])?;
            for e in entries {
                let scope = match &e.scope {
                    crate::grammar::KeyScope::Document => "document".to_string(),
                    crate::grammar::KeyScope::Host(h) => h.clone(),
                };
                put_key.execute(params![
                    path.as_str(),
                    e.key,
                    crate::markdown::slug(&e.key),
                    e.line as i64,
                    e.heading_slug,
                    scope,
                    e.knowledge as i64,
                    e.rule_id
                ])?;
            }
        }
        let mut put_probe =
            conn.prepare("INSERT OR REPLACE INTO probes (path, exists_) VALUES (?1, ?2)")?;
        for (path, exists) in &state.probes {
            put_probe.execute(params![path.as_str(), i64::from(*exists)])?;
        }
        if let Some(assets) = &state.assets {
            conn.execute_batch("DELETE FROM assets")?;
            let mut put =
                conn.prepare("INSERT INTO assets (store, asset, sidecar) VALUES (?1, ?2, ?3)")?;
            for a in assets {
                put.execute(params![
                    a.knowledge as i64,
                    a.asset.as_str(),
                    a.sidecar.as_ref().map(|s| s.as_str())
                ])?;
            }
        }
        if let Some(schemas) = &state.schemas {
            conn.execute_batch("DELETE FROM schemas")?;
            let mut put = conn.prepare(
                "INSERT OR REPLACE INTO schemas (path, store, key, hash, globs, asset_globs, \
                 uses, line, archived) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for s in schemas {
                put.execute(params![
                    s.path.as_str(),
                    s.knowledge as i64,
                    s.key,
                    s.hash as i64,
                    s.globs,
                    s.asset_globs,
                    s.uses,
                    s.line as i64,
                    s.archived
                ])?;
            }
        }
        if let Some(formats) = &state.formats {
            conn.execute_batch("DELETE FROM formats")?;
            let mut put = conn.prepare("INSERT INTO formats (name, hash) VALUES (?1, ?2)")?;
            for (name, hash) in formats {
                put.execute(params![name, *hash as i64])?;
            }
        }
        if let Some(additional) = &state.additional {
            conn.execute_batch("DELETE FROM additional")?;
            let mut put = conn.prepare("INSERT INTO additional (path, schema) VALUES (?1, ?2)")?;
            for a in additional {
                put.execute(params![
                    a.path.as_str(),
                    a.schema.as_ref().map(|s| s.as_str()).unwrap_or("")
                ])?;
            }
        }
        if let Some(identities) = &state.identities {
            conn.execute_batch("DELETE FROM stores")?;
            let mut put =
                conn.prepare("INSERT INTO stores (root, identity, former) VALUES (?1, ?2, ?3)")?;
            for row in identities {
                put.execute(params![row.knowledge as i64, row.identity, row.former])?;
            }
        }
        if let Some(v) = state.formats_hash {
            conn.put_meta("formats_hash", v)?;
        }
        Ok(())
    }
}
