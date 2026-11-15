//! This module reads the files of `/.sokf/formats.d/`, the layer
//! `formats.d`, in which no file takes precedence over another (`TL_128`,
//! CONF-11): it drops the declarations that are alike, refuses those that
//! clash, gives the folder the types of `/.sokf/formats.yaml` that settle a
//! clash, parses each file, and refuses a format below the folder that a
//! type of the folder no longer compiles.

use std::collections::{BTreeMap, btree_map::Entry};

use super::formats::{formats_path, layer_of, line_message};
use crate::{
    Error, RepoPath, Result, Spanned, Yaml,
    grammar::{
        Format, FormatRegistry, FormatSource, Grammar, PackLayer, RegexCache, Segment, Shape,
    },
};

/// `settled_types` returns `below`, the layers under the folder, with the
/// declarations of `repo`, the path and the text of `/.sokf/formats.yaml`,
/// of each type that two of `docs`, the files of `/.sokf/formats.d/` as
/// `without_alike` leaves them, declare, or `None` where there is no such
/// type. It drops each such type from each of `docs`, so that every file of
/// the folder, and `with_packs` when it compiles a format of the folder
/// again, compiles against the declaration that the run takes, and
/// `with_packs` finds no clash in it.
// sokf-implements: CONF-11_AC-10
pub(super) fn settled_types(
    docs: &mut [(RepoPath, Spanned)],
    repo: Option<(&RepoPath, &Spanned)>,
    below: &FormatRegistry,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<Option<FormatRegistry>> {
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
    let mut base = below.clone();
    base.types = base.types.layered(&types);
    Ok(Some(base))
}

/// `check_below` refuses each format of `below`, the layers under
/// `/.sokf/formats.d/`, that a schema sees and that no longer compiles
/// against a type of the folder that a schema sees, because `layered` keeps
/// such a format as it was compiled against the type of its own layer. A
/// schema sees a format that neither a file of `files` nor `repo` declares,
/// since a patch of either compiles as the entry that it makes, and a type
/// of `files` that `below` declares otherwise and `repo` doesn't declare.
/// The format compiles against the types that a schema sees: those of
/// `folder`, the folder's layer on `below`, with those of `repo` on top.
/// The error names each such format, with its layer and each type of the
/// folder or of `repo` that it uses and that differs from the one below,
/// and lies on the file of the first type of the folder that it uses.
// sokf-implements: CONF-11_AC-5
// sokf-implements: CONF-11_AC-11
pub(super) fn check_below(
    below: &FormatRegistry,
    folder: &FormatRegistry,
    files: &[(RepoPath, FormatRegistry)],
    repo: &FormatRegistry,
    grammar: &Grammar,
    regexes: &RegexCache,
) -> Result<()> {
    let mut type_file: BTreeMap<&str, &RepoPath> = BTreeMap::new();
    for (path, file) in files {
        for (name, (def, _)) in &file.types.entries {
            // A type that a layer below declares alike changes nothing.
            if below.types.get(name).is_some_and(|d| d != def)
                && !repo.types.entries.contains_key(name)
            {
                type_file.entry(name.as_str()).or_insert(path);
            }
        }
    }
    if type_file.is_empty() {
        return Ok(());
    }
    // A merged entry holds no `patch` key, so only the types of the base
    // count.
    let seen = FormatRegistry {
        types: folder.types.layered(&repo.types),
        ..FormatRegistry::default()
    };
    let repo_path = formats_path();
    let mut errors: Vec<(&RepoPath, String)> = Vec::new();
    for (name, (format, source)) in &below.entries {
        if files.iter().any(|(_, f)| f.entries.contains_key(name))
            || repo.entries.contains_key(name)
        {
            continue;
        }
        // Each type of the folder that the format uses, and each type of
        // `/.sokf/formats.yaml` that differs from the one below, which can
        // be a cause too.
        let mut used: Vec<(&str, &RepoPath)> = Vec::new();
        for hole in holes(format) {
            let file = type_file.get(hole).copied().or_else(|| {
                repo.types
                    .get(hole)
                    .is_some_and(|d| below.types.get(hole) != Some(d))
                    .then_some(&repo_path)
            });
            if let Some(file) = file
                && !used.iter().any(|(u, _)| *u == hole)
            {
                used.push((hole, file));
            }
        }
        let Some(&(_, path)) = used.iter().find(|(_, p)| **p != repo_path) else {
            continue;
        };
        let raw = &format.raw;
        let entry = Spanned::new(
            Yaml::Map(vec![(Spanned::new(name.clone(), raw.line), raw.clone())]),
            raw.line,
        );
        let again = FormatRegistry::from_yaml_at(
            &entry,
            None,
            source.clone(),
            grammar,
            regexes,
            path,
            Some(&seen),
        );
        if let Err(findings) = again {
            // The lines of the findings are those of the format's own file,
            // and tramlines names the format before each.
            let prefix = format!("format `{name}`: ");
            let said: Vec<String> = findings
                .iter()
                .map(|f| {
                    let m = crate::report::message(f);
                    m.strip_prefix(&prefix).map_or(m.clone(), str::to_string)
                })
                .collect();
            let types: Vec<String> = used
                .iter()
                .map(|(t, p)| match p {
                    p if *p == path => format!("the type `{t}` of this file"),
                    p => format!("the type `{t}` of `{p}`"),
                })
                .collect();
            errors.push((
                path,
                format!(
                    "the format `{name}` of the layer `{}` no longer compiles against {}: {}",
                    source.label(),
                    types.join(" and "),
                    said.join("; ")
                ),
            ));
        }
    }
    errors.sort_by(|a, b| a.0.cmp(b.0));
    match errors.first() {
        None => Ok(()),
        Some((first, _)) => {
            let messages: Vec<String> = errors
                .iter()
                .map(|(p, m)| match p {
                    p if p == first => m.clone(),
                    p => format!("{p}: {m}"),
                })
                .collect();
            Err(Error::formats((*first).clone(), messages.join("; ")))
        }
    }
}

/// `holes` returns the type of each hole of each template of `format`.
fn holes(format: &Format) -> impl Iterator<Item = &str> {
    format
        .shape
        .iter()
        .chain(format.variants.iter().map(|(_, v)| &v.shape))
        .filter_map(|shape| match shape {
            Shape::Template(t) => Some(t.segments.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|segment| match segment {
            Segment::Hole { ty, .. } => Some(ty.as_str()),
            Segment::Literal(_) => None,
        })
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

/// `refuse_findings` refuses what tramlines' `with_host` finds in the layer
/// `folder` under `repo`: each format of the layer that no longer compiles,
/// and any clash that its rule finds and sokf's doesn't, with the findings
/// of every file.
pub(super) fn refuse_findings(folder: &PackLayer, repo: &FormatRegistry) -> Result<()> {
    let (_, findings) = folder.with_host(repo);
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
