//! This module applies the planned moves to the document paths, then
//! computes the expected index of every directory from the model (format
//! specification §9.4).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{IndexEntry, render};
use crate::{FileKind, Model, RepoPath, fix::Move};

/// `Indexes` holds the expected text of every index, keyed by the index's
/// path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Indexes {
    /// `expected` maps each directory's `index.md` path to its expected
    /// text, with LF line endings.
    pub expected: BTreeMap<RepoPath, String>,
}

/// `touched` returns the directories whose indexes the moves change: the
/// directory that each move leaves and the one it enters. When a move
/// enters a directory that the walk didn't find, the move creates it, so
/// each directory above it gains an entry, up to the first directory that
/// exists, and `touched` returns those too.
// sokf-implements: SOKF_VALIDATION_55
pub fn touched(model: &Model, moves: &[Move]) -> BTreeSet<RepoPath> {
    let found: BTreeSet<&RepoPath> = model.tree.dirs.iter().map(|d| &d.path).collect();
    let mut out = BTreeSet::new();
    for m in moves {
        out.extend(m.from.parent());
        let mut dir = m.to.parent();
        while let Some(d) = dir {
            let exists = found.contains(&d);
            dir = d.parent();
            out.insert(d);
            if exists {
                break;
            }
        }
    }
    out
}

/// `expected` applies `moves` to the document paths, then computes the
/// expected index of every directory that the walk visited under a root. It
/// also computes the expected index of each directory that a move enters. A
/// check calls `expected` with no moves.
// sokf-implements: SOKF_FORMAT_96
// sokf-implements: SOKF_VALIDATION_33
pub fn expected(model: &Model, moves: &[Move]) -> Indexes {
    expected_for(model, moves, None)
}

/// `expected_for` computes the expected indexes of the directories in
/// `only`, or of every directory when `only` is `None`. An incremental run
/// uses it to compute the indexes of the directories that a change touched,
/// from the walk's files and the store's rows (CACHE-1_AC-18).
// sokf-implements: SOKF_FORMAT_96
// sokf-implements: SOKF_FORMAT_99
// sokf-implements: SOKF_FORMAT_100
// sokf-implements: SOKF_FORMAT_101
// sokf-implements: SOKF_FORMAT_102
// sokf-implements: SOKF_FORMAT_104
// sokf-implements: SOKF_FORMAT_107
// sokf-implements: SOKF_FORMAT_140
// sokf-implements: CACHE-1_AC-18
pub fn expected_for(model: &Model, moves: &[Move], only: Option<&BTreeSet<RepoPath>>) -> Indexes {
    let remap: HashMap<&RepoPath, &RepoPath> = moves.iter().map(|m| (&m.from, &m.to)).collect();
    let roots: Vec<RepoPath> = model.knowledges.iter().map(|k| k.root.clone()).collect();
    let wanted = |dir: &RepoPath| only.is_none_or(|o| o.contains(dir));

    // Find every directory that has an index, and the knowledge that owns it.
    let mut dirs: BTreeMap<RepoPath, usize> = model
        .tree
        .dirs
        .iter()
        .map(|d| (d.path.clone(), d.root))
        .collect();
    for m in moves {
        let mut dir = m.to.parent();
        while let Some(d) = dir {
            if dirs.contains_key(&d) {
                break;
            }
            let Some(owner) = crate::walk::innermost(&roots, &d) else {
                break;
            };
            dirs.insert(d.clone(), owner);
            dir = d.parent();
        }
    }

    // Group the `.md` files by directory, at their paths after the moves.
    let mut files_of: BTreeMap<RepoPath, Vec<(&RepoPath, &RepoPath)>> = BTreeMap::new();
    for file in &model.tree.files {
        if !matches!(file.kind, FileKind::Document | FileKind::Schema) {
            continue;
        }
        let landed = remap.get(&file.path).map_or(&file.path, |p| p);
        if let Some(parent) = landed.parent()
            && wanted(&parent)
        {
            files_of
                .entry(parent)
                .or_default()
                .push((landed, &file.path));
        }
    }
    // Group the directories by their parent directory.
    let mut subdirs_of: BTreeMap<RepoPath, Vec<&RepoPath>> = BTreeMap::new();
    for d in dirs.keys() {
        if let Some(parent) = d.parent()
            && wanted(&parent)
        {
            subdirs_of.entry(parent).or_default().push(d);
        }
    }

    let mut out = Indexes::default();
    for (dir, &k) in &dirs {
        if !wanted(dir) {
            continue;
        }
        let knowledge = &model.knowledges[k];
        let mut listed: Vec<(RepoPath, IndexEntry)> = Vec::new();
        for &(landed, original) in files_of.get(dir).into_iter().flatten() {
            listed.push((
                landed.clone(),
                entry_for(model, k, landed, original, &remap),
            ));
        }
        for &child in subdirs_of.get(dir).into_iter().flatten() {
            listed.push((
                child.clone(),
                IndexEntry::Directory {
                    name: child.file_name().to_string(),
                },
            ));
        }
        listed.sort_by(|a, b| a.0.cmp(&b.0));
        let entries: Vec<IndexEntry> = listed.into_iter().map(|(_, e)| e).collect();
        let heading = if dir == &knowledge.root {
            knowledge
                .manifest
                .as_ref()
                .and_then(|m| m.name.as_deref())
                .filter(|n| !n.is_empty())
                .unwrap_or(dir.file_name())
        } else {
            dir.file_name()
        };
        if let Some(path) = dir.join("index.md") {
            out.expected.insert(path, render(heading, &entries));
        }
    }
    out
}

/// `entry_for` returns a document's entry: the id form when the document's
/// `id` is unique in its knowledge, and the path form otherwise.
fn entry_for(
    model: &Model,
    k: usize,
    landed: &RepoPath,
    original: &RepoPath,
    remap: &HashMap<&RepoPath, &RepoPath>,
) -> IndexEntry {
    let name = landed.file_name().trim_end_matches(".md").to_string();
    let fields = match model.loaded_document(original) {
        Some((_, d)) => Some(crate::model::Fields {
            id: d.core.id.clone(),
            title: d.core.title.clone(),
            description: d.core.description.clone(),
        }),
        // The fields of a document or schema that the run skipped come from
        // the store.
        None => model.fallback.as_ref().and_then(|f| f.fields(original)),
    };
    let Some(fields) = fields else {
        return IndexEntry::File { name };
    };
    let Some(id) = fields.id.as_deref() else {
        return IndexEntry::File { name };
    };
    let unique = model.ids(k, id).as_slice() == [original.clone()];
    if !unique {
        return IndexEntry::File { name };
    }
    // A sidecar's entry links to its asset, at the asset's path after the
    // moves (SOKF_FORMAT_81), because the id identifies the content.
    let path = match model.knowledges[k].asset_of(original) {
        Some(asset) => remap.get(&asset.path).map_or(&asset.path, |p| p).clone(),
        None => landed.clone(),
    };
    IndexEntry::Document {
        title: fields.title.unwrap_or_else(|| id.to_string()),
        id: id.to_string(),
        path,
        description: fields.description,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::test_support::{model_at, write};

    fn p(s: &str) -> RepoPath {
        RepoPath::parse(s).unwrap()
    }

    fn temp(name: &str) -> PathBuf {
        crate::test_support::fresh_dir(&format!("index-{name}"))
    }

    fn store(dir: &std::path::Path) {
        write(
            dir,
            "knowledge/manifest.sokf.yaml",
            "sokf: \"0.6\"\nname: Project KB\nscope: test\n",
        );
        write(
            dir,
            "knowledge/schemas/note.md",
            "---\ntype: Schema\nid: schema-note\ntitle: Note\ndescription: A note.\n---\n\n````yaml\n````\n",
        );
        // The store has a subdirectory `core` and a file `core.md`, and the
        // index must list the directory first.
        write(
            dir,
            "knowledge/core.md",
            "---\ntype: Note\nid: core\ntitle: Core\ndescription: |\n  Two\n  lines.\n---\n\n# Core\n",
        );
        write(
            dir,
            "knowledge/core/a.md",
            "---\ntype: Note\nid: a\n---\n\n# A\n",
        );
        // The next files hold a duplicate id, a document with no id, and a
        // broken document.
        write(
            dir,
            "knowledge/core/dup1.md",
            "---\ntype: Note\nid: dup\n---\n\n# D1\n",
        );
        write(
            dir,
            "knowledge/core/dup2.md",
            "---\ntype: Note\nid: dup\n---\n\n# D2\n",
        );
        write(
            dir,
            "knowledge/core/noid.md",
            "---\ntype: Note\n---\n\n# N\n",
        );
        write(dir, "knowledge/core/broken.md", "---\n: [\n---\n\n# B\n");
        write(dir, "knowledge/core/notes.txt", "not listed\n");
        write(dir, "knowledge/empty/.keep", "");
    }

    // sokf-tests: SOKF_FORMAT_96
    // sokf-tests: SOKF_FORMAT_101
    // sokf-tests: SOKF_FORMAT_102
    // sokf-tests: SOKF_FORMAT_104
    // sokf-tests: SOKF_FORMAT_105
    // sokf-tests: SOKF_FORMAT_107
    #[test]
    fn lists_every_directory_in_the_forms_of_spec_9() {
        let dir = temp("forms");
        store(&dir);
        let m = model_at(&dir, &["/knowledge"]);
        let idx = expected(&m, &[]);
        let keys: Vec<&str> = idx.expected.keys().map(RepoPath::as_str).collect();
        assert_eq!(
            keys,
            vec![
                "/knowledge/core/index.md",
                "/knowledge/empty/index.md",
                "/knowledge/index.md",
                "/knowledge/schemas/index.md",
            ]
        );
        assert_eq!(
            idx.expected[&p("/knowledge/index.md")],
            "# Project KB\n\n\
             - [core/](core/index.md)\n\
             - [Core][sokf:core] - Two lines.\n\
             - [empty/](empty/index.md)\n\
             - [schemas/](schemas/index.md)\n\n\
             <!-- sokf:links -->\n\n\
             [sokf:core]: /knowledge/core.md\n"
        );
        assert_eq!(
            idx.expected[&p("/knowledge/core/index.md")],
            "# core\n\n\
             - [a][sokf:a]\n\
             - [broken](broken.md)\n\
             - [dup1](dup1.md)\n\
             - [dup2](dup2.md)\n\
             - [noid](noid.md)\n\n\
             <!-- sokf:links -->\n\n\
             [sokf:a]: /knowledge/core/a.md\n"
        );
        assert_eq!(idx.expected[&p("/knowledge/empty/index.md")], "# empty\n");
        // The index lists a schema as a document.
        assert_eq!(
            idx.expected[&p("/knowledge/schemas/index.md")],
            "# schemas\n\n- [Note][sokf:schema-note] - A note.\n\n\
             <!-- sokf:links -->\n\n[sokf:schema-note]: /knowledge/schemas/note.md\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: SOKF_FORMAT_99
    // sokf-tests: SOKF_FORMAT_100
    #[test]
    fn heading_falls_back_to_the_directory_name_and_a_nested_root_is_its_own() {
        let dir = temp("nested");
        write(
            &dir,
            "knowledge/manifest.sokf.yaml",
            "sokf: \"0.6\"\nname: \"\"\n",
        );
        write(
            &dir,
            "knowledge/inner/manifest.sokf.yaml",
            "sokf: \"0.6\"\nname: Inner\nscope: test\n",
        );
        write(
            &dir,
            "knowledge/inner/i.md",
            "---\ntype: Note\nid: i\n---\n\n# I\n",
        );
        write(
            &dir,
            "docs/manifest.sokf.yaml",
            "sokf: \"0.6\"\nname: [1]\n",
        );
        let m = model_at(&dir, &["/knowledge", "/knowledge/inner", "/docs"]);
        let idx = expected(&m, &[]);
        assert_eq!(
            idx.expected[&p("/knowledge/index.md")],
            "# knowledge\n\n- [inner/](inner/index.md)\n"
        );
        assert_eq!(
            idx.expected[&p("/knowledge/inner/index.md")],
            "# Inner\n\n- [i][sokf:i]\n\n<!-- sokf:links -->\n\n[sokf:i]: /knowledge/inner/i.md\n"
        );
        assert_eq!(idx.expected[&p("/docs/index.md")], "# docs\n");
        fs::remove_dir_all(&dir).unwrap();
    }

    // sokf-tests: SOKF_VALIDATION_33
    // sokf-tests: CLI-4_AC-5
    // sokf-tests: SOKF_VALIDATION_55
    #[test]
    fn moves_relist_a_document_where_it_lands_and_name_the_touched_directories() {
        let dir = temp("moves");
        store(&dir);
        let m = model_at(&dir, &["/knowledge"]);
        let moves = vec![
            Move {
                from: p("/knowledge/core/a.md"),
                to: p("/knowledge/closed/deep/a.md"),
            },
            Move {
                from: p("/knowledge/core/noid.md"),
                to: p("/knowledge/core/noid.md"),
            },
        ];
        // The move creates `closed/` and `closed/deep/`, so `/knowledge` and
        // `closed/` gain an entry too.
        assert_eq!(
            touched(&m, &moves),
            BTreeSet::from([
                p("/knowledge"),
                p("/knowledge/closed"),
                p("/knowledge/closed/deep"),
                p("/knowledge/core"),
            ])
        );
        let idx = expected(&m, &moves);
        // Each directory the move enters gets an index, including the
        // intermediate one, and the outer index lists the new subdirectory.
        assert_eq!(
            idx.expected[&p("/knowledge/closed/index.md")],
            "# closed\n\n- [deep/](deep/index.md)\n"
        );
        assert_eq!(
            idx.expected[&p("/knowledge/closed/deep/index.md")],
            "# deep\n\n- [a][sokf:a]\n\n<!-- sokf:links -->\n\n[sokf:a]: /knowledge/closed/deep/a.md\n"
        );
        assert!(!idx.expected[&p("/knowledge/core/index.md")].contains("sokf:a"));
        assert!(idx.expected[&p("/knowledge/index.md")].contains("- [closed/](closed/index.md)\n"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
