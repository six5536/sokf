//! What sokf hands same-same and what it reads of a generated block beside
//! same-same's answer: the path under which a pair reads, and the labels of
//! an index as sokf writes them.

use sokf_core::{CacheMode, Report};

use crate::common::TempTree;

/// The overview, which includes the CSS and the editor settings.
const OVERVIEW: &str = "---\ntype: Overview\nid: overview\ntitle: Overview\ndescription: Reads the requirement.\nlinks:\n  - rel: references\n    to: req-cli\n---\n\n# Overview\n\nThe CLI requirement is [CLI-1][sokf:#cli-1] in [the criteria][sokf:req-cli#acceptance-criteria], governed by [its schema][sokf:schema-requirement].\n\n<!-- sokf:include req-cli -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /src/site.css -->\n<!-- /sokf:include -->\n\n<!-- sokf:include /.vscode/settings.json -->\n<!-- /sokf:include -->\n";

/// The editor's settings, JSON with comments by their folder.
const SETTINGS: &str = "{\n  // the width of a line\n  \"editor.rulers\": [80],\n}\n";

/// `tree` returns the clean fixture with the overview and its sources,
/// after one `--fix` that fills the blocks.
fn tree() -> TempTree {
    let tree = TempTree::clean_with(&[
        ("knowledge/overview.md", OVERVIEW),
        ("src/site.css", "a { color: red; }\n"),
        (".vscode/settings.json", SETTINGS),
    ]);
    let report = tree.validate(true, CacheMode::On);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let filled = tree.read("knowledge/overview.md");
    let again = tree.validate(true, CacheMode::On);
    assert!(again.errors.is_empty(), "{:?}", again.errors);
    assert_eq!(
        tree.read("knowledge/overview.md"),
        filled,
        "a second `--fix` writes nothing"
    );
    tree
}

/// `relay` replaces `from` with `to` in the file at `rel`.
fn relay(tree: &TempTree, rel: &str, from: &str, to: &str) {
    let text = tree.read(rel);
    assert!(text.contains(from), "{rel} holds no {from:?}");
    tree.write(rel, &text.replacen(from, to, 1));
}

/// `errors` returns the message and the key of each error of `report`.
fn errors(report: &Report) -> Vec<String> {
    report
        .errors
        .iter()
        .map(|f| format!("{} ({})", f.message, f.criterion))
        .collect()
}

// An include of `.vscode/settings.json` reads as JSON with comments by its
// folder, so a copy without the trailing comma means the same.
// sokf-tests: CLI-4_AC-113
#[test]
fn an_include_reads_under_its_path_from_the_root() {
    let tree = tree();
    relay(&tree, "knowledge/overview.md", "[80],\n}", "[80]\n}");
    let report = tree.validate(false, CacheMode::Off);
    assert!(report.errors.is_empty(), "{:?}", errors(&report));
}

// An index whose reference label changed only in its case, or that holds an
// inline link in place of a reference, or a definition whose label changed
// its case, is stale; `--fix` writes it as sokf does, and a second `--fix`
// writes nothing.
// sokf-tests: CLI-4_AC-117
#[test]
fn an_index_holds_its_labels_as_sokf_writes_them() {
    let tree = tree();
    let index = tree.read("knowledge/index.md");
    for (from, to) in [
        ("[Overview][sokf:overview]", "[Overview][SOKF:OVERVIEW]"),
        (
            "[Overview][sokf:overview]",
            "[Overview](/knowledge/overview.md)",
        ),
        (
            "[sokf:overview]: /knowledge/overview.md",
            "[SOKF:OVERVIEW]: /knowledge/overview.md",
        ),
    ] {
        relay(&tree, "knowledge/index.md", from, to);
        let report = tree.validate(false, CacheMode::Off);
        assert!(
            report
                .errors
                .iter()
                .any(|f| f.criterion == "SOKF_FORMAT_97"),
            "{to}: {:?}",
            errors(&report)
        );
        let fixed = tree.validate(true, CacheMode::On);
        assert!(fixed.errors.is_empty(), "{to}: {:?}", errors(&fixed));
        assert_eq!(tree.read("knowledge/index.md"), index, "{to}");
        let again = tree.validate(true, CacheMode::On);
        assert!(again.errors.is_empty(), "{to}: {:?}", errors(&again));
        assert_eq!(tree.read("knowledge/index.md"), index, "{to}");
    }
    // An index that a formatter re-laid stays current.
    relay(
        &tree,
        "knowledge/index.md",
        "- [Overview][sokf:overview] - Reads the requirement.",
        "- [Overview][sokf:overview] -\n  Reads the requirement.",
    );
    let report = tree.validate(false, CacheMode::Off);
    assert!(report.errors.is_empty(), "{:?}", errors(&report));
}
