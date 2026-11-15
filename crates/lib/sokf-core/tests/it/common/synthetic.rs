//! This module writes a synthetic knowledge store of `n` documents in a
//! given shape, and the same seed always gives the same store. The store
//! holds three types of document. Specs define keyed rules. Notes
//! link to specs, cite their keys, and include one spec each. Memos make up
//! a hundredth of the documents, and no other document reads them, so that
//! a test can measure the cost of an edit to the schema of a small type.

use sokf_core::CacheMode;

use super::TempTree;

/// The shape of a synthetic store.
#[derive(Debug, Clone)]
pub struct Shape {
    /// `docs` is the number of specs and notes: a quarter of them are specs
    /// and the rest are notes. The store also holds one memo for each
    /// hundred documents, and `docs` doesn't count the memos.
    pub docs: usize,
    /// `per_dir` is the number of notes in each directory.
    pub per_dir: usize,
    /// `seed` seeds the choice of the specs that each note links to, cites,
    /// and includes.
    pub seed: u64,
}

/// `Synthetic` describes the store that `write` made. It holds the relative
/// paths of the documents, in the order of writing.
#[derive(Debug, Clone)]
pub struct Synthetic {
    /// `specs` holds the path of each spec: `spec-<n>` is at
    /// `knowledge/specs/spec-<n>.md`.
    pub specs: Vec<String>,
    /// `notes` holds the path of each note: `note-<n>` is at
    /// `knowledge/notes/<dir>/note-<n>.md`.
    pub notes: Vec<String>,
    /// `memos` holds the path of each memo: `memo-<n>` is at
    /// `knowledge/memos/memo-<n>.md`. Memos are a type of their own, and no
    /// other document reads them.
    pub memos: Vec<String>,
    /// `reads` holds, for each note, the indexes into `specs` of the three
    /// specs that the note reads: the two it links to and the one it
    /// includes.
    pub reads: Vec<[usize; 3]>,
}

impl Synthetic {
    /// `readers_of` returns the indexes of the notes that link to or
    /// include spec `s`.
    pub fn readers_of(&self, s: usize) -> Vec<usize> {
        self.reads
            .iter()
            .enumerate()
            .filter(|(_, r)| r.contains(&s))
            .map(|(n, _)| n)
            .collect()
    }

    /// `includers_of` returns the indexes of the notes that include spec
    /// `s`. An edit to the body of spec `s` reaches these notes.
    pub fn includers_of(&self, s: usize) -> Vec<usize> {
        self.reads
            .iter()
            .enumerate()
            .filter(|(_, r)| r[2] == s)
            .map(|(n, _)| n)
            .collect()
    }
}

const SPEC_SCHEMA: &str = r"---
type: Schema
id: schema-spec
title: Spec schema
description: A specification of numbered rules.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Spec
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: What this specification covers.
  - heading: Rules
    level: 2
    guidance: Write one rule per item, each with its key.
    blocks:
      - kind: bullet-list
        each:
          pattern: '^(?P<key>SPEC-\d+_R-\d+): .+$'
          key: key
          unique: knowledge
          example: 'SPEC-1_R-1: the reader SHALL find rule one of spec 1.'
````
";

const NOTE_SCHEMA: &str = r"---
type: Schema
id: schema-note
title: Note schema
description: A note that reads specifications.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Note
links:
  - rel: references
    min: 0
    guidance: Link to the specs that the note reads.
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        repeat: true
        example: Reads rule one of spec 1.
      - kind: include
        repeat: true
        guidance: Include a spec by its id.
        example: |
          <!-- sokf:include spec-1 -->
          <!-- /sokf:include -->
````
";

/// `MEMO_SCHEMA_PATH` is the path of the memo schema, `memo.md`. A test
/// edits this file to measure what an edit to the schema of a small type
/// costs.
pub const MEMO_SCHEMA_PATH: &str = "knowledge/schemas/memo.md";

const MEMO_SCHEMA: &str = r"---
type: Schema
id: schema-memo
title: Memo schema
description: A memo that stands alone.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Memo
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: This memo notes the day's decision.
````
";

/// `Rng` is a deterministic xorshift generator, so the same seed always
/// gives the same store.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("the remainder is below n")
    }
}

/// `write` writes a store of `shape` under `tree`, then validates the store
/// once with `--fix` to fill its generated content, so that the store has no
/// errors.
pub fn write(tree: &TempTree, shape: &Shape) -> Synthetic {
    tree.write(
        "knowledge/manifest.sokf.yaml",
        "sokf: \"0.6\"\nname: synthetic\nscope: test\ndescription: A synthetic store.\n",
    );
    tree.write("knowledge/schemas/spec.md", SPEC_SCHEMA);
    tree.write("knowledge/schemas/note.md", NOTE_SCHEMA);
    tree.write(MEMO_SCHEMA_PATH, MEMO_SCHEMA);
    let spec_count = (shape.docs / 4).max(2);
    let note_count = shape.docs.saturating_sub(spec_count);
    let memo_count = (shape.docs / 100).max(1);
    let mut memos = Vec::with_capacity(memo_count);
    for m in 0..memo_count {
        let rel = format!("knowledge/memos/memo-{m}.md");
        tree.write(&rel, &memo(m));
        memos.push(rel);
    }
    let mut specs = Vec::with_capacity(spec_count);
    for s in 0..spec_count {
        let rel = format!("knowledge/specs/spec-{s}.md");
        tree.write(&rel, &spec(s));
        specs.push(rel);
    }
    let mut rng = Rng(shape.seed.max(1));
    let mut notes = Vec::with_capacity(note_count);
    let mut reads = Vec::with_capacity(note_count);
    for n in 0..note_count {
        let a = rng.below(spec_count);
        let mut b = rng.below(spec_count);
        if b == a {
            b = (a + 1) % spec_count;
        }
        let c = rng.below(spec_count);
        let rel = format!("knowledge/notes/{}/note-{n}.md", n / shape.per_dir.max(1));
        tree.write(&rel, &note(n, a, b, c));
        notes.push(rel);
        reads.push([a, b, c]);
    }
    let report = tree.validate_all(true, CacheMode::Off);
    assert!(
        report.errors.is_empty(),
        "the synthetic store is not clean: {:?}",
        report.errors
    );
    Synthetic {
        specs,
        notes,
        memos,
        reads,
    }
}

fn memo(m: usize) -> String {
    format!(
        "---\ntype: Memo\nid: memo-{m}\ntitle: Memo {m}\ndescription: The memo numbered {m}.\n---\n\n\
         # Memo {m}\n\nMemo {m} notes the day's decision.\n"
    )
}

fn spec(s: usize) -> String {
    format!(
        "---\ntype: Spec\nid: spec-{s}\ntitle: Spec {s}\ndescription: The specification numbered {s}.\n---\n\n\
         # Spec {s}\n\nSpecification {s} covers three rules.\n\n## Rules\n\n\
         - SPEC-{s}_R-1: the reader SHALL find rule one of spec {s}.\n\
         - SPEC-{s}_R-2: the reader SHALL find rule two of spec {s}.\n\
         - SPEC-{s}_R-3: the reader SHALL find rule three of spec {s}.\n"
    )
}

fn note(n: usize, a: usize, b: usize, c: usize) -> String {
    format!(
        "---\ntype: Note\nid: note-{n}\ntitle: Note {n}\ndescription: The note numbered {n}.\n\
         links:\n  - rel: references\n    to: spec-{a}\n  - rel: references\n    to: spec-{b}\n---\n\n\
         # Note {n}\n\nNote {n} reads [SPEC-{a}_R-1][sokf:spec-{a}#spec-{a}_r-1] of [spec {a}][sokf:spec-{a}] and [spec {b}][sokf:spec-{b}].\n\n\
         <!-- sokf:include spec-{c} -->\n<!-- /sokf:include -->\n"
    )
}
