//! The store keeps, between runs, one row of metadata for each document,
//! with the ids that the document declares and the keys that it captures
//! (CACHE-1_AC-13). The row holds what the store knows of a document
//! without reading its file. A check reads every other fact of the
//! document from the file when it needs it.

use std::collections::BTreeSet;

use rusqlite::params;

use super::{Cache, conn::Conn, sync::SyncState};
use crate::{
    Document, RepoPath,
    grammar::KeyScope,
    model::{KeyEntry, KeyIndex},
    source::Fnv,
};

// sokf:begin row
/// `DocRow` is what the store keeps of one document besides its
/// relationships (CACHE-1_AC-13). It holds the fields that an index lists,
/// whether the document is broken, and the hash of the document's parts
/// read by other documents. It holds no text of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRow {
    /// The knowledge store that holds the document.
    pub knowledge: usize,
    /// The document's `id`.
    pub id: Option<String>,
    /// The document's `type`.
    pub type_name: Option<String>,
    /// The document's `title`.
    pub title: Option<String>,
    /// The document's `description`.
    pub description: Option<String>,
    /// Whether the document's frontmatter is missing or doesn't parse.
    /// Such a document has no readable type, so `new` refuses to choose
    /// the next sequence number while the store holds a broken document.
    pub broken: bool,
    /// The hash of the parts of the document that other documents read.
    pub visible: u64,
    /// The row keeps the scalar frontmatter values, as `encode_scalars`
    /// writes them, so that the store knows a document's lifecycle state
    /// without reading its file.
    pub scalars: String,
}
// sokf:end row

impl DocRow {
    /// `of` returns the row of `doc` in the knowledge store `knowledge`.
    // sokf-implements: CACHE-1_AC-13
    pub fn of(knowledge: usize, doc: &Document) -> DocRow {
        DocRow {
            knowledge,
            id: doc.core.id.clone(),
            type_name: doc.core.type_.clone(),
            title: doc.core.title.clone(),
            description: doc.core.description.clone(),
            broken: !doc.core.present,
            visible: doc.visible_hash(),
            scalars: crate::model::archive::encode_scalars(&crate::model::archive::scalars(doc)),
        }
    }

    /// `hash` returns the hash of the row, so that the commit doesn't
    /// rewrite a row that the store already holds.
    pub fn hash(&self) -> u64 {
        let mut h = Fnv::new();
        h.update(&(self.knowledge as u64).to_le_bytes());
        for field in [&self.id, &self.type_name, &self.title, &self.description] {
            match field {
                Some(s) => {
                    h.update(&[1]);
                    h.update(s.as_bytes());
                }
                None => h.update(&[0]),
            }
            h.update(&[0xff]);
        }
        h.update(&[u8::from(self.broken)]);
        h.update(&self.visible.to_le_bytes());
        h.update(self.scalars.as_bytes());
        h.finish()
    }
}

impl Cache {
    /// `record_row` keeps the row of a document that this run read. The
    /// commit writes the row when it differs from the store's.
    pub fn record_row(&self, path: &RepoPath, row: DocRow) {
        self.state.borrow_mut().rows.insert(path.clone(), row);
    }

    /// `old_row` returns the row that the store held for `path` before this
    /// run.
    pub fn old_row(&self, path: &RepoPath) -> Option<DocRow> {
        self.conn()
            .query_row(
                "SELECT store, id, type, title, description, broken, visible_hash, scalars \
                 FROM documents WHERE path = ?1",
                [path.as_str()],
                |r| {
                    Ok(DocRow {
                        knowledge: r.get::<_, i64>(0)? as usize,
                        id: r.get(1)?,
                        type_name: r.get(2)?,
                        title: r.get(3)?,
                        description: r.get(4)?,
                        broken: r.get::<_, i64>(5)? != 0,
                        visible: r.get::<_, i64>(6)? as u64,
                        scalars: r.get(7)?,
                    })
                },
            )
            .ok()
    }

    /// `stored_keys_for` returns the keys that the store holds for the
    /// given slugs, from the documents that cite or capture them. It leaves
    /// out the rows of the documents that this run removed, and the rows of
    /// the documents in `except`. This run captured the keys of those
    /// documents.
    pub fn stored_keys_for(
        &self,
        slugs: &BTreeSet<String>,
        except: &BTreeSet<RepoPath>,
    ) -> KeyIndex {
        let mut index = KeyIndex::default();
        let removed = self.removed_set();
        let Ok(mut stmt) = self.conn().prepare(
            "SELECT path, key, line, heading_slug, scope, knowledge, rule_id FROM keys \
             WHERE slug = ?1 ORDER BY path, line, key",
        ) else {
            return index;
        };
        for slug in slugs {
            let Ok(rows) = stmt.query_map([slug.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            }) else {
                continue;
            };
            for (path, key, line, heading_slug, scope, knowledge, rule_id) in rows.flatten() {
                let Some(path) = RepoPath::parse(&path) else {
                    continue;
                };
                if removed.contains(&path) || except.contains(&path) {
                    continue;
                }
                index.add(KeyEntry {
                    key,
                    path,
                    line: line as usize,
                    heading_slug,
                    scope: match scope.as_str() {
                        "document" => KeyScope::Document,
                        host => KeyScope::Host(host.to_string()),
                    },
                    knowledge: knowledge as usize,
                    rule_id,
                });
            }
        }
        index
    }

    /// `commit_rows` writes the rows and ids of the run inside the commit's
    /// transaction.
    pub(super) fn commit_rows(&self, conn: &Conn, state: &SyncState) -> rusqlite::Result<()> {
        let mut delete_doc = conn.prepare("DELETE FROM documents WHERE path = ?1")?;
        let mut delete_ids = conn.prepare("DELETE FROM ids WHERE path = ?1")?;
        let mut insert_doc = conn.prepare(
            "INSERT OR REPLACE INTO documents (path, store, id, type, title, description, \
             broken, visible_hash, scalars) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut insert_id =
            conn.prepare("INSERT OR IGNORE INTO ids (store, id, path) VALUES (?1, ?2, ?3)")?;
        for path in &state.removed {
            delete_doc.execute([path.as_str()])?;
            delete_ids.execute([path.as_str()])?;
        }
        for (path, row) in &state.rows {
            let known = state
                .known
                .get(path)
                .and_then(|k| k.as_ref())
                .and_then(|k| k.row_hash);
            if known == Some(row.hash()) {
                continue;
            }
            delete_ids.execute([path.as_str()])?;
            insert_doc.execute(params![
                path.as_str(),
                row.knowledge as i64,
                row.id,
                row.type_name,
                row.title,
                row.description,
                i64::from(row.broken),
                row.visible as i64,
                row.scalars,
            ])?;
            if let Some(id) = &row.id {
                insert_id.execute(params![row.knowledge as i64, id, path.as_str()])?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Source, model::ParseMode};

    fn doc(text: &str) -> Document {
        let source = Source::from_text(RepoPath::parse("/k/a.md").unwrap(), text);
        let parsed = crate::model::parse_source(&source, ParseMode::Document);
        Document::new(source, parsed)
    }

    // sokf-tests: CACHE-1_AC-13
    #[test]
    fn a_row_carries_the_fields_and_the_hash_and_no_text() {
        let full = doc("---\ntype: A\nid: a\ntitle: T\ndescription: D\n---\n\n# T\n\nBody.\n");
        let row = DocRow::of(2, &full);
        assert_eq!(row.knowledge, 2);
        assert_eq!(row.id.as_deref(), Some("a"));
        assert_eq!(row.type_name.as_deref(), Some("A"));
        assert_eq!(row.title.as_deref(), Some("T"));
        assert_eq!(row.description.as_deref(), Some("D"));
        assert!(!row.broken);
        assert_eq!(row.visible, full.visible_hash());
        // A body edit changes neither the row nor its hash.
        let edited = doc("---\ntype: A\nid: a\ntitle: T\ndescription: D\n---\n\n# T\n\nMore.\n");
        assert_eq!(DocRow::of(2, &edited), row);
        assert_eq!(DocRow::of(2, &edited).hash(), row.hash());
        // A title edit changes both. A document whose frontmatter is
        // missing or doesn't parse is broken, as it is for `new`.
        let retitled = doc("---\ntype: A\nid: a\ntitle: U\ndescription: D\n---\n\n# T\n");
        assert_ne!(DocRow::of(2, &retitled).hash(), row.hash());
        assert!(DocRow::of(2, &doc("---\n: [\n---\n\n# T\n")).broken);
        assert!(DocRow::of(2, &doc("# No frontmatter\n")).broken);
    }
}
