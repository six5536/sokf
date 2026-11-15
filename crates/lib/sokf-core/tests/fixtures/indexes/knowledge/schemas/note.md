---
type: Schema
id: schema-note
title: Note schema
description: A note.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Note
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: Milk, bread, and the good coffee.
````
