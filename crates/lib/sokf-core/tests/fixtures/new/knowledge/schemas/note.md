---
type: Schema
id: schema-note
title: Note Schema
description: A note with a one-word title and no id template.
---

````yaml
line-limit: 40
frontmatter:
  type:
    const: Note
  title:
    pattern: "^[A-Z][a-z]+$"
    example: Groceries
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        max: { hard: 3, when-at-limit: "Start another note for the rest." }
        example: Milk, bread, and the good coffee.
````
