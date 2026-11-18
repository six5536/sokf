---
type: Schema
id: schema-note
title: Note Schema
description: A note that records decisions. The unknown key `bogus` breaks the grammar.
---

````yaml
line-limit: 0
bogus: 1
types:
  decision-id: 'D-[1-9][0-9]*'
frontmatter:
  type:
    const: Note
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: Some text.
      - kind: bullet-list
        each:
          template: '${id:decision-id} ${}.'
          key: id
          example: 'D-1 One.'
````
