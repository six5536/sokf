//! This module reads the `[[pack]]` entries of `/.sokf/config.toml`, which
//! declare the packs of the repository in order (CONF-9), and edits them
//! for `sokf pack add` and `sokf pack remove`, keeping every other line of
//! the file.

use std::collections::BTreeSet;

use serde::Deserialize;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use super::file::config_path;
use crate::{Error, RepoPath, RepoRoot, Result, source::read_optional};

// sokf:begin config-pack
/// `PackEntry` holds one `[[pack]]` entry (CONF-9): a pack of the
/// repository, by its name, with the source, the ref, and the directory
/// that `sokf pack sync` fetches it from, and the store that receives its
/// schemas and documents.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackEntry {
    /// The pack's name, which its manifest gives.
    pub name: String,
    /// A git URL or path, `builtin:<NAME>`, or the `NAME` of an embedded
    /// pack alone.
    pub source: String,
    /// The branch, tag, or commit of a git source. Default: the source's
    /// default branch.
    #[serde(rename = "ref", default)]
    pub git_ref: Option<String>,
    /// The pack's directory in a git source. Default: the source's root.
    #[serde(default)]
    pub path: Option<String>,
    /// The knowledge root that receives the pack's schemas and documents.
    /// Default: `/knowledge`.
    #[serde(default)]
    pub store: Option<String>,
}
// sokf:end config-pack

/// `PacksFile` is the part of the configuration that this module reads.
#[derive(Debug, Default, Deserialize)]
struct PacksFile {
    #[serde(default)]
    pack: Vec<PackEntry>,
}

/// `read_packs` returns the `[[pack]]` entries of `/.sokf/config.toml`, in
/// their order, and none when the file is absent. It refuses an entry that
/// lacks `name` or `source` or has another key, a name that isn't a slug, a
/// store that isn't `/`-rooted, and a second entry of one name.
// sokf-implements: CONF-9_AC-1
// sokf-implements: CONF-9_AC-2
pub fn read_packs(root: &RepoRoot) -> Result<Vec<PackEntry>> {
    let path = config_path();
    let Some(text) = read_optional(&path.to_fs(root))? else {
        return Ok(Vec::new());
    };
    let file: PacksFile =
        toml::from_str(&text).map_err(|e| Error::config(path.clone(), e.message()))?;
    let mut names = BTreeSet::new();
    for p in &file.pack {
        if !crate::pack::is_slug(&p.name) {
            return Err(Error::config(
                path,
                format!("the pack name `{}` is not a slug", p.name),
            ));
        }
        if !names.insert(p.name.clone()) {
            return Err(Error::config(
                path,
                format!("two [[pack]] entries declare the pack {}", p.name),
            ));
        }
        if let Some(s) = &p.store
            && RepoPath::parse(s).is_none()
        {
            return Err(Error::config(
                path,
                format!(
                    "the store `{s}` of the pack {} is not a /-rooted path",
                    p.name
                ),
            ));
        }
    }
    Ok(file.pack)
}

/// `add_entry` returns the text of the configuration `text`, or of an
/// absent file when it is `None`, with `entry` appended as the last
/// `[[pack]]` entry. Every other line, comment, and blank line stays.
// sokf-implements: CLI-13_AC-65
pub fn add_entry(text: Option<&str>, entry: &PackEntry) -> Result<String> {
    let mut doc = parse(text)?;
    let mut table = Table::new();
    table.insert("name", value(&entry.name));
    table.insert("source", value(&entry.source));
    for (key, v) in [
        ("ref", &entry.git_ref),
        ("path", &entry.path),
        ("store", &entry.store),
    ] {
        if let Some(v) = v {
            table.insert(key, value(v));
        }
    }
    match doc.get_mut("pack") {
        None => {
            let mut packs = ArrayOfTables::new();
            packs.push(table);
            doc.insert("pack", Item::ArrayOfTables(packs));
        }
        Some(Item::ArrayOfTables(packs)) => packs.push(table),
        Some(_) => return Err(not_a_list()),
    }
    Ok(doc.to_string())
}

/// `remove_entry` returns the text of the configuration `text` without the
/// `[[pack]]` entry of the pack `name`, or `None` when no entry declares
/// it. Every other line, comment, and blank line stays.
// sokf-implements: CLI-13_AC-65
pub fn remove_entry(text: &str, name: &str) -> Result<Option<String>> {
    let mut doc = parse(Some(text))?;
    let Some(item) = doc.get_mut("pack") else {
        return Ok(None);
    };
    let Item::ArrayOfTables(packs) = item else {
        return Err(not_a_list());
    };
    let Some(i) = packs
        .iter()
        .position(|t| t.get("name").and_then(Item::as_str) == Some(name))
    else {
        return Ok(None);
    };
    packs.remove(i);
    if packs.is_empty() {
        doc.remove("pack");
    }
    Ok(Some(doc.to_string()))
}

fn parse(text: Option<&str>) -> Result<DocumentMut> {
    text.unwrap_or_default()
        .parse::<DocumentMut>()
        .map_err(|e| Error::config(config_path(), e.message()))
}

fn not_a_list() -> Error {
    Error::config(config_path(), "`pack` is not a list of [[pack]] tables")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> PackEntry {
        PackEntry {
            name: name.into(),
            source: format!("git@example.com:acme/{name}.git"),
            git_ref: None,
            path: Some("packs/base".into()),
            store: None,
        }
    }

    // sokf-tests: CLI-13_AC-65
    #[test]
    fn an_edit_keeps_every_other_line_of_the_file() {
        let text = "# The roots.\n[knowledge]\nroots = [\"/knowledge\"]\n\n# The house's packs.\n[[pack]]\nname = \"base\" # the base\nsource = \"x\"\n";
        let added = add_entry(Some(text), &entry("rust")).unwrap();
        assert!(added.starts_with(text), "{added}");
        assert!(
            added.ends_with(
                "\n[[pack]]\nname = \"rust\"\nsource = \"git@example.com:acme/rust.git\"\npath = \"packs/base\"\n"
            ),
            "{added}"
        );
        let removed = remove_entry(&added, "base").unwrap().unwrap();
        assert!(removed.contains("# The roots.\n[knowledge]"), "{removed}");
        assert!(!removed.contains("name = \"base\""), "{removed}");
        assert!(removed.contains("name = \"rust\""), "{removed}");
        assert_eq!(remove_entry(&removed, "base").unwrap(), None);
        let none = remove_entry(&removed, "rust").unwrap().unwrap();
        assert!(!none.contains("[[pack]]"), "{none}");
        let fresh = add_entry(None, &entry("base")).unwrap();
        assert!(fresh.starts_with("[[pack]]\nname = \"base\"\n"), "{fresh}");
    }
}
