---
type: SokfSchemaSpecification
id: sokf-schema-spec
title: "SOKF: Schema specification"
description: "This specification defines the schemas of a knowledge store: what a schema governs, and its rules, scaffolds, lifecycles, and templates."
version: "0.6"
status: draft
---

# SOKF: Schema specification

This specification defines the schemas of the Superdev Open Knowledge Format (SOKF): how a schema constrains the documents of a knowledge store.

SOKF stores knowledge as Markdown documents in a knowledge store. A schema states the rules for the documents of one type. The validator checks each document against the schema of its type, and a tool derives from the schema the scaffold that a new document starts from.

- [About this specification](#about-this-specification)
- [1. Terminology](#1-terminology)
- [2. Schema documents](#2-schema-documents)
- [3. What a schema governs](#3-what-a-schema-governs)
- [4. Rules](#4-rules)
- [5. Scaffolds](#5-scaffolds)
- [6. Lifecycle](#6-lifecycle)
- [7. Templates](#7-templates)
- [8. Id templates](#8-id-templates)
- [9. Schema terms](#9-schema-terms)

## About this specification

Each normative rule of this specification is a criterion in the EARS (Easy Approach to Requirements Syntax) form, with a key of the form `SOKF_SCHEMA_NUMBER`, where `NUMBER` is the number of the criterion. The descriptions of the keys in the tramlines-md grammar and in the [SOKF schema extension][sokf:sokf-extension] are normative.

## 1. Terminology

This specification uses the terms of the [format specification][sokf:sokf-format-spec], and adds these:

- <a id="unit"></a>`unit`: A heading, paragraph, list item, table row, line of a fence, or frontmatter value in a document.
- <a id="template"></a>`template`: The text of a unit, with a typed placeholder wherever the text varies (§7).
- <a id="placeholder"></a>`placeholder`: The part of a template, `${name:type}`, that matches any value of its type (§7).
- <a id="lifecycle"></a>`lifecycle`: The ordered dimensions of state that the documents of a type move through (§6).
- <a id="dimension"></a>`dimension`: An ordered list of states, stored in one frontmatter key (§6).
- <a id="scaffold"></a>`scaffold`: The starting document that a validator derives from a schema for one combination of the states of the schema (§5).

## 2. Schema documents

A schema states the structural rules for the documents of one type, or for a set of additional files.

- <a id="sokf_schema_1"></a>SOKF_SCHEMA_1 [ubiquitous]: the Schema document SHALL contain its schema as a single fenced YAML block opened with four backticks.
- <a id="sokf_schema_2"></a>SOKF_SCHEMA_2 [ubiquitous]: the schema SHALL parse as YAML 1.2 with the core schema, and use no anchors, aliases, or tags.

## 3. What a schema governs

A schema declares what it governs with one of three keys:

- `type`: A `type` const in the `frontmatter` of the schema. The schema governs every document of that type.
- `additional-files`: One or more `/`-rooted globs. The schema governs every repository file that a glob matches, as an additional file.
- `asset-files`: One or more `/`-rooted globs within the knowledge store of the schema, declared alongside the `type` const. Every file that a glob matches is an asset of the type, and the documents of the type are the sidecars of those assets.

An additional file belongs to the knowledge store of the schema that governs it. An additional file that contains include blocks also has a definition block ([format specification][sokf:sokf-format-spec] §9).

The schema of a sidecar defines its body, as for any other document, and `blocks: []` makes a sidecar that has only frontmatter.

- <a id="sokf_schema_3"></a>SOKF_SCHEMA_3 [ubiquitous]: the schema SHALL say what it governs in exactly one of the three ways.
- <a id="sokf_schema_4"></a>SOKF_SCHEMA_4 [ubiquitous]: the schema SHALL declare `additional-files` on its own, without `type`, `asset-files`, or `lifecycle`.
- <a id="sokf_schema_5"></a>SOKF_SCHEMA_5 [ubiquitous]: the schema SHALL declare `additional-files` globs that are `/`-rooted and match no document and no index in a knowledge store.
- <a id="sokf_schema_6"></a>SOKF_SCHEMA_6 [ubiquitous]: the additional file SHALL match the glob of exactly one schema.
- <a id="sokf_schema_7"></a>SOKF_SCHEMA_7 [ubiquitous]: the schema SHALL declare `asset-files` globs that match no reserved file, no file under `schemas/`, no `.md` file, and no additional file.
- <a id="sokf_schema_8"></a>SOKF_SCHEMA_8 [ubiquitous]: the schema SHALL declare `asset-files` globs that are `/`-rooted and lie within its knowledge store.
- <a id="sokf_schema_9"></a>SOKF_SCHEMA_9 [ubiquitous]: the schema SHALL declare `asset-files` globs that match no file that the `asset-files` globs of another schema also match.
- <a id="sokf_schema_10"></a>SOKF_SCHEMA_10 [ubiquitous]: the schema SHALL begin each `asset-files` glob with its lifecycle `root` followed by `/**/`, where it declares a `root`.
- <a id="sokf_schema_11"></a>SOKF_SCHEMA_11 [ubiquitous]: the document SHALL be the sidecar of an asset that the `asset-files` globs of its schema match, where its schema declares such globs.
- <a id="sokf_schema_12"></a>SOKF_SCHEMA_12 [ubiquitous]: the sidecar SHALL have the `type` of the schema whose glob matches its asset.
- <a id="sokf_schema_44"></a>SOKF_SCHEMA_44 [ubiquitous]: the consumer SHALL resolve each id in an additional file within the knowledge store of the schema that governs the file.

## 4. Rules

A schema is strict: it requires everything it declares and allows nothing else. A schema relaxes a rule only with a key that exists for that purpose, such as `links-open` (see the SOKF schema extension).

- <a id="sokf_schema_13"></a>SOKF_SCHEMA_13 [ubiquitous]: the knowledge store SHALL contain exactly one schema for each type in use, other than `Schema`.
- <a id="sokf_schema_14"></a>SOKF_SCHEMA_14 [ubiquitous]: the document SHALL conform to the schema of its type.
- <a id="sokf_schema_15"></a>SOKF_SCHEMA_15 [ubiquitous]: the Schema document SHALL conform to the `schema-document` schema of the [SOKF schema extension][sokf:sokf-extension].
- <a id="sokf_schema_16"></a>SOKF_SCHEMA_16 [ubiquitous]: the additional file SHALL conform to the schema whose glob matches it.
- <a id="sokf_schema_17"></a>SOKF_SCHEMA_17 [ubiquitous]: the document SHALL receive every citation that the `cited-by` entries of its schema require.
- <a id="sokf_schema_33"></a>SOKF_SCHEMA_33 [ubiquitous]: the document SHALL have, for each `links` entry of its schema, between `min` and `max` links with that `rel` to a type in `to-type`.
- <a id="sokf_schema_34"></a>SOKF_SCHEMA_34 [ubiquitous]: the document SHALL have only links whose `rel` appears in an entry of its schema, unless the schema declares `links-open`.
- <a id="sokf_schema_35"></a>SOKF_SCHEMA_35 [ubiquitous]: the schema SHALL name, in the `keys` of each `cited-by` entry, a rule that captures a key.
- <a id="sokf_schema_48"></a>SOKF_SCHEMA_48 [ubiquitous]: the schema SHALL declare in each `cited-by` entry exactly one of `from-type` and `from-marker`.
- <a id="sokf_schema_49"></a>SOKF_SCHEMA_49 [ubiquitous]: the marker SHALL have one of the relationships in the `markers` list of the cited document's schema, where that schema declares `markers`.
- <a id="sokf_schema_52"></a>SOKF_SCHEMA_52 [ubiquitous]: the schema SHALL name, in the `section` of each `plans-paths` entry, the `rule-id` of one of its sections.
- <a id="sokf_schema_53"></a>SOKF_SCHEMA_53 [ubiquitous]: the validator SHALL accept a body link to, a `resource` that lists, and an include of a path that a document plans, although no file exists there.
- <a id="sokf_schema_54"></a>SOKF_SCHEMA_54 [ubiquitous]: the body SHALL link no archived document from a paragraph that a pattern in the `current-links` of its schema matches.
- <a id="sokf_schema_37"></a>SOKF_SCHEMA_37 [ubiquitous]: the frontmatter SHALL contain the path of a file in the repository as each value that its schema binds to the format `repo-path`.
- <a id="sokf_schema_38"></a>SOKF_SCHEMA_38 [ubiquitous]: the frontmatter SHALL contain the id of a document in its knowledge store as each value that its schema binds to the format `document-id`.

A `sokf:` body link cites a document when a `cited-by` entry lists the type of the document that contains the link. The containing document can be in any knowledge store. The link cites the document whether or not it has a fragment. An entry can declare `keys`, the name of a rule that captures keys. Each key that this rule captures then needs its own citation. The citation is a link whose fragment is the slug of the key, or the slug of the key's heading when the key is in a heading. Included text, generated text, and links from the document to itself do not count as citations. A link counts even when the document that contains it has failures of its own.

An entry that names `from-marker` counts markers in place of body links ([format specification][sokf:sokf-format-spec] §9.5). The value of `from-marker` is a relationship. A marker with that relationship cites the document or the key that its reference resolves to. A marker of another relationship does not count, so a test of a criterion does not stand in for its implementation.

A schema declares `markers`, a list of relationships, to limit the markers that cite its documents and their keys, and `markers: []` refuses every marker. When the list refuses a marker, the validator reports the failure at the file and line of that marker.

A schema declares `plans-paths` to let a document plan the files that its work creates. Each entry names a section by its `rule-id`, and can declare `only`. While a document is in the states of an entry's `only`, each `/`-rooted path that a link in such a section names is planned. The section runs from its heading to the next heading of the same or a higher level. When the document leaves those states, or the file exists, the path is checked like any other, and the fix fills an include of the file from the file.

A schema declares `current-links` to keep its promises current. Each entry is a pattern for a paragraph that makes a promise, such as one that begins with `Planned:`, and such a paragraph links no archived document, because the work that an archived document records is done or dropped.

A `links` entry requires at least one link unless it declares `min`, and allows any number unless it declares `max`. An entry with `only` applies only in the states that it names.

A rule uses `unique` to set the scope in which each key it captures is unique: its document (the default), its knowledge store (`knowledge`), or the whole repository (`repository`).

When an include block refers to a sidecar, a `source-pattern` matches the path of the asset, followed by `#<region>` when the include marker selects a region. A `source-pattern` never matches the id of a document. A validator checks the target of an include block against its `source-pattern`, as the [tramlines-md specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-md-spec.md) states (TLMD_163).

The validator checks every document against the core frontmatter keys of the format specification, whether or not its schema declares them. A schema declares a core key only to require it or to narrow its value, and declares `id` only with a `template` or a `const`.

## 5. Scaffolds

A tool creates a new document of a type from a scaffold of its schema, not from the complete documents in `examples`. The validator derives one scaffold for each combination of the states of the schema. The text of a scaffold comes only from the fixed texts and the `example` values of the rules, and from the `examples` of the sections that repeat. A new document keeps the `sokf:optional` regions of its scaffold.

The [tramlines-md specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-md-spec.md#9-scaffolds) states how the validator derives a scaffold, writes its markers, and checks it and each example against the schema (TLMD_114 to TLMD_124, TLMD_164, TLMD_176 to TLMD_191, TL_64, TL_66, and TL_69). It also states that a governed document keeps no `only` or `guidance` marker (TLMD_10 and TLMD_158), pairs each `optional` marker (TLMD_12), and draws a warning for an `optional` region (TLMD_11). SOKF spells the markers `sokf:only`, `sokf:guidance`, and `sokf:optional`.

A `sokf:guidance` line is an HTML comment in the body and a YAML comment in the frontmatter.

The frontmatter of a scaffold has one link for each `links` entry of the schema, and no optional region.

A `sokf:only` marker gates a part that only some states allow. In the body, the marker is a comment line before the part, and in the frontmatter, it is a comment at the end of a line.

```text
<!-- sokf:only state=open,done kind=bug -->
resolution: none # sokf:only state=closed
```

When a tool creates a document, it keeps or drops each marked part according to the states of the document, and it removes the markers.

Some checks apply only to files in the repository, and not to scaffolds or examples. These are the checks that a sidecar has an asset, that a sidecar has the type of the schema that governs its asset, that `resource` exists, that links resolve, and that `repo-path` and `document-id` values refer to existing files and documents.

## 6. Lifecycle

A type schema can declare a lifecycle, which is a list of dimensions. Each dimension is an ordered list of states, and a document records its state in each dimension in a frontmatter key. The [tramlines specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-spec.md) requires each lifecycle key, with a value from the states of its dimension (TL_62 and TL_108).

The `root` of a schema is the `/`-rooted directory in the knowledge store that holds the documents of its type. A schema without a path-enforced dimension can declare a `root`, and its documents then lie under the root, at any depth. A dimension is path-enforced when its states determine the directory of the document. That directory lies under the `root` of the schema. Below the `root`, each path-enforced state is the name of one subdirectory, in the order of the dimensions. A dimension that is not path-enforced affects only its frontmatter key. The frontmatter is the authority, so a fix moves the file to match it ([validation specification][sokf:sokf-validation-spec]). A sidecar moves together with its asset.

- <a id="sokf_schema_26"></a>SOKF_SCHEMA_26 [ubiquitous]: the document SHALL be in the directory that its path-enforced states determine.
- <a id="sokf_schema_27"></a>SOKF_SCHEMA_27 [ubiquitous]: the schema SHALL declare `root`, as a `/`-rooted directory within its knowledge store, where it declares a path-enforced dimension.
- <a id="sokf_schema_51"></a>SOKF_SCHEMA_51 [ubiquitous]: the document SHALL lie under the `root` of its schema, where the schema declares a `root` and no path-enforced dimension.

A dimension can declare `archived`, a list of its states whose documents are history. A type schema with no lifecycle can declare `archived: true`, which makes every document of its type history. A tool that shows the knowledge, such as a graph, leaves out an archived document unless its user asks for it. The validator checks an archived document as it checks any other.

- <a id="sokf_schema_50"></a>SOKF_SCHEMA_50 [ubiquitous]: the schema SHALL list in the `archived` key of each dimension only the states of that dimension, and declare `archived: true` only where it declares no lifecycle.

## 7. Templates

A rule fixes the shape of a unit in one of three ways:

- `text`: The exact text.
- `template`: The text, with a typed placeholder, `${name:type}`, wherever it varies. It matches the whole unit.
- `pattern`: A regular expression (regex), for shapes that a template cannot express.

The type of a placeholder is either a named regex or a list of literal choices. A schema defines its own types in `types`, alongside the built-in types `text`, `word`, `int`, `slug`, `seq`, `store`, and `ref`. The type `store` matches the store prefix of a qualified id, and `ref` matches any target of a `sokf:` label ([format specification][sokf:sokf-format-spec] §8). A named placeholder, or a named group in a pattern, captures a key. Every shape treats each sequence of whitespace characters in a unit as a single space, and ignores the indent of each line of a fence.

A validator can supply its own types and line formats, and a schema that uses them depends on that validator.

The [tramlines specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-spec.md) states the rules that a shape and a type follow (TL_22, TL_105, and TL_29).

## 8. Id templates

A type schema can constrain `id` with a template, or fix it with `const`. A tool that creates a document fills in the placeholders of the template. The placeholder `${:seq}` stands for a number with `seq-width` digits, or three digits when `seq-width` is absent, and `${:slug}` stands for one or more slug words. Sequence numbers can have gaps. The [tramlines specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-spec.md) states the rules of an id template (TL_58, TL_114 to TL_116, and TL_122).

- <a id="sokf_schema_32"></a>SOKF_SCHEMA_32 [ubiquitous]: the document SHALL have a sequence number that no other document of its type has, where its id template contains `${:seq}`.
- <a id="sokf_schema_46"></a>SOKF_SCHEMA_46 [event]: WHEN a writer creates a document whose id template contains `${:seq}`, the writer SHALL give it a number one higher than the highest number in use for its type.

## 9. Schema terms

- <a id="sokf_schema_43"></a>SOKF_SCHEMA_43 [ubiquitous]: the schema SHALL conform to the tramlines-md grammar together with the [SOKF schema extension][sokf:sokf-extension].

The [tramlines specifications](https://github.com/six5536/tramlines/tree/main/knowledge/spec) define the terms of a schema. The SOKF schema extension adds these:

- `Governing keys`: `type`, `additional-files`, and `asset-files` (§3).
- `Lifecycle`: `root`, `path`, and `archived` (§6).
- `Links`: `links`, `cited-by` (its inverse), and `markers`.
- `Key scopes`: `knowledge` and `repository`.
- `Block kind`: `include`.
- `Frontmatter formats`: `repo-path` and `document-id`.
- `Reference types`: `store` and `ref`.

<!-- sokf:links -->

[sokf:sokf-extension]: /sokf-spec/sokf.tramlines.yaml
[sokf:sokf-format-spec]: /sokf-spec/sokf-format-spec.md
[sokf:sokf-validation-spec]: /sokf-spec/sokf-validation-spec.md
