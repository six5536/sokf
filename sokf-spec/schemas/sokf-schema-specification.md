---
type: Schema
id: schema-sokf-schema-specification
title: SOKF schema specification schema
description: "This schema governs the one document that specifies SOKF schemas: its rules are criteria in the EARS (Easy Approach to Requirements Syntax) form, and some of its sections hold fixed sentences."
---

<!-- prettier-ignore -->
````yaml
line-limit: 0

guidance: >
  The schema specification defines how a schema constrains the documents of a knowledge
  store, for the people and agents who write schemas and the validators that check them.
  The store contains one document of this type. Each normative rule is a criterion in the
  EARS form, with a key of the form `SOKF_SCHEMA_NUMBER`, where `NUMBER` is the number of
  the criterion. A new criterion takes the next free number, and no criterion reuses the
  number of a removed one, because other documents and the code cite criteria by key. The
  subject of each criterion is the conforming thing, or the role that the rule binds. Each
  subject is one of the terms of section 1. All other text is informative. A named list, a
  bullet list whose items each name one thing in backticks and then say what it is, takes
  the place of a table. The sections keep their
  numbers, because the specification and the documents that cite it refer to a section by
  its number, such as §7.

frontmatter:
  type:
    const: SokfSchemaSpecification
  id:
    const: sokf-schema-spec
  title:
    const: "SOKF: Schema specification"
  description:
    guidance: The description says what the specification defines, in one sentence.
    example: "This specification defines the schemas of a knowledge store: what a schema governs, and its rules, scaffolds, lifecycles, and templates."
  version:
    template: '${:int}.${:int}'
    example: "0.6"
    guidance: The version is the SOKF version that the document defines.
  status:
    enum: [draft, stable]
    guidance: "The status is `draft` while the version is open to change, and `stable` after the version is fixed."

types:
  sokf-rule-id: 'SOKF_SCHEMA_\d+'
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
    - unit
    - template
    - placeholder
    - lifecycle
    - dimension
    - scaffold

formats:
  sokf-criterion:
    key: id
    variants:
      ubiquitous:
        template: '${id:sokf-rule-id} [ubiquitous]: the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_1 [ubiquitous]: the schema SHALL declare `root` where it declares a path-enforced dimension."
      event:
        template: '${id:sokf-rule-id} [event]: WHEN ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_2 [event]: WHEN an agent rewrites the content of a document, the agent SHALL keep each `verified` entry byte for byte."
      state:
        template: '${id:sokf-rule-id} [state]: WHILE ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_3 [state]: WHILE a document is in a path-enforced state, the document SHALL be in the directory derived from its states."
      unwanted:
        template: '${id:sokf-rule-id} [unwanted]: IF ${}, THEN the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_4 [unwanted]: IF a link resolves to no document, THEN the consumer SHALL show the link as a broken edge."
      optional:
        template: '${id:sokf-rule-id} [optional]: WHERE ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_5 [optional]: WHERE a validator offers a fix, the validator SHALL run the fix before it checks the document."
      complex-event:
        template: '${id:sokf-rule-id} [complex]: WHILE ${}, WHEN ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_6 [complex]: WHILE a sidecar is in a path-enforced state, WHEN its state changes, the writer SHALL move the sidecar with its asset."
      complex-unwanted:
        template: '${id:sokf-rule-id} [complex]: WHILE ${}, IF ${}, THEN the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_SCHEMA_7 [complex]: WHILE a writer copies a document, IF the copy has the id of the original, THEN the writer SHALL give the copy a new id."
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
      The opening is the fixed sentence that says what the specification defines.
      Informative sentences follow it and say what SOKF is and what a schema does for a store.
    blocks:
      - kind: prose
        rules:
          - text: "This specification defines the schemas of the Superdev Open Knowledge Format (SOKF): how a schema constrains the documents of a knowledge store."
            max: 1
          - pattern: '.'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: 'SOKF is a format in which agents and people both read and maintain the knowledge of a system, such as the contract and the design of the Plan 9 plumber. A schema states what each type of document contains, so that every design in a store has the same sections.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each section of the
          specification, in order. Each item links to the section's heading, so that a reader
          sees the outline of the whole specification before any of its sections.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[About this specification](#about-this-specification)'
            - '[1. Terminology](#1-terminology)'
            - '[2. Schema documents](#2-schema-documents)'
            - '[3. What a schema governs](#3-what-a-schema-governs)'
            - '[4. Rules](#4-rules)'
            - '[5. Scaffolds](#5-scaffolds)'
            - '[6. Lifecycle](#6-lifecycle)'
            - '[7. Templates](#7-templates)'
            - '[8. Id templates](#8-id-templates)'
            - '[9. Schema terms](#9-schema-terms)'
  - heading: About this specification
    level: 2
    guidance: This section is fixed text that says how the specification states its rules.
    blocks:
      - kind: prose
        max: 1
        each:
          text: "Each normative rule of this specification is a criterion in the EARS (Easy Approach to Requirements Syntax) form, with a key of the form `SOKF_SCHEMA_NUMBER`, where `NUMBER` is the number of the criterion. The descriptions of the keys in the tramlines-md grammar and in the [SOKF schema extension][sokf:sokf-extension] are normative."
  - heading: "1. Terminology"
    level: 2
    rule-id: section-1
    guidance: >
      This section lists the terms that the specification adds to those of the format
      specification, one in each item: the term in backticks, then its definition. The
      subject of each criterion is one of these terms, because the schema declares them as
      the type `sokf-subject`, and the validator checks each subject against that type.
    blocks:
      - kind: prose
        max: 1
        each:
          text: "This specification uses the terms of the [format specification][sokf:sokf-format-spec], and adds these:"
      - kind: bullet-list
        each:
          template: '`${name}`: ${}'
          key: name
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - '`unit`: A heading, paragraph, list item, table row, line of a fence, or frontmatter value, such as the heading of the Approach section of the plumber''s design.'
            - '`placeholder`: A typed element of a template, `${name:type}`, that matches any value of its type, as `${:slug}` matches `plumber`.'
            - '`scaffold`: The starting document that a validator derives from a schema for one combination of the states of its lifecycle.'
  - heading: "2. Schema documents"
    level: 2
    rule-id: section-2
    guidance: This section says how a Schema document contains its schema.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The Schema document `knowledge/schemas/design.md` contains the schema of the type `Design`, which governs the design of the plumber and every other design in the store.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_20 [ubiquitous]: the Schema document SHALL contain its schema as a single fenced YAML block.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`line-limit`: The largest number of lines that a document of the type can contain, such as 2000 for a design.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: |
          type: Schema
          id: schema-design
          title: Design Schema
  - heading: "3. What a schema governs"
    level: 2
    rule-id: section-3
    guidance: This section describes the three ways for a schema to identify the files that it governs, and the rules for each.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The design schema declares the `type` const `Design`, so it governs `design-plumber` wherever the design is in its knowledge store.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_30 [ubiquitous]: the schema SHALL identify the files that it governs in exactly one of the three ways.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`type`: A `type` const in the `frontmatter` rules of the schema, such as `Design`. The schema governs every document of that type.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: |
          frontmatter:
            type:
              const: Design
  - heading: "4. Rules"
    level: 2
    rule-id: section-4
    guidance: This section says how a document conforms to its schema, and which of its links and citations count toward the entries of its schema.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The design of the plumber links to `contract-plumbing` with the relationship `implements`, and that link counts toward the `implements` entry in the `links` of the design schema.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_40 [ubiquitous]: the document SHALL have only links whose `rel` appears in an entry of its schema.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`implements`: The document describes how its building block keeps the promises of the target, as `design-plumber` describes how the plumber keeps the promises of `contract-plumbing`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: |
          links:
            - rel: implements
              to-type: Contract
              min: 0
  - heading: "5. Scaffolds"
    level: 2
    rule-id: section-5
    guidance: This section says how a validator derives the scaffolds of a schema, and which markers a scaffold contains.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The command `sokf new Design` writes a scaffold whose examples describe the plumber, and the writer replaces each example with the text of the new design.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_50 [ubiquitous]: the scaffold SHALL contain the `guidance` of the schema as `sokf:guidance` lines.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`sokf:optional`: The pair of markers around a section or a block that the schema allows but does not require, such as the Approach section of a design.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: |
          <!-- sokf:optional -->
          ## Approach
          <!-- /sokf:optional -->
  - heading: "6. Lifecycle"
    level: 2
    rule-id: section-6
    guidance: This section says how a schema declares a lifecycle, how a document records its state in each dimension, and how a path-enforced state determines the directory of the document.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'While its state is `open`, the issue for the matcher''s report of the failed pattern is in `knowledge/issues/open/`. After its state changes to `closed`, the validator''s fix moves the issue to `knowledge/issues/closed/`.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_60 [ubiquitous]: the document SHALL be in the directory that its path-enforced states determine.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`root`: The directory that contains the directories of the path-enforced states, such as `/knowledge/issues`, which contains `open/` and `closed/`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: "state: open"
  - heading: "7. Templates"
    level: 2
    rule-id: section-7
    guidance: This section says how a rule fixes the shape of a unit with text, a template, or a pattern, and which types a placeholder can have.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The template `design-${:slug}` matches the id `design-plumber`, because the placeholder `${:slug}` matches `plumber`.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_70 [ubiquitous]: the schema SHALL give each rule that fixes the shape of a single unit exactly one of `text`, `template`, and `pattern`.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`template`: The text that a unit matches, with a typed placeholder wherever the text varies, such as `design-${:slug}`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: |
          code:
            template: '${:requirement-code}'
            example: 'PLMB'
  - heading: "8. Id templates"
    level: 2
    rule-id: section-8
    guidance: This section says how a schema constrains the id of a document with a template or a const, and how a tool fills in the template.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The command `sokf new Design --slug plumber` fills the id template `design-${:slug}` to make the id `design-plumber`, and the design keeps that id when its file moves.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_80 [ubiquitous]: the schema SHALL declare only id templates that become slugs when `x` replaces each placeholder.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`${:slug}`: One or more slug words, such as `plumber`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: |
          id:
            template: "design-${:slug}"
  - heading: "9. Schema terms"
    level: 2
    rule-id: section-9
    guidance: This section lists the keys that the SOKF schema extension adds to the terms of the tramlines-md grammar.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          A paragraph explains a rule and is informative. A sentence that says what has to be
          true is a criterion, not prose, because only a criterion is normative.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example: 'The key scope `repository` requires a key such as `PLUMB-1` to be unique across every knowledge store in the repository. A bare link to `PLUMB-1` therefore resolves to the plumber''s contract.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list contains criteria, or it is a named list in place of a table. A sentence
          before a named list says what the list contains, and each item describes one thing:
          its name in backticks, then what it is. A paragraph or an example separates two lists, because
          two lists in a row read as one list.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example: 'SOKF_SCHEMA_90 [ubiquitous]: the schema SHALL conform to the tramlines-md grammar together with the SOKF schema extension.'
          - template: '`${}`: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example: '`Block kind`: `include`. An `include` block of a design is a block that sokf fills with a region of a file, such as a function of `match.c`.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows an example of a rule, and it adds no rule of its own.
        example: "unique: repository"
````
