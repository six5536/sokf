//! This module generates the JSON Schema of each JSON output that the tool prints or writes.
//! It builds each schema from the type that the tool serialises, so a schema cannot differ
//! from the output. A test checks that the files in `/knowledge/json/` match the schemas that
//! `output_schemas` returns.

use schemars::{JsonSchema, Schema, generate::SchemaSettings, transform::RecursiveTransform};
use serde_json::Value;

use crate::{
    GraphJson, HarnessReport, InitResult, LocateReport, NewResult, ValidateReport, harness,
};

/// `Output` names each JSON output that the tool prints or writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// The report of `validate --json`.
    Validate,
    /// The graph of `graph --json`.
    Graph,
    /// The report of `init --json`.
    Init,
    /// The report of `harness install --json` and `harness status --json`.
    Harness,
    /// The answer that `harness hook` writes for the harness.
    HarnessHookAnswer,
    /// The report of `locate --json`.
    Locate,
    /// The report of `new --json`.
    New,
}

impl Output {
    /// `ALL` lists every output, in the order of `output_schemas`.
    pub const ALL: [Output; 7] = [
        Output::Validate,
        Output::Graph,
        Output::Init,
        Output::Harness,
        Output::HarnessHookAnswer,
        Output::Locate,
        Output::New,
    ];

    /// `file` returns the name of the output's schema file in `/knowledge/json/`.
    pub fn file(self) -> &'static str {
        match self {
            Output::Validate => "validate-report.schema.json",
            Output::Graph => "graph.schema.json",
            Output::Init => "init-report.schema.json",
            Output::Harness => "harness-report.schema.json",
            Output::HarnessHookAnswer => "harness-hook-answer.schema.json",
            Output::Locate => "locate-report.schema.json",
            Output::New => "new-report.schema.json",
        }
    }

    /// `schema` returns the JSON Schema of the output.
    pub fn schema(self) -> Schema {
        match self {
            Output::Validate => schema::<ValidateReport>(),
            Output::Graph => schema::<GraphJson>(),
            Output::Init => schema::<InitResult>(),
            Output::Harness => schema::<HarnessReport>(),
            Output::HarnessHookAnswer => schema::<harness::HookAnswer>(),
            Output::Locate => schema::<LocateReport>(),
            Output::New => schema::<NewResult>(),
        }
    }

    /// `schema_text` returns the schema as the file in `/knowledge/json/` holds it: pretty
    /// JSON with a final newline.
    pub fn schema_text(self) -> String {
        serde_json::to_string_pretty(&self.schema()).expect("a schema serialises") + "\n"
    }
}

/// `output_schemas` returns the schema of each output, paired with the name of its file in
/// `/knowledge/json/`.
pub fn output_schemas() -> Vec<(&'static str, Schema)> {
    Output::ALL.iter().map(|o| (o.file(), o.schema())).collect()
}

// `schema` generates a Draft 2020-12 schema. It joins the lines of each paragraph of a
// description into one line, because a doc comment wraps its lines for the source, not for
// the reader of the schema.
fn schema<T: JsonSchema>() -> Schema {
    SchemaSettings::draft2020_12()
        .with_transform(RecursiveTransform(unwrap_description))
        .into_generator()
        .into_root_schema_for::<T>()
}

fn unwrap_description(schema: &mut Schema) {
    if let Some(Value::String(text)) = schema.get_mut("description") {
        *text = text
            .split("\n\n")
            .map(|paragraph| paragraph.split('\n').collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n\n");
    }
}
