//! These tests run the Design schema of the fixtures through `sokf new` and
//! `sokf validate`, on the fixture `new`, to check what the engine derives
//! from it and how the engine reads a design.

use std::fmt::Write as _;

use common::Fixture;

use crate::common;

/// `DESIGN_SCHEMA` is a Design schema with optional sections, one of which
/// holds a worked example after a concept's prose.
const DESIGN_SCHEMA: &str =
    include_str!("../../../../lib/sokf-core/tests/fixtures/schemas/design.md");

/// `with_design` returns the fixture `new` with the Design schema of the
/// fixtures.
fn with_design() -> Fixture {
    let f = Fixture::new("new");
    f.write("knowledge/schemas/design.md", DESIGN_SCHEMA);
    f
}

/// `scaffold` returns the scaffold of a new design.
fn scaffold(f: &Fixture) -> String {
    let (stdout, stderr, code) = f.run(&["new", "Design", "--slug", "example"]);
    assert_eq!(code, 0, "{stderr}");
    stdout
}

// Each optional section of a design's scaffold is a region of its own, so a
// writer keeps one and deletes the next.
// sokf-tests: CLI-6_AC-1
#[test]
fn each_optional_section_of_a_design_is_a_region_of_its_own() {
    let text = scaffold(&with_design());
    let regions: Vec<&str> = text
        .split("<!-- sokf:optional -->")
        .skip(1)
        .map(|r| r.split("<!-- /sokf:optional -->").next().unwrap())
        .collect();
    assert!(regions.len() > 1, "{text}");
    for region in regions {
        assert!(region.matches("\n## ").count() <= 1, "{region}");
    }
}

// A concept's prose followed directly by a worked example, a sentence that
// ends with a colon and a code block, reads as the prose and the example.
// sokf-tests: CLI-4_AC-1
#[test]
fn a_worked_example_after_a_concept_s_prose_reads_as_an_example() {
    let f = with_design();
    let text: String = scaffold(&f)
        .lines()
        .filter(|l| !l.contains("sokf:guidance"))
        .filter(|l| {
            !matches!(
                *l,
                "A failed write leaves the old row in place:"
                    | "- A write that the disk refuses fails, and the cache keeps the old row."
                    | "- A write of a row that no check produced fails, and the cache stores nothing."
            )
        })
        .fold(String::new(), |mut out, l| {
            let _ = writeln!(out, "{l}");
            out
        });
    assert!(
        text.contains("checks that the write fails with this error:"),
        "{text}"
    );
    f.write("knowledge/designs/design-example.md", &text);
    let (stdout, _, _) = f.run(&["validate"]);
    // No finding names the design, so its schema loaded and the engine read
    // the prose and the example without a TLMD_69.
    let mine: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("design-example.md"))
        .collect();
    assert!(mine.is_empty(), "{mine:?}");
}
