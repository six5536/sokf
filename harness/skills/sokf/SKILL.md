---
name: sokf
description: Read this skill before you run sokf, before you write or change a `sokf-` marker, such as `sokf-implements` or `sokf-tests`, in any file, and before you read or edit anything under a knowledge root, a directory with a `manifest.sokf.yaml`.
---

# SOKF v0.7 agent reference

SOKF, the Superdev Open Knowledge Format, is a format for a project's knowledge, kept in its git repository as Markdown files that each describe one thing. `sokf` creates stores and documents, finds documents, shows how they link to each other and to code, and validates every file, repairing what it can.

This reference says which command to run for each task. It also gives the rules for writing documents, links, markers, and schemas that the tool doesn't state itself. `sokf validate` doesn't check every one of these rules, so follow them even when it reports nothing. For everything else, read `sokf <command> --help`, `sokf guide`, `sokf info grammar`, and each finding of `sokf validate`.

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
- [Sources and citations](#sources-and-citations)
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
  manifest.sokf.yaml   # sokf: "0.7", name, and scope (the store's owner), both slugs. `sokf init` writes it. Optional: `unique: knowledge` keeps the store's keys unique only within it, `extends: <scope>/<name>` inherits that store's schemas, and `primary-language: de` names the language of its text (default `en`).
  index.md             # generated in every directory by `--fix`. Never edit it.
  schemas/<TYPE>.md    # one Schema document for each type
  **/*.md              # documents
  <FILE>               # an asset: a non-.md file that a schema's `asset-files` glob matches
  <FILE>.md            # the asset's sidecar, a document. Every asset has one.
```

A file that must stay outside every store, such as a JSON Schema that consumers fetch from a fixed path, can't be an asset. Describe it with an ordinary document whose `resource` is the file's path, and link to the file by its path. `sokf graph` on the file shows the document.

## Document

The following example shows a document, with its frontmatter and its body:

```markdown
---
type: Subsystem # required, free-form
id: subsystem-logging # required: a unique kebab-case slug, which the schema's id template can constrain
title: Logging subsystem
description: How each crate writes its log, and which configuration settings it reads.
resource: /crates/lib/example-core/src/logging.rs # the file that the document describes, or a list of paths. Omit it for an abstract document.
tags: [core, logging]
status: draft # draft, stable (the default), or deprecated. validate warns at each link, marker, and include that cites a deprecated document.
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

## Sources and citations

Each `sources` entry has `resource`, a path from the repository root or a URL, and `title`, and an `id` when the body cites it. Cite the source of a claim with a footnote labelled with the id, such as `claim.[^log4rs]`.

## Commands

`/.sokf/config.toml` lists the project's own knowledge roots, `/knowledge` by default, and the paths that sokf skips. Another tool can add roots in a file below `/.sokf/roots.d/`, and paths to skip in a file below `/.sokf/excludes.d/`. Never edit a file there that another tool wrote, because the tool rewrites it. sokf prints each path from the repository root, starting with `/`, such as `/src/main.rs`. Write paths in that form in `to` and `resource`. Only `locate` prints a path on disk.

sokf has these commands:

- `sokf graph [START]` shows how the knowledge links, as text, as JSON with `--json`, or as a Mermaid diagram with `--mermaid`. Run it with no `START` at the start of a session to see each store's layout. `START` can be a document id, a file, a key as `#KEY`, or a directory. Before you change a file, run `graph` on it to see the documents that describe it and the keys that its markers cite.

  `-l`, `--to`, and `--edge` filter with a JMESPath predicate, such as `-l "type=='Decision'"`, over the fields that `--json` shows. Read `sokf graph --json-schema` before you write one.
- `sokf locate <ID>` prints a document's path on disk, so read a document with `cat "$(sokf locate <ID>)"`.
- `sokf guide <ID|PATH>` prints the guidance that the document's schema gives for each of its parts at its states, and `--state <KEY>=<STATE>` prints it at a state that the document is moving to. Read it before you write or change a document.
- `sokf new <TYPE> --slug <SLUG> [--state <KEY>=<STATE>]... --write` creates a document from its schema, with the id, title, and states filled in, at the path that its states require. `sokf new --help` gives the options for an id without a template, a title, a sidecar, and a type that two stores declare. [New documents](#new-documents) says what to do next.
- `sokf validate [PATHS]` checks the knowledge, and `--fix` repairs what it can: generated blocks, indexes, links written in a longer form, and the directory of a document whose state changed. `--fix` changes no file whose schema breaks the grammar, so repair the schema first.
- `sokf init [DIR]` creates a store, in `/knowledge` by default, and makes it a knowledge root. Use it instead of writing a manifest by hand. `DIR` can already hold documents.
- `sokf harness install <HARNESS>...`, such as `claude`, `codex`, or `all`, writes this skill, the agent's instructions, and the permission to run sokf into the harness's files. `--scope` chooses where: the repository's files (the default), your home directory, or the repository's files that stay out of git. Run the command again after you upgrade sokf. It keeps a part that someone edited: `sokf harness diff` shows the edit, and `--force <PART>` overwrites it. In `/.sokf/config.toml`, `with = ["hooks"]` under `[harness.<HARNESS>]` turns on a hook that runs `sokf validate` at each prompt, and `without` lists the parts to skip. No command writes either.
- `sokf info grammar` prints the schema language. Read it before you write or change a schema. `sokf info formats [--schema <ID>]` prints the line formats and placeholder types that a schema can use.

`sokf validate` lists only the warnings at a stale or absent harness part and at files that an older sokf wrote, and counts the rest. Run `sokf validate --warnings` after you write a document, because a `sokf:optional` region left in it, a link to a deprecated document, and a custom rel are only warnings.

## Agent rules

Follow these rules when you write a document:

- Write each fact in one place. If a README or another file already states a fact, summarise the fact and cite that file in `sources` instead of copying the fact.
- Don't record an author or a date in the frontmatter, because git already records who changed each file and when.
- Write each `description` for a reader of the index, which shows it beside the title. Tell someone who is deciding whether to open the document what they need beyond the title, in terms they already know, usually in one sentence. Judge what that is for each document, rather than follow a pattern such as a list of its parts. Follow the schema's guidance for `description` too.
- You can choose the `id` of a new document, but never change an existing `id`, even when you rename or move the document, because other documents and markers cite it.
- Create each document, a sidecar too (`--for <ASSET>`), with `sokf new --write` in one step, because `new` allocates the id and writes the file where its states require. If you place the file yourself, get the id from `sokf new <TYPE> --slug <SLUG> --id-only`. Never guess a sequence number: `new` takes the next free one, and skips those that other branches and worktrees of this repository gave, but it can't see an unpushed branch of another clone, or a document that another `new` is writing at the same moment. So when `sokf validate` reports, after a merge, two documents of one type with the same sequence number, renumber the one from your own branch: take a free number from `sokf new <TYPE> --slug <SLUG> --id-only`, and rename its file, its `id`, and every link, `links` entry, and marker that cites it.
- Never add, edit, reorder, or delete a `verified` entry: only the human or process that it names writes it. A sidecar's entry covers its asset too. Leave a warning that an entry has lapsed for its author.
- To change a document's lifecycle state, set the state keys in its frontmatter and let `--fix` move the file. Never move the file by hand. `--fix` moves a sidecar together with its asset.
- Label each part that you propose `[agent]`, and write `[human]` only where the developer said the part or confirmed it in so many words. One label covers the whole unit. A label names no person. `sokf guide` shows which parts take a label, and the words of each label in the schema's language.

## New documents

Never copy a scaffold by hand: `sokf new` adapts it to the document's states.

After `sokf new` writes a document, replace every example text and reference of the scaffold with your own. `validate` doesn't report an example text that you leave, only a reference that resolves to nothing, such as `to: target-id`. Write the part that each `sokf:guidance` line guides, then delete the line.

Keep the content of a `sokf:optional` region and delete both markers, or delete the whole region. `validate` lists a region left in a document only with `--warnings`.

## Links

Follow these rules when you link one document to another:

- `rel` is the kind of link, and sokf derives its inverse, so declare each link once, from one side: relates-to↔relates-to, part-of→has-part, depends-on→depended-on-by, references→referenced-by, supersedes→superseded-by, implements→implemented-by, tests→tested-by, and contradicts↔contradicts. Use a custom kebab-case rel, which produces a warning, only where none of these fits. Record what a document derives from in `sources`, not with a `derived-from` rel.
- `to` is the target's id, with its store before it when the target is in another store, such as `sokf-spec:glossary`.
- In the body, link to a document or an asset as `[text][sokf:<ID>]`, which keeps working when the target moves, and to anything else by a path or a URL. A path that starts with `/` resolves from the repository root, and any other path from the file that holds it.
- For a sidecar, `sokf:<ID>` refers to its asset, so `![Flowers][sokf:flowers]` shows the picture, and `sokf:<ID>#` refers to the sidecar itself.
- To cite part of a document, add the slug of a heading or a key as the fragment: `[CLI-1_AC-2][sokf:req-cli#cli-1_ac-2]` or `[sokf:req-cli#acceptance-criteria]`. Cite a key that its schema makes unique in the store or the repository as `[sokf:#cli-1_ac-2]`, and part of the current document as `[text](#cli-1_ac-2)`.
- To refer to another store, put its name before the id, as in `[text][sokf:sokf-spec:glossary]`, `[sokf:sokf-spec:#user]`, or `<!-- sokf:include sokf-spec:graph-schema -->`, and write a store in another scope as `scope/name`.

## Markers

A marker is a comment line in a source file, outside any knowledge root, that cites one key or one document. For example, `// sokf-implements: CLI-1_AC-2` goes above the code that implements the criterion `CLI-1_AC-2`, and `# sokf-tests: CLI_P-5` goes above the test of the property `CLI_P-5`. `sokf validate` checks every marker, but `--fix` never rewrites one, so fix markers by hand.

Follow these rules when you write a marker:

- The tag `sokf-<REL>:` comes first on the line, after the comment leader, in lowercase. `REL` is a `links` rel, such as `implements`, `tests`, or `references`. One reference follows it, so two keys need two lines.
- A bare lowercase kebab-case word is a document id, such as `design-cache`. Any other bare word is a key's text, such as `CLI-1_AC-2`, and matches only a key whose schema declares `unique: repository`. Cite any other key as `<ID>#<KEY>`. The forms `store:id` and `store:#key` work as they do after `sokf:`. Put the store's name first for a store whose manifest declares `unique: knowledge`, and where two stores have a document with the same id, as in `six5536/login:#login-2_ac-1`.
- Write a marker in a plain comment, such as `//` or `#`, not in a doc comment, such as `///`, which puts it in the generated documentation.
- When `validate` reports that no `sokf-implements:` or `sokf-tests:` marker cites a key, add the marker above the code that implements the key or the test that tests it. Never put one on code that does neither.
- Keep markers accurate: when you change what code implements or tests, update its markers in the same edit. When a key is renamed or removed, `sokf validate` reports every marker that cites it.
- To write text that looks like a marker but is not one, such as a sample line in a test, put it between `sokf:ignore-begin` and `sokf:ignore-end` lines, or after a `sokf:ignore-next-line` line. In HTML or Markdown, write each directive in a comment of its own, such as `<!-- sokf:ignore-begin -->`. A fenced code block in a `.md` file outside the roots needs none.

## Generated blocks

A generated block is a part of a document whose content `--fix` writes: the definition block after `<!-- sokf:links -->` at the end of a document, the anchor `<a id="…"></a>` of each key, and each `index.md`. Write the `sokf:` links and the comment lines that start a block, never a block's content.

An include block runs from `<!-- sokf:include <ID | /PATH[#REGION]> -->` to `<!-- /sokf:include -->`, and `--fix` fills it. Include a document or an asset by its id, and a source file by its path. A region of a file lies between a `sokf:begin <NAME>` and a `sokf:end <NAME>` comment line, and `<ID>#<NAME>` includes the region `NAME` of an asset.

## Schemas

Read `sokf info grammar` before you write or change a schema: it gives every key and what it does. This section gives only the rules for writing a schema that the grammar doesn't.

Write a reference in a template as `[${}][sokf:${:ref}]`, never as a regex, because the `ref` type matches every target that a `sokf:` link can have.

Make each example concrete, because agents imitate what they are shown: write a line that a real document would contain about a real subject, never a slot (`<name>`), an instruction (`Describe the component.`), or filler (`One item.`).

Put what an example can't show in `guidance`. Write guidance in the voice of the document, because guidance can leak into the text that a writer copies:

- It describes the part in the third person and the present tense.
- It states each rule as a fact about the part, with its reason.
- It never addresses the writer.
- It never begins a sentence with a verb.
- It never includes a counter-example.

Give `guidance` for `description` only where the type needs something that the rule on descriptions in [Agent rules](#agent-rules) doesn't say. Never put the count in a `when-at-limit`, because `sokf new` and `sokf guide` write the count before it, as in "At most 3 items. Never leave out or merge content to stay within the count. When at the limit: Split the list." Write a `when-at-limit`'s own "Never … to stay within the count." only against another kind of squeeze, such as folding several systems into one node, because the words before it already forbid leaving out or merging content.

sokf writes its own words into a scaffold and into `sokf guide`, such as the words before each `when-at-limit`. To give those words in a language other than English, write them in `/.sokf/texts.yaml` or a file below `/.sokf/texts.d/`, as a map from the language tag to the entries `attribution`, `description`, `labels`, `limit`, `rule-unit`, and `units`. A schema's `language`, or else the manifest's `primary-language`, picks the language.
