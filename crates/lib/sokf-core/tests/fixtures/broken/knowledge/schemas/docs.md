---
type: Schema
id: schema-docs
title: Docs
description: Governs docs with a warning-level rule.
---

````yaml
line-limit: 3
additional-files: "/docs/**/*.md"
sections:
  - heading: Guide
    level: 1
    blocks:
      - kind: prose
        severity: warn
        example: Run `sokf validate` before you commit.
````
