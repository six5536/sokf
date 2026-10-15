//! A SOKF schema is the engine's schema plus what SOKF's extension of the
//! grammar adds to it (sokf-schema-spec, contract-schema-extension): the
//! dispatch, the asset globs, the lifecycle root and paths, the link rules,
//! and the citation rules.

use tramlines_md::grammar::lower_common;

use super::{RuleCommon, Schema};
use crate::{Finding, RepoPath, Spanned};

/// `Dispatch` says which files a schema governs (SOKF_SCHEMA_3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dispatch {
    /// `Type` selects every document of the named type.
    Type(String),
    /// `Globs` selects every repository file that matches one of the globs.
    Globs(Vec<String>),
}

/// `LinkRule` is one entry of a schema's `links`, the grammar's `link` rule.
#[derive(Debug, Clone)]
pub struct LinkRule {
    /// `common` holds the keys that every rule shares.
    pub common: RuleCommon,
    /// `rel` is the relationship of the links that the entry governs.
    pub rel: String,
    /// `to_type` lists the types that a link can point to, and an empty list
    /// admits any type.
    pub to_type: Vec<String>,
    /// `min` is the fewest links allowed. It defaults to one when the key is
    /// absent.
    pub min: usize,
    /// `max` is the most links allowed, if the entry sets it.
    pub max: Option<usize>,
    /// `guidance`: the reason for the link, shown with the link.
    pub guidance: Option<String>,
}

/// `CitedFrom` says what a `cited-by` entry counts as a citation
/// (SOKF_SCHEMA_48).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitedFrom {
    /// `from-type` lists the types whose body links count: a link from a
    /// document of any of them is a citation.
    Types(Vec<String>),
    /// `from-marker`: the relationship of the markers that count.
    Marker(String),
}

/// `CitationRule` is one `cited-by` entry, the grammar's `citation` rule:
/// the inverse of a link rule.
#[derive(Debug, Clone)]
pub struct CitationRule {
    /// `common` holds the keys that every rule shares. Its `only` applies to
    /// the cited document's states.
    pub common: RuleCommon,
    /// `from-type` or `from-marker`.
    pub from: CitedFrom,
    /// `keys` names the rule whose keys must each be cited. `None` means that
    /// the document itself must be cited.
    pub keys: Option<String>,
}

/// `PathPlan` is one `plans-paths` entry, the grammar's `path-plan` rule: a
/// section whose path links plan files, while the document is in the states
/// of its `only`.
#[derive(Debug, Clone)]
pub struct PathPlan {
    /// `common` holds the keys that every rule shares. Its `only` applies to
    /// the planning document's states.
    pub common: RuleCommon,
    /// `section` is the `rule-id` of the section whose path links plan files.
    pub section: String,
}

/// `SokfSchema` is a SOKF schema lowered from its file: the engine's schema
/// and the keys that SOKF's extension adds to it.
#[derive(Debug, Clone)]
pub struct SokfSchema {
    /// `core` is the engine's schema.
    pub core: Schema,
    /// How the schema selects its files: by a `type` constant or by globs.
    pub dispatch: Dispatch,
    /// `asset_files` holds the globs of the assets that this type's documents
    /// describe, and the list is empty for a type without assets.
    pub asset_files: Vec<String>,
    /// `root` is the directory under which the documents of the type sit.
    pub root: Option<String>,
    /// The lifecycle dimensions that declare `path`, in lifecycle order.
    pub paths: Vec<String>,
    /// `links` holds the schema's link rules, one for each `rel`.
    pub links: Vec<LinkRule>,
    /// `links_open` is true when the schema accepts a link of any `rel`.
    pub links_open: bool,
    /// `cited_by` holds the schema's citation rules.
    pub cited_by: Vec<CitationRule>,
    /// `markers` lists the relationships of the markers that can cite a
    /// document of the type. `None` admits every relationship.
    pub markers: Option<Vec<String>>,
    /// `plans_paths` holds the sections whose path links plan files.
    pub plans_paths: Vec<PathPlan>,
    /// `current_links` holds the patterns of the paragraphs that link no
    /// archived document.
    pub current_links: Vec<regex::Regex>,
}

fn fail(
    path: &RepoPath,
    line: usize,
    criterion: &'static str,
    message: impl Into<String>,
) -> Finding {
    Finding::error(path.clone(), Some(line), criterion, message)
}

/// `glob_problem` says what is wrong with a glob of a schema, as the end of
/// a sentence about it, or returns `None` when the glob starts with / and
/// parses.
fn glob_problem(g: &str) -> Option<String> {
    if !g.starts_with('/') {
        return Some("does not start with /".into());
    }
    globset::Glob::new(g)
        .err()
        .map(|e| format!("does not parse: {e}"))
}

fn list(node: &Spanned) -> Vec<String> {
    node.as_str_or_list().unwrap_or_default()
}

/// `glob_inside` reports whether every path that `glob` can match lies under
/// `dir`: it checks that the glob's literal prefix, up to its first wildcard,
/// starts with `dir/`.
pub fn glob_inside(glob: &str, dir: &RepoPath) -> bool {
    let prefix: String = glob
        .chars()
        .take_while(|c| !matches!(c, '*' | '?' | '[' | '{'))
        .collect();
    let dir = format!("{}/", dir.as_str().trim_end_matches('/'));
    prefix.starts_with(&dir)
}

impl SokfSchema {
    /// `of_schema_document` returns the schema that the grammar's
    /// `schema-document` section defines. That schema governs Schema
    /// documents and demands nothing that SOKF adds. Tests also use it as a
    /// base.
    pub fn of_schema_document(core: Schema) -> Self {
        SokfSchema {
            core,
            dispatch: Dispatch::Type("Schema".into()),
            asset_files: vec![],
            root: None,
            paths: vec![],
            links: vec![],
            links_open: true,
            cited_by: vec![],
            markers: None,
            plans_paths: vec![],
            current_links: vec![],
        }
    }

    /// `lower` reads the keys that SOKF's extension adds from `yaml`, the
    /// schema file at `path`, and builds the SOKF schema around `core`, the
    /// engine's schema of the same file. It also checks the rules of the SOKF
    /// schema extension (contract-schema-extension) that the generic check
    /// cannot test. It returns the schema with a finding for each rule that
    /// the file breaks.
    // sokf-implements: EXT-2_AC-2
    // sokf-implements: SOKF_SCHEMA_3
    // sokf-implements: SOKF_SCHEMA_27
    pub fn lower(core: Schema, yaml: &Spanned, path: &RepoPath) -> (Self, Vec<Finding>) {
        let mut findings = Vec::new();
        let ext = |key: &str| core.extensions.get(key);
        let globs = ext("additional-files").map(list);
        let assets = ext("asset-files").map(list);
        // A schema dispatches on a `type` const, on `additional-files`, or
        // on a `type` const with `asset-files`, and it must declare exactly
        // one of `type` and `additional-files`.
        let dispatch = match (core.core.type_const().map(str::to_string), globs) {
            (Some(_), Some(_)) => {
                findings.push(fail(
                    path,
                    yaml.line,
                    "SOKF_SCHEMA_3",
                    "the schema declares both a `type` const and `additional-files`. Declare only one of them",
                ));
                Dispatch::Globs(vec![])
            }
            (None, None) => {
                let message = if assets.is_some() {
                    "`asset-files` needs a `type` const as well"
                } else {
                    "the schema declares neither a `type` const nor `additional-files`. Declare one of them"
                };
                findings.push(fail(path, yaml.line, "SOKF_SCHEMA_3", message));
                Dispatch::Globs(vec![])
            }
            (Some(t), None) => Dispatch::Type(t),
            (None, Some(globs)) => {
                let line = ext("additional-files").map_or(yaml.line, |a| a.line);
                for g in &globs {
                    if let Some(why) = glob_problem(g) {
                        findings.push(fail(
                            path,
                            line,
                            "SOKF_SCHEMA_5",
                            format!("`additional-files` glob `{g}` {why}"),
                        ));
                    }
                }
                Dispatch::Globs(globs)
            }
        };
        let asset_files = match assets {
            None => vec![],
            Some(globs) => {
                if ext("additional-files").is_some() {
                    let line = ext("asset-files").map_or(yaml.line, |a| a.line);
                    findings.push(fail(
                        path,
                        line,
                        "SOKF_SCHEMA_4",
                        "the schema declares `asset-files` together with `additional-files`. Declare only one of them",
                    ));
                }
                globs
            }
        };
        let paths: Vec<String> = core
            .core
            .lifecycle
            .iter()
            .filter(|d| {
                d.extensions
                    .get("path")
                    .and_then(|p| p.as_bool())
                    .unwrap_or(false)
            })
            .map(|d| d.key.clone())
            .collect();
        let root = ext("root").and_then(|r| r.as_str()).map(str::to_string);
        // Each archived state must be a state of its own dimension. A
        // schema-level `archived: true` is allowed only where the schema has
        // no lifecycle dimension to declare it on.
        // sokf-implements: EXT-7_AC-2
        // sokf-implements: SOKF_SCHEMA_50
        for d in &core.core.lifecycle {
            let Some(node) = d.extensions.get("archived") else {
                continue;
            };
            for state in list(node) {
                if !d.states.contains(&state) {
                    findings.push(fail(
                        path,
                        node.line,
                        "SOKF_SCHEMA_50",
                        format!(
                            "`archived` state `{state}` is not a state of the dimension `{}`",
                            d.key
                        ),
                    ));
                }
            }
        }
        // sokf-implements: EXT-2_AC-3
        if let Some(a) = ext("archived")
            && !core.core.lifecycle.is_empty()
        {
            findings.push(fail(
                path,
                a.line,
                "SOKF_SCHEMA_50",
                "the schema declares `archived` together with `lifecycle`. Declare `archived` on a dimension",
            ));
        }
        if let Some(l) = yaml.get("lifecycle") {
            if ext("additional-files").is_some() {
                findings.push(fail(
                    path,
                    l.line,
                    "SOKF_SCHEMA_4",
                    "the schema declares `lifecycle` together with `additional-files`. Declare only one of them",
                ));
            }
            if !paths.is_empty() && root.is_none() {
                findings.push(fail(
                    path,
                    l.line,
                    "SOKF_SCHEMA_27",
                    "a path-enforced dimension needs `root`",
                ));
            }
        }
        if let Some(r) = ext("root")
            && root.as_deref().and_then(RepoPath::parse).is_none()
        {
            findings.push(fail(
                path,
                r.line,
                "SOKF_SCHEMA_27",
                "`root` is not a valid repository path: it must start with / and have no empty, `.`, or `..` segment",
            ));
        }
        findings.extend(asset_globs(
            path,
            ext("asset-files").map_or(yaml.line, |a| a.line),
            &asset_files,
            root.as_deref(),
        ));
        let mut links = Vec::new();
        for node in ext("links").and_then(|l| l.as_seq()).unwrap_or(&[]) {
            let (common, problems) = lower_common(node, &core.core, path);
            findings.extend(problems);
            let usize_of = |key: &str| {
                node.get(key)
                    .and_then(|v| v.as_int())
                    .map(|i| usize::try_from(i).unwrap_or(0))
            };
            links.push(LinkRule {
                common,
                rel: node
                    .get("rel")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                to_type: node.get("to-type").map(list).unwrap_or_default(),
                min: usize_of("min").unwrap_or(1),
                max: usize_of("max"),
                guidance: node
                    .get("guidance")
                    .and_then(|g| g.as_str())
                    .map(str::to_string),
            });
        }
        let links_open = ext("links-open").and_then(|l| l.as_bool()).unwrap_or(false);
        // A citation entry's `keys` identifies a rule of this schema that
        // captures a key, and sokf resolves it as it resolves the rule of
        // `for-each-key-of`.
        // sokf-implements: EXT-9_AC-5
        let mut cited_by = Vec::new();
        for node in ext("cited-by").and_then(|l| l.as_seq()).unwrap_or(&[]) {
            let (common, problems) = lower_common(node, &core.core, path);
            findings.extend(problems);
            let keys = node
                .get("keys")
                .and_then(|k| k.as_str())
                .map(str::to_string);
            if let Some(k) = &keys
                && let Err(why) = core.rule_captures_key(k)
            {
                findings.push(fail(
                    path,
                    node.line,
                    "SOKF_SCHEMA_35",
                    why.message("keys", k),
                ));
            }
            // The grammar demands exactly one of `from-type` and
            // `from-marker`. The entry counts markers only when it has
            // `from-marker` and no `from-type`.
            let from = match (node.get("from-type"), node.get("from-marker")) {
                (None, Some(m)) => CitedFrom::Marker(m.as_str().unwrap_or_default().to_string()),
                (t, _) => CitedFrom::Types(t.map(list).unwrap_or_default()),
            };
            cited_by.push(CitationRule { common, from, keys });
        }
        let markers = ext("markers").map(list);
        // A path plan's `section` is the `rule-id` of a section of this
        // schema.
        // sokf-implements: SOKF_SCHEMA_52
        let mut plans_paths = Vec::new();
        for node in ext("plans-paths").and_then(|l| l.as_seq()).unwrap_or(&[]) {
            let (common, problems) = lower_common(node, &core.core, path);
            findings.extend(problems);
            let section = node
                .get("section")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string();
            if !core
                .sections
                .iter()
                .any(|s| s.rule_id.as_deref() == Some(section.as_str()))
            {
                findings.push(fail(
                    path,
                    node.line,
                    "SOKF_SCHEMA_52",
                    format!("`section: {section}` names no section of the schema; give the `rule-id` of a section"),
                ));
            }
            plans_paths.push(PathPlan { common, section });
        }
        // The grammar checks that each pattern compiles (TL_118).
        let current_links = ext("current-links")
            .map(list)
            .unwrap_or_default()
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect();
        let schema = SokfSchema {
            core,
            dispatch,
            asset_files,
            root,
            paths,
            links,
            links_open,
            cited_by,
            markers,
            plans_paths,
            current_links,
        };
        (schema, findings)
    }

    /// `has_assets` reports whether the schema's documents are sidecars, that
    /// is, whether the schema declares `asset-files` (SOKF_SCHEMA_11).
    pub fn has_assets(&self) -> bool {
        !self.asset_files.is_empty()
    }

    /// `is_path` reports whether the lifecycle dimension `key` declares `path`.
    pub fn is_path(&self, key: &str) -> bool {
        self.paths.iter().any(|p| p == key)
    }
}

/// `asset_globs` checks the glob rules for `asset-files` (SOKF_SCHEMA_8,
/// SOKF_SCHEMA_10): each glob starts with /, parses, and lies inside the
/// schema's knowledge root, which is the grandparent of the schema's path.
/// Where `root` is declared, each glob also begins with `root` followed by `/**/`.
// sokf-implements: EXT-2_AC-1
fn asset_globs(path: &RepoPath, line: usize, globs: &[String], root: Option<&str>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let knowledge = path.parent().and_then(|d| d.parent());
    for g in globs {
        if let Some(why) = glob_problem(g) {
            findings.push(fail(
                path,
                line,
                "SOKF_SCHEMA_8",
                format!("`asset-files` glob `{g}` {why}"),
            ));
            continue;
        }
        if let Some(k) = &knowledge
            && !glob_inside(g, k)
        {
            findings.push(fail(
                path,
                line,
                "SOKF_SCHEMA_8",
                format!("`asset-files` glob `{g}` lies outside the knowledge root `{k}`"),
            ));
        }
        if let Some(r) = root
            && !g.starts_with(&format!("{}/**/", r.trim_end_matches('/')))
        {
            findings.push(fail(
                path,
                line,
                "SOKF_SCHEMA_10",
                format!(
                    "`asset-files` glob `{g}` does not begin with `{r}/**/`, where `{r}` is the lifecycle root"
                ),
            ));
        }
    }
    findings
}
