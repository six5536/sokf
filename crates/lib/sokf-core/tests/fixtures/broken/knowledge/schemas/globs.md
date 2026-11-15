---
type: Schema
id: schema-globs
title: Globs
description: Both dispatches, a bad glob, and a lifecycle beside globs.
---

````yaml
line-limit: 0
additional-files: ["docs/*.md", "/knowledge/decisions/*.md", "/docs/**/*.txt"]
frontmatter:
  type:
    const: Both
lifecycle:
  - { key: s, states: [x] }
sections: []
````
