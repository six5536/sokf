//! The `formatter` module compares a generated block that differs from its
//! expected text by the form that the project's formatter gives both texts,
//! under settings of sokf's own (design-loading, "Comparing by layout").
//! A formatter changes the layout of a text and never its meaning, so two
//! texts with the same form mean the same.
//!
//! A pass of the check collects its pairs of texts that differ byte for
//! byte, and `Session::settle` formats every text that neither the session
//! nor the store of formatted texts holds, in one run of the formatter.

mod preset;
mod run;
mod sample;
mod store;

use std::{
    cell::{OnceCell, RefCell},
    collections::{BTreeSet, HashMap, HashSet},
    path::PathBuf,
};

pub use preset::{Formatter, PRESETS, find};
pub use store::FORMATTED_FILE;

use crate::{Finding, RepoPath, RepoRoot, config::FormatConfig, source::Fnv};

/// `WARNING` is the key of the warning that the formatter failed on a text
/// or didn't start (CLI-4_AC-49).
pub const WARNING: &str = "CLI-4_AC-49";

/// A `Pair` is a text that sokf would write and the text that a file holds,
/// which differ byte for byte, with the extension of the language in which
/// the formatter formats both, and the file that holds the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// The file that holds the text.
    pub path: RepoPath,
    /// The extension of the language of both texts, such as `json` or `md`.
    pub ext: String,
    /// The text that sokf would write.
    pub expected: String,
    /// The text that the file holds.
    pub actual: String,
}

/// `hash_parts` returns the hash of `parts`, each after its length, so
/// that no two lists of parts share their bytes.
fn hash_parts(parts: &[&str]) -> u64 {
    let mut h = Fnv::new();
    for part in parts {
        h.update(&(part.len() as u64).to_le_bytes());
        h.update(part.as_bytes());
    }
    h.finish()
}

/// `pair_key` returns the key of the pair of the texts `a` and `b` in the
/// language `ext`.
fn pair_key(ext: &str, a: &str, b: &str) -> u64 {
    hash_parts(&[ext, a, b])
}

/// `fingerprint` returns the hash of the formatter that the files of the
/// repository at `root` reveal: its command, its settings, and its version
/// where the `package.json` beside its program gives one, which costs no
/// process. The store of the cache records it, so that a new formatter, or
/// a new version of one, checks every document again (CACHE-1_AC-6).
pub fn fingerprint(root: &RepoRoot) -> u64 {
    let Some(f) = find(root, &FormatConfig::default()) else {
        return 0;
    };
    let version = run::program(root, &f.program)
        .zip(f.package.as_deref())
        .and_then(|(path, package)| run::package_version(root, &path, package))
        .unwrap_or_default();
    hash_parts(&[&f.identity(), &version])
}

/// A `Layout` records, for the pairs that a pass settled, which ones have
/// the same form and which ones the formatter failed on or left unsettled,
/// with the warning that a check adds where it meets such a pair, and the
/// files that hold an unsettled pair.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// The keys of the pairs whose two texts have the same form.
    same: HashSet<u64>,
    /// The keys of the pairs that the formatter fails on for good.
    failed: HashSet<u64>,
    /// The warning of each pair that the formatter left unsettled, which
    /// says why, by the key of the pair.
    unsettled_pairs: HashMap<u64, Finding>,
    /// The warning that the formatter fails on a text for good.
    failed_warning: Option<Finding>,
    /// The files that hold a pair that the formatter left unsettled.
    unsettled: BTreeSet<RepoPath>,
}

impl Layout {
    /// `same` returns whether the texts `a` and `b` in the language `ext`
    /// mean the same: whether they are equal, or the formatter gave them the
    /// same form (CLI-4_AC-44, CLI-4_AC-45).
    // sokf-implements: CLI-4_AC-44
    // sokf-implements: CLI-4_AC-45
    // sokf-implements: CLI-4_AC-48
    pub fn same(&self, ext: &str, a: &str, b: &str) -> bool {
        a == b || self.same.contains(&pair_key(ext, a, b))
    }

    /// `warning` returns the warning that the formatter failed, when it
    /// failed on the texts `a` or `b` or left them unsettled (CLI-4_AC-49).
    pub fn warning(&self, ext: &str, a: &str, b: &str) -> Option<Finding> {
        if a == b {
            return None;
        }
        let key = pair_key(ext, a, b);
        if let Some(warning) = self.unsettled_pairs.get(&key) {
            Some(warning.clone())
        } else if self.failed.contains(&key) {
            self.failed_warning.clone()
        } else {
            None
        }
    }

    /// `unsettled` returns the files that hold a pair whose texts the
    /// formatter left unsettled, which the next run checks again
    /// (CACHE-2_AC-9).
    pub fn unsettled(&self) -> &BTreeSet<RepoPath> {
        &self.unsettled
    }

    /// `extend` adds what `other` records to this layout.
    pub fn extend(&mut self, other: Layout) {
        self.same.extend(other.same);
        self.failed.extend(other.failed);
        self.unsettled_pairs.extend(other.unsettled_pairs);
        self.unsettled.extend(other.unsettled);
        if other.failed_warning.is_some() {
            self.failed_warning = other.failed_warning;
        }
    }
}

/// A `Session` holds what the runs of one command share of the formatter:
/// the formatter, its program and version, the forms of the texts that the
/// command formatted, and the store of formatted texts.
pub struct Session {
    /// The repository root.
    root: RepoRoot,
    /// Whether the session keeps the forms in the store file.
    on: bool,
    /// What the session found and formatted so far.
    state: RefCell<State>,
}

/// A `State` is what a session found and formatted so far.
#[derive(Default)]
struct State {
    /// The formatter and its program, once the session looked for it.
    found: OnceCell<Option<(Formatter, Option<PathBuf>)>>,
    /// The formatter's version, once the session read it, or why it gave
    /// none in time.
    version: OnceCell<Result<Option<String>, String>>,
    /// The store of formatted texts, once the session opened it.
    store: OnceCell<Option<store::Store>>,
    /// The form of each text by its extension and its text, or `None` where
    /// the formatter fails on it for good.
    forms: HashMap<(String, String), Option<String>>,
    /// The texts that the formatter left unsettled in this command, each
    /// with the warning that says why, which the command formats no more.
    unsettled: HashMap<(String, String), String>,
    /// Why the session formats nothing more: the formatter doesn't start,
    /// its environment fails it, or it lasted longer than its limit. The
    /// session still reads the store.
    stopped: Option<String>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("on", &self.on)
            .finish_non_exhaustive()
    }
}

/// `form_key` returns the key in the store of the form of `text` in the
/// language `ext`, by the formatter `identity` at `version` (CACHE-2_AC-6).
fn form_key(identity: &str, version: &str, ext: &str, text: &str) -> String {
    let hash = hash_parts(&[identity, version, ext, text]);
    format!("{hash:016x}{:08x}", text.len())
}

impl Session {
    /// `new` returns a session for the repository at `root`, which keeps the
    /// forms in the store file when `on` holds, and in memory otherwise
    /// (CACHE-2_AC-8).
    // sokf-implements: CACHE-2_AC-8
    pub fn new(root: &RepoRoot, on: bool) -> Session {
        Session {
            root: root.clone(),
            on,
            state: RefCell::new(State::default()),
        }
    }

    /// `settle` formats the texts of `pairs` with the formatter that
    /// `config` names or that the repository reveals, and returns the layout
    /// of the pairs. A text that the session or the store holds is not
    /// formatted again, nor a text whose pair's other text the formatter
    /// fails on for good, and the formatter runs once over the rest. With no
    /// pair, or no formatter, it starts no process (CACHE-3_AC-13,
    /// CLI-4_AC-48).
    // sokf-implements: CACHE-2_AC-5
    // sokf-implements: CACHE-2_AC-6
    // sokf-implements: CACHE-3_AC-13
    // sokf-implements: CLI-4_AC-48
    // sokf-implements: CLI-4_AC-49
    pub fn settle(&self, config: &FormatConfig, pairs: &[Pair]) -> Layout {
        let mut layout = Layout::default();
        if pairs.is_empty() {
            return layout;
        }
        let mut state = self.state.borrow_mut();
        let found = state
            .found
            .get_or_init(|| {
                find(&self.root, config).map(|f| {
                    let program = run::program(&self.root, &f.program);
                    (f, program)
                })
            })
            .clone();
        let Some((formatter, program)) = found else {
            return layout;
        };
        let pairs: Vec<&Pair> = pairs.iter().filter(|p| formatter.formats(&p.ext)).collect();
        if pairs.is_empty() {
            return layout;
        }
        let mut wanted: Vec<(String, String)> = Vec::new();
        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        for p in &pairs {
            for text in [&p.expected, &p.actual] {
                if !seen.insert((&p.ext, text)) {
                    continue;
                }
                let key = (p.ext.clone(), text.clone());
                if !state.forms.contains_key(&key) && !state.unsettled.contains_key(&key) {
                    wanted.push(key);
                }
            }
        }
        if !wanted.is_empty() {
            if let Some(program) = &program {
                self.format(&mut state, &formatter, program, &wanted, &pairs, config);
            } else if state.stopped.is_none() {
                let why = if formatter.program.contains(['/', '\\']) {
                    format!(
                        "`{}` names no file from the repository root",
                        formatter.program
                    )
                } else {
                    format!(
                        "`{}` is in neither `node_modules/.bin` nor the path",
                        formatter.program
                    )
                };
                state.stopped = Some(doesnt_start(&formatter, &why));
            }
        }
        // While the session formats nothing more, each text that it still
        // needs is unsettled for the same reason.
        if let Some(why) = state.stopped.clone() {
            for key in wanted {
                if !state.forms.contains_key(&key) {
                    state.unsettled.entry(key).or_insert_with(|| why.clone());
                }
            }
        }
        let warning = |m: String| Finding::warning(formatter.origin.clone(), None, WARNING, m);
        let mut failed_any = false;
        for p in pairs {
            let key = pair_key(&p.ext, &p.expected, &p.actual);
            let form = |text: &String| state.forms.get(&(p.ext.clone(), text.clone()));
            match (form(&p.expected), form(&p.actual)) {
                (Some(Some(a)), Some(Some(b))) => {
                    if a == b {
                        layout.same.insert(key);
                    }
                }
                // A text that the formatter fails on for good compares byte
                // for byte, whatever the form of the other text.
                (Some(None), _) | (_, Some(None)) => {
                    layout.failed.insert(key);
                    failed_any = true;
                }
                _ => {
                    let why = [&p.expected, &p.actual]
                        .into_iter()
                        .find_map(|t| state.unsettled.get(&(p.ext.clone(), t.clone())))
                        .cloned()
                        .unwrap_or_default();
                    layout.unsettled_pairs.insert(key, warning(why));
                    layout.unsettled.insert(p.path.clone());
                }
            }
        }
        layout.failed_warning = failed_any.then(|| {
            warning(format!(
                "the formatter {} failed on a text that sokf compares, so sokf compared that text byte for byte",
                formatter.name
            ))
        });
        layout
    }

    /// `format` finds the form of each of `wanted` in the store, leaving
    /// out a hang that reached a shorter limit than `timeout` of `config`,
    /// and, unless the session stopped, formats the rest that a pair of
    /// `pairs` still needs in one run of the formatter, and records each
    /// form and each failure in the session and the store, with the limit of
    /// each hang, and each unsettled text in the session alone.
    fn format(
        &self,
        state: &mut State,
        formatter: &Formatter,
        program: &std::path::Path,
        wanted: &[(String, String)],
        pairs: &[&Pair],
        config: &FormatConfig,
    ) {
        let version = state
            .version
            .get_or_init(|| {
                run::version(
                    &self.root,
                    program,
                    formatter.package.as_deref(),
                    config.timeout,
                )
            })
            .clone();
        let version = match (version, &formatter.package) {
            (Err(why), _) => {
                state.stopped = Some(doesnt_start(formatter, &why));
                return;
            }
            (Ok(Some(v)), _) => v,
            // A configured command may not answer `--version`, and its
            // batch then shows whether it starts.
            (Ok(None), None) => String::new(),
            (Ok(None), Some(_)) => {
                let why = format!("`{} --version` gives no version", program.display());
                state.stopped = Some(doesnt_start(formatter, &why));
                return;
            }
        };
        let identity = formatter.identity();
        let keys: Vec<String> = wanted
            .iter()
            .map(|(e, t)| form_key(&identity, &version, e, t))
            .collect();
        let held = state
            .store
            .get_or_init(|| self.on.then(|| store::Store::open(&self.root)).flatten())
            .as_ref()
            .map(|s| s.get(&keys, config.timeout))
            .unwrap_or_default();
        for (i, key) in wanted.iter().enumerate() {
            if let Some(form) = held.get(&keys[i]) {
                state.forms.insert(key.clone(), form.clone());
            }
        }
        if state.stopped.is_some() {
            return;
        }
        let (missing, others) = batch_of(wanted, pairs, &state.forms);
        if missing.is_empty() {
            return;
        }
        let texts: Vec<(&str, &str)> = missing
            .iter()
            .map(|&i| (wanted[i].0.as_str(), wanted[i].1.as_str()))
            .collect();
        let outcomes = match run::format(formatter, program, &texts, &others, config.timeout) {
            Ok(outcomes) => outcomes,
            Err(why) => {
                state.stopped = Some(doesnt_start(formatter, &why));
                return;
            }
        };
        let mut rows = Vec::new();
        for (&i, outcome) in missing.iter().zip(outcomes) {
            let (form, hang) = match outcome {
                run::Outcome::Form(form) => (Some(form), None),
                run::Outcome::Failed => (None, None),
                // A hang stops the command, as a text left untried after it
                // does, even when no text comes after it.
                run::Outcome::Hung(limit) => {
                    if state.stopped.is_none() {
                        state.stopped = Some(fault_message(formatter, run::Fault::Hang, config));
                    }
                    (None, Some(limit))
                }
                run::Outcome::Skipped => continue,
                run::Outcome::Unsettled(fault) => {
                    let why = fault_message(formatter, fault, config);
                    if fault.stops() && state.stopped.is_none() {
                        state.stopped = Some(why.clone());
                    }
                    state.unsettled.insert(wanted[i].clone(), why);
                    continue;
                }
            };
            rows.push((keys[i].clone(), form.clone(), hang));
            state.forms.insert(wanted[i].clone(), form);
        }
        if let Some(Some(store)) = state.store.get_mut() {
            store.put(&rows);
        }
    }
}

/// `batch_of` returns the places in `wanted` of the texts that the batch
/// formats: those that `forms` doesn't hold and that a pair of `pairs`
/// still needs, because its other text isn't one that the formatter fails
/// on for good. It also returns the other texts of the pairs of each of
/// them, by their place in the batch, or none where one lies outside it.
fn batch_of(
    wanted: &[(String, String)],
    pairs: &[&Pair],
    forms: &HashMap<(String, String), Option<String>>,
) -> (Vec<usize>, Vec<Vec<usize>>) {
    let failed: HashSet<(&str, &str)> = forms
        .iter()
        .filter(|(_, form)| form.is_none())
        .map(|((e, t), _)| (e.as_str(), t.as_str()))
        .collect();
    let mut partners: HashMap<(&str, &str), Vec<&str>> = HashMap::new();
    for p in pairs {
        for (a, b) in [(&p.expected, &p.actual), (&p.actual, &p.expected)] {
            partners.entry((&p.ext, a)).or_default().push(b);
        }
    }
    let missing: Vec<usize> = (0..wanted.len())
        .filter(|&i| {
            let (ext, text) = (wanted[i].0.as_str(), wanted[i].1.as_str());
            !forms.contains_key(&wanted[i])
                && partners
                    .get(&(ext, text))
                    .is_some_and(|o| o.iter().any(|other| !failed.contains(&(ext, *other))))
        })
        .collect();
    let place: HashMap<(&str, &str), usize> = missing
        .iter()
        .enumerate()
        .map(|(n, &i)| ((wanted[i].0.as_str(), wanted[i].1.as_str()), n))
        .collect();
    let others = missing
        .iter()
        .map(|&i| {
            let (ext, text) = (wanted[i].0.as_str(), wanted[i].1.as_str());
            partners
                .get(&(ext, text))
                .into_iter()
                .flatten()
                .map(|other| place.get(&(ext, *other)).copied())
                .collect::<Option<Vec<usize>>>()
                .unwrap_or_default()
        })
        .collect();
    (missing, others)
}

/// `doesnt_start` returns the warning that `formatter` doesn't start, for
/// the reason `why`.
fn doesnt_start(formatter: &Formatter, why: &str) -> String {
    format!(
        "the formatter {} doesn't start: {why}. sokf compared the generated blocks byte for byte",
        formatter.name
    )
}

/// `fault_message` returns the warning that `formatter` left texts
/// unsettled for the reason `fault`.
fn fault_message(formatter: &Formatter, fault: run::Fault, config: &FormatConfig) -> String {
    let name = &formatter.name;
    match fault {
        run::Fault::Sample => format!(
            "the formatter {name} failed on its sample, a text that every formatter that sokf knows formats, so its environment is at fault, such as an old Node.js or no network for its plugins. sokf compared the generated blocks byte for byte, and the next run formats them again"
        ),
        run::Fault::NoSample => format!(
            "the formatter {name} failed on a text in a language that sokf has no sample of, so sokf compared that text byte for byte, and the next run formats it again"
        ),
        run::Fault::Hang => {
            let secs = config.timeout.as_secs();
            let unit = if secs == 1 { "second" } else { "seconds" };
            format!(
                "the formatter {name} ran for longer than {secs} {unit} on a text, so sokf compared the texts byte for byte, and the next run formats those that it didn't reach"
            )
        }
        run::Fault::Broken => format!(
            "sokf couldn't run the formatter {name} again after its first run, or write the files for it, so sokf compared the texts byte for byte, and the next run formats them again"
        ),
    }
}

#[cfg(test)]
mod tests;
