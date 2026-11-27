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

/// `scaffold` returns the scaffold of a new design, as the guide of the new
/// document gives it.
fn scaffold(f: &Fixture) -> String {
    f.scaffold(&["Design", "--slug", "example"])
}

// Each optional section of a design is a part of its own in the guide of a
// new design: the guide marks it optional and gives it an example of its own
// that holds its heading alone, so a writer writes one and leaves out the
// next. The new design holds none of them.
// sokf-tests: CLI-6_AC-1
#[test]
fn each_optional_section_of_a_design_is_a_region_of_its_own() {
    let f = with_design();
    let (out, err, code) = f.run(&["new", "Design", "--slug", "example", "--json"]);
    assert_eq!(code, 0, "{err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    let optional: Vec<&serde_json::Value> = value["guide"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["optional"] == true && s["level"] == 2)
        .collect();
    assert!(optional.len() > 1, "{out}");
    let document = value["document"].as_str().unwrap();
    for section in optional {
        let example = section["example"].as_str().unwrap_or_default();
        assert!(
            format!("\n{example}").matches("\n## ").count() <= 1,
            "{example}"
        );
        let heading = example.lines().next().unwrap_or_default();
        assert!(
            heading.is_empty() || !document.contains(heading),
            "{document}"
        );
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
