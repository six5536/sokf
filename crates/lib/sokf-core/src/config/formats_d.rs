//! This module reads the files of `/.sokf/formats.d/`, the layer
//! `formats.d`, in which no file takes precedence over another (`TL_128`,
//! CONF-11): it drops the declarations that are alike, refuses those that
//! clash, gives the folder the types of `/.sokf/formats.yaml` that settle a
//! clash, parses each file, and refuses what tramlines' `with_host` finds
//! in the layer under `/.sokf/formats.yaml`.

use std::collections::{BTreeMap, btree_map::Entry};

use tramlines_core::grammar::BrokenFormat;

use super::formats::{formats_path, layer_of, line_message};
use crate::{
    Error, RepoPath, Result, Spanned, Yaml,
    grammar::{FormatRegistry, FormatSource, Grammar, PackLayer, RegexCache},
};

/// `settled_types` returns `below`, the layers under the folder, with the
/// declarations of `repo`, the path and the text of `/.sokf/formats.yaml`,
/// of each type that two of `docs`, the files of `/.sokf/formats.d/` as
/// `without_alike` leaves them, declare, or `None` where there is no such
/// type. It drops each such type from each of `docs`, so that every file of
/// the folder, and `with_packs` when it compiles a format of the folder
/// again, compiles against the declaration that the run takes, and
/// `with_packs` finds no clash in it. It also returns each format of
/// `below` that no longer compiles against such a type. The layers keep
/// each such format as `below` compiled it, so that a patch in
/// `/.sokf/formats.yaml` can merge onto it. `unsettled` refuses the format
/// unless `/.sokf/formats.yaml` declares it.
// sokf-implements: CONF-11_AC-10
pub(super) fn settled_types(
    docs: &mut [(RepoPath, Spanned)],
    repo: Option<(&RepoPath, &Spanned)>,
    below: &FormatRegistry,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<Option<(FormatRegistry, Vec<BrokenFormat>)>> {
    let Some((repo_path, repo_doc)) = repo else {
        return Ok(None);
    };
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for (_, doc) in docs.iter() {
        for (name, _) in doc
            .get("types")
            .and_then(Spanned::as_map)
            .unwrap_or_default()
        {
            *count.entry(name.value.clone()).or_default() += 1;
        }
    }
    let declared = |name: &str| repo_doc.get("types").and_then(|t| t.get(name)).is_some();
    let names: Vec<String> = count
        .into_iter()
        .filter(|(name, n)| *n > 1 && declared(name))
        .map(|(name, _)| name)
        .collect();
    if names.is_empty() {
        return Ok(None);
    }
    for (_, doc) in docs.iter_mut() {
        if let Some(Yaml::Map(types)) = map_mut(doc, "types") {
            types.retain(|(name, _)| !names.contains(&name.value));
        }
    }
    // Only the settled types compile here, so that any other fault of the
    // file is reported where the file is parsed as the repository layer.
    let kept = repo_doc
        .get("types")
        .and_then(Spanned::as_map)
        .unwrap_or_default()
        .iter()
        .filter(|(name, _)| names.contains(&name.value))
        .cloned()
        .collect();
    let only = Spanned::new(
        Yaml::Map(vec![(
            Spanned::new("types".to_string(), repo_doc.line),
            Spanned::new(Yaml::Map(kept), repo_doc.line),
        )]),
        repo_doc.line,
    );
    let none = Spanned::new(Yaml::Map(Vec::new()), 1);
    let types = layer_of(
        repo_path,
        &only,
        &none,
        FormatSource::Repo,
        grammar,
        regexes,
        below,
    )?
    .types;
    // Each format below that uses a settled type compiles against it, as a
    // schema sees it. The settled type is the same in `/.sokf/formats.yaml`,
    // so `with_host` doesn't compile such a format again.
    let settled = FormatRegistry {
        types,
        ..FormatRegistry::default()
    };
    let (mut out, broken) = below.layered(&settled);
    for b in &broken {
        if let Some(entry) = below.entries.get(&b.name) {
            out.entries.insert(b.name.clone(), entry.clone());
        }
    }
    Ok(Some((out, broken)))
}

/// `unsettled` refuses each format of `broken` unless `repo`, the repository
/// layer, declares it in full or patches it, and reports tramlines' findings on
/// `/.sokf/formats.yaml`. A format of `broken` lies below the folder and no
/// longer compiles against a type that `/.sokf/formats.yaml` settles for the
/// folder.
// sokf-implements: CONF-11_AC-12
pub(super) fn unsettled(broken: &[BrokenFormat], repo: &FormatRegistry) -> Result<()> {
    let path = formats_path();
    let findings: Vec<crate::Finding> = broken
        .iter()
        .filter(|b| !repo.entries.contains_key(&b.name))
        .flat_map(|b| b.findings(&path, None))
        .collect();
    refuse(&findings)
}

/// `folder_files` parses each of `docs`, the files of `/.sokf/formats.d/`,
/// on `below`, the layers under the folder, with the types of every file of
/// the folder on top.
pub(super) fn folder_files(
    docs: &[(RepoPath, Spanned)],
    below: &FormatRegistry,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<Vec<(RepoPath, FormatRegistry)>> {
    // Each file sees the types of every file of the folder, so that a
    // format of one file can use a type that another declares. No two
    // files declare a type here, because `check_clashes` refuses a type
    // that two files declare differently unless `/.sokf/formats.yaml`
    // settles it, and `settled_types` then drops it from both.
    // sokf-implements: CONF-11_AC-8
    let mut shared = below.clone();
    for (path, doc) in docs.iter().rev() {
        let none = Spanned::new(Yaml::Map(Vec::new()), 1);
        let types = layer_of(path, doc, &none, source_of(path), grammar, regexes, below)?;
        shared.types = shared.types.layered(&types.types);
    }
    let mut files = Vec::new();
    for (path, doc) in docs {
        let registry = doc
            .get("registry")
            .ok_or_else(|| Error::formats(path.clone(), "The file has no `registry` map."))?;
        let file = layer_of(
            path,
            doc,
            registry,
            source_of(path),
            grammar,
            regexes,
            &shared,
        )?;
        files.push((path.clone(), file));
    }
    Ok(files)
}

/// `refuse_findings` lays `repo`, the repository layer, on `folder`, the
/// layer `formats.d`, with tramlines' `with_host`, and returns the registry.
/// It refuses what `with_host` finds, with the findings of every file:
///
/// - each format of the folder that no longer compiles;
/// - each clash that `with_host`'s rule finds and sokf's `check_clashes`
///   doesn't;
/// - on `/.sokf/formats.yaml`, each format of the layers under `repo` that a
///   type of `repo` breaks;
/// - on the file that declares the first type of the folder that breaks
///   the format, each format below the folder that a type of the folder
///   breaks and `repo` leaves broken.
// sokf-implements: CONF-11_AC-11
// sokf-implements: CONF-11_AC-12
pub(super) fn refuse_findings(folder: &PackLayer, repo: &FormatRegistry) -> Result<FormatRegistry> {
    let (registry, findings) = folder.with_host(repo, &formats_path());
    refuse(&findings)?;
    Ok(registry)
}

/// `refuse` stops the run with the findings `findings` of formats files, on
/// the file of the first, each of another file with its path.
fn refuse(findings: &[crate::Finding]) -> Result<()> {
    if let Some(first) = findings.first() {
        let messages: Vec<String> = findings
            .iter()
            .map(|f| match &f.path {
                p if *p == first.path => line_message(f),
                p => format!("{p}: {}", line_message(f)),
            })
            .collect();
        return Err(Error::formats(first.path.clone(), messages.join("; ")));
    }
    Ok(())
}

/// `MAPS` names the two maps of a formats file that declare names, with
/// what each declares: the formats of `registry` and the types of `types`.
const MAPS: [(&str, &str); 2] = [("format", "registry"), ("type", "types")];

/// `without_alike` drops from `docs`, the files of `/.sokf/formats.d/` in
/// the order of their paths, each format and each type that is alike with
/// the declaration of the first file that declares the name, so that the
/// first file's declaration stands for the name. A declaration that differs
/// from the first stays, for `check_clashes` to report, with each later one
/// alike with it, so that the error names every file that disagrees.
/// `below`, the layers under the folder, holds the entry onto which a patch
/// merges.
// sokf-implements: CONF-11_AC-9
pub(super) fn without_alike(docs: &mut [(RepoPath, Spanned)], below: &FormatRegistry) {
    let mut first: BTreeMap<(&str, String), Spanned> = BTreeMap::new();
    for (_, doc) in docs {
        for (what, map) in MAPS {
            let Some(Yaml::Map(entries)) = map_mut(doc, map) else {
                continue;
            };
            entries.retain(
                |(name, value)| match first.entry((map, name.value.clone())) {
                    Entry::Occupied(earlier) => {
                        // Only a format merges onto an entry below.
                        let base = below
                            .get(&name.value)
                            .filter(|_| what == "format")
                            .map(|f| &f.raw);
                        !alike(earlier.get(), value, base)
                    }
                    Entry::Vacant(slot) => {
                        slot.insert(value.clone());
                        true
                    }
                },
            );
        }
    }
}

/// `is_patch` returns whether the entry `value` of a formats file is a
/// `patch: true` entry, which changes the entry of its name below it.
pub(crate) fn is_patch(value: &Spanned) -> bool {
    value.get("patch").and_then(Spanned::as_bool) == Some(true)
}

/// `map_mut` returns the value of the key `key` of the map `doc`.
fn map_mut<'a>(doc: &'a mut Spanned, key: &str) -> Option<&'a mut Yaml> {
    let Yaml::Map(entries) = &mut doc.value else {
        return None;
    };
    entries
        .iter_mut()
        .find(|(k, _)| k.value == key)
        .map(|(_, v)| &mut v.value)
}

/// `alike` returns whether two declarations of one name are alike: their
/// parsed values are equal, whatever their lines and the order of the keys
/// of each map, and a format's `variants` come in the same order, because a
/// line tries them in that order. `base` is the entry of the layers below
/// with the name: where `base` has variants, a patch merges each variant
/// that `base` holds in place, adds the others at the end, and drops a null
/// that deletes nothing, so only the order of the variants that it adds
/// counts.
fn alike(a: &Spanned, b: &Spanned, base: Option<&Spanned>) -> bool {
    // A patch merges onto the variants of the entry below only where that
    // entry has a map of them, and otherwise brings its own whole.
    let below = base
        .and_then(|e| e.get("variants"))
        .filter(|v| v.as_map().is_some() && is_patch(a));
    let counts = |name: &str, value: &Spanned| match below {
        Some(held) => held.get(name).is_none() && !matches!(value.value, Yaml::Null),
        None => true,
    };
    let order = |d: &Spanned| -> Option<Vec<String>> {
        let variants = d.get("variants")?.as_map()?;
        Some(
            variants
                .iter()
                .filter(|(k, v)| counts(&k.value, v))
                .map(|(k, _)| k.value.clone())
                .collect(),
        )
    };
    same_value(a, b) && order(a) == order(b)
}

/// `same_value` returns whether two values are equal, whatever their lines
/// and the order of the keys of each map.
fn same_value(a: &Spanned, b: &Spanned) -> bool {
    match (&a.value, &b.value) {
        (Yaml::Seq(x), Yaml::Seq(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_value(p, q))
        }
        (Yaml::Map(x), Yaml::Map(y)) => {
            x.len() == y.len()
                && x.iter().all(|(k, v)| {
                    y.iter()
                        .find(|(l, _)| l.value == k.value)
                        .is_some_and(|(_, w)| same_value(v, w))
                })
        }
        // `.nan` is one value to YAML, though it equals no float.
        (Yaml::Float(x), Yaml::Float(y)) => x.total_cmp(y).is_eq(),
        (x, y) => x == y,
    }
}

/// `check_clashes` refuses each format and each type that two of `docs`,
/// the files of `/.sokf/formats.d/` in the order of their paths, declare,
/// unless `repo`, the text of `/.sokf/formats.yaml`, declares it in full: a
/// format that is no patch, or a type. It reads the names of the files as
/// `without_alike` leaves them, without the declarations alike with the
/// first, before any file compiles. The error is on the later file,
/// and names the format or the type, the first file that declares it, and
/// the later one.
// sokf-implements: CONF-11_AC-2
// sokf-implements: CONF-11_AC-3
pub(super) fn check_clashes(docs: &[(RepoPath, Spanned)], repo: Option<&Spanned>) -> Result<()> {
    let names = |doc: &'_ Spanned, map: &str| -> Vec<String> {
        doc.get(map)
            .and_then(Spanned::as_map)
            .unwrap_or_default()
            .iter()
            .map(|(k, _)| k.value.clone())
            .collect()
    };
    let decided = |map: &str, name: &str| {
        repo.and_then(|r| r.get(map))
            .and_then(Spanned::as_map)
            .unwrap_or_default()
            .iter()
            .find(|(k, _)| k.value == name)
            .is_some_and(|(_, v)| !is_patch(v))
    };
    let mut clashes: Vec<(&RepoPath, String)> = Vec::new();
    for (what, map) in MAPS {
        let mut first: BTreeMap<String, &RepoPath> = BTreeMap::new();
        for (path, doc) in docs {
            for name in names(doc, map) {
                match first.get(&name) {
                    Some(earlier) if !decided(map, &name) => {
                        clashes.push((path, clash(what, &name, earlier, path)));
                    }
                    Some(_) => {}
                    None => {
                        first.insert(name, path);
                    }
                }
            }
        }
    }
    clashes.sort_by(|a, b| a.0.cmp(b.0));
    match clashes.first() {
        None => Ok(()),
        Some((path, _)) => Err(Error::formats(
            (*path).clone(),
            clashes
                .iter()
                .map(|(_, m)| m.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )),
    }
}

/// `clash` says that `what`, a format or a type, named `name`, is declared
/// in both `first` and `later` with declarations that aren't alike.
fn clash(what: &str, name: &str, first: &RepoPath, later: &RepoPath) -> String {
    format!(
        "{what} `{name}` is declared differently in `{first}` and `{later}` of \
         /.sokf/formats.d/, and no file of the folder takes precedence; declare \
         it in full in `{}` to choose one",
        formats_path()
    )
}

/// `source_of` returns the source of the entries of the file `path` of
/// `/.sokf/formats.d/`: its path (CLI-5_AC-43).
fn source_of(path: &RepoPath) -> FormatSource {
    FormatSource::File(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml;

    fn value(text: &str) -> Spanned {
        yaml::parse(text, 1).unwrap()
    }

    #[test]
    fn alike_compares_parsed_values_and_the_order_of_the_variants() {
        let entry = "template: '${:colour} line.'\nexample: Red line.\n";
        for other in [
            entry,
            "# A comment.\nexample: \"Red line.\"\ntemplate: \"${:colour} line.\"\n",
            "{ example: Red line., template: '${:colour} line.' }\n",
        ] {
            assert!(alike(&value(entry), &value(other), None), "{other:?}");
        }
        for other in [
            "template: '${:colour} line.'\nexample: Blue line.\n",
            "template: '${:colour} line.'\nexample: Red line.\nkey: colour\n",
            "template: '${:colour} line.'\n",
        ] {
            assert!(!alike(&value(entry), &value(other), None), "{other:?}");
        }
        // A string and a number of the same text parse to different values.
        assert!(!alike(&value("['1']"), &value("[1]"), None));
        assert!(alike(&value("[.nan]"), &value("[.NaN]"), None));
        // The choices of a type keep their order.
        assert!(!alike(&value("[a, b]"), &value("[b, a]"), None));
        let variants = |a: &str, b: &str| {
            value(&format!(
                "variants:\n  {a}:\n    text: {a}\n    example: {a}\n  {b}:\n    example: {b}\n    text: {b}\n"
            ))
        };
        assert!(alike(
            &variants("one", "two"),
            &variants("one", "two"),
            None
        ));
        assert!(!alike(
            &variants("one", "two"),
            &variants("two", "one"),
            None
        ));
        // A patch keeps the place of each variant of the entry below.
        let patch = |a: &str, b: &str| {
            value(&format!(
                "patch: true\nvariants:\n  {a}:\n    example: {a}\n  {b}:\n    example: {b}\n"
            ))
        };
        let base = variants("one", "two");
        assert!(alike(
            &patch("one", "two"),
            &patch("two", "one"),
            Some(&base)
        ));
        assert!(!alike(&patch("one", "new"), &patch("new", "one"), None));
        // Without variants below, the patch brings its own, nulls and all.
        let null = |first: bool| {
            let (x, y) = ("  x: ~\n", "  y:\n    example: y\n");
            let (a, b) = if first { (x, y) } else { (y, x) };
            value(&format!("patch: true\nvariants:\n{a}{b}"))
        };
        let single = value("pattern: '.*'\nexample: x\n");
        assert!(!alike(&null(true), &null(false), Some(&single)));
        assert!(alike(&null(true), &null(false), Some(&base)));
        let other = variants("three", "four");
        assert!(!alike(
            &patch("one", "two"),
            &patch("two", "one"),
            Some(&other)
        ));
    }
}
