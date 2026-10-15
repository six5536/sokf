//! SOKF acts as the check engine's host. It uses the model to answer the
//! engine's questions about what lies outside a document.

use tramlines_core::frontmatter::Host as FrontmatterHost;
use tramlines_md::{
    check::{self as engine, CheckOptions, Host, ScaffoldFront, Subject},
    markdown::Dialect,
};

use super::{CheckOutput, Ctx, DocView, current_links, frontmatter as fm_check, link_rules};
use crate::{
    Locator, RepoPath, Severity,
    grammar::{Dispatch, SokfSchema, States},
    markdown::{IncludeTarget, SOKF},
    model::{
        CoreFields,
        frontmatter::{CORE_KEYS, core_fields},
        keys::Label,
    },
    scaffold::SokfScaffold,
};

/// `SokfHost` is the host for one check of a document against `schema`.
/// Its context holds the model and the knowledge that the checked file
/// belongs to, and `core` holds the core fields of the document. The rules
/// of the schema's `links` read `core`. In a scaffold, the host resolves no
/// target outside the document and reports each one as unknown.
pub struct SokfHost<'a> {
    ctx: &'a Ctx<'a>,
    schema: &'a SokfSchema,
    core: Option<&'a CoreFields>,
}

impl<'a> SokfHost<'a> {
    /// `new` returns the host for a check of a document with the core
    /// fields `core` against `schema` in `ctx`.
    pub fn new(ctx: &'a Ctx<'a>, schema: &'a SokfSchema, core: Option<&'a CoreFields>) -> Self {
        SokfHost { ctx, schema, core }
    }
}

impl Host for SokfHost<'_> {
    /// `dialect` spells the engine's markers in SOKF's namespace, so the
    /// engine reports each `sokf:only`, `sokf:guidance`, or `sokf:optional`
    /// marker left in a governed document. A repair leaves a guidance line,
    /// and the part that it guides, as they are.
    // sokf-implements: FORMAT-3_AC-1
    // sokf-implements: FORMAT-3_AC-2
    fn dialect(&self) -> Dialect {
        SOKF
    }

    /// `image` returns the path of the file that a `sokf:<id>` image
    /// displays. For a sidecar, that path is the path of its asset
    /// (FORMAT-2_AC-1). The path is unknown in a scaffold, and also for an
    /// id that resolves to no document.
    // sokf-implements: FORMAT-2_AC-4
    fn image(&self, reference: &str) -> Subject {
        if self.ctx.scaffold {
            return Subject::Unknown;
        }
        let label = Label::parse(reference);
        label
            .id
            .and_then(|id| {
                self.ctx
                    .model
                    .document_in(self.ctx.knowledge?, label.store, id)
            })
            .map_or(Subject::Unknown, |(_, doc)| {
                Subject::Text(self.ctx.model.content_path(&doc.path).to_string())
            })
    }

    /// `region` resolves the target of an include marker. A path stays as
    /// written. A sidecar's id, whole or with a region, resolves to the path
    /// of its asset, with `#<region>` appended when the marker specifies a
    /// region. Any other id resolves to a document. The target is unknown in
    /// a scaffold, and also for an id that resolves to nothing. The generated
    /// check reports an id that resolves to nothing.
    // sokf-implements: EXT-4_AC-1
    fn region(&self, target: &str) -> Subject {
        let IncludeTarget::Document {
            store,
            id,
            fragment,
        } = IncludeTarget::parse(target)
        else {
            return Subject::Text(target.to_string());
        };
        if self.ctx.scaffold {
            return Subject::Unknown;
        }
        let Some((_, doc)) = self
            .ctx
            .knowledge
            .and_then(|k| self.ctx.model.document_in(k, store.as_deref(), &id))
        else {
            return Subject::Unknown;
        };
        match (self.ctx.model.asset_of(&doc.path), fragment.as_deref()) {
            (Some((_, asset)), None) => Subject::Text(asset.path.to_string()),
            (Some((_, asset)), Some(region)) if !region.is_empty() => {
                Subject::Text(format!("{}#{region}", asset.path))
            }
            _ => Subject::Document,
        }
    }

    /// `check` runs the rules of the schema's `links` on the document, after
    /// the engine's rules. A gated `links` rule, one whose `only` limits it
    /// to some lifecycle states, records each entry that it matches. The
    /// engine's check of a scaffold's markers reads those records
    /// (TLMD_174).
    fn check(
        &self,
        doc: &engine::DocView<'_>,
        states: &States,
        _options: CheckOptions,
        locator: &Locator,
        out: &mut CheckOutput,
    ) {
        if let Some(core) = self.core {
            link_rules::check(doc.path, core, self.schema, states, self.ctx, locator, out);
        }
        current_links::check(doc, self.schema, self.ctx, out);
    }
}

impl FrontmatterHost for SokfHost<'_> {
    /// `scalar_format` checks the two formats that SOKF adds. A `repo-path`
    /// value must be a `/`-rooted path to a file in the tree, and a
    /// `document-id` value must be the id of a document in the knowledge.
    /// In a scaffold, `scalar_format` checks only that a `repo-path` value is
    /// `/`-rooted, because a scaffold refers to no real file or document.
    // sokf-implements: EXT-6_AC-1
    fn scalar_format(&self, name: &str, value: &str) -> Option<Result<(), String>> {
        match name {
            "repo-path" => Some(match RepoPath::parse(value) {
                None => Err(format!("is not a /-rooted path: `{value}`")),
                Some(_) if self.ctx.scaffold => Ok(()),
                Some(p) if self.ctx.model.exists(&p) => Ok(()),
                Some(_) => Err(format!("refers to a file that does not exist: `{value}`")),
            }),
            "document-id" if self.ctx.scaffold => None,
            "document-id" => {
                let known = self
                    .ctx
                    .knowledge
                    .and_then(|k| self.ctx.model.document(k, value))
                    .is_some();
                Some(if known {
                    Ok(())
                } else {
                    Err(format!("is not the id of any document: `{value}`"))
                })
            }
            _ => None,
        }
    }

    /// `reserved_keys` returns the core fields of the format specification,
    /// which the document check reads whether or not a schema declares them.
    fn reserved_keys(&self) -> &[&str] {
        CORE_KEYS
    }
}

/// `SchemaHost` is the host for the engine's check of a schema against
/// itself. It writes the frontmatter of each scaffold as `SokfScaffold`
/// does, and it answers every other question as `SokfHost` answers it for
/// a scaffold. A scaffold or an example therefore refers to no real file
/// or document (FORMAT-1_AC-7). Its rules check the core fields of the
/// frontmatter and the schema's `links`, so the engine holds each scaffold
/// and each example to them.
pub struct SchemaHost<'a> {
    check: SokfHost<'a>,
    scaffold: SokfScaffold<'a>,
}

impl<'a> SchemaHost<'a> {
    /// `new` returns the host for the check of `schema`. `ctx` must be a
    /// scaffold's context.
    pub fn new(ctx: &'a Ctx<'a>, schema: &'a SokfSchema) -> Self {
        debug_assert!(ctx.scaffold);
        SchemaHost {
            check: SokfHost::new(ctx, schema, None),
            scaffold: SokfScaffold::new(schema),
        }
    }
}

impl FrontmatterHost for SchemaHost<'_> {
    fn scalar_format(&self, name: &str, value: &str) -> Option<Result<(), String>> {
        self.check.scalar_format(name, value)
    }

    fn reserved_keys(&self) -> &[&str] {
        self.check.reserved_keys()
    }
}

impl Host for SchemaHost<'_> {
    fn dialect(&self) -> Dialect {
        self.check.dialect()
    }

    fn image(&self, reference: &str) -> Subject {
        self.check.image(reference)
    }

    fn region(&self, target: &str) -> Subject {
        self.check.region(target)
    }

    fn scaffold_front(&self, states: &States) -> Option<ScaffoldFront> {
        self.scaffold.scaffold_front(states)
    }

    /// `check` holds a scaffold or an example to the core fields of its
    /// frontmatter and to the schema's `links`, which the engine doesn't
    /// know. The engine reports each error on the schema. An additional file
    /// has no frontmatter, so it has no core fields and no `links`. The
    /// engine alone reports a frontmatter that doesn't parse.
    // sokf-implements: EXT-1_AC-3
    fn check(
        &self,
        doc: &engine::DocView<'_>,
        states: &States,
        _options: CheckOptions,
        locator: &Locator,
        out: &mut CheckOutput,
    ) {
        let Some(frontmatter) = doc.frontmatter else {
            return;
        };
        if matches!(self.check.schema.dispatch, Dispatch::Globs(_)) {
            return;
        }
        let core = core_fields(Some(Ok(frontmatter)), doc.path);
        out.findings.extend(
            fm_check::check_core_fields(&core, doc.path)
                .into_iter()
                .filter(|f| f.severity == Severity::Error),
        );
        link_rules::check(
            doc.path,
            &core,
            self.check.schema,
            states,
            self.check.ctx,
            locator,
            out,
        );
    }
}

// sokf:begin check-against
/// `check_against` checks a document against a schema with the engine,
/// whose host runs the schema's `links` rules as part of the check.
/// `locator` identifies the schema that each finding of a schema rule names.
// sokf-implements: CLI-4_AC-32
pub fn check_against(
    view: DocView<'_>,
    schema: &SokfSchema,
    states: &States,
    ctx: &Ctx<'_>,
    locator: Locator,
) -> CheckOutput {
    let host = SokfHost::new(ctx, schema, view.core);
    let options = CheckOptions {
        scaffold: ctx.scaffold,
    };
    engine::check_against(view.engine(), &schema.core, states, &host, options, locator)
}
// sokf:end check-against
