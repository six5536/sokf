//! This module holds sokf's store of the declined parts: the parts that `without` lists under
//! `[harness.NAME]`, which the adapter's `TomlDeclined` reads, and each part that is off by
//! default and that `with` doesn't list (CONF-4).

use std::path::PathBuf;

use agent_harness_adapter_core::{DeclinedStore, TomlDeclined};

/// `OFF_BY_DEFAULT` lists the parts that `install` skips unless `with` lists them: the hook,
/// which runs at every prompt, so that a repository chooses it.
pub const OFF_BY_DEFAULT: &[&str] = &["hooks"];

/// `bad_with` returns the message for the first part of `parts`, the `with` list of the harness
/// `harness`, that isn't off by default, because such an entry opts in to nothing.
// sokf-implements: CONF-4_AC-6
pub fn bad_with(harness: &str, parts: &[String]) -> Option<String> {
    parts
        .iter()
        .find(|p| !OFF_BY_DEFAULT.contains(&p.as_str()))
        .map(|p| {
            let allowed: Vec<String> = OFF_BY_DEFAULT.iter().map(|p| format!("`{p}`")).collect();
            format!(
                "harness.{harness}.with lists `{p}`, which isn't off by default; it can list only {}",
                allowed.join(", ")
            )
        })
}

/// `SokfDeclined` reads the declined parts of a harness from one configuration file: the
/// parts that `without` lists, and each part of `OFF_BY_DEFAULT` that `with` doesn't list.
pub struct SokfDeclined {
    inner: TomlDeclined,
    path: PathBuf,
    shown: String,
    held: Option<String>,
}

impl SokfDeclined {
    /// `new` returns the store of the file `path`, named `shown` in refusals.
    pub fn new(path: PathBuf, shown: String) -> Self {
        SokfDeclined {
            inner: TomlDeclined::new(path.clone(), shown.clone()),
            path,
            shown,
            held: None,
        }
    }

    /// `holding` declines the part `part` of every harness too, whatever the file says.
    #[must_use]
    pub fn holding(mut self, part: &str) -> Self {
        self.held = Some(part.to_string());
        self
    }

    /// `with` returns the parts that `with` lists for `harness`. A file that is absent, or
    /// that has no such list, lists none. It refuses a file that doesn't parse, a `with` that
    /// isn't a list of part names, and a part that isn't off by default, at every scope.
    pub fn with(&self, harness: &str) -> agent_harness_adapter_core::Result<Vec<String>> {
        let refuse = |m: String| agent_harness_adapter_core::Error::file(&self.shown, m);
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(refuse(e.to_string())),
        };
        let doc = text
            .parse::<toml::Table>()
            .map_err(|e| refuse(e.message().to_string()))?;
        let Some(list) = doc
            .get("harness")
            .and_then(|h| h.get(harness))
            .and_then(|t| t.get("with"))
        else {
            return Ok(Vec::new());
        };
        let not_list = || {
            refuse(format!(
                "`harness.{harness}.with` is not a list of part names"
            ))
        };
        let parts = list
            .as_array()
            .ok_or_else(not_list)?
            .iter()
            .map(|v| v.as_str().map(str::to_string).ok_or_else(not_list))
            .collect::<agent_harness_adapter_core::Result<Vec<_>>>()?;
        if let Some(m) = bad_with(harness, &parts) {
            return Err(refuse(m));
        }
        Ok(parts)
    }

    /// `off_by_default` returns the parts of `OFF_BY_DEFAULT` that `with` doesn't list for
    /// `harness`.
    pub fn off_by_default(&self, harness: &str) -> agent_harness_adapter_core::Result<Vec<String>> {
        let with = self.with(harness)?;
        Ok(OFF_BY_DEFAULT
            .iter()
            .filter(|p| !with.iter().any(|w| w == *p))
            .map(|p| (*p).to_string())
            .collect())
    }

    /// `without` returns the parts that `without` lists for `harness`.
    pub fn without(&self, harness: &str) -> agent_harness_adapter_core::Result<Vec<String>> {
        self.inner.declined(harness)
    }

    /// `shown` returns the name of the file in refusals.
    pub fn shown(&self) -> &str {
        &self.shown
    }
}

impl DeclinedStore for SokfDeclined {
    // sokf-implements: CONF-4_AC-4
    // sokf-implements: CONF-4_AC-5
    fn declined(&self, harness: &str) -> agent_harness_adapter_core::Result<Vec<String>> {
        let mut declined = self.without(harness)?;
        for part in self
            .off_by_default(harness)?
            .into_iter()
            .chain(self.held.clone())
        {
            if !declined.contains(&part) {
                declined.push(part);
            }
        }
        Ok(declined)
    }

    fn location(&self) -> String {
        self.inner.location()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: CONF-4_AC-4
    // sokf-tests: CONF-4_AC-5
    // sokf-tests: CONF-4_AC-6
    #[test]
    fn the_hooks_are_declined_unless_with_lists_them() {
        let dir = crate::test_support::fresh_dir("harness-declined");
        let path = dir.join("config.toml");
        let store = SokfDeclined::new(path.clone(), "/config.toml".into());
        // A missing file declines the parts that are off by default.
        assert_eq!(store.declined("claude").unwrap(), ["hooks"]);
        std::fs::write(&path, "[harness.claude]\nwithout = [\"skills\"]\n").unwrap();
        assert_eq!(store.declined("claude").unwrap(), ["skills", "hooks"]);
        std::fs::write(&path, "[harness.claude]\nwith = [\"hooks\"]\n").unwrap();
        assert_eq!(store.declined("claude").unwrap(), Vec::<String>::new());
        assert_eq!(store.declined("codex").unwrap(), ["hooks"]);
        // `without` wins over `with`.
        std::fs::write(
            &path,
            "[harness.claude]\nwith = [\"hooks\"]\nwithout = [\"hooks\"]\n",
        )
        .unwrap();
        assert_eq!(store.declined("claude").unwrap(), ["hooks"]);
        // A part that is on by default is refused, at every scope.
        std::fs::write(&path, "[harness.claude]\nwith = [\"skills\"]\n").unwrap();
        let e = store.declined("claude").unwrap_err().to_string();
        assert!(e.contains("harness.claude.with lists `skills`"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
