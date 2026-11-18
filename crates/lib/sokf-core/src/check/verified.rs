//! This module checks whether each `verified` entry of a document has
//! lapsed: whether a change to the content of the document, or to the
//! asset of a sidecar, is later than the entry's `at` (§7 of the format
//! specification, and the concern "Lapsed entries" of design-operations).

use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use super::Ctx;
use crate::{
    Document, Finding, RepoPath, Source,
    history::{Change, Failed, Merge, When},
    model::{Query, core_fields, frontmatter::VerifiedEntry},
    source, yaml,
};

/// `check` reports each `verified` entry of `doc` that a later change to
/// the content of the document, or to its asset, lapsed, as a warning at
/// the line of the entry. The check reads the history only of a document
/// with an entry whose `at` it reads, and notes the query of the history
/// and of the asset's body, so that the cache checks the document again
/// after a commit or a change to the asset.
// sokf-implements: CLI-4_AC-92
pub fn check(doc: &Document, ctx: &Ctx<'_>) -> Vec<Finding> {
    if ctx.scaffold {
        return Vec::new();
    }
    let entries: Vec<(&VerifiedEntry, i64)> = doc
        .core
        .verified
        .iter()
        .filter_map(|e| Some((e, deadline(&e.at)?)))
        .collect();
    if entries.is_empty() {
        return Vec::new();
    }
    let history = ctx.model.history();
    ctx.model.note(Query::History(history.head().to_string()));
    if !history.readable() {
        return Vec::new();
    }
    let asset = ctx.model.asset_of(&doc.path).map(|(_, a)| a.path.clone());
    if let Some(a) = &asset {
        ctx.model.note(Query::Body(a.clone()));
    }
    let Ok(changes) = history.changes(&doc.path, &doc.source.text, doc.id(), asset.as_ref()) else {
        // A `git` that failed leaves the history unknown, so the next run
        // checks the document again.
        ctx.model.note(Query::Run);
        return Vec::new();
    };
    // A sidecar's `resource` is its asset's path, which `--fix` writes, so
    // a move of the pair changes no content.
    let derived: &[&str] = if asset.is_some() {
        &["verified", "resource"]
    } else {
        &["verified"]
    };
    let reads: Vec<Read> = changes
        .document
        .into_iter()
        .map(|c| Read::of(&doc.path, c, derived))
        .collect();
    let mut branches = Branches::default();
    let mut findings = Vec::new();
    for (entry, deadline) in entries {
        let followed = branches.follow(&reads, entry, |merge| {
            Ok(history
                .merged(merge, asset.is_some())?
                .into_iter()
                .map(|changes| Branch {
                    reads: changes
                        .document
                        .into_iter()
                        .map(|c| Read::of(&doc.path, c, derived))
                        .collect(),
                    asset: changes.asset,
                })
                .collect())
        });
        let (added, merged) = followed.unwrap_or_else(|Failed| {
            // A `git` that failed leaves the merged branch unknown, so the
            // check reads the history of `HEAD` alone, and the next run
            // checks the document again.
            ctx.model.note(Query::Run);
            let added = reads.iter().find(|r| r.adds(entry));
            (
                added.map(|r| r.change.when.clone()).into_iter().collect(),
                Vec::new(),
            )
        });
        // The check counts no change of a commit that added the entry, or
        // of the working tree where the working tree added it, as later
        // than the entry.
        // sokf-implements: CLI-4_AC-97
        // sokf-implements: CLI-4_AC-98
        let candidates: Vec<(&Change, Option<&RepoPath>)> = reads
            .iter()
            .chain(merged.iter().flat_map(|b| &b.reads))
            .filter(|c| c.content)
            .map(|c| (&c.change, None))
            .chain(
                changes
                    .asset
                    .iter()
                    .chain(merged.iter().flat_map(|b| &b.asset))
                    .map(|c| (c, asset.as_ref())),
            )
            .filter(|(c, _)| !added.contains(&c.when))
            .collect();
        let lapse = candidates
            .iter()
            .filter(|(c, _)| c.time > deadline)
            .min_by_key(|(c, _)| c.time);
        if let Some((change, asset)) = lapse {
            findings.push(warning(doc, entry, change, *asset));
        } else if candidates.iter().any(|(c, _)| c.when == When::WorkingTree) {
            // A change of the working tree dates from the time of the run,
            // so the entry lapses once its `at` passes, which no stat
            // records: every run checks the document again.
            ctx.model.note(Query::Run);
        }
    }
    findings
}

/// `warning` returns the warning that `change`, to the document or to
/// `asset`, lapsed `entry`.
fn warning(
    doc: &Document,
    entry: &VerifiedEntry,
    change: &Change,
    asset: Option<&RepoPath>,
) -> Finding {
    let what = asset.map_or_else(
        || "the document".to_string(),
        |a| format!("the asset `{a}`"),
    );
    let when = match &change.when {
        When::Commit(id) => format!("in the commit `{}`", &id[..id.len().min(12)]),
        When::WorkingTree => "in changes that aren't committed".to_string(),
    };
    Finding::warning(
        doc.path.clone(),
        Some(entry.line),
        "CLI-4_AC-92",
        format!(
            "the `verified` entry of `{}` at `{}` has lapsed, because {what} changed after it {when}. Only `{}` renews the entry",
            entry.by, entry.at, entry.by
        ),
    )
}

/// A `Branch` is what the check reads from the merged branch of a merge
/// commit: the changes to the document and to the asset.
struct Branch {
    reads: Vec<Read>,
    asset: Vec<Change>,
}

/// `Branches` holds the merged branch of each merge commit that the check
/// of a document has read, by the merge commit, one `Branch` for each
/// parent after the first, so that the check reads each one once.
#[derive(Default)]
struct Branches(BTreeMap<String, Vec<Rc<Branch>>>);

impl Branches {
    /// `follow` returns the commits that added `entry`, and the merged
    /// branch of each merge among them whose branch adds the entry too.
    /// The first commit is the one of `reads` that added the entry. Where a
    /// commit is a merge, and the merged branch of one of its parents,
    /// which `read` reads, adds the entry and keeps it to the parent, the
    /// check follows the merge to the commit of that branch that added it,
    /// and so on. A merge whose merged branches don't hold the entry added
    /// the entry itself, so its branches are left out. It returns `Failed`
    /// where `read` fails.
    // sokf-implements: CLI-4_AC-105
    fn follow(
        &mut self,
        reads: &[Read],
        entry: &VerifiedEntry,
        read: impl Fn(&Merge) -> Result<Vec<Branch>, Failed>,
    ) -> Result<(Vec<When>, Vec<Rc<Branch>>), Failed> {
        let adder = |r: &Read| (r.change.when.clone(), r.change.merge.clone());
        let mut next = reads.iter().find(|r| r.adds(entry)).map(adder);
        let mut added = Vec::new();
        let mut merged = Vec::new();
        while let Some((when, merge)) = next.take() {
            let (When::Commit(commit), Some(merge)) = (&when, merge) else {
                added.push(when);
                break;
            };
            if !self.0.contains_key(commit) {
                let branches = read(&merge)?.into_iter().map(Rc::new).collect();
                self.0.insert(commit.clone(), branches);
            }
            added.push(when.clone());
            // The newest change of a branch that adds or drops the entry
            // tells whether the branch holds it.
            for branch in &self.0[commit] {
                let last = branch
                    .reads
                    .iter()
                    .find(|r| r.adds(entry) || r.drops(entry));
                if let Some(r) = last.filter(|r| r.adds(entry)) {
                    next = Some(adder(r));
                    merged.push(Rc::clone(branch));
                    break;
                }
            }
        }
        Ok((added, merged))
    }
}

/// A `Read` is a change to the document, with what the check reads from
/// its texts: whether it changes the content, and the entries before and
/// after it.
struct Read {
    change: Change,
    content: bool,
    before: BTreeSet<(String, String)>,
    after: BTreeSet<(String, String)>,
}

impl Read {
    /// `of` reads `change` to the document at `path`, without the keys of
    /// `derived`. A text that is absent or unreadable holds no entry, and a
    /// change to it changes the content.
    fn of(path: &RepoPath, change: Change, derived: &[&str]) -> Read {
        let before = change.before.as_deref().map(|t| parse(path, t, derived));
        let after = change.after.as_deref().map(|t| parse(path, t, derived));
        let content = match (&before, &after) {
            (Some((a, _)), Some((b, _))) => a != b,
            _ => true,
        };
        Read {
            content,
            before: before.map(|(_, e)| e).unwrap_or_default(),
            after: after.map(|(_, e)| e).unwrap_or_default(),
            change,
        }
    }

    /// `adds` returns whether the change adds `entry`: the text after it
    /// holds the entry, by its `by` and its `at`, and the text before it
    /// doesn't.
    fn adds(&self, entry: &VerifiedEntry) -> bool {
        let key = (entry.by.clone(), entry.at.clone());
        self.after.contains(&key) && !self.before.contains(&key)
    }

    /// `drops` returns whether the change drops `entry`: the text before it
    /// holds the entry, by its `by` and its `at`, and the text after it
    /// doesn't.
    fn drops(&self, entry: &VerifiedEntry) -> bool {
        let key = (entry.by.clone(), entry.at.clone());
        self.before.contains(&key) && !self.after.contains(&key)
    }
}

/// `parse` returns the content of `text`, without the lines of each key
/// of `derived`, and its `verified` entries. A key's lines run to the next
/// key of the frontmatter, or to the end of the frontmatter, so that a
/// comment among the entries goes with them. A text whose frontmatter is
/// absent or doesn't parse is its content whole, with no entry. Every
/// content ends without the last line break, so that only the keys make a
/// difference.
// sokf-implements: CLI-4_AC-96
fn parse(path: &RepoPath, text: &str, derived: &[&str]) -> (String, BTreeSet<(String, String)>) {
    let source = Source::from_text(path.clone(), text);
    let split = source::split(&source);
    let mut cut: Vec<std::ops::Range<usize>> = Vec::new();
    let mut entries = BTreeSet::new();
    if let Some((front, line)) = split.frontmatter {
        let core = core_fields(Some(yaml::parse(front, line).as_ref()), path);
        // The frontmatter's closing line comes before the body's first line.
        let close = split.body_line.saturating_sub(1);
        for key in derived {
            if let Some(start) = core.line_of(key) {
                let end = core
                    .lines
                    .values()
                    .copied()
                    .filter(|&l| l > start)
                    .min()
                    .unwrap_or(close);
                cut.push(start..end);
            }
        }
        entries = core
            .verified
            .iter()
            .map(|e| (e.by.clone(), e.at.clone()))
            .collect();
    }
    let content: Vec<&str> = text
        .lines()
        .enumerate()
        .filter(|(i, _)| !cut.iter().any(|r| r.contains(&(i + 1))))
        .map(|(_, l)| l)
        .collect();
    (content.join("\n"), entries)
}

/// `deadline` returns the last second, in seconds since the epoch, that
/// an `at` covers: the last second of the day for a date alone, and the
/// second of the time otherwise, in UTC where the time gives no offset. A
/// fraction of a second drops. It returns `None` for an `at` that isn't an
/// ISO 8601 date or datetime.
// sokf-implements: CLI-4_AC-99
// sokf-implements: CLI-4_AC-100
fn deadline(at: &str) -> Option<i64> {
    let num = |text: &str| text.parse::<i64>().ok();
    let (date, time) = match at.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (at, None),
    };
    let mut parts = date.split('-');
    let (year, month, day) = (
        num(parts.next()?)?,
        num(parts.next()?)?,
        num(parts.next()?)?,
    );
    if parts.next().is_some()
        || !(1..=12).contains(&month)
        || !(1..=days_in(year, month)).contains(&day)
    {
        return None;
    }
    let midnight = days_from_civil(year, month, day) * 86_400;
    let Some(time) = time else {
        return Some(midnight + 86_399);
    };
    // The offset begins at `Z`, `+`, or `-`, after the clock.
    let (clock, offset) = match time.find(['Z', '+', '-']) {
        Some(i) => time.split_at(i),
        None => (time, ""),
    };
    let mut fields = clock.split(':');
    let hours = num(fields.next()?)?;
    let minutes = num(fields.next()?)?;
    let seconds = match fields.next() {
        Some(field) => num(field.split('.').next()?)?,
        None => 0,
    };
    let offset = match offset {
        "" | "Z" => 0,
        offset => {
            let sign = if offset.starts_with('-') { -1 } else { 1 };
            let digits: String = offset[1..].chars().filter(char::is_ascii_digit).collect();
            if digits.len() != 4 {
                return None;
            }
            sign * (num(&digits[..2])? * 3600 + num(&digits[2..])? * 60)
        }
    };
    // A leap second reads as the last second of its minute.
    if !(0..24).contains(&hours) || !(0..60).contains(&minutes) || !(0..=60).contains(&seconds) {
        return None;
    }
    Some(midnight + hours * 3600 + minutes * 60 + seconds.min(59) - offset)
}

/// `days_in` returns the number of days of the month `month` of `year`.
fn days_in(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `days_from_civil` returns the number of days from 1970-01-01 to the
/// date `y`-`m`-`d` of the proleptic Gregorian calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // The year starts in March, so that the leap day ends it.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_alone_ends_at_the_last_second_of_its_day() {
        assert_eq!(deadline("1970-01-01"), Some(86_399));
        assert_eq!(deadline("2026-02-01"), Some(1_769_990_399));
        assert_eq!(deadline("2000-02-29"), Some(951_868_799));
    }

    #[test]
    fn a_time_reads_its_offset_and_utc_without_one() {
        assert_eq!(deadline("2026-02-01T10:00"), Some(1_769_940_000));
        assert_eq!(deadline("2026-02-01T10:00:00Z"), Some(1_769_940_000));
        assert_eq!(deadline("2026-02-01T12:00:00+02:00"), Some(1_769_940_000));
        assert_eq!(deadline("2026-02-01T08:30:00.75-0130"), Some(1_769_940_000));
        assert_eq!(deadline("2026-02-01T10:00:30"), Some(1_769_940_030));
    }

    #[test]
    fn a_text_that_isnt_a_date_has_no_deadline() {
        for at in [
            "soon",
            "2026",
            "2026-13-01",
            "2026-01-01-01",
            "2026-01-01T1",
            "2026-01-01T10:00+1",
        ] {
            assert_eq!(deadline(at), None, "{at}");
        }
    }

    #[test]
    fn the_content_leaves_out_the_verified_key_alone() {
        let path = RepoPath::parse("/k/a.md").unwrap();
        let one =
            "---\nid: a\nverified:\n  - { by: human:a, at: 2026-01-01 }\ntitle: T\n---\n\nBody\n";
        let two = "---\nid: a\nverified:\n  - { by: human:a, at: 2026-01-01 }\n  - { by: human:b, at: 2026-02-01 }\ntitle: T\n---\n\nBody\n";
        let (a, ea) = parse(&path, one, &["verified"]);
        let (b, eb) = parse(&path, two, &["verified"]);
        assert_eq!(a, b);
        assert_eq!(a, "---\nid: a\ntitle: T\n---\n\nBody");
        let commented = "---\nid: a\nverified:\n# reviews\n  - { by: human:a, at: 2026-01-01 }\ntitle: T\n---\n\nBody\n";
        assert_eq!(parse(&path, commented, &["verified"]).0, a);
        let last =
            "---\nid: a\ntitle: T\nverified:\n- { by: human:a, at: 2026-01-01 }\n---\n\nBody\n";
        assert_eq!(parse(&path, last, &["verified"]).0, a);
        let none = "---\nid: a\ntitle: T\n---\n\nBody\n";
        assert_eq!(parse(&path, none, &["verified"]).0, a);
        assert_eq!(ea.len(), 1);
        assert_eq!(eb.len(), 2);
        let (c, ec) = parse(&path, "no frontmatter\n", &["verified"]);
        assert_eq!(c, "no frontmatter");
        assert!(ec.is_empty(), "{ec:?}");
    }
}
