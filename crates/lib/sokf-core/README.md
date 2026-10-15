# sokf-core

`sokf-core` is the core library of [sokf](https://crates.io/crates/sokf), the
validator for Superdev Open Knowledge Format (SOKF) knowledge stores.

> This crate is the library half of the `sokf` command-line tool. If you want
> to validate a knowledge store, install the [`sokf`](https://crates.io/crates/sokf)
> tool, not this crate. The maintainers publish the library API mainly so that
> the binary can depend on a released version. The API isn't stable.

- [What it does](#what-it-does)
- [Licence](#licence)

## What it does

The binary parses the arguments and renders the output. This crate holds the
logic that the binary calls, so that tests can run that logic without
starting a process. The crate exposes these entry points:

- `init` creates a knowledge store: the manifest, the two generated indexes,
  and the configuration entry that makes the directory a root. It reads only
  the configuration and the filesystem, and it checks every reason to refuse
  before it writes anything.
- `harness_install` and `harness_status` integrate agent harnesses, the
  programs that run AI coding agents, such as Claude Code, Codex, and
  Cursor, through agent-harness-adapter. sokf declares one integration, and each
  harness writes it as parts in its own files: whole files, blocks in files,
  and entries in JSON or TOML files. These functions work out the state of
  each part from the tree, the embedded content, and the record under
  `/.sokf/`. `harness_install` writes the files that bring the tree in line
  with the integration. `harness::hook` handles a harness's `stop` event: it
  runs `validate` with `fix`. If errors remain, the hook makes the agent keep
  working and reports the errors to it, in the harness's own form. It blocks
  on the same errors only once in a session.
- `validate` walks the repository, parses every document, pairs every asset
  with its sidecar, the document that describes the asset, and compiles every
  schema against the schema language. It works out the expected content of
  every generated block and of every directory's index. It repairs the files
  with `fix`. It runs the document check of the SOKF validation
  specification. It returns a `Report`, whose JSON form the command-line
  contract defines.
- `info_grammar` and `info_formats` return the schema language (the
  tramlines core grammar, the tramlines-md grammar, and then SOKF's schema
  extension) and the
  three-layer format registry.
- `new` builds a document of the given type from the template in that type's
  schema.
  It drops each part of the template whose `sokf:only` marker excludes the
  requested states. It fills in the id with the next
  sequence number, and fills in the title and the lifecycle keys. It rewrites
  each heading that is bound to a frontmatter key to the value of that key.
  It places the document in the directory that its states determine. It
  writes the document when the caller sets `write`. `new` builds a `Schema`
  document from the grammar's own template.
- `graph` builds a graph of the knowledge, with nodes and edges, and adds the
  inverse of every edge. It runs a query that starts at any node and follows
  edges to a given depth. JMESPath predicates filter the documents, the
  edges, and the edge targets that the query reaches. It
  returns the result as a tree and in the bounded JSON form that the tool
  specification defines.
- `locate` finds the filesystem path of a document by its id, and, when
  asked, the path of the asset that a sidecar describes.
- `markdown`, `grammar`, `include`, `check`, `fix`, `graph`, and `Expr` are
  the layers behind these entry points, exposed for tests and tooling.
- `Error` and `Result` are the crate's one error type and its result type.
  The command-line tool prints the `Display` output of an `Error` unchanged
  after `error: `, and exits with code 2 for every variant.

At build time, the crate embeds SOKF's schema extension from the
repository's `sokf-spec/` knowledge store, and the shared formats from
`knowledge/formats/`.
The tramlines core grammar and the tramlines-md grammar come with the
`tramlines-core` and `tramlines-md` crates.

For the full reference, see the [sokf-core API documentation on docs.rs](https://docs.rs/sokf-core).

## Licence

`sokf-core` is released under the Elastic License 2.0. See the
[LICENSE file](https://github.com/six5536/sokf/blob/main/LICENSE).
