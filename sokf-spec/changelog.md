---
type: Changelog
id: changelog
title: SOKF changelog
description: The format's change history, one section per version, newest first.
links:
  - rel: part-of
    to: sokf-format-spec
---

# SOKF changelog

This changelog records the changes to the [format specification][sokf:sokf-format-spec] and to the schema and validation specifications. Version 0.6 split the Superdev Open Knowledge Format (SOKF) specification into the format, schema, and validation specifications. Each § in an entry before 0.6 refers to a section of the retired SOKF specification, `sokf-spec/sokf-spec.md`, which the history of sokf's development holds.

- [Notes](#notes)
- [0.6](#06)
- [0.5](#05)
- [0.4](#04)
- [0.3](#03)
- [0.2](#02)
- [0.1](#01)

## Notes

Before 1.0, a minor version bump can break compatibility. From 1.0, a
minor bump is a backward-compatible addition, and a major bump can break
compatibility. The SOKF knowledge declares the version that it targets
with the manifest's `sokf` key (§2).

The grammar and the shared formats carry versions of their own. Both
versions are earlier than 1.0, and neither has been released. Nothing
reads either number, so a change to either one bumps its minor version
whether it breaks a schema or not, and the git history records what
changed.

## 0.6

Amendments on 18 to 23 September, 27 September, 1 October, and 2 October 2026 changed this version. The
changes are additive for documents: a 0.5 document conforms unchanged.
Schema authors must rewrite their schemas for the new grammar.

- Split: the SOKF specification becomes the format, schema, and
  validation specifications. Each rule is a criterion with a key, and
  each finding states the key of the criterion that it breaks. Line
  formats are not part of SOKF, and a validator can supply its own. This version withdraws the tool specification and the Model Context Protocol (MCP) specification, and the contracts of sokf describe the tool.
- Engine: the [tramlines specifications](https://github.com/six5536/tramlines/tree/main/knowledge/spec)
  state the rules of the schema language, so the schema specification
  withdraws SOKF_SCHEMA_18 to SOKF_SCHEMA_25, 28 to 31, 36, 39 to 41, 45,
  and 47. A finding of the engine states the key of a tramlines criterion,
  such as `TL_56` or `TLMD_152`.
- Planned paths (§4): a schema can declare `plans-paths`, and a document plans each path that a link in such a section names while the document is in the entry's states (SOKF_SCHEMA_52, SOKF_SCHEMA_53). A link, a `resource`, and an include can name a planned path before the file exists.
- Current links (§4): a schema can declare `current-links`, patterns of the paragraphs that make a promise, and such a paragraph links no archived document (SOKF_SCHEMA_54).
- Roots (§6): a schema without a path-enforced dimension can declare a `root`, and its documents lie under it (SOKF_SCHEMA_51), so SOKF_SCHEMA_27 requires a `root` only where a dimension is path-enforced. The fix moves a document that lies outside such a root into it (SOKF_VALIDATION_54).
- Repeated headings (§8.5): each heading has the slug that GitHub gives it, so a later heading with a taken slug takes a suffix, such as `foo-1`, and a fragment with that suffix resolves to it (SOKF_FORMAT_155). A fragment no longer matches more than one heading.
- Lifecycle (§10): a type schema can declare ordered state dimensions,
  each a required frontmatter key. A path-enforced dimension sets the
  document's directory under the schema's `root`, and `--fix` moves a
  misplaced document (§11). A path-enforced dimension qualifies the rule
  of §2 that "paths carry no mandated meaning".
- Id template (§5, §10): a type schema can narrow the slug rule with a
  template of `{seq:N}` and `{slug}` tokens, and `sokf next-id` allocates
  the next sequence number.
- Includes (§9): a dependency graph replaces the no-nesting rule.
  An included file can itself contain includes. The validator copies
  the content of each nested include into the block, so that the block
  contains no include of its own. An include cycle is an error. The
  validator expands an included file before the file that includes it.
  The validator recognises markers only outside fenced code blocks. An
  extraction leaves out the region marker lines, and regions of
  different names can overlap. The include block opens its fence with
  one more backtick than the longest run of backticks in the included
  content. The path form cannot point to a document or an additional
  file. The checks of the including document skip the interior of an
  include. The mapping from the
  extension of an included file to the language tag of its code fence
  moves to the tool specification. A 0.5 block whose region contained other
  markers is stale until `--fix` runs.
- Additional files (§1, §10, §11, §12): the specification defines how
  a schema's glob selects the files that the schema governs. Before 0.6, only the grammar described that rule.
  An additional file is a repository file without frontmatter that a
  schema governs, and it is not a document.
- Line endings (§9, §11): LF (line feed) and CRLF (carriage return and
  line feed) are the same line ending in every comparison, so a CRLF
  file is never stale for that reason alone. `--fix` writes each file
  with its own line ending.
- Validate (tool specification): the tool specification defines the
  options, configuration, and report of `sokf validate`.
- Diff check removed (§1, §4, §5, §11, §12): no tool can tell an
  agent's commit from a human's, so the specification withdraws the
  per-commit check that was meant to enforce `restricted` fields and
  immutable ids. The rules stay as conventions for agents, and reviewers uphold them. A repository conforms when its knowledge
  conforms.
- Clarifications: YAML is the YAML 1.2 core schema without anchors,
  aliases, or tags (§3, §11). Globs follow the gitignore syntax (§9).
  The manifest must declare a version that the validator implements
  (§11). `--fix` leaves a block that it cannot regenerate as it found it
  (§11).
- Citations (§8, §9, §11): a body link can carry a fragment, as in
  `sokf:<id>#<fragment>`. The fragment points to a key that a schema
  defines, or to a heading by its slug. A body link can cite a key bare
  when the key is unique in the repository. The definition block has one entry for each label.
- Slug (§5): the specification states the regular expression once.
- Warnings (§11): a schema rule that declares `severity: warn` raises a
  warning.
- Fix (§11): `--fix` rewrites renamed headings, headings bound to
  frontmatter, and setext headings.

Amended 19 September 2026, before any validator implemented 0.6:

- Formats (§10): a contract's `formats` resolves through three layers:
  the contract, then `/.sokf/formats.yaml`, then the tool's embedded
  registry. A nearer entry replaces a farther entry entirely. The
  specification withdraws the rule that a contract cannot redefine a
  registry name. `sokf info grammar` and `sokf info formats` replace
  `sokf validate --grammar` (tool specification).
- Heading slugs (§8): the validator forms a heading slug as repository
  hosts form it, with underscores kept, so that a `#<slug>` in the
  definition block resolves on the host.
- Manifest (§11): the validator checks the manifest's `name`, as
  §12 already required.
- Core fields (§11): the third step of validation checks the shape of
  every core field that a document has, and not only `verified`.
- Multiple roots (§11): a path link can point to a document of another
  knowledge. To the knowledge that contains the link, that document is a repository file.
- Includes (§9): the specification removes the sentence about
  `sokf:generated-by`, which it never defined.
- Fix (§11): when `--fix` refuses a move because the target exists, the
  refusal is a finding, and the check still runs.

Amended 20 September 2026:

- Indexes (§1, §2, §9, §10, §11, §12): every directory of a knowledge contains an `index.md`, and the validator generates every index. An index
  consists of a heading, one entry for each document and for each
  subdirectory, each addressed by id, and a definition block. The
  validator reports a missing or stale index, and `--fix` writes it. An
  index is the only file that `--fix` creates. An
  `additional-files` glob cannot match an index inside a knowledge. An
  authored index is stale until `--fix` runs. Consumers tolerate a missing
  index.

- Assets (§1, §2, §4, §7, §8, §9, §10, §11, §12): an asset is a
  non-`.md` file that a schema's `asset-files` glob matches. Its
  sidecar is the document beside it whose name is the asset's filename
  followed by `.md`. Every asset has a sidecar, of the type that the
  glob belongs to. The sidecar's `resource` is required and points to
  the asset. Its `verified` lapses when either file changes. `sokf:<id>`
  refers to the content, so a body link or an include that cites a
  sidecar's id resolves to the asset, and `sokf:<id>#` resolves to the
  sidecar. For a bare label, the definition block points to the asset.
  The path form of an include cannot point to an asset. `--fix` moves a sidecar together with
  its asset and rewrites a stale `resource`, which is the only
  frontmatter edit it makes. It never creates a sidecar. Every `.md`
  under a root stays a document. The change is additive: a knowledge
  with no `asset-files` schema conforms unchanged.

Amended 21 September 2026:

- Templates (§10): the grammar renames a contract's required `examples`
  to `templates`. The validator checks each template against its own
  schema, so a template tests the schema. `sokf new` copies a template
  to create a document, so a template's text is placeholder text. A
  contract can also declare `examples`: complete documents at their own
  lifecycle states, which nothing copies. `sokf new` fills in the
  sequence number of an id template, and the tool specification removes
  `sokf next-id`. Documents conform unchanged. Schema authors rewrite
  their schemas for the new grammar.

Amended 22 September 2026:

- Templates (§10): a contract no longer has to declare `templates`. When
  a contract declares none, the validator derives one for each
  combination of lifecycle states from the contract's own rules, marks
  each part that applies only in some states, and checks each derived
  template as it checks a declared one. A rule that the derivation cannot satisfy is an error on
  the schema, and the error identifies that rule. The validator derives no
  text from a pattern, and it writes nothing back into the schema. A
  derived template wraps a part that the contract does not demand in a
  `sokf:optional` region, from `<!-- sokf:optional -->` to
  `<!-- /sokf:optional -->`. The command `sokf new` keeps the region, so that the
  writer of the document decides whether to keep the part. A region left
  in a governed document is a warning, and a marker with no partner is an error. The change is
  additive: a contract that declares its templates behaves as before.
- Report (§4.2, tool specification): a finding that a contract rule
  raises starts with the schema's id and then identifies the rule.
  It identifies the rule by its `rule-id` when the rule declares one,
  and otherwise by its section and block position, its frontmatter key,
  or its `rel`. A reader who follows the finding to the
  schema then finds the rule.

Amended 23 September 2026:

- Templates (§10): a contract declares no `templates`. The validator
  derives every template, and the template's text is the contract's own:
  the text that a rule prescribes and the `example` that each rule has. A
  part that none of these supplies is an error on the schema. The
  contract's `guidance` says what an example cannot, and a derived template includes the guidance as `sokf:guidance` lines before the parts that the lines
  guide. A `sokf:guidance` line in a governed document is an error, and
  `--fix` neither writes nor removes one. Documents conform unchanged.
  Schema authors rewrite their schemas for the new grammar.
- Schemas (§10): a contract declares no `counter-examples`. The
  contract's `guidance` says what a writer must not do where no rule
  forbids it.

Amended 27 September 2026:

- Cited by (§10, §11): a schema can declare `cited-by`, the inverse of
  `links`. Each entry demands a body link to every key that a named rule
  captures, or to every document of the type. At least one document of a
  type in `from-type` holds that body link. The entry's `only` limits it to cited documents in the listed states, and the entry has its own `severity`. A citation is a resolved `sokf:` link in another document, in any store, outside included and generated text. A document cites
  a key by the key's slug, and it can cite the key of a heading by the
  heading's slug instead. The validator reports
  an uncited key at its line and an uncited document with no line. The
  change is additive: a schema that declares no `cited-by` conforms
  unchanged.

Amended 1 October 2026:

- Markers (format §9.5, validation §2): a repository file outside every
  knowledge store can cite a key or a document with a marker. A marker is a comment
  line whose first word is `sokf-<rel>:` followed by one reference, for
  example `// sokf-implements: CLI-1_AC-2`. A bare word in lowercase
  kebab-case is an id, any other bare word is a key unique in the
  repository, and a reference without a store resolves in every store.
  The directives `sokf:ignore-begin`, `sokf:ignore-end`, and
  `sokf:ignore-next-line` make the validator skip text that looks like a
  marker. The validator reads every repository file outside the
  knowledge stores for markers. It skips a file that git ignores or the
  repository excludes, and a file with a NUL byte in its first 8 KB. It warns on a custom
  relationship, reports an error for a marker whose `sokf-<rel>:` word
  has the wrong case, and never rewrites a marker. The change is
  additive: a repository with no marker conforms unchanged.
- Relationships (format §8): `tests`, with the inverse `tested-by`, is a
  core relationship.
- Keys: the keys of the criteria of the three specifications are unique
  in the repository, so that a marker can cite them bare.

Amended 2 October 2026:

- Markers as citations (schema §4, SOKF_SCHEMA_48, SOKF_SCHEMA_49): a
  `cited-by` entry declares exactly one of `from-type` and `from-marker`.
  `from-marker` names a relationship, and only a marker of that
  relationship counts as a citation. A schema can declare `markers`, a list of the relationships that a marker can use to cite the schema's documents and their keys. The validator
  reports a marker of any other relationship as an error at its line, and
  `markers: []` rejects every marker. The change is additive: a schema that declares
  neither conforms unchanged.
- Archived documents (schema §6, SOKF_SCHEMA_50): a lifecycle dimension
  can declare `archived`, the states whose documents are history, and a
  type schema with no lifecycle can declare `archived: true`. A tool
  that shows the knowledge leaves an archived document out unless the
  user asks for it. The change is additive: a schema that declares
  neither conforms unchanged.

## 0.5

An amendment on 18 September 2026 changed this version. It is a full rewrite,
which adds `sokf validate` and schemas.

## 0.4

This version breaks 0.3. A body link to a concept addresses it by `id`
(§8), and a document that contains one contains a generated definition
block at its foot (§9). A 0.3 knowledge whose body links point to paths
conforms to 0.3 and not to 0.4. `superdev validate --fix` converts such a
knowledge to 0.4.

Amended 30 August 2026: the gloss of `implements` in §8 reads "a plan or
issue implementing a feature request or contract". Conformance does not
change: the vocabulary and the inverses stay the same.

## 0.3

This version breaks 0.2 twice.

- Conformance is pass or fail (§11). The specification removes the three-level ladder: a
  knowledge satisfies every rule or it does not conform.
- The format is SOKF, the Superdev Open Knowledge Format, renamed from
  AOKF. The manifest key is `sokf` and the reserved file is
  `manifest.sokf.yaml`. A consumer does not read the 0.2 names `aokf` and
  `manifest.aokf.yaml`.

## 0.2

This version adds `implements` and `implemented-by` to the core
relationship vocabulary (§8), for a plan or issue that delivers a
specification. The change breaks nothing: a 0.1 consumer reads the
unknown `rel` as `relates-to`.

## 0.1

This is the first version of the specification.

<!-- sokf:links -->

[sokf:sokf-format-spec]: /sokf-spec/sokf-format-spec.md
