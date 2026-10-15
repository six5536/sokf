//! This module finds which formats and types in the registry of formats a
//! schema uses, and computes a fingerprint of that registry. Together,
//! `schema_uses` and `fingerprint` let the cache mark as dirty only the
//! schemas that use a format or a type changed by an edit to the registry
//! (CACHE-1_AC-17).

use std::{collections::BTreeSet, sync::OnceLock};

use regex::Regex;

use super::{FormatRegistry, TypeDef};
use crate::source::fnv1a64;

/// `schema_uses` returns the names of the registry entries that a schema's
/// text uses, sorted and without duplicates: `t:<name>` for the type of each
/// placeholder, `${x:name}`, wherever it appears, and `f:<name>` for each
/// format named in a `format:` key. The list also holds the names that the
/// schema defines itself, because a schema's own entry can patch the shared
/// one. The cost of these extra names is at most one more check of that
/// schema.
// sokf-implements: CACHE-1_AC-17
pub fn schema_uses(text: &str) -> Vec<String> {
    static HOLES: OnceLock<Regex> = OnceLock::new();
    static FORMATS: OnceLock<Regex> = OnceLock::new();
    let holes =
        HOLES.get_or_init(|| Regex::new(r"\$\{[^{}]*:([A-Za-z0-9_-]+)\}").expect("a valid regex"));
    let formats = FORMATS.get_or_init(|| {
        Regex::new(r"\bformat:\s*['\x22]?([A-Za-z0-9_-]+)").expect("a valid regex")
    });
    let mut uses: BTreeSet<String> = BTreeSet::new();
    for cap in holes.captures_iter(text) {
        uses.insert(format!("t:{}", &cap[1]));
    }
    for cap in formats.captures_iter(text) {
        uses.insert(format!("f:{}", &cap[1]));
    }
    uses.into_iter().collect()
}

/// `fingerprint` returns one hash for each entry of the registry that a schema
/// can use, keyed `f:<name>` for a format and `t:<name>` for a type. A format's
/// hash covers its compiled shapes, the examples of its variants, its key, its
/// example, and its prohibited patterns. Because the compiled shapes include
/// the types they use, a change to one of those types changes the format's
/// hash too. A type's hash covers its definition. The store keeps these
/// hashes, so that it can compare an edited registry with the old one entry
/// by entry.
// sokf-implements: CACHE-1_AC-17
pub fn fingerprint(registry: &FormatRegistry) -> Vec<(String, u64)> {
    let mut out = Vec::with_capacity(registry.entries.len() + registry.types.entries.len());
    for (name, (format, _)) in &registry.entries {
        let mut text = String::new();
        for (label, shape) in format
            .shape
            .iter()
            .map(|s| ("", s))
            .chain(format.variants.iter().map(|(n, v)| (n.as_str(), &v.shape)))
        {
            text.push_str(label);
            text.push('\t');
            match shape.regex() {
                Some(re) => text.push_str(re.as_str()),
                None => text.push_str(&format!("{shape:?}")),
            }
            text.push('\n');
        }
        for (_, v) in &format.variants {
            text.push_str(&v.example);
            text.push('\n');
        }
        if let Some(key) = &format.key {
            text.push_str(key);
        }
        text.push('\n');
        if let Some(example) = &format.example {
            text.push_str(example);
        }
        text.push('\n');
        // A ban's message is part of what a finding and a scaffold say, so
        // a change to it changes the format.
        for p in &format.prohibited {
            text.push_str(p.pattern.as_str());
            text.push('\t');
            text.push_str(&p.message);
            text.push('\n');
        }
        out.push((format!("f:{name}"), fnv1a64(text.as_bytes())));
    }
    for (name, (def, _)) in &registry.types.entries {
        let text = match def {
            TypeDef::Regex(r) => r.clone(),
            TypeDef::Choices(c) => c.join("\n"),
        };
        out.push((format!("t:{name}"), fnv1a64(text.as_bytes())));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_schema_uses_its_hole_types_and_its_named_formats() {
        let text = "id:\n  template: \"issue-${:seq}-${:slug}\"\nblocks:\n  - each: { format: requirement-ears }\n  - each:\n      format: 'user-story'\n      template: '${n:int}. ${}'\n";
        assert_eq!(
            schema_uses(text),
            vec![
                "f:requirement-ears",
                "f:user-story",
                "t:int",
                "t:seq",
                "t:slug"
            ]
        );
        assert!(schema_uses("sections: []").is_empty());
    }
}
