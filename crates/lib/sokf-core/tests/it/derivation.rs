//! A key, such as a heading key, can derive from a frontmatter value, and the
//! keys under a heading derive from the heading key. The schema declares each
//! derivation beside the key it constrains.

use common::{TempTree, examples, json, lined, new_options, not_missing};
use sokf_core::{CacheMode, NewOptions, new};

use crate::common;

/// `API_SCHEMA` describes parts keyed `CODE-n` under a numbered parent,
/// criteria keyed `CODE-n_AC-m` under each part, and properties keyed
/// `CODE_P-n`. Its examples use the code `LINK` of the frontmatter example,
/// because an example whose key starts with another code contradicts its
/// schema.
const API_SCHEMA: &str = r"---
type: Schema
id: schema-api
description: Parts, criteria, and properties, all keyed by the code.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Api
  code:
    pattern: '^[A-Z]{1,10}$'
    example: LINK
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: 1 Interface
    level: 2
    rule-id: interface
    blocks: []
  - heading:
      template: '${:int}.${:int} ${id:requirement-id} ${}'
      key: id
      key-derives-from: { from: code, separator: '-' }
      example: 1.1 LINK-1 Follow a link
    level: 3
    repeat: true
    parent: interface
    number-from-parent: true
    rule-id: parts
    blocks:
      - kind: bullet-list
        each:
          template: '${id:criterion-id}: ${}'
          key: id
          key-derives-from: { rule: parts, separator: '_' }
          example: 'LINK-1_AC-1: THE API SHALL redirect a known short code.'
  - heading: 2 Properties
    level: 2
    blocks:
      - kind: bullet-list
        each:
          template: '${id:property-id}: ${}'
          key: id
          key-derives-from: { from: code, separator: '_' }
          example: 'LINK_P-1: FOR ANY valid URL, THE API SHALL redirect to it.'
````
";

const GOOD: &str = "---\ntype: Api\nid: links\ntitle: Links\ncode: LINK\n---\n\n# Links\n\n## 1 Interface\n\n### 1.1 LINK-1 Follow a link\n\n- LINK-1_AC-1: THE API SHALL redirect a known short code.\n\n### 1.2 LINK-2 Create a link\n\n- LINK-2_AC-1: THE API SHALL return a new short code.\n\n## 2 Properties\n\n- LINK_P-1: FOR ANY valid URL, THE API SHALL redirect to it.\n";

fn tree(doc: &str) -> TempTree {
    TempTree::clean_with(&[
        ("knowledge/schemas/api.md", API_SCHEMA),
        ("knowledge/links.md", doc),
        (".sokf/formats.yaml", crate::common::requirement_formats()),
    ])
}

/// `errors` returns the errors on the document and its schema, after `--fix`
/// writes the anchors and the index.
fn errors(tree: &TempTree) -> Vec<String> {
    errors_on(tree, &["links.md", "api.md"])
}

/// `errors_on` returns the errors on the files named `names`, after `--fix`
/// writes the anchors and the index.
fn errors_on(tree: &TempTree, names: &[&str]) -> Vec<String> {
    tree.validate(true, CacheMode::Off)
        .errors
        .iter()
        .filter(|f| names.contains(&f.path.file_name()))
        .map(lined)
        .collect()
}

#[test]
fn keys_that_derive_from_the_frontmatter_and_the_heading_pass() {
    let t = tree(GOOD);
    assert!(errors(&t).is_empty(), "{:?}", errors(&t));
}

#[test]
fn a_heading_key_and_a_key_that_do_not_take_the_code_are_reported() {
    let doc = GOOD
        .replace("1.2 LINK-2", "1.2 CORE-2")
        .replace("LINK-2_AC-1", "CORE-2_AC-1")
        .replace("LINK_P-1", "LNK_P-1");
    let got = errors(&tree(&doc));
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(
        got[0].contains(
            "key `CORE-2` derives from `CORE`, which is not `LINK`, the value of the frontmatter `code`"
        ),
        "{got:?}"
    );
    assert!(
        got[1].contains("key `LNK_P-1` derives from `LNK`, which is not `LINK`, the value of the frontmatter `code`"),
        "{got:?}"
    );
}

// The key of a criterion under a heading derives from the heading key, not
// from the code: the heading key is the prefix of the criterion's key.
// Validation checks each key against its own derivation.
#[test]
fn a_key_under_a_heading_derives_from_a_heading_key() {
    let doc = GOOD.replace("LINK-2_AC-1", "LINK-9_AC-1");
    let got = errors(&tree(&doc));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0]
            .contains("key `LINK-9_AC-1` derives from `LINK-9`, which is not a key of set `parts`"),
        "{got:?}"
    );
}

// The scaffold writes every key with the code of the frontmatter example,
// as the guide of a new document shows it while the document's `code` is
// empty.
#[test]
fn the_template_writes_every_key_with_the_code() {
    let t = tree(GOOD);
    let made = new(
        &t.root(),
        &NewOptions {
            cwd: t.dir().join("knowledge"),
            id: Some("more".into()),
            write: true,
            ..new_options(t.dir(), "Api")
        },
    )
    .unwrap();
    let all = examples(&made).join("\n");
    for line in [
        "### 1.1 LINK-1 Follow a link",
        "LINK-1_AC-1: THE API SHALL",
        "LINK_P-1: FOR ANY",
    ] {
        assert!(all.contains(line), "{all}");
    }
    // The new document captures no key, so it reports only what its writer
    // has still to write.
    let report = t.validate(true, CacheMode::Off);
    assert_eq!(
        not_missing(&report, made.path.as_str()),
        Vec::<String>::new(),
        "{}",
        json(&report)
    );
}

// An example whose key starts with a code other than the frontmatter
// example contradicts its schema, and the validator reports the schema at
// each such example. The criterion's example `ABC-1_AC-1` starts with the
// key of the heading's example, `ABC-1`, so only the heading and the
// property are reported.
#[test]
fn an_example_that_does_not_start_with_the_code_is_reported() {
    let schema = API_SCHEMA
        .replace("1.1 LINK-1 Follow", "1.1 ABC-1 Follow")
        .replace("'LINK-1_AC-1:", "'ABC-1_AC-1:")
        .replace("'LINK_P-1:", "'ABC_P-1:");
    let t = TempTree::clean_with(&[
        (".sokf/formats.yaml", crate::common::requirement_formats()),
        ("knowledge/schemas/api.md", &schema),
        ("knowledge/links.md", GOOD),
    ]);
    let got = errors_on(&t, &["api.md"]);
    assert_eq!(got.len(), 2, "{got:?}");
    for key in ["ABC-1", "ABC_P-1"] {
        assert!(
            got.iter().any(|f| f.contains(&format!(
                "the key `{key}` of the example does not start with `LINK`, the value that the scaffold writes for the frontmatter `code`"
            ))),
            "{got:?}"
        );
    }
}

/// `BUG_SCHEMA` keys each level-2 heading `CODE-n`, where the code `CE-BUG`
/// contains the separator `-` of the derivation.
const BUG_SCHEMA: &str = r"---
type: Schema
id: schema-bugset
description: Bugs, each a heading keyed by a code that contains the separator.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Bugset
  code:
    pattern: '^[A-Z]+(?:-[A-Z]+)*$'
    example: CE-BUG
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading:
      pattern: '^(?<id>[A-Z]+(?:-[A-Z]+)*-[0-9]+) .+$'
      key: id
      key-derives-from: { from: code, separator: '-' }
      example: CE-BUG-1 First part
    level: 2
    repeat: true
    blocks: []
````
";

const BUGS: &str = "---\ntype: Bugset\nid: cache-bugs\ntitle: Cache bugs\ncode: CE-BUG\n---\n\n# Cache bugs\n\n## CE-BUG-1 First part\n\n## CE-BUG-2 Second part\n";

fn bug_tree(doc: &str) -> TempTree {
    TempTree::clean_with(&[
        ("knowledge/schemas/bugset.md", BUG_SCHEMA),
        ("knowledge/cache-bugs.md", doc),
        (".sokf/formats.yaml", crate::common::requirement_formats()),
    ])
}

// A key derives from a code that contains the separator: `CE-BUG-1` is the
// code `CE-BUG`, the separator, and `1`. A key that starts with another
// code is still reported.
#[test]
fn a_key_derives_from_a_code_that_contains_the_separator() {
    let names = ["cache-bugs.md", "bugset.md"];
    let got = errors_on(&bug_tree(BUGS), &names);
    assert!(got.is_empty(), "{got:?}");
    let got = errors_on(
        &bug_tree(&BUGS.replace("## CE-BUG-2", "## CE-FIX-2")),
        &names,
    );
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains(
            "key `CE-FIX-2` derives from `CE-FIX`, which is not `CE-BUG`, the value of the frontmatter `code`"
        ),
        "{got:?}"
    );
}

// The scaffold writes the code once in the key of each heading, as the
// guide of a new document shows it, and the new document lacks only what its
// writer writes.
#[test]
fn the_template_keeps_a_code_that_contains_the_separator_once() {
    let t = bug_tree(BUGS);
    let made = new(
        &t.root(),
        &NewOptions {
            cwd: t.dir().join("knowledge"),
            id: Some("more-bugs".into()),
            write: true,
            ..new_options(t.dir(), "Bugset")
        },
    )
    .unwrap();
    let all = examples(&made).join("\n");
    assert!(all.contains("## CE-BUG-1 First part"), "{all}");
    assert!(!all.contains("CE-BUG-BUG"), "{all}");
    let report = t.validate(true, CacheMode::Off);
    assert_eq!(
        not_missing(&report, made.path.as_str()),
        Vec::<String>::new(),
        "{}",
        json(&report)
    );
}

/// `PREFIX_SCHEMA` lists codes, and keys each part `CODE-n` from one of the
/// codes, so that both `CE` and `CE-BUG` are allowed prefixes.
const PREFIX_SCHEMA: &str = r"---
type: Schema
id: schema-codeset
description: Codes, and parts keyed by one of the codes.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Codeset
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: bullet-list
        rule-id: codes
        each:
          pattern: '^Code (?<c>[A-Z]+(?:-[A-Z]+)*)$'
          key: c
          example: Code CE
  - heading: Parts
    level: 2
    blocks:
      - kind: bullet-list
        each:
          pattern: '^(?<id>[A-Z]+(?:-[A-Z]+)*-[0-9]+) .+$'
          key: id
          key-derives-from: { rule: codes, separator: '-' }
          example: CE-1 First part
````
";

// When both `CE` and `CE-BUG` are allowed prefixes, `CE-BUG-1` derives
// from the longer, and `CE-2` from `CE`.
#[test]
fn a_key_that_two_prefixes_start_derives_from_the_longer() {
    let doc = "---\ntype: Codeset\nid: codes\ntitle: Codes\n---\n\n# Codes\n\n- Code CE\n- Code CE-BUG\n\n## Parts\n\n- CE-BUG-1 First part\n- CE-2 Second part\n";
    let t = TempTree::clean_with(&[
        (".sokf/formats.yaml", crate::common::requirement_formats()),
        ("knowledge/schemas/codeset.md", PREFIX_SCHEMA),
        ("knowledge/codes.md", doc),
    ]);
    let got = errors_on(&t, &["codes.md", "codeset.md"]);
    assert!(got.is_empty(), "{got:?}");
}
