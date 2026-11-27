//! A tool of agent-harness-adapter with the markers and the files of sokf but with
//! the parts that a test chooses, as raw parts of Claude Code. A test uses it
//! to write a content that sokf no longer embeds, as an older sokf would have
//! written it, and to learn the value that the adapter records for a content. A raw
//! part's name must not be the name of an item, such as `skills`.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use agent_harness_adapter_core::{
    DeclinedStore, Harness, InstallOptions, InstallResult, Integration, Part, Scope, TomlDeclined,
    Tool, claude::Claude,
};

use super::TempTree;

/// `TestTool` installs `parts` for Claude Code as sokf in `root`.
pub struct TestTool {
    root: PathBuf,
    parts: Vec<Part>,
}

impl Tool for TestTool {
    fn name(&self) -> &'static str {
        "sokf"
    }

    fn harnesses(&self) -> Vec<Arc<dyn Harness>> {
        vec![Arc::new(Claude)]
    }

    fn integration(&self, _scope: Scope) -> Integration {
        self.parts
            .iter()
            .fold(Integration::new(), |i, p| i.part("claude", p.clone()))
    }

    fn root(&self, _scope: Scope) -> agent_harness_adapter_core::Result<PathBuf> {
        Ok(self.root.clone())
    }

    fn record_path(&self, _scope: Scope) -> agent_harness_adapter_core::Result<PathBuf> {
        Ok(self.root.join(".sokf/harness.toml"))
    }

    fn declined_store(
        &self,
        _scope: Scope,
    ) -> agent_harness_adapter_core::Result<Box<dyn DeclinedStore + '_>> {
        Ok(Box::new(TomlDeclined::new(
            self.root.join(".sokf/config.toml"),
            "/.sokf/config.toml",
        )))
    }
}

/// `install_parts` installs `parts` in the tree at `root` with `--force`, and
/// returns the adapter's result.
pub fn install_parts(root: &Path, parts: Vec<Part>) -> InstallResult {
    let tool = TestTool {
        root: root.to_path_buf(),
        parts,
    };
    agent_harness_adapter_core::install(
        &tool,
        &InstallOptions::new(["claude"], Scope::Project).force(true),
    )
    .unwrap()
}

/// `status_parts` returns the adapter's status of `parts` in the tree at `root`.
pub fn status_parts(root: &Path, parts: Vec<Part>) -> InstallResult {
    let tool = TestTool {
        root: root.to_path_buf(),
        parts,
    };
    agent_harness_adapter_core::status(&tool, ["claude"], Scope::Project).unwrap()
}

/// `recorded_value` returns the value that the adapter records for `part` once it
/// has written it: a fingerprint, or for a part of files a table of each file's
/// fingerprint by its path in the part.
pub fn recorded_value(part: Part) -> toml::Value {
    let tree = TempTree::empty("adapter-hash");
    let name = part.name().to_string();
    install_parts(tree.dir(), vec![part]);
    let record = tree.read(".sokf/harness.toml");
    let table: toml::Table = record.parse().unwrap();
    table["claude"]
        .get(&name)
        .unwrap_or_else(|| panic!("no value of `{name}` in {record}"))
        .clone()
}
