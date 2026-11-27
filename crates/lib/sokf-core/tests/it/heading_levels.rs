//! A section whose heading is a bare template, such as `${}`, matches any
//! heading text. The checker therefore matches a heading against a section of
//! its own level first, so that such a section never takes the title.

use common::{TempTree, new_options};
use sokf_core::{NewOptions, new};

use crate::common;

/// `OPEN_SCHEMA` has subsections whose headings are any text.
const OPEN_SCHEMA: &str = r#"---
type: Schema
id: schema-open
title: Open schema
description: The subsections have headings of any text.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Open
  id:
    template: "open-${:seq}-${:slug}"
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Topics
    level: 2
    rule-id: topics
    blocks: []
  - heading: { template: '${}', example: Errors }
    level: 3
    repeat: true
    parent: topics
    blocks:
      - kind: prose
        example: Each part returns its errors.
````
"#;

/// `scaffold` writes the schema and returns the body of its scaffold, as the
/// guide of a new document gives it: the example of each section that lies
/// in no other, in order.
fn scaffold(tree: &TempTree) -> String {
    tree.write("knowledge/schemas/open.md", OPEN_SCHEMA);
    let options = NewOptions {
        slug: Some("thing".into()),
        ..new_options(tree.dir(), "Open")
    };
    let guide = new(&tree.root(), &options).unwrap().guide.unwrap();
    let sections: Vec<String> = guide
        .sections
        .into_iter()
        .filter(|s| s.parent.is_none())
        .filter_map(|s| s.example)
        .collect();
    format!("\n{}\n", sections.join("\n\n"))
}

// The title heading takes the document's title, although the template of the
// subsections also matches it.
#[test]
fn a_bare_template_of_another_level_never_takes_the_title() {
    let tree = TempTree::fixture("clean");
    let doc = scaffold(&tree);
    assert!(doc.contains("\n# Thing\n"), "{doc}");
    assert!(doc.contains("\n### Errors\n"), "{doc}");
}
