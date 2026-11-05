//! This module holds the files that the documents plan: the repository paths
//! that a planning section of a document links while the document is in the
//! states of its plan (SOKF_SCHEMA_53). A check accepts a planned path that
//! doesn't exist yet, and every path under a planned directory
//! (SOKF_SCHEMA_55).

use std::collections::{BTreeMap, BTreeSet};

use super::{Model, Query};
use crate::RepoPath;

/// A `Planned` is one path that a document plans.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Planned {
    /// The path, without the trailing `/` characters of its link.
    pub path: RepoPath,
    /// Whether the link ends with `/`, so that the plan names a directory
    /// and every path under it.
    pub dir: bool,
}

impl Planned {
    /// `key` returns the planned path as one line of text, the form in which
    /// the store keeps it: the path, with a `/` after it for a directory.
    pub fn key(&self) -> String {
        if self.dir {
            format!("{}/", self.path)
        } else {
            self.path.to_string()
        }
    }

    /// `parse` reads the text that `key` returns.
    pub fn parse(key: &str) -> Option<Planned> {
        match key.strip_suffix('/').filter(|rest| !rest.is_empty()) {
            Some(path) => Some(Planned {
                path: RepoPath::parse(path)?,
                dir: true,
            }),
            None => Some(Planned {
                path: RepoPath::parse(key)?,
                dir: false,
            }),
        }
    }
}

/// `PlanIndex` maps each planned path to the documents that plan it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlanIndex {
    by_target: BTreeMap<Planned, BTreeSet<RepoPath>>,
}

impl PlanIndex {
    /// `add` records that the document at `planner` plans `target`.
    pub fn add(&mut self, planner: &RepoPath, target: &Planned) {
        self.by_target
            .entry(target.clone())
            .or_default()
            .insert(planner.clone());
    }

    /// `contains` reports whether some document plans `target`: the path
    /// itself, or a directory that holds it.
    // sokf-implements: SOKF_SCHEMA_55
    pub fn contains(&self, target: &RepoPath) -> bool {
        let planned = |path: &RepoPath, dir: bool| {
            self.by_target.contains_key(&Planned {
                path: path.clone(),
                dir,
            })
        };
        planned(target, false)
            || std::iter::successors(Some(target.clone()), RepoPath::parent)
                .any(|path| planned(&path, true))
    }

    /// `targets` returns every planned path.
    pub fn targets(&self) -> Vec<Planned> {
        self.by_target.keys().cloned().collect()
    }

    /// `planners` returns the documents that plan `target`.
    pub fn planners(&self, target: &Planned) -> Vec<RepoPath> {
        self.by_target
            .get(target)
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// `of` returns the paths that the document at `planner` plans.
    pub fn of(&self, planner: &RepoPath) -> BTreeSet<Planned> {
        self.by_target
            .iter()
            .filter(|(_, planners)| planners.contains(planner))
            .map(|(target, _)| target.clone())
            .collect()
    }
}

impl Model {
    /// `is_planned` reports whether a document plans a file at `path`, and
    /// records the query for `path` and for each directory above it, so
    /// that the reader is checked again when the plan of the path or of a
    /// directory that holds it changes.
    pub fn is_planned(&self, path: &RepoPath) -> bool {
        for planned in std::iter::successors(Some(path.clone()), RepoPath::parent) {
            self.note(Query::Planned(planned));
        }
        self.plans.contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planned(path: &str, dir: bool) -> Planned {
        Planned {
            path: RepoPath::parse(path).unwrap(),
            dir,
        }
    }

    #[test]
    fn a_planned_path_keeps_its_slash_in_the_store() {
        for (path, dir) in [("/a/b", false), ("/a/b", true), ("/", false), ("/", true)] {
            let p = planned(path, dir);
            assert_eq!(Planned::parse(&p.key()), Some(p.clone()), "{}", p.key());
        }
        assert_eq!(planned("/a", true).key(), "/a/");
        assert_eq!(planned("/", true).key(), "//");
    }

    // sokf-tests: SOKF_SCHEMA_55
    #[test]
    fn a_planned_directory_plans_every_path_under_it() {
        let mut plans = PlanIndex::default();
        let planner = RepoPath::parse("/k/issue.md").unwrap();
        plans.add(&planner, &planned("/x/fixtures", true));
        plans.add(&planner, &planned("/y/one.rs", false));
        let has = |p: &str| plans.contains(&RepoPath::parse(p).unwrap());
        assert!(has("/x/fixtures"));
        assert!(has("/x/fixtures/a.json"));
        assert!(has("/x/fixtures/deep/b.json"));
        assert!(!has("/x/fixturesmore"));
        assert!(!has("/x"));
        assert!(has("/y/one.rs"));
        assert!(!has("/y/one.rs/under"));
    }
}
