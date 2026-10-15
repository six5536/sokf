---
type: Grammar
id: sokf-extension
title: 'SOKF schema extension: usage guide'
description: What SOKF adds to the tramlines-md grammar, section by section, with a reference at the end.
resource: /sokf-spec/sokf.tramlines.yaml
version: "0.6"
---

# SOKF schema extension: usage guide

This guide is for anyone, human or agent, who writes a schema of the
Superdev Open Knowledge Format (SOKF) or a document that a schema governs.

A SOKF schema is written in the tramlines-md grammar, which lays the
Markdown format on the tramlines core grammar. SOKF adds to that grammar in
`sokf.tramlines.yaml`, the file that this guide describes. The [tramlines specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-spec.md)
explains shapes, formats, lifecycles, and examples, and the
[tramlines-md specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-md-spec.md) explains
sections, blocks, keys, and scaffolds. This guide covers only what SOKF adds.
The [schema specification][sokf:sokf-schema-spec] defines when a schema is
required and what it governs, and the
[validation specification][sokf:sokf-validation-spec] defines how a
validator checks a document against it.

Where this guide and the grammar differ, the grammar governs.

- [1. What SOKF adds](#1)
- [2. What a schema governs](#2)
- [3. Writing a Schema document](#3)
- [4. Key scopes and citations](#4)
- [5. Include blocks and images](#5)
- [6. Links](#6)
- [7. Lifecycle paths](#7)
- [8. Assets and sidecars](#8)
- [9. Workflow](#9)
- [10. Pitfalls](#10)
- [11. Reference](#11)

## <a id="1"></a>1. What SOKF adds

`sokf.tramlines.yaml` extends the tramlines-md grammar and leaves every
definition of the grammar as it is. The extension adds the following:

- `schema-document`, the schema for Schema documents. A Schema document is
  any `.md` file in `schemas/` at the root of a knowledge store, other than
  `index.md` ([format specification][sokf:sokf-format-spec] §2). The
  validator checks the fenced block of a Schema document against the grammar
  and this extension together.
- The keys `additional-files` and `asset-files`, which declare what a schema
  governs (§2).
- The keys `root` of a lifecycle and `path` of a dimension (§7).
- The `links` rules (§6).
- The key scopes `knowledge` and `repository` (§4), the block kind `include`
  (§5), and the frontmatter formats `repo-path` and `document-id` (§3).
- The reference types `store` and `ref` (§4).

SOKF uses the markers of tramlines, the schema language that SOKF extends,
in the namespace `sokf`: `sokf:only`,
`sokf:guidance`, and `sokf:optional`, the include block `sokf:include`, the
definition block `sokf:links`, and labels with the scheme `sokf:`
([format specification][sokf:sokf-format-spec] §8, §9). The schema language
is the tramlines core grammar and the tramlines-md grammar, extended by this
file, and `sokf info grammar` prints all three.

## <a id="2"></a>2. What a schema governs

A schema governs either the documents of one type, through its `type`
const, or a set of additional files
([schema specification][sokf:sokf-schema-spec] §3). Additional files are
repository files without frontmatter, such as a README or a changelog, and
`/`-rooted globs select them. A glob cannot match a document, or an
`index.md` inside a knowledge store, because the tool generates indexes
([format specification][sokf:sokf-format-spec] §9). A type schema can also
declare `asset-files`: globs that select the non-document files stored
beside its documents. Each document of such a type is the sidecar of one
asset (§8).

A schema takes exactly one of three forms: the `type` const, or
`additional-files` on its own, or the `type` const with `asset-files`. The
validator checks an additional file against its schema, but the file is not
a document: it has no `id`, no `links`, and no lifecycle.

## <a id="3"></a>3. Writing a Schema document

A schema is the body of a document of type `Schema`: a single YAML fence
opened with four backticks.

<!-- prettier-ignore -->
`````markdown
---
type: Schema
id: schema-decision
title: Decision Schema
description: One architectural decision, with its context, its choice, and its consequences.
---

````yaml
line-limit: 120
guidance: Each document records one decision, because a reader cites and supersedes each decision on its own.

frontmatter:
  type:
    const: Decision

sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: Context
    level: 2
    blocks:
      - kind: prose
        max: 2
        example: Two binaries doubled the release surface and users installed the wrong one.
````
`````

- The frontmatter `description` of the Schema document tells its reader
  what the schema is for. The `guidance` of the schema is written for the
  person who writes a document.
- SOKF reserves its core keys (`type`, `id`, `title`, `description`,
  `tags`, `resource`, `status`, `sources`, `links`, `verified`), and the
  validator checks them in every document, whether or not a schema declares
  them. A schema declares `type` to say what it governs, declares `id` only
  to give a template or a `const`, and declares any other core key only to
  require it or to narrow its value.
- SOKF adds two frontmatter formats: `repo-path`, a `/`-rooted path to a
  file in the repository, and `document-id`, the id of a document in the
  knowledge store. A scaffold contains placeholders in place of a real file or
  document, so the validator checks neither format in a scaffold.
- A tool that creates a document writes `type`, `id`, and `title` first. The
  `type` is the const. The `id` is the id template with its sequence number
  set to one, or `TYPE-example` when there is no template, where `TYPE` is
  the type as a slug. The `title` is the words of the slug in the id, which
  the tool makes into a title. The tool writes the edges of
  §6 last. For a real new document, it fills in the next sequence number
  ([schema specification][sokf:sokf-schema-spec] §8).

For the slug `retry-logic`, a tool that creates a document derives the
following scaffold from an Issue schema with the lifecycle of §7 and prints
it. The SOKF keys come first, and the `sokf:guidance` and `sokf:only`
markers appear where the schema places them:

<!-- prettier-ignore -->
```markdown
---
type: Issue
id: issue-001-retry-logic
title: Retry logic
state: open
kind: bug
---

<!-- sokf:guidance The writer replaces each of the following examples with their own text. The issue describes one bug or one feature, and what closed it. -->

# Retry logic

The index writer drops the entry of a moved file.

<!-- sokf:only kind=bug -->
## Reproduction

1. Move a document with `--fix`.
```

## <a id="4"></a>4. Key scopes and citations

`unique: knowledge` or `unique: repository` widens the scope in which a key
must be unique beyond its document. When two documents hold the same key,
the validator reports the clash on both of them. A document can cite a key
that is unique beyond its own document in bare form: `sokf:#KEY` from the
same store, and `sokf:STORE:#KEY` from another
([format specification][sokf:sokf-format-spec] §8). `KEY` is the slug of
the key, and `STORE` is the name of the key's store. The scope `repository`
widens where the key is unique, and a key of that scope is cited in the same
way as a key of scope `knowledge`.

**Citations.** A body link cites a key or a heading of another document
with a fragment ([format specification][sokf:sokf-format-spec] §8):

```markdown
Implements [CLI-1_AC-2][sokf:req-cli#cli-1_ac-2]; see [the criteria][sokf:req-cli#acceptance-criteria].
A repository-unique key needs no document: [CLI-1_AC-2][sokf:#cli-1_ac-2].
```

A fragment refers to an anchor by its slug. A slug follows the rule
that repository hosts use for headings: lowercase the text, keep letters,
digits, hyphens, and underscores, drop other characters, and turn spaces
into hyphens. The fragment resolves to a captured key first, and to
a heading otherwise. Each captured key has a generated anchor, which `--fix` writes
before the key with the slug as its id
([format specification][sokf:sokf-format-spec] §9). No schema rule reads
the anchor. These cases are errors:

- A key whose slug equals the slug of another key or of a heading in the
  same document. The error is on that document.
- A fragment written as the text of the key, such as `#CLI-1_AC-2`. `--fix`
  rewrites it to the slug.
- A fragment that matches two headings with the same text. The error is on
  the citing document. To cite a repeated section, cite the key of its
  heading.

The `sokf:links` definition block holds one entry for each citation,
fragment included.

SOKF adds two built-in types, which are reserved in the same way as the
built-in types of tramlines:

| Type    | Matches                                                              |
| ------- | -------------------------------------------------------------------- |
| `store` | the store prefix of a reference, `name:` or `scope/name:`, or no prefix ([format specification][sokf:sokf-format-spec] §8) |
| `ref`   | any target of a `sokf:` label: an optional store prefix followed by an id with an optional fragment, or a bare fragment |

To match a reference, use the reference types, as in `[${}][sokf:${:ref}]`,
or `sokf:${:store}contract-${:slug}` for a link to a contract, instead of a
regular expression of the label. Then no schema has to change when the form
of a reference changes.

## <a id="5"></a>5. Include blocks and images

SOKF adds the block kind `include`, which declares an include block
([format specification][sokf:sokf-format-spec] §9). The tool generates the
content of an include block, so no rule reads it. Include blocks and images
take a `source-pattern`:

```yaml
- kind: include
  source-pattern: '^/crates/.*\.rs#' # a region of a Rust file
  example: /crates/lib/sokf-core/src/check/mod.rs#check
```

When the target of an image or an include is the id of a sidecar,
qualified or not, the `source-pattern` matches the path of the asset. For
an include that selects a region, the path has `#<region>` appended
([format specification][sokf:sokf-format-spec] §8, §9). The id of a
document never matches, so `^/` accepts a file or an asset and rejects a
document.

## <a id="6"></a>6. Links

`links` governs the edges of a document:

```yaml
links:
  - rel: part-of
    to-type: Architecture
  - rel: implements
    to-type: [Contract, Plan]
    guidance: The contract or plan this subsystem exists to deliver.
  - rel: relates-to
    min: 0
```

An entry requires at least `min` edges (one when it declares no `min`) and
allows at most `max`. `min: 0` allows an edge without requiring one. A link
whose `rel` matches no entry is an error, unless the schema declares
`links-open: true`. An edge counts only if it resolves, and, where the entry
names a `to-type`, only if it resolves to a document of that type. A tool
that creates a document writes one edge for each entry into the scaffold,
including those with `min: 0`, so that the author sees every edge they can add. The target of each edge is a
placeholder id, which the validator reports until the author replaces it
with the id of a real document.

### 6.1 Cited by

`cited-by` is the inverse of `links`: it declares what has to cite a
document of this type.

```yaml
cited-by:
  - from-type: Stories
    keys: criteria
    only: { scope: [public] }
  - from-type: ArchitectureOverview
```

An entry with `keys` requires at least one document of a type in `from-type`
to cite every key that the named rule captures. The citation uses the slug
of the key or, for the key of a heading, the slug of the heading. `keys` is
the name of a rule of this schema that captures a key, as `for-each-key-of`
does. An entry without `keys` requires a citation of the document itself,
with any fragment or none, because citing a part cites the whole. `only`
limits the check to cited documents in the given states.

A citation is a resolved `sokf:` body link in another document, in any
store. These do not count: included and generated text (so an index never
satisfies a requirement), a path link, and a link from a document to
itself. A citing document that has errors of its own still counts, and the
validator reports those errors on it. The validator reports a missing
citation on the uncited key at its line, or on the uncited document with no
line, with the `severity` of the entry. The check runs after the validator
checks every document, so a validator that reports on only some paths still
counts every citation.

An entry that declares `from-marker` in place of `from-type` counts markers
([format specification][sokf:sokf-format-spec] §9.5) instead of body links.
Only a marker of the named relationship counts, so a test of a criterion
never counts as its implementation:

```yaml
markers: [implements, tests]
cited-by:
  - from-marker: implements
    keys: criteria
    severity: warn
```

`markers` lists the relationships of the markers that can cite a document
of the type or one of its keys. The validator reports a marker of another
relationship as an error at the file and line of the marker. `markers: []`
refuses every marker, and a schema that leaves out `markers` admits every
one. A validator that does not read markers skips every `from-marker`
entry, and warns once on each schema that declares one.

## <a id="7"></a>7. Lifecycle paths

A type schema can lay out the store by its lifecycle:

```yaml
root: /knowledge/issues
lifecycle:
  - { key: state, states: [open, closed], path: true }
  - { key: kind, states: [bug, feature] }
```

A dimension with `path: true` adds one directory level under `root`, in the
order of the dimensions. The frontmatter determines where a file belongs, and
`--fix` moves a file that is in the wrong place. A schema declares `root`
exactly when a dimension declares `path`, and `root` lies inside the
knowledge store. Only a type schema can declare a lifecycle.

A dimension can list the states whose documents are archived. `sokf graph`
leaves those documents out unless `--archived` is given:

```yaml
lifecycle:
  - { key: state, states: [open, closed], path: true, archived: [closed] }
```

Each archived state is one of the states of the dimension. A type schema
with no lifecycle declares `archived: true` to archive every document of its
type.

## <a id="8"></a>8. Assets and sidecars

A type schema can declare `asset-files`: `/`-rooted globs inside the
knowledge store that select files that are not `.md` files. Each file that
a glob matches is an asset. Its sidecar is the `.md` file next to it, named
after the asset with `.md` appended, and the sidecar is a document of this
type ([format specification][sokf:sokf-format-spec] §2,
[schema specification][sokf:sokf-schema-spec] §3). The type of a sidecar is
the type of the schema whose glob matches the asset. Under the following schema, `flowers.png.md` next to
`/knowledge/media/flowers.png` has `type: Image`, and every document of type
`Image` is a sidecar.

<!-- prettier-ignore -->
````yaml
line-limit: 40
frontmatter:
  type:
    const: Image
  resource:
    example: /knowledge/media/flowers.png
asset-files: ["/knowledge/media/**/*.png", "/knowledge/media/**/*.jpg"]
preamble:
  guidance: The sidecar has one line of caption for the picture.
  blocks:
    - kind: prose
      max: 1
      example: Wild flowers on the ridge, June.
````

- A sidecar requires `resource`, the path of its asset. `--fix`
  rewrites it after a `git mv` of the two files.
- When a lifecycle places the sidecar in another directory, `--fix` moves
  the asset along with it. With a lifecycle `root`, every glob begins with
  `root` followed by `/**/`.
- A glob cannot match a reserved file, a file under `schemas/`, a `.md`
  file, or an additional file, and no file can match the globs of two
  schemas.
- A scaffold or an example has no real asset beside it, so the
  validator does not check that its `resource` exists.
- The schema defines the body of a sidecar. `blocks: []` under `preamble`
  makes a sidecar consist of frontmatter only.

`sokf:ID`, where `ID` is the id of a sidecar, refers to the asset, not its sidecar, so
`![Flowers][sokf:flowers]` embeds the picture and keeps working after a
move. `sokf:flowers#`, with an empty fragment, links to the sidecar itself,
and `sokf:flowers#caption` links to a heading in it.

An include block has three forms for an asset:

- An include of `flowers` inserts the asset, fenced according to its
  extension.
- An include of `flowers#` inserts the body of the sidecar.
- An include of `rates#q3` inserts the region `q3` of a text asset.

An image reference with no `links` entry counts as a
`relates-to` edge. To declare the edge in `links`, use the core relationship
`has-part`.

## <a id="9"></a>9. Workflow

Write the schema first, and revise it until both the schema and the
scaffold derived from it pass the validator. A tool that creates a document
prints the scaffold, and sokf is one such tool: its [README](/README.md)
lists its commands. Then set the rules of the schema to `severity: warn`
and make the store conform to the schema. The warnings let the store's
existing documents pass the validator while you bring them into line. After
the store conforms, remove `severity: warn`, so that a failure is an error
again.

## <a id="10"></a>10. Pitfalls

- Do not repeat the core frontmatter keys in a schema, because the validator
  checks them in every document (§3).
- Do not copy the `sokf:only` markers of a scaffold into a document, and do not
  leave the `sokf:guidance` lines of a scaffold in a document.

## <a id="11"></a>11. Reference

The following tables and lists summarise each key that SOKF adds. The full
definition of a key is its `doc` entry in the extension, which a tool can
print.

### Schema

| Key                | Type                 | Meaning                                                          |
| ------------------ | -------------------- | ---------------------------------------------------------------- |
| `additional-files` | string or list       | `/`-rooted globs that select additional files, in place of a `type` const (§2). |
| `asset-files`      | string or list       | `/`-rooted globs inside the knowledge store that select assets (§8). |
| `root`             | string               | The directory under which dimensions with `path: true` lay out documents (§7). |
| `links`            | list of _link_       | The edges that are required and allowed (§6).                    |
| `links-open`       | boolean              | Allows any `rel` (§6).                                           |
| `cited-by`         | list of _citation_   | What has to cite the documents or keys of this type (§6.1).      |
| `markers`          | list of strings      | The relationships of the markers that can cite this type (§6.1). |
| `archived`         | boolean              | Archives every document of the type. Allowed only when the schema has no lifecycle (§7). |

### Values SOKF adds

- `unique` (heading, unit): `knowledge`, `repository` (§4).
- `kind` (block): `include`, which takes `source-pattern` (§5).
- `format` (frontmatter constraint): `repo-path`, `document-id` (§3).
- `path` (lifecycle dimension): `true` adds a directory level under `root`
  (§7).
- `archived` (lifecycle dimension): the states whose documents are archived
  (§7).

### Link

`rel` (required), `guidance`, `to-type`, `min`, `max`, `only`,
`severity`.

### Citation

`from-type` or `from-marker` (exactly one), `keys`, `only`, `severity` (§6.1).

<!-- sokf:links -->

[sokf:sokf-format-spec]: /sokf-spec/sokf-format-spec.md
[sokf:sokf-schema-spec]: /sokf-spec/sokf-schema-spec.md
[sokf:sokf-validation-spec]: /sokf-spec/sokf-validation-spec.md
