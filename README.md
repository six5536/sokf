# sokf

[![Publish](https://github.com/six5536/sokf/actions/workflows/publish.yml/badge.svg)](https://github.com/six5536/sokf/actions/workflows/publish.yml)
[![crates.io](https://img.shields.io/crates/v/sokf.svg)](https://crates.io/crates/sokf)
[![npm](https://img.shields.io/npm/v/@six5536/sokf.svg)](https://www.npmjs.com/package/@six5536/sokf)
[![docs.rs](https://img.shields.io/docsrs/sokf-core)](https://docs.rs/sokf-core)
[![License: Elastic-2.0](https://img.shields.io/badge/license-Elastic--2.0-blue.svg)](LICENSE)

sokf validates knowledge stores in the Superdev Open Knowledge Format (SOKF):
directories of Markdown documents with YAML frontmatter, kept in a Git
repository and maintained largely by AI agents. It checks every document
against its type's schema, resolves every link, keeps generated content
current, allocates document ids, prints the store as a graph from any point,
and locates a document by id.

- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
- [Agent harness](#agent-harness)
- [Output and exit codes](#output-and-exit-codes)
- [Specifications](#specifications)
- [Project layout](#project-layout)
- [Development](#development)
- [Licence](#licence)

## Install

Install the launcher, which brings the prebuilt binary for your machine, or
build sokf from crates.io:

```sh
npm install -g @six5536/sokf   # prebuilt binary, Linux/macOS/Windows
cargo install sokf             # from source, needs a Rust toolchain
```

Prebuilt binaries cover Linux and macOS on `x64` and `arm64`, and Windows on
`x64`. The Linux builds are statically linked against musl, so they need no
particular glibc version and run on Alpine too. The crates aren't on crates.io
while the repository is private, so the source install builds from git.

## Usage

A store is a directory with a manifest, and `sokf init` creates one.

```sh
sokf init                           # a store at /knowledge, registered as a root
sokf init docs/kb --name project-kb # a store elsewhere, added to /.sokf/config.toml
sokf harness install claude         # the sokf skill, instructions, stop hook, and permission for Claude Code
sokf harness install codex cursor   # the parts that OpenAI Codex and Cursor support; `all` names every harness
sokf harness status all             # each part of each harness: current, shared, stale, edited, absent
sokf validate                       # check every knowledge store and additional file
sokf validate --fix                 # regenerate blocks and indexes, move lifecycle documents, and rewrite headings
sokf validate knowledge/core --json # report on a directory, as one JSON object
sokf validate --json --query "errors[].path"   # a JMESPath projection of the report
sokf validate --warnings --no-cache # list the warnings, and skip the cache
sokf validate --changed knowledge/core/planner.md   # read the metadata of only the paths named and the paths under them; the rest come from the cache

sokf graph                          # the layout of every knowledge root: its directories and documents
sokf graph planner                  # one document and its edges; --depth 2 for two hops out
sokf graph src/planner.rs           # the documents that describe or include a file
sokf graph '#cli-1_ac-2'            # where a key is defined, and who cites it
sokf graph --all --archived         # every entry past the limit of 20, and the archived documents
sokf graph -l "type=='Issue'" --format '{id}  {state}  {title}'   # the rows a predicate selects
sokf graph --json --query "nodes[?kind=='document'].id"           # the JSON form, projected
sokf graph planner --mermaid        # a Mermaid flowchart
sokf locate planner                 # the absolute path of a document: cat "$(sokf locate planner)"
sokf locate --asset flowers         # the file a sidecar describes; --json gives both paths

sokf info grammar                   # the schema language, verbatim: the tramlines core grammar, the tramlines-md grammar, then SOKF's schema extension
sokf info formats --schema readme   # the line formats a schema sees
sokf new Issue --slug retry --write # a document from its schema's template, at its derived path

sokf completions zsh                # a completion script for your shell
sokf --help
sokf --version
```

`--repo-root <DIR>` sets the repository root. Without it, sokf uses the Git
repository that contains the current directory.

The cache, under `/.sokf/cache/`, holds the results of the last run. With the
cache, a run rechecks only the changed files and the files that depend on
them. With `--no-cache`, sokf reads every file from disk and leaves the cache
unchanged. Both `--repo-root` and `--no-cache` are global options.

## Configuration

Three files under `/.sokf/` configure the tool:

- `/.sokf/config.toml` lists the knowledge roots (default `/knowledge`), the
  paths that sokf skips in its walk of the repository, and the declined
  harness parts.
- `/.sokf/formats.yaml` adds line formats for the whole repository.
- `/.sokf/harness.toml` records the harness parts that the tool wrote.

Every path that the tool prints is relative to the repository root and starts
with `/`, as paths in documents do. Only `locate` prints filesystem paths.
sokf accepts a path argument in either form.

## Agent harness

`sokf harness install claude` makes a repository ready for Claude Code. It
writes the `sokf` skill, an instructions block, a stop hook, and the
permission to run `sokf`. The hook runs `sokf validate --fix` when the agent
ends its turn, and hands the remaining errors back to the agent. The command
touches only the parts that it writes. It keeps a part that you edited unless
you pass `--force`. To refresh the stale parts after an upgrade, run the
command again.

sokf installs the same integration into every agent harness of
[agent-harness-adapter](https://github.com/six5536/agent-harness-adapter). Name one or
more, or `all`. A harness that has no place for a part, shown as none in the
following table, reports the part as unsupported:

| Harness | Name | Instructions | Hook | Permission | Skill |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `claude` | `CLAUDE.md` or `AGENTS.md` | `.claude/settings.json` | `.claude/settings.json` | `.claude/skills` |
| OpenAI Codex | `codex` | `AGENTS.md` | `.codex/hooks.json` | none | `.agents/skills` |
| Factory Droid | `factory` | `AGENTS.md` | `.factory/hooks.json` | none | `.agents/skills` |
| Gemini CLI | `gemini` | `GEMINI.md` | `.gemini/settings.json` | `.gemini/settings.json` | `.agents/skills` |
| GitHub Copilot | `copilot` | `AGENTS.md` | `.github/hooks/sokf.json` | none | `.agents/skills` |
| Cursor | `cursor` | `AGENTS.md` | `.cursor/hooks.json` | `.cursor/cli.json` | `.agents/skills` |
| Pi | `pi` | `AGENTS.md` | `.pi/extensions/sokf.ts` | none | `.agents/skills` |
| OpenCode | `opencode` | `AGENTS.md` | `.opencode/plugins/sokf.ts` | `opencode.json` | `.agents/skills` |
| Kilo Code | `kilo` | `AGENTS.md` | `.kilo/plugins/sokf.ts` | `kilo.json` | `.agents/skills` |
| Qwen Code | `qwen` | `AGENTS.md` | `.qwen/settings.json` | `.qwen/settings.json` | `.agents/skills` |
| Devin | `devin` | `AGENTS.md` | `.devin/hooks.v1.json` | `.devin/config.json` | `.agents/skills` |
| Any `AGENTS.md` agent | `agents` | `AGENTS.md` | none | none | `.agents/skills` |

A file that several harnesses read, such as `AGENTS.md`, is written once, and
the report shows it as `shared` for the others. `--scope user` installs into
your home directory, for every repository, and `--scope local` into the
repository's files that stay out of git (Claude Code and GitHub Copilot). Keep
`/.sokf/harness.local.toml` and `/.sokf/config.local.toml` out of git too.

## Output and exit codes

`sokf validate` prints one line for each finding,
`PATH:LINE: error: MESSAGE`, followed by the counts. `PATH` is the file,
`LINE` is the line in it, and `MESSAGE` describes the finding. Each message
ends with the authority for the finding: the key of the broken criterion, or
the schema and its broken rule:

```
moved   /knowledge/issues/open/issue-003-retry.md -> /knowledge/issues/closed/issue-003-retry.md
rewrote /knowledge/architecture.md
/knowledge/architecture.md:41: error: `[sokf:planer]` resolves to no document (SOKF_FORMAT_61)
/README.md: error: section `## Usage` is missing (schema schema-readme, section "Usage")

2 errors, 0 of 1 warnings listed
```

The last line gives the number of warnings listed out of the total.
`--warnings` lists them all.

Without a start argument, `sokf graph` prints the layout of each knowledge
root in the way that `tree` does, with each store's description and each
document's id:

```
/knowledge - The example knowledge of the graph design.
├── config
├── core/
│   ├── executor
│   └── planner
└── schemas/
    └── subsystem

4 / 4 docs
```

For a document, `sokf graph` prints its edges: `links`, `resource`,
`sources`, and includes, and it adds the inverse of every edge. `(*)` marks a
node whose edges are printed elsewhere, so the output stays finite at any
depth.

For a group of more than 20 entries, the command prints the first 20 and a
line that counts the rest. It folds the includes of one file into one line.
It leaves out the documents that a schema archives, such as closed issues.
`--all` and `--archived` show these entries.

The last line gives the number of documents printed out of those in the whole
view. When the view reaches an archived document, the line also gives the
number of archived documents printed out of their total. The following view
reaches none:

```
planner
├── includes → /crates/app/sokf/src/main.rs#cli
├── depends-on → config
├── depended-on-by → executor
├── references → planer
└── describes → /crates/lib/example-core/src/planner.rs

3 / 3 docs
```

sokf exits with `0` when it finds no error, with `1` when it finds errors,
and with `2` on a usage or internal error. It reports a usage or internal
error on stderr as `error: MESSAGE`, where `MESSAGE` describes the error.
When a downstream pipe closes (`sokf info grammar | head`), sokf stops
without an error.

## Specifications

Three core specifications define the format:
[`sokf-spec/sokf-format-spec.md`](sokf-spec/sokf-format-spec.md),
[`sokf-spec/sokf-schema-spec.md`](sokf-spec/sokf-schema-spec.md), and
[`sokf-spec/sokf-validation-spec.md`](sokf-spec/sokf-validation-spec.md).
They are documents of `sokf-spec`, the knowledge store of SOKF itself. The
schemas in `knowledge/schemas/` show SOKF in use: they are the schemas of the
knowledge store in which sokf's own development is recorded.

The schema language comes from [tramlines](https://github.com/six5536/tramlines),
a Markdown schema engine with its own
[specifications and grammars](https://github.com/six5536/tramlines/tree/main/knowledge/spec).
SOKF's schema extension,
[`sokf-spec/sokf.tramlines.yaml`](sokf-spec/sokf.tramlines.yaml), extends the
tramlines-md grammar. The guide to the extension,
[`sokf-spec/sokf.tramlines.yaml.md`](sokf-spec/sokf.tramlines.yaml.md), sits
beside the extension.

The agent reference, which describes the format on one page for an AI agent,
is the [`sokf` skill](harness/skills/sokf/SKILL.md). `sokf harness install`
writes the skill into a repository.

## Project layout


```
sokf-spec/               the knowledge store of SOKF itself: its specifications, its grammar, and its formats, which sokf validates
harness/                 the embedded harness integration: the sokf skill and the instructions block
crates/lib/sokf-core     the library: it walks the repository, builds the model, hosts SOKF's schema extension in the tramlines engine, checks and fixes the knowledge, reports the findings, builds the graph, runs queries, and installs the harness
crates/app/sokf          the binary: it parses the command line, calls the library, and renders the results as text, JSON, or Mermaid
packages/sokf            the npm launcher (a JS shim that spawns the binary)
packages/sokf-OS-CPU     the five prebuilt-binary packages it selects from
scripts/                 the version and smoke-test scripts that the release runs
.github/workflows/       publish.yml, which builds and publishes each release
```


In `packages/sokf-OS-CPU`, `OS` is `linux`, `darwin`, or `win32`, and `CPU` is
`x64` or `arm64`.

## Development

`.mise.toml` and `rust-toolchain.toml` pin the toolchains, which
[mise](https://mise.jdx.dev/) installs.


```sh
npm run build           # cargo build --workspace
npm run lint            # cargo clippy --workspace
npm run fmt             # cargo fmt --all
npm run verify-version  # check that every version in the tree agrees
cargo run -p sokf -- validate   # run the validator from source
```


For the setup, and for how to report a bug or ask for a feature, see the
[contributing guide](CONTRIBUTING.md).

## Licence

sokf is released under the Elastic License 2.0. The [licence file](LICENSE) states its terms.
