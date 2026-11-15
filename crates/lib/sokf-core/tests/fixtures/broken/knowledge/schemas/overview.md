---
type: Schema
id: schema-overview
title: Overview Schema
description: An overview that materializes other documents.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Overview
links:
  - rel: references
    min: 0
    guidance: What the overview cites.
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: What the tool is made of, one include to a part.
      - kind: include
        repeat: true
        guidance: An include of a document by its id.
        example: |
          <!-- sokf:include planner -->
          <!-- /sokf:include -->
````
