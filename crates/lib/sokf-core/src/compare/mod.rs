//! The `compare` module asks the library same-same whether a generated block
//! that differs from its expected text byte for byte means the same as it
//! (design-loading, "Comparing by meaning"). same-same compares the formats
//! that it knows by their meaning, and any other format through the project's
//! formatter, or else by the bytes. The module builds the two sides of each
//! pair, reads each answer as current or stale, and words the warning of a
//! formatter that failed (CLI-4_AC-49). It passes same-same no type of sokf.
//!
//! A pass of the check collects its pairs, and `Session::settle` compares
//! them in one batch, so that the check and the fix read the same answers
//! from the `Layout` of `Expected`.

mod names;
mod session;
mod words;

use std::{
    collections::{BTreeSet, HashMap},
    ops::Range,
};

pub(crate) use names::name_of;
pub use session::Session;

use crate::{Finding, RepoPath};

/// `WARNING` is the key of the warning that the formatter failed on a text
/// or didn't start (CLI-4_AC-49).
pub const WARNING: &str = "CLI-4_AC-49";

/// A `Pair` is a text that sokf would write and the text that a file holds,
/// which differ byte for byte, with the name under which same-same reads
/// both, and the file that holds the text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pair {
    /// The file that holds the text.
    pub path: RepoPath,
    /// The name of both texts, the path of their file from the repository's
    /// root, such as `src/package.json` or `.vscode/settings.json`, which
    /// tells same-same their format by the file's name and, for a pattern of
    /// two parts, its folder (CLI-4_AC-113).
    pub name: String,
    /// Whether both texts are a part of a file, read without the indentation
    /// that all their lines share.
    pub fragment: bool,
    /// The text that sokf would write.
    pub expected: String,
    /// The text that the file holds.
    pub actual: String,
    /// Where the expected text lies in its source, for an include of a file.
    pub source: Option<Source>,
}

/// A `Source` is where the expected text of an include of a file lies in
/// the file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Source {
    /// The included file.
    pub path: RepoPath,
    /// The lines of the file that the expected text is, as one run of whole
    /// lines, for an include of a region, or `None` where the text leaves
    /// out a line of the run, or where it is the whole file.
    pub range: Option<Range<u32>>,
    /// The line of the file of each line of the expected text.
    pub lines: Vec<u32>,
}

/// A `Reading` is what sokf takes from same-same's answer for a pair.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Reading {
    /// Whether the block is current.
    current: bool,
    /// The first line of the included file that differs, for a stale
    /// include of a file where same-same gives it.
    line: Option<usize>,
    /// The warning that the formatter failed on the pair or didn't start.
    warning: Option<Finding>,
    /// Whether the formatter left the pair unsettled, so that the next run
    /// compares it again.
    unsettled: bool,
}

/// A `Layout` records what same-same answered for each pair that a pass
/// settled, and the files that hold a pair that the formatter left
/// unsettled.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// The reading of each pair.
    readings: HashMap<Pair, Reading>,
    /// The files that hold a pair that the formatter left unsettled.
    unsettled: BTreeSet<RepoPath>,
}

impl Layout {
    /// `same` returns whether the pair `pair` means the same: whether its
    /// texts are equal, or same-same finds them the same, apart from the
    /// named things that `[format]` reads as the same (CLI-4_AC-44,
    /// CLI-4_AC-45, CLI-4_AC-109).
    // sokf-implements: CLI-4_AC-44
    // sokf-implements: CLI-4_AC-45
    // sokf-implements: CLI-4_AC-48
    pub fn same(&self, pair: &Pair) -> bool {
        pair.expected == pair.actual || self.readings.get(pair).is_some_and(|r| r.current)
    }

    /// `warning` returns the warning that the formatter failed on the pair
    /// `pair` or didn't start, when it did and the pair is stale
    /// (CLI-4_AC-49).
    pub fn warning(&self, pair: &Pair) -> Option<Finding> {
        self.readings.get(pair).and_then(|r| r.warning.clone())
    }

    /// `line` returns the first line of the source that differs, for a
    /// stale pair where same-same gives it (CLI-4_AC-110).
    pub fn line(&self, pair: &Pair) -> Option<usize> {
        self.readings
            .get(pair)
            .filter(|r| !r.current)
            .and_then(|r| r.line)
    }

    /// `unsettled` returns the files that hold a pair that the formatter
    /// left unsettled, which the next run checks again (CACHE-2_AC-9).
    pub fn unsettled(&self) -> &BTreeSet<RepoPath> {
        &self.unsettled
    }

    /// `holds` returns whether the layout has a reading of the pair.
    pub(crate) fn holds(&self, pair: &Pair) -> bool {
        self.readings.contains_key(pair)
    }

    /// `extend` adds what `other` records to this layout.
    pub fn extend(&mut self, other: Layout) {
        self.readings.extend(other.readings);
        self.unsettled.extend(other.unsettled);
    }

    /// `insert` records the reading of a pair.
    fn insert(&mut self, pair: Pair, reading: Reading) {
        if reading.unsettled {
            self.unsettled.insert(pair.path.clone());
        }
        self.readings.insert(pair, reading);
    }
}

/// `stale_message` returns the message of a stale include block whose
/// target is written as `written`, with the first line of its source that
/// differs, where same-same gives it (CLI-4_AC-110).
pub fn stale_message(written: &str, source: Option<&RepoPath>, line: Option<usize>) -> String {
    match (source, line) {
        (Some(source), Some(line)) => {
            format!("`sokf:include {written}` is stale: `{source}` differs from line {line}")
        }
        _ => format!("`sokf:include {written}` is stale"),
    }
}

#[cfg(test)]
mod tests;
