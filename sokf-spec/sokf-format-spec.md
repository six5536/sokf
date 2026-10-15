---
type: SokfFormatSpecification
id: sokf-format-spec
title: "SOKF: Format specification"
description: The format of a knowledge store, with its manifest, its documents and their frontmatter, its sidecars, its links, and the generated blocks of its files.
version: "0.6"
status: draft
---

# SOKF: Format specification

This specification defines the Superdev Open Knowledge Format (SOKF): what a knowledge store is, and what its files hold.

SOKF is a format for storing canonical knowledge. It combines Markdown files, structured metadata, and the history of a git repository, so that both agents and humans can read and maintain the knowledge. It is a superset of the Open Knowledge Format (OKF) v0.2.

A validator checks the knowledge against schemas. It reports each document that departs from the structure of its schema.

- [About this specification](#about-this-specification)
- [1. Terminology](#1-terminology)
- [2. SOKF knowledge store structure](#2-sokf-knowledge-store-structure)
- [3. Documents](#3-documents)
- [4. Write classes](#4-write-classes)
- [5. Identity](#5-identity)
- [6. Sources](#6-sources)
- [7. Verification](#7-verification)
- [8. Relationships](#8-relationships)
- [9. Paths, generated blocks, and markers](#9-paths-generated-blocks-and-markers)
- [10. Conformance](#10-conformance)
- [11. Versioning](#11-versioning)

## About this specification

Each normative rule of this specification is a criterion in the EARS (Easy Approach to Requirements Syntax) form, with a key of the form `SOKF_FORMAT_n`.

## 1. Terminology

- <a id="knowledge-store"></a>`knowledge store` - A directory tree of documents with a manifest at its root, as this specification describes. It is also called a `knowledge` for short.
- <a id="manifest"></a>`manifest` - The file `manifest.sokf.yaml` at the root of a knowledge store, which describes the store (§2).
- <a id="store-identity"></a>`store identity` - The full name of a store, `<scope>/<name>`, taken from the `scope` and `name` in its manifest (§2).
- <a id="document"></a>`document` - A Markdown file with frontmatter, which holds one unit of knowledge.
- <a id="frontmatter"></a>`frontmatter` - The YAML block between the `---` lines at the top of a document.
- <a id="body"></a>`body` - Everything in a document after its frontmatter.
- <a id="source"></a>`source` - Material that a document is based on, recorded as an entry in `sources` (§6).
- <a id="link"></a>`link` - A typed, directed relationship from one document to another, declared in `links` (§8).
- <a id="body-link"></a>`body link` - A Markdown link in the body that refers to a document by its `id` (§8).
- <a id="schema"></a>`schema` - The structural rules, written in YAML, for the documents of one type or for a set of additional files (the [schema specification][sokf:sokf-schema-spec]).
- <a id="schema-document"></a>`Schema document` - A document of type `Schema` whose body is a single schema. Where the distinction does not matter, "schema" also refers to its Schema document ([schema specification][sokf:sokf-schema-spec]).
- <a id="additional-file"></a>`additional file` - A repository file without frontmatter, such as a README, that a schema governs through a glob ([schema specification][sokf:sokf-schema-spec]).
- <a id="asset"></a>`asset` - A file in a knowledge store, such as an image or a dataset, that matches the `asset-files` glob of a schema ([schema specification][sokf:sokf-schema-spec]).
- <a id="sidecar"></a>`sidecar` - The document that describes an asset. It is next to the asset and is named after it with `.md` appended, such as `flowers.png.md` for `flowers.png` (§2).
- <a id="repository-file"></a>`repository file` - Any file in the repository that is not a document, an index, a manifest, or an asset (§2).
- <a id="index"></a>`index` - The generated `index.md` in each directory of a knowledge store, which lists the contents of the directory (§9).
- <a id="label"></a>`label` - The reference of a body link, `sokf:<id>`, optionally with a store before the id and a fragment after it (§8).
- <a id="fragment"></a>`fragment` - The part of a reference after `#`, which identifies an anchor in the target (§8).
- <a id="slug"></a>`slug` - The id of the anchor of a heading or a key, derived from its text by the rules in §8.5.
- <a id="canonical-form"></a>`canonical form` - The shortest way of writing a reference that still identifies its target (§8).
- <a id="criterion"></a>`criterion` - A normative rule, written in the EARS form. Each criterion starts with its key, then a tag in brackets that names its EARS pattern, then the rule. The tag `[ubiquitous]` marks a rule that always holds. The tag `[event]` marks a rule that holds when the event in its `WHEN` clause happens. The tag `[unwanted]` marks a rule for the unwanted situation in its `IF … THEN` clause.
- <a id="key"></a>`key` - An identifier that a schema rule captures from part of a document, such as `SOKF_FORMAT_1` ([schema specification][sokf:sokf-schema-spec]).
- <a id="anchor"></a>`anchor` - The generated empty HTML link that marks the position of a key, `<a id="<slug>"></a>` (§9).
- <a id="definition-block"></a>`definition block` - The generated reference definitions at the end of a file, which give the path for each `sokf:` label (§9).
- <a id="include-block"></a>`include block` - A pair of markers that enclose a generated copy of the body of a document, of an asset, or of a region of a file (§9).
- <a id="region"></a>`region` - A named part of a file, between a `sokf:begin` line and a `sokf:end` line (§9.6).
- <a id="fenced-code-block"></a>`fenced code block` - A code block that opens and closes with a code fence, as CommonMark defines it.
- <a id="marker"></a>`marker` - A comment line in a repository file outside every knowledge store that cites one key or one document with a relationship, such as `// sokf-implements: CLI-1_AC-2` (§9.5).
- <a id="trust-tier"></a>`trust tier` - How well confirmed a document is, based on its current `verified` entries (§7).
- <a id="actor"></a>`actor` - A string that identifies who did something: `human:<id>` for a human, `process:<id>` for a process, or `<producer>/<version>` for an agent or a tool (§7).
- <a id="role"></a>`role` - A party that a criterion applies to: an agent, a human, a writer, a process, a consumer, or the validator.
- <a id="validator"></a>`validator` - The deterministic tool that checks a knowledge store and its repository against the SOKF specifications ([validation specification][sokf:sokf-validation-spec]).
- <a id="fix"></a>`fix` - The automatic repair of mechanical errors that the validator runs before it checks the documents, such as rewriting references and generating include blocks, anchors, and indexes ([validation specification][sokf:sokf-validation-spec]).
- <a id="agent"></a>`agent` - A writer driven by a language model.
- <a id="human"></a>`human` - A person who writes or reviews the knowledge.
- <a id="writer"></a>`writer` - An agent or a human that writes documents.
- <a id="process"></a>`process` - A deterministic program that writes to the knowledge, such as a link checker.
- <a id="consumer"></a>`consumer` - A program that reads a knowledge store to display, search, or export it, or to pass it to an agent.

## 2. SOKF knowledge store structure

A knowledge store is a directory in the repository, such as `/knowledge`, and its writers organise the documents into subdirectories as they choose. The path of a document matters only where a schema declares a lifecycle: the set of states that the documents of its type move through. In that case the schema can map each state to the directory that holds the documents in that state ([schema specification][sokf:sokf-schema-spec]).

```text
<knowledge>/
  manifest.sokf.yaml  # The manifest.
  index.md            # The generated index (§9).
  schemas/            # The Schema documents.
    index.md
    <type>.md
  <document>.md
  <subdirectory>/
    index.md
    <document>.md
    <asset>           # A file that an asset-files glob matches.
    <asset>.md        # Its sidecar, a document.
```

A knowledge store reserves these files and directories:

- `manifest.sokf.yaml` - The manifest, at the root of the knowledge store.
- `index.md` - The index, in every directory of the knowledge store (§9).
- `schemas/` - The directory that holds the Schema documents, at the root of the knowledge store ([schema specification][sokf:sokf-schema-spec]).

Every other `.md` file in a knowledge store is a document. A file that is not a `.md` file is an asset when it matches the `asset-files` glob of exactly one schema ([schema specification][sokf:sokf-schema-spec]). The sidecar of an asset is the `.md` file next to it, named after the asset with `.md` appended, and the two filenames are the only connection between them. Any other file in a knowledge store is a repository file.

- <a id="sokf_format_1"></a>SOKF_FORMAT_1 [ubiquitous]: the knowledge store SHALL hold a manifest at its root.
- <a id="sokf_format_2"></a>SOKF_FORMAT_2 [ubiquitous]: the knowledge store SHALL keep every Schema document in the `schemas/` directory at its root.
- <a id="sokf_format_3"></a>SOKF_FORMAT_3 [ubiquitous]: the knowledge store SHALL hold nothing in `schemas/` except Schema documents and the index.
- <a id="sokf_format_119"></a>SOKF_FORMAT_119 [ubiquitous]: the validator SHALL treat every `.md` file in a knowledge store as a document, except indexes and the files in `schemas/`.
- <a id="sokf_format_120"></a>SOKF_FORMAT_120 [ubiquitous]: the validator SHALL treat every `.md` file in `schemas/`, except its index, as a Schema document.
- <a id="sokf_format_4"></a>SOKF_FORMAT_4 [ubiquitous]: the asset SHALL have a sidecar next to it, named after the asset with `.md` appended.
- <a id="sokf_format_5"></a>SOKF_FORMAT_5 [event]: WHEN a writer copies a Markdown file into a knowledge store, the writer SHALL add frontmatter to the file.
- <a id="sokf_format_6"></a>SOKF_FORMAT_6 [event]: WHEN a writer copies a Markdown file into a knowledge store, the writer SHALL record where the file came from in `sources` (§6).
- <a id="sokf_format_7"></a>SOKF_FORMAT_7 [ubiquitous]: the writer SHALL keep no change log of the documents in the knowledge store.

The manifest describes the knowledge store:

```yaml
sokf: "0.6"
name: example-knowledge
scope: example-org
description: Knowledge for the example project.
renamed-from: [example-org/old-knowledge, other-org/example-knowledge]
```

- `sokf` - The version of this specification that the knowledge store targets.
- `name` - The name of the store among the stores of its owner.
- `scope` - The owner of the store, such as an npm scope, a Maven group, or a GitHub owner.
- `description` - A summary of the store.
- `renamed-from` - The former identities of the store.

The store identity is `<scope>/<name>`, and documents in other stores use it to refer to this store (§8). A reference that uses a former identity still resolves, and the fix rewrites it to the current identity ([validation specification][sokf:sokf-validation-spec]).

- <a id="sokf_format_8"></a>SOKF_FORMAT_8 [ubiquitous]: the manifest SHALL parse as YAML 1.2 with the core schema, and use no anchors, aliases, or tags.
- <a id="sokf_format_121"></a>SOKF_FORMAT_121 [ubiquitous]: the manifest SHALL contain UTF-8 text that forms a single YAML map.
- <a id="sokf_format_9"></a>SOKF_FORMAT_9 [ubiquitous]: the manifest SHALL declare the `sokf` version that the knowledge store targets.
- <a id="sokf_format_10"></a>SOKF_FORMAT_10 [ubiquitous]: the manifest SHALL declare a slug `name` and a slug `scope`.
- <a id="sokf_format_11"></a>SOKF_FORMAT_11 [ubiquitous]: the manifest SHALL hold each entry of `renamed-from` in the form `<scope>/<name>`.
- <a id="sokf_format_12"></a>SOKF_FORMAT_12 [ubiquitous]: the store identity SHALL differ from every current and former identity of the other knowledge stores in the repository.
- <a id="sokf_format_13"></a>SOKF_FORMAT_13 [ubiquitous]: the manifest SHALL list under `renamed-from` only identities that no other manifest in the repository lists.
- <a id="sokf_format_14"></a>SOKF_FORMAT_14 [event]: WHEN a human changes a store identity, the human SHALL add the former identity to `renamed-from`.
- <a id="sokf_format_15"></a>SOKF_FORMAT_15 [ubiquitous]: the agent SHALL leave the `name` and `scope` of the manifest unchanged.
- <a id="sokf_format_16"></a>SOKF_FORMAT_16 [ubiquitous]: the manifest SHALL contain no `producer`, `generated`, or `counts` key in the working tree (§4).

## 3. Documents

A document is a UTF-8 Markdown file: a YAML frontmatter block followed by a body in standard Markdown. The schema of the document defines the sections of the body ([schema specification][sokf:sokf-schema-spec]), and the rest of the body is free-form. A footnote attributes a claim to a source (§6).

The frontmatter of a document can hold these core fields:

- `type` - The kind of document.
- `id` - The identity of the document (§5).
- `title` - The name to display. When it is absent, a consumer can display the filename.
- `description` - A one-line summary, for indexes and previews.
- `tags` - Short labels that group documents across directories.
- `resource` - The `/` path or URL of what the document describes, or a list of them. An abstract document, which describes no file and no URL, omits it.
- `status` - `draft`, `stable`, or `deprecated`. When it is absent, the status is `stable`.
- `sources` - The sources of the document (§6).
- `links` - The links of the document (§8).
- `verified` - The confirmations of the document (§7).
- `generated` - `{ by, at }`, taken from the git history at export.

Each core field has a write class (§4). The following document has most of the core fields:

```markdown
---
type: Subsystem
id: subsystem-logging
title: Logging Subsystem
description: Requirements for the logging sub-system.
resource: /crates/lib/example-core/src/logging.rs
tags: [core, logging]
sources:
  - id: log4rs
    resource: https://crates.io/log4rs
    title: log4rs documentation
links:
  - rel: depends-on
    to: config
    note: Reads logging configuration and security policy.
---

# Logging

The logging system reads [config][sokf:config] and emits a
set of logs.[^log4rs]

[^log4rs]: log4rs documentation

<!-- sokf:links -->

[sokf:config]: /knowledge/config.md
```

- <a id="sokf_format_17"></a>SOKF_FORMAT_17 [ubiquitous]: the document SHALL contain UTF-8 text that begins with its frontmatter.
- <a id="sokf_format_18"></a>SOKF_FORMAT_18 [ubiquitous]: the frontmatter SHALL parse as YAML 1.2 with the core schema, and use no anchors, aliases, or tags.
- <a id="sokf_format_122"></a>SOKF_FORMAT_122 [ubiquitous]: the frontmatter SHALL form a single YAML map.
- <a id="sokf_format_19"></a>SOKF_FORMAT_19 [ubiquitous]: the frontmatter SHALL include a `type` that is a non-empty string.
- <a id="sokf_format_20"></a>SOKF_FORMAT_20 [event]: WHEN a document covers a file outside the knowledge store, the writer SHALL summarise the file rather than copy it.
- <a id="sokf_format_21"></a>SOKF_FORMAT_21 [event]: WHEN a document covers a file outside the knowledge store, the writer SHALL cite the file in `sources` (§6).
- <a id="sokf_format_22"></a>SOKF_FORMAT_22 [ubiquitous]: the writer SHALL keep each piece of knowledge in exactly one place in the knowledge store.
- <a id="sokf_format_26"></a>SOKF_FORMAT_26 [ubiquitous]: the frontmatter SHALL give `title` and `description`, when present, as strings.
- <a id="sokf_format_27"></a>SOKF_FORMAT_27 [ubiquitous]: the frontmatter SHALL give `tags`, when present, as a list of strings.
- <a id="sokf_format_123"></a>SOKF_FORMAT_123 [ubiquitous]: the frontmatter SHALL give `sources` and `links`, when present, as lists of maps.
- <a id="sokf_format_28"></a>SOKF_FORMAT_28 [ubiquitous]: the frontmatter SHALL give `status`, when present, as `draft`, `stable`, or `deprecated`.
- <a id="sokf_format_29"></a>SOKF_FORMAT_29 [ubiquitous]: the frontmatter SHALL give `resource`, when present, as a URL, a `/` path, or a non-empty list of these.
- <a id="sokf_format_30"></a>SOKF_FORMAT_30 [ubiquitous]: the frontmatter SHALL make each `/` path in `resource` resolve to a file or directory in the repository.
- <a id="sokf_format_31"></a>SOKF_FORMAT_31 [ubiquitous]: the sidecar SHALL give `resource` as a single string: the `/` path of its asset.

## 4. Write classes

Every frontmatter field and every manifest key has a write class, which determines who writes it:

- `open` - Humans, agents, and tools write it.
- `restricted` - Only humans and processes write it.
- `stamped` - Export tools write it at export time.

Each core field (§3) is `open`, except `verified`, which is `restricted`, and `generated`, which is `stamped`.

An extension key is any frontmatter key that is not a core field. Either the schema of the type declares it, or the schema allows it through open frontmatter ([schema specification][sokf:sokf-schema-spec]). Extension keys are `open` unless the project assigns another class. A project makes an extension key `restricted` or `stamped` when its value records a fact that no check can confirm.

The validator enforces the `stamped` class. It cannot check that agents leave `restricted` fields alone, so a human reviewer enforces the `restricted` class.

- <a id="sokf_format_23"></a>SOKF_FORMAT_23 [ubiquitous]: the agent SHALL leave every `restricted` field of a document or manifest exactly as it is, byte for byte, and add none.
- <a id="sokf_format_24"></a>SOKF_FORMAT_24 [ubiquitous]: the document SHALL contain no `stamped` field in the working tree.
- <a id="sokf_format_25"></a>SOKF_FORMAT_25 [ubiquitous]: the writer SHALL add no frontmatter field that records the author, the time of a change, or earlier content of a document.

## 5. Identity

- <a id="sokf_format_32"></a>SOKF_FORMAT_32 [ubiquitous]: the document SHALL have an `id` that is a slug: lowercase words joined by `-`, matching `^[a-z0-9]+(-[a-z0-9]+)*$`.
- <a id="sokf_format_33"></a>SOKF_FORMAT_33 [ubiquitous]: the document SHALL have an `id` that no other document in its knowledge store has.
- <a id="sokf_format_34"></a>SOKF_FORMAT_34 [event]: WHEN a writer renames or moves a document, the writer SHALL keep its `id` unchanged.
- <a id="sokf_format_35"></a>SOKF_FORMAT_35 [ubiquitous]: the agent SHALL never change the `id` of a document that has one.
- <a id="sokf_format_141"></a>SOKF_FORMAT_141 [ubiquitous]: the process SHALL never change the `id` of a document that has one.
- <a id="sokf_format_36"></a>SOKF_FORMAT_36 [event]: WHEN a human changes the `id` of a document, the human SHALL update every reference to the old `id`.

## 6. Sources

The `sources` of a document record the material it is based on.

```yaml
sources:
  - id: planner-src
    resource: /crates/lib/example-core/src/planner.rs
    title: Planner source
  - id: clap-docs
    resource: https://docs.rs/clap/latest/clap/
    title: clap documentation
```

Each source has these keys:

- `resource` - A path from the repository root, or a URL.
- `id` - The footnote label for the source. It is local to its document and unrelated to document ids (§5).
- `title` - A label to display.

A footnote attributes a claim to a source:

```markdown
The planner never writes to the filesystem.[^planner-src]

[^planner-src]: Planner source
```

- <a id="sokf_format_37"></a>SOKF_FORMAT_37 [ubiquitous]: the source SHALL have a `resource` that is either a URL or a `/` path to a file or directory in the repository.
- <a id="sokf_format_38"></a>SOKF_FORMAT_38 [ubiquitous]: the source SHALL have no keys other than `resource`, `id`, and `title`.
- <a id="sokf_format_124"></a>SOKF_FORMAT_124 [ubiquitous]: the source SHALL give `id` and `title`, when present, as strings.
- <a id="sokf_format_39"></a>SOKF_FORMAT_39 [event]: WHEN the body cites a source, the source SHALL have an `id`.
- <a id="sokf_format_40"></a>SOKF_FORMAT_40 [event]: WHEN a writer attributes a claim to a source, the writer SHALL use a Markdown footnote labelled with the `id` of the source.
- <a id="sokf_format_41"></a>SOKF_FORMAT_41 [ubiquitous]: the document SHALL contain only footnotes whose label is the `id` of one of its sources.

## 7. Verification

The `verified` key of a document records who has confirmed that its content matches what the document describes. Each entry is one confirmation.

Each entry has these keys:

- `by` - The actor who confirmed the content.
- `at` - When the content was confirmed: an ISO 8601 date, or date and time.

```yaml
verified:
  - { by: human:rsewell, at: 2026-08-04T09:00:00Z }
  - { by: process:link-checker, at: 2026-08-04T02:00:00Z }
```

- <a id="sokf_format_42"></a>SOKF_FORMAT_42 [ubiquitous]: the document SHALL give `verified` as a list of entries, or as a single entry.
- <a id="sokf_format_43"></a>SOKF_FORMAT_43 [ubiquitous]: the document SHALL give each entry of `verified` a `by` that is a `human:` or `process:` actor.
- <a id="sokf_format_44"></a>SOKF_FORMAT_44 [ubiquitous]: the document SHALL give each entry of `verified` an `at` that is an ISO 8601 date, or date and time.
- <a id="sokf_format_45"></a>SOKF_FORMAT_45 [ubiquitous]: the human SHALL add to `verified` only entries whose `by` identifies that human.
- <a id="sokf_format_46"></a>SOKF_FORMAT_46 [ubiquitous]: the process SHALL add to `verified` only entries whose `by` identifies that process.
- <a id="sokf_format_47"></a>SOKF_FORMAT_47 [ubiquitous]: the agent SHALL leave the `verified` key of every document exactly as it is, byte for byte, and add none.

The trust tier of a document derives from its current entries. The tiers, from lowest to highest, are:

- `unverified` - No current entries.
- `machine-confirmed` - Current entries from `process:` actors only.
- `human-reviewed` - At least one current entry from a `human:` actor.

An entry is current until it lapses. It lapses when the content of its file changes in git after its `at`, and an entry on a sidecar also lapses when the asset changes after its `at`. A consumer reads the git history of the file to tell whether an entry has lapsed.

- <a id="sokf_format_48"></a>SOKF_FORMAT_48 [ubiquitous]: the consumer SHALL derive the trust tier of a document from its current entries.
- <a id="sokf_format_49"></a>SOKF_FORMAT_49 [ubiquitous]: the consumer SHALL read documents at every trust tier.

## 8. Relationships

The `links` key of a document lists its links. Each link is a map with these keys:

- `rel` - The type of relationship.
- `to` - The target, as an `id` or a `/` path. For a document in another store, the store comes before the `id` (§8.2).
- `note` - A one-line explanation of the link.

A link is a directed edge from its document to the target in `to`. A path to a document refers to that document, and the canonical form of a target is its `id`.

- <a id="sokf_format_50"></a>SOKF_FORMAT_50 [ubiquitous]: the link SHALL have a string `rel` and a string `to`.
- <a id="sokf_format_51"></a>SOKF_FORMAT_51 [ubiquitous]: the link SHALL have no keys other than `rel`, `to`, and a string `note`.
- <a id="sokf_format_52"></a>SOKF_FORMAT_52 [ubiquitous]: the link SHALL have a `rel` in lowercase kebab-case.
- <a id="sokf_format_53"></a>SOKF_FORMAT_53 [ubiquitous]: the link SHALL have a `to` that resolves to a document or to a file in the repository.
- <a id="sokf_format_54"></a>SOKF_FORMAT_54 [ubiquitous]: the consumer SHALL resolve a `to` as an `id` first, and as a path only when no `id` matches.

These are the core relationships, each with its inverse:

- `relates-to` - Inverse `relates-to`. The document is related to the target.
- `part-of` - Inverse `has-part`. The document is part of the target.
- `depends-on` - Inverse `depended-on-by`. The document needs the target to work.
- `references` - Inverse `referenced-by`. The document cites the target.
- `supersedes` - Inverse `superseded-by`. The document replaces the target, which is deprecated.
- `implements` - Inverse `implemented-by`. The document delivers the target, as an issue delivers a contract.
- `tests` - Inverse `tested-by`. The document or file checks that the target holds, as a test checks a criterion.
- `contradicts` - Inverse `contradicts`. The document conflicts with the target, and the text of the document explains how to resolve the conflict.

A custom relationship is a lowercase kebab-case word outside the core set. Writers use a core relationship where one fits, and a custom one where none does. The validator warns about each custom relationship ([validation specification][sokf:sokf-validation-spec]).

- <a id="sokf_format_55"></a>SOKF_FORMAT_55 [ubiquitous]: the consumer SHALL record a `rel` outside the core set on the edge as written, but give the edge the meaning and the inverse of `relates-to`.
- <a id="sokf_format_56"></a>SOKF_FORMAT_56 [ubiquitous]: the writer SHALL record what a document is derived from in `sources` (§6), not with a `derived-from` relationship.
- <a id="sokf_format_57"></a>SOKF_FORMAT_57 [ubiquitous]: the consumer SHALL treat each source whose `resource` is a `/` path as a `derives-from` edge, with the inverse `derived-by`.
- <a id="sokf_format_58"></a>SOKF_FORMAT_58 [ubiquitous]: the writer SHALL declare each edge only once, from one side.
- <a id="sokf_format_59"></a>SOKF_FORMAT_59 [event]: WHEN a consumer builds a graph, the consumer SHALL add the inverse of every edge.
- <a id="sokf_format_113"></a>SOKF_FORMAT_113 [event]: WHEN the target of an edge declares the inverse edge itself, the consumer SHALL use that declaration as the inverse, and add no other.
- <a id="sokf_format_114"></a>SOKF_FORMAT_114 [ubiquitous]: the consumer SHALL treat two edges with identical fields as one edge.
- <a id="sokf_format_115"></a>SOKF_FORMAT_115 [ubiquitous]: the consumer SHALL treat each `/` path in `resource` as a `describes` edge, with the inverse `described-by`.
- <a id="sokf_format_116"></a>SOKF_FORMAT_116 [ubiquitous]: the consumer SHALL treat each include marker as an `includes` edge from the including file to the included document or file, with the inverse `included-by`.
- <a id="sokf_format_117"></a>SOKF_FORMAT_117 [ubiquitous]: the consumer SHALL derive no edge from a body link to a path, or from an index entry.

### 8.1 Body links

A body link refers to a document by its `id`. It is a reference-style Markdown link whose label is `sokf:` followed by the id:

```markdown
The planner reads [config][sokf:config] before it plans.
```

Links to other files, such as source files or a README, use the path or URL of the file.

- <a id="sokf_format_60"></a>SOKF_FORMAT_60 [ubiquitous]: the body SHALL use a `sokf:` label to link to, or embed, any document or asset.
- <a id="sokf_format_61"></a>SOKF_FORMAT_61 [ubiquitous]: the body link SHALL resolve to a document.
- <a id="sokf_format_62"></a>SOKF_FORMAT_62 [unwanted]: IF a body link resolves to no document, THEN the consumer SHALL report it as a broken edge.
- <a id="sokf_format_63"></a>SOKF_FORMAT_63 [ubiquitous]: the consumer SHALL resolve the id in a label in the same way as a `to`.

### 8.2 References to other stores

A reference to a document in another store begins with the store, separated from the rest of the reference by `:`. An id with a store in front of it is called a qualified id:

```markdown
[text][sokf:sokf-spec:sokf-validation-spec]
[sokf:sokf-spec:sokf-validation-spec#3]
[sokf:sokf-spec:#user]
[sokf:sokf-spec:flowers#]
to: sokf-spec:sokf-validation-spec
```

The store is written in one of two ways:

- `<name>` - A store in the same scope as the citing store.
- `<scope>/<name>` - A store under any root of the repository (§2).

A reference without a store refers to a document in its own store. A qualified id works anywhere an id works, with the same meaning. The `resource` and `sources[].resource` keys refer to files by path (§3, §6).

- <a id="sokf_format_64"></a>SOKF_FORMAT_64 [ubiquitous]: the document SHALL name the store in each reference by the current or a former identity of a knowledge store in the repository.
- <a id="sokf_format_135"></a>SOKF_FORMAT_135 [unwanted]: IF a reference names a store that is not a knowledge store of the repository, THEN the consumer SHALL treat the reference as a broken edge.

### 8.3 Canonical references

Each target has exactly one canonical reference: the shortest one that identifies it. A reference within its own store has no store prefix. A reference to another store in the same scope has the prefix `<name>:`, and a reference to a store in another scope has the prefix `<scope>/<name>:`.

A reference is not canonical when it has a longer prefix than needed, a former identity, a bare fragment that refers to a key in another store, or a path to a document or an asset. The fix rewrites each of these in canonical form ([validation specification][sokf:sokf-validation-spec]). Canonical form concerns only the store prefix and the path, and never the fragment. A label that refers to its own document, such as `[sokf:overview#…]` inside the overview document, therefore stays as it is. It stays even though a link to the fragment alone would be shorter.

- <a id="sokf_format_65"></a>SOKF_FORMAT_65 [ubiquitous]: the document SHALL use the canonical form for each label, each `to`, and each include marker.

### 8.4 Sidecars

A `sokf:<id>` label refers to the content of a document, and the content of a sidecar is its asset. For example, `![Flowers][sokf:flowers]` displays the image, and `[the report][sokf:q3-report]` links to a PDF. A label with an empty fragment, `sokf:<id>#`, refers to the file of the document itself. For a sidecar, that file is its `.md` file, and for any other document it is the document.

An image reference is a body link. A typed edge to an asset is a `has-part` link to its sidecar.

- <a id="sokf_format_66"></a>SOKF_FORMAT_66 [event]: WHEN a `sokf:` label has an empty fragment, the body link SHALL have an id before the fragment.
- <a id="sokf_format_67"></a>SOKF_FORMAT_67 [ubiquitous]: the body SHALL refer to a sidecar in every image reference that uses a `sokf:` label.

### 8.5 Citing part of a document

A body link can point to part of a document with a fragment: `sokf:<id>#<fragment>`. The fragment is the id of an anchor in the document. The id of a heading is its slug. A key that the schema of the target captures ([schema specification][sokf:sokf-schema-spec]) has the slug of the key as its id, and the generated anchor of the key carries that id (§9).

A slug follows the rule that repository hosts apply to the text of a heading: lowercase the text, remove every character other than a letter, a digit, a space, a hyphen, or an underscore, and turn each space into a hyphen. For example, the key `CLI-1_AC-2` has the slug `cli-1_ac-2`, and a document cites it as `[CLI-1_AC-2][sokf:req-cli#cli-1_ac-2]`.

Each heading of a document has a slug of its own, as GitHub gives it. The headings count in document order, including a heading inside a quote or a list item. The first heading with a slug keeps it. A later heading with the same slug takes the slug, a hyphen, and the lowest count from 1 that gives a slug no earlier heading holds. For example, the headings `Foo`, `Foo`, `Foo 1`, and `Foo` have the slugs `foo`, `foo-1`, `foo-1-1`, and `foo-2`.

A schema can declare a key unique beyond its document. Such a key also has a bare form: `sokf:#<slug>` in its own store, and `sokf:<store>:#<slug>` in another store.

A link whose destination is only a fragment, `[text](#<fragment>)`, points to an anchor in the same document.

A fragment on the id of a sidecar points to part of the body of the sidecar.

This specification gives no meaning to a fragment on a path link to a repository file.

- <a id="sokf_format_68"></a>SOKF_FORMAT_68 [ubiquitous]: the document SHALL give each key a slug that differs from the slug of every other key and from the slug that SOKF_FORMAT_155 gives every heading in the document.
- <a id="sokf_format_69"></a>SOKF_FORMAT_69 [ubiquitous]: the key SHALL have a slug that no other key has within the uniqueness scope of its schema, the knowledge store or the repository.
- <a id="sokf_format_70"></a>SOKF_FORMAT_70 [ubiquitous]: the body link SHALL use a bare fragment only for a key that its schema declares unique beyond its document.
- <a id="sokf_format_71"></a>SOKF_FORMAT_71 [ubiquitous]: the body link SHALL have a fragment that identifies one key or one heading of its target.
- <a id="sokf_format_72"></a>SOKF_FORMAT_72 [ubiquitous]: the body link SHALL use the slug of the key or heading as each fragment.
- <a id="sokf_format_155"></a>SOKF_FORMAT_155 [ubiquitous]: the consumer SHALL give each heading of a document, in document order, its slug, or, when an earlier heading holds that slug, the slug followed by `-` and the lowest count from 1 that gives a slug no earlier heading holds.
- <a id="sokf_format_134"></a>SOKF_FORMAT_134 [ubiquitous]: the consumer SHALL resolve each bare fragment `sokf:<store>:#<slug>` of a key unique in the repository to the document that captures that key.
- <a id="sokf_format_73"></a>SOKF_FORMAT_73 [ubiquitous]: the body SHALL use, as the fragment of each link whose destination is only a fragment, the slug of one key or one heading in the same document.
- <a id="sokf_format_74"></a>SOKF_FORMAT_74 [ubiquitous]: the body SHALL contain a reference definition for every reference-style link whose label is not a `sokf:` label.

### 8.6 Frontmatter links in the body

Each entry in `links` has a counterpart in the body: a body link to the target of the entry. A label with a fragment that points to the target is also a counterpart, as is an image reference to the target. A body link with no matching entry in `links` is an untyped `relates-to` edge, and the text around it gives its meaning.

- <a id="sokf_format_75"></a>SOKF_FORMAT_75 [ubiquitous]: the body SHALL contain, for each link, a body link to the target of the link.
- <a id="sokf_format_118"></a>SOKF_FORMAT_118 [ubiquitous]: the consumer SHALL treat each body link that mirrors no link as a `relates-to` edge.

## 9. Paths, generated blocks, and markers

Paths take one of three forms:

- `/<path>` - A path from the repository root. This is the preferred form for links in the body. The `resource` and `sources[].resource` keys take this form or a URL.
- `<path>` - A path relative to the file that contains it, as in standard Markdown, used in body links.
- `<url>` - An absolute URL.

Globs in this specification follow gitignore rules: `*` matches within one directory level, and `**` matches at any depth.

- <a id="sokf_format_76"></a>SOKF_FORMAT_76 [ubiquitous]: the body SHALL make each path that it links to or embeds resolve to a file or directory in the repository.
- <a id="sokf_format_125"></a>SOKF_FORMAT_125 [ubiquitous]: the validator SHALL match each glob by gitignore rules, where `*` stops at `/` and `**` matches at any depth.

### 9.1 Definition blocks

The definition block ends a file and states the path for each `sokf:` label that the file uses:

```markdown
<!-- sokf:links -->

[sokf:#cli-1_ac-2]: /knowledge/req-cli.md#cli-1_ac-2
[sokf:config]: /knowledge/config.md
[sokf:flowers]: /knowledge/images/flowers.png
[sokf:flowers#]: /knowledge/images/flowers.png.md
[sokf:planner]: /knowledge/core/planner.md
[sokf:req-cli#acceptance-criteria]: /knowledge/req-cli.md#acceptance-criteria
[sokf:sokf-spec:sokf-validation-spec#3]: /sokf-spec/sokf-validation-spec.md#3
```

Each definition gives the path that its label refers to (§8). For the id of a sidecar, that path is the path of the asset. With an empty fragment, it is the path of the sidecar, and with a fragment that identifies a heading or a key, it is the path of the sidecar followed by `#<fragment>`. For any other document, an empty fragment gives the path of the document.

The fix writes the block, and rewrites it whenever a document or an asset moves in any store.

- <a id="sokf_format_77"></a>SOKF_FORMAT_77 [event]: WHEN a document uses a `sokf:` label, the document SHALL end with a definition block.
- <a id="sokf_format_78"></a>SOKF_FORMAT_78 [event]: WHEN an additional file uses a `sokf:` label, the additional file SHALL end with a definition block.
- <a id="sokf_format_79"></a>SOKF_FORMAT_79 [ubiquitous]: the definition block SHALL begin with the line `<!-- sokf:links -->` and end the file.
- <a id="sokf_format_80"></a>SOKF_FORMAT_80 [ubiquitous]: the definition block SHALL contain one reference definition for each `sokf:` label in its file, sorted by label in ascending order.
- <a id="sokf_format_81"></a>SOKF_FORMAT_81 [ubiquitous]: the definition block SHALL give in each definition the current `/` path of the target, followed by `#<fragment>` when the label has a fragment.
- <a id="sokf_format_82"></a>SOKF_FORMAT_82 [ubiquitous]: the consumer SHALL resolve each label by its id (§8), never through the definition block.

### 9.2 Anchors

Every key that the schema of a document captures ([schema specification][sokf:sokf-schema-spec]) has one generated anchor: an empty HTML link whose id is the slug of the key (§8), such as `<a id="cli-1_ac-2"></a>`. The tramlines-md specification (TLMD_104) places the anchor at the start of the heading that captures the key. Where no heading captures the key, the anchor goes before the text that the schema matches to capture the key. For a key on a line of a code or HTML block, the anchor goes on a line of its own before the block. Schema rules ignore anchors. An include copies a body without its anchors, so each anchor belongs to the document that captures its key.

- <a id="sokf_format_83"></a>SOKF_FORMAT_83 [ubiquitous]: the document SHALL contain exactly one anchor for each key that its schema captures, and no other anchors.
- <a id="sokf_format_84"></a>SOKF_FORMAT_84 [ubiquitous]: the anchor SHALL sit where TLMD_104 to TLMD_106 of the [tramlines-md specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-md-spec.md#7-keys-and-anchors) place it.
- <a id="sokf_format_126"></a>SOKF_FORMAT_126 [ubiquitous]: the validator SHALL read the text of each unit of a document ([schema specification][sokf:sokf-schema-spec#unit]) without its anchors.

### 9.3 Include blocks

An include block contains a copy of the body of another document, or of a region of a file (§9.6), between two markers:

```markdown
<!-- sokf:include <id> -->

…the named document's body…

<!-- /sokf:include -->
```

The opening marker refers to one of the following:

- `<id>` - The expanded body of the document. For a document in another store, the store comes before the id (§8).
- `<sidecar-id>` - The asset of the sidecar, fenced.
- `<sidecar-id>#<name>` - The region `<name>` of the asset of the sidecar, fenced.
- `<sidecar-id>#` - The expanded body of the sidecar.
- `/<path>` - The file, fenced.
- `/<path>#<name>` - The region `<name>` of the file, fenced.

The expanded body of a document is its body without its definition block, its anchors, and the markers of its own include blocks, with the content between those markers kept. The writer writes the markers, and the fix writes the generated copy between them and keeps it up to date ([validation specification][sokf:sokf-validation-spec]). The fix adds every id that the copy cites to the definition block of the including document. A sidecar can include its own asset, and that include is not a cycle.

````markdown
<!-- sokf:include /crates/app/sokf/src/main.rs#cli -->

```rust
…the named region of the file…
```

<!-- /sokf:include -->
````

To include from another store, put the store before the id:

```markdown
<!-- sokf:include other:overview -->

…the body of the document `overview` of the store `other`…

<!-- /sokf:include -->
```

- <a id="sokf_format_85"></a>SOKF_FORMAT_85 [ubiquitous]: the include block SHALL contain, between its markers, the current content that its opening marker refers to, in the form that the preceding list gives.
- <a id="sokf_format_86"></a>SOKF_FORMAT_86 [ubiquitous]: the include block SHALL close with the line `<!-- /sokf:include -->`.
- <a id="sokf_format_87"></a>SOKF_FORMAT_87 [ubiquitous]: the include block SHALL have an opening marker whose id, fragment, path, and region, where present, each resolve in the repository.
- <a id="sokf_format_88"></a>SOKF_FORMAT_88 [ubiquitous]: the include block SHALL refer to a document or an asset by id, and use a path only for other files in the repository.
- <a id="sokf_format_89"></a>SOKF_FORMAT_89 [ubiquitous]: the include block SHALL NOT refer by path to an additional file of the knowledge store that holds the including file.
- <a id="sokf_format_90"></a>SOKF_FORMAT_90 [ubiquitous]: the include block SHALL put a fragment only on the id of a sidecar.
- <a id="sokf_format_91"></a>SOKF_FORMAT_91 [ubiquitous]: the knowledge store SHALL have an include graph without cycles, in which each document is a node and each opening marker is an edge.
- <a id="sokf_format_92"></a>SOKF_FORMAT_92 [ubiquitous]: the include block SHALL put a file or a region inside a fence.
- <a id="sokf_format_93"></a>SOKF_FORMAT_93 [ubiquitous]: the include block SHALL tag its fence with the tag that the list of fence tags gives for the extension of the file, and leave it untagged only for a file without an extension.
- <a id="sokf_format_94"></a>SOKF_FORMAT_94 [ubiquitous]: the include block SHALL open its fence with at least three backticks, and with one more than the longest run of backticks in the content.

The extension of a file is the text after the last dot in its name, unless that dot is the first character. Each extension has this fence tag:

- `rs` - `rust`.
- `yml` - `yaml`.
- `ts` - `typescript`.
- `py` - `python`.
- `js` - `javascript`.
- `sh` - `bash`.
- `md` - `markdown`.
- `<other>` - The extension itself.

The fix keeps the meaning of the original in a copy. When the original is a body from another store, the fix rewrites each of its labels in the form that the including store uses (§8). Without that rewrite, a label would refer to a document of the including store instead of a document of the store of the original. For example, `[sokf:rates]` in a document of the store `other` becomes `[sokf:other:rates]` in the copy.

- <a id="sokf_format_128"></a>SOKF_FORMAT_128 [ubiquitous]: the include block SHALL contain, for a region, the lines of every region with that name, in file order, without the marker lines.
- <a id="sokf_format_129"></a>SOKF_FORMAT_129 [ubiquitous]: the include block SHALL contain the body of a document without its definition block and anchors.
- <a id="sokf_format_130"></a>SOKF_FORMAT_130 [ubiquitous]: the include block SHALL contain each label of a body from another store in the form that the including store uses.
- <a id="sokf_format_131"></a>SOKF_FORMAT_131 [ubiquitous]: the document SHALL contain no include closing marker without an opening marker before it.
- <a id="sokf_format_132"></a>SOKF_FORMAT_132 [ubiquitous]: the validator SHALL recognise include markers, `sokf:links` markers, and anchors only outside fenced code blocks.
- <a id="sokf_format_136"></a>SOKF_FORMAT_136 [ubiquitous]: the include block SHALL contain, for a path without `#`, the whole file without its region marker lines.
- <a id="sokf_format_137"></a>SOKF_FORMAT_137 [ubiquitous]: the include block SHALL contain, for a region, its lines regardless of any other regions that overlap it, without the marker lines of any region.
- <a id="sokf_format_138"></a>SOKF_FORMAT_138 [event]: WHEN a region contains no lines, the include block SHALL contain an empty fence.
- <a id="sokf_format_139"></a>SOKF_FORMAT_139 [ubiquitous]: the validator SHALL apply none of the rules that this specification sets for the body of a document to the content of an include block.
- <a id="sokf_format_154"></a>SOKF_FORMAT_154 [ubiquitous]: the validator SHALL take from the content of an include block only its `sokf:` labels, to add them to the definition block.

### 9.4 Indexes

Every directory of a knowledge store has an index that lists its contents, and the fix writes the whole file.

```markdown
# core

- [Executor][sokf:executor] - Applies planned actions.
- [Planner][sokf:planner] - Pure planning stage; no filesystem writes.
- [plugins/](plugins/index.md)

<!-- sokf:links -->

[sokf:executor]: /knowledge/core/executor.md
[sokf:planner]: /knowledge/core/planner.md
```

- <a id="sokf_format_96"></a>SOKF_FORMAT_96 [ubiquitous]: the knowledge store SHALL have an index in every directory, including the root and `schemas/`, regardless of the excludes and git-ignore rules of the repository.
- <a id="sokf_format_97"></a>SOKF_FORMAT_97 [ubiquitous]: the index SHALL contain only its heading, its list of entries, and its definition block, with no frontmatter.
- <a id="sokf_format_98"></a>SOKF_FORMAT_98 [ubiquitous]: the index SHALL begin with the heading `# <name>`.
- <a id="sokf_format_99"></a>SOKF_FORMAT_99 [ubiquitous]: the index SHALL take its `<name>` at the root of the knowledge store from the `name` in the manifest, where that is a non-empty string.
- <a id="sokf_format_100"></a>SOKF_FORMAT_100 [ubiquitous]: the index SHALL take its `<name>` from the name of its directory in all other cases.
- <a id="sokf_format_101"></a>SOKF_FORMAT_101 [ubiquitous]: the index SHALL list one entry for each `.md` file in its directory other than `index.md`, and one for each subdirectory.
- <a id="sokf_format_102"></a>SOKF_FORMAT_102 [ubiquitous]: the index SHALL list its entries in the byte order of their paths.
- <a id="sokf_format_103"></a>SOKF_FORMAT_103 [ubiquitous]: the index SHALL list a document as `- [<title>][sokf:<id>] - <description>`.
- <a id="sokf_format_104"></a>SOKF_FORMAT_104 [ubiquitous]: the index SHALL use as `<title>` the `title` of the document where it is a string, and otherwise its `id`.
- <a id="sokf_format_105"></a>SOKF_FORMAT_105 [ubiquitous]: the index SHALL omit ` - <description>` where the `description` of the document is not a string.
- <a id="sokf_format_106"></a>SOKF_FORMAT_106 [ubiquitous]: the index SHALL contain a single space in place of each line break in a title or a description.
- <a id="sokf_format_107"></a>SOKF_FORMAT_107 [unwanted]: IF a document has frontmatter that fails to parse, no `id`, or an `id` that another document shares (§5), THEN the index SHALL list it as `- [<file>](<file>.md)`.
- <a id="sokf_format_108"></a>SOKF_FORMAT_108 [ubiquitous]: the index SHALL list a subdirectory as `- [<name>/](<name>/index.md)`.
- <a id="sokf_format_140"></a>SOKF_FORMAT_140 [ubiquitous]: the index SHALL list a knowledge store nested in its directory as a subdirectory.
- <a id="sokf_format_133"></a>SOKF_FORMAT_133 [event]: WHEN a directory contains no subdirectory and no `.md` file other than its index, the index SHALL contain no list.
- <a id="sokf_format_109"></a>SOKF_FORMAT_109 [ubiquitous]: the index SHALL end with a definition block whenever one of its entries uses a `sokf:` label.
- <a id="sokf_format_110"></a>SOKF_FORMAT_110 [ubiquitous]: the index SHALL separate its heading, list, marker, and definitions with one blank line each, and end with a single line ending.

The index lists schemas and sidecars as documents, and an asset appears only through its sidecar. A subdirectory `core` comes before a file `core.md`.

### 9.5 Markers

A marker is a comment line in a repository file outside every knowledge store. It states a fact about the lines around it, such as that the code under it implements a criterion, and cites the key or the document that the fact concerns:

```rust
// sokf-implements: CLI-1_AC-2
fn parse(args: &[String]) -> Options {
```

The tag of a marker is the first word of its line after whitespace and ASCII punctuation, such as `//`, `#`, `--`, `/*`, or `<!--`, so a marker fits the comment syntax of any file. The tag is `sokf-`, a relationship in the form that §8 gives, and a colon, with whitespace after the colon. A qualified id, such as `sokf-spec:glossary`, has no whitespace after its colon, so it is not a tag.

One reference follows the tag, and after the reference come only whitespace and characters that are neither letters nor digits, such as the end of a block comment. A marker states one fact, so a line of code that implements two criteria has two markers.

```text
# sokf-tests: CLI_P-5
/* sokf-implements: sokf-spec:#sokf_format_96 */
<!-- sokf-references: design-cache#merge-sync -->
```

The reference takes any form that a label takes after `sokf:` (§8), or the form of a bare key:

- `<id>` - A document, or the asset of a sidecar.
- `<id>#<fragment>` - A key or a heading of a document.
- `#<key>` - A key that its schema declares unique beyond its document.
- `<store>:` - Before any of the preceding forms, the store of the target (§8.2).
- `<key>` - A key that its schema declares unique in the repository, written as its text, such as `CLI-1_AC-2`.

A bare word in lowercase kebab-case is an id, and any other bare word is a key. A marker belongs to no knowledge store, so a reference without a store resolves in every knowledge store of the repository. A fragment or a key in a marker matches a key by slug. The writer of a marker can therefore give a key as written, such as `CLI-1_AC-2`, and not only as its slug. The fix leaves every marker as written, because a marker is in a file that the writer of the code owns.

A schema can require, for each key or document of its type, a marker with a given relationship. A schema can also limit the relationships of the markers that cite those keys and documents ([schema specification][sokf:sokf-schema-spec] §4).

A writer protects text that looks like a marker, but is not one, with an ignore region: the lines between a `sokf:ignore-begin` line and the next `sokf:ignore-end` line. A `sokf:ignore-next-line` line protects the line after it. Each of these directives is the first word of its line after whitespace and ASCII punctuation, as the tag of a marker is. The end of a comment, such as `-->` or `*/`, can follow a directive, as it can follow a marker's reference.

- <a id="sokf_format_142"></a>SOKF_FORMAT_142 [ubiquitous]: the marker SHALL begin its line with the tag `sokf-<rel>:` and whitespace, after only whitespace and ASCII punctuation.
- <a id="sokf_format_143"></a>SOKF_FORMAT_143 [ubiquitous]: the marker SHALL hold its relationship in lowercase kebab-case, the form of a `rel` in §8.
- <a id="sokf_format_144"></a>SOKF_FORMAT_144 [ubiquitous]: the marker SHALL hold exactly one reference after its tag, followed by only whitespace and characters that are neither letters nor digits.
- <a id="sokf_format_145"></a>SOKF_FORMAT_145 [ubiquitous]: the marker SHALL hold a reference that resolves to one document, or to one key or one heading of a document.
- <a id="sokf_format_146"></a>SOKF_FORMAT_146 [ubiquitous]: the consumer SHALL read a bare word of a marker in lowercase kebab-case as an id, and any other bare word as a key.
- <a id="sokf_format_147"></a>SOKF_FORMAT_147 [ubiquitous]: the consumer SHALL resolve a bare key of a marker only to a key that its schema declares unique in the repository.
- <a id="sokf_format_148"></a>SOKF_FORMAT_148 [ubiquitous]: the consumer SHALL resolve a reference of a marker without a store in every knowledge store of the repository.
- <a id="sokf_format_149"></a>SOKF_FORMAT_149 [ubiquitous]: the marker SHALL name the store of an id that more than one knowledge store holds.
- <a id="sokf_format_150"></a>SOKF_FORMAT_150 [ubiquitous]: the consumer SHALL match the fragment and the key of a marker by their slug.
- <a id="sokf_format_151"></a>SOKF_FORMAT_151 [ubiquitous]: the consumer SHALL read no marker in an ignore region, and none on the line after a `sokf:ignore-next-line` line.
- <a id="sokf_format_152"></a>SOKF_FORMAT_152 [ubiquitous]: the repository file SHALL pair each `sokf:ignore-begin` line with a later `sokf:ignore-end` line, and contain no `sokf:ignore-begin` line inside an ignore region.
- <a id="sokf_format_153"></a>SOKF_FORMAT_153 [ubiquitous]: the repository file SHALL contain no `sokf:ignore-end` line outside an ignore region.
- <a id="sokf_format_156"></a>SOKF_FORMAT_156 [event]: WHEN only whitespace and characters that are neither letters nor digits follow an ignore directive on its line, the consumer SHALL read the directive.

### 9.6 Regions

A region of a file starts at a line containing `sokf:begin <name>` and ends at a later line containing `sokf:end <name>`. The markers use the comment syntax of the file and match by substring. A name matches `[A-Za-z0-9_-]+`. A `sokf:begin` or `sokf:end` followed by other text is not a marker.

The fix joins regions with the same name in file order. Regions with different names can overlap, and the fix extracts each one on its own. When the fix extracts a region, it removes every region marker line, of any name, and keeps all other lines unchanged. An empty region gives an empty fence. A path without `#` refers to the whole file, without its marker lines.

- <a id="sokf_format_95"></a>SOKF_FORMAT_95 [ubiquitous]: the region SHALL pair each `sokf:begin <name>` with the next `sokf:end <name>`, and have no `sokf:end <name>` without an open `sokf:begin <name>` before it.
- <a id="sokf_format_127"></a>SOKF_FORMAT_127 [ubiquitous]: the region SHALL have a name that matches `[A-Za-z0-9_-]+`.
- <a id="sokf_format_112"></a>SOKF_FORMAT_112 [ubiquitous]: the region SHALL contain no second `sokf:begin <name>` while `<name>` is still open.

## 10. Conformance

A knowledge store conforms to SOKF when it meets each criterion of this specification and of the [schema specification][sokf:sokf-schema-spec] that applies to it.

- <a id="sokf_format_111"></a>SOKF_FORMAT_111 [ubiquitous]: the consumer SHALL read a knowledge store that has any of the faults in the following list.

A consumer reads a knowledge store that has any of these faults:

- `Missing field` - An absent optional frontmatter field
- `Unknown type` - A `type` that no schema governs
- `Unknown key` - A frontmatter key that the schema does not declare
- `Unknown relation` - A `rel` outside the core set
- `Broken link` - A link or a body link that resolves to no document
- `Refused document` - A document that its schema rejects
- `Lone asset` - An asset without a sidecar
- `Missing index` - An index that is absent or out of date
- `Missing manifest` - A knowledge store without a manifest

## 11. Versioning

The change history of the format is in [the changelog][sokf:changelog].

<!-- sokf:links -->

[sokf:changelog]: /sokf-spec/changelog.md
[sokf:sokf-schema-spec]: /sokf-spec/sokf-schema-spec.md
[sokf:sokf-schema-spec#unit]: /sokf-spec/sokf-schema-spec.md#unit
[sokf:sokf-validation-spec]: /sokf-spec/sokf-validation-spec.md
