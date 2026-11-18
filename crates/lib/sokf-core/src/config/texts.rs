//! This module reads sokf's own texts in layers (CONF-12): sokf's English, then
//! the files of `/.sokf/texts.d/`, in which no file takes precedence over
//! another, then `/.sokf/texts.yaml`. An entry of each layer replaces the same
//! entry of the layer below. The module also looks a text up in a language.

use std::collections::BTreeMap;

use tramlines_core::text::is_language_tag;

use crate::{
    Error, RepoPath, RepoRoot, Result, Spanned, Yaml, grammar::squash, report::and_list, source,
    yaml,
};

/// `TEXTS_DIR` is the drop-in folder of the texts, relative to the
/// repository root.
const TEXTS_DIR: &str = ".sokf/texts.d";

/// `ENGLISH` holds sokf's own texts in English.
const ENGLISH: &str = include_str!("texts.yaml");

/// `UNITS` names each unit that a limit counts, whose forms the entry
/// `units` gives.
pub(crate) const UNITS: &[&str] = &[
    "paragraph",
    "item",
    "row",
    "line",
    "fence",
    "image",
    "definition",
    "repetition",
    "value",
    "link",
    "unit",
];

/// `LABELS` names each attribution label, whose words the entry `labels`
/// gives and whose meaning the entry `attribution` gives.
pub(crate) const LABELS: [&str; 2] = ["human", "agent"];

/// `texts_path` returns the path of the project's own file of texts.
fn texts_path() -> RepoPath {
    RepoPath::parse("/.sokf/texts.yaml").expect("literal")
}

/// `Text` is one entry of a language: words, or the two forms of a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    /// The words of a rule or a template.
    Words(String),
    /// The forms of a unit: `one` for a count of 1, `other` for every other
    /// count and for a ban.
    Forms {
        /// The form for a count of 1.
        one: String,
        /// The form for every other count.
        other: String,
    },
}

/// `Texts` holds sokf's texts, keyed by language, as a tag in lower case, and
/// by entry, such as `limit.hard` or `units.item`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Texts {
    entries: BTreeMap<(String, String), Text>,
}

impl Texts {
    /// `english` returns sokf's own English texts, without the layers of the
    /// repository.
    pub fn english() -> Texts {
        let path = RepoPath::parse("/crates/lib/sokf-core/src/config/texts.yaml").expect("literal");
        let entries = parse(&path, ENGLISH).expect("sokf's own texts parse");
        Texts {
            entries: entries
                .into_iter()
                .map(|e| ((e.language, e.entry), e.text))
                .collect(),
        }
    }

    /// `get` returns the entry `entry` of the language `language`: of the whole
    /// tag, in any case, else of the tag with its last subtags removed one at a
    /// time, down to the language alone, as the lookup of RFC 4647 does.
    // sokf-implements: CONF-12_AC-6
    pub(crate) fn get(&self, language: &str, entry: &str) -> Option<&Text> {
        lookup(language)
            .into_iter()
            .find_map(|tag| self.entries.get(&(tag, entry.to_string())))
    }

    /// `with` returns these texts with each entry of `text`, a file of
    /// texts, in place of the same entry.
    #[cfg(test)]
    pub(crate) fn with(mut self, text: &str) -> Texts {
        let path = RepoPath::parse("/.sokf/texts.yaml").expect("literal");
        for e in parse(&path, text).expect("the texts parse") {
            self.entries.insert((e.language, e.entry), e.text);
        }
        self
    }

    /// `words` returns the words of the entry `entry` of `language`.
    pub fn words(&self, language: &str, entry: &str) -> Option<&str> {
        match self.get(language, entry)? {
            Text::Words(w) => Some(w),
            Text::Forms { .. } => None,
        }
    }

    /// `label` returns the words of the label `key` in `language`, with each
    /// run of whitespace made one space, or `None` where the language has
    /// none.
    pub fn label(&self, language: &str, key: &str) -> Option<String> {
        self.squashed(language, &format!("labels.{key}"))
    }

    /// `attribution` returns the meaning of the label `key` in `language`,
    /// with each run of whitespace made one space, or `None` where the
    /// language has none.
    pub fn attribution(&self, language: &str, key: &str) -> Option<String> {
        self.squashed(language, &format!("attribution.{key}"))
    }

    /// `squashed` returns the words of the entry `entry` of `language`, with
    /// each run of whitespace made one space, or `None` where they are absent
    /// or blank.
    fn squashed(&self, language: &str, entry: &str) -> Option<String> {
        self.words(language, entry)
            .map(squash)
            .filter(|w| !w.is_empty())
    }

    /// `check_labels` refuses a language in which the labels `human` and
    /// `agent` take the same words, each label's own words or else its key,
    /// or in which the words of one label are the key of the other, because a
    /// document could not tell the two apart: the engine reads a label's key
    /// as the label where no label takes it as its words. `sources` names the
    /// file of each entry that a file of the repository gives, and `written`
    /// each language tag as a file writes it.
    // sokf-implements: CONF-12_AC-10
    fn check_labels(
        &self,
        sources: &BTreeMap<(String, String), RepoPath>,
        written: &BTreeMap<String, String>,
    ) -> Result<()> {
        let tags: std::collections::BTreeSet<&str> = self
            .entries
            .keys()
            .filter(|(_, entry)| entry.starts_with("labels."))
            .map(|(tag, _)| tag.as_str())
            .collect();
        for tag in tags {
            let [human, agent] =
                LABELS.map(|key| self.label(tag, key).unwrap_or_else(|| key.to_string()));
            // The fault, and the labels whose entries hold it, the later first.
            let (fault, at): (String, &[&str]) = if human == agent {
                (
                    format!("the labels `human` and `agent` both take the words `{human}`"),
                    &["agent", "human"],
                )
            } else if human == LABELS[1] {
                (
                    format!("the label `human` takes the words `{human}`, the key of `agent`"),
                    &["human"],
                )
            } else if agent == LABELS[0] {
                (
                    format!("the label `agent` takes the words `{agent}`, the key of `human`"),
                    &["agent"],
                )
            } else {
                continue;
            };
            // The file that gave an entry that holds the fault, where a file
            // of the repository gave one.
            let file = at
                .iter()
                .find_map(|key| {
                    let entry = format!("labels.{key}");
                    lookup(tag)
                        .into_iter()
                        .find(|t| self.entries.contains_key(&(t.clone(), entry.clone())))
                        .and_then(|t| sources.get(&(t, entry)))
                })
                .cloned()
                .unwrap_or_else(texts_path);
            let tag = written.get(tag).map_or(tag, String::as_str);
            return Err(Error::config(
                file,
                format!(
                    "in `{tag}`, {fault}, so a document could not tell the two labels apart; give \
                     them other words"
                ),
            ));
        }
        Ok(())
    }

    /// `forms` returns the two forms of the unit `unit` in `language`.
    pub fn forms(&self, language: &str, unit: &str) -> Option<(&str, &str)> {
        match self.get(language, &format!("units.{unit}"))? {
            Text::Forms { one, other } => Some((one, other)),
            Text::Words(_) => None,
        }
    }
}

/// `lookup` returns the tags to try for `language`, in lower case: the whole
/// tag, then the tag less each last subtag in turn, skipping a shorter tag
/// that ends in a single-letter subtag, which only introduces an extension.
fn lookup(language: &str) -> Vec<String> {
    let lower = language.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('-').collect();
    (1..=parts.len())
        .rev()
        .filter(|&n| n == parts.len() || n == 1 || parts[n - 1].len() > 1)
        .map(|n| parts[..n].join("-"))
        .collect()
}

/// `texts_files_lenient` returns `/.sokf/texts.yaml` and each file `*.yaml`
/// at any depth below `/.sokf/texts.d/`, as `load_texts` reads them, and
/// skips each directory of the folder that can't be read, so that the files
/// of every other directory still count.
pub fn texts_files_lenient(root: &RepoRoot) -> Vec<RepoPath> {
    let mut files = super::d_folder::d_files(root, TEXTS_DIR, "yaml", true).unwrap_or_default();
    files.push(texts_path());
    files
}

/// `load_texts` reads sokf's texts: its English, then each file `*.yaml` at
/// any depth below `/.sokf/texts.d/`, then `/.sokf/texts.yaml`. An entry of
/// a later layer replaces the same entry of an earlier one. It refuses a
/// file that doesn't parse as a file of texts, and an entry that two files
/// of the folder give different texts and `/.sokf/texts.yaml` doesn't give.
// sokf-implements: CONF-12_AC-1
// sokf-implements: CONF-12_AC-2
// sokf-implements: CONF-12_AC-3
pub fn load_texts(root: &RepoRoot) -> Result<Texts> {
    let mut texts = Texts::english();
    let mut folder: BTreeMap<(String, String), (Parsed, RepoPath)> = BTreeMap::new();
    let mut clashes: Vec<(RepoPath, Parsed, RepoPath)> = Vec::new();
    for path in super::d_folder::d_files(root, TEXTS_DIR, "yaml", false)? {
        let Some(text) = source::read_optional(&path.to_fs(root))? else {
            continue;
        };
        for e in parse(&path, &text)? {
            let key = (e.language.clone(), e.entry.clone());
            match folder.get(&key) {
                Some((first, _)) if first.text == e.text => {}
                Some((_, first_path)) => clashes.push((first_path.clone(), e, path.clone())),
                None => {
                    folder.insert(key, (e, path.clone()));
                }
            }
        }
    }
    let repo_path = texts_path();
    let repo = match source::read_optional(&repo_path.to_fs(root))? {
        Some(text) => parse(&repo_path, &text)?,
        None => Vec::new(),
    };
    let settled = |e: &Parsed| {
        repo.iter()
            .any(|r| r.language == e.language && r.entry == e.entry)
    };
    if let Some((first, e, later)) = clashes.iter().find(|(_, e, _)| !settled(e)) {
        return Err(Error::config(
            later.clone(),
            format!(
                "the text `{}` of `{}` is given differently in `{first}` and `{later}` of \
                 /{TEXTS_DIR}/, and no file of the folder takes precedence; give it in `{repo_path}` \
                 to choose one",
                e.entry, e.written
            ),
        ));
    }
    let mut sources: BTreeMap<(String, String), RepoPath> = BTreeMap::new();
    let mut written: BTreeMap<String, String> = BTreeMap::new();
    for (key, (e, path)) in folder {
        sources.insert(key.clone(), path);
        written.insert(e.language, e.written);
        texts.entries.insert(key, e.text);
    }
    for e in repo {
        let key = (e.language.clone(), e.entry);
        sources.insert(key.clone(), repo_path.clone());
        written.insert(e.language, e.written);
        texts.entries.insert(key, e.text);
    }
    texts.check_labels(&sources, &written)?;
    Ok(texts)
}

/// `Parsed` is one entry of a file of texts: its language in lower case and
/// as the file writes it, the entry, and its text.
#[derive(Debug, Clone)]
struct Parsed {
    language: String,
    written: String,
    entry: String,
    text: Text,
}

/// `parse` reads the file of texts `path`, whose text is `text`, as a map
/// from language tags to their entries, and refuses any other shape, a
/// language that the file gives twice, an entry that sokf doesn't define, a
/// placeholder that its entry doesn't define, and a template without the
/// count or the name that its entry states.
// sokf-implements: CONF-12_AC-4
// sokf-implements: CONF-12_AC-5
// sokf-implements: CONF-12_AC-8
fn parse(path: &RepoPath, text: &str) -> Result<Vec<Parsed>> {
    let fail = |line: usize, message: String| {
        Error::config(path.clone(), format!("line {line}: {message}"))
    };
    let doc = yaml::parse(text, 1).map_err(|e| Error::config(path.clone(), e.to_string()))?;
    if matches!(doc.value, Yaml::Null) {
        return Ok(Vec::new());
    }
    let Some(languages) = doc.as_map() else {
        return Err(fail(
            doc.line,
            "the file is no map from language tags to their texts".into(),
        ));
    };
    let mut out = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (tag, value) in languages {
        if !is_language_tag(&tag.value) {
            return Err(fail(
                tag.line,
                format!("`{}` is no language tag in the syntax of BCP 47", tag.value),
            ));
        }
        // A tag counts without case, so `EN` repeats `en`.
        if let Some(first) = seen.insert(tag.value.to_ascii_lowercase(), tag.line) {
            return Err(fail(
                tag.line,
                format!(
                    "`{}` gives the language of line {first} again; give each language once",
                    tag.value
                ),
            ));
        }
        let mut push = |entry: String, text: Text| {
            out.push(Parsed {
                language: tag.value.to_ascii_lowercase(),
                written: tag.value.clone(),
                entry,
                text,
            });
        };
        for (key, value) in map_of(value, &tag.value, &fail)? {
            match key.value.as_str() {
                "description" => push(
                    key.value.clone(),
                    words(value, &key.value, &[], &[], &fail)?,
                ),
                "rule-unit" => push(
                    key.value.clone(),
                    words(value, &key.value, &["name", "unit"], &["name"], &fail)?,
                ),
                "limit" => {
                    for (kind, value) in map_of(value, "limit", &fail)? {
                        if !["hard", "soft", "ban"].contains(&kind.value.as_str()) {
                            return Err(fail(
                                kind.line,
                                format!(
                                    "`limit.{}` is no text of sokf's; `limit` holds `hard`, `soft`, and `ban`",
                                    kind.value
                                ),
                            ));
                        }
                        let entry = format!("limit.{}", kind.value);
                        // A ban states no count, so only a hard and a soft
                        // limit need one.
                        let needs: &[&str] = if kind.value == "ban" { &[] } else { &["count"] };
                        let text = words(value, &entry, &["count", "unit"], needs, &fail)?;
                        push(entry, text);
                    }
                }
                "units" => {
                    for (unit, value) in map_of(value, "units", &fail)? {
                        if !UNITS.contains(&unit.value.as_str()) {
                            let names: Vec<String> =
                                UNITS.iter().map(|u| format!("`{u}`")).collect();
                            return Err(fail(
                                unit.line,
                                format!(
                                    "`units.{}` is no unit of sokf's; the units are {}",
                                    unit.value,
                                    and_list(&names)
                                ),
                            ));
                        }
                        let entry = format!("units.{}", unit.value);
                        push(entry.clone(), forms(value, &entry, &fail)?);
                    }
                }
                what @ ("labels" | "attribution") => labelled(value, what, &fail)?
                    .into_iter()
                    .for_each(|(entry, text)| push(entry, text)),
                other => {
                    return Err(fail(
                        key.line,
                        format!(
                            "`{other}` is no text of sokf's; a language holds `attribution`, `description`, `labels`, `limit`, `rule-unit`, and `units`"
                        ),
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// `labelled` returns each entry `<what>.<label>` of `value`, the map of the
/// entry `what`, `labels` or `attribution`, with its words. It refuses a
/// label other than `human` and `agent`, and words of `labels` that no label
/// could hold.
fn labelled(
    value: &Spanned,
    what: &str,
    fail: &dyn Fn(usize, String) -> Error,
) -> Result<Vec<(String, Text)>> {
    let mut out = Vec::new();
    for (label, value) in map_of(value, what, fail)? {
        if !LABELS.contains(&label.value.as_str()) {
            return Err(fail(
                label.line,
                format!(
                    "`{what}.{}` is no label of sokf's; the labels are `human` and `agent`",
                    label.value
                ),
            ));
        }
        let entry = format!("{what}.{}", label.value);
        let text = words(value, &entry, &[], &[], fail)?;
        if what == "labels" {
            label_words(&text, &entry, value.line, fail)?;
        }
        out.push((entry, text));
    }
    Ok(out)
}

/// `label_words` refuses the words `text` of the label `entry`, at `line`,
/// where they are blank, hold a square bracket, or begin with `^`, because
/// the engine reads such words as none and takes the key in their place.
// sokf-implements: CONF-12_AC-9
fn label_words(
    text: &Text,
    entry: &str,
    line: usize,
    fail: &dyn Fn(usize, String) -> Error,
) -> Result<()> {
    let Text::Words(w) = text else {
        return Ok(());
    };
    let words = squash(w);
    let fault = if words.is_empty() {
        "are blank"
    } else if words.contains(['[', ']']) {
        "hold a square bracket"
    } else if words.starts_with('^') {
        "begin with `^`, which marks a footnote"
    } else {
        return Ok(());
    };
    Err(fail(
        line,
        format!("the words of `{entry}` {fault}, so no label could hold them"),
    ))
}

/// `map_of` returns the entries of `value`, the value of `what`, and refuses
/// a value that isn't a map.
fn map_of<'v>(
    value: &'v Spanned,
    what: &str,
    fail: &dyn Fn(usize, String) -> Error,
) -> Result<&'v [(Spanned<String>, Spanned)]> {
    value
        .as_map()
        .ok_or_else(|| fail(value.line, format!("`{what}` is no map")))
}

/// `words` returns the string `value` of the entry `entry`, and refuses a
/// value that isn't a string, that holds a placeholder other than
/// `allowed`, or that lacks a placeholder of `needs`.
fn words(
    value: &Spanned,
    entry: &str,
    allowed: &[&str],
    needs: &[&str],
    fail: &dyn Fn(usize, String) -> Error,
) -> Result<Text> {
    let Some(text) = value.as_str() else {
        return Err(fail(value.line, format!("`{entry}` is no string")));
    };
    for name in placeholders(text) {
        if !allowed.contains(&name) {
            let takes = if allowed.is_empty() {
                "takes no placeholder".to_string()
            } else {
                let names: Vec<String> = allowed.iter().map(|a| format!("`{{{a}}}`")).collect();
                format!("takes {}", and_list(&names))
            };
            return Err(fail(
                value.line,
                format!("`{entry}` holds `{{{name}}}`, which is no placeholder of it; it {takes}"),
            ));
        }
    }
    if let Some(name) = needs.iter().find(|n| !placeholders(text).contains(n)) {
        return Err(fail(
            value.line,
            format!("`{entry}` holds no `{{{name}}}`, which it needs"),
        ));
    }
    Ok(Text::Words(text.to_string()))
}

/// `forms` returns the forms `one` and `other` of the unit `entry`, a map
/// that holds the two strings. It ignores any other form, such as `few`.
fn forms(value: &Spanned, entry: &str, fail: &dyn Fn(usize, String) -> Error) -> Result<Text> {
    let shape = || {
        fail(
            value.line,
            format!("`{entry}` is no map of the strings `one` and `other`"),
        )
    };
    value.as_map().ok_or_else(shape)?;
    let form = |name: &str| -> Result<String> {
        let v = value.get(name).ok_or_else(shape)?;
        match words(v, &format!("{entry}.{name}"), &[], &[], fail)? {
            Text::Words(w) => Ok(w),
            Text::Forms { .. } => Err(shape()),
        }
    };
    Ok(Text::Forms {
        one: form("one")?,
        other: form("other")?,
    })
}

/// `name_at` returns the name of the placeholder whose `{` stands just
/// before `after`, and where its `}` lies in `after`, or `None` where no
/// placeholder starts there. A name is one or more letters, digits, `-`,
/// or `_`, so `{}` is text.
fn name_at(after: &str) -> Option<(usize, &str)> {
    let end = after.find('}')?;
    let name = &after[..end];
    (!name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    .then_some((end, name))
}

/// `placeholders` returns the name of each placeholder `{name}` of `text`.
pub(crate) fn placeholders(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        match name_at(after) {
            Some((end, name)) => {
                out.push(name);
                rest = &after[end + 1..];
            }
            None => rest = after,
        }
    }
    out
}

/// `fill_template` returns `template` with each placeholder `{name}` of `values`
/// replaced by its value, in one pass, so that no value is read as a
/// template.
pub(crate) fn fill_template(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let hit = name_at(after).and_then(|(end, name)| {
            values
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (end, *v))
        });
        if let Some((end, value)) = hit {
            out.push_str(value);
            rest = &after[end + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
