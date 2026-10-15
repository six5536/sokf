//! This module parses the two forms of `sokf:` reference: a label,
//! `sokf:<store>:<id>#<fragment>` (sokf-format-spec), and the target of an
//! include marker (SOKF_FORMAT_88).

use serde::{Deserialize, Serialize};

use super::{Link, LinkTarget};
use crate::model::keys::Label;

/// `split_store` splits a qualified id at its first colon into the store and
/// the id. `sokf-spec:glossary` splits into the store `sokf-spec` and the id
/// `glossary`. In a label with only a fragment, such as `sokf:sokf-spec:#K`,
/// the text before the `#` is `sokf-spec:`, which splits into the store and
/// an empty id. `glossary` has no colon, so it has no store.
pub fn split_store(text: &str) -> (Option<&str>, &str) {
    match text.split_once(':') {
        Some((store, rest)) => (Some(store), rest),
        None => (None, text),
    }
}

impl<'a> Label<'a> {
    /// `parse` splits the text that follows `sokf:` in a label into its
    /// parts. The text has the form `store:id#fragment`, and any part can be
    /// absent. `parse` treats an empty id as absent.
    pub fn parse(rest: &'a str) -> Self {
        let (head, fragment) = match rest.split_once('#') {
            Some((head, fragment)) => (head, Some(fragment)),
            None => (rest, None),
        };
        let (store, id) = split_store(head);
        Label {
            store,
            id: (!id.is_empty()).then_some(id),
            fragment,
        }
    }

    /// `of` returns the parts of a `sokf:` link target, and `None` for any
    /// other target.
    pub fn of(target: &'a LinkTarget) -> Option<Self> {
        match target {
            LinkTarget::Host(rest) => Some(Label::parse(rest)),
            _ => None,
        }
    }
}

/// `label_parts` returns the parts of a label written with its `sokf:`
/// prefix, and `None` for text without the prefix.
pub fn label_parts(label: &str) -> Option<Label<'_>> {
    label.strip_prefix("sokf:").map(Label::parse)
}

/// `sokf_label` returns the `sokf:` label of a link that addresses a
/// document by id, and `None` for any other link. The label has the form of
/// a label in the definition block.
pub fn sokf_label(link: &Link) -> Option<String> {
    match &link.target {
        LinkTarget::Host(rest) => Some(format!("sokf:{rest}")),
        _ => None,
    }
}

/// `IncludeTarget` is the target of an include marker, in one of its two
/// forms (SOKF_FORMAT_88).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncludeTarget {
    /// `<!-- sokf:include <id>[#<fragment>] -->` includes a document by id.
    /// When the id is a sidecar's, the target without a fragment is the
    /// asset, a named fragment is a region of the asset, and an empty
    /// fragment is the sidecar's body (FORMAT-2_AC-5).
    Document {
        /// `store` is the store that the marker names, in the form `name` or
        /// `scope/name`, or `None` when it names none.
        store: Option<String>,
        /// `id` is the id of the document to include.
        id: String,
        /// `fragment` is the text after `#`, or `None` when there is no `#`.
        /// It is empty for `<id>#`.
        fragment: Option<String>,
    },
    /// `<!-- sokf:include /path#region -->` includes a file, or a region of
    /// it, by its path.
    File {
        /// `path` is the path as the marker writes it, starting with `/`.
        path: String,
        /// `region` is the name of the region, or `None` when the marker
        /// names none.
        region: Option<String>,
    },
}

impl IncludeTarget {
    /// `as_written` returns the target in the form it takes in the open
    /// marker.
    pub fn as_written(&self) -> String {
        match self {
            IncludeTarget::Document {
                store,
                id,
                fragment,
            } => format!(
                "{}{id}{}",
                store
                    .as_deref()
                    .map(|s| format!("{s}:"))
                    .unwrap_or_default(),
                fragment
                    .as_deref()
                    .map(|f| format!("#{f}"))
                    .unwrap_or_default()
            ),
            IncludeTarget::File { path, region } => match region {
                Some(r) => format!("{path}#{r}"),
                None => path.clone(),
            },
        }
    }

    /// `parse` reads the target from the text of an open marker: text that
    /// starts with `/` is a file, and any other text is a document.
    pub fn parse(text: &str) -> Self {
        let (head, tail) = match text.split_once('#') {
            Some((h, t)) => (h.to_string(), Some(t.to_string())),
            None => (text.to_string(), None),
        };
        if text.starts_with('/') {
            IncludeTarget::File {
                path: head,
                region: tail,
            }
        } else {
            let (store, id) = split_store(&head);
            IncludeTarget::Document {
                store: store.map(str::to_string),
                id: id.to_string(),
                fragment: tail,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_split_into_their_parts() {
        let l = |s| Label::parse(s);
        assert_eq!(
            l("config"),
            Label {
                store: None,
                id: Some("config"),
                fragment: None
            }
        );
        assert_eq!(
            l("req#CLI-1_AC-2"),
            Label {
                store: None,
                id: Some("req"),
                fragment: Some("CLI-1_AC-2")
            }
        );
        assert_eq!(
            l("#K"),
            Label {
                store: None,
                id: None,
                fragment: Some("K")
            }
        );
        assert_eq!(
            l("acme/common:#K"),
            Label {
                store: Some("acme/common"),
                id: None,
                fragment: Some("K")
            }
        );
        assert_eq!(label_parts("sokf:a#b"), Some(l("a#b")));
        assert_eq!(label_parts("a#b"), None);
        assert_eq!(split_store("glossary"), (None, "glossary"));
        assert_eq!(
            split_store("sokf-spec:glossary"),
            (Some("sokf-spec"), "glossary")
        );
        assert_eq!(split_store("acme/common:"), (Some("acme/common"), ""));
    }

    #[test]
    fn a_label_is_rebuilt_as_written() {
        let link = |rest: &str| Link {
            target: LinkTarget::Host(rest.into()),
            line: 1,
            is_image: false,
            end_line: 1,
            inline: None,
        };
        assert_eq!(sokf_label(&link("a")).unwrap(), "sokf:a");
        assert_eq!(sokf_label(&link("a#b")).unwrap(), "sokf:a#b");
        assert_eq!(sokf_label(&link("#K")).unwrap(), "sokf:#K");
        let url = Link {
            target: LinkTarget::Url("https://x".into()),
            ..link("a")
        };
        assert!(sokf_label(&url).is_none());
    }

    #[test]
    fn include_targets() {
        assert_eq!(IncludeTarget::parse("/a.rs#cli").as_written(), "/a.rs#cli");
        assert_eq!(IncludeTarget::parse("/a.rs").as_written(), "/a.rs");
        assert_eq!(
            IncludeTarget::parse("doc-id"),
            IncludeTarget::Document {
                store: None,
                id: "doc-id".into(),
                fragment: None
            }
        );
        // When the id is a sidecar's, a fragment targets a region of its
        // asset, and an empty fragment targets the body of the sidecar
        // (FORMAT-2_AC-5).
        assert_eq!(
            IncludeTarget::parse("rates#q3"),
            IncludeTarget::Document {
                store: None,
                id: "rates".into(),
                fragment: Some("q3".into())
            }
        );
        assert_eq!(
            IncludeTarget::parse("flowers#"),
            IncludeTarget::Document {
                store: None,
                id: "flowers".into(),
                fragment: Some(String::new())
            }
        );
        assert_eq!(IncludeTarget::parse("rates#q3").as_written(), "rates#q3");
        assert_eq!(IncludeTarget::parse("flowers#").as_written(), "flowers#");
    }
}
