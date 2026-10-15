//! This module builds the model from the tree. It loads each file according
//! to its kind, indexes the documents, matches the `additional-files` and
//! `asset-files` globs, and reports each file that more than one schema
//! matches.

use std::collections::HashMap;

use globset::GlobMatcher;

use super::{
    AdditionalFile, Asset, Knowledge, Loader, Model, ParseMode, SchemaDocument, SchemaRef, manifest,
};
use crate::{FileKind, Finding, RepoPath, Result, Tree, walk, yaml};

/// `build` builds the model from `tree`. It reports one finding for a file
/// that fails to load, and still builds the rest of the tree.
// sokf-implements: SOKF_SCHEMA_5
// sokf-implements: SOKF_SCHEMA_6
pub fn build(tree: Tree, loader: &mut dyn Loader) -> Result<Model> {
    build_with(tree, loader, None)
}

/// `build_with` builds the model as `build` does, and asks `fallback`, the
/// store, for the documents that the loader doesn't give (CACHE-1_AC-21).
pub fn build_with(
    tree: Tree,
    loader: &mut dyn Loader,
    fallback: Option<Box<dyn super::Fallback>>,
) -> Result<Model> {
    let mut problems = Vec::new();
    let mut knowledges = Vec::new();
    // sokf-implements: CONF-2_AC-2
    for (k, root) in tree.roots.iter().enumerate() {
        knowledges.push(build_knowledge(
            &tree,
            k,
            root,
            loader,
            fallback.as_deref(),
            &mut problems,
        ));
    }

    // Additional files: this pass matches each file of the tree against the
    // `additional-files` globs of every schema. A file that matches is an
    // additional file unless it is a document, a schema, an index, or a
    // manifest.
    let mut matchers: Vec<(SchemaRef, String, GlobMatcher)> = Vec::new();
    for (k, knowledge) in knowledges.iter().enumerate() {
        for (i, schema) in knowledge.schemas.iter().enumerate() {
            for g in schema.globs.iter().filter(|g| g.starts_with('/')) {
                if let Ok(glob) = walk::repo_glob(g) {
                    matchers.push((
                        SchemaRef {
                            knowledge: k,
                            index: i,
                        },
                        g.clone(),
                        glob.compile_matcher(),
                    ));
                }
            }
        }
    }
    let mut additional = Vec::new();
    let mut dispatch = std::collections::BTreeMap::new();
    for file in &tree.files {
        let matches: Vec<&(SchemaRef, String, GlobMatcher)> = matchers
            .iter()
            .filter(|(_, _, m)| m.is_match(file.path.relative()))
            .collect();
        if matches.is_empty() {
            continue;
        }
        // A glob must not match a document or a knowledge's index, because
        // sokf generates the index (SOKF_SCHEMA_5).
        let what = match file.kind {
            FileKind::Document | FileKind::Schema => Some("document"),
            FileKind::Index => Some("index"),
            _ => None,
        };
        if let Some(what) = what {
            for (r, g, _) in &matches {
                let schema = &knowledges[r.knowledge].schemas[r.index];
                problems.push(schema.finding(
                    "SOKF_SCHEMA_5",
                    format!(
                        "the `additional-files` glob `{g}` matches the {what} `{}`",
                        file.path
                    ),
                ));
            }
            continue;
        }
        if file.kind == FileKind::Manifest {
            continue;
        }
        // sokf-implements: CONF-2_AC-5
        if matches.len() > 1 {
            let mut names: Vec<String> = matches
                .iter()
                .map(|(r, _, _)| knowledges[r.knowledge].schemas[r.index].id())
                .collect();
            names.sort();
            names.dedup();
            for (r, _, _) in &matches {
                let schema = &knowledges[r.knowledge].schemas[r.index];
                problems.push(schema.finding(
                    "SOKF_SCHEMA_6",
                    format!(
                        "{} matches the schemas {}, but a file can match only one schema",
                        file.path,
                        names.join(" and ")
                    ),
                ));
            }
        }
        let schema = if matches.len() == 1 {
            Some(matches[0].0)
        } else {
            None
        };
        dispatch.insert(file.path.clone(), schema);
        match loader.additional(&file.path) {
            Ok(None) => {}
            Ok(Some(source)) => {
                let parsed = loader.parse(&source, ParseMode::Plain);
                additional.push(AdditionalFile {
                    path: file.path.clone(),
                    source,
                    body: parsed.body,
                    segments: parsed.segments,
                    schema,
                });
            }
            Err(e) => problems.push(load_failure(&file.path, &e)),
        }
    }

    // Assets: an asset is a candidate file that matches the `asset-files`
    // globs of exactly one schema in its knowledge. This pass pairs each asset
    // with the `.md` file beside it (FORMAT-1_AC-1). It runs after the pass
    // over the additional files, so that it can report a glob that matches an
    // additional file.
    for k in 0..knowledges.len() {
        let (assets, found) = asset_pass(&tree, &knowledges, k, &additional, fallback.as_deref());
        problems.extend(found);
        let knowledge = &mut knowledges[k];
        for asset in assets {
            let i = knowledge.assets.len();
            knowledge.asset_by_path.insert(asset.path.clone(), i);
            if asset.has_sidecar {
                knowledge.asset_of_sidecar.insert(asset.sidecar_path(), i);
            }
            knowledge.assets.push(asset);
        }
    }

    Ok(Model {
        knowledges,
        additional,
        dispatch,
        tree,
        problems,
        reads: Default::default(),
        fallback,
        markers: Default::default(),
        marker_check: crate::MarkerCheck::Off,
        plans: Default::default(),
    })
}

/// `asset_pass` returns the assets of knowledge `k`, and the problems that
/// the `asset-files` globs of its schemas cause.
// sokf-implements: FORMAT-1_AC-1
// sokf-implements: EXT-2_AC-1
fn asset_pass(
    tree: &Tree,
    knowledges: &[Knowledge],
    k: usize,
    additional: &[AdditionalFile],
    fallback: Option<&dyn super::Fallback>,
) -> (Vec<Asset>, Vec<Finding>) {
    let knowledge = &knowledges[k];
    let mut problems = Vec::new();
    let mut matchers: Vec<(usize, String, GlobMatcher)> = Vec::new();
    for (i, schema) in knowledge.schemas.iter().enumerate() {
        for g in &schema.asset_globs {
            // The grammar reports a glob that lies outside the knowledge
            // (SOKF_SCHEMA_8), and this pass skips it, so the glob matches nothing.
            if !crate::grammar::glob_inside(g, &knowledge.root) {
                continue;
            }
            if let Ok(glob) = walk::repo_glob(g) {
                matchers.push((i, g.clone(), glob.compile_matcher()));
            }
        }
    }
    if matchers.is_empty() {
        return (Vec::new(), problems);
    }
    let schemas_dir = knowledge.root.join("schemas");
    let mut assets = Vec::new();
    for file in tree.files.iter().filter(|f| f.root == Some(k)) {
        let matches: Vec<&(usize, String, GlobMatcher)> = matchers
            .iter()
            .filter(|(_, _, m)| m.is_match(file.path.relative()))
            .collect();
        if matches.is_empty() {
            continue;
        }
        let refused = match file.kind {
            FileKind::Manifest | FileKind::Index => Some("a reserved file"),
            FileKind::Schema => Some("a file under `schemas/`"),
            FileKind::Document => Some("a `.md` file"),
            FileKind::Candidate
                if schemas_dir
                    .as_ref()
                    .is_some_and(|d| file.path.starts_with(d)) =>
            {
                Some("a file under `schemas/`")
            }
            FileKind::Candidate if additional.iter().any(|a| a.path == file.path) => {
                Some("an additional file")
            }
            FileKind::Candidate => None,
        };
        if let Some(what) = refused {
            for (i, g, _) in &matches {
                let schema = &knowledge.schemas[*i];
                problems.push(schema.finding(
                    "SOKF_SCHEMA_7",
                    format!("The `asset-files` glob `{g}` matches {what}: {}", file.path),
                ));
            }
            continue;
        }
        let mut claimants: Vec<usize> = matches.iter().map(|(i, _, _)| *i).collect();
        claimants.sort_unstable();
        claimants.dedup();
        if claimants.len() > 1 {
            let names: Vec<String> = claimants
                .iter()
                .map(|&i| knowledge.schemas[i].id())
                .collect();
            for &i in &claimants {
                let schema = &knowledge.schemas[i];
                problems.push(schema.finding(
                    "SOKF_SCHEMA_9",
                    format!(
                        "{} matches the `asset-files` globs of the schemas {}, but a file can match only one schema",
                        file.path,
                        names.join(" and ")
                    ),
                ));
            }
            continue;
        }
        let sidecar_path = super::sidecar_path(&file.path);
        let sidecar = knowledge.by_path.get(&sidecar_path).copied();
        let has_sidecar =
            sidecar.is_some() || fallback.is_some_and(|f| f.is_document(&sidecar_path));
        assets.push(Asset {
            path: file.path.clone(),
            schema: SchemaRef {
                knowledge: k,
                index: claimants[0],
            },
            sidecar,
            has_sidecar,
        });
    }
    (assets, problems)
}

fn load_failure(path: &RepoPath, e: &crate::Error) -> Finding {
    let message = match e {
        crate::Error::Io { source, .. } => format!("sokf cannot read the file: {source}"),
        other => format!("sokf cannot read the file: {other}"),
    };
    Finding::error(path.clone(), None, "SOKF_FORMAT_17", message)
}

// sokf-implements: SOKF_FORMAT_3
fn build_knowledge(
    tree: &Tree,
    k: usize,
    root: &RepoPath,
    loader: &mut dyn Loader,
    fallback: Option<&dyn super::Fallback>,
    problems: &mut Vec<Finding>,
) -> Knowledge {
    let manifest_path = root.join("manifest.sokf.yaml").expect("literal name");
    let mut knowledge = Knowledge {
        root: root.clone(),
        manifest: None,
        manifest_path: manifest_path.clone(),
        schemas: Vec::new(),
        documents: Vec::new(),
        indexes: Vec::new(),
        assets: Vec::new(),
        by_id: HashMap::new(),
        by_path: HashMap::new(),
        by_type: HashMap::new(),
        schemas_by_type: HashMap::new(),
        asset_by_path: HashMap::new(),
        asset_of_sidecar: HashMap::new(),
    };
    let schemas_dir = root.join("schemas");
    for file in tree.files.iter().filter(|f| f.root == Some(k)) {
        // `schemas/` holds Schema documents, which lie directly in it, and
        // indexes. A document in a subdirectory and a file that is not
        // Markdown are neither (SOKF_FORMAT_3).
        if matches!(file.kind, FileKind::Document | FileKind::Candidate)
            && schemas_dir
                .as_ref()
                .is_some_and(|d| file.path.starts_with(d))
        {
            problems.push(Finding::error(
                file.path.clone(),
                None,
                "SOKF_FORMAT_3",
                "`schemas/` holds only Schema documents, each directly in it, and the index",
            ));
        }
        match file.kind {
            FileKind::Manifest => match loader.load(&file.path) {
                Ok(source) => {
                    knowledge.manifest = Some(manifest(yaml::parse(&source.text, 1), &file.path));
                }
                Err(e) => problems.push(load_failure(&file.path, &e)),
            },
            // When the loader doesn't read a schema, sokf uses the store's
            // metadata for the schema instead. When the store has none, the
            // loader reads the whole schema (CACHE-1_AC-25).
            FileKind::Schema => match loader.schema(&file.path) {
                Ok(read) => {
                    let schema = match read {
                        Some(doc) => SchemaDocument::new(doc),
                        None => match fallback.and_then(|f| f.schema_meta(&file.path)) {
                            Some(meta) => SchemaDocument::placeholder(file.path.clone(), meta),
                            None => match loader.full_document(&file.path) {
                                Ok(doc) => SchemaDocument::new(doc),
                                Err(e) => {
                                    problems.push(load_failure(&file.path, &e));
                                    continue;
                                }
                            },
                        },
                    };
                    if let Some(t) = &schema.type_const {
                        knowledge
                            .schemas_by_type
                            .entry(t.clone())
                            .or_default()
                            .push(knowledge.schemas.len());
                    }
                    knowledge.schemas.push(schema);
                }
                Err(e) => problems.push(load_failure(&file.path, &e)),
            },
            FileKind::Document => match loader.document(&file.path) {
                Ok(None) => {}
                // The build pairs assets with their sidecars later, so the
                // insert pairs none here.
                Ok(Some(doc)) => knowledge.insert_document(doc),
                Err(e) => problems.push(load_failure(&file.path, &e)),
            },
            FileKind::Index => match loader.index(&file.path) {
                Ok(None) => {}
                Ok(Some(index)) => knowledge.indexes.push(index),
                Err(e) => problems.push(load_failure(&file.path, &e)),
            },
            FileKind::Candidate => {}
        }
    }
    knowledge
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::{Config, FsLoader, Located, RepoRoot, collect};

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("build-{name}"));
        fs::create_dir_all(dir.join("knowledge/schemas")).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(
            dir.join("knowledge/manifest.sokf.yaml"),
            "sokf: \"0.6\"\nname: k\nscope: test\n",
        )
        .unwrap();
        fs::write(dir.join("knowledge/index.md"), "# Index\n").unwrap();
        fs::write(
            dir.join("knowledge/schemas/note.md"),
            "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/readme.md"),
            "---\ntype: Schema\nid: schema-readme\n---\n\n````yaml\nline-limit: 0\nadditional-files: ['/README.md', '/docs/**/*.md', '/knowledge/*.md', '/knowledge/index.md']\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/twice.md"),
            "---\ntype: Schema\nid: schema-twice\n---\n\n````yaml\nline-limit: 0\nadditional-files: '/README.md'\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/a.md"),
            "---\ntype: Note\nid: a\n---\n\n# A\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/b.md"),
            "---\ntype: Note\nid: a\n---\n\n# B\n",
        )
        .unwrap();
        fs::write(dir.join("README.md"), "# Readme\n").unwrap();
        fs::write(dir.join("docs/guide.md"), "# Guide\n").unwrap();
        fs::write(dir.join("docs/bad.md"), [0xFF]).unwrap();
        dir
    }

    #[test]
    fn additional_file_globs_stop_star_at_a_slash() {
        let dir = crate::test_support::fresh_dir("build-glob");
        fs::create_dir_all(dir.join("knowledge/schemas")).unwrap();
        fs::create_dir_all(dir.join("docs/sub")).unwrap();
        fs::write(
            dir.join("knowledge/manifest.sokf.yaml"),
            "sokf: \"0.6\"\nname: k\nscope: test\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/docs.md"),
            "---\ntype: Schema\nid: schema-docs\n---\n\n````yaml\nline-limit: 0\nadditional-files: '/docs/*.md'\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(dir.join("docs/guide.md"), "# Guide\n").unwrap();
        fs::write(dir.join("docs/sub/deep.md"), "# Deep\n").unwrap();
        let root = RepoRoot::new(dir.clone());
        let tree = collect(&root, &Config::default()).unwrap();
        let mut loader = FsLoader::new(&root);
        let model = build(tree, &mut loader).unwrap();
        let schema_of = |p: &str| {
            model
                .additional
                .iter()
                .find(|a| a.path.as_str() == p)
                .and_then(|a| a.schema)
        };
        assert!(schema_of("/docs/guide.md").is_some());
        assert!(schema_of("/docs/sub/deep.md").is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: FORMAT-1_AC-1
    // sokf-tests: EXT-2_AC-1
    #[test]
    fn the_asset_pass_pairs_matches_and_refuses_what_spec_10_names() {
        let dir = crate::test_support::fresh_dir("build-assets");
        for d in [
            "knowledge/schemas",
            "knowledge/media",
            "knowledge/data",
            "docs",
        ] {
            fs::create_dir_all(dir.join(d)).unwrap();
        }
        let schema = |id: &str, ty: &str, globs: &str| {
            format!(
                "---\ntype: Schema\nid: {id}\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: {ty}\nasset-files: {globs}\nsections: []\n````\n"
            )
        };
        fs::write(
            dir.join("knowledge/manifest.sokf.yaml"),
            "sokf: \"0.6\"\nname: k\nscope: test\n",
        )
        .unwrap();
        // The Image schema matches the PNG files in `media`. The Wide schema
        // matches every PNG file in the knowledge, so the PNG files in
        // `media` match two schemas. The other globs of the Wide schema match
        // a reserved file, a file under `schemas/`, a `.md` file, and an
        // additional file. The Outside schema has a glob outside the
        // knowledge, which matches nothing.
        fs::write(
            dir.join("knowledge/schemas/image.md"),
            schema("schema-image", "Image", "'/knowledge/media/*.png'"),
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/wide.md"),
            schema("schema-wide", "Wide", "['/knowledge/**/*.png', '/knowledge/manifest.sokf.yaml', '/knowledge/index.md', '/knowledge/schemas/*', '/knowledge/*.md', '/knowledge/notes.txt']"),
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/outside.md"),
            schema("schema-outside", "Outside", "'/docs/*.csv'"),
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/data.md"),
            schema("schema-data", "Data", "'/knowledge/data/*.csv'"),
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/readme.md"),
            "---\ntype: Schema\nid: schema-readme\n---\n\n````yaml\nline-limit: 0\nadditional-files: '/knowledge/notes.txt'\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(dir.join("knowledge/index.md"), "# k\n").unwrap();
        fs::write(dir.join("knowledge/schemas/stray.bin"), "").unwrap();
        fs::write(dir.join("knowledge/a.md"), "---\ntype: Note\nid: a\n---\n").unwrap();
        fs::write(dir.join("knowledge/notes.txt"), "notes\n").unwrap();
        fs::write(dir.join("knowledge/media/flowers.png"), "").unwrap();
        fs::write(
            dir.join("knowledge/media/flowers.png.md"),
            "---\ntype: Image\nid: flowers\n---\n",
        )
        .unwrap();
        fs::write(dir.join("knowledge/data/rates.csv"), "").unwrap();
        fs::write(
            dir.join("knowledge/data/rates.csv.md"),
            "---\ntype: Data\nid: rates\n---\n",
        )
        .unwrap();
        fs::write(dir.join("knowledge/data/orphan.csv"), "").unwrap();
        fs::write(
            dir.join("knowledge/data/loose.csv.md"),
            "---\ntype: Note\nid: loose\n---\n",
        )
        .unwrap();
        fs::write(dir.join("docs/x.csv"), "").unwrap();
        let root = RepoRoot::new(dir.clone());
        let tree = collect(&root, &Config::default()).unwrap();
        let model = build(tree, &mut FsLoader::new(&root)).unwrap();
        let k = &model.knowledges[0];
        let p = |s: &str| RepoPath::parse(s).unwrap();
        // The assets are the CSV pair and the orphan, and nothing else: the
        // PNG file matches two schemas, and the outside glob matches nothing.
        let paths: Vec<&str> = k.assets.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["/knowledge/data/orphan.csv", "/knowledge/data/rates.csv"]
        );
        let rates = k.asset_at(&p("/knowledge/data/rates.csv")).unwrap();
        assert_eq!(rates.sidecar_path(), p("/knowledge/data/rates.csv.md"));
        assert_eq!(
            rates.sidecar.map(|i| k.documents[i].id()),
            Some(Some("rates"))
        );
        assert!(
            k.asset_at(&p("/knowledge/data/orphan.csv"))
                .unwrap()
                .sidecar
                .is_none()
        );
        assert_eq!(
            model
                .asset_of(&p("/knowledge/data/rates.csv.md"))
                .map(|(_, a)| a.path.as_str()),
            Some("/knowledge/data/rates.csv")
        );
        assert_eq!(
            model
                .content_path(&p("/knowledge/data/rates.csv.md"))
                .as_str(),
            "/knowledge/data/rates.csv"
        );
        assert_eq!(
            model.content_path(&p("/knowledge/a.md")).as_str(),
            "/knowledge/a.md"
        );
        // A `.md` file beside a file that isn't an asset is a plain document.
        assert!(model.asset_of(&p("/knowledge/data/loose.csv.md")).is_none());
        assert!(matches!(
            model.by_path(&p("/knowledge/data/rates.csv")),
            Some(Located::Asset(0, _))
        ));
        let messages: Vec<&str> = model.problems.iter().map(|f| f.message.as_str()).collect();
        for needle in [
            "matches a reserved file: /knowledge/manifest.sokf.yaml",
            "matches a reserved file: /knowledge/index.md",
            "matches a file under `schemas/`: /knowledge/schemas/stray.bin",
            "matches a file under `schemas/`: /knowledge/schemas/image.md",
            "matches a `.md` file: /knowledge/a.md",
            "matches an additional file: /knowledge/notes.txt",
            "/knowledge/media/flowers.png matches the `asset-files` globs of the schemas schema-image and schema-wide",
        ] {
            assert!(
                messages.iter().any(|m| m.contains(needle)),
                "{needle}: {messages:?}"
            );
        }
        assert_eq!(
            messages
                .iter()
                .filter(|m| m.contains("schema-image and schema-wide"))
                .count(),
            2,
            "sokf reports the error on both schemas"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn builds_and_indexes_a_tree() {
        let dir = temp("build");
        let root = RepoRoot::new(dir.clone());
        let tree = collect(&root, &Config::default()).unwrap();
        let mut loader = FsLoader::new(&root);
        let model = build(tree, &mut loader).unwrap();
        assert_eq!(model.knowledges.len(), 1);
        let k = &model.knowledges[0];
        assert_eq!(k.manifest.as_ref().unwrap().name.as_deref(), Some("k"));
        assert_eq!(k.schemas.len(), 3);
        assert_eq!(k.documents.len(), 2);
        assert_eq!(k.indexes.len(), 1);
        assert_eq!(k.schemas_for("Note").len(), 1);
        assert!(k.schemas_for("Nope").is_empty());
        assert_eq!(k.documents_of("Note").count(), 2);
        // A duplicate id resolves to no document.
        assert!(k.document("a").is_none());
        assert!(model.document(0, "a").is_none());
        assert!(k.schema_by_id("schema-note").is_some());
        let p = |s: &str| RepoPath::parse(s).unwrap();
        assert!(matches!(
            model.by_path(&p("/knowledge/a.md")),
            Some(Located::Document(0, _))
        ));
        assert!(matches!(
            model.by_path(&p("/knowledge/schemas/note.md")),
            Some(Located::Schema(0, _))
        ));
        assert!(matches!(
            model.by_path(&p("/knowledge/index.md")),
            Some(Located::Index(0, _))
        ));
        assert!(matches!(
            model.by_path(&p("/docs/guide.md")),
            Some(Located::Additional(_))
        ));
        assert!(model.by_path(&p("/nope.md")).is_none());
        assert!(model.is_document_of(0, &p("/knowledge/a.md")));
        assert!(!model.is_document_of(1, &p("/knowledge/a.md")));
        assert!(model.is_materialized(&p("/docs/guide.md")));
        assert_eq!(model.knowledge_of(&p("/knowledge/x.md")), Some(0));
        assert_eq!(model.knowledge_of(&p("/x.md")), None);
        assert_eq!(model.documents().count(), 2);
        // README.md matches two schemas, so sokf reports an error on each
        // schema and assigns the file no schema.
        let readme = model
            .additional
            .iter()
            .find(|a| a.path.as_str() == "/README.md")
            .unwrap();
        assert!(readme.schema.is_none());
        let messages: Vec<&str> = model.problems.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(
            messages
                .iter()
                .filter(|m| m.contains("matches the schemas"))
                .count(),
            2
        );
        // The glob `/knowledge/*.md` matches documents, so sokf reports an
        // error on the schema.
        assert!(
            messages
                .iter()
                .any(|m| m.contains("matches the document `/knowledge/a.md`")),
            "{messages:?}"
        );
        // When a glob matches a knowledge's index, sokf reports an error on
        // the schema, and the index doesn't become an additional file.
        assert!(
            messages
                .iter()
                .any(|m| m.contains("matches the index `/knowledge/index.md`")),
            "{messages:?}"
        );
        assert!(
            model
                .additional
                .iter()
                .all(|a| a.path.as_str() != "/knowledge/index.md")
        );
        // sokf reports one finding for the unreadable file.
        assert!(
            messages.iter().any(|m| m.contains("cannot read")),
            "{messages:?}"
        );
        let guide = model
            .additional
            .iter()
            .find(|a| a.path.as_str() == "/docs/guide.md")
            .unwrap();
        assert_eq!(
            guide.schema,
            Some(SchemaRef {
                knowledge: 0,
                index: 1
            })
        );
        assert_eq!(model.schema(guide.schema.unwrap()).id(), "schema-readme");
        fs::remove_dir_all(&dir).unwrap();
    }
}
