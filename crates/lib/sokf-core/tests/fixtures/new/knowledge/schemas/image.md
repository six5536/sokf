---
type: Schema
id: schema-image
title: Image Schema
description: A picture in the media library.
---

````yaml
line-limit: 40
root: /knowledge/media
lifecycle:
  - { key: state, states: [draft, published], path: true }
frontmatter:
  type:
    const: Image
  resource:
    example: /knowledge/media/draft/flowers.png
asset-files: ["/knowledge/media/**/*.png"]
preamble:
  guidance: One line under the picture.
  blocks:
    - kind: prose
      max: { hard: 1, when-at-limit: "Put more about the picture in a document that links it." }
      example: Flowers on the kitchen table.
````
