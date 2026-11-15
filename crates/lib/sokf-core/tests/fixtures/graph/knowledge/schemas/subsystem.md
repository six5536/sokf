---
type: Schema
id: subsystem
title: Subsystem
---

# Subsystem

````yaml
line-limit: 0
frontmatter:
  type:
    const: Subsystem
links:
  - rel: depends-on
    min: 0
    guidance: What the subsystem needs.
  - rel: references
    min: 0
    guidance: What the subsystem cites.
sections:
  - heading: { from: title }
    level: 1
    guidance: The title.
    blocks:
      - kind: prose
````
