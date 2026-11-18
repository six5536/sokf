---
type: Schema
id: schema-issue
title: Issue schema
description: An issue lies in the folder of its state, for the tests of the indexes that `sokf new` writes.
---

<!-- prettier-ignore -->
````yaml
line-limit: 0
root: /knowledge/issues
lifecycle:
  - { key: state, states: [open, closed], path: true }

frontmatter:
  type:
    const: Issue
  id:
    template: "issue-${:seq}-${:slug}"

sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        example: 'The index writer drops the entry of a moved file.'
````
