---
name: sokf
description: This skill is the agent reference for SOKF, the Superdev Open Knowledge Format, and for sokf, the command that creates and checks SOKF knowledge. Read it before you run sokf, before you write or change a marker such as `// sokf-implements: <KEY>` in any file, and before you read or edit anything under a knowledge root. A knowledge root is a directory with a `manifest.sokf.yaml`. It covers the layout of a store, the shape of a document, the commands, the agent rules, links, markers, generated blocks, and schemas.
---

# SOKF v0.6 agent reference

SOKF, the Superdev Open Knowledge Format, is a format for a project's knowledge. The knowledge is a set of Markdown files, kept in the project's git repository, and each file describes one thing. `sokf` is the command that works on the knowledge: it creates stores and documents, finds documents, shows how they link to each other and to code, and validates every file, repairing what it can.

This reference describes what a store and a document look like, which command to run for each task, and which rules to follow when you write a document, a link, or a marker. `sokf validate` enforces every rule in this reference.

This reference uses these terms throughout:

- A _document_ is a Markdown file that starts with a block of YAML frontmatter. The frontmatter describes the document.
- A _knowledge store_ is a directory of documents with a manifest file, and that directory is the store's _knowledge root_.
- Each document has a type, and the type's _schema_ sets out what a document of that type must contain.
- A schema can give its type a _lifecycle_: the states that a document can be in. Each state can have its own directory.
- A _key_ is an identifier, such as the criterion id `CLI-1_AC-2`, that a schema rule extracts from a document's text.
- An _asset_ is a file that isn't Markdown, such as an image. The _sidecar_ of an asset is the document that sits next to the asset and describes it.
- A _marker_ is a comment in a source file that cites a key or a document.
- A _scaffold_ is the example text that `sokf new` takes from a schema to create a new document. You then replace the scaffold with your own text.

This reference has these sections:

- [Layout](#layout)
- [Document](#document)
- [Sources and attribution](#sources-and-attribution)
- [Commands](#commands)
- [Agent rules](#agent-rules)
- [New documents](#new-documents)
- [Links](#links)
- [Markers](#markers)
- [Generated blocks](#generated-blocks)
- [Schemas](#schemas)

## Layout

A knowledge store has this layout:

```
<KNOWLEDGE>/
  manifest.sokf.yaml   # Every store needs one. It contains sokf: "0.6", the store's name, and its scope, which is the store's owner. Both are slugs, lowercase words joined by hyphens, and the store's identity is scope/name.
  index.md             # a generated listing in every directory. `--fix` writes it, so never write it yourself.
  schemas/<TYPE>.md    # one Schema document for each type. Put schemas only here.
  **/*.md              # documents, whose frontmatter is YAML 1.2 without anchors, aliases, or tags
  <FILE>               # an asset: a non-.md file that a schema's `asset-files` glob matches
  <FILE>.md            # the asset's sidecar, a document. Every asset has one.
```

Every `.md` file under a knowledge root is a document, so give frontmatter to any Markdown file that you copy in. A document's schema gives the document's path a meaning by declaring a lifecycle, as the [schemas](#schemas) section describes. Without a lifecycle, the path has no meaning.

Each asset, such as an image, a PDF, or a dataset, has a sidecar: the document `<FILE>.md` next to the asset, whose `resource` is the asset's path. An asset without a sidecar is an error that `--fix` cannot repair. To repair it, create the sidecar with `sokf new <TYPE> --for <FILE> --write`, where `TYPE` is the type whose `asset-files` glob, a path pattern with wildcards, matches the asset.

An _additional file_, such as a README or a CHANGELOG, is a repository file without frontmatter whose path matches one of a schema's globs. An additional file is not a document. A glob must not match a document or an `index.md` inside a knowledge root.

## Document

The following example shows a document, with its frontmatter and its body:

```markdown
---
type: Subsystem # required, free-form
id: subsystem-logging # required: a unique kebab-case slug, which the schema's id template can constrain
title: Logging subsystem
description: The logging subsystem writes the log of every crate.
resource: /crates/lib/example-core/src/logging.rs # the file that the document describes, or a list of paths. Omit it for an abstract document.
tags: [core, logging]
status: draft # draft, stable, or deprecated. A document without a status is stable.
sources:
  - id: log4rs
    resource: https://crates.io/log4rs
    title: log4rs documentation
links:
  - rel: depends-on
    to: config
    note: The logging subsystem reads its settings from the configuration.
---

# Logging

The logging subsystem reads its settings from the [configuration][sokf:config].[^log4rs]

[^log4rs]: log4rs documentation

<!-- sokf:links -->

[sokf:config]: /knowledge/config.md
```

## Sources and attribution

Each entry of `sources` has `resource`, `id`, and `title`, and only those keys. `resource` is required: it is a path from the repository root, such as `/repo-root-path`, or a URL. `id` is required when the body cites the source. To attribute a claim, add a footnote labelled with the source's `sources[].id`, such as `claim.[^log4rs]`.

## Commands

sokf finds the repository root with git, and `--repo-root <DIR>` sets the root instead. The file `/.sokf/config.toml` lists the knowledge roots and the paths that sokf skips. By default, the only knowledge root is `/knowledge`. sokf prints each path relative to the repository root, starting with `/`, such as `/src/main.rs`. Write paths in this form in `to` and `resource`. Only `locate` prints a path on disk.

sokf has these commands:

- `sokf graph [START]` shows how the knowledge is linked, starting from `START`. It prints text by default, JSON with `--json`, or a Mermaid diagram with `--mermaid`. `START` can be a document id, a file, a key written as `#KEY`, or a directory:
  - With no `START`, `graph` shows the layout: each store, its directories, and its documents. Run it at the start of a session.
  - For a document id, `graph` shows the document's links, and the files whose markers cite the document, with the number of markers in each file.
  - For a file, `graph` shows the documents that describe or include the file, and each marker in the file, with the key that it cites and its line. Before you change a file, run `graph` on it.
  - For a `#KEY`, `graph` shows where the key is defined and what cites it, with the file and the line of each marker that cites it.
  - For a directory, `graph` shows the directory's layout.

  The options `-l`, `--to`, and `--edge` each take a predicate over the fields that `--json` shows, written in JMESPath, a query language for JSON. `sokf graph --json-schema` prints the JSON Schema of those fields, and every command that takes `--json` takes `--json-schema` too. Read the schema before you write a predicate or a `--query`. `graph` takes these options:
  - `--archived` includes archived documents, such as closed issues.
  - `--all` lists every entry of a long group.
  - `--depth <N>` follows links up to `N` steps away.
  - `-l "type=='Decision'"` keeps only the rows whose document matches.
  - `--to "type=='Subsystem'"` keeps only the links whose target matches.
  - `--edge "rel=='depended-on-by'"` keeps only the matching links. This example shows what depends on a document.
  - `--format '{id}  {title}'` sets the template of each document's line in text output.
  - `--json --query <EXPR>` selects fields from the JSON with the JMESPath expression `EXPR`.
- `sokf locate <ID>` prints the path on disk of the document whose id is `ID`. To read a document, run `cat "$(sokf locate <ID>)"`. `sokf locate --asset <ID>` prints the path of the asset that the sidecar `ID` describes, and `--json` prints both paths.
- `sokf new <TYPE> --slug <SLUG> [--state <KEY>=<STATE>]... --write` creates a document of the type `TYPE` from its schema, with the id, title, and states filled in, at the path that its states require. `SLUG` is the kebab-case part of the id, and each `--state` sets the lifecycle key `KEY` to `STATE`. Without `--write`, `new` prints the document instead of writing it. `--json` prints `id`, `path`, and `document`, and `--id-only` prints only the id. For a type whose schema declares no template for its id, pass `--id <ID>`. For a sidecar, pass `--for <ASSET>`, the path of the asset. For a schema, run `sokf new Schema --id <ID>`. When more than one store declares the type, put the store before it, as in `sokf new sokf-spec:Grammar`, or pass `--dir` or `--for` in that store. [New documents](#new-documents) says what to do with the document that `new` writes.
- `sokf validate [PATHS]` checks the knowledge. `--fix` repairs what it can: it rewrites generated blocks and indexes, moves each file to the directory that its lifecycle state requires, updates each sidecar's `resource`, and rewrites each heading whose text a schema takes from a frontmatter value. `PATHS` limits what `validate` reports and repairs, but `validate` still checks all the knowledge.
- `sokf init [DIR] [--name <NAME>] [--scope <SCOPE>]` creates a knowledge store in `DIR`, with its manifest and indexes, and adds `DIR` to the knowledge roots in `/.sokf/config.toml`. `DIR` is `/knowledge` by default. `NAME` is the store's name, a slug that defaults to the last part of `DIR`. `SCOPE` is the slug of the store's owner, which defaults to the owner of the git remote `origin`. Use `init` instead of writing a manifest by hand, because `init` also adds the root.
- `sokf harness install claude` adds the following to the repository's Claude Code files: this skill, a block of instructions for the agent, a stop hook that runs `sokf validate --fix` at the end of each turn, and permission to run `sokf`. Name another harness, such as `codex`, `cursor`, `gemini`, `copilot`, `factory`, `pi`, `opencode`, `kilo`, `qwen`, `devin`, or `agents`, or several, or `all`, to install into its files instead. After you upgrade `sokf`, run the same command again. `sokf harness status claude` shows what is out of date.
- `sokf info grammar` prints the grammar of the schema language in three parts. First comes the core grammar of tramlines, a language for describing documents. Next comes the grammar of tramlines-md, the extension of tramlines for what a Markdown document has. Last come the keys that SOKF adds to those two grammars. `sokf info formats [--schema <ID>]` prints the line formats and placeholder types that a schema can use.

`sokf validate` prints each error as one line, `path:line: error: message (KEY)`. The key identifies the broken rule, such as `SOKF_FORMAT_53` from the specifications in `sokf-spec/`, or `TL_56` or `TLMD_152` from the tramlines specifications. When a schema rule causes the error, the line gives the schema id in parentheses before the key. The parentheses also hold the rule inside the schema when the error has one. `validate` counts warnings, and `--warnings` lists them too. `--json` prints one object with `errors`, `warnings`, and `fixes`, and `--query <EXPR>` selects fields from it with a JMESPath expression. sokf exits with code 1 when the knowledge has errors, and with code 2 on a usage error.

## Agent rules

Follow these rules when you write a document:

- Write each fact in one place. If a README or another file already states a fact, summarise the fact and cite that file in `sources` instead of copying the fact.
- Don't record an author or a date in the frontmatter, because git already records who changed each file and when.
- You can choose the `id` of a new document, but never change an existing `id`, even when you rename or move the document, because other documents and markers cite it.
- Create documents with `sokf new`, because `new` allocates the id and writes the file where its states require. If you place the file yourself, get the id from `sokf new <TYPE> --slug <SLUG> --id-only`. Never guess a sequence number, because sokf allocates the next free one, and a guessed number can collide with another document's.
- Never add, edit, reorder, or delete a `verified` entry. Each entry records that a human or a process checked the document, and only that human or process writes it. `verified` has the form `[{ by: human:<ID>|process:<ID>, at: ISO8601 }]`, and an entry older than the last change to the content has lapsed. A sidecar's `verified` covers its asset too, and lapses when either file changes.
- Never write `generated`, or the manifest's `producer`, `generated`, or `counts`. Export tooling writes these keys when it exports a store for publication, and the working tree must not hold them.
- To change a document's lifecycle state, set the state keys in its frontmatter and let `--fix` move the file. Never move the file by hand. `--fix` moves a sidecar together with its asset.
- You can write every other key unless the project's conventions restrict it.

## New documents

Never copy a scaffold by hand, because `sokf new` adapts the scaffold to your document's states. `new` keeps or drops each part marked `<!-- sokf:only … -->` according to those states, and strips the markers. A `sokf:only` marker left in a document is an error.

After `sokf new` writes a document, replace the scaffold's example text and references with your own, because `validate` reports any that you leave. The new document also contains comments that you resolve as you write:

- A `<!-- sokf:guidance … -->` line (`# sokf:guidance …` in frontmatter) comes before each part that it guides. Read each guidance line, replace that part's example text with your own, and then delete the line. A guidance line left in a document is an error.
- A `<!-- sokf:optional -->` … `<!-- /sokf:optional -->` region contains a part that the schema does not require. Either keep its content and delete both markers, or delete the whole region. A region left in a document produces a warning, and an unpaired marker is an error.

## Links

Follow these rules when you link one document to another:

- `rel` is the kind of link, and each kind has an inverse: relates-to↔relates-to, part-of→has-part, depends-on→depended-on-by, references→referenced-by, supersedes→superseded-by, implements→implemented-by, and contradicts↔contradicts. A custom kebab-case rel is allowed, but it produces a warning. There is no `derived-from` rel: use `sources` instead. Declare each link once, from one side, because sokf derives the inverse.
- `to` is the target's `id`, with its store before it when the target is in another store, such as `sokf-spec:glossary`. `--fix` rewrites a path to a document as the document's id.
- For every `links` entry, also link to the target in the body with `[text][sokf:<ID>]`. The opposite isn't required: a body link without a `links` entry counts as an untyped relates-to link.
- In the body, link to documents and assets with a `sokf:<ID>` link, never with a path, because a `sokf:` link keeps working when its target moves. `--fix` rewrites an inline path to a document or an asset as a `sokf:` link. Link to anything else with a path or a URL. A path that starts with `/` resolves from the repository root, and a relative path resolves from the file that contains it. Every path in `resource`, `sources[].resource`, or a body link must point to an existing file or directory.
- `sokf:<ID>` refers to a document's content. A sidecar's content is its asset, so `![Flowers][sokf:flowers]` shows the picture and keeps working when the picture moves. `sokf:<ID>#`, with an empty fragment, refers to the sidecar file itself. An image reference must use a sidecar's id.
- To cite part of a document, add a fragment that names a heading or a key. A fragment is always a slug in the GitHub style: lowercase, with letters, digits, hyphens, and underscores kept, and spaces turned into hyphens. Write `[CLI-1_AC-2][sokf:req-cli#cli-1_ac-2]` for a key, or `[sokf:req-cli#acceptance-criteria]` for a heading. Within its store, you can cite a key whose schema declares `unique: knowledge` or `unique: repository` without the document's id, as `[sokf:#cli-1_ac-2]`. A fragment alone, `[text](#cli-1_ac-2)`, refers to the current document. The validator resolves every fragment, and `--fix` rewrites a fragment written as the key's text, such as `#CLI-1_AC-2`, as its slug.
- To refer to another store, put the store name before the id: `[text][sokf:sokf-spec:glossary]`, `[sokf:sokf-spec:#user]`, `<!-- sokf:include sokf-spec:graph-schema -->`, or `to: sokf-spec:glossary`. Write a store in another scope as `scope/name`. Each target has one spelling, the shortest, and `--fix` writes that spelling. `sokf locate` and `sokf graph` accept every form.

## Markers

A marker is a comment line in a source file, outside any knowledge root, that cites one key or one document. For example, `// sokf-implements: CLI-1_AC-2` goes above the code that implements the criterion `CLI-1_AC-2`, and `# sokf-tests: CLI_P-5` goes above the test of the property `CLI_P-5`. `sokf validate` checks every marker, but `--fix` never rewrites one, so fix markers by hand.

Follow these rules when you write a marker:

- The lowercase tag `sokf-<REL>:` is the first word on the line after any whitespace and ASCII punctuation, such as a comment leader. `REL` is a `links` rel, such as `implements`, `tests`, or `references`, and a custom rel produces a warning. After the tag come whitespace and exactly one reference, and only punctuation, such as `*/` or `-->`, can follow the reference. Two keys need two marker lines. A tag in another case, such as `SOKF-Implements:`, is an error.
- A bare lowercase kebab-case word in a reference is a document id, such as `design-cache`. Any other bare word is a key written as its text, such as `CLI-1_AC-2` or `SOKF_FORMAT_96`, and it matches only keys whose schema declares `unique: repository`. Every form that works after `sokf:` also works here: `id#fragment`, `store:id`, `#key`, and `store:#key`. sokf looks up a reference without a store in every store. If two stores have documents with the same id, put the store before the id. Fragments match by slug, so you can write a key as its text.
- If `validate` reports that no `sokf-implements:` or `sokf-tests:` marker cites a key, the key's schema requires one. Add the marker above the code that implements the key or the test that tests it. Never put one on code that does neither.
- Keep markers accurate: when you change what code implements or tests, update its markers in the same edit. When a key is renamed or removed, `sokf validate` reports every marker that cites it.
- To write text that looks like a marker but is not one, such as a sample line in a test or a document, put it between `sokf:ignore-begin` and `sokf:ignore-end` lines, or after a `sokf:ignore-next-line` line. In an HTML or Markdown file, write each directive in a comment of its own, such as `<!-- sokf:ignore-begin -->`. Ignore regions do not nest.
- With `[markers] check = "warn"` in `/.sokf/config.toml`, `validate` reports marker failures as warnings, and with `"off"` it skips markers.

## Generated blocks

A generated block is a part of a document whose content `--fix` writes. Write the comment lines that start a generated block, never its content:

- A _definition block_ ends every document that has `sokf:` body links. It is the line `<!-- sokf:links -->`, followed by one `[<LABEL>]: /path` line for each cited label. Write the `sokf:` links, and `--fix` writes the block.
- An _anchor_, `<a id="<SLUG>"></a>`, marks each key in the document, once. `--fix` writes, moves, and removes anchors, so never touch one yourself.
- An _index_, the `index.md` in every directory, lists the directory's documents and subdirectories. `--fix` writes it. Read an index to see what its directory contains, but never edit one.
- An _include block_ runs from `<!-- sokf:include <ID | /PATH[#REGION]> -->` to `<!-- /sokf:include -->`, and `--fix` fills it. Includes can nest, but a cycle is an error. The path form identifies a source file, never a document, an additional file, or an asset. To include a document or an asset, use its id. A block whose target is a sidecar's id includes its asset, a block whose target is `<ID>#<NAME>` includes the region `NAME` of the asset, and a block whose target is `<ID>#` includes the sidecar's body. A region in a file lies between a `sokf:begin <NAME>` comment line and a `sokf:end <NAME>` comment line. The name matches `[A-Za-z0-9_-]+`. The included text leaves those two lines out. An include comment inside a fenced code block has no effect.

## Schemas

Every type in use, except `Schema`, has exactly one schema: a `type: Schema` document whose body is one YAML block fenced with four backticks. The schema declares the type that it defines as `frontmatter: { type: { const: <TYPE> } }`, a fixed value of the `type` key. Every document must validate against its type's schema.

A schema is strict by default: the validator requires its declared sections, keys, and blocks, refuses undeclared ones, and enforces their order. To relax that strictness, declare `optional`, `repeat`, or `*-open`.

A schema is made of rules. A rule describes the required form of a heading, paragraph, item, row, line, or frontmatter value with one of three _shapes_: `text` (exact), `template` (the text with typed placeholders, `${name:type}`, matched whole), or `pattern` (a regex, for what a template cannot express). Before sokf matches a text against a shape, sokf collapses each run of whitespace in the text to one space, and removes the indent from a line inside a fence.

A placeholder type is a named regex or a list of choices, taken from the schema's `types`, `/.sokf/formats.yaml`, the shared formats, or the built-ins `text`, `word`, `int`, `slug`, `seq`, `store`, and `ref`. Write a reference as `[${}][sokf:${:ref}]`, never as a regex, because the `ref` type already matches every target that a `sokf:` link can have.

`sokf new` builds a scaffold only from the schema's rules: each part's text is the `text` or `example` of the rule that requires it. A rule with a `template`, `pattern`, or `source-pattern` needs an `example`. sokf checks each rule's `example` against the rule, shows the example whenever the rule fails, and copies it into every new document.

Make each example concrete, because agents imitate what they are shown: write a line that a real document would contain about a real subject, never a slot (`<name>`), an instruction (`Describe the component.`), or filler (`One item.`).

An `example` of a block or a unit rule can be a list of strings, and `sokf new` writes one unit for each string, in order. A section that declares `repeat: true` can also declare `examples`, a list of texts. Each of these texts begins with a heading of the section's level. `sokf new` writes the texts unchanged after the occurrence that it derives from the rules. A schema can also list `examples`, real documents of the type for a reader to look at, and `sokf new` never copies them.

Put what an example cannot show in `guidance`, on the schema or on a section, preamble, block, frontmatter key, lifecycle dimension, or link. Write guidance in the voice of the document, because guidance can leak into the text that a writer copies. Guidance follows these rules:

- It describes the part in the third person and the present tense.
- It states each rule as a fact about the part, with its reason.
- It never addresses the writer.
- It never begins a sentence with a verb.
- It never includes a counter-example.

Put a ban in `prohibited-patterns`, a list of regexes.

A schema can describe additional files instead of a type of document: it declares `additional-files` globs and no `type` const, and its rules apply to the files that the globs match. A schema can also declare `asset-files` globs beside its `type` const. The files that those globs match are assets, and the schema's documents are their sidecars.

A schema can also declare the following:

- `root` is the `/`-rooted directory of the store that holds the type's documents. Without a `path: true` dimension, a document can lie at any depth under it. `sokf new` writes a new document there, and `--fix` moves a document that lies outside it into it.
- `lifecycle` declares ordered dimensions. Each dimension has a `key`, its `states`, an optional `path: true`, and an optional `archived: [states]`. Each dimension becomes a required frontmatter key whose value is one of its states. When a dimension has `path: true`, the schema also declares `root`, and the document's state for that dimension is the name of the directory under `root` that holds the document. `sokf graph` hides a document in an `archived` state. A schema with no lifecycle can declare `archived: true` to archive all its documents. Any rule can have `only: { state: [closed] }`, so that it applies only in those states.
- `plans-paths: [{ section: files, only: { state: [open] } }]` lets a document plan the files that its work creates: while the document is in those states, each `/`-rooted path that a link in the section with that `rule-id` names counts as existing, so a design can link it, list it in `resource`, or include it with a sketch before the code exists. `current-links: ['^Planned: ']` makes each matching paragraph link no archived document.
- `links` declares the links that a document must or can declare, one entry for each `rel`, with `to-type`, `min`, `max`, and `only`.
- `cited-by` is the inverse of `links`: it declares what must cite the type's documents. Each of its entries has exactly one of `from-type` and `from-marker`. `from-type` is a type, and `from-marker` is a marker rel. An entry can also have `keys`, which is a rule id, and it can have `only` and `severity`. With `from-type`, a document of that type, in any store, must link with a `sokf:` body link to each key that the rule given in `keys` extracts. Without `keys`, the document must link to the cited document itself. With `from-marker: implements`, a `sokf-implements:` marker must cite each one instead. A marker of any other rel does not count. Included text does not count either, and neither do a document's links to itself.
- `markers: [implements, tests]` limits which marker rels can cite the type's documents and keys, and `markers: []` refuses all markers.
- `frontmatter: { id: { template: "issue-${:seq}-${:slug}" } }` declares the id template, the form of the id. The template uses only placeholders that `sokf new` fills. A placeholder named after a lifecycle key, such as `${kind}`, stands for the document's state for that key: `issue-${:seq}-${kind}-${:slug}`. The `seq` placeholder is a sequence number that is unique within the type. It is `seq-width` digits wide, and `seq-width` is 3 by default. Write a fixed id as `id: { const: … }`.
- `heading` is exact text, `{ from: <KEY> }` for a heading whose text is the value of the frontmatter key `KEY`, or a heading rule such as `{ template: '${n:int}. ${}', key: n, example: "8. Links" }`. `key` names the placeholder or named group whose matched text becomes the key. `key-derives-from`, beside `key`, requires the key's prefix to be a key that another rule extracts, or a frontmatter value, so that the criterion `CLI-1_AC-2` derives from its part `CLI-1`.
- `sections` with `blocks` makes each section's body an ordered list of typed blocks, such as prose, lists, tables, and code, with rules for each unit. A _unit_ is one item, row, or paragraph of a block. `key` extracts identifiers as keys, and `unique` widens the scope in which each key must be unique. A key's slug must differ from every heading slug and every other key's slug in the document.
- A block can apply its rules as a grammar: it can take `rules` instead of `each` or `sequence`, and a `code` or `html` block can apply `lines` rules to its content lines. sokf tries the rules in order. A rule's _guard_ is its `select-pattern`, a regex that selects units by something other than the rule's shape. A rule with a shape and no guard accounts for the units that its shape matches. A rule with a guard accounts for the units that its guard selects. A rule with only a guard and `min` or `max` counts the units that its guard selects, but accounts for none of them. A unit or line that no rule accounts for is an error unless the block sets `units-open` or `lines-open`.
- `formats` declares line formats. Each format has either one shape and an `example`, or `variants`: shapes that sokf tries in order, each with its own `example`. A schema-local entry replaces the shared one whole, unless it declares `patch: true` and states only its difference as a JSON merge patch (RFC 7386), in which `null` deletes a key.
