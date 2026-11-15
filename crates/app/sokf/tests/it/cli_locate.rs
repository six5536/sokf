//! End-to-end tests of `sokf locate` on a document with findings.

use common::Fixture;

use crate::common;

// sokf-tests: CLI-8_AC-2
#[test]
fn locate_resolves_a_document_that_fails_its_check() {
    let f = Fixture::new("clean");
    // The frontmatter parses and has an id. The type has no schema, and the
    // document is in no index.
    f.write(
        "knowledge/odd.md",
        "---\ntype: Nope\nid: odd-one\n---\n\n# Odd\n",
    );
    let expected = format!("{}\n", f.path("knowledge/odd.md").display());
    let (stdout, stderr, code) = f.run(&["--no-cache", "locate", "odd-one"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, expected);
    let (report, _, code) = f.run(&["validate"]);
    assert_eq!(code, 1, "{report}");
    assert!(report.contains("/knowledge/odd.md"), "{report}");
    // The check has filled the store, and `locate` reads the same path from it.
    let (stdout, stderr, code) = f.run(&["locate", "odd-one"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, expected);
}
