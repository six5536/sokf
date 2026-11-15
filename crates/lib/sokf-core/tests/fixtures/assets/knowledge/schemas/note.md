---
type: Schema
id: schema-note
title: Note Schema
description: A note.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Note
links:
  - rel: has-part
    min: 0
    guidance: An asset the note embeds.
sections:
  - heading: { from: title }
    level: 1
    guidance: The title.
    blocks:
      - kind: prose
        example: The kitchen, the week the flowers came.
      - kind: image
        optional: true
        repeat: true
        source-pattern: '^/knowledge/media/'
        guidance: Pictures from the media library.
        example: '![Flowers][sokf:flowers]'
      - kind: include
        optional: true
        repeat: true
        guidance: Data shown in place.
        example: |
          <!-- sokf:include rates -->
          <!-- /sokf:include -->
````
