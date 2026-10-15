---
type: Schema
id: schema-json-schema
title: JSON Schema sidecar schema
description: A JSON Schema sidecar sits beside a JSON Schema file and tells a developer, in one paragraph, what the JSON output is and how the file is kept current.
---

<!-- prettier-ignore -->
````yaml
line-limit: 20
root: /knowledge/json

asset-files: /knowledge/json/**/*.schema.json

frontmatter:
  type:
    const: JsonSchema

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The heading is the title from the frontmatter, and one paragraph follows it. The paragraph
      tells a developer what the JSON output is and how a test keeps the schema file current.
      It describes the output first, then names the command and option that print it. For
      output in a file, it names the program that writes the file. A link to the contract part
      that defines the output follows, when a contract defines it. The paragraph then names
      the test that generates the schema file. It also names the command that runs the test.
      The test overwrites
      any hand edit to the file, so the paragraph says where a change belongs. The paragraph
      describes the output as a whole, and the schema file describes each key. The test
      generates each key's description from the type, so a key described in the paragraph
      would fall out of date.
    blocks:
      - kind: prose
        max: 1
        example: 'Each JSON object holds one message: the source application, the destination port, the working directory, the type of the data, the attributes, the length of the data in bytes, and the data itself. The plumb command prints one such object for each message that it reads from a port when it runs as `plumb -j`. The file `plumbmsg.schema.json` describes each key of the object. The test `jsonschema.rc` generates that file from the `Plumbmsg` type, so `mk test` replaces any hand edit to the file. A change to the object belongs in the `Plumbmsg` type.'
````
