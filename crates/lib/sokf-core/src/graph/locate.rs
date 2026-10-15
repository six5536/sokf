//! `locate` turns ids into absolute filesystem paths (CLI-8).

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Serialize;

use super::resolve_id;
use crate::{Error, Model, Result, model::KeyIndex};

/// A `Location` records where one id's document is, and where its asset is when
/// the document is a sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The id.
    pub id: String,
    /// The document's filesystem path.
    pub path: PathBuf,
    /// The asset's filesystem path, for a sidecar.
    pub asset: Option<PathBuf>,
}

/// The report of `sokf locate --json`: one entry for each id, in argument order
/// (CLI-8_AC-5).
// `LocateReport` is a wire type: `sokf locate --json` prints its report with
// this type, and the `output_schema` module generates the published JSON schema,
// `locate-report.schema.json`, from it.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(title = "sokf locate report")]
pub struct LocateReport(Vec<LocateEntry>);

/// A location entry holds one id and where its document is.
#[derive(Serialize, JsonSchema)]
#[schemars(rename = "Location")]
struct LocateEntry {
    /// The id, as given.
    id: String,
    /// The absolute filesystem path of the document.
    path: String,
    /// The absolute filesystem path of the asset, when the document is a sidecar: a
    /// document that describes a separate file, the asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    asset: Option<String>,
}

impl From<&[Location]> for LocateReport {
    fn from(located: &[Location]) -> Self {
        LocateReport(
            located
                .iter()
                .map(|l| LocateEntry {
                    id: l.id.clone(),
                    path: l.path.display().to_string(),
                    asset: l.asset.as_ref().map(|a| a.display().to_string()),
                })
                .collect(),
        )
    }
}

/// `locate` returns the filesystem paths of each id, in argument order. If an
/// id doesn't resolve, the whole call fails. With `asset`, the call also fails
/// if an id's document isn't a sidecar.
// sokf-implements: CLI-8_AC-1
// sokf-implements: CLI-8_AC-4
// sokf-implements: CLI-8_AC-2
// sokf-implements: CLI-8_AC-3
// sokf-implements: CLI-8_AC-7
pub fn locate(
    model: &Model,
    keys: Option<&KeyIndex>,
    ids: &[String],
    asset: bool,
) -> Result<Vec<Location>> {
    ids.iter()
        .map(|id| {
            let path = resolve_id(model, keys, id)?;
            let asset_path = model
                .asset_of(&path)
                .map(|(_, a)| a.path.to_fs(&model.tree.root));
            if asset && asset_path.is_none() {
                return Err(Error::Locate(format!(
                    "`{id}` identifies {path}, which is not a sidecar and has no asset"
                )));
            }
            Ok(Location {
                id: id.clone(),
                path: path.to_fs(&model.tree.root),
                asset: asset_path,
            })
        })
        .collect()
}
