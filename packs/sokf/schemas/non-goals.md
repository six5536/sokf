---
type: Schema
id: schema-non-goals
title: Non-goals schema
description: What the system that a knowledge store describes doesn't do, and why.
---

<!-- prettier-ignore -->
````yaml
line-limit: 200

guidance: >
  The non-goals document lists what the system doesn't do, each thing with its reason. A
  developer reads it before adding a feature, so that nobody builds something that the
  system leaves out on purpose. A knowledge store has one non-goals document, with the id
  `non-goals`.

frontmatter:
  type:
    const: NonGoals
  id:
    const: non-goals
  title:
    template: "What ${} doesn't do"
    example: What the plumber doesn't do
  description:
    guidance: The description says whose non-goals the document lists.
    example: The things that the plumber doesn't do, each with its reason.

sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        max: 1
        guidance: >
          The opening is one paragraph. It says what the list contains, the things that
          the system leaves out on purpose, each with its reason. It also says that a
          developer reads the list before adding a feature.
        example: This document lists the things that the plumber leaves out on purpose, each with its reason. A developer reads the list before adding a feature to the plumber.
      - kind: bullet-list
        guidance: >
          Each item has three parts: a topic in bold with a colon, one sentence about
          something the system doesn't do, and the reason after _Why:_. An item records
          something that a developer would expect the system to do. That thing extends an
          existing feature, or someone proposed it and the maintainers rejected the
          proposal. A rejected way to build one building block belongs in that block's
          design, as a decision. The list has at most 10 items, because a longer list
          becomes a log of every rejection. The change that makes the system do one of
          these things also removes that item.
        rules:
          - template: '**${}**: ${}. Why: ${}.'
            min: 1
            max: 10
            example:
              - '**Delivery confirmation**: The plumber doesn''t tell a sender when the receiving program has read the message. Why: in a user interface, a write to `send` that never waits matters more than certain delivery.'
              - '**Data interpretation**: The plumber doesn''t interpret the data of a message beyond matching the rules'' patterns against it. Why: the rules alone decide where a message goes, and the receiving program decides what the data means.'
              - '**Other users**: The plumber doesn''t pass a message to another user''s programs. Why: one plumber runs for each user on each terminal, and it applies only that user''s rules.'
````
