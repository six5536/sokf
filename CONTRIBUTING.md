# Contributing to sokf

This repository is a read-only mirror of sokf. Each release reaches it as one commit, which an export from the project's own repository creates, and nobody commits to it directly. This guide explains how to report a bug or ask for a feature, and how to build sokf from this repository.

- [Report a bug or ask for a feature](#report-a-bug-or-ask-for-a-feature)
- [Build sokf](#build-sokf)

## Report a bug or ask for a feature

Open an issue, with the bug report or the feature request template. A pull request can't be merged here, because the next release replaces the whole tree. When you have a change in mind, describe it in an issue, and include a patch there if you have one.

## Build sokf

The project pins its toolchains in `rust-toolchain.toml` and `.mise.toml`, which [mise](https://mise.jdx.dev/) installs:

```sh
mise install
cargo build --locked -p sokf
```

The build takes the schema engine, the `tramlines-core` and `tramlines-md` crates of [tramlines](https://github.com/six5536/tramlines), from crates.io.

To build the npm packages as the release does, install the JavaScript workspace and build each platform's binary:

```sh
npm install
npm run build:all
```

`.github/workflows/publish.yml` is the release: it builds the five binaries, smoke-tests them, publishes the packages, and creates the GitHub release with the notes that the tag's message carries. The GitHub releases hold the notes of every release: they are the project's changelog.
