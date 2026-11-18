---
type: Schema
id: schema-note
title: Note Schema
description: A note with a preamble.
---

````yaml
line-limit: 0
root: /knowledge/notes
frontmatter:
  type:
    const: Note
  id:
    template: "note-${:slug}"
preamble:
  guidance: The preamble says what the note is about.
  blocks:
    - kind: prose
      guidance: One paragraph introduces the note.
      example: This note records the quirks of the kettle.
sections:
  - heading: Details
    level: 2
    blocks:
      - kind: bullet-list
        guidance: Each item names one quirk, with its key before a colon.
        each:
          template: '${id:slug}: ${}'
          key: id
          unique: knowledge
          example: 'lid: The lid sticks.'
      - kind: code
        optional: true
        min: 0
        lang: any
        max: { hard: 0, when-at-limit: "Describe the code in the note's text." }
        guidance: A note holds no code.
        example: x
````
