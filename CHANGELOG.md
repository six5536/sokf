# Changelog

This file records the notable changes of each release of sokf. git-cliff writes the notes of each release from the commits of the release, and the notes of 0.1.0, the first public release, describe all that sokf does. While sokf is pre-1.0, a minor version might contain breaking changes.

## [0.1.0] - 2026-10-15

This release is the first public release of sokf. It publishes the crates `sokf-core` and `sokf` to crates.io, and the npm packages `@six5536/sokf` and `@six5536/sokf-<os>-<cpu>` to npm.

### Added

- A section that declares `for-each-key-of` can declare `examples`. The
  scaffold writes the occurrence of each key after the first from the next
  text of `examples`, so that each occurrence has its own text rather than a
  copy of the first. The architecture schema's example uses `examples` on its
  `for-each-key-of` section to show three subsystems.

- A schema can mark states of a lifecycle dimension as `archived`, or mark
  a type with no lifecycle with `archived: true`. `sokf graph` leaves
  archived documents out unless you pass `--archived`.

- `sokf graph '#key'` prints where the key is defined, with its line, and
  the documents that cite it. With a file as its start, `sokf graph` prints
  the documents that describe the file or a directory that holds it,
  nearest first.

- `sokf graph --all` prints every entry past the limit and every region of
  a folded include.

- The JSON of `graph` and of `validate` has a `summary` object.

- The `example` of a block or a unit rule can be a list of texts. The scaffold
  writes one unit for each text, in order, so that a new document shows
  several items rather than one, and the checker checks each text and names
  the one that fails.

- A section with `repeat: true` can list further occurrences of itself in
  `examples`. The scaffold writes each one, unchanged, after the occurrence
  that it derives from the rules.

- Markers: a comment line in a file outside the knowledge roots cites a
  key or a document of the knowledge, as `// sokf-implements: CLI-1_AC-2`
  does. `sokf validate` reports each marker whose reference resolves to
  nothing, each malformed marker, each tag in the wrong case, and each
  ignore directive out of order. A marker with a custom relationship
  raises a warning. A reference with no store resolves in every store, and
  a bare key resolves among the keys that are unique in the repository.
  `sokf:ignore-begin`, `sokf:ignore-end`, and `sokf:ignore-next-line` keep
  text that looks like a marker from being read as one. You can set
  `[markers] check` in `/.sokf/config.toml` to `error`, `warn`, or `off`. A run reads a file
  outside the roots only when its stat changes, and checks a file again
  from its stored markers when what it cites changes. `tests`, with the
  inverse `tested-by`, is a core relationship.

- Markers as citations: a `cited-by` entry can set `from-marker: <rel>`
  in place of `from-type`. In that case, only a marker of that
  relationship counts as a citation. A schema can therefore demand an
  `implements` marker for each criterion and a `tests` marker for each property. A
  schema's `markers` lists the relationships of the markers that can cite
  its documents, and a marker of any other relationship fails at its own
  line. `markers: []` refuses every marker. With
  `[markers] check = "off"`, no `from-marker` entry is checked, and its
  schema raises one warning.

- An id template can hold a lifecycle key's state: `${kind}`, a hole with
  no type, named for a lifecycle key, as in `issue-${:seq}-${kind}-${:slug}`.
  `sokf new` fills it from `--state`, or with the key's first state, and
  the validator reports a document whose id holds a state other than its
  own.

- A schema can declare `cited-by`, the inverse of `links`. Each entry
  demands that every key that a named rule captures, or every document of
  the type, is cited by a body link from at least one document of a type
  in `from-type`. `only` gates the entry on the cited document's states,
  and the entry carries its own `severity`. A citation is a resolved `sokf:`
  link in another document, of any store, outside included and generated
  text. The validator reports the finding on the uncited key at its line,
  or on the uncited document. The check runs after every document is
  checked, from the links that the link check already resolves. The
  validator refuses a schema whose `keys` entry or `for-each-key-of` refers
  to a rule that captures no key.

- A contract's template is derived. The tool derives one template for each
  combination of lifecycle states from the rules themselves, and marks each
  gated part. It checks each derived template against the contract, so a
  generator that writes what the contract refuses fails the schema rather
  than the document. The template takes all of its text from the `text` and
  `example` of the rules. It reports by name a part that no rule supplies,
  derives no text from a pattern, and writes nothing back into the schema.

- A section, a block, a unit rule, and a frontmatter constraint can carry
  `example`: text of the kind that the rule demands. The example is
  checked against the rule that carries it, shown by every failure of the
  rule, and written by the derived template. A unit rule that binds a
  format and states no example takes the format's example.

- A contract, a section, a preamble, a block, a frontmatter constraint, a
  lifecycle dimension, and a link can carry `guidance`: what an example
  cannot say. The derived template writes it as one `sokf:guidance` line
  before the part it guides, and the template opens with a guidance line
  that starts with a fixed sentence:
  `Each part below is an example: replace it with your own.` A governed
  document that keeps a guidance line has an error at that line, and
  `--fix` neither writes nor removes one. No failure prints guidance.

- A derived template writes an optional section or block where the
  derivation can write it. It wraps each run of them in a `sokf:optional`
  region, opened and closed as an include block is, so a label and the list
  under it read as one decision. `sokf new` keeps the
  region, because what it holds is the reader's decision. Where a rule
  inside an optional part offers nothing to write, the part is left out
  and a warning identifies the rule that kept it out.

- A finding that a contract rule raises identifies the rule after the
  schema's id. It gives the rule's `rule-id` when the rule declares one,
  and otherwise its section and block position, frontmatter key, or `rel`.
  As a result, a reader can find the rule in the schema that the authority
  points to.

- A block can take `rules`, guarded rules over its units, in place of
  `each` or `sequence`. Each entry governs the units that its
  `select-pattern` matches, `min` and `max` bound how many units it takes,
  and a unit that no entry takes is an error unless `units-open` is set.
  The unit rule of a `code` or `html` block can take `lines`, the same
  guarded rules over the block's content lines. These rules check a
  diagram's arrows, node names, and element count, and each failure reports
  the line it read. A key captured from a line belongs to the enclosing
  block's `rule-id`.

- A `formats` entry can declare `patch` and state only how it differs from
  the entry it shadows. JSON Merge Patch (RFC 7386) merges the two.

- Two consecutive prose rules divide one run of paragraphs between them, so
  an optional block between them no longer changes the body's shape.

- A unit rule that reads a fence line by line can state the text of that
  fence, as its `text` or its `example`. A derived template copies it, and
  the line rules read their keys out of the copy, so a section demanded per
  captured key appears once per box the diagram draws. When the template was
  built from the line rules alone, a diagram of five classes produced
  five copies of one box: one line per rule, each repeated to meet its
  `min`.

- `harness.<name>.command` in `/.sokf/config.toml` sets the program that a
  profile's hook lines run in place of `sokf`. A repository that builds
  the tool points the key at its own build, so that the hook it installs
  runs that build.

- `sokf new <TYPE> [--slug | --id] [--title] [--state <K>=<V>]... [--dir |
--for <PATH>] [--write | --json | --id-only]` creates a document of a type
  from its schema's template. The first template that covers the requested
  lifecycle states is the seed. The parts that its `sokf:only` markers
  exclude are dropped, and the markers are stripped. `id` takes the next
  sequence number of the id template, `title` derives from the slug, the
  lifecycle keys are set, `verified` and `generated` are dropped, and every
  heading, paragraph, and item bound to a frontmatter key is rewritten. The
  document goes into the directory that its states determine, or into
  `--dir`. `--for <PATH>` creates a sidecar beside its asset, and
  `sokf new Schema --id <id>` seeds a schema from the grammar's own
  template. `--write` places the file and reports `created <path>`;
  `--id-only` prints the id alone.

- `sokf init [DIR] [--name] [--description] [--json]` creates a knowledge
  store, the manifest and the two generated indexes, and makes the directory
  a knowledge root. It writes `/.sokf/config.toml` when none exists, and
  otherwise appends the root in place and keeps the file's comments. It
  refuses a directory that is not empty, and writes nothing when it refuses.
- `sokf harness install <NAME> [--without <PART>]... [--force] [--json]` and
  `sokf harness status <NAME> [--json]` write the integration of an agent
  harness into the repository from the profile that the binary embeds, and
  report its state. The `claude` profile for Claude Code has four parts: the `sokf` skill, an
  instructions block in `AGENTS.md` or `CLAUDE.md`, a `Stop` hook that runs
  `sokf validate --fix` when the agent ends its turn and hands back the
  errors that remain, and the permission to run `sokf`. `install` makes the
  tree match the profile and writes nothing that matches already. A part
  that you edited is left as it is unless you pass `--force`. A part
  declined with `--without` is recorded in `/.sokf/config.toml` and stays
  declined. `/.sokf/harness.toml` records what was written, so `status` tells a stale part from an edited
  one. The agent reference ships as the `sokf` skill, from
  `harness/skills/sokf/SKILL.md`.
- `sokf validate [PATHS] [--fix] [--json [--query]] [--warnings]` checks
  every document in every knowledge root, and every additional file,
  against version 0.6 of the Superdev Open Knowledge Format (SOKF). It
  checks manifests, frontmatter, ids and templates, links and fragments,
  generated include and definition blocks, schemas and their examples,
  lifecycle directories, and the grammar's contract rules. It checked a
  schema's `counter-examples` as well, until their removal, which the
  following Changed section records. `--fix` regenerates
  stale blocks, moves lifecycle documents, and rewrites renamed, bound, and
  setext headings. Findings are ordered by path and line and end with their
  authority, and a run with findings exits with `1`. `--json` prints the
  report as one object, and `--query <EXPR>` prints a JMESPath projection
  of it.
- `sokf graph [START...] [--depth] [-l] [--edge] [--to] [--format | --json
[--query] | --mermaid]` prints the knowledge as a graph from any point.
  The layout of each root prints as `tree` does, with every document's
  `links`, `resource`, `sources`, and include edges under it. The tool
  synthesises the inverse of each edge. As `cargo tree` does, it expands
  each node once in the whole output and marks it `(*)` everywhere else, so
  the output stays bounded by the number of nodes and edges at any depth.
  JMESPath predicates select rows, edges, and targets. The JSON form is the
  contract, and a Mermaid flowchart is drawn from the same query. A
  `--query` result keeps the key order of the object it selects, and the
  written order of a multi-select hash. The evaluator passes the JMESPath
  compliance suite.
- `sokf locate [--asset | --json] <id>...` prints the absolute filesystem
  path of each document. `--asset` prints the asset that a sidecar describes,
  and `--json` prints both.
- Assets and sidecars (spec §2, §10): an asset is a non-`.md` file that a
  schema's `asset-files` glob matches. The asset is
  described by its sidecar, the document beside it whose name is the
  asset's filename plus `.md`.
  `validate` reports an asset with no sidecar, a sidecar of another type, a
  document of an asset type that is not a sidecar, and a sidecar whose
  `resource` is not the asset's path. `--fix` rewrites that `resource` and
  moves a sidecar together with its asset. `sokf:<id>` refers to the
  content, so an embed or an include naming a sidecar's id gives the asset
  and survives a move. `sokf:<id>#` refers to the sidecar. `graph` prints an
  asset as an `asset` node with its `sidecar`.
- The generated index (spec §9): every directory of a knowledge carries an
  `index.md` that the tool writes whole, with one id-addressed entry per
  document and one per subdirectory, and a definition block for their
  links.
  `validate` reports a directory with no index and an index that differs
  from the generated content. `--fix` writes them after the moves, reported as `created` and
  `rewrote`. A schema glob that matches an index is an error on the schema.
- `sokf info grammar` and `sokf info formats [--layer] [--schema]` print the
  embedded grammar and the registry of formats in its three layers.
  `--schema` refuses a schema that more than one root declares.
- `sokf next-id <type> [--slug]` prints the next sequence number for a type
  whose schema declares an id template. It reads the number through the
  cache.
- `--repo-root <DIR>` and `--no-cache` are global options.
  `/.sokf/config.toml` lists the knowledge roots and the repository
  excludes, and `/.sokf/formats.yaml` holds the repository layer of line
  formats. A cache under `/.sokf/cache/` keeps the parse of each file, and
  every command that reads the knowledge shares it. A path argument is
  read in the spec form (`/`-rooted from the repository root)
  or as a filesystem path.
- `sokf-core` holds the walk, parse, grammar, check, include, fix, report,
  cache, graph, and query layers behind the commands, with fixtures,
  snapshots, and property tests.

- `sokf new` takes a type qualified by its store, such as `sokf-spec:Grammar`,
  when more than one store declares the type. Without a qualifier, the store of
  `--dir` or `--for` chooses the schema, and a refusal lists each qualified
  form of the type.

- A schema without a path-enforced dimension can declare a `root`, the
  directory that holds its documents. `sokf new` writes a new document there
  and refuses a `--dir` outside it, the validator reports a document that lies
  outside it, and `--fix` moves such a document into it. The schemas of
  sokf's own knowledge declare their roots.

- A schema can declare `plans-paths`, and while a document is in the states
  of an entry, each path that a link in the named section gives is planned: a
  link to it, a `resource` that lists it, and an include of it pass before the
  file exists, and the fix keeps an include's sketch until the file exists. The
  Issue schema plans the files of an open issue's Files section.

- A schema can declare `current-links`, patterns of the paragraphs that make a
  promise, and such a paragraph links no archived document. The Design and
  Contract schemas use it for `Planned:` paragraphs.

- Each command that takes `--json` takes `--json-schema`, which prints the
  JSON Schema of that command's `--json` output and reads no repository. The
  flag refuses any other argument.

- `sokf graph` shows the markers in code that cite a document. Under a
  document, the markers of one file and one relationship print as one edge
  with their count, such as `implemented-by → /src/load.rs (2 markers)`.
  `--all`, a file, or a key as the start prints each marker with the key that
  it cites and its line. A marker edge has the kind `marker`, and its JSON
  carries `line`, or `markers` when it is folded.

- `sokf completions <shell>` prints shell completions for bash, zsh, fish,
  PowerShell, and elvish, and a hidden `sokf man` renders a roff man page.
  sokf generates both from its command definitions, written with clap, a
  Rust library for parsing command-line arguments. Both ship in the release
  archives.
- `--version` and `-V` print `sokf X.Y.Z`, and every command takes `--help`.
  The tool exits with `0` on success and `2` on error, and exits cleanly when
  the program reading its output closes the pipe.
- `sokf-core`, the library half, holds the logic that the binary drives,
  and the `Error` and `Result` pair whose messages the CLI prints verbatim.
- The release pipeline builds binaries for Linux and macOS (`x64` and `arm64`)
  and Windows (`x64`), and publishes them behind the `sokf` launcher
  package. It also creates a GitHub release that carries the
  archives, `SHA256SUMS`, the man page, and the completions. Linux binaries
  are statically linked against musl.
- The continuous integration (CI) gate, which `ci.yml` and `release.yml`
  share so that the two cannot
  drift apart, runs fmt, clippy with `-D warnings`, tests on macOS and
  Windows, doctests, docs with warnings as errors, the npm launcher test, version consistency across
  the tree, a per-crate line-coverage gate of 90%, and `cargo-deny` for
  licences, bans, and sources. Security advisories run on a schedule instead, so
  a new RUSTSEC entry opens an issue rather than failing an unrelated pull
  request.

### Changed

- `sokf harness` supports every agent harness of agent-harness-adapter, not only
  Claude Code: `claude`, `codex` (OpenAI Codex), `factory` (Factory Droid),
  `gemini` (Gemini CLI), `copilot` (GitHub Copilot), `cursor`, `pi`,
  `opencode`, `kilo`, `qwen`, `devin`, and `agents` (any agent that reads `AGENTS.md`). Breaking changes:
  - `sokf harness install` and `sokf harness status` take one or more
    harnesses, or `all`, and `--scope project|user|local`. `user` writes
    under the home directory and keeps its record and declined parts in
    `~/.sokf/`; `local` writes the files that stay out of git (Claude Code's
    `.claude/settings.local.json`) and keeps its record in
    `/.sokf/harness.local.toml` and its declined parts in
    `/.sokf/config.local.toml`. `--without` applies to each harness named.
  - The text report groups the parts under a `<harness>:` line, indents them,
    and adds `unsupported:`, `note:`, and `warning:` lines. A location that
    several harnesses read, such as `/AGENTS.md` or `/.agents/skills`, is
    written once and reports `shared` for the others, as `(instructions, by
    claude)`.
  - The JSON report is `{scope, harnesses: [{harness, parts, unsupported,
    notes}], warnings}`; a part can have the state `shared` and the key `by`.
    `harness-report.schema.json` is regenerated.
  - The parts are in the order `instructions`, `skills`, `hooks`,
    `permissions`, and the skill part of Claude Code is `/.claude/skills`, not
    `/.claude/skills/sokf`. The skill's frontmatter quotes its description,
    so an installed skill reads `edited` once; run
    `sokf harness install claude --force` to update it.
  - The instructions block says what to do in any repository with
    `/.sokf/config.toml`, and to run `sokf validate --fix` where no hook
    runs it, so that one block serves every harness and the user scope.
  - `sokf harness hook <HARNESS> <EVENT>` answers each harness in its own
    form. The hook line that earlier versions installed,
    `sokf harness hook claude stop`, keeps working. An unknown harness is
    `no harness named`, an unknown event is `sokf has no hook for the event`,
    and `agents`, which runs no hooks, exits with code 2.
  - The stop hook's guard is one file for each harness and session,
    `/.sokf/cache/harness-stop-<harness>-<session>.hash`, in place of
    `/.sokf/cache/harness-stop.hash`, and the hook removes it when the
    knowledge is clean, so the same errors, when they come back, block once
    more. With `--no-cache`, the hook lets the agent stop when the harness
    says it already continues because of a stop hook.
  - Outside a repository, or in a repository with neither
    `/.sokf/config.toml` nor `/knowledge`, the stop hook lets the agent stop
    and prints no error, so a hook installed for the user is quiet elsewhere.
  - `harness.<name>.command` sets the hook program of any harness.
  - A settings file that holds sokf's hook twice reads `edited`, not
    `current`.

- `sokf harness install` and `sokf harness status` run on the crate
  agent-harness-adapter-core, which now holds the part kinds, the states, the record,
  the declined parts, the hook answer, the stop hook's loop guard, and the
  CLI's exit codes. The move changes these behaviours:
  - The `hooks` part owns the hook entries whose command contains
    ` harness hook `, not the whole group that holds them. A hook of your own
    in the same group stays, and `--force` no longer removes it.
  - A declined part is never read, so a `.claude/settings.json` that doesn't
    parse no longer stops `install` or `status` when both of its parts are
    declined.
  - The record hashes a `region` part by its words joined by single spaces,
    and a `merge` part by the JSON array of its entries. A part that an
    earlier sokf recorded, and that is stale, reads `edited` once; run
    `sokf harness install --force` to update it.
  - In the JSON of `harness install` and `harness status`, `state` is the
    state in which the command found each part, and a new key, `action`,
    says what `install` did to a part that it wrote: `created`, `rewrote`,
    or `updated`. The text report is unchanged.
  - A region whose closing marker was deleted is your text: the block goes
    after it instead of replacing what follows it.

- The launcher is `@six5536/sokf`, and the platform packages are
  `@six5536/sokf-<os>-<cpu>`, on npm.
- sokf builds against the tramlines engine that gives each optional section
  of a scaffold a region of its own, lets a group that opens with a paragraph
  take the end of the prose before it, and requires a message on each
  prohibited pattern. Each prohibited pattern in sokf's schemas and formats
  has its message, and a finding gives the message in place of the pattern.
  The parts diagram of a design bans the classes that it can't use, and a
  caption bans the quotation mark and the angle bracket, each with a message.

- sokf checks each document with the `tramlines-core` and `tramlines-md` crates
  of the [tramlines](https://github.com/six5536/tramlines) repository, in place
  of its own copy of the engine. The engine's specifications and grammars live
  there too, and the `tramlines-md` crate and command no longer ship with sokf.

- Each finding of the engine ends with the key of the criterion that it breaks,
  such as `(TLMD_152)`, after the schema and the rule. One fault gives one
  finding, on the line of the fault, and many messages are reworded. The
  engine also makes checks that it didn't make before, so a schema or a
  document that passed can now fail.

- The crates, the npm packages, and the README declare sokf's licence as the
  Elastic License 2.0, which `LICENSE` grants. They declared MIT before.

- `sokf info grammar` prints the tramlines core grammar, then the tramlines-md
  grammar, and then SOKF's extension, separated by `---`.

- The cache's store format changed, so the first run after an upgrade rebuilds
  the cache.

- The shared format `requirement-summary` (formats 0.4) takes a full sentence
  whose subject is the part, such as `` `sokf validate` checks… `` or `The settings
  screen lets…`, and refuses a summary that starts with a verb. The type
  `third-person-verb` is removed, and the types `predicate` and `code-name` are new.

- `sokf graph` with no start prints the layout of each store: its
  directories, and its documents by id, with no edges. With `--depth N`, it
  prints the edges of each document in the layout. A group of more than 20
  entries prints 20 of them and a line that counts the rest. The includes
  of a file fold to one line, a typed link hides the mention that mirrors
  it, and an asset prints once, under its sidecar's id. The text ends,
  after a blank line, with
  `<shown> / <total> docs (<shown> / <total> archived)`.

- The output of `sokf validate` ends, after a blank line, with
  `<n> errors, <listed> / <all> warnings`, or with `0 warnings` when there
  are no warnings.

- The cache keeps the scalar frontmatter values of each document and the
  `archived` setting of each schema. The tool rebuilds the store once to add
  them.

- When its start is a document, a file, a key, or a directory outside the
  knowledge, `sokf graph` reads only the edges near that start from the
  cache. It reads the edges of the start, then the edges of the nodes they
  reach, one hop at a time, until it reaches its depth. The cache finds the
  edges that point to a node through a new index of the edges by their
  target. The tool rebuilds the store once to add the index.

- `graph`, `locate`, `new`, and `info formats --schema` first check what
  changed since the last run, as `validate` does. They keep the result in
  the cache and report none of its findings. As a result, a lookup answers
  from the repository as it is. The next command, whether a lookup or
  `validate`, does not repeat the work. At a million files, a lookup after
  one change costs as much as a `validate`, and the `validate` after it
  reads only the manifest. With `--no-cache`, a lookup
  reads every document and runs no check, as before.

- The walk outside the knowledge roots runs on parallel threads, the
  table of files is stored in path order, and a run with `--changed`
  compares only the changed paths with the store. At a million files, on a
  release build, a run with `--changed` takes a third of a second, and a
  full run with one change takes a little over a second.

- The cache is one SQLite store, `/.sokf/cache/sokf.sqlite`, that holds
  metadata:

  - Each file's length, modification time, and hash.
  - One row per document, with its id, type, title, and description.
  - The relationships between documents: the ids, keys, and citations.
  - Each check's findings and reads.

- A run stats every file
  and reads only the files whose stat differs from the stored one or lies
  within two seconds of the last run. It checks only the changed documents
  and the documents that read what changed. It reads those documents, and
  the documents they read, from their files and holds them in memory. It
  reports the rest from the store. A change to a schema checks the
  documents of its type and the documents that read the schema. A change
  to the formats checks the documents whose schemas use a format or a type
  that changed. A change to a manifest checks the documents that resolve a
  store by name. A change to the configuration rebuilds the store.

- The tool computes and reads the index of a directory only when a change
  touched the directory, through a document in it, a subdirectory, its
  manifest, an id that moved to another document, or a pairing. Otherwise
  the tool keeps the stored finding. An
  additional file is read and checked only when it changed, a fact it
  read changed, or the glob that governs it changed. `locate`, `new`, and
  `info formats --schema` answer from the store and read only the changed
  documents. `graph` builds from the edges that the last check stored, and
  reads a document only for the fields of a node that it prints or tests.
  A command that does not check writes nothing to the store. The sync
  compares the walk with the store in one pass in path order and holds
  only the rows that changed.

- `validate --changed <PATH>`, given once per
  path, passes the paths that the caller knows have changed, for example
  from `git diff --name-only`. The run reads the metadata of only those
  paths and the paths under them, lists each of them as a walk of the
  repository lists it, and takes every other file as the cache recorded
  it. The next run without the
  option finds a change that the list omits. A run exits with code 2 when
  `--changed` is combined with `--no-cache` or given a path outside the
  repository.

- A run reads and compiles a schema only when a check needs it, for a
  document of its type, an additional file it governs, or the schema
  itself, or when `locate` or `graph` resolves a key of a changed document.
  A run that checks nothing reads no schema.

- When a source file or an asset
  changes, appears, or disappears, the run checks the documents that
  include or link it again. It finds the change by the file's stat, and
  doesn't read the file.

- A warm run reuses the cache, and a cold run starts with no cache. A warm
  run reports a finding on a schema at the same line as a cold run, even
  when the warm run did not read the schema. A document that is
  checked again only because its citations changed keeps its keys, so its
  coverage findings are those of a cold run. The cache keeps those
  keys, so a warm `locate` of one of them finds it.

- When `/.sokf/config.toml` is absent, `sokf` refuses a missing
  `/knowledge` as it refuses an empty one, with exit code 2 and the same
  message, and no longer raises an input/output (I/O) error.

- `--no-cache` keeps the same store in memory.

- The per-file JSON entries and their schema, `cache-entry.schema.json`,
  are removed.

- `--fix` works out its repairs from the documents that have findings.

- The cache counts the files it reads and the statements it runs, so that
  a test can bound the work of a run.

- Each rule now states what it expects only through its example. A schema
  said one thing up to three times, in `description`, `message`, and a
  hand-written template, and an
  agent filling the template copied its prompt-shaped text
  verbatim. `templates`, `description` on a rule, `message` everywhere,
  and a format's `doc` are removed. The tool builds each failure
  mechanically from the rule. A shape failure quotes the text, the shape,
  and the rule's example. A ban quotes the span it matched and the pattern.
  A guarded rule quotes its selector and the count. A rule that declares a regex
  (`pattern`, `heading-pattern`, `source-pattern`, or `key`) carries an `example` of
  it, unless it binds a format that carries one or states its `text`.
  Guidance that repeats the example beside it is reported.

- `counter-examples` are removed, for three reasons. First, a
  counter-example reached neither the derived template nor a failure, so it
  tested the schema, not the writer. Second, it cost tokens wherever an
  agent read the schema. Third, it showed that agent a wrong document in
  full. A rule that refuses good text already fails on its
  example. What a writer must not do, where no rule says it, belongs in
  `guidance`.

- A ban now has one key, `prohibited-patterns`, a list of regexes, on the
  contract, on a unit rule, and on a format. It replaces `forbid` and the
  singular `prohibited-pattern`.

- A derived template writes a link rule whose `min` is zero as well, with
  the placeholder target a demanded link takes, so the template shows an
  optional edge instead of hiding it. A template's guidance lines do not count
  against `line-limit`.

- Two `sokf:optional` regions that meet in a derived template are written
  as one. An optional block at the end of a section and an optional section
  after it were marked as two regions back to back, which is one decision
  marked twice.

- A guarded rule states its own `min`. The key defaulted to zero with a
  `select-pattern` and to one without, so its value depended on another
  key. A rule whose guard was meant to demand a unit governed the units it
  selected, but silently demanded none of them. A rule under `rules` or
  `lines` that declares a guard and no `min` is now reported. To govern
  units without demanding any, write `min: 0`. Otherwise, write the number
  demanded. Every other `min` states
  its default in the grammar. No schema in either store uses a guard, so
  no schema changes.

- Breaking for schemas: a `text` value is a string or
  `{ from: <frontmatter key> }`, the frontmatter value standing in for the
  string. `heading` takes one, so `heading-from: k` is written
  `heading: { from: k }`, and the validator suggests the rewrite. A unit rule
  gains `text`, the unit's whole text. `--fix` and `sokf new` rewrite a
  bound paragraph or item as they rewrite a bound heading.
- The tool walks a knowledge root inside another root once, and the inner
  root owns the files under it.
- Every path the tool prints, in text and in JSON, is in the spec form:
  `/`-rooted from the repository root, as paths in documents are. `locate`
  alone prints filesystem paths.
- The embedded `ears` format carries a top-level `example`, as the grammar
  requires of every format.
- The grammar's `list` doc includes `const` among the constraints that apply
  to every element, which is how the validator reads it.

### Fixed

- With `--repo-root`, the repository root is the directory's canonical path,
  so a path argument resolved from the current directory lies under the root
  when `--repo-root` reaches the directory through a symbolic link, as the
  temporary directory of macOS does. Before, the tool refused the path as a
  usage error.
- A build on Windows embeds the grammar, the formats, and the harness files
  with LF line endings, as every other build does. Before, a Windows checkout
  gave them CRLF, so `sokf info grammar` printed CRLF and `sokf harness
  install` wrote CRLF files.
- `sokf validate --changed` no longer fails, panics, or reports wrong findings
  when a check reads a removed file that the list of changed paths leaves
  out. The run checks the repository again with a walk. With `--fix`, the run
  no longer writes into a removed directory that the cache still lists.

- `sokf validate --changed` lists each named path as a run without the
  option lists it. An exclude no longer hides a named file inside a knowledge
  root, and a named file that the walk prunes, such as a gitignored file, a
  file of the cache, or a symbolic link, stays out of the run. A named
  directory stands for every path under it, whether it was added or removed.

- The check after a fix in `sokf validate --fix` compares the files with the
  first check of the same run, so it reads only what the fix wrote, and the
  counts of each check cover that check alone.

- A removed file that a schema's `additional-files` glob matched leaves no
  output in the cache's store.

- When `sokf validate --fix` moves a document into a directory that the move
  creates, the same run rewrites the index of each directory above it that
  gains the new directory, so a second run has nothing to fix.

- An ignore directive in an HTML comment, such as `<!-- sokf:ignore-begin -->`,
  opens an ignore region. The end of a comment can follow a directive, as it
  can follow a marker's reference.

- A run that shares the cache with another run writes nothing when the other
  run commits first. Before, a run that committed last could pair a file's
  current record with rows that the other run derived from the file's old
  text, and the cache then reported stale errors, such as a missing glossary
  key, until the cache was removed.

- A section whose heading is a bare template, such as `${}`, no longer
  takes a heading of another level, such as the title, when a section of
  the heading's own level matches it.

- With the cache on, an include of a sidecar's id again resolves to its
  asset when the asset comes back beside the sidecar, in `graph` and in
  the findings of `validate`. A file that leaves an `additional-files` glob
  loses its edges in `graph`.

- `sokf graph` treats a `START` that starts with `.`, or ends with `/` or
  `.md`, only as a path, as its contract says. Before the fix, a document
  whose id took such a shape made the start ambiguous.

- A region of an included file whose markers break their pairing is
  reported at that file and the line of the marker, once, and no longer
  at the include block of each document that includes it.

- A body link that kept its target and changed its fragment, as from
  `#cli-1_ac-1` to `#cli-1_ac-2`, left the `cited-by` findings of the
  target as they were until the target itself changed.

- `sokf init` with a configuration file that has no `knowledge.roots`
  wrote the key with the new store alone, so the default root
  `/knowledge` stopped being checked. It now lists every root in effect
  whose directory exists, then the new store, as it does when there is no
  configuration file.
- The scaffold of a schema wrote a list item without the nested blocks
  that its rule demands, so the scaffold failed its own schema. The
  nested blocks are now written under the item.
- `sokf graph` printed the same `included-by` line once for each region
  that a document includes: an `included-by` line now prints its region
  after the `rel`, as `included-by #schema → contract-schema-language`.
- A run through the cache kept the coverage warning of a document that a
  new document had come to cite. The warning stayed until the cited
  document itself changed. The second round of the run now counts the
  citations that the first round found.

- A gated part of a template that carries no `sokf:only` marker is
  reported on the schema. The check walked the markers written and could
  not see the marker left out. `sokf new` would then copy the part into a
  state whose rules refuse it, and the failure would land on the document.
  The check now compares markers with gates, and gates with markers.
- A declared block position takes a block only where the
  block satisfies it. A position claimed a block on its kind alone and
  never gave it back. Claiming blocks in that way caused three faults:

  - An optional, repeating, or grouped position took the block that the
    next position was written for.
  - A section whose order `blocks-ordered: false` frees was decided by the
    document's order.
  - A position whose bounds admitted a whole run of paragraphs or list
    items took the whole run.

  Six ordinary shapes could
  not be written, among them one or more `yaml` code blocks before a
  `text` one, and an optional lead paragraph before a required one. A
  position that the schema demands once still takes its block, whatever
  it reports, so a malformed block is reported at its own position. When
  a position gives a block back and no later position can take a block of
  that kind, the position reports why it refused the block. Dividing a run
  now covers `bullet-list` and `numbered-list` as well as `prose`.
- A block of a kind no rule declares is reported once. One stray paragraph
  before a declared diagram drew three errors: the paragraph was not the
  block the rule expected, the paragraph was undeclared, and the diagram
  after it was called undeclared as well, although the contract declares it.
  The rule now looks for the block it declares. It reports the kind of
  block it expected and the kind it found only when no such block follows.
- A key that a `format` declares is now as strict as a key that a rule
  declares. A unit matching the format's shape while carrying no key was
  accepted and captured nothing, so a mistyped identifier left the
  traceability chain unreported. `key-optional` waives the requirement
  for the key.
- A `for-each-key-of` rule demands a section for each key. Such a section
  was paired with the key that the rule refers to. The uniqueness check
  read the pair as a clash, so the rule was unusable.
  A key that must be unique across documents was compared with itself in
  the same way, and that check now compares only different documents.
- A prohibited pattern that applies to the whole document matched text
  inside backticks, so a pattern quoted as code, such as a glob, was
  reported as prose.
- A `region` block is compared by the words it holds rather than by its
  layout, so a Markdown formatter that rewraps or spaces the block leaves
  it current. The tool owns what the block says. The file that holds the
  block, and the layout that a formatter gives it, belong to the file's
  owner. The block is also
  written the way a formatter leaves it, so a formatter changes nothing in
  the file.
- The tool identified its own hook group only by a command that began with
  `sokf`. It therefore read a hook line that pointed at another build as
  absent, and `install` added a second group beside it. The tool now
  identifies its group by the `harness hook` command that the group runs,
  whichever program runs it. Such a line reports as `edited` and is left
  alone.

### Removed

- The shared format `ears` and its types `ears-precondition`, `ears-trigger`,
  and `the` are removed. `requirement-ears` is the one EARS format: a schema
  that bound `ears` binds `requirement-ears`, and each criterion takes a comma
  after its trigger, no THEN in an event, and a full stop at the end.
