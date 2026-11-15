---
type: Schema
id: schema-dataset
title: Dataset Schema
description: A CSV file and its description.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Dataset
  resource:
    example: /knowledge/data/rates.csv
asset-files: ["/knowledge/data/**/*.csv"]
preamble:
  guidance: What the data is, then the data.
  blocks:
    - kind: prose
      max: 1
      example: Exchange rates by day, one currency to a column.
    - kind: include
      repeat: true
      guidance: The data itself, or a region of it.
      example: |
        <!-- sokf:include rates -->
        <!-- /sokf:include -->
````
