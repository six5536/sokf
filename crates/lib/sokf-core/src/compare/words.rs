//! This module turns same-same's answer for a pair into sokf's reading of
//! it, and words the warning of a formatter that failed (CLI-4_AC-49) from
//! the reason that same-same gives as data.

use same_same::{Answer, NamedSet, Note, Unavailable, Verdict};

use super::{Pair, Reading, WARNING};
use crate::{Finding, RepoPath};

/// `reading` returns what sokf takes from `answer` for `pair`: the block is
/// current where same-same finds the texts the same, apart from the named
/// things of `same` at most, and stale otherwise, with the first line of the
/// source that differs and the warning at `origin` of a formatter that
/// failed or didn't start. `ranged` says that the expected text went to
/// same-same as the range of its file's lines, whose place names a line of
/// the file.
// sokf-implements: CLI-4_AC-109
// sokf-implements: CLI-4_AC-110
// sokf-implements: CLI-4_AC-114
// sokf-implements: CONF-10_AC-22
pub(super) fn reading(
    pair: &Pair,
    answer: &Answer,
    ranged: bool,
    same: NamedSet,
    origin: &RepoPath,
) -> Reading {
    let current = match &answer.verdict {
        Verdict::Same => true,
        // A named thing that sokf doesn't read as the same, such as how a
        // YAML 1.1 reader takes a value, or one that a later release of
        // same-same adds, is a difference.
        Verdict::SameApartFrom(named) => named.is_subset(same),
        _ => false,
    };
    if current {
        return Reading {
            current,
            ..Reading::default()
        };
    }
    // An include of a document's body, a definition block, and an index
    // have no file whose line the place names.
    let line = answer
        .place
        .as_ref()
        .zip(pair.source.as_ref())
        .and_then(|(place, source)| source_line(source, ranged, place.a.lines.start));
    let file = pair
        .source
        .as_ref()
        .map_or(&pair.path, |source| &source.path);
    let (warning, unsettled) = match warning(answer, file) {
        Some((message, unsettled)) => (
            Some(Finding::warning(origin.clone(), None, WARNING, message)),
            unsettled,
        ),
        None => (None, false),
    };
    Reading {
        current,
        line,
        warning,
        unsettled,
    }
}

/// `source_line` returns the line of the included file of the line `line`
/// of the input that stood for its expected text: the line itself for a
/// range of the file, where `ranged` holds, and otherwise the line of the
/// file that the expected text took that line from.
fn source_line(source: &super::Source, ranged: bool, line: u32) -> Option<usize> {
    let line = if ranged {
        line
    } else {
        let index = usize::try_from(line.checked_sub(1)?).ok()?;
        match source.lines.get(index) {
            Some(&line) => line,
            // A place after the last line lies after the last line taken.
            None if index == source.lines.len() => source.lines.last()? + 1,
            None => return None,
        }
    };
    usize::try_from(line).ok()
}

/// `warning` returns the message of the warning that the answer's notes
/// give for the texts of the file `file`, and whether the formatter left the
/// pair unsettled: a formatter that couldn't run, which the next run tries
/// again, comes before one that can't format a text, which same-same
/// remembers. A text on which the formatter ran past its limit is unsettled
/// too, because same-same keeps a hang only of a formatter that it runs with
/// its samples, which then costs the next run no wait, and a hang of a
/// command can be a busy machine's.
fn warning(answer: &Answer, file: &RepoPath) -> Option<(String, bool)> {
    let unavailable = answer.notes.iter().find_map(|note| match note {
        Note::NormaliserUnavailable { id, why } => {
            Some((unavailable_message(&id.name, why, file), true))
        }
        _ => None,
    });
    if unavailable.is_some() {
        return unavailable;
    }
    answer.notes.iter().find_map(|note| match note {
        Note::NormaliserCannot { id, .. } => Some((
            format!("the formatter {} fails on `{file}`", id.name),
            false,
        )),
        _ => None,
    })
}

/// `unavailable_message` says that the formatter `name` couldn't run on the
/// texts of the file `file`, and what same-same found wrong.
fn unavailable_message(name: &str, why: &Unavailable, file: &RepoPath) -> String {
    match why {
        Unavailable::Missing { program } if program.contains(['/', '\\']) => {
            format!("the formatter {name} doesn't start: `{program}` names no file")
        }
        Unavailable::Missing { program } => format!(
            "the formatter {name} doesn't start: `{program}` is in neither `node_modules/.bin` nor the path"
        ),
        Unavailable::DoesntStart { why } => format!("the formatter {name} doesn't start: {why}"),
        Unavailable::SampleFailed => {
            let sample = file.extension().map_or_else(
                || "sample".to_string(),
                |ending| format!("`.{ending}` sample"),
            );
            format!("the formatter {name} fails even on a plain {sample}; check its installation")
        }
        Unavailable::Hung { limit } => {
            let secs = limit.as_secs();
            let unit = if secs == 1 { "second" } else { "seconds" };
            format!(
                "the formatter {name} runs longer than `format.timeout`, {secs} {unit}, on `{file}`"
            )
        }
        Unavailable::Broken { why } => format!("the formatter {name} fails: {why}"),
        // The formatter has no sample of the file's kind, or the reason is one
        // that a later release of same-same adds.
        _ => format!("the formatter {name} fails on `{file}`"),
    }
}
