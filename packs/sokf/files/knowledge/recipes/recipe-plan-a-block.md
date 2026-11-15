---
type: Recipe
id: recipe-plan-a-block
title: Plan a block before its code
description: The order in which a writer records a new building block, from the issue to the contract, and a change to an existing block, so that the knowledge validates clean at each stage.
---

# Plan a block before its code

This recipe explains in what order to write the documents of a building block that isn't built yet, so that `sokf validate` reports no error at any stage. Use it when you plan a new block, or a change to a block that exists, before you write its code. It covers the seven stages of a new block, from the issue to its closing, and then the change to an existing block.

The order follows from the schemas. A contract's criteria each need a `sokf-implements:` or a `sokf-tests:` marker, so a contract can't exist before its code. A story's `Contracts:` line has to link a contract, and a node for a planned block looks the same as a node for a built one. Until the code exists, the [issue][sokf:schema-issue] holds the block's criteria and plans its files, and `Planned:` paragraphs that link the issue describe the block elsewhere.

- [Write the issue](#write-the-issue)
- [Mark the plan in the architecture and the stories](#mark-the-plan-in-the-architecture-and-the-stories)
- [Write the design](#write-the-design)
- [Write the code and the tests](#write-the-code-and-the-tests)
- [Move the criteria into the contract](#move-the-criteria-into-the-contract)
- [Replace the planned paragraphs](#replace-the-planned-paragraphs)
- [Close the issue](#close-the-issue)
- [Change an existing block](#change-an-existing-block)

## Write the issue

The issue is the only document of the block at this stage. Its Acceptance criteria section states each criterion of the new block in EARS, because no contract holds the criteria yet. Its Files section links each file that the work creates, and while the issue is open, each linked path counts as existing. A link that ends with `/` plans a whole directory:

```markdown
### Files

- [`/sys/src/cmd/plumb/explain.c`](/sys/src/cmd/plumb/explain.c): the matcher records the pattern that failed.
- [`/sys/src/cmd/plumb/test/`](/sys/src/cmd/plumb/test/): the tests of the report.
```

Create the issue with `sokf new Issue`, and run `sokf validate` after you write it.

## Mark the plan in the architecture and the stories

The [architecture][sokf:schema-architecture] and the [stories][sokf:schema-stories] describe the block only in `Planned:` paragraphs that link the open issue, as the [general documentation standard][sokf:documentation-standard-general#describing-a-system] says. A `Planned:` paragraph after the caption of a subsystem or a group describes the part that the subsystem gains. A `Planned:` paragraph in an existing story describes what the user gains.

The node of the block, the subsystem's own section, and a new story wait for a later stage. A node would look built, and a new story's `Contracts:` line has to link a contract that doesn't exist yet.

## Write the design

The [design][sokf:schema-design] links and includes the planned files before they exist. An include of a planned path holds a sketch of the code, and `sokf validate --fix` keeps the sketch until the file exists:

````markdown
<!-- sokf:include /sys/src/cmd/plumb/explain.c#failedpattern -->
```c
int failedpattern(Ruleset *rs, Plumbmsg *m);
```
<!-- /sokf:include -->
````

The design's opening links the architecture's section of the subsystem that holds the `Planned:` paragraph, and the architecture's Designs section lists the design, because the design schema requires an architecture that cites each design. The design has no `implements` link until the contract exists.

## Write the code and the tests

The code and the tests create the planned files. Each region that the design includes lies between `sokf:begin` and `sokf:end` comment lines, and `sokf validate --fix` then replaces each sketch with the code. Write the `sokf-implements:` and `sokf-tests:` markers in this stage, and commit them with the contract of the next stage, because a marker that cites a key no contract has yet fails validation.

## Move the criteria into the contract

The [contract][sokf:schema-contract] receives the criteria from the issue in the same commit as the markers that cite them, because a marker that cites a missing key fails validation, and a criterion that no marker cites fails coverage. For a new contract, run `sokf new Contract`. Each EARS item of the issue's Acceptance criteria becomes a criterion of the contract, and the item in the issue becomes a link to the criterion's key. The design gains its `implements` link to the contract.

## Replace the planned paragraphs

The architecture and the stories now describe the built block. In the architecture, the node in the subsystem's diagram, the subsystem's section, and the contract's entry in the Contracts section replace the `Planned:` paragraphs. In the stories, a `Contracts:` line that links the contract, in an existing story or in a new one, replaces the `Planned:` paragraph. Replace the paragraphs before the issue closes, because a `Planned:` paragraph that links a closed issue fails validation.

## Close the issue

The issue closes after every criterion holds. Tick each item of its Acceptance criteria, write its Outcome, and set `state: closed`. `sokf validate --fix` then moves the issue to the directory of its state.

## Change an existing block

For a change to a block that exists, the contract stays the authority, because its criteria and their markers stay true until the code changes:

1. The issue states each new criterion in EARS in its Acceptance criteria, and links each existing criterion that the change keeps or alters by its key.
2. `Planned:` paragraphs that link the issue describe the change in the contract's Inputs and Outputs, and in the design's Interfaces and Data where the change reaches them.
3. The code, the tests, and the contract's new or changed criteria go into one commit with their markers, and the description of what now exists replaces each `Planned:` paragraph.
4. The architecture and the stories change as in [Replace the planned paragraphs](#replace-the-planned-paragraphs), and the issue closes.

<!-- sokf:links -->

[sokf:documentation-standard-general#describing-a-system]: /knowledge/documentation-standards/general.md#describing-a-system
[sokf:schema-architecture]: /knowledge/schemas/architecture.md
[sokf:schema-contract]: /knowledge/schemas/contract.md
[sokf:schema-design]: /knowledge/schemas/design.md
[sokf:schema-issue]: /knowledge/schemas/issue.md
[sokf:schema-stories]: /knowledge/schemas/stories.md
