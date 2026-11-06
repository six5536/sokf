//! This module reads `pack.sokf.yaml`, the manifest of a schema pack, and
//! compares the versions that the manifest gives (CLI-13).

use std::collections::BTreeMap;

use crate::{Error, Result, yaml};

/// `MANIFEST` is the file name of a pack's manifest.
pub const MANIFEST: &str = "pack.sokf.yaml";

/// `KEYS` lists the keys of a manifest. Each is required except `depends`.
const KEYS: &[&str] = &["name", "version", "requires-sokf", "description", "depends"];

/// A `PackManifest` is the content of `pack.sokf.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackManifest {
    /// The pack's name, a slug.
    pub name: String,
    /// The pack's version, in semantic versioning.
    pub version: String,
    /// The oldest version of sokf that the pack works with.
    pub requires_sokf: String,
    /// One sentence that says what the pack holds.
    pub description: String,
    /// The packs that the pack depends on, by name, each with the oldest
    /// version that it works with.
    pub depends: BTreeMap<String, String>,
}

/// `parse_manifest` reads the text of a manifest. `label` names the file in
/// an error. It refuses a manifest that lacks a required key, has an
/// unknown key, or gives a name that isn't a slug or a version that isn't
/// `MAJOR.MINOR.PATCH`, and a `depends` that isn't a map from the name of
/// another pack to such a version.
// sokf-implements: CLI-13_AC-1
// sokf-implements: SOKF_FORMAT_160
// sokf-implements: SOKF_FORMAT_161
pub fn parse_manifest(text: &str, label: &str) -> Result<PackManifest> {
    let fail = |m: String| Error::Pack(format!("{label}: {m}"));
    let doc = yaml::parse(text, 1).map_err(|e| fail(e.to_string()))?;
    let map = doc
        .as_map()
        .ok_or_else(|| fail("the manifest is not a map".into()))?;
    for (key, _) in map {
        if !KEYS.contains(&key.value.as_str()) {
            return Err(fail(format!("unknown key `{}`", key.value)));
        }
    }
    let get = |key: &str| -> Result<String> {
        doc.get(key)
            .and_then(|v| v.scalar_text())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| fail(format!("the manifest has no `{key}`")))
    };
    let manifest = PackManifest {
        name: get("name")?,
        version: get("version")?,
        requires_sokf: get("requires-sokf")?,
        description: get("description")?,
        depends: depends(&doc, &fail)?,
    };
    if !is_slug(&manifest.name) {
        return Err(fail(format!(
            "the name `{}` is not a slug: lowercase words of letters and digits, joined by `-`",
            manifest.name
        )));
    }
    for (key, v) in [
        ("version", &manifest.version),
        ("requires-sokf", &manifest.requires_sokf),
    ] {
        if parse_version(v).is_none() {
            return Err(fail(format!(
                "the `{key}` `{v}` is not a version of the form MAJOR.MINOR.PATCH"
            )));
        }
    }
    Ok(manifest)
}

/// `depends` reads the optional `depends` of the manifest `doc`: a map from
/// the name of a pack to the oldest version of it that the pack works with.
// sokf-implements: CLI-13_AC-67
fn depends(
    doc: &crate::Spanned,
    fail: &dyn Fn(String) -> Error,
) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let Some(value) = doc.get("depends") else {
        return Ok(out);
    };
    let map = value.as_map().ok_or_else(|| {
        fail("`depends` is not a map from the name of a pack to its version".into())
    })?;
    for (key, v) in map {
        let name = key.value.clone();
        let version = v.scalar_text().unwrap_or_default();
        if !is_slug(&name) {
            return Err(fail(format!(
                "the name `{name}` in `depends` is not a slug"
            )));
        }
        if parse_version(&version).is_none() {
            return Err(fail(format!(
                "the version `{version}` of `{name}` in `depends` is not of the form MAJOR.MINOR.PATCH"
            )));
        }
        out.insert(name, version);
    }
    Ok(out)
}

/// `is_slug` returns whether `s` is lowercase words of letters and digits,
/// joined by `-`.
pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| {
            !w.is_empty()
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// `parse_version` reads `MAJOR.MINOR.PATCH`, with any pre-release or build
/// suffix after `-` or `+` left out of the comparison.
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// `tool_version` returns the version of the running tool.
pub fn tool_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// `requires_newer` returns whether `required` names a version newer than
/// `tool`. A version that doesn't parse counts as newer.
// sokf-implements: CLI-13_AC-13
pub fn requires_newer(required: &str, tool: &str) -> bool {
    match (parse_version(required), parse_version(tool)) {
        (Some(r), Some(t)) => r > t,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str =
        "name: method\nversion: 1.2.0\nrequires-sokf: 0.1.0\ndescription: The method of a store.\n";

    #[test]
    fn a_manifest_reads_its_four_keys() {
        let m = parse_manifest(GOOD, "pack.sokf.yaml").unwrap();
        assert_eq!(m.name, "method");
        assert_eq!(m.version, "1.2.0");
        assert_eq!(m.requires_sokf, "0.1.0");
        assert_eq!(m.description, "The method of a store.");
        assert!(m.depends.is_empty());
    }

    // sokf-tests: CLI-13_AC-67
    #[test]
    fn a_manifest_reads_the_packs_that_it_depends_on() {
        let text = format!("{GOOD}depends:\n  house-base: 1.4.0\n");
        let m = parse_manifest(&text, "pack.sokf.yaml").unwrap();
        assert_eq!(
            m.depends,
            BTreeMap::from([("house-base".to_string(), "1.4.0".to_string())])
        );
        for bad in [
            "depends: [a]\n",
            "depends:\n  Base: 1.0.0\n",
            "depends:\n  base: 1.0\n",
        ] {
            let e = parse_manifest(&format!("{GOOD}{bad}"), "x/pack.sokf.yaml").unwrap_err();
            assert!(e.to_string().contains("depends"), "{bad:?} -> {e}");
        }
    }

    // sokf-tests: CLI-13_AC-1
    #[test]
    fn a_bad_manifest_is_refused_with_its_label() {
        for text in [
            "name: [",
            "- a\n",
            "name: method\n",
            "name: Method\nversion: 1.0.0\nrequires-sokf: 0.1.0\ndescription: d\n",
            "name: method\nversion: 1.0\nrequires-sokf: 0.1.0\ndescription: d\n",
            "name: method\nversion: 1.0.0\nrequires-sokf: 0.1.0\ndescription: d\nextra: 1\n",
        ] {
            let e = parse_manifest(text, "x/pack.sokf.yaml").unwrap_err();
            assert!(
                e.to_string().starts_with("pack: x/pack.sokf.yaml: "),
                "{text:?} -> {e}"
            );
        }
    }

    #[test]
    fn versions_compare_by_their_three_numbers() {
        assert!(!requires_newer("0.1.3", "0.1.3"));
        assert!(!requires_newer("0.1.2", "0.1.3"));
        assert!(requires_newer("0.2.0", "0.1.3"));
        assert!(requires_newer("1.0.0", "0.9.9"));
        assert!(!requires_newer("0.1.3-rc.1", "0.1.3"));
        assert!(requires_newer("x", "0.1.3"));
        assert_eq!(parse_version("1.2.3+build"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn a_slug_is_lowercase_words_joined_by_hyphens() {
        assert!(is_slug("sokf"));
        assert!(is_slug("my-method-2"));
        assert!(!is_slug(""));
        assert!(!is_slug("-a"));
        assert!(!is_slug("a--b"));
        assert!(!is_slug("A"));
    }
}
