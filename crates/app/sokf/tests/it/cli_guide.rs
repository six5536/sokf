//! These tests run `sokf guide` from end to end on the fixture `guide`, and
//! check the guide's text against a snapshot, its JSON, each form of an
//! argument, `--state`, and the exit code of every refusal.

use common::Fixture;

use crate::{common, json_schemas::valid};

/// `guide` runs `sokf guide` with `args` on `f`, and returns its stdout,
/// after it checks that the command succeeded and wrote nothing to stderr.
fn guide(f: &Fixture, args: &[&str]) -> String {
    let args: Vec<&str> = ["guide"].iter().chain(args).copied().collect();
    let (stdout, stderr, code) = f.run(&args);
    assert_eq!((code, stderr.as_str()), (0, ""), "{args:?}: {stdout}");
    stdout
}

// The guide of an open task holds the schema's guidance and ban, then the
// guidance of each key and link of the open state, then each section of the
// open state under its heading, in the schema's order, and leaves out the
// sections and the key of the done state. The guide of a note holds its
// preamble, and leaves out the block that a hard limit of zero bans. The
// cache changes nothing that the guide prints.
// sokf-tests: CLI-17_AC-5
// sokf-tests: CLI-17_AC-6
// sokf-tests: CLI-17_AC-8
// sokf-tests: CLI-17_AC-29
// sokf-tests: CLI-17_AC-9
// sokf-tests: CLI-17_AC-10
// sokf-tests: CLI-17_AC-14
// sokf-tests: CLI-17_AC-16
// sokf-tests: CLI-17_AC-22
// sokf-tests: CLI-17_AC-33
// sokf-tests: CLI-17_AC-41
#[test]
fn guide_prints_the_guidance_of_the_states_in_the_schemas_order() {
    let f = Fixture::new("guide");
    let text = guide(&f, &["task-001-descale", "note-kettle"]);
    insta::assert_snapshot!("guide_of_an_open_task_and_a_note", text);
    assert!(
        !text.contains("Outcome") && !text.contains("A note holds no code"),
        "{text}"
    );
    assert_eq!(
        guide(&f, &["--no-cache", "task-001-descale", "note-kettle"]),
        text
    );
}

// `--state` replaces one dimension of the document's own states: the open
// task shows the sections of the done state, each that it lacks marked as
// optional or required. A done task holds its Outcome, which has no mark.
// sokf-tests: CLI-17_AC-11
// sokf-tests: CLI-17_AC-29
// sokf-tests: CLI-17_AC-14
// sokf-tests: CLI-17_AC-15
#[test]
fn state_replaces_one_dimension_of_the_documents_own() {
    let f = Fixture::new("guide");
    let text = guide(&f, &["task-001-descale", "--state", "state=done"]);
    assert!(
        text.starts_with("/knowledge/tasks/open/task-001-descale.md (Task, state=done)\n"),
        "{text}"
    );
    assert!(text.contains("\n## Outcome (absent, required)\n"), "{text}");
    assert!(
        text.contains("\n## Follow-up (absent, optional)\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "\n  bullet-list: At most 3 items. Never leave out or merge content to stay within the count. When at the limit: File a task for the rest.\n"
        ),
        "{text}"
    );
    // A block's own limit comes first, then the limit of each of its rules,
    // which bounds only the units that the rule takes.
    f.write(
        "knowledge/schemas/recipe.md",
        "---\ntype: Schema\nid: schema-recipe\ntitle: Recipe Schema\ndescription: A recipe.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Recipe\nsections:\n  - heading: Method\n    level: 2\n    blocks:\n      - kind: numbered-list\n        max: { soft: 5, when-at-limit: 'Split the recipe.' }\n        rules:\n          - pattern: '^Wait'\n            rule-id: wait\n            min: 0\n            max: { hard: 1, when-at-limit: 'Join the waits.' }\n            example: Wait an hour.\n          - pattern: '^[A-Z]'\n            example: Boil the water.\n````\n",
    );
    f.write(
        "knowledge/recipe.md",
        "---\ntype: Recipe\nid: recipe\ntitle: Recipe\n---\n\n## Method\n\n1. Boil the water.\n",
    );
    let recipe = guide(&f, &["recipe"]);
    assert!(
        recipe.contains("\n  numbered-list: Usually up to 5 items. Never leave out or merge content to stay within the count. When at the limit: Split the recipe. At most 1 `wait` item. Never leave out or merge content to stay within the count. When at the limit: Join the waits.\n"),
        "{recipe}"
    );
    assert!(
        !text.contains("Steps") && !text.contains("owner:"),
        "{text}"
    );
    let done = guide(&f, &["task-002-buy-a-kettle"]);
    assert!(done.contains("\n## Outcome\n"), "{done}");
    assert!(done.contains("\n# <title>\n"), "{done}");
    // A heading renamed in the schema still holds its section.
    f.write(
        "knowledge/tasks/done/task-002-buy-a-kettle.md",
        &std::fs::read_to_string(f.path("knowledge/tasks/done/task-002-buy-a-kettle.md"))
            .unwrap()
            .replace("## Outcome", "## Result"),
    );
    let schema = f.path("knowledge/schemas/task.md");
    let text = std::fs::read_to_string(&schema).unwrap().replace(
        "  - heading: Outcome\n",
        "  - heading: Outcome\n    renamed-from: [Result]\n",
    );
    f.write("knowledge/schemas/task.md", &text);
    let renamed = guide(&f, &["task-002-buy-a-kettle"]);
    assert!(renamed.contains("\n## Outcome\n"), "{renamed}");
}

// An id in each form that a reference takes, a key that the document
// captures, and a path from the current directory or from the repository
// root each name the same document.
// sokf-tests: CLI-17_AC-1
#[test]
fn every_form_of_an_id_and_a_path_names_the_document() {
    let f = Fixture::new("guide");
    let text = guide(&f, &["note-kettle"]);
    for arg in [
        "sokf:note-kettle",
        "guide-fixture:note-kettle",
        "test/guide-fixture:note-kettle",
        "note-kettle#details",
        "#lid",
        "/knowledge/notes/note-kettle.md",
    ] {
        assert_eq!(guide(&f, &[arg]), text, "{arg}");
    }
    let (stdout, stderr, code) = f.run_here(&["guide", "knowledge/notes/note-kettle.md"]);
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        (text.as_str(), "", 0)
    );
}

// A schema's guide comes from the grammar's own schema of a schema.
// sokf-tests: CLI-17_AC-21
#[test]
fn a_schema_takes_the_guide_of_the_grammars_schema() {
    let f = Fixture::new("guide");
    let text = guide(&f, &["schema-note"]);
    assert!(
        text.starts_with("/knowledge/schemas/note.md (Schema)\n"),
        "{text}"
    );
    let (stdout, _, code) = f.run(&[
        "guide",
        "--json",
        "--query",
        "documents[0].schema",
        "schema-note",
    ]);
    assert_eq!((stdout.as_str(), code), ("null\n", 0));
}

// The JSON passes its schema and holds each part with its limits and
// whether the document holds each section, and `--query` selects from it.
// sokf-tests: CLI-17_AC-17
// sokf-tests: CLI-17_AC-18
// sokf-tests: CLI-17_AC-33
#[test]
fn the_json_matches_its_schema_and_the_query_selects_from_it() {
    let f = Fixture::new("guide");
    let stdout = guide(&f, &["--json", "task-001-descale", "note-kettle"]);
    let value = valid("guide-report.schema.json", &stdout);
    let task = &value["documents"][0];
    assert_eq!(task["path"], "/knowledge/tasks/open/task-001-descale.md");
    assert_eq!(task["schema"], "/knowledge/schemas/task.md");
    assert_eq!(task["states"], serde_json::json!({ "state": "open" }));
    assert_eq!(task["links"][0]["rel"], "depends-on");
    let limit = &task["links"][0]["limit"];
    assert_eq!(
        (
            limit["kind"].as_str(),
            limit["count"].as_u64(),
            limit["when_at_limit"].as_str()
        ),
        (
            Some("hard"),
            Some(2),
            Some("Merge the tasks that this one waits for.")
        )
    );
    let held: Vec<(&str, bool)> = task["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["heading"].as_str().unwrap(), s["held"].as_bool().unwrap()))
        .collect();
    assert_eq!(held, [("<title>", true), ("Steps", true), ("Notes", false)]);
    let limit = &task["sections"][0]["blocks"][0]["limits"][0];
    assert_eq!(
        (limit["kind"].as_str(), limit["count"].as_u64()),
        (Some("soft"), Some(1))
    );
    assert!(value["documents"][1]["preamble"]["blocks"].is_array());
    let (stdout, stderr, code) = f.run(&[
        "guide",
        "task-001-descale",
        "--json",
        "--query",
        "documents[0].sections[].heading",
    ]);
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        ("[\"<title>\",\"Steps\",\"Notes\"]\n", "", 0)
    );
    let (stdout, _, code) = f.run(&["guide", "note-kettle", "--json", "--query", "nothing"]);
    assert_eq!((stdout.as_str(), code), ("null\n", 0));
}

// A `links` rule that a hard limit of 0 bans is left out of the guide, as
// the scaffold leaves out its entries.
// sokf-tests: CLI-17_AC-34
#[test]
fn a_links_rule_that_a_ban_leaves_out_has_no_guide() {
    let f = Fixture::new("guide");
    let schema = std::fs::read_to_string(f.path("knowledge/schemas/task.md")).unwrap();
    f.write(
        "knowledge/schemas/task.md",
        &schema.replace("max: { hard: 2,", "max: { hard: 0,"),
    );
    let text = guide(&f, &["task-001-descale"]);
    assert!(
        text.contains("  state: ") && !text.contains("links (depends-on)"),
        "{text}"
    );
}

/// `refused` runs `sokf guide` with each argument list of `cases` on `f`, and
/// checks that it exits with code 2, prints nothing to stdout, and says the
/// text of its case on stderr.
fn refused(f: &Fixture, cases: &[(&[&str], &str)]) {
    for (args, needle) in cases {
        let args: Vec<&str> = ["guide"].iter().chain(*args).copied().collect();
        let (stdout, stderr, code) = f.run(&args);
        assert_eq!((stdout.as_str(), code), ("", 2), "{args:?}: {stderr}");
        assert!(stderr.contains(needle), "{args:?}: {stderr}");
    }
}

// Each argument that names no document, or a document without a schema
// that passes the grammar, is refused with code 2, and the error names the
// argument or the document.
// sokf-tests: CLI-17_AC-2
// sokf-tests: CLI-17_AC-3
// sokf-tests: CLI-17_AC-4
// sokf-tests: CLI-17_AC-23
#[test]
fn every_argument_that_names_no_guide_exits_2_and_prints_nothing() {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/orphan.md",
        "---\ntype: Orphan\nid: orphan\ntitle: Orphan\n---\n\n# Orphan\n",
    );
    f.write(
        "knowledge/untyped.md",
        "---\nid: untyped\ntitle: Untyped\n---\n\n# Untyped\n",
    );
    f.write(
        "knowledge/tasks/open/task-009-twin.md",
        "---\ntype: Task\nid: note-kettle\ntitle: Twin\nstate: open\n---\n\n# Twin\n",
    );
    f.write("README.md", "# A file outside every store\n");
    f.write(
        "knowledge/schemas/bare.md",
        "---\ntype: Schema\nid: schema-bare\ntitle: Bare Schema\ndescription: A schema that breaks the grammar.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Bare\nsections:\n  - heading: Body\n    level: 2\n    blocks:\n      - kind: prose\n        max: 1\n        example: Text.\n````\n",
    );
    f.write(
        "knowledge/bare.md",
        "---\ntype: Bare\nid: bare\ntitle: Bare\n---\n\n## Body\n\nText.\n",
    );
    refused(
        &f,
        &[
            (&["nothing"], "`nothing` is neither an id nor a path"),
            (&["#nothing"], "`#nothing` is neither an id nor a path"),
            (
                &["note-kettle"],
                "`note-kettle` identifies more than one document",
            ),
            (
                &["/knowledge/tasks"],
                "`/knowledge/tasks` names /knowledge/tasks, which is no document",
            ),
            (
                &["/README.md"],
                "`/README.md` names /README.md, which is no document",
            ),
            (&["/knowledge/manifest.sokf.yaml"], "which is no document"),
            (
                &["orphan"],
                "/knowledge/orphan.md: `Orphan` has no schema in /knowledge",
            ),
            (&["untyped"], "/knowledge/untyped.md has no `type`"),
            (
                &["bare"],
                "/knowledge/bare.md: the schema of `Bare` does not pass the grammar",
            ),
            (&["task-001-descale", "nothing"], "`nothing`"),
            (&[], "<ID|PATH>"),
        ],
    );
    // A word that is both an id and a path is refused, with both readings.
    f.write("task-001-descale", "");
    let (stdout, stderr, code) = f.run_here(&["guide", "task-001-descale"]);
    assert_eq!((stdout.as_str(), code), ("", 2), "{stderr}");
    assert!(
        stderr.contains("`task-001-descale` is both an id (/knowledge/tasks/open/task-001-descale.md) and a path (/task-001-descale)"),
        "{stderr}"
    );
}

// Each `--state` that the lifecycle doesn't declare, a dimension given
// twice, and a `--query` without `--json` or that doesn't parse are refused
// with code 2.
// sokf-tests: CLI-17_AC-12
// sokf-tests: CLI-17_AC-13
// sokf-tests: CLI-17_AC-24
// sokf-tests: CLI-17_AC-19
// sokf-tests: CLI-17_AC-20
// sokf-tests: CLI-17_AC-23
#[test]
fn every_option_that_fails_exits_2_and_prints_nothing() {
    let f = Fixture::new("guide");
    refused(
        &f,
        &[
            (
                &["task-001-descale", "--state", "kind=bug"],
                "/knowledge/tasks/open/task-001-descale.md: `--state kind=bug`: the lifecycle declares no `kind`",
            ),
            (
                &["task-001-descale", "--state", "state=closed"],
                "`closed` is not a state of `state`",
            ),
            (
                &[
                    "task-001-descale",
                    "--state",
                    "state=open",
                    "--state",
                    "state=done",
                ],
                "`--state state` is given twice",
            ),
            (
                &["note-kettle", "task-001-descale", "--state", "state=done"],
                "/knowledge/notes/note-kettle.md: `--state state=done`: the lifecycle declares no `state`",
            ),
            (&["task-001-descale", "--query", "documents"], "--json"),
            (
                &["task-001-descale", "--json", "--query", "documents["],
                "query",
            ),
            (&["task-001-descale", "--state", "state"], "KEY=STATE"),
        ],
    );
    // A document's own state that the lifecycle doesn't declare, or a state
    // that the frontmatter doesn't set, is refused unless `--state` gives it.
    f.write(
        "knowledge/tasks/open/task-001-descale.md",
        "---\ntype: Task\nid: task-001-descale\ntitle: Descale the kettle\nstate: opn\n---\n\n# Descale the kettle\n\nText.\n",
    );
    f.write(
        "knowledge/tasks/open/task-003-stateless.md",
        "---\ntype: Task\nid: task-003-stateless\ntitle: Stateless\n---\n\n# Stateless\n\nText.\n",
    );
    refused(
        &f,
        &[
            (
                &["task-001-descale"],
                "/knowledge/tasks/open/task-001-descale.md: `state: opn` is not a state of `state`. The states are open, done, so give `--state state=STATE`",
            ),
            (
                &["task-003-stateless"],
                "/knowledge/tasks/open/task-003-stateless.md: the frontmatter sets no `state`, so give `--state state=STATE`",
            ),
        ],
    );
    let text = guide(&f, &["task-001-descale", "--state", "state=open"]);
    assert!(text.contains("\n## Steps (absent, required)\n"), "{text}");
}

// A heading holds only its own section: a heading inside a parent counts
// only under a heading of that parent, and a heading whose text a section
// gives exactly counts toward no section of a template.
// sokf-tests: CLI-17_AC-15
// sokf-tests: CLI-17_AC-25
// sokf-tests: CLI-17_AC-26
// sokf-tests: CLI-17_AC-27
// sokf-tests: CLI-17_AC-28
#[test]
fn a_heading_holds_only_its_own_section() {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/plan.md",
        "---\ntype: Schema\nid: schema-plan\ntitle: Plan Schema\ndescription: A plan and its review.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Plan\nsections:\n  - heading: Plan\n    level: 2\n    rule-id: plan\n    blocks: []\n  - heading: Steps\n    level: 3\n    parent: plan\n    optional: true\n    blocks: []\n  - heading: Review\n    level: 2\n    rule-id: review\n    optional: true\n    blocks: []\n  - heading: Steps\n    level: 3\n    parent: review\n    optional: true\n    blocks: []\n  - heading: { template: '${}', example: Appendix }\n    level: 2\n    optional: true\n    blocks: []\n````\n",
    );
    f.write(
        "knowledge/plan.md",
        "---\ntype: Plan\nid: plan\ntitle: Plan\n---\n\n## Plan\n\n### Steps\n",
    );
    let (stdout, stderr, code) = f.run(&[
        "guide",
        "plan",
        "--json",
        "--query",
        "documents[0].sections[].[heading, parent, held]",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "[[\"Plan\",null,true],[\"Steps\",\"plan\",true],[\"Review\",null,false],[\"Steps\",\"review\",false],[\"${}\",null,false]]\n"
    );
    // Two sections whose rules share a line of the schema keep apart.
    f.write(
        "knowledge/schemas/memo.md",
        "---\ntype: Schema\nid: schema-memo\ntitle: Memo Schema\ndescription: A memo.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Memo\nsections: [{heading: Alpha, level: 2, blocks: []}, {heading: Beta, level: 2, optional: true, blocks: []}]\n````\n",
    );
    f.write(
        "knowledge/memo.md",
        "---\ntype: Memo\nid: memo\ntitle: Memo\n---\n\n## Alpha\n",
    );
    let text = guide(&f, &["memo"]);
    assert!(
        text.ends_with("\n## Alpha\n\n## Beta (absent, optional)\n"),
        "{text}"
    );
    // A parent two levels up holds its section, a `renamed-from` text gives
    // way to a section of that exact text, and a required section inside an
    // absent parent is required only under it.
    f.write(
        "knowledge/schemas/report.md",
        "---\ntype: Schema\nid: schema-report\ntitle: Report Schema\ndescription: A report.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Report\nsections:\n  - heading: Report\n    level: 1\n    rule-id: report\n    blocks: []\n  - heading: Context\n    level: 2\n    optional: true\n    blocks: []\n  - heading: Steps\n    level: 3\n    parent: report\n    optional: true\n    blocks: []\n  - heading: Overview\n    level: 2\n    renamed-from: [Summary]\n    blocks: []\n  - heading: Summary\n    level: 2\n    optional: true\n    blocks: []\n  - heading: Review\n    level: 2\n    rule-id: review\n    optional: true\n    blocks: []\n  - heading: Findings\n    level: 3\n    parent: review\n    blocks: []\n````\n",
    );
    f.write(
        "knowledge/report.md",
        "---\ntype: Report\nid: report\ntitle: Report\n---\n\n# Report\n\n## Context\n\n### Steps\n\n## Summary\n",
    );
    let text = guide(&f, &["report"]);
    assert!(
        text.ends_with(
            "\n# Report\n\n## Context\n\n### Steps\n\n## Overview (absent, required)\n\n## Summary\n\n## Review (absent, optional)\n  Example:\n    ## Review\n\n    ### Findings\n\n### Findings (absent, required under Review)\n"
        ),
        "{text}"
    );
}

// A heading that a formatter spells another way holds its section, as the
// check reads it: `_main_` holds `The *main* part`, and a heading bound to a
// title with emphasis holds its section in the other spelling. A heading
// that stands for a bound heading and reads otherwise holds nothing, though
// the check gives it the section to repair.
// sokf-tests: CLI-17_AC-15
#[test]
fn a_respelt_heading_holds_its_section() {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/part.md",
        "---\ntype: Schema\nid: schema-part\ntitle: Part Schema\ndescription: A part.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Part\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n  - heading: The *main* part\n    level: 2\n    blocks: []\n  - heading: The __other__ part\n    level: 2\n    optional: true\n    blocks: []\n````\n",
    );
    f.write(
        "knowledge/part.md",
        "---\ntype: Part\nid: part\ntitle: Use *the* tool\n---\n\n# Use _the_ tool\n\n## The _main_ part\n\n## The **other** part\n",
    );
    f.run(&["validate", "--fix"]);
    let (stdout, stderr, code) = f.run(&["validate", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), stderr.as_str()),
        (0, "0 errors, 0 warnings\n", "")
    );
    let held = |f: &Fixture| {
        let (stdout, stderr, code) = f.run(&[
            "guide",
            "part",
            "--json",
            "--query",
            "documents[0].sections[].held",
        ]);
        assert_eq!(code, 0, "{stderr}");
        stdout
    };
    assert_eq!(held(&f), "[true,true,true]\n");
    f.write(
        "knowledge/part.md",
        "---\ntype: Part\nid: part\ntitle: Use *the* tool\n---\n\n# Use the tool\n\n## The _main_ part\n",
    );
    assert_eq!(held(&f), "[false,true,false]\n");
    // A heading that the shapes of two sections accept holds only the one
    // that the check gives it.
    f.write(
        "knowledge/schemas/part.md",
        "---\ntype: Schema\nid: schema-part\ntitle: Part Schema\ndescription: A part.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Part\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n  - heading: { template: 'Step ${:int}', example: Step 1 }\n    level: 2\n    optional: true\n    blocks: []\n  - heading: { template: '${}', example: Notes }\n    level: 2\n    optional: true\n    blocks: []\n````\n",
    );
    f.write(
        "knowledge/part.md",
        "---\ntype: Part\nid: part\ntitle: Use *the* tool\n---\n\n# Use _the_ tool\n\n## Step 1\n",
    );
    assert_eq!(held(&f), "[true,true,false]\n");
}

// Each example carries the document's own frontmatter values, so the
// example of a heading bound to the title is the document's title, and the
// text leaves out each example that the document holds already, such as the
// line of its id, while the JSON gives every example.
// sokf-tests: CLI-17_AC-38
// sokf-tests: CLI-17_AC-39
// sokf-tests: CLI-17_AC-40
#[test]
fn each_example_carries_the_documents_values() {
    let f = Fixture::new("guide");
    let (json, err, code) = f.run(&["guide", "task-001-descale", "--json"]);
    assert_eq!(code, 0, "{err}");
    let value = valid("guide-report.schema.json", &json);
    let doc = &value["documents"][0];
    assert_eq!(doc["frontmatter"][1]["example"], "id: task-001-descale");
    assert_eq!(
        doc["sections"][0]["example"],
        "# Descale the kettle\n\nThe kettle needs descaling before it breaks."
    );
    let text = guide(&f, &["task-001-descale"]);
    assert!(
        !text.contains("id: task-001-descale")
            && !text.contains("The kettle needs descaling")
            && text.contains("      owner: ada\n"),
        "{text}"
    );
}

// A section that the document lacks prints its example, though a section of
// the same heading and the same example under another parent is in the
// document, because the guide asks the check's match which section a heading
// holds.
// sokf-tests: CLI-17_AC-40
#[test]
fn a_heading_of_another_parent_hides_no_example() {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/run.md",
        "---\ntype: Schema\nid: schema-run\ntitle: Run Schema\ndescription: A run.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Run\nsections:\n  - heading: Setup\n    level: 2\n    rule-id: setup\n    blocks: []\n  - heading: Notes\n    level: 3\n    parent: setup\n    blocks:\n      - kind: prose\n        example: The kettle is full.\n  - heading: Teardown\n    level: 2\n    rule-id: teardown\n    blocks: []\n  - heading: Notes\n    level: 3\n    parent: teardown\n    blocks:\n      - kind: prose\n        example: The kettle is full.\n````\n",
    );
    f.write(
        "knowledge/run.md",
        "---\ntype: Run\nid: run\ntitle: Run\n---\n\n## Setup\n\n### Notes\n\nThe kettle is full.\n\n## Teardown\n",
    );
    let text = guide(&f, &["run"]);
    assert!(
        text.ends_with(
            "### Notes (absent, required)\n  Example:\n    ### Notes\n\n    The kettle is full.\n"
        ),
        "{text}"
    );
}

// A document whose own value no scaffold holds, such as a title that the
// paragraph bound to it can't hold, still gets the schema's examples.
// sokf-tests: CLI-17_AC-39
#[test]
fn a_value_that_no_scaffold_holds_leaves_the_schemas_examples() {
    let f = Fixture::new("guide");
    f.write(
        "knowledge/schemas/card.md",
        "---\ntype: Schema\nid: schema-card\ntitle: Card Schema\ndescription: A card.\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Card\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks:\n      - kind: prose\n        each: { text: { from: title } }\n  - heading: Back\n    level: 2\n    blocks:\n      - kind: prose\n        example: The answer is on the back.\n````\n",
    );
    f.write(
        "knowledge/card.md",
        "---\ntype: Card\nid: card\ntitle: <div>\n---\n\n# <div>\n",
    );
    let text = guide(&f, &["card"]);
    assert!(
        text.contains("    The answer is on the back.\n") && text.contains("      Card example\n"),
        "{text}"
    );
}

// The example of a key is the schema's, where the document's own value would
// stand in it, so a value that breaks the key's rule hides no example.
// sokf-tests: CLI-17_AC-38
#[test]
fn a_keys_example_is_the_schemas_whatever_the_document_holds() {
    let f = Fixture::new("guide");
    let path = "knowledge/tasks/open/task-001-descale.md";
    let text = std::fs::read_to_string(f.path(path)).unwrap();
    f.write(
        path,
        &text.replace("state: open\n", "state: open\nowner: Ada9\n"),
    );
    let out = guide(&f, &["task-001-descale"]);
    assert!(out.contains("    Example:\n      owner: ada\n"), "{out}");
}
