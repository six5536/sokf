---
type: Schema
id: schema-readme
title: Readme Schema
description: The repository readme, with its rules. The unknown key `bogus` breaks the grammar.
---

````yaml
line-limit: 0
bogus: 1
additional-files: /README.md
types:
  rule-id: 'R-[1-9][0-9]*'
sections:
  - heading: Readme
    level: 1
    blocks:
      - kind: prose
        example: Some text.
      - kind: bullet-list
        each:
          template: '${id:rule-id} ${}.'
          key: id
          example: 'R-1 One rule.'
````
