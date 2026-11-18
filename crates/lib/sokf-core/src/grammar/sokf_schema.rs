//! A SOKF schema is the engine's schema plus what SOKF's extension of the
//! grammar adds to it (sokf-schema-spec, contract-schema-extension): the
//! dispatch, the asset globs, the lifecycle root and paths, the link rules,
//! the citation rules, and the target types of the unit rules.

use std::collections::BTreeMap;

use tramlines_core::grammar::Lowerer;
use tramlines_md::grammar::lower_common;

use super::{Block, Limit, RegexCache, RuleCommon, Schema, States, TypeRegistry, Unit};
use crate::{Finding, RepoPath, Spanned};

/// `Dispatch` says which files a schema governs (`SOKF_SCHEMA_3`).
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
    /// `max` is the limit on the links, if the entry sets it: a hard limit
    /// bounds them, and a soft one bounds nothing. Its `when-at-limit` says
    /// what the writer does at the limit.
    pub max: Option<Limit>,
    /// `guidance`: the reason for the link, shown with the link.
    pub guidance: Option<String>,
}

/// `CitedFrom` says what a `cited-by` entry counts as a citation
/// (`SOKF_SCHEMA_48`).
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
    /// `keys` names the set of keys that must each be cited. `None` means that
    /// the document itself must be cited.
    pub keys: Option<String>,
    /// `message` is the sentence that the entry appends to each finding it
    /// raises, if the entry declares one.
    pub message: Option<String>,
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
    /// `unit_types` holds the `to-type` of each unit rule that declares
    /// one, by the rule's `Unit::index`, which names the rule in the
    /// engine's record of an accepted unit.
    pub unit_types: BTreeMap<usize, Vec<String>>,
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

/// `kebab` returns the lowercase kebab case form of `rel`, splitting camel
/// case and turning `_` and spaces into `-`, such as `test-of` for `TestOf`
/// or `test_of`. It returns `None` when the form still isn't a slug.
fn kebab(rel: &str) -> Option<String> {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in rel.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            out.push('-');
        }
        prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        out.push(match c {
            '_' | ' ' => '-',
            c => c.to_ascii_lowercase(),
        });
    }
    crate::markdown::is_slug(&out).then_some(out)
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

impl LinkRule {
    /// `banned` reports whether a hard limit of 0 bans the links of the
    /// rule, so that a scaffold and a guide leave the rule out.
    pub fn banned(&self) -> bool {
        self.max.as_ref().and_then(Limit::hard) == Some(0)
    }
}

impl SokfSchema {
    /// `type_root` returns the schema's `root`, the directory that holds
    /// the type's documents, as a path in the repository, when the schema
    /// declares one that parses.
    pub fn type_root(&self) -> Option<RepoPath> {
        RepoPath::parse(self.root.as_deref()?)
    }

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
            unit_types: BTreeMap::new(),
        }
    }

    /// `lower` reads the keys that SOKF's extension adds from `yaml`, the
    /// schema file at `path`, and builds the SOKF schema around `core`, the
    /// engine's schema of the same file. It also checks the rules of the SOKF
    /// schema extension (contract-schema-extension) that the generic check
    /// cannot test. It returns the schema with a finding for each rule that
    /// the file breaks.
    pub fn lower(core: Schema, yaml: &Spanned, path: &RepoPath) -> (Self, Vec<Finding>) {
        let mut lw = Lowering {
            core: &core,
            yaml,
            path,
            findings: Vec::new(),
        };
        let (dispatch, asset_files) = lw.dispatch();
        let paths = path_dimensions(&core);
        let root = lw.ext("root").and_then(|r| r.as_str()).map(str::to_string);
        lw.check_archived();
        lw.check_labels();
        lw.check_lifecycle(&paths, root.as_deref());
        lw.findings.extend(asset_globs(
            path,
            lw.ext("asset-files").map_or(yaml.line, |a| a.line),
            &asset_files,
            root.as_deref(),
        ));
        let links = lw.links();
        let links_open = lw
            .ext("links-open")
            .and_then(Spanned::as_bool)
            .unwrap_or(false);
        let cited_by = lw.cited_by();
        let markers = lw.markers();
        let plans_paths = lw.plans_paths();
        // The grammar checks that each pattern compiles (TL_118).
        let current_links = lw
            .ext("current-links")
            .map(list)
            .unwrap_or_default()
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect();
        let findings = lw.findings;
        let unit_types = unit_types(&core);
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
            unit_types,
        };
        (schema, findings)
    }

    /// `has_assets` reports whether the schema's documents are sidecars, that
    /// is, whether the schema declares `asset-files` (`SOKF_SCHEMA_11`).
    pub fn has_assets(&self) -> bool {
        !self.asset_files.is_empty()
    }

    /// `written_links` returns the link rules that a scaffold and a guide
    /// show at `states`: each rule that the states admit, less each rule
    /// that a hard limit of 0 bans.
    pub fn written_links<'s>(&'s self, states: &'s States) -> impl Iterator<Item = &'s LinkRule> {
        self.links
            .iter()
            .filter(|r| r.common.only.admits(states) && !r.banned())
    }

    /// `is_path` reports whether the lifecycle dimension `key` declares `path`.
    pub fn is_path(&self, key: &str) -> bool {
        self.paths.iter().any(|p| p == key)
    }
}

/// `Lowering` is the state that `SokfSchema::lower` shares between its
/// steps: the engine's schema, the schema file's YAML and path, and the
/// findings raised so far.
struct Lowering<'a> {
    /// `core` is the engine's schema of the file.
    core: &'a Schema,
    /// `yaml` is the schema file's YAML.
    yaml: &'a Spanned,
    /// `path` is the path of the schema file.
    path: &'a RepoPath,
    /// `findings` holds the findings raised so far, in the order of the steps.
    findings: Vec<Finding>,
}

impl<'a> Lowering<'a> {
    /// `ext` returns the node of the extension key `key`, if the schema
    /// declares it.
    fn ext(&self, key: &str) -> Option<&'a Spanned> {
        self.core.extensions.get(key)
    }

    /// `dispatch` reads how the schema selects its files, from its `type`
    /// const, `additional-files`, and `asset-files`, and returns the dispatch
    /// with the asset globs. It checks that the schema declares exactly one
    /// of `type` and `additional-files`, and `asset-files` only with `type`.
    // sokf-implements: EXT-2_AC-2
    // sokf-implements: SOKF_SCHEMA_3
    fn dispatch(&mut self) -> (Dispatch, Vec<String>) {
        let (path, yaml) = (self.path, self.yaml);
        let globs = self.ext("additional-files").map(list);
        let assets = self.ext("asset-files").map(list);
        // A schema dispatches on a `type` const, on `additional-files`, or
        // on a `type` const with `asset-files`, and it must declare exactly
        // one of `type` and `additional-files`.
        let dispatch = match (self.core.core.type_const().map(str::to_string), globs) {
            (Some(_), Some(_)) => {
                self.findings.push(fail(
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
                self.findings
                    .push(fail(path, yaml.line, "SOKF_SCHEMA_3", message));
                Dispatch::Globs(vec![])
            }
            (Some(t), None) => Dispatch::Type(t),
            (None, Some(globs)) => {
                let line = self.ext("additional-files").map_or(yaml.line, |a| a.line);
                for g in &globs {
                    if let Some(why) = glob_problem(g) {
                        self.findings.push(fail(
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
                if self.ext("additional-files").is_some() {
                    let line = self.ext("asset-files").map_or(yaml.line, |a| a.line);
                    self.findings.push(fail(
                        path,
                        line,
                        "SOKF_SCHEMA_4",
                        "the schema declares `asset-files` together with `additional-files`. Declare only one of them",
                    ));
                }
                globs
            }
        };
        (dispatch, asset_files)
    }

    /// `check_archived` checks where the schema declares `archived`: on a
    /// lifecycle dimension, with states of that dimension, or at the schema
    /// level, only where the schema has no lifecycle.
    fn check_archived(&mut self) {
        let path = self.path;
        // Each archived state must be a state of its own dimension. A
        // schema-level `archived: true` is allowed only where the schema has
        // no lifecycle dimension to declare it on.
        // sokf-implements: EXT-7_AC-2
        // sokf-implements: SOKF_SCHEMA_50
        for d in &self.core.core.lifecycle {
            let Some(node) = d.extensions.get("archived") else {
                continue;
            };
            for state in list(node) {
                if !d.states.contains(&state) {
                    self.findings.push(fail(
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
        if let Some(a) = self.ext("archived")
            && !self.core.core.lifecycle.is_empty()
        {
            self.findings.push(fail(
                path,
                a.line,
                "SOKF_SCHEMA_50",
                "the schema declares `archived` together with `lifecycle`. Declare `archived` on a dimension",
            ));
        }
    }

    /// `check_labels` refuses each key of `labels` other than `human` and
    /// `agent`, the attribution labels, because sokf gives no other label its
    /// words or a meaning, and `labels` without one of the two, because a
    /// scaffold would then label what an agent writes `[human]`.
    // sokf-implements: SOKF_SCHEMA_104
    fn check_labels(&mut self) {
        let Some(labels) = self.yaml.get("labels") else {
            return;
        };
        let items = labels.as_seq().unwrap_or_default();
        for key in crate::config::LABELS {
            if !items.iter().any(|i| i.as_str() == Some(key)) {
                self.findings.push(fail(
                    self.path,
                    labels.line,
                    "SOKF_SCHEMA_104",
                    format!(
                        "`labels` holds no `{key}`; a schema declares both of sokf's labels, `human` and `agent`"
                    ),
                ));
            }
        }
        for item in items {
            let Some(key) = item.as_str() else {
                continue;
            };
            if !crate::config::LABELS.contains(&key) {
                self.findings.push(fail(
                    self.path,
                    item.line,
                    "SOKF_SCHEMA_104",
                    format!(
                        "`labels` holds `{key}`, which is no label of sokf's; sokf's labels are `human` and `agent`"
                    ),
                ));
            }
        }
    }

    /// `check_lifecycle` checks `lifecycle` and `root` against the rest of
    /// the schema: `lifecycle` doesn't come with `additional-files`, a
    /// path-enforced dimension of `paths` has a `root`, and `root` is a valid
    /// repository path.
    // sokf-implements: SOKF_SCHEMA_27
    fn check_lifecycle(&mut self, paths: &[String], root: Option<&str>) {
        let path = self.path;
        if let Some(l) = self.yaml.get("lifecycle") {
            if self.ext("additional-files").is_some() {
                self.findings.push(fail(
                    path,
                    l.line,
                    "SOKF_SCHEMA_4",
                    "the schema declares `lifecycle` together with `additional-files`. Declare only one of them",
                ));
            }
            if !paths.is_empty() && root.is_none() {
                self.findings.push(fail(
                    path,
                    l.line,
                    "SOKF_SCHEMA_27",
                    "a path-enforced dimension needs `root`",
                ));
            }
        }
        if let Some(r) = self.ext("root")
            && root.and_then(RepoPath::parse).is_none()
        {
            self.findings.push(fail(
                path,
                r.line,
                "SOKF_SCHEMA_27",
                "`root` is not a valid repository path: it must start with / and have no empty, `.`, or `..` segment",
            ));
        }
    }

    /// `links` reads the schema's `links` entries into link rules.
    fn links(&mut self) -> Vec<LinkRule> {
        let mut links = Vec::new();
        // The engine reads the `max` of a link rule as it reads every other
        // limit, and reports an empty `when-at-limit` and a `min` above it. The grammar's check
        // has already refused a bare number and a map of the wrong keys.
        let regexes = RegexCache::default();
        let mut limits = Lowerer::new(&regexes, self.path, TypeRegistry::default());
        for node in self.ext("links").and_then(|l| l.as_seq()).unwrap_or(&[]) {
            let (common, problems) = lower_common(node, &self.core.core, self.path);
            self.findings.extend(problems);
            let usize_of = |key: &str| {
                node.get(key)
                    .and_then(Spanned::as_int)
                    .map(|i| usize::try_from(i).unwrap_or(0))
            };
            let min = usize_of("min").unwrap_or(1);
            // sokf-implements: EXT-8_AC-4
            let max = limits.limit(node, "max");
            // A `min` above the count of the `max`, soft or hard, is a
            // mistake in the schema, as the engine finds for every other
            // limit.
            // sokf-implements: EXT-8_AC-7
            limits.check_bounds(node.line, min, max.as_ref());
            links.push(LinkRule {
                common,
                rel: node
                    .get("rel")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                to_type: node.get("to-type").map(list).unwrap_or_default(),
                min,
                max,
                guidance: node
                    .get("guidance")
                    .and_then(|g| g.as_str())
                    .map(str::to_string),
            });
        }
        self.findings.append(&mut limits.findings);
        links
    }

    /// `cited_by` reads the schema's `cited-by` entries into citation rules,
    /// and checks that each entry's `keys` names a set of keys that holds a
    /// key.
    fn cited_by(&mut self) -> Vec<CitationRule> {
        let path = self.path;
        // A citation entry's `keys` names a set of keys of this schema, the
        // `rule-id` of a section or a block or a `key-of`, and sokf resolves it
        // as the engine resolves `for-each-key-of`.
        // sokf-implements: EXT-9_AC-5
        let mut cited_by = Vec::new();
        for node in self.ext("cited-by").and_then(|l| l.as_seq()).unwrap_or(&[]) {
            let (common, problems) = lower_common(node, &self.core.core, path);
            self.findings.extend(problems);
            let keys = node
                .get("keys")
                .and_then(|k| k.as_str())
                .map(str::to_string);
            if let Some(k) = &keys
                && let Err(why) = self.core.rule_captures_key(k)
            {
                self.findings.push(fail(
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
            // sokf-implements: EXT-9_AC-9
            let message = node
                .get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string);
            cited_by.push(CitationRule {
                common,
                from,
                keys,
                message,
            });
        }
        cited_by
    }

    /// `markers` reads the schema's `markers`, the relationships of the
    /// markers that can cite a document of the type, and checks that each is
    /// in lowercase kebab case.
    fn markers(&mut self) -> Option<Vec<String>> {
        let markers_node = self.ext("markers");
        let markers = markers_node.map(list);
        // A tag is lowercase kebab case, so a rel of `markers` in another
        // form matches no marker, and the schema would refuse every marker
        // that its author meant.
        // sokf-implements: EXT-9_AC-11
        for item in markers_node.and_then(|n| n.as_seq()).unwrap_or(&[]) {
            let Some(rel) = item.as_str() else { continue };
            if crate::markdown::is_slug(rel) {
                continue;
            }
            let fix = match kebab(rel) {
                Some(k) => format!("Write it as `{k}`"),
                None => "Write it as lowercase words of letters and digits joined by `-`".into(),
            };
            self.findings.push(fail(
                self.path,
                item.line,
                "EXT-9_AC-11",
                format!(
                    "`markers` names the relationship `{rel}`, which isn't lowercase kebab case, so it matches no marker. {fix}"
                ),
            ));
        }
        markers
    }

    /// `plans_paths` reads the schema's `plans-paths` entries, and checks
    /// that each entry's `section` names a section of the schema.
    fn plans_paths(&mut self) -> Vec<PathPlan> {
        let path = self.path;
        // A path plan's `section` is the `rule-id` of a section of this
        // schema.
        // sokf-implements: SOKF_SCHEMA_52
        let mut plans_paths = Vec::new();
        for node in self
            .ext("plans-paths")
            .and_then(|l| l.as_seq())
            .unwrap_or(&[])
        {
            let (common, problems) = lower_common(node, &self.core.core, path);
            self.findings.extend(problems);
            let section = node
                .get("section")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string();
            if !self
                .core
                .sections
                .iter()
                .any(|s| s.rule_id.as_deref() == Some(section.as_str()))
            {
                self.findings.push(fail(
                    path,
                    node.line,
                    "SOKF_SCHEMA_52",
                    format!("`section: {section}` names no section of the schema; give the `rule-id` of a section"),
                ));
            }
            plans_paths.push(PathPlan { common, section });
        }
        plans_paths
    }
}

/// `path_dimensions` returns the keys of the lifecycle dimensions of
/// `schema` that declare `path`, in lifecycle order.
fn path_dimensions(schema: &Schema) -> Vec<String> {
    schema
        .core
        .lifecycle
        .iter()
        .filter(|d| {
            d.extensions
                .get("path")
                .and_then(Spanned::as_bool)
                .unwrap_or(false)
        })
        .map(|d| d.key.clone())
        .collect()
}

/// `unit_types` returns the `to-type` of each unit rule of `schema` that
/// declares one, by the rule's `Unit::index`: the rules of each block of a
/// section or of the preamble, of the blocks nested in a block or an item,
/// and of a fence's `lines`.
// sokf-implements: EXT-5_AC-2
fn unit_types(schema: &Schema) -> BTreeMap<usize, Vec<String>> {
    let mut out = BTreeMap::new();
    let mut blocks: Vec<&Block> = schema
        .sections
        .iter()
        .filter_map(|s| s.blocks.as_deref())
        .chain(schema.preamble.iter().filter_map(|p| p.blocks.as_deref()))
        .flatten()
        .collect();
    while let Some(b) = blocks.pop() {
        blocks.extend(b.blocks.iter().flatten());
        let mut units: Vec<&Unit> = b.units().collect();
        while let Some(u) = units.pop() {
            units.extend(u.lines.iter().flatten());
            blocks.extend(u.blocks.iter().flatten());
            if let Some(types) = u.extensions.get("to-type") {
                out.insert(u.index, list(types));
            }
        }
    }
    out
}

/// `asset_globs` checks the glob rules for `asset-files` (`SOKF_SCHEMA_8`,
/// `SOKF_SCHEMA_10`): each glob starts with /, parses, and lies inside the
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
