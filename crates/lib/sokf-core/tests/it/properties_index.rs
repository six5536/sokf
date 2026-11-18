//! These property tests check the index that sokf generates against the
//! format specification. They check the content of each index, and check that
//! the index sokf plans for a set of moves matches the index of the tree after
//! the test makes those moves.

use std::{collections::BTreeSet, fmt::Write as _, fs};

use common::TempTree;
use proptest::prelude::*;
use sokf_core::{RepoPath, fix::Move, include::DefinitionEntry, index};

use crate::common;

/// `DIRS` lists the three directories that can hold a generated document.
const DIRS: [&str; 3] = ["", "sub/", "sub/deep/"];

/// `Doc` is one generated document.
#[derive(Debug, Clone)]
struct Doc {
    dir: usize,
    /// The kind of id: 0 gives `d{i}`, 1 gives the shared id `dup`, 2 gives
    /// no id, and 3 gives broken frontmatter.
    id: u8,
    title: Option<String>,
    description: Option<String>,
}

fn arb_doc() -> impl Strategy<Value = Doc> {
    (
        0usize..3,
        0u8..4,
        // A YAML literal block keeps each line's text, but it drops blank
        // lines and changes leading and trailing spaces, so every generated
        // line starts and ends with a letter.
        prop::option::of("[A-Za-z]([A-Za-z ]{0,6}[A-Za-z])?(\n[a-z]{1,4})?"),
        prop::option::of("[a-z]([a-z. ]{0,10}[a-z])?(\n[a-z]{1,4})?"),
    )
        .prop_map(|(dir, id, title, description)| Doc {
            dir,
            id,
            title,
            description,
        })
}

fn arb_docs() -> impl Strategy<Value = Vec<Doc>> {
    prop::collection::vec(arb_doc(), 0..6)
}

fn write_tree(docs: &[Doc]) -> TempTree {
    let tree = TempTree::empty("index-prop");
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: Store\nscope: test\n",
    );
    tree.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\ntitle: Note\n---\n\n````yaml\n````\n",
    );
    tree.write("knowledge/sub/notes.txt", "not listed\n");
    for (i, d) in docs.iter().enumerate() {
        let mut fm = String::from("type: Note\n");
        match d.id {
            0 => {
                let _ = writeln!(fm, "id: d{i}");
            }
            1 => fm.push_str("id: dup\n"),
            2 => {}
            _ => fm.push_str("id: &x bad\n"),
        }
        if let Some(t) = &d.title {
            let _ = write!(fm, "title: |\n  {}\n", t.replace('\n', "\n  "));
        }
        if let Some(t) = &d.description {
            let _ = write!(fm, "description: |\n  {}\n", t.replace('\n', "\n  "));
        }
        tree.write(
            &format!("knowledge/{}d{i}.md", DIRS[d.dir]),
            &format!("---\n{fm}---\n\n# D{i}\n"),
        );
    }
    tree
}

/// `expected_entry` returns the index entry and link definition that the
/// format specification (`SOKF_FORMAT_103` to `SOKF_FORMAT_107`) requires for a
/// generated document. The function computes them from the test's generated
/// input alone, not from the code under test.
fn expected_entry(docs: &[Doc], i: usize) -> (String, Option<DefinitionEntry>) {
    let d = &docs[i];
    let dups = docs.iter().filter(|o| o.id == 1).count();
    let unique = d.id == 0 || (d.id == 1 && dups == 1);
    if !unique {
        return (format!("- [d{i}](d{i}.md)"), None);
    }
    let id = if d.id == 0 {
        format!("d{i}")
    } else {
        "dup".to_string()
    };
    let one_line = |s: &str| s.replace('\n', " ");
    let title = d.title.as_deref().map_or(id.clone(), one_line);
    let mut line = format!("- [{title}][sokf:{id}]");
    if let Some(desc) = &d.description {
        let _ = write!(line, " - {}", one_line(desc));
    }
    let path = RepoPath::parse(&format!("/knowledge/{}d{i}.md", DIRS[d.dir])).unwrap();
    (
        line,
        Some(DefinitionEntry {
            label: format!("sokf:{id}"),
            path,
            slug: None,
        }),
    )
}

// sokf-tests: FORMAT_P-11
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    #[test]
    fn the_expected_index_meets_spec_9(docs in arb_docs()) {
        let tree = write_tree(&docs);
        let model = tree.model();
        let indexes = index::expected(&model, &[]);
        let mut dirs: BTreeSet<&str> = BTreeSet::from(["/knowledge", "/knowledge/schemas", "/knowledge/sub"]);
        if docs.iter().any(|d| d.dir == 2) {
            dirs.insert("/knowledge/sub/deep");
        }
        let keys: Vec<&str> = indexes.expected.keys().map(RepoPath::as_str).collect();
        let mut want: Vec<String> = dirs.iter().map(|d| format!("{d}/index.md")).collect();
        want.sort();
        prop_assert_eq!(keys, want);
        for dir in &dirs {
            let text = &indexes.expected[&RepoPath::parse(&format!("{dir}/index.md")).unwrap()];
            let heading = if *dir == "/knowledge" { "Store".to_string() } else { dir.rsplit('/').next().unwrap().to_string() };
            // The entries are in path order, so a subdirectory `sub` comes before a file `sub.md`.
            let mut entries: Vec<(String, String, Option<DefinitionEntry>)> = Vec::new();
            let dir_prefix = format!("{dir}/");
            for (i, d) in docs.iter().enumerate() {
                let path = format!("/knowledge/{}d{i}.md", DIRS[d.dir]);
                if path.strip_prefix(&dir_prefix).is_some_and(|rest| !rest.contains('/')) {
                    let (line, def) = expected_entry(&docs, i);
                    entries.push((path, line, def));
                }
            }
            if *dir == "/knowledge/schemas" {
                entries.push(("/knowledge/schemas/note.md".into(), "- [Note][sokf:schema-note]".into(), Some(DefinitionEntry { label: "sokf:schema-note".into(), path: RepoPath::parse("/knowledge/schemas/note.md").unwrap(), slug: None })));
            }
            for child in &dirs {
                if child.strip_prefix(&dir_prefix).is_some_and(|rest| !rest.contains('/')) {
                    let name = child.rsplit('/').next().unwrap();
                    entries.push((child.to_string(), format!("- [{name}/]({name}/index.md)"), None));
                }
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut want = format!("# {heading}\n");
            if !entries.is_empty() {
                want.push('\n');
                for (_, line, _) in &entries {
                    want.push_str(line);
                    want.push('\n');
                }
                let mut defs: Vec<DefinitionEntry> = entries.iter().filter_map(|(_, _, d)| d.clone()).collect();
                defs.sort();
                if !defs.is_empty() {
                    want.push('\n');
                    want.push_str("<!-- sokf:links -->\n\n");
                    for d in &defs {
                        let _ = writeln!(want, "[{}]: {}", d.label, d.path);
                    }
                }
            }
            prop_assert_eq!(text, &want);
        }
    }

    #[test]
    fn a_planned_tree_indexes_as_the_tree_after_the_moves(docs in arb_docs(), which in 0usize..6, to in 0usize..4) {
        prop_assume!(!docs.is_empty());
        let which = which % docs.len();
        let tree = write_tree(&docs);
        let before = tree.model();
        let from = RepoPath::parse(&format!("/knowledge/{}d{which}.md", DIRS[docs[which].dir])).unwrap();
        let target_dir = ["/knowledge", "/knowledge/sub", "/knowledge/new", "/knowledge/new/deeper"][to];
        let to = RepoPath::parse(&format!("{target_dir}/d{which}.md")).unwrap();
        prop_assume!(from != to);
        let planned = index::expected(&before, &[Move { from: from.clone(), to: to.clone() }]);
        let fs_to = to.to_fs(&tree.root());
        fs::create_dir_all(fs_to.parent().unwrap()).unwrap();
        fs::rename(from.to_fs(&tree.root()), &fs_to).unwrap();
        let after = tree.model();
        prop_assert_eq!(planned, index::expected(&after, &[]));
    }
}
