//! These property tests check that `init` creates a store that validates,
//! changes the config as little as it can, and makes the directory a root in
//! effect.

use std::{collections::BTreeMap, fmt::Write as _, path::PathBuf};

use common::TempTree;
use proptest::prelude::*;
use sokf_core::{
    CacheMode, ConfigText, InitOptions, RepoPath, init, load_config,
    scaffold::{ConfigChange, register},
};

use crate::common;

/// `DIRS` holds the directories that a generated `init` call can create a
/// store in.
const DIRS: [&str; 4] = ["knowledge", "docs/kb", "/knowledge/inner", "a/b/c"];

fn arb_name() -> impl Strategy<Value = Option<String>> {
    prop::option::of(
        prop::sample::select(vec!["knowledge", "project-kb", "kb2", "a-b-c"])
            .prop_map(str::to_string),
    )
}

fn arb_description() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[a-z][a-z ,.:#\"]{0,20}")
}

/// `arb_config_text` generates the text of a `config.toml`. The text might
/// hold a comment, a `[repository]` table with an `exclude` glob, and a
/// blank line. Its `[knowledge]` table is absent, has no `roots`, or lists
/// zero or more roots.
fn arb_config_text() -> impl Strategy<Value = String> {
    (
        any::<bool>(),
        0u8..4,
        any::<bool>(),
        any::<bool>(),
        prop::collection::vec(
            prop::sample::select(vec!["/knowledge", "/docs", "/x/y"]),
            0..3,
        ),
    )
        .prop_map(|(comment, roots_form, repository, blank, roots)| {
            let mut text = String::new();
            if comment {
                text.push_str("# The stores.\n");
            }
            if repository {
                text.push_str("[repository]\nexclude = [\"/vendor/**\"] # pruned\n");
                if blank {
                    text.push('\n');
                }
            }
            match roots_form {
                0 => {}
                1 => text.push_str("[knowledge]\n"),
                2 => text.push_str("[knowledge]\nroots = []\n"),
                _ => {
                    let listed: Vec<String> = roots.iter().map(|r| format!("\"{r}\"")).collect();
                    let _ = write!(text, "[knowledge]\nroots = [{}]\n", listed.join(", "));
                }
            }
            text
        })
}

fn roots_of(table: &toml::Table) -> Vec<String> {
    table
        .get("knowledge")
        .and_then(|k| k.get("roots"))
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// sokf-tests: CLI_P-21
// sokf-tests: CLI-2_AC-9
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(32))]

    #[test]
    fn a_fresh_store_validates_and_is_a_root_in_effect(which in 0usize..4, name in arb_name(), description in arb_description()) {
        let tree = TempTree::empty("init-prop");
        let dir = DIRS[which];
        let out = init(
            &tree.root(),
            &InitOptions {
                dir: Some(PathBuf::from(dir)),
                cwd: tree.dir().to_path_buf(),
                name: name.clone(),
                scope: Some("test".into()),
                description: description.clone(),
            },
        )
        .unwrap();
        let expected_root = RepoPath::parse(&format!("/{}", dir.trim_start_matches('/'))).unwrap();
        prop_assert_eq!(&out.root, &expected_root);
        // CLI-2_AC-9: the directory is a root in effect.
        let config = load_config(&tree.root()).unwrap();
        prop_assert!(config.roots.contains(&expected_root), "{:?}", config.roots);
        // CLI_P-21: validation reports no finding and has nothing to fix.
        let report = tree.validate(false, CacheMode::Off);
        prop_assert!(report.errors.is_empty(), "{:?}", report.errors);
        prop_assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let fixed = tree.validate(true, CacheMode::Off);
        prop_assert!(fixed.fixes.is_empty(), "{:?}", fixed.fixes);
    }
}

// sokf-tests: CLI_P-22
proptest! {
    #![proptest_config(crate::common::proptest_cases::cases(256))]

    #[test]
    fn config_edit_is_minimal(text in arb_config_text(), dir in prop::sample::select(vec!["/docs/kb", "/knowledge", "/x/y"])) {
        let before: toml::Table = text.parse().unwrap();
        let roots: Vec<RepoPath> = roots_of(&before).iter().map(|r| RepoPath::parse(r).unwrap()).collect();
        let dir = RepoPath::parse(dir).unwrap();
        let config = ConfigText { roots: roots.clone(), registered: vec![], declined: BTreeMap::default(), hook_commands: BTreeMap::default(), text: Some(text.clone()) };
        let change = register(&config, &[], &dir).unwrap();
        if roots.contains(&dir) {
            prop_assert_eq!(change, ConfigChange::Unchanged);
            return Ok(());
        }
        let ConfigChange::Update(after) = change else {
            prop_assert!(false, "expected an update, got {change:?}");
            return Ok(());
        };
        let parsed: toml::Table = after.parse().unwrap();
        let mut want = roots_of(&before);
        want.push(dir.as_str().to_string());
        prop_assert_eq!(roots_of(&parsed), want);
        // Everything but `knowledge.roots` is as it was.
        let mut before_rest = before.clone();
        let mut after_rest = parsed.clone();
        for t in [&mut before_rest, &mut after_rest] {
            if let Some(k) = t.get_mut("knowledge").and_then(|k| k.as_table_mut()) {
                k.remove("roots");
            }
            if t.get("knowledge").and_then(|k| k.as_table()).is_some_and(toml::map::Map::is_empty) {
                t.remove("knowledge");
            }
        }
        prop_assert_eq!(before_rest, after_rest);
        for line in text.lines().filter(|l| !l.contains("roots")) {
            prop_assert!(after.contains(line), "the edit lost line {line:?}. The new text is {after:?}");
        }
    }
}
