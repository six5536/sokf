//! This module decides where `sokf new` puts a new document: it works out
//! the path of the document, applies the `--dir` and `--for` rules, and makes
//! the default slug from the filename of an asset.

use std::path::Path;

use crate::{
    Error, Model, PathArg, RepoPath, RepoRoot, Result,
    check::derived_dir,
    grammar::{SokfSchema, States},
};

/// `derived` returns the directory that the lifecycle states of a
/// document derive, for a schema whose dimensions place its documents.
fn derived(schema: &SokfSchema, states: &States) -> Result<RepoPath> {
    derived_dir(schema, states)
        .ok_or_else(|| Error::Internal("the schema defines no directory for these states".into()))
}

/// `schema_dirs` lists the `schemas/` directory of each root, for a message.
fn schema_dirs(roots: &[&RepoPath]) -> String {
    roots
        .iter()
        .filter_map(|r| r.join("schemas"))
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `Site` holds the inputs that `place` and `asset` read: the repository,
/// the model, the schema of the type, the requested states, and the
/// directory that relative paths resolve from.
#[derive(Debug, Clone, Copy)]
pub struct Site<'a> {
    /// The repository root.
    pub root: &'a RepoRoot,
    /// The model.
    pub model: &'a Model,
    /// The type's schema.
    pub schema: &'a SokfSchema,
    /// The requested states.
    pub states: &'a States,
    /// The current directory, which relative paths resolve from.
    pub cwd: &'a Path,
}

/// `place` returns the path of the new document, named `<id>.md`. When the
/// schema enforces a path, the directory comes from the requested states.
/// Otherwise the directory is the `--dir` directory, or, when `--dir` is
/// absent, the schema's `root` or else the knowledge root. A `Schema` goes in the `schemas/` directory of the root.
/// `place` refuses `--dir` when the states give the directory, when the
/// `--dir` directory lies outside the knowledge or the schema's `root`, and, for a `Schema`, when
/// the `--dir` directory is not the `schemas/` directory of a root.
// sokf-implements: CLI-6_AC-9
// sokf-implements: CLI-6_AC-29
// sokf-implements: CLI-6_AC-30
// sokf-implements: CLI-6_AC-39
// sokf-implements: CLI-6_AC-40
// sokf-implements: CLI-6_AC-41
// sokf-implements: CLI-6_AC-58
pub fn place(
    site: &Site<'_>,
    knowledge: Option<usize>,
    id: &str,
    dir: Option<&Path>,
) -> Result<RepoPath> {
    let Site {
        root,
        model,
        schema,
        states,
        cwd,
    } = *site;
    let file = format!("{id}.md");
    let refuse = |msg: String| Error::New(msg);
    if !schema.paths.is_empty() {
        let derived = derived(schema, states)?;
        if let Some(d) = dir {
            return Err(refuse(format!(
                "the requested states fix the directory of the document, {derived}, so `--dir {}` cannot change it",
                d.display()
            )));
        }
        return derived
            .join(&file)
            .ok_or_else(|| Error::Internal(format!("{derived}/{file} is not a path")));
    }
    let roots: Vec<&RepoPath> = model.knowledges.iter().map(|k| &k.root).collect();
    let type_root = schema.root.as_deref().and_then(RepoPath::parse);
    let directory = match (dir, knowledge) {
        (Some(d), _) => {
            let shown = d.display().to_string();
            let resolved = d
                .to_str()
                .and_then(|s| PathArg(s).resolve_new(root, cwd))
                .ok_or_else(|| refuse(format!("{shown} lies outside the repository")))?;
            match knowledge {
                Some(k) => {
                    let kroot = &model.knowledges[k].root;
                    if !resolved.starts_with(kroot) {
                        return Err(refuse(format!(
                            "{resolved} lies outside the knowledge {kroot}"
                        )));
                    }
                    // sokf-implements: CLI-6_AC-65
                    if let Some(type_root) = type_root
                        && !resolved.starts_with(&type_root)
                    {
                        return Err(refuse(format!(
                            "{resolved} lies outside {type_root}, the root of the type's schema"
                        )));
                    }
                }
                None => {
                    let is_schemas = roots
                        .iter()
                        .any(|r| r.join("schemas").as_ref() == Some(&resolved));
                    if !is_schemas {
                        return Err(refuse(format!(
                            "{resolved} isn't the `schemas/` directory of a root. The `schemas/` directories are: {}",
                            schema_dirs(&roots)
                        )));
                    }
                }
            }
            resolved
        }
        // sokf-implements: CLI-6_AC-64
        (None, Some(k)) => type_root.unwrap_or_else(|| model.knowledges[k].root.clone()),
        (None, None) => match roots.as_slice() {
            [one] => one
                .join("schemas")
                .ok_or_else(|| Error::Internal("the root has no `schemas/` path".into()))?,
            _ => {
                return Err(refuse(format!(
                    "there is more than one root, so `--dir` must give a `schemas/` directory. The `schemas/` directories are: {}",
                    schema_dirs(&roots)
                )));
            }
        },
    };
    directory
        .join(&file)
        .ok_or_else(|| Error::Internal(format!("{directory}/{file} is not a path")))
}

/// `asset` returns the path of the asset that a sidecar is created for,
/// and the path of that sidecar. It refuses a path that is not an asset of
/// the type, an asset that already has a sidecar, and, when the schema
/// enforces a path, an asset outside the directory that the requested states
/// give.
// sokf-implements: CLI-6_AC-14
// sokf-implements: CLI-6_AC-43
// sokf-implements: CLI-6_AC-44
// sokf-implements: CLI-6_AC-45
pub fn asset(
    site: &Site<'_>,
    knowledge: usize,
    type_name: &str,
    path: &Path,
) -> Result<(RepoPath, RepoPath)> {
    let Site {
        root,
        model,
        schema,
        states,
        cwd,
    } = *site;
    let refuse = |msg: String| Error::New(msg);
    let shown = path.display().to_string();
    let resolved = path
        .to_str()
        .and_then(|s| PathArg(s).resolve(root, cwd))
        .ok_or_else(|| refuse(format!("{shown} is not a path in the repository")))?;
    let Some((k, asset)) = model.asset_at(&resolved) else {
        return Err(refuse(format!(
            "{resolved} is not an asset: no `asset-files` glob of `{type_name}` matches it"
        )));
    };
    let owner = model.schema(asset.schema).type_const.clone();
    if k != knowledge || owner.as_deref() != Some(type_name) {
        return Err(refuse(format!(
            "{resolved} is an asset of `{}`, not of `{type_name}`",
            owner.unwrap_or_default()
        )));
    }
    if asset.has_sidecar {
        return Err(refuse(format!(
            "{resolved} already has a sidecar, {}",
            asset.sidecar_path()
        )));
    }
    if !schema.paths.is_empty() {
        let derived = derived(schema, states)?;
        let parent = resolved.parent().unwrap_or_else(RepoPath::root);
        if parent != derived {
            return Err(refuse(format!(
                "{resolved} is in {parent}, but the requested states put sidecars in {derived}. Move the asset to {derived}, or request the states that match {parent}"
            )));
        }
    }
    Ok((resolved.clone(), asset.sidecar_path()))
}

/// `slugify` turns a filename stem into a slug. It lower-cases the stem,
/// replaces each run of characters that are not ASCII letters or digits
/// with one hyphen, and drops leading and trailing hyphens. It returns
/// `None` when no letters or digits remain.
// sokf-implements: CLI-6_AC-31
pub fn slugify(stem: &str) -> Option<String> {
    let mut out = String::new();
    let mut pending = false;
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            if pending && !out.is_empty() {
                out.push('-');
            }
            pending = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending = true;
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // sokf-tests: CLI-6_AC-31
    #[test]
    fn slugify_lowers_and_hyphenates() {
        assert_eq!(slugify("flowers").as_deref(), Some("flowers"));
        assert_eq!(
            slugify("Wild Flowers 2024").as_deref(),
            Some("wild-flowers-2024")
        );
        assert_eq!(slugify("my.file.v2").as_deref(), Some("my-file-v2"));
        assert_eq!(slugify("--Shouty__NAME--").as_deref(), Some("shouty-name"));
        assert_eq!(slugify("äöü").as_deref(), None);
        assert_eq!(slugify("").as_deref(), None);
    }
}
