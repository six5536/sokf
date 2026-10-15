---
type: Schema
id: schema-formats
title: Formats schema
description: A formats document describes a file of shared line formats, so that a schema author can bind a format by name without reading the file.
---

<!-- prettier-ignore -->
````yaml
line-limit: 250
root: /knowledge/formats

asset-files: /knowledge/formats/**/sokf.formats.yaml

guidance: >
  A formats document describes a file of shared line formats, so that a schema author can
  bind a format by name without reading the file. The document explains the formats rather
  than copying the file, for two reasons: the file is the authority, and `sokf info formats`
  prints every entry.

frontmatter:
  type:
    const: Formats
  version:
    template: '${:int}.${:int}'
    example: "0.3"
    guidance: The version is the file's `version`, as a major and a minor number.

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says what the document helps its reader do and what it contains. It
      names the kinds of line that the formats describe and the sections that follow. It
      names the kinds rather than counting the formats, because a count becomes wrong
      when the file changes.
    blocks:
      - kind: prose
        example:
          - 'This document describes `plumb.formats.yaml`, so that the author of a plumber schema can bind a format by name without reading the file. The formats describe the lines of a plumbing rule and the header lines of a message.'
          - 'Each entry of the file states the shape of a line, the patterns that the line must not contain, and an example. The document covers the types, the formats, how a schema binds a format, and the order in which sokf looks up a format name.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each level-2 section, in
          order. Each item links to the section's heading.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Types](#types)'
            - '[Formats](#formats)'
            - '[Binding a format](#binding-a-format)'
            - '[Layers](#layers)'

  - heading: { template: '${}', example: "Types" }
    level: 2
    repeat: true
    guidance: >
      The sections come in this order: the types, then the formats that use them, then
      how a schema binds a format, then the layers in which sokf looks up a format name.
      A section holds its entries in a list rather than a table, because a list item keeps
      each entry on one line of the source. Each item begins with the entry's name in bold
      and a colon.
    examples:
      - |
        ## Formats

        The file defines these formats:

        - **`plumb-rule`**: `${:rule-object} ${:rule-verb} ${}`, one rule of a rule set, such as `plumb to edit`
        - **`plumb-header`**: `${:header-name} ${}`, one of the six header lines that come before the data of a message
      - |
        ## Binding a format

        A unit rule binds a format with the property `format`. The following block rule requires every item of its block to be a plumbing rule:

        ```yaml
        each: { format: plumb-rule }
        ```

        sokf reports each item that doesn't match the format, and shows the format's example.
      - |
        ## Layers

        sokf looks up a format name in three layers, nearest first, and uses the nearest definition:

        - **Schema**: the `formats` and `types` of the schema that binds the format
        - **Repository**: the file `/.sokf/formats.yaml`, which has the same shape as `plumb.formats.yaml`
        - **Shared**: `plumb.formats.yaml`

        A change for the whole repository belongs in `/.sokf/formats.yaml`, because a change in a schema applies to that schema alone.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        example: 'The file defines these types, in addition to the built-in types:'
      - kind: bullet-list
        optional: true
        repeat: true
        example:
          - '**`rule-object`**: `src`, `dst`, `wdir`, `type`, `attr`, `data`, `arg`, or `plumb`'
          - '**`rule-verb`**: `is`, `isdir`, `isfile`, `matches`, `set`, `add`, `delete`, `to`, `start`, or `client`'
          - '**`header-name`**: `src`, `dst`, `wdir`, `type`, `attr`, or `ndata`'
      - kind: code
        optional: true
        repeat: true
        lang: any
````
