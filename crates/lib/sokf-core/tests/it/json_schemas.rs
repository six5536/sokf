//! Each JSON Schema in `/knowledge/json/` must be the schema that the wire type of its output
//! generates, value for value, in any layout, because the project's formatter lays the files
//! out. The published shape then can't drift from what the tool prints or writes. After a wire
//! type changes, run these tests with `SOKF_UPDATE_SCHEMAS=1` to write the files again, and
//! then the formatter.

use std::{fs, path::PathBuf};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../knowledge/json")
}

#[test]
fn each_committed_schema_is_the_generated_one() {
    let update = std::env::var_os("SOKF_UPDATE_SCHEMAS").is_some();
    let mut stale = Vec::new();
    for output in sokf_core::Output::ALL {
        let (name, text) = (output.file(), output.schema_text());
        let path = dir().join(name);
        if update {
            fs::create_dir_all(dir()).unwrap();
            fs::write(&path, &text).unwrap();
        } else {
            let value = |t: &str| serde_json::from_str::<serde_json::Value>(t).ok();
            let committed = fs::read_to_string(&path).ok();
            if committed.as_deref().and_then(value) != value(&text) {
                stale.push(name);
            }
        }
    }
    assert!(
        stale.is_empty(),
        "these files in /knowledge/json/ differ from the schemas their wire types generate: \
         {stale:?}. Run the test with SOKF_UPDATE_SCHEMAS=1 to write them"
    );
}

// Every file in `/knowledge/json/` other than `index.md` must be the schema of an output or the
// sidecar of such a schema: the Markdown file beside it with the same stem. A file that no output
// generates describes nothing.
#[test]
fn no_schema_file_is_left_without_an_output() {
    let known: Vec<&str> = sokf_core::output_schemas()
        .iter()
        .map(|(n, _)| *n)
        .collect();
    for entry in fs::read_dir(dir()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if name == "index.md" {
            continue;
        }
        if let Some(stem) = name.strip_suffix(".md") {
            assert!(
                known.contains(&stem),
                "{name} is a Markdown file for a schema, but no output has a schema of that name"
            );
        } else {
            assert!(
                known.contains(&name.as_str()),
                "{name} is a schema, but no output generates it"
            );
        }
    }
}
