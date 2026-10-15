//! This module makes the config change that makes a new store a root in
//! effect (`sokf init`).

use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::{Error, RepoPath, Result, config::ConfigText};

/// `ConfigChange` is the only edit that `init` can make to the repository
/// outside `DIR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChange {
    /// `DIR` is a root in effect already.
    Unchanged,
    /// No config exists. `Create` holds the text of a new one.
    Create(String),
    /// A config exists: its text with `DIR` appended to `knowledge.roots`.
    Update(String),
}

/// `register` decides the change that makes `dir` a root in effect.
/// `existing` lists each root in effect whose directory exists. A new
/// config, or a `roots` key that `register` adds, lists them before `dir`.
// sokf-implements: CLI-2_AC-9
// sokf-implements: CLI-2_AC-25
// sokf-implements: CONF-2_AC-1
// sokf-implements: CONF-5_AC-1
pub fn register(
    config: &ConfigText,
    existing: &[RepoPath],
    dir: &RepoPath,
) -> Result<ConfigChange> {
    let Some(text) = &config.text else {
        if dir.as_str() == "/knowledge" {
            return Ok(ConfigChange::Unchanged);
        }
        let mut roots: Vec<&RepoPath> = existing.iter().filter(|r| *r != dir).collect();
        roots.push(dir);
        let listed: Vec<String> = roots.iter().map(|r| format!("\"{r}\"")).collect();
        return Ok(ConfigChange::Create(format!(
            "[knowledge]\nroots = [{}]\n",
            listed.join(", ")
        )));
    };
    if config.roots.contains(dir) {
        return Ok(ConfigChange::Unchanged);
    }
    let mut doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| {
        Error::Init(format!("{}: {}", crate::config::config_path(), e.message()))
    })?;
    let table = doc
        .entry("knowledge")
        .or_insert(Item::Table(Table::new()))
        .as_table_like_mut()
        .ok_or_else(|| Error::Init("/.sokf/config.toml: `knowledge` is not a table".into()))?;
    // If the file has no `roots` key, `register` adds one. As in a new file,
    // the key lists each existing root in effect, then `DIR`. If the key
    // listed `DIR` alone, the tool would stop checking the stores that exist.
    // sokf-implements: CLI-2_AC-21
    let in_effect: Array = existing
        .iter()
        .filter(|r| *r != dir)
        .map(|r| r.as_str())
        .collect();
    let roots = table
        .entry("roots")
        .or_insert(Item::Value(Value::Array(in_effect)))
        .as_array_mut()
        .ok_or_else(|| {
            Error::Init("/.sokf/config.toml: `knowledge.roots` is not an array".into())
        })?;
    roots.push(dir.as_str());
    Ok(ConfigChange::Update(doc.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn config(roots: &[&str], text: Option<&str>) -> ConfigText {
        ConfigText {
            roots: roots.iter().map(|r| p(r)).collect(),
            declined: Default::default(),
            hook_commands: Default::default(),
            text: text.map(str::to_string),
        }
    }

    // sokf-tests: CLI-2_AC-9
    // sokf-tests: CLI-2_AC-25
    // sokf-tests: CONF-5_AC-1
    #[test]
    fn each_case_of_the_tool_spec() {
        // With no config file and the default directory, `register` writes
        // nothing.
        assert_eq!(
            register(
                &config(&["/knowledge"], None),
                &[p("/knowledge")],
                &p("/knowledge")
            )
            .unwrap(),
            ConfigChange::Unchanged
        );
        // With no config file and another directory, `register` creates a file
        // that lists the existing roots and then `DIR`.
        assert_eq!(
            register(
                &config(&["/knowledge"], None),
                &[p("/knowledge")],
                &p("/docs/kb")
            )
            .unwrap(),
            ConfigChange::Create("[knowledge]\nroots = [\"/knowledge\", \"/docs/kb\"]\n".into())
        );
        assert_eq!(
            register(&config(&["/knowledge"], None), &[], &p("/docs/kb")).unwrap(),
            ConfigChange::Create("[knowledge]\nroots = [\"/docs/kb\"]\n".into())
        );
        // If the file lists `DIR` already, `register` writes nothing.
        let text = "[knowledge]\nroots = [\"/docs/kb\"]\n";
        assert_eq!(
            register(&config(&["/docs/kb"], Some(text)), &[], &p("/docs/kb")).unwrap(),
            ConfigChange::Unchanged
        );
        // If the file does not list `DIR`, `register` appends it in place and
        // keeps the comments and order.
        let text = "# The stores.\n[repository]\nexclude = [\"/vendor/**\"]  # pruned\n\n[knowledge]\nroots = [\n  \"/knowledge\", # the default\n]\n";
        let ConfigChange::Update(after) =
            register(&config(&["/knowledge"], Some(text)), &[], &p("/docs/kb")).unwrap()
        else {
            panic!("expected an update");
        };
        // The lines outside the array stay as they were. After `register`
        // appends the element, the array still holds its comment.
        assert!(
            after.starts_with(
                "# The stores.\n[repository]\nexclude = [\"/vendor/**\"]  # pruned\n\n[knowledge]\nroots = [\n"
            ),
            "{after}"
        );
        assert!(after.contains("# the default"), "{after}");
        assert!(after.contains("\"/docs/kb\""), "{after}");
        let parsed: toml::Table = after.parse().unwrap();
        assert_eq!(
            parsed["knowledge"]["roots"],
            toml::Value::Array(vec!["/knowledge".into(), "/docs/kb".into()])
        );
        // A `[knowledge]` table without `roots` gains the key, and a file
        // without the table gains both. The key lists the roots in effect
        // whose directory exists, then `DIR`. It leaves out a root in effect
        // whose directory does not exist.
        // sokf-tests: CLI-2_AC-21
        let ConfigChange::Update(after) = register(
            &config(&["/knowledge"], Some("[knowledge]\n")),
            &[p("/knowledge")],
            &p("/docs/kb"),
        )
        .unwrap() else {
            panic!("expected an update");
        };
        assert_eq!(
            after,
            "[knowledge]\nroots = [\"/knowledge\", \"/docs/kb\"]\n"
        );
        let ConfigChange::Update(after) = register(
            &config(&["/knowledge"], Some("[repository]\nexclude = []\n")),
            &[p("/knowledge")],
            &p("/docs/kb"),
        )
        .unwrap() else {
            panic!("expected an update");
        };
        let parsed: toml::Table = after.parse().unwrap();
        assert_eq!(
            parsed["knowledge"]["roots"],
            toml::Value::Array(vec!["/knowledge".into(), "/docs/kb".into()])
        );
        assert!(after.starts_with("[repository]\nexclude = []\n"), "{after}");
        let ConfigChange::Update(after) = register(
            &config(&["/knowledge"], Some("[knowledge]\n")),
            &[],
            &p("/docs/kb"),
        )
        .unwrap() else {
            panic!("expected an update");
        };
        assert_eq!(after, "[knowledge]\nroots = [\"/docs/kb\"]\n");
    }
}
