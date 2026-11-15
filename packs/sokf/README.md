# The sokf pack

The sokf pack, `@sokf/sokf`, is a method for a project's knowledge in [SOKF](../../README.md). It holds the schemas of an architecture, stories, contracts, designs, issues, recipes, a glossary, non-goals, and standards, the line formats that those schemas share, three documentation standards, a recipe for rewriting a document, a recipe for planning a block before its code, and the instructions that give the order of the work.

The pack is a pack of [pakf](https://github.com/six5536/pakf), the tool that shares project files through packs. This README helps you take the pack into a project and says what it holds:

- [Take the pack](#take-the-pack)
- [What the pack holds](#what-the-pack-holds)
- [Versions](#versions)

## Take the pack

In a git repository with a knowledge store at `/knowledge`, such as one that `sokf init` created, take the pack of a release of sokf, 0.6.0 or later, with its tag:

```sh
pakf add https://github.com/six5536/sokf.git --ref v0.6.0 --path packs/sokf
sokf validate --fix
```

`pakf add` copies each file of the pack to its path in the project, and `sokf validate --fix` writes the index of each directory that the pack fills. A later `pakf sync` merges each change of the pack into your edits. pakf's own README describes the commands that keep the pack current.

The pack's schemas place their documents under `/knowledge/`, such as `root: /knowledge/recipes`, so the pack fits a project whose store lies at `/knowledge`.

## What the pack holds

The pack is one tree of files under `files/`, each at the path where it lands in the project:

```text
packs/sokf/
├── pakf.yaml                                       the manifest
├── instructions.md                                 the order of the work, for coding agents
└── files/
    ├── knowledge/schemas/                          the schemas
    ├── knowledge/documentation-standards/          the documentation standards
    ├── knowledge/recipes/                          the recipes
    ├── .sokf/formats.d/pakf/@sokf/sokf/formats.yaml   the line formats that the schemas share
    └── .pakf/ignore.d/@sokf/sokf/ignore            the indexes that sokf writes, which pakf patch leaves out
```

sokf reads the formats as a file of its folder `/.sokf/formats.d/`, as it reads any file there. The ignore file names `**/index.md`, because sokf writes an index in every directory of a store, and no pack owns one.

## Versions

The pack takes the version of sokf, and each release of sokf tags it. Take the pack at the tag of the sokf that the project runs, because a schema of the pack can use a rule that only that sokf reads.
