---
type: Schema
id: schema-sokf-format-specification
title: SOKF format specification schema
description: This schema shapes the one document that specifies the SOKF format. It requires each rule to be a criterion in the EARS (Easy Approach to Requirements Syntax) form, and it sets the title, the opening sentence, the sentence about how rules are stated, and the conformance sentence word for word.
---

<!-- prettier-ignore -->
````yaml
line-limit: 0

guidance: >
  The format specification defines what a knowledge store is and what its files hold, for
  the people who build a validator or a consumer and for the writers who maintain a knowledge
  store. It is the only document of its type.

  Each normative rule is a criterion in the EARS form with a key `SOKF_FORMAT_n`. A new
  criterion takes the next free number, and no criterion reuses the number of a removed
  one, because the code, the tests, and other documents cite criteria by their keys.

  The subject of a criterion is the conforming thing, or the role that the rule binds.
  Section 1 defines each subject as a term.

  All text other than the criteria is informative.

  The sections keep their numbers, because the code and other documents cite them by
  number, such as §9.3.

  A named list takes the place of a table, because each item names one thing and says what
  it is.

frontmatter:
  type:
    const: SokfFormatSpecification
  id:
    const: sokf-format-spec
  title:
    const: "SOKF: Format specification"
  description:
    guidance: The description lists the parts of a knowledge store that the specification defines.
    example: The format of a knowledge store, with its manifest, its documents and their frontmatter, its sidecars, its links, and the generated blocks of its files.
  version:
    template: '${:int}.${:int}'
    example: "0.6"
    guidance: The version is the SOKF version that the document defines.
  status:
    enum: [draft, stable]
    guidance: "The status is `draft` while the version can still change, and `stable` after the version is fixed."

types:
  sokf-rule-id: 'SOKF_FORMAT_\d+'
  sokf-subject:
    - knowledge store
    - manifest
    - store identity
    - document
    - frontmatter
    - body
    - source
    - link
    - body link
    - schema
    - Schema document
    - additional file
    - asset
    - sidecar
    - repository file
    - index
    - label
    - fragment
    - slug
    - canonical form
    - key
    - anchor
    - definition block
    - include block
    - region
    - marker
    - trust tier
    - actor
    - role
    - validator
    - agent
    - human
    - writer
    - process
    - consumer

formats:
  sokf-criterion:
    key: id
    variants:
      ubiquitous:
        template: '${id:sokf-rule-id} [ubiquitous]: the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_1 [ubiquitous]: the manifest SHALL declare a slug `name` and a slug `scope`."
      event:
        template: '${id:sokf-rule-id} [event]: WHEN ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_2 [event]: WHEN an agent rewrites the content of a document, the agent SHALL keep each `verified` entry unchanged, byte for byte."
      state:
        template: '${id:sokf-rule-id} [state]: WHILE ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_3 [state]: WHILE a document is in a path-enforced state, the document SHALL be in the directory that its state maps to."
      unwanted:
        template: '${id:sokf-rule-id} [unwanted]: IF ${}, THEN the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_4 [unwanted]: IF a link resolves to no document, THEN the consumer SHALL show the link as a broken edge."
      optional:
        template: '${id:sokf-rule-id} [optional]: WHERE ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_5 [optional]: WHERE a validator offers a fix for a document, the validator SHALL run the fix before it checks the document."
      complex-event:
        template: '${id:sokf-rule-id} [complex]: WHILE ${}, WHEN ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_6 [complex]: WHILE a sidecar is in a path-enforced state, WHEN its state changes, the writer SHALL keep the sidecar beside its asset."
      complex-unwanted:
        template: '${id:sokf-rule-id} [complex]: WHILE ${}, IF ${}, THEN the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_FORMAT_7 [complex]: WHILE a writer copies a document, IF the copy has the original's id, THEN the writer SHALL give the copy a new id."
    prohibited-patterns:
      - pattern: '\b([Ss]hould|[Mm]ust|[Ww]ill|[Mm]ay|[Mm]ight|[Cc]an)\b'
        message: 'The line uses no weak modal verb, such as should, must, will, may, might, or can.'
      - pattern: '\bSHALL\b.*\bSHALL\b'
        message: 'The line holds one SHALL.'
      - pattern: '\b([Ff]ast|[Qq]uickly|[Ee]asy|[Ee]asily|[Uu]ser-friendly|[Aa]ppropriate|[Aa]s needed)\b'
        message: 'The line uses no vague word, such as fast, easy, appropriate, or as needed.'
      - pattern: '\betc\.'
        message: 'The line ends no list with etc.'
      - pattern: 'and/or'
        message: 'The line uses no and/or.'
      - pattern: '\bSHALL (NOT )?be (\w+ed|\w+[^p\W]en|\w+wn|sent|kept|built|left|lost|made|read|set|put|held|told|found|sold|run|done|shut|meant|spent|paid|laid|said|brought|bought|caught|taught|thought)\b'
        message: 'A criterion states its response in the active voice.'

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says what SOKF is and what it is for, and the fixed sentence says what the
      specification defines.
    blocks:
      - kind: prose
        rules:
          - text: "This specification defines the Superdev Open Knowledge Format (SOKF): what a knowledge store is, and what its files hold."
            max: 1
          - pattern: '.'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - 'SOKF is a format for storing canonical knowledge. It combines Markdown files, structured metadata, and the history of a git repository, so that both agents and people can read and maintain the knowledge.'
              - 'A validator checks the knowledge against schemas. It reports each document that departs from the structure of its schema.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each section of the
          specification, in order. Each item links to the section's heading, so that a reader
          sees the outline of the specification before reading any section. Each item includes
          its section's number, as the heading does.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[About this specification](#about-this-specification)'
            - '[1. Terminology](#1-terminology)'
            - '[2. SOKF knowledge store structure](#2-sokf-knowledge-store-structure)'
            - '[3. Documents](#3-documents)'
            - '[4. Write classes](#4-write-classes)'
            - '[5. Identity](#5-identity)'
            - '[6. Sources](#6-sources)'
            - '[7. Verification](#7-verification)'
            - '[8. Relationships](#8-relationships)'
            - '[9. Paths, generated blocks, and markers](#9-paths-generated-blocks-and-markers)'
            - '[10. Conformance](#10-conformance)'
            - '[11. Versioning](#11-versioning)'
  - heading: About this specification
    level: 2
    guidance: This section says, in fixed text, how the specification states its rules.
    blocks:
      - kind: prose
        max: 1
        each:
          text: "Each normative rule of this specification is a criterion in the EARS (Easy Approach to Requirements Syntax) form, with a key of the form `SOKF_FORMAT_n`."
  - heading: "1. Terminology"
    level: 2
    rule-id: section-1
    guidance: >
      This section defines each term of the specification, one item to a term: the term's
      name in backticks, then what it is. The subject of each criterion is one of these terms,
      and the schema checks each subject against the type `sokf-subject`, which lists them.
    blocks:
      - kind: bullet-list
        each:
          template: '`${name}` - ${}'
          key: name
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - '`document` - A Markdown file that has frontmatter and contains one unit of knowledge, such as the contract `contract-plumbing`.'
            - '`label` - The reference of a body link, `sokf:<id>`, optionally with a store before the id and a fragment after it, such as `sokf:contract-plumbing#plumb-1_ac-2`.'
            - '`writer` - An agent or a human that writes documents.'
  - heading: "2. SOKF knowledge store structure"
    level: 2
    rule-id: section-2
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'A knowledge store is a directory in the repository, such as `/knowledge`, and its writers organise the documents into subdirectories as they choose.'
            - 'The path of a document matters only where a schema maps each state of a lifecycle to a directory.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_1 [ubiquitous]: the knowledge store SHALL contain a manifest at its root.'
              - 'SOKF_FORMAT_10 [ubiquitous]: the manifest SHALL declare a slug `name` and a slug `scope`.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`manifest.sokf.yaml` - The manifest, at the root of the knowledge store.'
              - '`schemas/` - The directory that contains the Schema documents, at the root of the knowledge store.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          sokf: "0.6"
          name: plumber
          scope: plan9
          description: The contract, the design, and the decisions of the Plan 9 plumber.
  - heading: "3. Documents"
    level: 2
    rule-id: section-3
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'A document is a UTF-8 Markdown file: a YAML frontmatter block followed by a body in standard Markdown.'
            - 'The schema of a document defines the sections of its body, so every design, such as `design-plumber`, has the same sections in the same order.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_17 [ubiquitous]: the document SHALL contain UTF-8 text that begins with its frontmatter.'
              - 'SOKF_FORMAT_22 [ubiquitous]: the writer SHALL keep each piece of knowledge in exactly one place in the knowledge store.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`frontmatter` - The YAML block between the `---` lines at the top of the document.'
              - '`body` - Everything in the document after its frontmatter.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          type: Design
          id: design-plumber
          title: Plumber
          description: How the plumber matches each message against the rules and delivers it to a port.
          resource: /sys/src/cmd/plumb
  - heading: "4. Write classes"
    level: 2
    rule-id: section-4
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'Every frontmatter field and every manifest key has a write class, which determines who writes it.'
            - 'An agent that rewrites `design-plumber` leaves its `verified` entries exactly as they are, because `verified` is a `restricted` field.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_23 [ubiquitous]: the agent SHALL leave every `restricted` field of a document or manifest unchanged, byte for byte, and add no `restricted` field.'
              - 'SOKF_FORMAT_24 [ubiquitous]: the document SHALL contain no `stamped` field in the working tree.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`open` - Humans, agents, and tools write a field of this class.'
              - '`restricted` - Only humans and processes write a field of this class.'
              - '`stamped` - Export tools write a field of this class at export time.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          generated: { by: sokf/0.6.0, at: 2026-08-04T09:00:00Z }
  - heading: "5. Identity"
    level: 2
    rule-id: section-5
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'A document keeps its id when a writer moves it to another directory, so every link to `design-plumber` still resolves.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_33 [ubiquitous]: the document SHALL have an `id` that no other document in its knowledge store has.'
              - 'SOKF_FORMAT_34 [event]: WHEN a writer renames or moves a document, the writer SHALL keep its `id` unchanged.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`id` - The identifier of the document, a slug such as `design-plumber`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          id: contract-plumbing
  - heading: "6. Sources"
    level: 2
    rule-id: section-6
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'The `sources` of a document list the material that the document is based on, such as the plumber''s paper and its source code.'
            - 'A footnote attributes a claim to a source by the source''s `id`:'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_38 [ubiquitous]: the source SHALL have no keys other than `resource`, `id`, and `title`.'
              - 'SOKF_FORMAT_41 [ubiquitous]: the document SHALL contain only footnotes whose label is the `id` of one of its sources.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`resource` - A path from the repository root, or a URL.'
              - '`id` - The footnote label for the source, local to its document.'
              - '`title` - The name that a consumer displays for the source.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          sources:
            - id: plan9-plumb
              resource: https://github.com/plan9foundation/plan9/blob/main/sys/doc/plumb.ms
              title: Plumbing and Other Utilities (Pike)
  - heading: "7. Verification"
    level: 2
    rule-id: section-7
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'The `verified` field of a document records who has checked the document against the thing it describes, and when.'
            - 'An entry is current until git records a change to the document after the time in the entry''s `at`.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_45 [ubiquitous]: the human SHALL add to `verified` only entries whose `by` identifies that human.'
              - 'SOKF_FORMAT_48 [ubiquitous]: the consumer SHALL derive the trust tier of a document from its current entries.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`unverified` - No current entries.'
              - '`machine-confirmed` - Current entries from `process:` actors only.'
              - '`human-reviewed` - At least one current entry from a `human:` actor.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          verified:
            - { by: human:rob, at: 2026-08-04T09:00:00Z }
            - { by: process:link-checker, at: 2026-08-04T02:00:00Z }
  - heading: "8. Relationships"
    level: 2
    rule-id: section-8
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'A link is a directed edge from its document to the target in `to`, so the `implements` link of `design-plumber` is an edge to `contract-plumbing`.'
            - 'A consumer adds the inverse of every edge, so the graph also has an `implemented-by` edge from `contract-plumbing` to `design-plumber`.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_50 [ubiquitous]: the link SHALL have a string `rel` and a string `to`.'
              - 'SOKF_FORMAT_59 [event]: WHEN a consumer builds a graph, the consumer SHALL add the inverse of every edge.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`depends-on` - Inverse `depended-on-by`. The document relies on the target.'
              - '`implements` - Inverse `implemented-by`. The document satisfies the target, as `design-plumber` satisfies `contract-plumbing`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a manifest or the frontmatter of
          a document, and states no rule of its own, because only a criterion states a rule.
        example: |
          links:
            - rel: implements
              to: contract-plumbing
  - heading: { template: '8.${:int} ${}', example: "8.1 Body links" }
    level: 3
    parent: section-8
    optional: true
    repeat: true
    blocks-ordered: false
    guidance: >
      Each subsection covers one topic of its section, and its number begins with the section's
      number, because the code and other documents cite a subsection by number.
    examples:
      - |
        ### 8.5 Citing part of a document

        A body link points to a key or a heading of a document with a fragment, such as `[PLUMB-1_AC-2][sokf:contract-plumbing#plumb-1_ac-2]`, which cites one criterion of the plumbing contract.

        - SOKF_FORMAT_71 [ubiquitous]: the body link SHALL have a fragment that identifies one key or one heading of its target.
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the subsection and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'A body link refers to a document by its `id`. It is a reference-style Markdown link whose label is `sokf:` followed by the id:'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_61 [ubiquitous]: the body link SHALL resolve to a document.'
              - 'SOKF_FORMAT_62 [unwanted]: IF a body link resolves to no document, THEN the consumer SHALL report it as a broken edge.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`<name>` - A store in the same scope as the citing store.'
              - '`<scope>/<name>` - A store under any root of the repository.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a body link or a reference, and
          states no rule of its own, because only a criterion states a rule.
        example: |
          The plumber implements the [Plumbing][sokf:contract-plumbing] contract.
  - heading: "9. Paths, generated blocks, and markers"
    level: 2
    rule-id: section-9
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'Writers prefer a path from the repository root, such as `/sys/src/cmd/plumb/match.c`, for a link in the body, because the path resolves to the same file wherever the link is.'
            - 'Globs follow gitignore rules: `*` matches within one directory level, and `**` matches at any depth.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_76 [ubiquitous]: the body SHALL link to or embed only paths that resolve to a file or directory in the repository.'
              - 'SOKF_FORMAT_125 [ubiquitous]: the validator SHALL match each glob by gitignore rules, where `*` stops at `/` and `**` matches at any depth.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`/<path>` - A path from the repository root.'
              - '`<path>` - A path relative to the file that contains it, as in standard Markdown.'
              - '`<url>` - An absolute URL.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a definition block or an index,
          and states no rule of its own, because only a criterion states a rule.
        example: |
          [sokf:contract-plumbing]: /knowledge/contracts/contract-plumbing.md
          [sokf:design-plumber]: /knowledge/designs/design-plumber.md
  - heading: { template: '9.${:int} ${}', example: "9.1 Definition blocks" }
    level: 3
    parent: section-9
    optional: true
    repeat: true
    blocks-ordered: false
    guidance: >
      Each subsection covers one kind of generated text, or the markers or the regions of a file, and its number begins with the
      section's number, because the code and other documents cite a subsection by number,
      such as §9.3 for include blocks.
    examples:
      - |
        ### 9.5 Markers

        A marker is a comment line in a repository file outside every knowledge store, such as `/* sokf-implements: PLUMB-1_AC-2 */` above the code of `matchruleset` in `match.c`.

        - SOKF_FORMAT_145 [ubiquitous]: the marker SHALL contain a reference that resolves to one document, or to one key or one heading of a document.
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the subsection and gives their reasons, and it is
          informative. A sentence that says what has to be true is a criterion in a list.
          Only a criterion has a key, and the code and the tests cite that key.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'The definition block ends a file and defines the path of each `sokf:` label that the file uses.'
            - 'Tools write the block, and rewrite it whenever a document or an asset moves, so `[sokf:design-plumber]` still resolves after the design moves.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_FORMAT_79 [ubiquitous]: the definition block SHALL begin with the line `<!-- sokf:links -->` and end the file.'
              - 'SOKF_FORMAT_80 [ubiquitous]: the definition block SHALL contain one reference definition for each `sokf:` label in its file, sorted by label in ascending order.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`rs` - `rust`.'
              - '`yml` - `yaml`.'
              - '`<other>` - The extension itself.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a criterion with an example, such as a definition block or an index,
          and states no rule of its own, because only a criterion states a rule.
        example: |
          <!-- sokf:links -->

          [sokf:contract-plumbing#plumb-1_ac-2]: /knowledge/contracts/contract-plumbing.md#plumb-1_ac-2
          [sokf:design-plumber]: /knowledge/designs/design-plumber.md
  - heading: "10. Conformance"
    level: 2
    rule-id: section-10
    guidance: >
      This section says, in the fixed sentence, when a knowledge store conforms, and then
      lists the faults that a consumer accepts. The list exists because a consumer reads a
      knowledge store even when the store has faults.
    blocks:
      - kind: prose
        rules:
          - text: "A knowledge store conforms to SOKF when it meets each criterion of this specification and of the [schema specification][sokf:sokf-schema-spec] that applies to it."
            max: 1
          - pattern: '.'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: 'A consumer reads a knowledge store even when the store has faults, so a broken link in `design-plumber` removes only one edge from the graph.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item names one
          thing in backticks and then says what it is. A paragraph or a code block separates
          two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_FORMAT_111 [ubiquitous]: the consumer SHALL read a knowledge store that has any of the faults in the following list.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`Viewer` - A consumer that shows the knowledge to a person.'
      - kind: prose
        optional: true
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'A consumer accepts these faults:'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list of faults, one fault to an item: its
          name in backticks, then what it is.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_FORMAT_101 [ubiquitous]: the consumer SHALL read a document whose links are broken.'
          - template: '`${}` - ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '`Broken link` - A link or a body link that resolves to no document.'
              - '`Unknown relation` - A `rel` outside the core set.'
              - '`Lone asset` - An asset without a sidecar.'
  - heading: "11. Versioning"
    level: 2
    rule-id: section-11
    guidance: This section says where the change history of the format is.
    blocks:
      - kind: prose
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The change history of the format is in [the changelog][sokf:changelog].'
````
