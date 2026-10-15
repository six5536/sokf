//! `compile_all` checks the schema of every Schema document against the
//! grammar and turns it into the form that the checks use. It does this once
//! per run.
//!
//! Check, Fix, and Scaffold share the compiled schemas.

use super::{FormatRegistry, Grammar, RegexCache, SokfSchema, compile, embedded_formats};
use crate::{Finding, Model, RepoPath, model::SchemaRef};

/// `Compiled` holds the grammar and the compiled schemas of one run.
#[derive(Debug)]
pub struct Compiled {
    /// `grammar` holds the grammar that each schema is checked against.
    pub grammar: Grammar,
    /// `registry` holds the formats that schemas can use. It holds the embedded
    /// formats and the repository's formats, and when two formats share a
    /// name, the repository's format replaces the embedded one.
    pub registry: FormatRegistry,
    /// `regexes` holds each regular expression that the schemas use, so that
    /// each one is compiled only once.
    pub regexes: RegexCache,
    /// `schema_document` is the schema that Schema documents follow, compiled
    /// from the grammar's `schema-document` section.
    pub schema_document: SokfSchema,
    /// `schemas` holds the schema of each Schema document. An entry is `None`
    /// when the schema doesn't conform to the grammar, or when the run hasn't
    /// read the document.
    pub schemas: Vec<(SchemaRef, Option<SokfSchema>)>,
    /// `problems` holds the findings that compiling the schemas produced.
    pub problems: Vec<Finding>,
    /// `done` holds every schema compiled so far, including those that failed.
    done: std::collections::HashSet<SchemaRef>,
}

impl Compiled {
    /// `compile_more` compiles every schema that the model has read since the
    /// last call. Those are the schemas that a check looked up for a
    /// document's type (CACHE-1_AC-25).
    pub fn compile_more(&mut self, model: &Model) {
        for (k, knowledge) in model.knowledges.iter().enumerate() {
            for (i, schema) in knowledge.schemas.iter().enumerate() {
                let r = SchemaRef {
                    knowledge: k,
                    index: i,
                };
                if !schema.loaded || self.done.contains(&r) {
                    continue;
                }
                let compiled = compile_one(
                    schema,
                    &knowledge.root,
                    &self.grammar,
                    &self.registry,
                    &self.regexes,
                    &mut self.problems,
                );
                match self.schemas.iter_mut().find(|(s, _)| *s == r) {
                    Some(slot) => slot.1 = compiled,
                    None => self.schemas.push((r, compiled)),
                }
                self.done.insert(r);
            }
        }
    }
    /// `schema` returns the schema of Schema document `r`, or `None` when the
    /// schema hasn't compiled.
    pub fn schema(&self, r: SchemaRef) -> Option<&SokfSchema> {
        self.schemas
            .iter()
            .find(|(s, _)| *s == r)
            .and_then(|(_, c)| c.as_ref())
    }

    /// `for_type` returns the schema that governs `type_name` in `knowledge`,
    /// when exactly one schema declares the type and that schema compiled. It
    /// records that the caller asked for the type's schema (CACHE-1_AC-25).
    pub fn for_type(
        &self,
        model: &Model,
        knowledge: usize,
        type_name: &str,
    ) -> Option<(SchemaRef, &SokfSchema)> {
        model.note(crate::model::Query::Schema {
            knowledge,
            key: type_name.to_string(),
        });
        let k = model.knowledges.get(knowledge)?;
        match k.schemas_for(type_name) {
            [one] => {
                let r = SchemaRef {
                    knowledge,
                    index: *one,
                };
                self.schema(r).map(|c| (r, c))
            }
            _ => None,
        }
    }

    /// `for_doc` returns the schema that governs the type of `doc` in
    /// `knowledge`, as `for_type` does, or `None` for a document without a
    /// type.
    pub fn for_doc(
        &self,
        model: &Model,
        knowledge: usize,
        doc: &crate::Document,
    ) -> Option<(SchemaRef, &SokfSchema)> {
        doc.type_name()
            .and_then(|t| self.for_type(model, knowledge, t))
    }
}

/// `compile_all` compiles the grammar's `schema-document` schema and every
/// schema that the model has read.
// sokf-implements: SOKF_SCHEMA_15
// sokf-implements: SOKF_SCHEMA_43
pub fn compile_all(
    model: &Model,
    grammar: Grammar,
    repo_formats: Option<FormatRegistry>,
) -> Compiled {
    let regexes = RegexCache::new();
    let mut problems = Vec::new();
    let embedded = embedded_formats(&grammar, &regexes).unwrap_or_default();
    let registry = match &repo_formats {
        Some(repo) => embedded.layered(repo),
        None => embedded,
    };
    // The schema extension is embedded, and a test checks that it has a
    // `schema-document` section.
    let written = grammar
        .section("schema-document")
        .expect("SOKF's schema extension has a `schema-document` section");
    let schema_document = compile(
        written,
        &grammar,
        &registry,
        &regexes,
        &RepoPath::root(),
        "grammar",
    )
    .unwrap_or_else(|findings| {
        problems.extend(findings);
        SokfSchema::of_schema_document(super::Schema::demand_nothing())
    });

    let mut compiled = Compiled {
        grammar,
        registry,
        regexes,
        schema_document,
        schemas: Vec::new(),
        problems,
        done: std::collections::HashSet::new(),
    };
    // Every schema gets an entry, so that each reference to a schema resolves.
    // `compile_more` compiles the schemas that the run has read, and the rest
    // wait for a later call.
    for (k, knowledge) in model.knowledges.iter().enumerate() {
        for i in 0..knowledge.schemas.len() {
            compiled.schemas.push((
                SchemaRef {
                    knowledge: k,
                    index: i,
                },
                None,
            ));
        }
    }
    compiled.compile_more(model);
    compiled
}

/// `compile_one` compiles one Schema document and pushes its problems onto
/// `problems`.
fn compile_one(
    schema: &crate::model::SchemaDocument,
    knowledge_root: &RepoPath,
    grammar: &Grammar,
    registry: &FormatRegistry,
    regexes: &RegexCache,
    problems: &mut Vec<Finding>,
) -> Option<SokfSchema> {
    if let Some(p) = &schema.schema_problem {
        problems.push(p.clone());
    }
    let yaml = schema.schema_yaml.as_ref()?;
    match compile(
        yaml,
        grammar,
        registry,
        regexes,
        &schema.doc.path,
        &schema.id(),
    ) {
        Ok(c) => {
            if let Some(root) = &c.root {
                let inside = RepoPath::parse(root).is_some_and(|r| r.starts_with(knowledge_root));
                if !inside {
                    problems.push(Finding::error(
                        schema.doc.path.clone(),
                        yaml.key_line("root"),
                        "SOKF_SCHEMA_27",
                        format!("`root` {root} lies outside the knowledge {knowledge_root}"),
                    ));
                }
            }
            Some(c)
        }
        Err(findings) => {
            problems.extend(findings);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::{Config, FsLoader, RepoRoot, build, collect, grammar::Dispatch, load_grammar};

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("compile-{name}"));
        fs::create_dir_all(dir.join("knowledge/schemas")).unwrap();
        fs::write(
            dir.join("knowledge/manifest.sokf.yaml"),
            "sokf: \"0.6\"\nname: k\nscope: test\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/note.md"),
            "---\ntype: Schema\nid: schema-note\n---\n\n````yaml\nline-limit: 0\nfrontmatter:\n  type:\n    const: Note\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/bad.md"),
            "---\ntype: Schema\nid: schema-bad\n---\n\n````yaml\nbogus: 1\n````\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/outside.md"),
            "---\ntype: Schema\nid: schema-outside\n---\n\n````yaml\nline-limit: 0\nroot: /elsewhere\nlifecycle:\n  - {key: s, states: [a], path: true}\nfrontmatter:\n  type:\n    const: Issue\nsections: []\n````\n",
        )
        .unwrap();
        fs::write(
            dir.join("knowledge/schemas/none.md"),
            "---\ntype: Schema\nid: schema-none\n---\n\nno fence\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn compiles_every_schema_and_the_grammar_schema_document() {
        let dir = temp("compile");
        let root = RepoRoot::new(dir.clone());
        let tree = collect(&root, &Config::default()).unwrap();
        let model = build(tree, &mut FsLoader::new(&root)).unwrap();
        let compiled = compile_all(&model, load_grammar().unwrap(), None);
        assert_eq!(
            compiled.schema_document.dispatch,
            Dispatch::Type("Schema".into())
        );
        assert!(compiled.schema_document.core.preamble.is_some());
        assert_eq!(compiled.schemas.len(), 4);
        // The schemas sort by path: bad, none, note, outside.
        assert!(
            compiled
                .schema(SchemaRef {
                    knowledge: 0,
                    index: 2
                })
                .is_some()
        );
        assert!(
            compiled
                .schema(SchemaRef {
                    knowledge: 0,
                    index: 0
                })
                .is_none()
        );
        assert!(
            compiled
                .schema(SchemaRef {
                    knowledge: 0,
                    index: 1
                })
                .is_none()
        );
        assert!(compiled.for_type(&model, 0, "Note").is_some());
        assert!(compiled.for_type(&model, 0, "Nope").is_none());
        assert!(compiled.for_type(&model, 1, "Note").is_none());
        let messages: Vec<&str> = compiled
            .problems
            .iter()
            .map(|f| f.message.as_str())
            .collect();
        assert!(
            messages.iter().any(|m| m.contains("unknown key `bogus`")),
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("lies outside the knowledge")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("no YAML fence")),
            "{messages:?}"
        );
        assert!(compiled.registry.get("requirement-ears").is_some());
        fs::remove_dir_all(&dir).unwrap();
    }
}
