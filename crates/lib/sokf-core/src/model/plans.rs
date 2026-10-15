//! This module holds the files that the documents plan: the repository paths
//! that a planning section of a document links while the document is in the
//! states of its plan (SOKF_SCHEMA_53). A check accepts a planned path that
//! doesn't exist yet.

use std::collections::{BTreeMap, BTreeSet};

use super::{Model, Query};
use crate::RepoPath;

/// `PlanIndex` maps each planned path to the documents that plan it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlanIndex {
    by_target: BTreeMap<RepoPath, BTreeSet<RepoPath>>,
}

impl PlanIndex {
    /// `add` records that the document at `planner` plans `target`.
    pub fn add(&mut self, planner: &RepoPath, target: &RepoPath) {
        self.by_target
            .entry(target.clone())
            .or_default()
            .insert(planner.clone());
    }

    /// `contains` reports whether some document plans `target`.
    pub fn contains(&self, target: &RepoPath) -> bool {
        self.by_target.contains_key(target)
    }

    /// `targets` returns every planned path.
    pub fn targets(&self) -> Vec<RepoPath> {
        self.by_target.keys().cloned().collect()
    }

    /// `planners` returns the documents that plan `target`.
    pub fn planners(&self, target: &RepoPath) -> Vec<RepoPath> {
        self.by_target
            .get(target)
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// `of` returns the paths that the document at `planner` plans.
    pub fn of(&self, planner: &RepoPath) -> BTreeSet<RepoPath> {
        self.by_target
            .iter()
            .filter(|(_, planners)| planners.contains(planner))
            .map(|(target, _)| target.clone())
            .collect()
    }
}

impl Model {
    /// `is_planned` reports whether a document plans a file at `path`, and
    /// records the query, so that the reader is checked again when the plan
    /// changes.
    pub fn is_planned(&self, path: &RepoPath) -> bool {
        self.note(Query::Planned(path.clone()));
        self.plans.contains(path)
    }
}
