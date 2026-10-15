//! This module presents sokf to agent-harness-adapter as a `Tool`: its name, which sets the
//! region markers, the harnesses it supports, its integration, and where it keeps the record
//! of the installed parts (CLI-3_AC-52) and the declined parts (CLI-3_AC-8) at each scope.

use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use agent_harness_adapter_core::{
    DeclinedStore, Harness, Integration, Scope, TomlDeclined, Tool, harness,
};

use crate::RepoPath;

use super::integration;

/// `HEADER` is the first line of the record file.
// sokf-implements: CLI-3_AC-58
pub const HEADER: &str = "# Written by sokf harness install. Do not edit it: sokf compares the parts with these hashes to tell a part you edited from a stale one.";

/// `record_path` returns the path of the record file of the project scope.
pub fn record_path() -> RepoPath {
    RepoPath::parse("/.sokf/harness.toml").expect("literal")
}

/// `LOCAL_RECORD` and `LOCAL_CONFIG` are the record and the declined parts of the local
/// scope, relative to the repository root. Like the files of the local scope, they belong
/// out of version control.
const LOCAL_RECORD: &str = ".sokf/harness.local.toml";
const LOCAL_CONFIG: &str = ".sokf/config.local.toml";

/// `SokfTool` is sokf in a repository, in the home directory, or both, with the program
/// that the hook line of each harness runs.
pub struct SokfTool {
    repo: Option<PathBuf>,
    home: Option<PathBuf>,
    programs: BTreeMap<String, String>,
}

impl SokfTool {
    /// `new` returns sokf with the repository `repo` and the home directory `home`, whose
    /// hook line for the harness `id` runs `programs[id]`, or `sokf` when `programs` has no
    /// entry for it.
    pub fn new(
        repo: Option<PathBuf>,
        home: Option<PathBuf>,
        programs: BTreeMap<String, String>,
    ) -> Self {
        SokfTool {
            repo,
            home,
            programs,
        }
    }

    fn repo(&self) -> agent_harness_adapter_core::Result<&PathBuf> {
        self.repo
            .as_ref()
            .ok_or_else(|| agent_harness_adapter_core::Error::Refused("no repository".into()))
    }

    fn home(&self) -> agent_harness_adapter_core::Result<&PathBuf> {
        self.home
            .as_ref()
            .ok_or_else(|| agent_harness_adapter_core::Error::Refused("no home directory".into()))
    }
}

impl Tool for SokfTool {
    fn name(&self) -> &str {
        "sokf"
    }

    /// Every harness of the adapter.
    fn harnesses(&self) -> Vec<Arc<dyn Harness>> {
        harness::builtin()
    }

    /// The integration, whose hook runs the program that the configuration sets for each
    /// harness.
    fn integration(&self, _scope: Scope) -> Integration {
        integration(&self.programs)
    }

    fn root(&self, scope: Scope) -> agent_harness_adapter_core::Result<PathBuf> {
        match scope {
            Scope::User => self.home().cloned(),
            _ => self.repo().cloned(),
        }
    }

    // sokf-implements: CLI-3_AC-52
    fn record_path(&self, scope: Scope) -> agent_harness_adapter_core::Result<PathBuf> {
        Ok(match scope {
            Scope::User => self.home()?.join(".sokf/harness.toml"),
            Scope::Local => self.repo()?.join(LOCAL_RECORD),
            _ => record_path().to_fs(&crate::RepoRoot::new(self.repo()?.clone())),
        })
    }

    // sokf-implements: CLI-3_AC-8
    // sokf-implements: CONF-4_AC-1
    // sokf-implements: CONF-5_AC-1
    fn declined_store(
        &self,
        scope: Scope,
    ) -> agent_harness_adapter_core::Result<Box<dyn DeclinedStore + '_>> {
        let (path, shown) = match scope {
            Scope::User => (
                self.home()?.join(".sokf/config.toml"),
                "~/.sokf/config.toml".to_string(),
            ),
            Scope::Local => (self.repo()?.join(LOCAL_CONFIG), format!("/{LOCAL_CONFIG}")),
            _ => {
                let path = crate::config::config_path();
                (
                    path.to_fs(&crate::RepoRoot::new(self.repo()?.clone())),
                    path.to_string(),
                )
            }
        };
        Ok(Box::new(TomlDeclined::new(path, shown)))
    }

    fn record_header(&self) -> String {
        HEADER.to_string()
    }
}

#[cfg(test)]
mod tests {
    use agent_harness_adapter_core::harness::Context;

    use super::*;

    // sokf-tests: CONF-4_AC-2
    // sokf-tests: CONF-4_AC-3
    #[test]
    fn each_harness_renders_its_own_hook_program() {
        let dir = crate::test_support::fresh_dir("harness-tool");
        let tool = SokfTool::new(
            Some(dir.clone()),
            None,
            [("codex".to_string(), "./bin/sokf".to_string())].into(),
        );
        let ids: Vec<String> = tool.harnesses().iter().map(|h| h.id().into()).collect();
        assert_eq!(
            ids,
            [
                "claude", "codex", "factory", "gemini", "copilot", "cursor", "pi", "opencode",
                "kilo", "qwen", "devin", "agents"
            ]
        );
        let cx = Context::new("sokf", Scope::Project, &dir, None);
        let hooks = |id: &str| {
            let h = tool.harnesses().into_iter().find(|h| h.id() == id).unwrap();
            let parts = h.render(&tool.integration(Scope::Project), &cx).unwrap();
            format!("{:?}", parts.iter().find(|p| p.name() == "hooks").unwrap())
        };
        assert!(hooks("codex").contains("./bin/sokf harness hook codex stop"));
        assert!(hooks("claude").contains("\"sokf harness hook claude stop\""));
        assert!(tool.root(Scope::User).is_err());
        assert_eq!(tool.root(Scope::Local).unwrap(), dir);
        assert_eq!(
            tool.record_path(Scope::Local).unwrap(),
            dir.join(".sokf/harness.local.toml")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
