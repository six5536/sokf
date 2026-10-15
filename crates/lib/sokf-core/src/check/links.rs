//! This module checks links: labels, fragments, paths, images, footnotes,
//! and mirroring (SOKF_FORMAT_30, SOKF_FORMAT_37, SOKF_FORMAT_41,
//! SOKF_FORMAT_53, and SOKF_FORMAT_60 to SOKF_FORMAT_76).

use super::{Citation, Citations, Ctx, DocView};
use crate::{
    Finding, RepoPath,
    graph::{Target, resolve_to},
    markdown::{Link, LinkTarget, is_url, sokf_label},
    model::keys::{self, Label, Unresolved},
};

/// `check_document` checks every body link, image, and footnote of a
/// document or additional file, and the `resource`, `sources`, and `links`
/// of its frontmatter. It records each `sokf:` label that resolves in a typed
/// document in `citations`. The coverage pass reads `citations`.
// sokf-implements: SOKF_FORMAT_30
// sokf-implements: SOKF_FORMAT_37
// sokf-implements: SOKF_FORMAT_41
// sokf-implements: SOKF_FORMAT_53
// sokf-implements: SOKF_FORMAT_60
// sokf-implements: SOKF_FORMAT_61
// sokf-implements: SOKF_FORMAT_64
// sokf-implements: SOKF_FORMAT_65
// sokf-implements: SOKF_FORMAT_66
// sokf-implements: SOKF_FORMAT_67
// sokf-implements: SOKF_FORMAT_70
// sokf-implements: SOKF_FORMAT_71
// sokf-implements: SOKF_FORMAT_72
// sokf-implements: SOKF_FORMAT_73
// sokf-implements: SOKF_FORMAT_74
// sokf-implements: SOKF_FORMAT_75
// sokf-implements: SOKF_FORMAT_76
// sokf-implements: FORMAT-2_AC-7
// sokf-implements: FORMAT-2_AC-8
pub fn check_document(doc: DocView<'_>, ctx: &Ctx<'_>, citations: &mut Citations) -> Vec<Finding> {
    let mut findings = Vec::new();
    let path = doc.path;
    let fail =
        |findings: &mut Vec<Finding>, line: Option<usize>, msg: String, key: &'static str| {
            findings.push(Finding::error(path.clone(), line, key, msg));
        };
    let knowledge = ctx.knowledge;
    for link in &doc.body.links {
        check_link(link, doc, ctx, &mut findings, citations);
    }
    suffix_clashes(doc, ctx, &mut findings);
    let Some(core) = doc.core else {
        return findings;
    };
    // Each footnote label must match a `sources[].id`.
    for (label, line) in &doc.body.footnote_refs {
        if !core.sources.iter().any(|s| s.id.as_deref() == Some(label)) {
            fail(
                &mut findings,
                Some(*line),
                format!("footnote `[^{label}]` matches no `sources[].id`"),
                "SOKF_FORMAT_41",
            );
        }
    }
    // Each path in `resource` and `sources[].resource` must name a file that
    // exists, or, in `resource`, a file that a document plans.
    // sokf-implements: SOKF_SCHEMA_53
    for resource in core.resource.iter().flat_map(|r| r.entries()) {
        if let Some(p) = RepoPath::parse(resource)
            && !ctx.model.exists(&p)
            && !ctx.model.is_planned(&p)
        {
            fail(
                &mut findings,
                core.line_of("resource"),
                format!("`resource` refers to a file that doesn't exist: `{resource}`"),
                "SOKF_FORMAT_30",
            );
        }
    }
    for source in &core.sources {
        if let Some(p) = RepoPath::parse(&source.resource) {
            if !ctx.model.exists(&p) {
                fail(
                    &mut findings,
                    Some(source.line),
                    format!(
                        "`sources[].resource` refers to a file that doesn't exist: {}",
                        source.resource
                    ),
                    "SOKF_FORMAT_37",
                );
            }
        } else if !is_url(&source.resource) {
            fail(
                &mut findings,
                Some(source.line),
                format!(
                    "`sources[].resource` must be a `/` path or a URL: {}",
                    source.resource
                ),
                "SOKF_FORMAT_37",
            );
        }
    }
    // Each `links` entry must resolve, and a body link must mirror it.
    let Some(k) = knowledge else {
        return findings;
    };
    for entry in &core.links {
        let Some(target) = resolve_to(ctx.model, k, &entry.to) else {
            // When the entry refers to a store that is not a knowledge root, the
            // message mentions that store, as it does for a label.
            let (message, key) = match crate::model::split_store(&entry.to) {
                (Some(store), _)
                    if !entry.to.starts_with('/') && ctx.model.store(k, store).is_none() =>
                {
                    (
                        format!(
                            "`links` entry `to: {}` refers to the store `{store}`, which is not a knowledge root of this repository",
                            entry.to
                        ),
                        "SOKF_FORMAT_64",
                    )
                }
                _ => (
                    format!("`links` entry `to: {}` resolves to nothing", entry.to),
                    "SOKF_FORMAT_53",
                ),
            };
            fail(&mut findings, Some(entry.line), message, key);
            continue;
        };
        // An entry must refer to a document by its canonical id, not by a path or
        // by an id with a longer store prefix.
        if let Some(canonical) = canonical_to(ctx, k, &target)
            && canonical != entry.to
        {
            fail(
                &mut findings,
                Some(entry.line),
                format!(
                    "write the `links` entry `to: {}` as `to: {canonical}`, the shortest form that identifies the document in this store. `--fix` makes this change",
                    entry.to
                ),
                "SOKF_FORMAT_65",
            );
        }
        let mirrored = doc.body.links.iter().any(|l| match (&l.target, &target) {
            // A bare key does not identify a document, so it cannot mirror a `links` entry.
            (
                LinkTarget::Host(rest),
                Target::Document {
                    path: target_path, ..
                },
            ) => {
                let label = Label::parse(rest);
                label.id.is_some()
                    && keys::resolve(ctx.model, ctx.keys, k, label)
                        .is_ok_and(|r| &r.path == target_path)
            }
            (
                LinkTarget::Repo { path, .. },
                Target::Document {
                    path: target_path, ..
                },
            ) => path == target_path,
            (LinkTarget::Repo { path, .. }, Target::File(target_path)) => path == target_path,
            (LinkTarget::Relative { target: rel, .. }, Target::File(target_path)) => {
                doc.path.resolve_relative(rel).as_ref() == Some(target_path)
            }
            _ => false,
        });
        if !mirrored {
            fail(
                &mut findings,
                Some(entry.line),
                format!(
                    "`links` entry `to: {}` has no body link that mirrors it",
                    entry.to
                ),
                "SOKF_FORMAT_75",
            );
        }
    }
    findings
}

fn check_link(
    link: &Link,
    doc: DocView<'_>,
    ctx: &Ctx<'_>,
    findings: &mut Vec<Finding>,
    citations: &mut Citations,
) {
    let path = doc.path;
    let what = if link.is_image { "image" } else { "link" };
    let fail = |findings: &mut Vec<Finding>, msg: String, key: &'static str| {
        findings.push(Finding::error(path.clone(), Some(link.line), key, msg));
    };
    match &link.target {
        LinkTarget::Host(_) => {
            let label = sokf_label(link).unwrap_or_default();
            let parts = Label::of(&link.target).unwrap_or_default();
            match ctx.knowledge {
                None => fail(
                    findings,
                    format!(
                        "`[{label}]` cites a document from a file that is outside any knowledge root"
                    ),
                    "SOKF_FORMAT_61",
                ),
                Some(k) => {
                    let resolved = keys::resolve(ctx.model, ctx.keys, k, parts);
                    // A resolved label from a typed document is a citation
                    // of its target (SOKF_SCHEMA_17).
                    // sokf-implements: EXT-9_AC-2
                    if let Ok(r) = &resolved
                        && let Some(source_type) = doc.core.and_then(|c| c.type_.as_deref())
                    {
                        citations.add(
                            r.path.clone(),
                            Citation {
                                slug: r.slug.clone(),
                                source: path.clone(),
                                source_type: source_type.to_string(),
                                rel: None,
                                line: link.line,
                            },
                        );
                    }
                    // Each target has one spelling: the label with the
                    // shortest store prefix that identifies it.
                    if let Ok(r) = &resolved
                        && let Some(canonical) =
                            ctx.model
                                .canonical_label(k, r.knowledge, parts.id, parts.fragment)
                        && canonical != label
                    {
                        fail(
                            findings,
                            format!(
                                "write `[{label}]` as `[{canonical}]`, the shortest form that identifies the document in this store. `--fix` makes this change"
                            ),
                            "SOKF_FORMAT_65",
                        );
                    }
                    match resolved {
                        Err(u) => fail(findings, u.message(&label), unresolved_key(&u, parts)),
                        // An image embeds a file, so its label must identify a
                        // sidecar, whose asset is that file.
                        Ok(resolved)
                            if link.is_image && ctx.model.asset_of(&resolved.path).is_none() =>
                        {
                            fail(
                                findings,
                                format!(
                                    "image `[{label}]` refers to the document `{}`, which is not a sidecar. An image must refer to an asset through its sidecar's id",
                                    resolved.path
                                ),
                                "SOKF_FORMAT_67",
                            );
                        }
                        Ok(_) => {}
                    }
                }
            }
        }
        LinkTarget::Reference(label) => {
            fail(
                findings,
                format!("reference-style link `[{label}]` matches no definition"),
                "SOKF_FORMAT_74",
            );
        }
        LinkTarget::Repo { path: target, .. } => check_path(target, link, doc, ctx, findings),
        LinkTarget::Relative { target, .. } => match doc.path.resolve_relative(target) {
            Some(resolved) => check_path(&resolved, link, doc, ctx, findings),
            None => fail(
                findings,
                format!("{what} `{target}` points outside the repository"),
                "SOKF_FORMAT_76",
            ),
        },
        LinkTarget::Invalid(url) => fail(
            findings,
            format!("{what} `{url}` points outside the repository"),
            "SOKF_FORMAT_76",
        ),
        // A fragment alone refers to an anchor of this document by its slug.
        LinkTarget::Anchor(fragment) => {
            if let Err(u) = keys::resolve_in(ctx.keys, path, doc.body, fragment) {
                let message = match u {
                    Unresolved::KeyForm(slug) => format!(
                        "`#{fragment}` uses the heading text. Use the slug `#{slug}` instead (`--fix` makes this change)"
                    ),
                    _ => format!("`#{fragment}` matches no heading or key in this document"),
                };
                fail(findings, message, "SOKF_FORMAT_73");
            }
        }
        LinkTarget::Url(_) => {}
    }
}

/// `suffix_clashes` reports each key whose slug equals the suffixed slug
/// of a heading, such as a key `foo-1` beside a second heading `Foo`. The
/// engine compares a key with the plain slug of each heading (TLMD_101),
/// and the suffix is the host's rule, which sokf models (SOKF_FORMAT_155).
// sokf-implements: SOKF_FORMAT_68
fn suffix_clashes(doc: DocView<'_>, ctx: &Ctx<'_>, findings: &mut Vec<Finding>) {
    let suffixed = crate::markdown::suffixed_headings(doc.body);
    if suffixed.is_empty() {
        return;
    }
    for entry in ctx.keys.of_document(doc.path) {
        let key_slug = crate::markdown::slug(&entry.key);
        for (line, slug) in &suffixed {
            if *line != entry.line && *slug == key_slug {
                findings.push(Finding::error(
                    doc.path.clone(),
                    Some(entry.line),
                    "SOKF_FORMAT_68",
                    format!(
                        "key `{}` has the slug `{slug}` of the heading at line {line}; keys and headings share one namespace",
                        entry.key
                    ),
                ));
            }
        }
    }
}

/// `unresolved_key` returns the criterion that a label breaks when it
/// resolves to nothing.
fn unresolved_key(u: &Unresolved, label: Label<'_>) -> &'static str {
    match u {
        Unresolved::NoStore(_) => "SOKF_FORMAT_64",
        Unresolved::NoDocument if label.id.is_none() && label.fragment == Some("") => {
            "SOKF_FORMAT_66"
        }
        Unresolved::NoDocument | Unresolved::DuplicateId => "SOKF_FORMAT_61",
        Unresolved::BareDocumentScope => "SOKF_FORMAT_70",
        Unresolved::KeyForm(_) => "SOKF_FORMAT_72",
        Unresolved::NoFragment | Unresolved::BareNotFound | Unresolved::BareAmbiguous => {
            "SOKF_FORMAT_71"
        }
    }
}

/// `canonical_to` returns the canonical `to` of a `links` entry that refers to
/// a document: its id, with the shortest store prefix that identifies it
/// (SOKF_FORMAT_65).
fn canonical_to(ctx: &Ctx<'_>, citing: usize, target: &Target) -> Option<String> {
    let Target::Document { id, knowledge, .. } = target else {
        return None;
    };
    ctx.model.canonical_include(citing, *knowledge, id, None)
}

/// `check_path` checks a path link or image. A link to a document or an asset
/// of any store must use its canonical `sokf:` label instead, which `--fix`
/// writes when it can (SOKF_FORMAT_60). A link to any other file must point to
/// a file that exists.
fn check_path(
    target: &RepoPath,
    link: &Link,
    doc: DocView<'_>,
    ctx: &Ctx<'_>,
    findings: &mut Vec<Finding>,
) {
    let what = if link.is_image {
        "the image"
    } else {
        "the link"
    };
    let named = ctx
        .knowledge
        .and_then(|k| crate::include::path_link_label(ctx.model, ctx.keys, k, doc.path, link));
    if let Some((label, fixable)) = named {
        let kind = if ctx.model.asset_at(target).is_some() {
            "the asset"
        } else {
            "the document"
        };
        findings.push(Finding::error(
            doc.path.clone(),
            Some(link.line),
            "SOKF_FORMAT_60",
            format!(
                "{what} refers to {kind} `{target}` by path. Refer to it by id as `[text][{label}]`{}",
                if fixable {
                    " (`--fix` makes this change)"
                } else {
                    ""
                }
            ),
        ));
        return;
    }
    // sokf-implements: SOKF_SCHEMA_53
    if !ctx.model.exists(target) && !ctx.model.is_planned(target) {
        findings.push(Finding::error(
            doc.path.clone(),
            Some(link.line),
            "SOKF_FORMAT_76",
            format!("{what} refers to a file that doesn't exist: {target}"),
        ));
    }
}
