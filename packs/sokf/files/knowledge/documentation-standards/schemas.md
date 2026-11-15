---
type: DocumentationStandard
id: documentation-standard-schemas
title: Schema documentation standard
description: How to write the guidance and the examples of a schema in the voice of the documents written from it.
sources:
  - id: plan9-plumb
    resource: https://github.com/plan9foundation/plan9/blob/main/sys/doc/plumb.ms
    title: Plumbing and Other Utilities (Pike)
  - id: plan9-plumb-src
    resource: https://github.com/plan9foundation/plan9/tree/main/sys/src/cmd/plumb
    title: The source code of the Plan 9 plumber
---

# Schema documentation standard

This standard sets the rules for the English of a schema, for a writer, a person or an agent, who writes or changes one. A schema has two kinds of English. Its _guidance_ explains one part of a document to the writer of that document, and its _examples_ stand in for each part of a new document. `sokf new` copies both into every new document, and the writer, an agent most of all, copies their style. So the guidance and the examples use the voice of the document itself.

- [Precedence](#precedence)
- [Guidance](#guidance)
- [Examples](#examples)
- [Example system](#example-system)

## Precedence

When two sources of style disagree, follow the one that comes first:

1. This standard
2. The [general documentation standard][sokf:documentation-standard-general], whose Instructions section gives way to the Guidance section here
3. The [formatting documentation standard][sokf:documentation-standard-formatting]

## Guidance

Write guidance in the voice of the document, so that nothing in it is out of place if it leaks into what the writer writes:

- Describe the part in the third person and the present tense, as the document would. Write "This section says which contracts the block implements", not "Say which contracts the block implements".
- State a rule as a fact about the part, with its reason: write "The diagram has at most nine nodes, because a larger one shows too much for one level", not "Keep the diagram to nine nodes".
- Don't begin a sentence of guidance with a verb, because an imperative addresses the writer, and the document has no writer in it.

## Examples

Write each example as a good document would read:

- Write each example as a sentence of the finished document: in the third person, in the present tense, in the active voice, and about a concrete subject.
- Give each example the detail of a real document in as few words as that detail needs, because the writer copies how much an example says as well as how it says it.
- Give a list or a repeated section several examples, because a writer copies the length of a list as well as its items.
- Begin a labelled item with its label in bold and a colon, as every document does, for example **Port**: a file in `/mnt/plumb` that a program reads messages from.

## Example system

Every example in every schema describes one real system, the plumber of Plan 9 from Bell Labs, so that a writer sees one consistent system in every schema. The plumber's paper[^plan9-plumb] and its source code[^plan9-plumb-src] are the authority for each fact. Where an example needs a fact that neither gives, such as a test or a measurement, the writer invents one. The invented fact fits the others, and the writer adds it to this list. The facts are these:

- The plumber is a file server that passes messages between interactive programs, such as a text editor, an image viewer, and the window system. One plumber runs for each user on each terminal, and it serves its files in `/mnt/plumb`.
- A program sends a message by writing it to `/mnt/plumb/send`, and receives messages by reading a port, such as `/mnt/plumb/edit` or `/mnt/plumb/image`. The file `/mnt/plumb/rules` contains the rules in use, and a write to it changes them at once.
- A message has six header lines: the source application, the destination port, the working directory, the type of the data, the attributes, and the length of the data in bytes. The data follows the header. The message `plumbtest`, `edit`, `/usr/rob/src`, `text`, `addr=27`, `5`, `mem.c` asks the editor to show line 27 of `mem.c`.
- The rules are written in a pattern-action language, as rule sets separated by blank lines. Each rule has an object, a verb, and an argument, such as `data matches '[a-zA-Z0-9_\-./]+'` or `plumb to edit`. The plumber tries the rule sets in order, and the first set whose patterns all succeed decides where the message goes.
- When no rule set matches and the message names no port, the write to `send` fails with the error `no matching plumb rule`. Each program that has opened the destination port receives one copy of the message. When no program has opened it, a `plumb start` or `plumb client` action starts one, and `plumb client` holds the message until the new program opens the port.
- A `click` attribute gives the position of the cursor in the data, and the plumber then sends only the part of the data that the rule set's patterns match around that position.
- The plumber is written in C, in `/sys/src/cmd/plumb`. `plumber.c` starts it, `rules.c` reads the rules, `match.c` matches a message against a rule set, and `fsys.c` serves the files, queues the messages for each port, and starts the program that a `plumb start` or `plumb client` action names. The plumber's types include `Plumbmsg`, `Ruleset`, `Rule`, and `Exec`, and its functions include `readrules`, `matchruleset`, and `startup`. A design calls the modules of these files the main module, the rules reader, the matcher, and the file server.
- A list of the verbs is in `plumber.h`, the parser in `rules.c` turns a verb's name into its number, and `match.c` handles each verb in a case of its own. Nothing checks that the three agree.
- At startup the plumber reads `lib/plumbing` in the user's home directory. When that file has a syntax error, the plumber prints `parse error` and exits. A write to `/mnt/plumb/rules` with a syntax error fails with `malformed rule`. Opening the rules file with truncation clears the rules in use, and closing it creates each port that a rule set names.
- When a program exits with messages in its queue for a port, the plumber discards those messages. A program that opens a port later receives only the messages that arrive after it opened the port.
- The tests are in `/sys/src/cmd/plumb/test/` and `mk test` runs them. `nomatch.rc` sends a message that matches no rule set and reads the error. `jsonschema.rc` generates the JSON Schema file `plumbmsg.schema.json` from the `Plumbmsg` type, so a hand edit to that file is lost at the next run.
- The plumb command's `-j` option prints each message that it reads from a port as a JSON object, and `plumbmsg.schema.json` describes that object.
- A write to `send` returns without waiting for the message to be read, so a sender can't know when it has been read. The design accepts the lack of a delivery acknowledgement, because in a user interface a write that never waits matters more than certain delivery.
- The architecture calls the whole system the plumbing system, with three subsystems: the plumber (`/sys/src/cmd/plumb`), the plumbing library `libplumb` (`/sys/src/libplumb`), and the plumb command (`/sys/src/cmd/plumb/plumb.c`). Programs link `libplumb` to read and write messages, and the plumb command sends a message from the shell.
- The window system `rio`, the editor `acme`, the image viewer `page`, and the mail file system `upas/fs` are external systems. The plumber reads its default rules from `/sys/lib/plumb/basic` and a user's rules from `lib/plumbing` in the user's home directory, both on the Plan 9 file server, a separate machine.
- The plumber's contract is `contract-plumbing`, with the code `PLUMB`, and its design is `design-plumber`, with the code `PLMB`.
- The examples' issue, `issue-001-feature-report-failed-pattern`, describes one change: the matcher reports which pattern failed. With the attribute `explain=yes`, a message that matches no rule set fails with `no matching plumb rule: arg isfile $0 failed in the rule set for edit`. `matchruleset` takes a new argument, through which it returns the number of the failed pattern. An error string of 9P, the Plan 9 file protocol, holds at most 128 bytes, so the error names the failed pattern of the last rule set only.
- A second example issue is a bug: a click at the end of the data matches nothing. The matcher looks only for a match that contains the character at the click position, and at the end of the data there is no such character. The fix lets a match end at the click position. The environment of the bug is Plan 9 from Bell Labs, fourth edition, on a 386 terminal.
- The contract's interfaces are PLUMB-1 `/mnt/plumb/send`, PLUMB-2 `/mnt/plumb/PORT`, and PLUMB-3 `/mnt/plumb/rules`, and its qualities are PLUMB-4 Performance, PLUMB-5 Reliability, PLUMB-6 Compatibility, and PLUMB-7 Usability. PLUMB-1_AC-2 is the error `no matching plumb rule`, and PLUMB-1_AC-4 is the `click` attribute. The change adds PLUMB-1_AC-5, PLUMB-1_AC-6, and PLUMB-1_AC-7. The contract has a version number of its own, 0.3, and the changelog records versions 0.1 to 0.3. The marker `// sokf-implements: PLUMB-1_AC-2` is above the code of `matchruleset` in `match.c`.
- The Stories document is "Following references". In ST-1 a user opens a file at a line from its name, in ST-2 a user shows an image from its file name, and in ST-3 a user changes what a click opens while working.
- The glossary has the terms **Click attribute**, **Message**, **Port**, and **User** in its Domain section, an area "Rules" with **Object**, **Rule**, **Rule set**, and **Verb**, and an area "Actions" with **Action**, **Plumb to**, **Plumb start**, and **Plumb client**.
- The documentation standard is a standard for the plumber's manual pages, `plumber(4)`, `plumb(1)`, `plumb(2)`, and `plumb(6)`, and for the comments in the default rules file. The coding standard is a C standard for `/sys/src/cmd/plumb`, with the topics Error handling, Memory, Names and layout, and Header files.
- The recipe explains how to add a rule set that sends a PostScript file to the image viewer, and how to test it while the plumber runs. The formats file `plumb.formats.yaml`, version 0.3, has the formats `plumb-rule` and `plumb-header` and the types `rule-object`, `rule-verb`, and `header-name`. The repository's own layer of formats is `/.sokf/formats.yaml`. The JSON Schema document describes the output of `plumb -j`.
- The penetration test examines the export of `/mnt/plumb` from the CPU server `cpu1.example.com` with `exportfs`, from 5 to 16 October 2026, with each test account running its own plumber.
- The knowledge store of the plumber has the name `plumber` and the scope `plan9`.

[^plan9-plumb]: Plumbing and Other Utilities (Pike)

[^plan9-plumb-src]: The source code of the Plan 9 plumber

<!-- sokf:links -->

[sokf:documentation-standard-formatting]: /knowledge/documentation-standards/formatting.md
[sokf:documentation-standard-general]: /knowledge/documentation-standards/general.md
