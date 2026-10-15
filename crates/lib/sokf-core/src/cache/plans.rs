//! The store keeps the paths that each document plans, so that a run that
//! doesn't read a planning document still knows its plans, and so that a
//! change to a plan makes the readers of each changed path dirty
//! (SOKF_SCHEMA_53).

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::params;

use super::{Cache, conn::Conn, sync::SyncState};
use crate::{RepoPath, model::PlanIndex};

impl Cache {
    /// `record_plans_of` keeps the paths that each document in `checked`
    /// plans in this run. The commit writes them in place of the document's
    /// rows in the store.
    pub fn record_plans_of(&self, plans: &PlanIndex, checked: &BTreeSet<RepoPath>) {
        let rows: BTreeMap<RepoPath, BTreeSet<RepoPath>> =
            checked.iter().map(|p| (p.clone(), plans.of(p))).collect();
        self.state.borrow_mut().plans_of.extend(rows);
    }

    /// `old_plans_of` returns the paths that the document at `path` planned,
    /// as the store recorded them.
    pub fn old_plans_of(&self, path: &RepoPath) -> BTreeSet<RepoPath> {
        self.conn()
            .rows(
                "SELECT target FROM plans WHERE path = ?1",
                [path.as_str()],
                |r| r.get::<_, String>(0),
            )
            .iter()
            .filter_map(|t| RepoPath::parse(t))
            .collect()
    }

    /// `stored_plans` returns the plans that the store records, except the
    /// plans of the documents in `except` and of the removed files.
    pub fn stored_plans(&self, except: &BTreeSet<RepoPath>) -> PlanIndex {
        let mut plans = PlanIndex::default();
        let removed = self.removed_set();
        let rows = self.conn().rows("SELECT path, target FROM plans", [], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        });
        for (path, target) in rows {
            let (Some(path), Some(target)) = (RepoPath::parse(&path), RepoPath::parse(&target))
            else {
                continue;
            };
            if !except.contains(&path) && !removed.contains(&path) {
                plans.add(&path, &target);
            }
        }
        plans
    }

    /// `commit_plans` replaces the rows in `plans` of each document that the
    /// run checked, and removes the rows of each removed file.
    pub(super) fn commit_plans(&self, conn: &Conn, state: &SyncState) -> rusqlite::Result<()> {
        let mut delete = conn.prepare("DELETE FROM plans WHERE path = ?1")?;
        let mut put = conn.prepare("INSERT INTO plans (path, target) VALUES (?1, ?2)")?;
        for path in &state.removed {
            delete.execute([path.as_str()])?;
        }
        for (path, targets) in &state.plans_of {
            delete.execute([path.as_str()])?;
            for target in targets {
                put.execute(params![path.as_str(), target.as_str()])?;
            }
        }
        Ok(())
    }
}
