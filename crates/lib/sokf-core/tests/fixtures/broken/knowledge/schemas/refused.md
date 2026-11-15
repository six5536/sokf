---
type: Schema
id: schema-refused
title: Refused
description: A root that is not rooted and a template that admits a non-slug.
---

````yaml
line-limit: 0
root: elsewhere
lifecycle:
  - { key: phase, states: [a], path: true }
frontmatter:
  type:
    const: Refused
  id:
    template: "Refused-${:seq}"
    seq-width: 2
sections: []
````
