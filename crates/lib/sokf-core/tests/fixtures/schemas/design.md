---
type: Schema
id: schema-design
title: Design schema
description: A design says how one building block is built, for the tests of a design's scaffold.
---

````yaml
line-limit: 0
root: /knowledge/designs

frontmatter:
  type:
    const: Design
  id:
    template: "design-${:slug}"

sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: 'This design describes the cache, which keeps the findings of each check.'

  - heading: Concerns
    level: 2
    optional: true
    rule-id: concerns
    blocks: []

  - heading: { template: '${}', example: Errors }
    level: 3
    repeat: true
    parent: concerns
    blocks:
      - kind: prose
        example:
          - 'A read of a row that doesn''t parse fails, and the cache drops the row.'
          - 'A failed write leaves the old row in place:'
      - kind: bullet-list
        optional: true
        example:
          - 'A write that the disk refuses fails, and the cache keeps the old row.'
          - 'A write of a row that no check produced fails, and the cache stores nothing.'
      - kind: group
        optional: true
        repeat: true
        blocks:
          - kind: prose
            max: { soft: 1, when-at-limit: "Say the rest in the paragraphs before the example." }
            each:
              pattern: '^[A-Z`].*:$'
              example: 'The test `foreign_row` writes a row that no check produced, and checks that the write fails with this error:'
          - kind: code
            lang: any
            example: |
              no such check

  - heading: Limits
    level: 2
    optional: true
    blocks:
      - kind: prose
        example: 'The cache can''t tell a stale row from a fresh one without the hash of its file.'
````
