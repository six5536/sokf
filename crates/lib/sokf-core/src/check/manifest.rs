//! This module checks each manifest against the format specification, and
//! checks that each store identity is unique among the roots.

use std::collections::BTreeMap;

use crate::{
    Finding, Knowledge, Model,
    markdown::is_slug,
    model::{Identity, extends::Broken, manifest::SOKF_VERSION},
};

/// `check` reports each way in which a store's manifest breaks the format.
/// The manifest must exist and be valid YAML. It must declare the version
/// that this tool implements, and its `name` and `scope` must be slugs. Each
/// former identity must have the form `<scope>/<name>`, and the manifest must
/// not contain a stamped key.
// sokf-implements: SOKF_FORMAT_1
// sokf-implements: SOKF_FORMAT_9
// sokf-implements: SOKF_FORMAT_10
// sokf-implements: SOKF_FORMAT_11
// sokf-implements: SOKF_FORMAT_16
// sokf-implements: SOKF_VALIDATION_3
pub fn check(knowledge: &Knowledge) -> Vec<Finding> {
    let path = &knowledge.manifest_path;
    let Some(manifest) = &knowledge.manifest else {
        return vec![Finding::error(
            path.clone(),
            None,
            "SOKF_FORMAT_1",
            "the manifest is missing",
        )];
    };
    let mut findings = manifest.problems.clone();
    if !manifest.problems.is_empty() {
        return findings;
    }
    match manifest.sokf.as_deref() {
        None => findings.push(Finding::error(
            path.clone(),
            None,
            "SOKF_FORMAT_9",
            "the manifest declares no `sokf` version",
        )),
        // sokf-implements: CLI-10_AC-2
        Some(v) if v != SOKF_VERSION => findings.push(Finding::error(
            path.clone(),
            manifest.line_of.get("sokf").copied(),
            "SOKF_VALIDATION_3",
            format!("the manifest declares sokf `{v}`. This tool implements `{SOKF_VERSION}`"),
        )),
        Some(_) => {}
    }
    for key in ["name", "scope"] {
        let value = match key {
            "name" => manifest.name.as_deref(),
            _ => manifest.scope.as_deref(),
        };
        let line = manifest.line_of.get(key).copied();
        match value.filter(|v| !v.trim().is_empty()) {
            Some(v) if is_slug(v) => {}
            Some(v) => findings.push(Finding::error(
                path.clone(),
                line,
                "SOKF_FORMAT_10",
                format!("the manifest's `{key}` `{v}` is not a slug. A slug is lower-case letters and digits joined by single hyphens"),
            )),
            None => findings.push(Finding::error(
                path.clone(),
                line,
                "SOKF_FORMAT_10",
                match key {
                    "name" => "the manifest declares no `name`".to_string(),
                    _ => "the manifest declares no `scope`. The scope names the store's owner as a slug: lower-case letters and digits joined by single hyphens. The store's identity is `<scope>/<name>`".to_string(),
                },
            )),
        }
    }
    for (old, line) in &manifest.renamed_from {
        if Identity::parse(old).is_none() {
            findings.push(Finding::error(
                path.clone(),
                Some(*line),
                "SOKF_FORMAT_11",
                format!("the manifest's `renamed-from` entry `{old}` is not of the form `<scope>/<name>`, where both parts are slugs"),
            ));
        }
    }
    // sokf-implements: SOKF_FORMAT_205
    // sokf-implements: FORMAT-5_AC-4
    if let Some(v) = manifest.unique.as_deref().filter(|v| *v != "knowledge") {
        findings.push(Finding::error(
            path.clone(),
            manifest.line_of.get("unique").copied(),
            "SOKF_FORMAT_205",
            format!("the manifest's `unique` is `{v}`, but the one value that caps the keys of the store is `knowledge`"),
        ));
    }
    for (key, line) in &manifest.stamped_present {
        findings.push(Finding::error(
            path.clone(),
            Some(*line),
            "SOKF_FORMAT_16",
            format!("the manifest contains `{key}`, a key that sokf adds only when it publishes the store. The key must not appear in the working tree"),
        ));
    }
    findings
}

/// `check_identities` reports each identity, current or former, that two or
/// more roots share (`SOKF_FORMAT_12`, `SOKF_FORMAT_13`). An identity must be
/// unique for two reasons: a qualified id must identify one store, and an
/// old reference must reach the store it meant. It reports a clash as a finding on the manifest of each
/// store that uses the identity.
// sokf-implements: SOKF_FORMAT_12
// sokf-implements: SOKF_FORMAT_13
// sokf-implements: CLI-4_AC-29
pub fn check_identities(model: &Model) -> Vec<Finding> {
    let mut holders: BTreeMap<Identity, Vec<usize>> = BTreeMap::new();
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        let mut own: Vec<Identity> = knowledge.identity().into_iter().collect();
        own.extend(knowledge.former());
        own.sort();
        own.dedup();
        for identity in own {
            holders.entry(identity).or_default().push(k);
        }
    }
    let mut findings = Vec::new();
    for (identity, stores) in holders.iter().filter(|(_, s)| s.len() > 1) {
        // Two stores with the same current identity break the rule on
        // identities. Two stores that list the same former identity break
        // the rule on `renamed-from`.
        let current = stores
            .iter()
            .any(|&o| model.knowledges[o].identity().as_ref() == Some(identity));
        let key = if current {
            "SOKF_FORMAT_12"
        } else {
            "SOKF_FORMAT_13"
        };
        for &k in stores {
            let knowledge = &model.knowledges[k];
            let others: Vec<String> = stores
                .iter()
                .filter(|&&o| o != k)
                .map(|&o| model.knowledges[o].root.to_string())
                .collect();
            let line = knowledge
                .manifest
                .as_ref()
                .and_then(|m| m.line_of.get("name").copied());
            findings.push(Finding::error(
                knowledge.manifest_path.clone(),
                line,
                key,
                format!(
                    "{} the identity `{identity}`. A store's identity, current or former, must be unique among the roots",
                    super::frontmatter::also_use(&others),
                ),
            ));
        }
    }
    findings
}

/// `check_extends` reports the `extends` line of each manifest that names
/// no knowledge store of the repository, or that lies on a cycle of
/// `extends` (`SOKF_FORMAT_206`, `SOKF_FORMAT_207`). Such a store is broken:
/// it inherits nothing, and the run checks none of its files
/// (FORMAT-5_AC-10).
// sokf-implements: SOKF_FORMAT_206
// sokf-implements: SOKF_FORMAT_207
// sokf-implements: FORMAT-5_AC-8
// sokf-implements: FORMAT-5_AC-9
pub fn check_extends(model: &Model) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        let Some(why) = model.broken(k).cloned() else {
            continue;
        };
        let line = knowledge
            .manifest
            .as_ref()
            .and_then(|m| m.line_of.get("extends").copied());
        let (key, message) = match why {
            Broken::NoStore(written) => (
                "SOKF_FORMAT_206",
                format!("the manifest's `extends` `{written}` names no knowledge store of the repository by its `<scope>/<name>`, so sokf checks no file of the store"),
            ),
            Broken::Shared(written) => (
                "SOKF_FORMAT_206",
                format!("the manifest's `extends` `{written}` is the identity of more than one knowledge store, so it names none, and sokf checks no file of the store"),
            ),
            Broken::Cycle => (
                "SOKF_FORMAT_207",
                "the manifest's `extends` leads back to the store through the `extends` of other stores, so sokf checks no file of the store".to_string(),
            ),
        };
        findings.push(Finding::error(
            knowledge.manifest_path.clone(),
            line,
            key,
            message,
        ));
    }
    findings
}
