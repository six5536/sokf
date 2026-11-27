//! This module holds the shared helpers of the integration tests: a temporary
//! copy of a fixture repository, and the functions that run `validate` on it
//! and read its findings. Dropping the copy removes it.

#![allow(dead_code)]

pub mod adapter;
pub mod graph_rows;
pub mod incremental;
pub mod proptest_cases;
pub mod synthetic;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use sokf_core::{
    BlockGuide, Cache, CacheMode, Config, DocumentGuide, Finding, Model, NewOptions, NewResult,
    RepoRoot, Report, ValidateOptions, load_model, validate,
};

/// `with_hooks` returns a configuration with a `[harness.NAME]` table for each harness of
/// `harnesses` that opts it in to its hooks part, which is off by default. Each table ends
/// with its `with` line, so a test may append more keys of the last table.
pub fn with_hooks(harnesses: &[&str]) -> String {
    harnesses
        .iter()
        .map(|h| format!("[harness.{h}]\nwith = [\"hooks\"]\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `age` sets the modification time of every file under `dir` to `secs`
/// seconds in the past, so that a run sees the files as modified longer ago
/// than its racy margin. That margin is the time before the last run within
/// which a file might change without a change to its modification time, so
/// the run reads such a file again.
pub fn age(dir: &Path, secs: u64) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            age(&path, secs);
        } else {
            age_file(&path, secs);
        }
    }
}

/// `age_file` sets the modification time of one file to `secs` seconds in the
/// past.
pub fn age_file(path: &Path, secs: u64) {
    let f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(secs))
        .unwrap();
}

/// A `TempTree` is a repository under `std::env::temp_dir()`, which is removed
/// on drop.
pub struct TempTree {
    dir: PathBuf,
}

impl TempTree {
    /// `fixture` copies `tests/fixtures/<name>` into a new tree.
    pub fn fixture(name: &str) -> Self {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let tree = Self::empty(name);
        copy_dir(&source, &tree.dir);
        tree
    }

    /// `clean_with` copies the `clean` fixture into a new tree and writes
    /// each `(rel, text)` of `files` into it.
    pub fn clean_with(files: &[(&str, &str)]) -> Self {
        let tree = Self::fixture("clean");
        for (rel, text) in files {
            tree.write(rel, text);
        }
        tree
    }

    /// `empty` creates a tree that holds an empty repository.
    pub fn empty(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sokf-test-{name}-{}-{}",
            std::process::id(),
            unique()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempTree { dir }
    }

    /// `root` returns the repository root of the tree.
    pub fn root(&self) -> RepoRoot {
        RepoRoot::new(self.dir.clone())
    }

    /// `dir` returns the directory of the tree.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `write` writes `text` to the file at `rel`, and creates its parent
    /// directories.
    pub fn write(&self, rel: &str, text: &str) {
        let path = self.dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// `take_requirement_formats` writes the formats of the lines of
    /// requirements documents, such as `user-story` and `requirement-ears`,
    /// as `/.sokf/formats.yaml`, because the tool embeds no format of its
    /// own.
    pub fn take_requirement_formats(&self) {
        self.write(".sokf/formats.yaml", requirement_formats());
    }

    /// `add_other_store` makes `/other` a second knowledge root, with a
    /// manifest that names the store `test/other`.
    pub fn add_other_store(&self) {
        self.write(
            ".sokf/config.toml",
            "[knowledge]\nroots = [\"/knowledge\", \"/other\"]\n",
        );
        self.write(
            "other/manifest.sokf.yaml",
            "sokf: \"0.7\"\nname: other\nscope: test\n",
        );
    }

    /// `read` returns the text of the file at `rel`.
    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.dir.join(rel)).unwrap()
    }

    /// `drop_guidance` does what an author does with the guidance of a created
    /// document: it deletes each guidance line and, after a guidance HTML
    /// comment, the blank line that follows it.
    pub fn drop_guidance(&self, rel: &str) {
        let text = self.read(rel);
        let mut out: Vec<&str> = Vec::new();
        let mut skip_blank = false;
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("<!-- sokf:guidance") || t.starts_with("# sokf:guidance") {
                skip_blank = t.starts_with("<!--");
                continue;
            }
            if skip_blank && line.trim().is_empty() {
                skip_blank = false;
                continue;
            }
            skip_blank = false;
            out.push(line);
        }
        self.write(rel, &(out.join("\n") + "\n"));
    }

    /// `files` returns every file under the tree with its text, sorted by
    /// path. It leaves out the `.sokf` cache directory.
    pub fn files(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        walk(&self.dir, &self.dir, &mut out);
        out.sort();
        out
    }

    /// `cache_files` returns each file in the tree's cache directory with its
    /// bytes, sorted by name.
    pub fn cache_files(&self) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<(String, Vec<u8>)> = fs::read_dir(self.dir.join(".sokf/cache"))
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        (name, fs::read(e.path()).unwrap_or_default())
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }

    /// `model` loads every document of the tree, with the default
    /// configuration and without the cache.
    pub fn model(&self) -> Model {
        let root = self.root();
        load_model(
            &root,
            &Config::default(),
            &Cache::open(&root, CacheMode::Off),
        )
        .unwrap()
    }

    /// `validate` runs `validate` with the given options. A document that a
    /// test writes by hand has no anchors, and `--fix` writes a missing one.
    /// `validate` therefore drops the finding for a missing anchor, so that a
    /// test sees only the findings of the rule it tests. `validate_all` keeps
    /// that finding, for the anchor tests.
    pub fn validate(&self, fix: bool, cache: CacheMode) -> Report {
        let mut report = self.validate_all(fix, cache);
        report
            .errors
            .retain(|f| !f.message.contains("` has no anchor; its repair writes `"));
        report
    }

    /// `validate_all` runs `validate` with the given options, and keeps every
    /// finding.
    pub fn validate_all(&self, fix: bool, cache: CacheMode) -> Report {
        validate(
            &self.root(),
            &ValidateOptions {
                paths: vec![],
                cwd: self.dir.clone(),
                fix,
                cache,
                query: None,
                changed: vec![],
                home: None,
            },
        )
        .unwrap()
        .0
    }

    /// `findings` runs `validate` with the cache set to `mode`, and returns
    /// each error and then each warning as `path:line: message`.
    pub fn findings(&self, mode: CacheMode) -> Vec<String> {
        lines(&self.validate_all(false, mode))
    }

    /// `validate_paths` runs `validate` on `paths`, without the cache.
    pub fn validate_paths(&self, paths: &[&str], fix: bool) -> Report {
        validate(
            &self.root(),
            &ValidateOptions {
                paths: paths.iter().map(PathBuf::from).collect(),
                cwd: self.dir.clone(),
                fix,
                cache: CacheMode::Off,
                query: None,
                changed: vec![],
                home: None,
            },
        )
        .unwrap()
        .0
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn unique() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() == ".sokf" {
                continue;
            }
            walk(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, fs::read_to_string(&path).unwrap_or_default()));
        }
    }
}

/// `requirement_formats` returns the types and the registry of the
/// fixture's formats of the lines of requirements documents, in the shape of
/// `/.sokf/formats.yaml`.
pub fn requirement_formats() -> &'static str {
    let text = include_str!("../../fixtures/formats/requirements.formats.yaml");
    &text[text.find("\ntypes:\n").expect("the fixture holds types") + 1..]
}

/// `on` returns the findings among `findings` on a file named `file_name`.
pub fn on<'a>(findings: &'a [Finding], file_name: &'a str) -> impl Iterator<Item = &'a Finding> {
    findings
        .iter()
        .filter(move |f| f.path.file_name() == file_name)
}

/// `lined` returns a finding as `line: message`, with the message as `said`
/// returns it.
pub fn lined(f: &Finding) -> String {
    format!("{}: {}", f.line.unwrap_or(0), said(&f.message))
}

/// `reported` returns a finding as `line: message`, with the message as the
/// report prints it.
pub fn reported(f: &Finding) -> String {
    format!("{}: {}", f.line.unwrap_or(0), sokf_core::report_message(f))
}

/// `lines` returns each error and then each warning of a report as
/// `path:line: message`.
pub fn lines(report: &Report) -> Vec<String> {
    report
        .errors
        .iter()
        .chain(report.warnings.iter())
        .map(|f| format!("{}:{:?}: {}", f.path, f.line, f.message))
        .collect()
}

/// `error_lines` returns each error of a report as `path:line: message`.
pub fn error_lines(report: &Report) -> Vec<String> {
    report
        .errors
        .iter()
        .map(|f| format!("{}:{:?}: {}", f.path, f.line, f.message))
        .collect()
}

/// `new_options` returns the options of `sokf new` for `type_name`, run
/// from `cwd`, with every other field at its default.
pub fn new_options(cwd: &Path, type_name: &str) -> NewOptions {
    NewOptions {
        type_name: type_name.to_string(),
        slug: None,
        id: None,
        title: None,
        seq: None,
        states: vec![],
        dir: None,
        for_asset: None,
        cwd: cwd.to_path_buf(),
        write: false,
        id_only: false,
        cache: CacheMode::Off,
    }
}

/// `json` returns the report as pretty-printed JSON, for snapshots.
pub fn json(report: &Report) -> String {
    serde_json::to_string_pretty(report).unwrap()
}

/// `said` returns a message without the suffix that cites the source of its
/// rule, so that a test checks what the message says and not where the rule is
/// in the schema. The suffix can itself contain brackets, because a locator can
/// refer to a pattern. `said` therefore finds the suffix by the word that
/// opens it.
pub fn said(message: &str) -> String {
    if !message.ends_with(')') {
        return message.to_string();
    }
    // The suffix can be the key of a criterion, such as `(SOKF_FORMAT_53)` or
    // `(CLI-4_AC-8)`.
    if let Some(i) = message.rfind(" (") {
        let inner = &message[i + 2..message.len() - 1];
        if !inner.is_empty()
            && inner
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return message[..i].to_string();
        }
    }
    message
        .rfind(" (schema ")
        .map_or_else(|| message.to_string(), |i| message[..i].to_string())
}

/// `MISSING` lists the criteria of the findings that report a part of a
/// document as missing: a frontmatter value, a block, a section, a unit of a
/// rule, a block that another kind of block stands in place of, and a link.
pub const MISSING: &[&str] = &[
    "TL_143",
    "TLMD_18",
    "TLMD_69",
    "TLMD_93",
    "TLMD_146",
    "TLMD_152",
    "SOKF_SCHEMA_33",
];

/// `not_missing` returns each finding of `report` on the document at `path`
/// that doesn't report a part as missing, as `lined` writes it: what the
/// check of an untouched skeleton has to leave empty.
pub fn not_missing(report: &Report, path: &str) -> Vec<String> {
    report
        .errors
        .iter()
        .chain(&report.warnings)
        .filter(|f| f.path.as_str() == path && !MISSING.contains(&f.criterion.as_str()))
        .map(lined)
        .collect()
}

/// `examples` returns each example of the guide of `result`, in the order of
/// the guide: the frontmatter keys, the `links` rules, the preamble, and each
/// section, each followed by its blocks.
pub fn examples(result: &NewResult) -> Vec<String> {
    guide_examples(result.guide.as_ref().expect("a guide"))
}

/// `guide_examples` returns each example of `guide`, as `examples` does.
pub fn guide_examples(guide: &DocumentGuide) -> Vec<String> {
    fn blocks(list: &[BlockGuide], out: &mut Vec<String>) {
        for b in list {
            out.extend(b.example.clone());
            blocks(&b.blocks, out);
        }
    }
    let mut out: Vec<String> = guide
        .frontmatter
        .iter()
        .filter_map(|k| k.example.clone())
        .collect();
    out.extend(guide.links.iter().map(|l| l.example.clone()));
    if let Some(p) = &guide.preamble {
        out.extend(p.example.clone());
        blocks(&p.blocks, &mut out);
    }
    for s in &guide.sections {
        out.extend(s.example.clone());
        blocks(&s.blocks, &mut out);
    }
    out
}
