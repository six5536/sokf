# sokf

`sokf` validates knowledge stores in the Superdev Open Knowledge Format
(SOKF). A knowledge store is a set of Markdown documents with YAML frontmatter
in a git repository. `sokf` checks each document against its schema, resolves
its links, keeps its generated content current, and allocates its ids. It
ships as prebuilt binaries for Linux, macOS, and Windows. The release workflow
of the sokf repository publishes each release to GitHub Packages from a single
git tag.

This README helps you install and run `sokf`. It covers installation, usage,
the documentation, and the licence:

- [Install](#install)
- [Usage](#usage)
- [Documentation](#documentation)
- [Licence](#licence)

## Install

```sh
npm install -g @six5536/sokf
```

This package only starts a prebuilt binary. It declares a prebuilt binary for each
supported platform as an `optionalDependency`, and npm installs only the
binary that matches your machine. When you run `sokf`, the JavaScript
launcher starts that binary.

The npm package supports Linux and macOS (`x64` and `arm64`) and Windows
(`x64`). On any other platform, or to build from source, install with the Rust
toolchain instead:

```sh
cargo install sokf
```

## Usage

```sh
sokf init [DIR]                   # create a knowledge store and register it as a root
sokf harness install claude       # install the sokf skill, instructions, stop hook, and permission for Claude Code (or the parts that codex, cursor, gemini, copilot, factory, pi, opencode, kilo, qwen, devin, agents, or all support)
sokf validate                     # check the knowledge stores and additional files
sokf validate --fix               # regenerate blocks and indexes, move each document to the folder that matches its state (a closed issue moves to closed/), rewrite headings
sokf validate --json              # print one JSON object on stdout; --query EXPR selects parts of it
sokf graph [START...]             # print the knowledge as a tree from any start point, as text, JSON (--json), or Mermaid (--mermaid)
sokf locate ID...                 # print the absolute path of each document
sokf info grammar                 # describe the schema language that the validator uses
sokf info formats --schema ID     # list the line formats that a schema can match
sokf new TYPE --slug SLUG --write # create a document from its schema's template
sokf completions zsh              # print a completion script for your shell
```

The commands use these placeholders, and a placeholder in square brackets is
optional:

- `DIR`: the directory of the new knowledge store.
- `START`: a document id, a key, or the path of a file or a directory, from
  which `sokf graph` starts.
- `ID`: a document id, such as `glossary`.
- `EXPR`: a JMESPath expression.
- `TYPE`: a document type, such as `Issue`.
- `SLUG`: the slug that `sokf new` puts in the id of the new document.

`sokf` exits with `0` when it finds no error, `1` when it finds errors, and
`2` on a usage error or an internal error.

## Documentation

The core specifications of the format, the contracts of the tool, the
grammar guide, and the development notes are in the
[sokf repository on GitHub](https://github.com/six5536/sokf#readme).

## Licence

The
[Elastic License 2.0](https://github.com/six5536/sokf/blob/main/LICENSE)
covers `sokf`.
