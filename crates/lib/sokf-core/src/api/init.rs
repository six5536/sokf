//! This module runs `sokf init`. It resolves `DIR`, reads the config
//! without requiring its roots to exist, and refuses bad input in the order
//! that CLI-2 sets out. After the checks pass, it builds the files of the
//! store, works out the change to the config, and writes both.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    Error, PathArg, RepoPath, RepoRoot, Result,
    config::{self, ConfigText},
    markdown::is_slug,
    scaffold,
};

/// `InitOptions` holds the options of `sokf init`.
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// `dir` is `DIR`, a path in the repository or a path relative to `cwd`.
    /// The default is `/knowledge`.
    pub dir: Option<PathBuf>,
    /// `cwd` is the current directory, from which a relative `DIR` resolves.
    pub cwd: PathBuf,
    /// `name` holds the value of `--name`. The default is the last segment of
    /// `DIR`.
    pub name: Option<String>,
    /// `scope` holds the value of `--scope`. The default is the owner in the
    /// URL of the git remote `origin`.
    pub scope: Option<String>,
    /// `description` holds the value of `--description`.
    pub description: Option<String>,
}

/// `InitResult` is the report of `sokf init --json`: it holds the directory
/// of the store that the command created. It also holds the files that the
/// command wrote (CLI-2_AC-12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(title = "sokf init report")]
pub struct InitResult {
    /// `root` is the directory of the store that the command created.
    pub root: RepoPath,
    /// `created` holds the files that the command created, ordered by path.
    pub created: Vec<RepoPath>,
    /// `updated` holds the files that the command changed, ordered by path.
    /// The list holds `/.sokf/config.toml` when the command added the store
    /// to `knowledge.roots`, and is empty otherwise.
    pub updated: Vec<RepoPath>,
}

/// `init` creates a knowledge store and registers it as a root in
/// `knowledge.roots`. It checks for every reason to refuse before it writes
/// anything.
// sokf-implements: CLI-9_AC-1
// sokf-implements: CONF-1_AC-3
// sokf-implements: CONF-2_AC-8
// sokf-implements: CLI-2_AC-11
pub fn init(root: &RepoRoot, opts: &InitOptions) -> Result<InitResult> {
    // sokf-implements: CLI-2_AC-8
    // sokf-implements: CLI-2_AC-13
    let dir = match &opts.dir {
        None => RepoPath::parse("/knowledge").expect("literal"),
        Some(given) => {
            let shown = given.display().to_string();
            given
                .to_str()
                .and_then(|s| PathArg(s).resolve_new(root, &opts.cwd))
                .ok_or_else(|| Error::Init(format!("{shown} lies outside the repository")))?
        }
    };
    let fs_dir = dir.to_fs(root);
    // sokf-implements: CLI-2_AC-14
    if fs_dir.is_file() {
        return Err(Error::Init(format!("{dir} is a file, not a directory")));
    }
    if fs_dir.is_dir() {
        let mut entries = std::fs::read_dir(&fs_dir).map_err(|e| Error::io(&fs_dir, e))?;
        if entries.next().is_some() {
            return Err(Error::Init(format!("{dir} is not empty")));
        }
    }
    // sokf-implements: CLI-2_AC-3
    let name = opts
        .name
        .clone()
        .unwrap_or_else(|| dir.file_name().to_string());
    // sokf-implements: CLI-2_AC-15
    if name.is_empty() {
        return Err(Error::Init("--name is empty".into()));
    }
    // sokf-implements: CLI-2_AC-18
    if !is_slug(&name) {
        let from = match opts.name {
            Some(_) => "--name".to_string(),
            None => format!("the name `{name}`, the last segment of {dir},"),
        };
        return Err(Error::Init(format!(
            "{from} is not a slug. A slug is lower-case letters and digits, joined by single hyphens. Pass --name with a slug."
        )));
    }
    // sokf-implements: CLI-2_AC-16
    // sokf-implements: CLI-2_AC-17
    // sokf-implements: CLI-2_AC-18
    // sokf-implements: CLI-2_AC-19
    let scope = match &opts.scope {
        Some(scope) if is_slug(scope) => scope.clone(),
        Some(scope) => {
            return Err(Error::Init(format!(
                "--scope `{scope}` is not a slug. A slug is lower-case letters and digits, joined by single hyphens."
            )));
        }
        None => remote_owner(root).ok_or_else(|| {
            Error::Init(
                "you passed no --scope, and the URL of the git remote `origin` has no owner to read. Pass --scope with the slug of the store's owner.".into(),
            )
        })?,
    };
    let config: ConfigText = config::read_lenient(root)?;
    let existing: Vec<RepoPath> = config
        .roots
        .iter()
        .filter(|r| r.to_fs(root).is_dir())
        .cloned()
        .collect();
    // The identity of a store is unique among the roots (SOKF_FORMAT_12). The
    // default name, the last segment of the directory, is the name most likely
    // to clash with another store.
    // sokf-implements: CLI-2_AC-10
    // sokf-implements: CLI-2_AC-20
    for other in &existing {
        let Some(path) = other.join("manifest.sokf.yaml") else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(path.to_fs(root)) else {
            continue;
        };
        let manifest = crate::model::manifest(crate::yaml::parse(&text, 1), other);
        let identity = format!("{scope}/{name}");
        if manifest.scope.as_deref() == Some(scope.as_str())
            && manifest.name.as_deref() == Some(name.as_str())
        {
            return Err(Error::Init(format!(
                "the store at {other} already has the identity `{identity}`. Pass --name with another slug."
            )));
        }
        // A reference to a former identity still resolves to its store
        // (SOKF_FORMAT_12), so a new store can't take that identity.
        if manifest
            .renamed_from
            .iter()
            .any(|(old, _)| *old == identity)
        {
            return Err(Error::Init(format!(
                "the store at {other} was once `{identity}`, and references to that identity still resolve to that store. Pass --name with another slug."
            )));
        }
    }
    let files = scaffold::store(&dir, &name, &scope, opts.description.as_deref());
    let change = scaffold::register(&config, &existing, &dir)?;
    let (created, updated) = scaffold::write(root, &files, &change)?;
    Ok(InitResult {
        root: dir,
        created,
        updated,
    })
}

/// `remote_owner` returns the owner in the URL of the git remote `origin`,
/// in lower case, when the owner is a slug. For example,
/// `https://github.com/ncoderz/sokf`, `git@github.com:ncoderz/sokf.git`, and
/// `ssh://git@host/ncoderz/sokf` all return `ncoderz`.
fn remote_owner(root: &RepoRoot) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root.as_path())
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    owner_of(String::from_utf8(out.stdout).ok()?.trim())
}

/// `owner_of` returns the owner in a remote URL, which is the first segment
/// of its path, in lower case, when the owner is a slug.
// sokf-implements: CLI-2_AC-28
fn owner_of(url: &str) -> Option<String> {
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?.1,
        None => url.split_once(':')?.1,
    };
    let owner = path
        .trim_start_matches('/')
        .split('/')
        .next()?
        .to_lowercase();
    is_slug(&owner).then_some(owner)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        crate::test_support::fresh_dir(&format!("init-{name}"))
    }

    fn opts(dir: &std::path::Path, given: Option<&str>, name: Option<&str>) -> InitOptions {
        InitOptions {
            dir: given.map(PathBuf::from),
            cwd: dir.to_path_buf(),
            name: name.map(str::to_string),
            scope: Some("test".into()),
            description: None,
        }
    }

    fn message(r: Result<InitResult>) -> String {
        match r {
            Err(Error::Init(m)) => m,
            other => panic!("expected an init refusal, got {other:?}"),
        }
    }

    // sokf-tests: CLI-2_AC-13
    // sokf-tests: CLI-2_AC-14
    // sokf-tests: CLI-2_AC-15
    // sokf-tests: CLI-2_AC-18
    // sokf-tests: CONF-1_AC-3
    // sokf-tests: CLI-9_AC-1
    #[test]
    fn refuses_in_order_and_writes_nothing() {
        let dir = temp("refuse");
        let root = RepoRoot::new(dir.clone());
        fs::write(dir.join("file.md"), "").unwrap();
        fs::create_dir_all(dir.join("full")).unwrap();
        fs::write(dir.join("full/manifest.sokf.yaml"), "").unwrap();
        fs::create_dir_all(dir.join("empty")).unwrap();
        fs::create_dir_all(dir.join(".sokf")).unwrap();
        assert_eq!(
            message(init(&root, &opts(&dir, Some("../outside"), None))),
            "../outside lies outside the repository"
        );
        assert_eq!(
            message(init(&root, &opts(&dir, Some("file.md"), None))),
            "/file.md is a file, not a directory"
        );
        assert_eq!(
            message(init(&root, &opts(&dir, Some("full"), None))),
            "/full is not empty"
        );
        assert_eq!(
            message(init(&root, &opts(&dir, Some("empty"), Some("")))),
            "--name is empty"
        );
        assert_eq!(
            message(init(&root, &opts(&dir, Some("empty"), Some("Project KB")))),
            "--name is not a slug. A slug is lower-case letters and digits, joined by single hyphens. Pass --name with a slug."
        );
        let mut unscoped = opts(&dir, Some("empty"), None);
        unscoped.scope = Some("Acme Inc".into());
        assert!(message(init(&root, &unscoped)).starts_with("--scope `Acme Inc` is not a slug"));
        fs::write(dir.join(".sokf/config.toml"), "not toml [").unwrap();
        let m = message(init(&root, &opts(&dir, Some("empty"), None)));
        assert!(m.starts_with("/.sokf/config.toml: "), "{m}");
        assert!(!dir.join("empty/manifest.sokf.yaml").exists());
        assert!(!dir.join("knowledge").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-2_AC-3
    // sokf-tests: CLI-2_AC-8
    #[test]
    fn the_name_defaults_to_the_last_segment() {
        let dir = temp("name");
        let root = RepoRoot::new(dir.clone());
        let out = init(&root, &opts(&dir, Some("docs/kb/"), None)).unwrap();
        assert_eq!(out.root, RepoPath::parse("/docs/kb").unwrap());
        assert_eq!(
            fs::read_to_string(dir.join("docs/kb/manifest.sokf.yaml")).unwrap(),
            "sokf: \"0.6\"\nname: kb\nscope: test\n"
        );
        let out = init(&root, &opts(&dir, None, Some("project-kb"))).unwrap();
        assert_eq!(out.root, RepoPath::parse("/knowledge").unwrap());
        assert!(
            fs::read_to_string(dir.join("knowledge/index.md"))
                .unwrap()
                .starts_with("# project-kb\n")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: CLI-2_AC-17
    #[test]
    fn the_owner_is_read_from_a_remote_url() {
        for url in [
            "https://github.com/ncoderz/sokf",
            "https://github.com/NCoderz/sokf.git",
            "git@github.com:ncoderz/sokf.git",
            "ssh://git@github.com/ncoderz/sokf",
        ] {
            assert_eq!(owner_of(url).as_deref(), Some("ncoderz"), "{url}");
        }
        assert_eq!(owner_of(""), None);
        assert_eq!(owner_of("https://host/under_score/x"), None);
    }

    // sokf-tests: CLI-2_AC-20
    #[test]
    fn a_name_its_scope_already_holds_is_refused() {
        let dir = temp("clash");
        let root = RepoRoot::new(dir.clone());
        init(&root, &opts(&dir, Some("a/knowledge"), None)).unwrap();
        assert_eq!(
            message(init(&root, &opts(&dir, Some("b/knowledge"), None))),
            "the store at /a/knowledge already has the identity `test/knowledge`. Pass --name with another slug."
        );
        init(&root, &opts(&dir, Some("b/knowledge"), Some("b-knowledge"))).unwrap();
        fs::write(
            dir.join("a/knowledge/manifest.sokf.yaml"),
            "sokf: \"0.6\"\nname: knowledge\nscope: test\nrenamed-from: [test/old]\n",
        )
        .unwrap();
        assert_eq!(
            message(init(&root, &opts(&dir, Some("c/kb"), Some("old")))),
            "the store at /a/knowledge was once `test/old`, and references to that identity still resolve to that store. Pass --name with another slug."
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
