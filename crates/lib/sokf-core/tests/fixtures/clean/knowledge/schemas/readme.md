---
type: Schema
id: schema-readme
title: Readme Schema
description: The repository readme.
---

````yaml
line-limit: 0
additional-files: /README.md
sections:
  - heading: sokf fixture
    level: 1
    guidance: The title.
    blocks:
      - kind: prose
        example: The fixture's readme, with the usage its source declares.
  - heading: Usage
    level: 2
    guidance: The CLI, materialized from the source.
    blocks:
      - kind: include
        source-pattern: '^/src/.*\.rs#'
        guidance: A named region of a Rust source file.
        example: |
          <!-- sokf:include /src/cli.rs#usage -->
          <!-- /sokf:include -->
````
