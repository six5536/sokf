//! This module implements `sokf new` (CLI-6).

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    Cache, CacheMode, Error, Model, PathArg, RepoPath, RepoRoot, Result, config,
    grammar::{self, SokfSchema},
    markdown::{is_slug, split_store},
    scaffold,
};

/// `NewOptions` holds the options of `sokf new`.
#[derive(Debug, Clone)]
pub struct NewOptions {
    /// `type_name` is the value of `TYPE`.
    pub type_name: String,
    /// `slug` is the value of `--slug`.
    pub slug: Option<String>,
    /// `id` is the value of `--id`.
    pub id: Option<String>,
    /// `title` is the value of `--title`.
    pub title: Option<String>,
    /// `states` holds the values of `--state`, each a key and a state, in the
    /// order of the arguments.
    pub states: Vec<(String, String)>,
    /// `dir` is the value of `--dir`, as a path in the repository or a path
    /// relative to `cwd`.
    pub dir: Option<PathBuf>,
    /// `for_asset` is the value of `--for`, as a path in the repository or a
    /// path relative to `cwd`.
    pub for_asset: Option<PathBuf>,
    /// `cwd` is the current directory, from which a relative `--dir` or
    /// `--for` resolves.
    pub cwd: PathBuf,
    /// `write` is true when the command has `--write`.
    pub write: bool,
    /// `id_only` is true when the command has `--id-only`.
    pub id_only: bool,
    /// `cache` is the cache mode.
    pub cache: CacheMode,
}

/// `NewResult` is the report of `sokf new --json`: it holds the document that
/// the command builds. The command prints the document but does not write it
/// (CLI-6_AC-12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(title = "sokf new report")]
pub struct NewResult {
    /// `id` is the `id` of the document.
    pub id: String,
    /// `path` is the path at which `--write` writes the document. An asset is
    /// a file of the knowledge that is not Markdown, such as an image, and a
    /// sidecar is the document that describes an asset. `--write` writes a
    /// sidecar beside its asset. It writes any other document as `<id>.md`,
    /// either in the directory that the schema gives the document's lifecycle
    /// states or in the directory set by `--dir`.
    pub path: RepoPath,
    /// `document` is the whole text of the document.
    pub document: String,
    /// `created` is true when `--write` wrote the file. The JSON report leaves this field
    /// out.
    #[serde(skip)]
    pub created: bool,
}

/// `new` builds a document of a type from the scaffold of the type's schema,
/// and writes it when `write` is set. When it refuses, it has written
/// nothing.
// sokf-implements: CLI-6_AC-1
// sokf-implements: CLI-6_AC-24
// sokf-implements: CLI-6_AC-32
// sokf-implements: CLI-6_AC-46
// sokf-implements: CLI-9_AC-1
// sokf-implements: CLI-6_AC-26
// sokf-implements: CLI-6_AC-13
// sokf-implements: CACHE-1_AC-21
// sokf-implements: CACHE-1_AC-27
// sokf-implements: CLI-6_AC-22
// sokf-implements: CLI-6_AC-25
pub fn new(root: &RepoRoot, opts: &NewOptions) -> Result<NewResult> {
    let (store, type_name) = split_store(opts.type_name.as_str());
    let config = config::load_config(root)?;
    let (grammar, repo) = super::load::load_formats(root)?;
    let cache = Cache::open(root, opts.cache);
    let mut model = super::load_for_lookup(root, &config, &cache)?;
    super::admit_schemas_of_type(&mut model, root, &cache, type_name)?;
    let compiled = grammar::compile_all(&model, grammar, repo);
    let chosen = chosen_store(root, &model, opts, store, type_name)?;
    let (knowledge, schema) = scaffold::schema_for(&model, &compiled, type_name, chosen)?;
    // A qualified `Schema` goes in the `schemas/` directory of its store.
    let schemas_dir = match (type_name, chosen, &opts.dir) {
        ("Schema", Some(k), None) => model.knowledges[k]
            .root
            .join("schemas")
            .map(|p| PathBuf::from(p.as_str())),
        _ => None,
    };
    let dir = opts.dir.as_deref().or(schemas_dir.as_deref());
    let states = scaffold::states(schema, &opts.states)?;
    let site = scaffold::Site {
        root,
        model: &model,
        schema,
        states: &states,
        cwd: &opts.cwd,
    };

    // sokf-implements: CLI-6_AC-51
    if opts.dir.is_some() && opts.for_asset.is_some() {
        return Err(Error::New(
            "`--dir` can't be given with `--for`, because the asset's location determines the document's path".to_string(),
        ));
    }
    // The location of a sidecar's asset determines the document's path, and
    // the asset's filename supplies the default slug.
    let asset = match (&opts.for_asset, schema.asset_files.is_empty(), knowledge) {
        (Some(_), true, _) | (Some(_), _, None) => {
            return Err(Error::New(format!(
                "`--for` is given, but `{type_name}` declares no `asset-files`"
            )));
        }
        (None, false, _) => {
            return Err(Error::New(format!(
                "`{type_name}` declares `asset-files`, so give `--for <PATH>` with the asset that the document describes"
            )));
        }
        (Some(path), false, Some(k)) => Some(scaffold::asset(&site, k, type_name, path)?),
        (None, true, _) => None,
    };
    // sokf-implements: CLI-6_AC-15
    let stem_slug = asset.as_ref().and_then(|(a, _)| {
        let name = a.file_name();
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        scaffold::slugify(stem)
    });
    let slug = opts.slug.clone().or(stem_slug);
    if asset.is_some() && slug.is_none() {
        return Err(Error::New(
            "the asset's filename can't be made into a slug, so give `--slug`".to_string(),
        ));
    }

    let id = resolve_id(&model, schema, knowledge, opts, slug.as_deref(), &states)?;
    // `--id-only` loads and compiles the knowledge as a full `new` does,
    // because the id template is in the compiled schema.
    if opts.id_only {
        return Ok(NewResult {
            id,
            path: RepoPath::root(),
            document: String::new(),
            created: false,
        });
    }

    // sokf-implements: CLI-6_AC-52
    let path = match &asset {
        Some((_, sidecar)) => sidecar.clone(),
        None => scaffold::place(&site, knowledge, &id, dir)?,
    };
    // sokf-implements: CLI-6_AC-16
    // sokf-implements: CLI-6_AC-59
    let seed = scaffold::seed(schema, &states).map_err(|refusals| {
        let schema = knowledge
            .and_then(|k| compiled.for_type(&model, k, type_name))
            .map(|(r, _)| model.schema(r).doc.path.clone());
        Error::New(scaffold::no_template(type_name, schema.as_ref(), &refusals))
    })?;
    let gated = scaffold::gate(&seed.document, &states);
    let identity = scaffold::identity(
        &id,
        slug.as_deref(),
        opts.title.as_deref(),
        states.clone(),
        asset.as_ref().map(|(a, _)| a.clone()),
    );
    let filled = scaffold::fill(&gated, schema, &identity);
    let document = scaffold::bind(
        &filled, &path, schema, &states, &model, &compiled, knowledge,
    )?;

    let mut created = false;
    // sokf-implements: CLI-6_AC-9
    // sokf-implements: CLI-9_AC-3
    if opts.write {
        let fs_path = path.to_fs(root);
        // sokf-implements: CLI-6_AC-20
        if fs_path.exists() {
            return Err(Error::New(format!("{path} exists, so sokf writes nothing")));
        }
        scaffold::create_new(&fs_path, &document)?;
        created = true;
    }
    Ok(NewResult {
        id,
        path,
        document,
        created,
    })
}

/// `chosen_store` returns the store that the command chooses for the type:
/// the store that qualifies the type, or else the store that holds the path
/// of `--dir` or `--for`, when that store declares the type. It refuses a
/// store that no root has, and a qualified type whose store differs from
/// the store of the path.
// sokf-implements: CLI-6_AC-60
// sokf-implements: CLI-6_AC-61
// sokf-implements: CLI-6_AC-62
fn chosen_store(
    root: &RepoRoot,
    model: &Model,
    opts: &NewOptions,
    store: Option<&str>,
    type_name: &str,
) -> Result<Option<usize>> {
    let named = match store {
        None => None,
        Some(s) => match model.stores_named(s).as_slice() {
            [k] => Some(*k),
            [] => return Err(Error::New(format!("no store is named `{s}`"))),
            _ => {
                return Err(Error::New(format!(
                    "more than one store is named `{s}`, so write its scope too, as `<scope>/{s}:{type_name}`"
                )));
            }
        },
    };
    let located = opts
        .dir
        .as_deref()
        .or(opts.for_asset.as_deref())
        .and_then(|p| p.to_str())
        .and_then(|p| PathArg(p).resolve_new(root, &opts.cwd))
        .and_then(|p| model.knowledge_of(&p))
        .filter(|&k| {
            type_name == "Schema" || !model.knowledges[k].schemas_for(type_name).is_empty()
        });
    match (named, located) {
        (Some(n), Some(l)) if n != l => Err(Error::New(format!(
            "`{store}:{type_name}` names the store in {}, but the path of `{option}` lies in {}",
            model.knowledges[n].root,
            model.knowledges[l].root,
            store = store.unwrap_or_default(),
            option = if opts.dir.is_some() { "--dir" } else { "--for" },
        ))),
        (n, l) => Ok(n.or(l)),
    }
}

/// `resolve_id` returns the id of the document under the rules of CLI-6
/// for `--slug` and `--id`.
// sokf-implements: CLI-6_AC-34
// sokf-implements: CLI-6_AC-49
// sokf-implements: CLI-6_AC-50
// sokf-implements: CLI-6_AC-18
// sokf-implements: CLI-6_AC-33
fn resolve_id(
    model: &crate::Model,
    schema: &SokfSchema,
    knowledge: Option<usize>,
    opts: &NewOptions,
    slug: Option<&str>,
    states: &crate::grammar::States,
) -> Result<String> {
    let type_name = split_store(opts.type_name.as_str()).1;
    match (schema.core.core.id_template(), knowledge) {
        (Some(template), Some(k)) => {
            if let Some(given) = &opts.id {
                return Err(Error::New(format!(
                    "`--id {given}` is given, but `{type_name}` has an id template, so use `--slug`"
                )));
            }
            let seq = if template.seq_width().is_some() {
                scaffold::next_seq(model, k, type_name, template)?
            } else {
                0
            };
            scaffold::render_id(template, seq, slug, states)
        }
        _ => {
            if opts.id_only {
                return Err(Error::New(format!(
                    "`{type_name}` has no id template, so give the id with `--id`"
                )));
            }
            if let Some(s) = &opts.slug {
                return Err(Error::New(format!(
                    "`--slug {s}` is given, but `{type_name}` has no id template, so use `--id`"
                )));
            }
            match &opts.id {
                Some(id) if is_slug(id) => Ok(id.clone()),
                Some(id) => Err(Error::New(format!("`{id}` is not a slug"))),
                None => Err(Error::New(format!(
                    "`{type_name}` has no id template, so `--id` is required"
                ))),
            }
        }
    }
}
