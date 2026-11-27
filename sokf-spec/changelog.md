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
- [0.7](#07)
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
versions are earlier than 1.0. Nothing reads either number, so a change
to either one bumps its minor version whether it breaks a schema or not,
and the git history records what changed. The grammar is at 0.8. It rose
from 0.7, the grammar of sokf 0.6.0, when a `links` entry's `max` became a
limit with its `when-at-limit`, an `only` became a condition, and a
`cited-by` entry's `keys` could name a `key-of`.

## 0.7

This version changes the schema language, and each item below says what it breaks in a 0.6 schema. A 0.6 document conforms unchanged while its schema declares none of the new keys, such as `labels`.

- Version (§2, validation specification §2): a store that targets this version declares `sokf: "0.7"` in its manifest (SOKF_FORMAT_9). A validator of 0.7 reports a manifest that declares `0.6` (SOKF_VALIDATION_3), so every store raises the version in its manifest.
- Limits (tramlines specifications): every `max` of a schema is a map of a `hard` or a `soft` count and its `when-at-limit`, which tells the writer where the content goes that does not fit (TL_137, TL_138). A soft limit states the usual count and bounds nothing (TL_139). A finding on a count above a hard limit ends with `When at the limit:` and the limit's `when-at-limit` (TL_77). A bare number in a `max` breaks the grammar. To keep the check, write `max: { hard: N, when-at-limit: "…" }`.
- Link limits (schema specification §4, schema extension): the `max` of a `links` entry is a limit with its `when-at-limit` text, as the `max` of a block is. Only a hard limit bounds the links, and its finding quotes the text (SOKF_SCHEMA_33, SOKF_SCHEMA_103). A bare number in the `max` of an entry breaks the grammar. To keep the check, write `max: { hard: N, when-at-limit: "…" }`. A scaffold writes no link of an entry that a hard `max` of 0 bans (schema specification §5).
- Limit names (tramlines-md specification): a rule under `rules` or `lines` that declares a `max` binds a format or declares a `rule-id`, so that the words of its limit name what it counts (TLMD_235). A 0.6 rule with a `max` and neither key breaks. To keep the limit, give the rule a `rule-id`.
- Key sets (schema specification §4, schema extension): the `rule-id` of a unit rule or a line rule only names the rule, and files no key (TLMD_236). Such a rule files its keys under the name in its `key-of` (TLMD_89). The `keys` of a `cited-by` entry names the `rule-id` of a section or a block, or a `key-of` (SOKF_SCHEMA_35). A 0.6 entry whose `keys` names the `rule-id` of a unit rule breaks. To keep it, give the rule a `key-of` of the same name.
- Runs by shape (tramlines-md specification): the validator divides a run of blocks of one kind between the positions of a schema by the text of their units, and never by a `min` or a `max` (TLMD_49, TLMD_74). A 0.6 schema that only a count could divide breaks (TLMD_149).
- Conditions (schema specification §5, schema extension): an `only` is a condition, either a map of dimensions to states or a list of such maps, and a list admits a document that any one of its maps admits (TL_19, TL_135). The `optional` of a section or a block can be a condition, and the part is then optional only in the states that the condition admits (TLMD_229). A `sokf:only` marker joins the maps of a list with ` | `. A 0.6 `only` is one map, and it means what it meant.
- Scaffold opening (tramlines-md specification): a scaffold opens its body with the schema's `guidance` and the messages of its bans, and with no sentence of the engine's own (TLMD_118). A scaffold of a schema without either has no opening line. The change breaks nothing.
- Store language (§2, schema specification §5): a manifest can declare `primary-language`, a BCP 47 tag, and a schema's `language` overrides it (SOKF_FORMAT_251, SOKF_SCHEMA_101). A store without the key is in English, `en`. A validator writes its own texts, such as the rule for a description and the words for a limit, into a scaffold only in the language of the schema (SOKF_SCHEMA_102). A store conforms unchanged.
- Descriptions (§3): `description` is no longer a one-line summary. It is the text that the index shows beside the title, for a reader deciding whether to open the document. §3.1 states the rule for writing a description, which is SOKF's own and holds for every type. A schema gives `guidance` for `description` only where its type needs something that the rule does not say. A store conforms unchanged.
- Attribution (§3.2, schema specification §5): a schema can declare the attribution labels `human` and `agent`, and a unit then carries `[human]` where a human said or confirmed all of it, and `[agent]` otherwise (SOKF_FORMAT_252, SOKF_FORMAT_253). A schema that declares labels declares both and no other (SOKF_SCHEMA_104). The validator words each label in the language of the schema, else as its key, and labels each unit of a scaffold `[agent]` (SOKF_SCHEMA_105 to SOKF_SCHEMA_108). A store whose schemas declare no labels conforms unchanged.
- Kept anchors (§8.5, §9.5): while the text of a unit breaks its rule, the anchor that the unit holds is still an anchor of the document, a kept anchor, and a fragment of a link or a marker that names that anchor resolves (SOKF_FORMAT_71 to SOKF_FORMAT_73, SOKF_FORMAT_145). A store conforms unchanged.
- Replacements (§3, validation specification §2): the warning at a citation of a deprecated document also names each document that a `superseded-by` link of the deprecated document names (SOKF_VALIDATION_65). A citation from such a document gets no warning (SOKF_VALIDATION_66). A `supersedes` link and a `superseded-by` link in the other direction record one replacement, and a warning names it once. The warning changes no error, so a store that cites a deprecated document still conforms.
- Line formats (sokf): sokf supplies no line format of its own, so the layer of formats that the tool embeds is empty. A schema takes each format that it names from its own `formats`, from `/.sokf/formats.yaml`, or from a file of `/.sokf/formats.d/`, such as one that a pack ships. A 0.6 schema that names a format that sokf 0.6 embedded, such as `requirement-ears`, breaks until the store supplies the format.
- No built-in pack (sokf): sokf no longer publishes the pack `@sokf/sokf`, whose schemas, line formats, and standards earlier releases shipped. A store that synced the pack keeps its copies of the files.
- Schema fence (schema specification §2, schema extension): the fence of a Schema document only has to hold its schema, so a formatter that writes the shortest fence that holds the content breaks no Schema document, and the scaffold of a Schema document, which `sokf new` gives as the example of its fence, still writes four backticks (SOKF_SCHEMA_1). The change breaks nothing.
- Fixed spelling (schema specification §4): the validator finds a planning section of `plans-paths` by tramlines' match of a heading to its section, and matches a pattern of `current-links` against the paragraph in tramlines' fixed spelling of its markup, with each run of whitespace read as one space, as a pattern of a schema reads it (SOKF_SCHEMA_109). A 0.6 pattern of `current-links` that names `*x*` or `__x__` as emphasis no longer matches, and is written as `_x_` or `**x**`, and the validator warns about each such run of a pattern, as tramlines warns about a pattern of its own (SOKF_SCHEMA_110).
- Empty fields (§3): a core field other than `type`, `id`, and `generated` whose value is empty, a null or a text of only whitespace, is absent, so `status:` or `resource:` with no value breaks no criterion of a present field, and a schema that requires the field reports it as missing its value (SOKF_FORMAT_256). A 0.6 document whose empty field failed its check now passes it, and an index names a document whose `title` is empty by its id, where it gave the empty title, and writes no ` - ` after a document whose `description` is empty.
- New documents (schema specification §5): a tool creates a new document from the skeleton of its schema, which holds only what is true of the document, such as its id, its title, its states, the headings and the fixed texts that the schema prescribes, and each required frontmatter key with no value, and no example, no guidance, no marker, and no optional part. The tool gives the writer each part's example, the text that the scaffold writes for the part, beside its guidance, and the rule for writing a description stands in that guidance and no longer in a guidance line of a scaffold. The check of an untouched new document reports each part that its writer has still to write, as tramlines 0.6 reports a required frontmatter key with no value as missing (TL_143). A document conforms unchanged.
- Guide (sokf): `sokf guide` prints the guidance that a document's schema gives for its parts at the document's states, with the words that a scaffold holds for each limit and each label. The change breaks nothing.

## 0.6

A 0.5 document conforms unchanged unless a `/`-rooted body link in it ends
with `/` and names a file (SOKF_FORMAT_158), or has an empty, `.`, or
`..` segment (SOKF_FORMAT_159). Schema authors must rewrite their
schemas for the new grammar.

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
- Hand-written indexes (validation specification §3): the fix leaves unchanged an index that holds anything other than a heading, a list of entries, and a definition block, and the document check reports it (SOKF_VALIDATION_56). A directory that a store adopts can hold an `index.md` written as a page, and the fix would otherwise destroy its text.
- Citation messages (schema extension): a `cited-by` entry can declare `message`, one sentence that a validator appends to each finding that the entry raises, after the finding's own text and before its locator. A schema that declares it needs a validator that knows the key.
- Body link types (schema specification §4, schema extension): a unit rule can declare `to-type`, and each `sokf:` link in a unit that the rule accepts must reach a document of one of its types, counted by the document of a link with a fragment (SOKF_SCHEMA_56). A schema that declares it needs a validator that knows the key.
- Schema packs withdrawn (§2): the format no longer defines a schema pack, the directory with the manifest `pack.sokf.yaml` that an earlier amendment of this version added, because a tool of its own now shares the files of a project, and a pack's files land at their paths as any other files of the repository. The version withdraws SOKF_FORMAT_160 to SOKF_FORMAT_163 and SOKF_FORMAT_249. A store conforms unchanged.
- Capped keys (§2, §8.5, §9.5): a manifest can declare `unique: knowledge`, which holds each key of the store unique only within the store, whatever its schema declares, and a marker then reaches a document or a key of the store only with the store before it (SOKF_FORMAT_69, SOKF_FORMAT_205, SOKF_FORMAT_208, SOKF_FORMAT_209). A manifest without `unique` conforms unchanged.
- Inherited schemas (§2, schema specification §4): a manifest can name in `extends` the store whose schemas the store inherits, and a schema of the store's own replaces the inherited schema of its type. An inherited `root` or `asset-files` glob under the root of its store stands for the same place in the inheriting store, and no store inherits a schema of additional files (SOKF_FORMAT_206, SOKF_FORMAT_207, SOKF_SCHEMA_13, SOKF_SCHEMA_96 to SOKF_SCHEMA_99). A manifest without `extends` conforms unchanged.
- Status (§3): the status serves readers and the filters of a tool. `draft` and `stable` change no check.
- Deprecated documents (§3, validation specification §2): the validator warns at each `sokf:` body link, marker, and include block that cites a document with `status: deprecated`, and names each document that declares a `supersedes` link to it (SOKF_VALIDATION_57 to SOKF_VALIDATION_64). A citation from a document that supersedes it, from an archived document, or from a deprecated document gets no warning. The warning changes no error, so a store that cites a deprecated document still conforms.
- Directory links (§9, schema specification §4): a `/`-rooted path that ends with `/` resolves only to a directory, as it does under POSIX, and a link of that form to a file fails (SOKF_FORMAT_157, SOKF_FORMAT_158). A `/`-rooted path with an empty, `.`, or `..` segment is reported as an invalid repository path (SOKF_FORMAT_159). A planning section that links a directory with the trailing `/` plans every path under it (SOKF_SCHEMA_55).
- Lenient markers (format specification §9.5, validation specification §2): a consumer reads the tag of a marker in any case, as the tag in lowercase, and reads a reference between backticks as the reference inside them (SOKF_FORMAT_143, SOKF_FORMAT_144), so that a linter's fix of a comment breaks no marker. A writer still writes the tag in lowercase. The validation specification withdraws SOKF_VALIDATION_52, the failure of a tag in another case. The change accepts every marker that 0.6 accepted before it.
- Assets and markers (schema specification §3, format specification §9.5): the schema specification gives the reason that an asset and its sidecar lie within a knowledge store (SOKF_SCHEMA_8), and says that a file outside every store is the `resource` of an ordinary document. A marker fits the comment syntax of a file that has comments, and a file without comments holds no marker.
- Fenced markers (format specification §9.5): a line inside a fenced code block of a Markdown file outside every store is an example, so a consumer reads no marker and no ignore directive there (SOKF_FORMAT_250). A README can then show a marker in a fence without an ignore region. A store whose fence of a README held a marker that it meant loses that marker, and its coverage reports the key again. An ignore directive inside a fence is an example too, so a region that begins outside a fence and ends inside one never ends.
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
- Templates (§10): the grammar renames a contract's required `examples`
  to `templates`. The validator checks each template against its own
  schema, so a template tests the schema. `sokf new` copies a template
  to create a document, so a template's text is placeholder text. A
  contract can also declare `examples`: complete documents at their own
  lifecycle states, which nothing copies. `sokf new` fills in the
  sequence number of an id template, and the tool specification removes
  `sokf next-id`. Documents conform unchanged. Schema authors rewrite
  their schemas for the new grammar.
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
- Cited by (§10, §11): a schema can declare `cited-by`, the inverse of
  `links`. Each entry demands a body link to every key that a named rule
  captures, or to every document of the type. At least one document of a
  type in `from-type` holds that body link. The entry's `only` limits it to cited documents in the listed states, and the entry has its own `severity`. A citation is a resolved `sokf:` link in another document, in any store, outside included and generated text. A document cites
  a key by the key's slug, and it can cite the key of a heading by the
  heading's slug instead. The validator reports
  an uncited key at its line and an uncited document with no line. The
  change is additive: a schema that declares no `cited-by` conforms
  unchanged.
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

- Rewrite: the specification is rewritten in full, and adds `sokf validate`
  and schemas.

## 0.4

This version breaks 0.3. A body link to a concept addresses it by `id`
(§8), and a document that contains one contains a generated definition
block at its foot (§9). A 0.3 knowledge whose body links point to paths
conforms to 0.3 and not to 0.4. `superdev validate --fix` converts such a
knowledge to 0.4.

- The gloss of `implements` in §8 reads "a plan or issue implementing a
  feature request or contract". Conformance does not change: the
  vocabulary and the inverses stay the same.

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
