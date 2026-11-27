//! This module tells which findings `sokf validate --fix` repairs, for the
//! line of the text report that counts them (CLI-4_AC-30), as a linter
//! counts the problems that a rule with a fixer raised.

use super::Finding;

/// `ALWAYS` holds the keys of the findings that `--fix` repairs wherever
/// they arise: a definition block that is missing, stale, or followed by
/// content, an index that is missing, a label, a `to`, or an include in a
/// longer form than its canonical one, a path link to a document or an
/// asset, a fragment of a label written as text, a sidecar's `resource`, a document out
/// of the directory of its states or of its schema's root, and a heading
/// that is setext, renamed, or bound to a frontmatter key.
const ALWAYS: [&str; 14] = [
    "SOKF_FORMAT_31",
    "SOKF_FORMAT_60",
    "SOKF_FORMAT_65",
    "SOKF_FORMAT_72",
    "SOKF_FORMAT_77",
    "SOKF_FORMAT_78",
    "SOKF_FORMAT_79",
    "SOKF_FORMAT_80",
    "SOKF_FORMAT_96",
    "SOKF_SCHEMA_26",
    "SOKF_SCHEMA_51",
    "TLMD_137",
    "TLMD_138",
    "TLMD_165",
];

/// `fixable` returns whether `--fix` repairs the finding `f`: a finding of a
/// key of `ALWAYS`, a stale include that sokf can regenerate, a fragment
/// written as a heading's text, a stale index in the generated form, an
/// include of an asset by its path, and an anchor that tramlines' repair
/// moves, writes, or removes.
pub(crate) fn fixable(f: &Finding) -> bool {
    match f.criterion.as_str() {
        "SOKF_FORMAT_85" => !f.message.contains("cannot be regenerated"),
        "SOKF_FORMAT_73" => f.message.contains("uses the heading text"),
        "SOKF_FORMAT_88" => f.message.contains("refers to an asset by path"),
        "SOKF_FORMAT_97" => f.message.starts_with("the index is stale"),
        "TLMD_109" => f.message.contains("its repair"),
        key => ALWAYS.contains(&key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RepoPath, Severity};

    fn finding(criterion: &str, message: &str) -> Finding {
        Finding::new(
            RepoPath::parse("/k/a.md").unwrap(),
            Some(1),
            Severity::Error,
            criterion,
            message,
        )
    }

    #[test]
    fn a_finding_is_fixable_by_its_key_and_variant() {
        assert!(fixable(&finding(
            "SOKF_FORMAT_85",
            "`sokf:include a` is stale"
        )));
        assert!(!fixable(&finding(
            "SOKF_FORMAT_85",
            "`sokf:include a` cannot be regenerated: a contains an include that does not resolve"
        )));
        assert!(fixable(&finding(
            "SOKF_FORMAT_80",
            "the definition block is stale"
        )));
        assert!(fixable(&finding(
            "SOKF_FORMAT_97",
            "the index is stale: it differs from the generated content"
        )));
        assert!(!fixable(&finding(
            "SOKF_FORMAT_97",
            "the file is not a generated index, so `--fix` doesn't overwrite it: move its content into a document"
        )));
        assert!(fixable(&finding(
            "SOKF_FORMAT_88",
            "`sokf:include /a.png` refers to an asset by path; include it by its sidecar's id as `sokf:include a`"
        )));
        assert!(!fixable(&finding(
            "SOKF_FORMAT_88",
            "`sokf:include /k/b.md` refers to a document by path. Include its body by id as `sokf:include b`, or show its source in a code block"
        )));
        assert!(fixable(&finding(
            "TLMD_109",
            "key K has no anchor; its repair writes `<a id=\"k\"></a>` before it"
        )));
        assert!(!fixable(&finding(
            "SOKF_FORMAT_61",
            "`[sokf:planer]` resolves to no document"
        )));
        assert!(!fixable(&finding("TL_74", "`x` fails at column 1")));
        assert!(fixable(&finding(
            "SOKF_FORMAT_73",
            "`#Retry once` uses the heading text. Use the slug `#retry-once` instead"
        )));
        assert!(!fixable(&finding(
            "SOKF_FORMAT_73",
            "`#D-1` matches no heading or key in this document"
        )));
    }
}
