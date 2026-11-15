---
type: Schema
id: schema-issue
title: Issue Schema
description: An issue tracked through its lifecycle.
---

````yaml
line-limit: 200
root: /knowledge/issues
lifecycle:
  - { key: state, states: [open, closed], path: true }
  - { key: kind, states: [bug, feature] }
frontmatter:
  type:
    const: Issue
  id:
    template: "issue-${:seq}-${:slug}"
links:
  - rel: implements
    to-type: Decision
    min: 0
    guidance: The decision the issue delivers.
sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: The index writer drops the entry of a moved file.
  - heading: Reproduction
    level: 2
    only: { kind: bug }
    guidance: Steps that show the bug.
    blocks:
      - kind: numbered-list
        example: Move a document with `--fix`.
  - heading: Conclusion
    level: 2
    only: { state: closed }
    guidance: What was done.
    blocks:
      - kind: prose
        example: The move now rewrites both indexes it touches.
````
