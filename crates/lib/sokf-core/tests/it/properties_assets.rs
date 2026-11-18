//! Property tests over assets and sidecars (contract-sokf-format): how an
//! asset binds to its sidecar, which path each link definition holds, how
//! the pair moves together, and whether any check reads an asset's bytes.

use std::{fmt::Write as _, fs};

use common::TempTree;
use proptest::prelude::*;
use sokf_core::{CacheMode, RepoPath};

use crate::common;

/// One generated file under `/knowledge/media/<dir>/`.
#[derive(Debug, Clone)]
struct File {
    /// The directory, as an index into `DIRS`: 0 is `draft` and 1 is
    /// `published`.
    dir: usize,
    /// The file name without its extension.
    stem: String,
    /// Whether the asset glob matches the file: a matched file ends in
    /// `.png`, and an unmatched one in `.txt`.
    matched: bool,
    /// Whether the `.md` beside it exists.
    sidecar: bool,
    /// The sidecar's `state`, as an index into `DIRS`. The state decides
    /// which directory the pair belongs in.
    state: usize,
    /// Whether the sidecar's `resource` holds the path of the file beside it.
    resource_ok: bool,
}

fn arb_file(i: usize) -> impl Strategy<Value = File> {
    (
        0usize..2,
        any::<bool>(),
        any::<bool>(),
        0usize..2,
        any::<bool>(),
    )
        .prop_map(move |(dir, matched, sidecar, state, resource_ok)| File {
            dir,
            stem: format!("f{i}"),
            matched,
            sidecar,
            state,
            resource_ok,
        })
}

fn arb_files() -> impl Strategy<Value = Vec<File>> {
    (1usize..5).prop_flat_map(|n| (0..n).map(arb_file).collect::<Vec<_>>())
}

const DIRS: [&str; 2] = ["draft", "published"];

impl File {
    fn ext(&self) -> &'static str {
        if self.matched { "png" } else { "txt" }
    }

    fn path(&self) -> String {
        format!(
            "/knowledge/media/{}/{}.{}",
            DIRS[self.dir],
            self.stem,
            self.ext()
        )
    }

    fn sidecar_path(&self) -> String {
        format!("{}.md", self.path())
    }
}

fn write_tree(files: &[File]) -> TempTree {
    let tree = TempTree::empty("assets-props");
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.7\"\nname: k\nscope: test\n",
    );
    tree.write(
        "knowledge/schemas/image.md",
        "---\ntype: Schema\nid: schema-image\n---\n\n````yaml\nline-limit: 0\nroot: /knowledge/media\nlifecycle:\n  - { key: state, states: [draft, published], path: true }\nfrontmatter:\n  type:\n    const: Image\n  resource: {}\nasset-files: ['/knowledge/media/**/*.png']\npreamble:\n  guidance: d\n  blocks:\n    - kind: prose\n      example: 'One line of prose.'\n      max: { hard: 1, when-at-limit: 'Put more about the picture in a document that links it.' }\n````\n",
    );
    tree.write(
        "knowledge/schemas/note.md",
        "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections:\n  - heading: { from: title }\n    level: 1\n    guidance: d\n    blocks:\n      - kind: prose\n        example: 'One line of prose.'\n      - kind: image\n        optional: true\n        repeat: true\n````\n",
    );
    let mut body = String::from("# Album\n\nText.\n\n");
    for f in files {
        tree.write(&f.path()[1..], "bytes\n");
        if f.sidecar {
            // A wrong `resource` either points to another file or is a list.
            // A list is wrong because a sidecar's `resource` must be one path
            // (SOKF_FORMAT_31).
            let resource = if f.resource_ok {
                f.path()
            } else if f.dir == 0 {
                format!("/knowledge/media/{}/other.png", DIRS[f.dir])
            } else {
                format!("[{}]", f.path())
            };
            tree.write(
                &f.sidecar_path()[1..],
                &format!(
                    "---\ntype: Image\nid: {}\nstate: {}\nresource: {resource}\n---\n\nText.\n",
                    f.stem, DIRS[f.state]
                ),
            );
            if f.matched {
                let _ = writeln!(body, "![x][sokf:{}]", f.stem);
            }
        }
    }
    tree.write(
        "knowledge/notes/album.md",
        &format!("---\ntype: Note\nid: album\ntitle: Album\n---\n\n{body}"),
    );
    // Bring every index up to date, so that the only fixes left are the
    // ones under test: write a placeholder index in each directory, then
    // run `--fix` on those paths alone.
    let mut dirs = vec![
        "knowledge",
        "knowledge/media",
        "knowledge/notes",
        "knowledge/schemas",
    ];
    for d in DIRS {
        if files.iter().any(|f| DIRS[f.dir] == d) {
            dirs.push(if d == "draft" {
                "knowledge/media/draft"
            } else {
                "knowledge/media/published"
            });
        }
    }
    let indexes: Vec<String> = dirs.iter().map(|d| format!("{d}/index.md")).collect();
    for index in &indexes {
        tree.write(index, "# placeholder\n");
    }
    let paths: Vec<&str> = indexes.iter().map(String::as_str).collect();
    tree.validate_paths(&paths, true);
    tree
}

// sokf-tests: FORMAT_P-1
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    #[test]
    fn every_asset_binds_to_the_md_beside_it(files in arb_files()) {
        let tree = write_tree(&files);
        let model = tree.model();
        let k = &model.knowledges[0];
        // Every matched file is an asset, and nothing else is.
        let assets: Vec<&str> = k.assets.iter().map(|a| a.path.as_str()).collect();
        let mut expected: Vec<String> = files.iter().filter(|f| f.matched).map(File::path).collect();
        expected.sort();
        prop_assert_eq!(assets, expected.iter().map(String::as_str).collect::<Vec<_>>());
        for asset in &k.assets {
            let beside = RepoPath::parse(&format!("{}.md", asset.path)).unwrap();
            let sidecar = asset.sidecar.map(|i| k.documents[i].path.clone());
            prop_assert_eq!(sidecar.clone(), k.by_path.get(&beside).map(|&i| k.documents[i].path.clone()));
            if let Some(s) = sidecar {
                prop_assert_eq!(model.asset_of(&s).map(|(_, a)| a.path.clone()), Some(asset.path.clone()));
                prop_assert_eq!(model.content_path(&s), asset.path.clone());
            }
        }
        // A `.md` beside an unmatched file is a plain document.
        for f in files.iter().filter(|f| !f.matched && f.sidecar) {
            let p = RepoPath::parse(&f.sidecar_path()).unwrap();
            prop_assert!(model.asset_of(&p).is_none());
            prop_assert_eq!(model.content_path(&p), p.clone());
        }
    }
}

// sokf-tests: FORMAT_P-2
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(64))]

    #[test]
    fn a_bare_label_names_the_asset_and_a_fragment_the_sidecar(files in arb_files()) {
        let tree = write_tree(&files);
        // Cite each sidecar twice: once bare, and once with an empty fragment.
        let mut body = String::from("# Cites\n\nText.\n\n");
        for f in files.iter().filter(|f| f.sidecar) {
            let _ = writeln!(body, "[a][sokf:{0}] [b][sokf:{0}#]", f.stem);
        }
        tree.write(
            "knowledge/notes/cites.md",
            &format!("---\ntype: Note\nid: cites\ntitle: Cites\n---\n\n{body}"),
        );
        tree.validate_paths(&["knowledge/notes/cites.md"], true);
        let text = tree.read("knowledge/notes/cites.md");
        for f in files.iter().filter(|f| f.sidecar) {
            let content = if f.matched { f.path() } else { f.sidecar_path() };
            prop_assert!(text.contains(&format!("[sokf:{}]: {content}\n", f.stem)), "{}", text);
            prop_assert!(text.contains(&format!("[sokf:{}#]: {}\n", f.stem, f.sidecar_path())), "{}", text);
        }
    }
}

// sokf-tests: CLI_P-4
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn the_pair_moves_together_or_not_at_all_and_fix_is_idempotent(files in arb_files()) {
        let tree = write_tree(&files);
        let fixed = tree.validate(true, CacheMode::Off);
        for f in &files {
            let derived = DIRS[f.state];
            let moved = f.sidecar && f.matched && f.state != f.dir;
            let sidecar_landed = format!("knowledge/media/{derived}/{}.png.md", f.stem);
            let asset_landed = format!("knowledge/media/{derived}/{}.png", f.stem);
            if moved {
                prop_assert!(tree.dir().join(&sidecar_landed).exists(), "{}", sidecar_landed);
                prop_assert!(tree.dir().join(&asset_landed).exists(), "{}", asset_landed);
                prop_assert!(!tree.dir().join(&f.path()[1..]).exists());
                // The `resource` holds the asset's new path.
                let wanted = format!("resource: /{asset_landed}\n");
                prop_assert!(tree.read(&sidecar_landed).contains(&wanted));
            } else if f.matched {
                prop_assert!(tree.dir().join(&f.path()[1..]).exists());
            }
        }
        let again = tree.validate(true, CacheMode::Off);
        prop_assert!(again.fixes.is_empty(), "{:?}", again.fixes);
        prop_assert_eq!(again.errors, fixed.errors);
    }
}

// sokf-tests: FORMAT_P-3
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn replacing_every_assets_bytes_changes_no_finding(files in arb_files(), bytes in "[a-z\n]{0,20}") {
        let tree = write_tree(&files);
        let before = tree.validate(false, CacheMode::Off);
        for f in files.iter().filter(|f| f.matched) {
            fs::write(tree.dir().join(&f.path()[1..]), &bytes).unwrap();
        }
        let after = tree.validate(false, CacheMode::Off);
        prop_assert_eq!(after.errors, before.errors);
        prop_assert_eq!(after.warnings, before.warnings);
    }
}
