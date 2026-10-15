//! This module loads the grammar that SOKF's schemas are written in, and the
//! formats that the tool embeds, and compiles one schema with them.

use tramlines_md::grammar::{FormatRegistry, Grammar, RegexCache, lower, validate_schema};

use super::SokfSchema;
use crate::{Finding, RepoPath, Spanned, markdown::SOKF};

/// `SOKF_TYPES` holds the types that SOKF defines. No layer can redefine
/// them (TL_29). The writer of a schema writes a reference as a placeholder
/// of one of these types, and the tool matches the placeholder with that
/// type's pattern.
pub const SOKF_TYPES: [(&str, &str); 2] = [
    // `store` matches an optional store prefix: none, `name:`, or
    // `scope/name:`, as in the qualified ids of sokf-format-spec.
    (
        "store",
        "(?:(?:[a-z0-9]+(?:-[a-z0-9]+)*/)?[a-z0-9]+(?:-[a-z0-9]+)*:)?",
    ),
    // `ref` matches any target of a `sokf:` label: an optional store prefix,
    // followed by either an id with an optional fragment or a fragment alone.
    (
        "ref",
        "(?:(?:[a-z0-9]+(?:-[a-z0-9]+)*/)?[a-z0-9]+(?:-[a-z0-9]+)*:)?(?:[a-z0-9]+(?:-[a-z0-9]+)*(?:#[A-Za-z0-9_-]*)?|#[A-Za-z0-9_-]+)",
    ),
];

/// `formats_text` returns the text of `knowledge/formats/sokf.formats.yaml`, the
/// shared line formats, unchanged.
pub fn formats_text() -> &'static str {
    include_str!(concat!(
        env!("OUT_DIR"),
        "/knowledge/formats/sokf.formats.yaml"
    ))
}

/// `embedded_formats` builds the registry of the formats that the tool embeds:
/// SOKF's reference types, followed by its shared formats.
// sokf-implements: EXT-11_AC-1
pub fn embedded_formats(grammar: &Grammar, regexes: &RegexCache) -> crate::Result<FormatRegistry> {
    Ok(FormatRegistry::from_text(
        formats_text(),
        &SOKF_TYPES,
        grammar,
        regexes,
    )?)
}

/// `compile` validates a schema against the grammar, and then lowers it,
/// together with the rules that SOKF's extension adds.
// sokf-implements: SOKF_SCHEMA_43
pub fn compile(
    yaml: &Spanned,
    grammar: &Grammar,
    registry: &FormatRegistry,
    regexes: &RegexCache,
    path: &RepoPath,
    id: &str,
) -> std::result::Result<SokfSchema, Vec<Finding>> {
    let problems = validate_schema(yaml, grammar, regexes, path);
    if !problems.is_empty() {
        return Err(problems);
    }
    let (core, mut findings) = lower(yaml, grammar, registry, regexes, path, id, &SOKF);
    let (schema, more) = SokfSchema::lower(core, yaml, path);
    findings.extend(more);
    if findings.is_empty() {
        Ok(schema)
    } else {
        Err(findings)
    }
}

/// `extension_text` returns the text of `sokf-spec/sokf.tramlines.yaml`,
/// SOKF's schema extension of the grammar, unchanged.
pub fn extension_text() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/sokf-spec/sokf.tramlines.yaml"))
}

/// `load_grammar` loads the grammar that SOKF's schemas are written in:
/// the tramlines-md grammar, laid on the tramlines core grammar, with SOKF's
/// extension merged into it.
// sokf-implements: SOKF_SCHEMA_43
pub fn load_grammar() -> crate::Result<Grammar> {
    let mut grammar = tramlines_md::grammar::load_grammar()?;
    grammar.extend(extension_text())?;
    Ok(grammar)
}

/// `grammar_text` returns the schema language as one YAML stream: the
/// tramlines core grammar, then the tramlines-md grammar, which lays the
/// markdown format on the core, and then SOKF's schema extension.
pub fn grammar_text() -> String {
    format!(
        "{}---\n{}---\n{}",
        tramlines_md::grammar::core_grammar_text(),
        tramlines_md::grammar::grammar_text(),
        extension_text()
    )
}
