//! The store answers questions about the documents that a run doesn't
//! hold, in five kinds of question: the ids in a store, the documents of a
//! type, whether a path is a document, the fields that an index lists, and
//! whether each document's frontmatter parses (CACHE-1_AC-13). The run reads every other
//! fact about a document from the document's file.

use crate::RepoPath;

/// `Fields` holds the fields of a document that an index entry needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fields {
    /// The `id` value.
    pub id: Option<String>,
    /// The `title` value.
    pub title: Option<String>,
    /// The `description` value.
    pub description: Option<String>,
}

/// `SchemaMeta` holds what the store keeps about a schema file that the
/// run doesn't read: enough to dispatch on the schema, match its globs, and
/// identify it. The run reads the file only when a check needs the compiled
/// schema (CACHE-1_AC-25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMeta {
    /// The `id` value.
    pub id: Option<String>,
    /// The `type` const, when the schema declares a type.
    pub type_const: Option<String>,
    /// The `additional-files` globs.
    pub globs: Vec<String>,
    /// The `asset-files` globs.
    pub asset_globs: Vec<String>,
    /// The names, such as `t:<type>` and `f:<format>`, that the schema refers
    /// to in the registry.
    pub uses: Vec<String>,
    /// The line where the body starts. A finding about the whole schema
    /// reports this line.
    pub body_line: usize,
    /// The `archived` value, which says which documents of the type are
    /// archived.
    pub archived: super::Archived,
}

// sokf:begin fallback
/// `Fallback` answers questions about the documents that the run doesn't
/// hold, from the store's rows. A model queries it after searching its own
/// documents, and never for a path that the model holds.
pub trait Fallback: std::fmt::Debug {
    /// `ids` returns the paths of the documents of `knowledge` that have
    /// `id`.
    fn ids(&self, knowledge: usize, id: &str) -> Vec<RepoPath>;

    /// `ids_of_type` returns the path and id of each document of
    /// `knowledge` whose type is `type_name`.
    fn ids_of_type(&self, knowledge: usize, type_name: &str) -> Vec<(RepoPath, String)>;

    /// `is_document` reports whether `path` is a document of any store.
    fn is_document(&self, path: &RepoPath) -> bool;

    /// `fields` returns the fields of the document at `path`.
    fn fields(&self, path: &RepoPath) -> Option<Fields>;

    /// `broken` reports whether the document at `path` has no frontmatter, or
    /// has frontmatter that fails to parse.
    fn broken(&self, path: &RepoPath) -> bool;

    /// `broken_in` returns the paths of the documents of `knowledge` whose
    /// frontmatter is missing or fails to parse.
    fn broken_in(&self, knowledge: usize) -> Vec<RepoPath>;

    /// `schema_meta` returns what the store keeps about the schema file at
    /// `path`.
    fn schema_meta(&self, path: &RepoPath) -> Option<SchemaMeta>;

    /// `scalars_of_type` returns the path and the scalar frontmatter values
    /// of each document of `knowledge` whose type is `type_name`.
    fn scalars_of_type(
        &self,
        knowledge: usize,
        type_name: &str,
    ) -> Vec<(RepoPath, Vec<(String, String)>)>;
}
// sokf:end fallback
