//! This module defines the parsed forms of a knowledge's files: documents,
//! schema documents, additional files, assets, and index files.

use serde::{Deserialize, Serialize};

use super::{CoreFields, core_fields};
use crate::{
    Body, Finding, RepoPath, Segments, Source, Spanned,
    markdown::{self, BlockKind},
    source, yaml,
};

/// `ParseMode` says how to parse a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseMode {
    /// `Document` parses a frontmatter block, then a body.
    Document,
    /// `Plain` parses a body only, with no frontmatter.
    Plain,
}

/// `Parsed` holds the result of parsing one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parsed {
    /// The frontmatter, when the file has one and it parses.
    pub frontmatter: Option<Spanned>,
    /// The core fields.
    pub core: CoreFields,
    /// The body.
    pub body: Body,
    /// The markers.
    pub segments: Segments,
    /// The line the body starts on.
    pub body_line: usize,
    /// Whether the file opens with a frontmatter block, parsed or not.
    pub frontmatter_found: bool,
}

/// `parse_source` parses `source` in the given `mode`.
pub fn parse_source(source: &Source, mode: ParseMode) -> Parsed {
    let (frontmatter, text, body_line) = match mode {
        ParseMode::Document => {
            let split = source::split(source);
            (split.frontmatter, split.body, split.body_line)
        }
        ParseMode::Plain => (None, source.text.as_str(), 1),
    };
    let yaml = frontmatter.map(|(text, line)| yaml::parse(text, line));
    // A plain file has no frontmatter to miss, so it has no core fields.
    let core = match mode {
        ParseMode::Document => core_fields(yaml.as_ref().map(|r| r.as_ref()), &source.path),
        ParseMode::Plain => CoreFields::default(),
    };
    let segments = markdown::segments(text, body_line);
    let body = markdown::parse(text, body_line, &segments);
    Parsed {
        frontmatter: yaml.and_then(Result::ok),
        core,
        body,
        segments,
        body_line,
        frontmatter_found: frontmatter.is_some(),
    }
}

// sokf:begin document
/// A `Document` is one parsed document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The file.
    pub path: RepoPath,
    /// The text.
    pub source: Source,
    /// The frontmatter, when it parses.
    pub frontmatter: Option<Spanned>,
    /// The core fields.
    pub core: CoreFields,
    /// The body.
    pub body: Body,
    /// The markers.
    pub segments: Segments,
    /// The line the body starts on.
    pub body_line: usize,
    /// The number of lines in the file.
    pub lines: usize,
    /// Whether the file opens with a frontmatter block, parsed or not.
    pub frontmatter_found: bool,
}
// sokf:end document

/// `Visible` holds only the parts of a document that the checks of other
/// documents read. A change anywhere else, such as to a line number or a code
/// span, therefore leaves the checks of other documents up to date
/// (CACHE-1_AC-5).
#[derive(Serialize)]
struct Visible<'a> {
    id: Option<&'a str>,
    type_: Option<&'a str>,
    title: Option<&'a str>,
    description: Option<&'a str>,
    status: Option<super::Status>,
    resource: Option<&'a super::Resource>,
    headings: Vec<(u8, &'a str, &'a str)>,
    anchors: Vec<&'a str>,
    frontmatter_found: bool,
}

impl Document {
    /// `new` assembles a document from its source and the result of parsing it.
    pub fn new(source: Source, parsed: Parsed) -> Self {
        Document {
            path: source.path.clone(),
            lines: source.line_count(),
            source,
            frontmatter: parsed.frontmatter,
            core: parsed.core,
            body: parsed.body,
            segments: parsed.segments,
            body_line: parsed.body_line,
            frontmatter_found: parsed.frontmatter_found,
        }
    }

    /// `visible_hash` returns the hash of the parts of this document that the
    /// checks of other documents read: its id, type, title, description,
    /// status, resource, headings, anchors, and whether it has a frontmatter
    /// block (CACHE-1_AC-13). The headings include each heading inside a
    /// quote or a list item, because a link can refer to it.
    pub fn visible_hash(&self) -> u64 {
        let headings = crate::markdown::link_headings(&self.body);
        let visible = Visible {
            id: self.core.id.as_deref(),
            type_: self.core.type_.as_deref(),
            title: self.core.title.as_deref(),
            description: self.core.description.as_deref(),
            status: self.core.status,
            resource: self.core.resource.as_ref(),
            headings: headings
                .iter()
                .map(|h| (h.level, h.text.as_str(), h.slug.as_str()))
                .collect(),
            anchors: self.body.anchors.iter().map(|a| a.id.as_str()).collect(),
            frontmatter_found: self.frontmatter_found,
        };
        source::fnv1a64(&serde_json::to_vec(&visible).unwrap_or_default())
    }

    /// `line_count` returns the number of lines in the file.
    pub fn line_count(&self) -> usize {
        self.lines
    }

    /// `body_text` returns the text of the body: everything after the
    /// frontmatter.
    pub fn body_text(&self) -> &str {
        source::split(&self.source).body
    }

    /// `id` returns the document's `id`, if it has one.
    pub fn id(&self) -> Option<&str> {
        self.core.id.as_deref()
    }

    /// `type_name` returns the document's `type`, if it has one.
    pub fn type_name(&self) -> Option<&str> {
        self.core.type_.as_deref()
    }
}

/// A `SchemaDocument` is a schema document together with the schema it holds,
/// parsed as YAML. When the run has not read the schema file, it is instead a
/// placeholder that the run builds from the store's record. The placeholder
/// holds the id, the `type` const, and the globs, but no text. It stays a
/// placeholder until a check needs the compiled schema (CACHE-1_AC-25).
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDocument {
    /// The document, or a placeholder with no text when the run has not
    /// read the file.
    pub doc: Document,
    /// The schema, parsed from the four-backtick YAML fence.
    pub schema_yaml: Option<Spanned>,
    /// Why the schema is absent, when it is.
    pub schema_problem: Option<Finding>,
    /// The `additional-files` globs, read before compilation.
    pub globs: Vec<String>,
    /// The `asset-files` globs, read before compilation.
    pub asset_globs: Vec<String>,
    /// The `type` const, read before compilation.
    pub type_const: Option<String>,
    /// The registry names that the schema's text uses, `t:<type>` and
    /// `f:<format>`. When the formats change, the run uses these names to
    /// find the schemas that the change affects (CACHE-1_AC-17).
    pub uses: Vec<String>,
    /// Whether and how the schema archives its documents, read before
    /// compilation.
    pub archived: super::Archived,
    /// Whether the run read the file. It is false for a placeholder from the
    /// store.
    pub loaded: bool,
}

impl SchemaDocument {
    /// `finding` returns an error about the schema, at the line where its
    /// body starts.
    pub fn finding(&self, criterion: &'static str, message: impl Into<String>) -> Finding {
        Finding::error(
            self.doc.path.clone(),
            Some(self.doc.body_line),
            criterion,
            message,
        )
    }

    /// `placeholder` builds a schema document from the store's record of a
    /// schema file that the run has not read (CACHE-1_AC-25). A check can
    /// use the placeholder only to dispatch on its type, to match globs, and
    /// to read its id.
    pub fn placeholder(path: RepoPath, meta: super::SchemaMeta) -> Self {
        let source = Source::from_text(path, "");
        let mut doc = Document::new(source.clone(), parse_source(&source, ParseMode::Document));
        doc.core.id = meta.id;
        doc.core.type_ = Some("Schema".to_string());
        doc.body_line = meta.body_line;
        SchemaDocument {
            doc,
            schema_yaml: None,
            schema_problem: None,
            globs: meta.globs,
            asset_globs: meta.asset_globs,
            type_const: meta.type_const,
            uses: meta.uses,
            archived: meta.archived,
            loaded: false,
        }
    }

    /// `new` extracts the schema from the YAML fence of a parsed schema
    /// document, and records why the schema is absent when it is.
    pub fn new(doc: Document) -> Self {
        let mut schema_yaml = None;
        let mut schema_problem = None;
        let fence = doc.body.blocks.iter().find(|b| b.kind == BlockKind::Code);
        match fence {
            Some(block) => {
                let text = block.units.first().map(|u| u.text.as_str()).unwrap_or("");
                match yaml::parse(text, block.line + 1) {
                    Ok(y) => schema_yaml = Some(y),
                    Err(e) => {
                        schema_problem = Some(Finding::error(
                            doc.path.clone(),
                            Some(e.line()),
                            "SOKF_SCHEMA_2",
                            format!("schema: {e}"),
                        ));
                    }
                }
            }
            None => {
                schema_problem = Some(Finding::error(
                    doc.path.clone(),
                    Some(doc.body_line),
                    "SOKF_SCHEMA_1",
                    "the Schema document has no schema because it has no YAML fence",
                ));
            }
        }
        let list = |key: &str| {
            schema_yaml
                .as_ref()
                .and_then(|c| c.get(key))
                .and_then(|a| a.as_str_or_list())
                .unwrap_or_default()
        };
        let (globs, asset_globs) = (list("additional-files"), list("asset-files"));
        let type_const = schema_yaml
            .as_ref()
            .and_then(|c| c.get("frontmatter"))
            .and_then(|f| f.get("type"))
            .and_then(|t| t.get("const"))
            .and_then(|c| c.scalar_text());
        let uses = crate::grammar::schema_uses(&doc.source.text);
        let archived = schema_yaml
            .as_ref()
            .map(super::Archived::of_schema)
            .unwrap_or_default();
        SchemaDocument {
            doc,
            schema_yaml,
            schema_problem,
            globs,
            asset_globs,
            type_const,
            uses,
            archived,
            loaded: true,
        }
    }

    /// `id` returns the schema's `id`, or its file name without `.md` when it
    /// has no `id`.
    pub fn id(&self) -> String {
        self.doc.id().map(str::to_string).unwrap_or_else(|| {
            self.doc
                .path
                .file_name()
                .trim_end_matches(".md")
                .to_string()
        })
    }
}

/// A `SchemaRef` identifies a schema by its knowledge and its index in that
/// knowledge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchemaRef {
    /// The knowledge.
    pub knowledge: usize,
    /// The schema's index in the knowledge.
    pub index: usize,
}

/// An `AdditionalFile` is a repository file that a schema governs through
/// its `additional-files` globs.
#[derive(Debug, Clone, PartialEq)]
pub struct AdditionalFile {
    /// The file.
    pub path: RepoPath,
    /// The text.
    pub source: Source,
    /// The body.
    pub body: Body,
    /// The markers.
    pub segments: Segments,
    /// The schema whose glob matches, when exactly one schema's glob does.
    pub schema: Option<SchemaRef>,
}

/// An `Asset` is a file that a schema's `asset-files` glob matches
/// (FORMAT-1_AC-1). The tool never reads its content (FORMAT-1_AC-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// The file.
    pub path: RepoPath,
    /// The schema whose glob matches.
    pub schema: SchemaRef,
    /// The sidecar's index in the knowledge's documents, when the `.md` file
    /// beside the asset exists and the run has read it.
    pub sidecar: Option<usize>,
    /// Whether the `.md` file beside the asset is a document that the run
    /// has read or that the store records.
    pub has_sidecar: bool,
}

impl Asset {
    /// `sidecar_path` returns the path of the asset's sidecar, whether or not
    /// the sidecar exists: the asset's path with `.md` appended.
    pub fn sidecar_path(&self) -> RepoPath {
        sidecar_path(&self.path)
    }
}

/// `sidecar_path` returns the path of the sidecar for an asset at `path`
/// (SOKF_FORMAT_4).
pub fn sidecar_path(path: &RepoPath) -> RepoPath {
    RepoPath::parse(&format!("{path}.md")).expect("a path plus `.md` is a path")
}

/// An `IndexFile` is an `index.md` file, held as text only. The tool
/// generates its whole content (SOKF_FORMAT_97), so the tool does not parse
/// it.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexFile {
    /// The file.
    pub path: RepoPath,
    /// The text.
    pub source: Source,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(text: &str) -> Source {
        Source::from_text(RepoPath::parse("/k/schemas/s.md").unwrap(), text)
    }

    #[test]
    fn parses_documents_and_plain_files() {
        let s = src("---\ntype: A\nid: a\n---\n\n# T\n\n<!-- sokf:links -->\n");
        let p = parse_source(&s, ParseMode::Document);
        assert_eq!(p.core.type_.as_deref(), Some("A"));
        assert_eq!(p.body.headings[0].line, 6);
        assert_eq!(p.body_line, 5);
        assert!(p.segments.definition.is_some());
        let d = Document::new(s.clone(), p);
        assert_eq!(d.line_count(), 8);
        assert_eq!(d.body_text(), "\n# T\n\n<!-- sokf:links -->\n");
        assert_eq!(d.id(), Some("a"));
        assert_eq!(d.type_name(), Some("A"));
        let plain = parse_source(&src("---\nnot: fm\n---\n# H\n"), ParseMode::Plain);
        assert!(plain.frontmatter.is_none());
        assert_eq!(
            plain
                .body
                .headings
                .iter()
                .find(|h| h.text == "H")
                .unwrap()
                .line,
            4
        );
        assert!(!plain.core.present);
        let json = serde_json::to_string(&plain).unwrap();
        let back: Parsed = serde_json::from_str(&json).unwrap();
        assert_eq!(back, plain);
    }

    // sokf-tests: CACHE-1_AC-5
    #[test]
    fn a_line_shift_is_not_visible_and_a_title_is() {
        let text = "---\ntype: A\nid: a\ntitle: T\n---\n\n# T\n\nSee [b][sokf:b].\n";
        let doc = |t: &str| {
            let s = Source::from_text(RepoPath::parse("/k/a.md").unwrap(), t);
            Document::new(s.clone(), parse_source(&s, ParseMode::Document))
        };
        let full = doc(text);
        let shifted = doc(&text.replacen("# T\n", "\n\n# T\n", 1));
        assert_ne!(shifted.body.headings[0].line, full.body.headings[0].line);
        assert_eq!(shifted.visible_hash(), full.visible_hash());
        let renamed = doc(&text.replacen("title: T", "title: U", 1));
        assert_ne!(renamed.visible_hash(), full.visible_hash());
        let reheaded = doc(&text.replacen("# T\n", "# U\n", 1));
        assert_ne!(reheaded.visible_hash(), full.visible_hash());
    }

    #[test]
    fn schemas_extract_their_schema() {
        let s = src(
            "---\ntype: Schema\nid: schema-x\n---\n\n````yaml\nadditional-files: [/README.md]\nasset-files: '/k/media/**/*.png'\nfrontmatter:\n  type:\n    const: X\n````\n",
        );
        let schema = SchemaDocument::new(Document::new(
            s.clone(),
            parse_source(&s, ParseMode::Document),
        ));
        assert_eq!(schema.globs, vec!["/README.md"]);
        assert_eq!(schema.asset_globs, vec!["/k/media/**/*.png"]);
        assert_eq!(schema.type_const.as_deref(), Some("X"));
        assert_eq!(
            sidecar_path(&RepoPath::parse("/k/media/a.png").unwrap()).as_str(),
            "/k/media/a.png.md"
        );
        assert_eq!(
            schema
                .schema_yaml
                .as_ref()
                .unwrap()
                .get("additional-files")
                .unwrap()
                .line,
            7
        );
        assert_eq!(schema.id(), "schema-x");
        assert!(schema.schema_problem.is_none());
        let none = src("---\ntype: Schema\n---\n\nno fence\n");
        let schema = SchemaDocument::new(Document::new(
            none.clone(),
            parse_source(&none, ParseMode::Document),
        ));
        assert!(schema.schema_yaml.is_none());
        assert!(
            schema
                .schema_problem
                .as_ref()
                .unwrap()
                .message
                .contains("no YAML fence")
        );
        assert_eq!(schema.id(), "s");
        let bad = src("---\ntype: Schema\n---\n\n````yaml\na: &x 1\n````\n");
        let schema = SchemaDocument::new(Document::new(
            bad.clone(),
            parse_source(&bad, ParseMode::Document),
        ));
        let problem = schema.schema_problem.unwrap();
        assert_eq!(problem.line, Some(6));
        assert!(problem.message.contains("anchor"));
    }
}
