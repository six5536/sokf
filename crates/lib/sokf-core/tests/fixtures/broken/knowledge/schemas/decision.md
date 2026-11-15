---
type: Schema
id: schema-decision
title: Decision Schema
description: One architectural decision.
---

````yaml
line-limit: 120
frontmatter:
  type:
    const: Decision
sections:
  - heading: { from: title }
    level: 1
    guidance: The decision's title, from frontmatter.
    blocks: []
  - heading: Context
    level: 2
    renamed-from: [Background]
    guidance: The forces in play.
    blocks:
      - kind: prose
        max: 2
        example: The index grows faster than a reader can scan it.
  - heading: Decision
    level: 2
    guidance: The choice, stated as fact.
    blocks:
      - kind: prose
        example: Every directory carries a generated index.
  - heading: Consequences
    level: 2
    guidance: What follows, one consequence per item.
    blocks:
      - kind: bullet-list
        max: 8
        example: An agent reads the index before it reads a document.
prohibited-patterns:
  - pattern: '\bTODO\b'
    message: 'A finished document contains no TODO.'
````
