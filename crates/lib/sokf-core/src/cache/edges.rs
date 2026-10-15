//! The store keeps the edges of each document for `graph`. The cache writes
//! them when the run commits, and reads them back either all at once or only
//! those that touch a set of paths or point under a directory. For a document that the current run checked, the cache answers
//! with the run's own edges, not with the stored rows.

use std::collections::BTreeSet;

use rusqlite::params;

use super::{Cache, conn::Conn, sync::SyncState};
use crate::{
    RepoPath,
    graph::{EdgeKind, EdgeStore, NodeKey, RawEdge},
};

/// `COLUMNS` selects the columns of an edge row, in the order that `row_edge`
/// reads them.
const COLUMNS: &str = "SELECT source, kind, written, target, missing, rel, region, note FROM edges";

/// `edge_code` returns the code under which the store keeps an `EdgeKind`.
fn edge_code(kind: EdgeKind) -> i64 {
    match kind {
        EdgeKind::Includes => 0,
        EdgeKind::Links => 1,
        EdgeKind::Resource => 2,
        EdgeKind::Sources => 3,
        // The graph builds a key's edges only when the query starts from that
        // key, so the store never holds an edge of this kind.
        EdgeKind::Keys => 4,
        // The graph reads a marker's edge from the citations that the check
        // of its file stores, so the edges table never holds one.
        EdgeKind::Marker => 5,
    }
}

/// `edge_kind` returns the `EdgeKind` of a stored code, and reads an unknown
/// code as a link.
fn edge_kind(code: i64) -> EdgeKind {
    match code {
        0 => EdgeKind::Includes,
        2 => EdgeKind::Resource,
        3 => EdgeKind::Sources,
        _ => EdgeKind::Links,
    }
}

impl Cache {
    /// `record_edges` holds the edges that one document declares until the
    /// commit writes them (CACHE-1_AC-11).
    // sokf-implements: CACHE-1_AC-22
    pub fn record_edges(&self, source: &RepoPath, edges: Vec<RawEdge>) {
        self.state.borrow_mut().edges.insert(source.clone(), edges);
    }

    /// `read_edges` returns the edges that `sql` selects with each set of
    /// `args`, and prepares the statement once for all of them. It skips the
    /// stored rows of each source that the run checked. In their place, it
    /// adds each edge from the current run that `keep` accepts. It also skips
    /// the rows of each removed file. Because it skips those rows, the answer
    /// stays correct even when a commit fails (CACHE-1_AC-9).
    fn read_edges(
        &self,
        sql: &str,
        args: &[Vec<&str>],
        keep: impl Fn(&RawEdge) -> bool,
    ) -> Vec<RawEdge> {
        let mut out = self.read_stored(sql, args, row_edge, |s, from| s.edges.contains_key(from));
        let state = self.state.borrow();
        for edges in state.edges.values() {
            out.extend(edges.iter().filter(|e| keep(e)).cloned());
        }
        out
    }

    /// `read_markers` returns the edges of the markers that `sql` selects from
    /// the stored citations with each set of `args`, as `read_edges` does for
    /// the edges of documents. It skips the stored rows of each file that the
    /// run checked, and adds in their place each marker of the run that
    /// `keep` accepts. It also skips the rows of each removed file.
    fn read_markers(
        &self,
        sql: &str,
        args: &[Vec<&str>],
        keep: impl Fn(&RawEdge) -> bool,
    ) -> Vec<RawEdge> {
        let mut out = self.read_stored(sql, args, row_marker, |s, from| {
            s.citations.contains_key(from)
        });
        let state = self.state.borrow();
        for rows in state.citations.values() {
            out.extend(
                rows.iter()
                    .filter_map(|c| {
                        let rel = c.rel.clone()?;
                        Some(marker_edge(
                            c.source.clone(),
                            &c.target,
                            c.slug.clone(),
                            rel,
                            c.line,
                        ))
                    })
                    .filter(|e| keep(e)),
            );
        }
        out
    }

    /// `read_stored` returns the stored edges that `sql` selects with each
    /// set of `args`, and prepares the statement once for all of them. It
    /// reads each row with `row`, and skips the rows of each source that
    /// `checked` says the run checked, and of each removed file.
    fn read_stored(
        &self,
        sql: &str,
        args: &[Vec<&str>],
        row: fn(&rusqlite::Row<'_>) -> rusqlite::Result<Option<RawEdge>>,
        checked: impl Fn(&SyncState, &RepoPath) -> bool,
    ) -> Vec<RawEdge> {
        let state = self.state.borrow();
        let removed: BTreeSet<&RepoPath> = state.removed.iter().collect();
        let mut out: Vec<RawEdge> = Vec::new();
        if let Ok(mut stmt) = self.conn().prepare(sql) {
            for one in args {
                if let Ok(rows) = stmt.query_map(rusqlite::params_from_iter(one), row) {
                    out.extend(
                        rows.flatten()
                            .flatten()
                            .filter(|e| !checked(&state, &e.from) && !removed.contains(&e.from)),
                    );
                }
            }
        }
        out
    }

    /// `commit_edges` writes the edges of each document that the run checked,
    /// in place of the document's stored rows.
    pub(super) fn commit_edges(conn: &Conn, state: &SyncState) -> rusqlite::Result<()> {
        let mut del_edges = conn.prepare("DELETE FROM edges WHERE source = ?1")?;
        let mut put_edge = conn.prepare(
            "INSERT INTO edges (source, kind, written, target, missing, rel, region, note) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for (source, edges) in &state.edges {
            del_edges.execute([source.as_str()])?;
            for e in edges {
                put_edge.execute(params![
                    source.as_str(),
                    edge_code(e.kind),
                    e.written,
                    e.to.as_str(),
                    i64::from(matches!(e.to, NodeKey::Missing(_))),
                    e.rel,
                    e.region,
                    e.note
                ])?;
            }
        }
        Ok(())
    }
}

/// `row_edge` reads one stored edge, and returns `None` when the edge's
/// source isn't a valid path.
fn row_edge(r: &rusqlite::Row<'_>) -> rusqlite::Result<Option<RawEdge>> {
    let source: String = r.get(0)?;
    let target: String = r.get(3)?;
    let missing: i64 = r.get(4)?;
    let Some(from) = RepoPath::parse(&source) else {
        return Ok(None);
    };
    let to = match (missing != 0, RepoPath::parse(&target)) {
        (false, Some(p)) => NodeKey::Path(p),
        _ => NodeKey::Missing(target),
    };
    Ok(Some(RawEdge {
        from,
        kind: edge_kind(r.get(1)?),
        written: r.get(2)?,
        to,
        rel: r.get(5)?,
        region: r.get(6)?,
        line: None,
        note: r.get(7)?,
    }))
}

/// `MARKERS` selects the columns of a marker's citation, in the order that
/// `row_marker` reads them. A citation with a `rel` comes from a marker.
const MARKERS: &str = "SELECT source, target, slug, rel, line FROM citations WHERE rel IS NOT NULL";

/// `marker_edge` returns the edge of a marker in `source` that cites `target`.
fn marker_edge(
    source: RepoPath,
    target: &RepoPath,
    slug: Option<String>,
    rel: String,
    line: usize,
) -> RawEdge {
    RawEdge {
        from: source,
        kind: EdgeKind::Marker,
        written: target.as_str().to_string(),
        to: NodeKey::Path(target.clone()),
        rel,
        region: slug,
        line: Some(line),
        note: None,
    }
}

/// `row_marker` reads the citation of one marker as its edge, and returns
/// `None` when a path isn't valid.
fn row_marker(r: &rusqlite::Row<'_>) -> rusqlite::Result<Option<RawEdge>> {
    let (source, target): (String, String) = (r.get(0)?, r.get(1)?);
    let (Some(source), Some(target)) = (RepoPath::parse(&source), RepoPath::parse(&target)) else {
        return Ok(None);
    };
    let line: i64 = r.get(4)?;
    Ok(Some(marker_edge(
        source,
        &target,
        r.get(2)?,
        r.get(3)?,
        line as usize,
    )))
}

/// The cache gives `graph` the edges that the store keeps (CACHE-1_AC-22).
// sokf-implements: CACHE-1_AC-22
// sokf-implements: CACHE-1_AC-38
// sokf-implements: CACHE-1_AC-44
// sokf-implements: CACHE-1_AC-45
impl EdgeStore for Cache {
    fn every(&self) -> Vec<RawEdge> {
        let mut out = self.read_edges(COLUMNS, &[Vec::new()], |_| true);
        out.extend(self.read_markers(MARKERS, &[Vec::new()], |_| true));
        out
    }

    fn touching(&self, paths: &BTreeSet<RepoPath>) -> Vec<RawEdge> {
        // The query returns an edge between two of `paths` once for each
        // end, and the graph keeps only one copy.
        let args: Vec<Vec<&str>> = paths.iter().map(|p| vec![p.as_str()]).collect();
        let touches =
            |e: &RawEdge| paths.contains(&e.from) || e.to.path().is_some_and(|p| paths.contains(p));
        let mut out = self.read_edges(
            &format!("{COLUMNS} WHERE source = ?1 OR (target = ?1 AND missing = 0)"),
            &args,
            touches,
        );
        out.extend(self.read_markers(
            &format!("{MARKERS} AND (source = ?1 OR target = ?1)"),
            &args,
            touches,
        ));
        out
    }

    fn under(&self, dir: &RepoPath) -> Vec<RawEdge> {
        let (low, high) = super::hint::under(dir);
        let mut out = self.read_edges(
            &format!("{COLUMNS} WHERE target >= ?1 AND target < ?2 AND missing = 0"),
            &[vec![&low, &high]],
            |e| e.to.path().is_some_and(|p| p != dir && p.starts_with(dir)),
        );
        // A file under `dir` that holds a marker is the source of the
        // marker's edge, not its target.
        out.extend(self.read_markers(
            &format!("{MARKERS} AND source >= ?1 AND source < ?2"),
            &[vec![&low, &high]],
            |e| e.from != *dir && e.from.starts_with(dir),
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CacheMode, RepoRoot};

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn edge(from: &str, to: &str) -> RawEdge {
        RawEdge {
            from: p(from),
            kind: EdgeKind::Links,
            written: to.into(),
            to: NodeKey::Path(p(to)),
            rel: "relates-to".into(),
            region: None,
            line: None,
            note: None,
        }
    }

    fn shown(mut edges: Vec<RawEdge>) -> Vec<String> {
        edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
        edges
            .iter()
            .map(|e| format!("{} {}", e.from, e.to.as_str()))
            .collect()
    }

    // sokf-tests: CACHE-1_AC-38
    #[test]
    fn the_store_gives_the_edges_near_paths_and_the_run_answers_for_its_checks() {
        let root = RepoRoot::new(std::env::temp_dir());
        let cache = Cache::open(&root, CacheMode::Off);
        // The last run stored four edges: one from `/a.md`, one from `/c.md`
        // to `/a.md`, one from `/d.md` to a file under `/src`, and one from
        // `/e.md`. Since that run, `/a.md` has changed and the file `/e.md`
        // has been removed.
        cache.record_edges(&p("/a.md"), vec![edge("/a.md", "/old.md")]);
        cache.record_edges(&p("/c.md"), vec![edge("/c.md", "/a.md")]);
        cache.record_edges(&p("/d.md"), vec![edge("/d.md", "/src/x.rs")]);
        cache.record_edges(&p("/e.md"), vec![edge("/e.md", "/a.md")]);
        cache.commit();
        // The current run has checked `/a.md`, but hasn't committed yet.
        {
            let mut state = cache.state.borrow_mut();
            state.edges.clear();
            state.removed.push(p("/e.md"));
        }
        cache.record_edges(&p("/a.md"), vec![edge("/a.md", "/b.md")]);
        let at: BTreeSet<RepoPath> = [p("/a.md")].into();
        assert_eq!(shown(cache.touching(&at)), ["/a.md /b.md", "/c.md /a.md"]);
        assert_eq!(shown(cache.touching(&[p("/b.md")].into())), ["/a.md /b.md"]);
        assert_eq!(shown(cache.under(&p("/src"))), ["/d.md /src/x.rs"]);
        assert!(cache.under(&p("/sr")).is_empty());
        // Each query finds its rows through an index, and never scans a table.
        for (sql, args) in [
            (
                format!("{COLUMNS} WHERE source = ?1 OR (target = ?1 AND missing = 0)"),
                vec!["/a.md"],
            ),
            (
                format!("{COLUMNS} WHERE target >= ?1 AND target < ?2 AND missing = 0"),
                vec!["/src/", "/src0"],
            ),
        ] {
            let plan: Vec<String> = cache
                .conn()
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .unwrap()
                .query_map(rusqlite::params_from_iter(args), |r| r.get::<_, String>(3))
                .unwrap()
                .flatten()
                .collect();
            assert!(
                plan.iter().all(|l| !l.starts_with("SCAN")),
                "{sql}: {plan:?}"
            );
        }
        assert_eq!(
            shown(cache.every()),
            ["/a.md /b.md", "/c.md /a.md", "/d.md /src/x.rs"]
        );
    }
}
