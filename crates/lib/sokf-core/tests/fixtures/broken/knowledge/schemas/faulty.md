---
type: Schema
id: schema-faulty
title: Faulty
description: A schema whose rule refuses its own example.
---

````yaml
line-limit: 0
root: /knowledge/faulty
lifecycle:
  - { key: phase, states: [a, b], path: true }
frontmatter:
  type:
    const: Faulty
  id:
    template: "faulty-${:seq}"
    seq-width: 2
sections:
  - heading: Only
    level: 1
    guidance: The one section.
    blocks:
      - kind: prose
        example: A draft of the one section.
        each:
          prohibited-patterns: [{pattern: '\bdraft\b', message: 'The prose contains no draft.'}]
````
