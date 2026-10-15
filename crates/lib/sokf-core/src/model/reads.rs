//! This module records what a check reads from the knowledge outside the
//! document that the check is about. Each read is a query, recorded against
//! the check that makes it. From these records, the next run finds the
//! checks that a change affects (CACHE-1_AC-4).

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use crate::RepoPath;

// sokf:begin query
/// A `Query` is one fact that a check reads from another document, from a
/// file, or from a set of documents, such as every document of a type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Query {
    /// The query reads the documents in a store that have the given id, and
    /// the facts of those documents.
    Doc {
        /// `knowledge` is the index of the store.
        knowledge: usize,
        /// `id` is the id of the documents.
        id: String,
    },
    /// The query reads what is at a path: a document and its facts, a
    /// schema, an index, an asset, or a file.
    Path(RepoPath),
    /// The query reads whether a file or a directory exists at a path.
    Exists(RepoPath),
    /// The query reads the documents of a type in a store.
    Type {
        /// `knowledge` is the index of the store.
        knowledge: usize,
        /// `type_name` is the name of the type.
        type_name: String,
    },
    /// The query reads the keys with a slug at repository scope.
    KeyRepository(String),
    /// The query reads the keys with a slug at knowledge scope, in a store.
    KeyKnowledge {
        /// `knowledge` is the index of the store.
        knowledge: usize,
        /// `slug` is the slug of the keys.
        slug: String,
    },
    /// The query reads the keys with a slug at document scope, in every
    /// document of a store.
    KeyDocument {
        /// `knowledge` is the index of the store.
        knowledge: usize,
        /// `slug` is the slug of the keys.
        slug: String,
    },
    /// The query reads the expanded body of a document, or the text of a
    /// file.
    Body(RepoPath),
    /// The query reads the citations of a document.
    Citations(RepoPath),
    /// The query reads the index that sokf expects a directory to have.
    Directory(RepoPath),
    /// The query reads the schema for a key in a store. The key is the type
    /// in the schema's `type` const. A schema without that const has its
    /// path as the key. Such a schema matches documents only by its globs.
    /// The store keeps a hash of each schema file whose type or path
    /// is the key, so that a change to one of those files changes the answer
    /// to the query.
    Schema {
        /// `knowledge` is the index of the store.
        knowledge: usize,
        /// `key` is the type that the schema's `type` const holds, or, for a
        /// schema without that const, the schema's path.
        key: String,
    },
    /// The query reads whether a document plans a file at a path: whether an
    /// open issue, for example, links the path in its Files section.
    Planned(RepoPath),
    /// The query reads the identities of every store, current and former.
    /// A qualified id, a link to another store, or an include from another
    /// store resolves through them.
    Identities,
}
// sokf:end query

impl Query {
    /// `key` returns the query as one line of text, the form in which the
    /// store keeps it.
    pub fn key(&self) -> String {
        match self {
            Query::Doc { knowledge, id } => format!("doc:{knowledge}:{id}"),
            Query::Path(p) => format!("path:{p}"),
            Query::Exists(p) => format!("exists:{p}"),
            Query::Type {
                knowledge,
                type_name,
            } => format!("type:{knowledge}:{type_name}"),
            Query::KeyRepository(slug) => format!("key:repository:{slug}"),
            Query::KeyKnowledge { knowledge, slug } => format!("key:knowledge:{knowledge}:{slug}"),
            Query::KeyDocument { knowledge, slug } => format!("key:document:{knowledge}:{slug}"),
            Query::Body(p) => format!("body:{p}"),
            Query::Citations(p) => format!("citations:{p}"),
            Query::Directory(p) => format!("dir:{p}"),
            Query::Schema { knowledge, key } => format!("{SCHEMA_PREFIX}{knowledge}:{key}"),
            Query::Planned(p) => format!("planned:{p}"),
            Query::Identities => "identities".to_string(),
        }
    }
}

/// `SCHEMA_PREFIX` begins the key of every schema query, so that when the
/// schema formats change, one range scan finds every reader of any schema
/// (CACHE-1_AC-17).
pub const SCHEMA_PREFIX: &str = "schema:";

/// `Reads` holds the reads of a run: the queries that each reader made.
#[derive(Debug, Default)]
pub struct Reads {
    reader: Option<RepoPath>,
    queries: BTreeMap<RepoPath, BTreeSet<Query>>,
}

impl Reads {
    /// `watch` makes `reader` the reader of every query that follows, and
    /// records `reader` even when it makes no query.
    pub fn watch(&mut self, reader: &RepoPath) {
        self.reader = Some(reader.clone());
        self.queries.entry(reader.clone()).or_default();
    }

    /// `unwatch` clears the reader, so that the queries that follow are not
    /// recorded.
    pub fn unwatch(&mut self) {
        self.reader = None;
    }

    /// `note` records a query against the watched reader, and does nothing
    /// when no reader is watched.
    pub fn note(&mut self, query: Query) {
        if let Some(reader) = &self.reader {
            self.queries
                .entry(reader.clone())
                .or_default()
                .insert(query);
        }
    }

    /// `of` returns the queries of `reader`.
    pub fn of(&self, reader: &RepoPath) -> BTreeSet<Query> {
        self.queries.get(reader).cloned().unwrap_or_default()
    }

    /// `all` returns the queries of every reader.
    pub fn all(&self) -> &BTreeMap<RepoPath, BTreeSet<Query>> {
        &self.queries
    }
}

/// A `Recorder` holds the reads of a model, and every read of the model goes
/// through it.
pub type Recorder = RefCell<Reads>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_has_one_key_and_reads_are_per_reader() {
        let p = RepoPath::parse("/k/a.md").unwrap();
        assert_eq!(
            Query::Doc {
                knowledge: 0,
                id: "x".into()
            }
            .key(),
            "doc:0:x"
        );
        assert_eq!(Query::Path(p.clone()).key(), "path:/k/a.md");
        assert_eq!(Query::Exists(p.clone()).key(), "exists:/k/a.md");
        assert_eq!(
            Query::Type {
                knowledge: 1,
                type_name: "T".into()
            }
            .key(),
            "type:1:T"
        );
        assert_eq!(Query::KeyRepository("s".into()).key(), "key:repository:s");
        assert_eq!(
            Query::KeyKnowledge {
                knowledge: 0,
                slug: "s".into()
            }
            .key(),
            "key:knowledge:0:s"
        );
        assert_eq!(
            Query::KeyDocument {
                knowledge: 0,
                slug: "s".into()
            }
            .key(),
            "key:document:0:s"
        );
        assert_eq!(Query::Body(p.clone()).key(), "body:/k/a.md");
        assert_eq!(Query::Citations(p.clone()).key(), "citations:/k/a.md");
        assert_eq!(Query::Directory(p.clone()).key(), "dir:/k/a.md");
        assert_eq!(
            Query::Schema {
                knowledge: 0,
                key: "Note".into()
            }
            .key(),
            "schema:0:Note"
        );
        assert_eq!(Query::Identities.key(), "identities");
        let mut reads = Reads::default();
        reads.note(Query::Exists(p.clone()));
        assert!(reads.all().is_empty());
        reads.watch(&p);
        reads.note(Query::Exists(p.clone()));
        reads.note(Query::Exists(p.clone()));
        reads.unwatch();
        reads.note(Query::Body(p.clone()));
        assert_eq!(reads.of(&p).len(), 1);
        assert_eq!(reads.all().len(), 1);
    }
}
