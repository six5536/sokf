---
type: Schema
id: schema-changelog
title: Changelog Schema
description: A changelog records the changes to one specification, with one section for each version and the newest version first, and it is part of the specification that it records.
---

<!-- prettier-ignore -->
````yaml
line-limit: 0

guidance: >
  A changelog records the changes to one specification, so that a reader who knows one
  version finds what each later version changes. The changelog has one section for each
  version, newest first, and each section says what the version changes and what the
  change breaks. The changelog is part of the specification whose changes it records, and
  each section cites the specification's sections and criteria rather than restating them.

frontmatter:
  type:
    const: Changelog

links:
  - rel: part-of
    guidance: The changelog has one part-of link, to the specification whose history it records.

types:
  version: '\d+\.\d+'

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening is one paragraph that names the specification whose changes the changelog
      records, and a contents list follows it.
    blocks:
      - kind: prose
        max: 1
        example: 'This changelog records the versions of the Plumbing contract, which states what a program can rely on when it sends and receives messages through the plumber.'
      - kind: bullet-list
        guidance: >
          The contents list has one item for each section, in order, and each item links to
          its section's heading, so that a reader sees every version before reading any.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Notes](#notes)'
            - '[0.2](#02)'
            - '[0.1](#01)'
  - heading: Notes
    level: 2
    guidance: >
      This section states the versioning rule, which says which versions can break
      compatibility. It comes before the versions, because a reader needs the rule to read
      them.
    blocks:
      - kind: prose
        example: 'Before 1.0, a minor version can break a program that uses the plumber, and its section says how. From 1.0, a minor version only adds to the contract, and only a major version can break compatibility.'
  - heading: { template: '${v:version}', key: v, example: "0.3" }
    level: 2
    repeat: true
    guidance: >
      Each section describes one version, and its heading is the version number, which is
      the section's key. The section says what the version changes and what the change
      breaks. A reader then knows whether a program or a document that conforms to the
      previous version still conforms. An amendment to a version is part of that version's
      section, under the amendment's date.
    examples:
      - |
        ## 0.2

        This version adds the `click` attribute. The change breaks no program: a 0.1 message has no `click` attribute, so the plumber delivers the message unchanged. The version changes one criterion:

        - Click (PLUMB-1_AC-4): a `click` attribute gives the position of the cursor in the data. The plumber sends only the part of the data that the matching rule set's patterns match around that position.
      - |
        ## 0.1

        This is the first version of the contract.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        example: 'This version makes one change that can break a program written for 0.2. A program that opens `/mnt/plumb/rules` for writing while another program has it open fails with the error `file already open`.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: Each item names the changed criterion or section by its key or its heading, and then says what changed.
        example:
          - 'One writer (PLUMB-3): the plumber lets one program at a time open `/mnt/plumb/rules` for writing, so that two writers never mix their rules.'
          - 'Client (PLUMB-1_AC-3): a `plumb client` action holds the message until the program that it starts opens the port.'

````
