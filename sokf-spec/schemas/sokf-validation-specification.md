---
type: Schema
id: schema-sokf-validation-specification
title: SOKF Validation Specification Schema
description: "This schema defines the one document that specifies validation in the Superdev Open Knowledge Format (SOKF). The document contains the rules, written as criteria in the EARS (Easy Approach to Requirements Syntax) form, and the fixed text that each document of this type repeats word for word."
---

<!-- prettier-ignore -->
````yaml
line-limit: 0

guidance: >
  The validation specification defines what a validator checks in a knowledge store, what
  it reports, and what its fix repairs. It is written for the builders of a validator and
  for the writers who read its findings. It is the only document of its type.

  Each normative rule is a criterion in the EARS form with a key `SOKF_VALIDATION_N`. A new
  criterion takes the next free number, and no later criterion reuses the number of a
  removed one. The numbering works this way for two reasons: a key identifies one rule
  permanently, and contracts and issues cite criteria by their keys.

  The subject of a criterion is the conforming thing, or the role bound by the rule. Each
  subject is a term of section 1 or of the specifications named in section 1.

  All other text is informative. The sections keep their numbers, because the specification
  cites its sections by number, such as §2, as the format specification does.

frontmatter:
  type:
    const: SokfValidationSpecification
  id:
    const: sokf-validation-spec
  title:
    const: "SOKF: Validation specification"
  description:
    guidance: The description says what the specification defines.
    example: "This specification defines the validation of a knowledge store: what a validator checks, what it reports, and what its fix repairs."
  version:
    template: '${:int}.${:int}'
    example: "0.6"
    guidance: The version is the SOKF version that the document defines.
  status:
    enum: [draft, stable]
    guidance: "The status is `draft` while the version can still change, and `stable` after the version is fixed."

types:
  sokf-rule-id: 'SOKF_VALIDATION_\d+'
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
    - document check
    - fix
    - failure
    - warning

formats:
  sokf-criterion:
    key: id
    variants:
      ubiquitous:
        template: '${id:sokf-rule-id} [ubiquitous]: the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_2 [ubiquitous]: the validator SHALL treat LF and CRLF as the same line ending in each comparison."
      event:
        template: '${id:sokf-rule-id} [event]: WHEN ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_14 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite each stale definition block and each stale include block, in dependency order."
      state:
        template: '${id:sokf-rule-id} [state]: WHILE ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_3 [state]: WHILE a document is in a path-enforced state, the document SHALL lie in the directory that its schema maps the state to."
      unwanted:
        template: '${id:sokf-rule-id} [unwanted]: IF ${}, THEN the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_4 [unwanted]: IF a link uses a custom relationship, THEN the validator SHALL report a warning."
      optional:
        template: '${id:sokf-rule-id} [optional]: WHERE ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_5 [optional]: WHERE a validator offers the fix, the validator SHALL run the fix before the document check."
      complex-event:
        template: '${id:sokf-rule-id} [complex]: WHILE ${}, WHEN ${}, the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_6 [complex]: WHILE a sidecar is in a path-enforced state, WHEN the validator fixes a tree, the validator SHALL move the sidecar together with its asset."
      complex-unwanted:
        template: '${id:sokf-rule-id} [complex]: WHILE ${}, IF ${}, THEN the ${:sokf-subject} SHALL ${}.'
        example: "SOKF_VALIDATION_7 [complex]: WHILE the validator fixes a tree, IF a move would overwrite a file, THEN the validator SHALL leave every file of that move in place."
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
      The opening says, in a fixed sentence, what the specification defines, and the
      paragraphs after that sentence say what SOKF is and how a validator checks a knowledge
      store.
    blocks:
      - kind: prose
        rules:
          - text: "This specification defines validation in the Superdev Open Knowledge Format (SOKF): what a validator checks, reports, and repairs."
            max: 1
          - pattern: '.'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - 'SOKF is a format for storing canonical knowledge, which agents and people both read and maintain.'
              - 'A validator reads every document of a knowledge store, such as the contract `contract-plumbing` and the design `design-plumber`, and reports each rule that a document breaks.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each section of the
          specification, in order, so that a reader sees the whole outline before reading any
          section. Each item links to its section's heading and keeps the section's number.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[About this specification](#about-this-specification)'
            - '[1. Terminology](#1-terminology)'
            - '[2. Document check](#2-document-check)'
            - '[3. Fix](#3-fix)'
  - heading: About this specification
    level: 2
    guidance: This section says, in fixed text, how the specification states its rules.
    blocks:
      - kind: prose
        max: 1
        each:
          text: "Each normative rule of this specification is a criterion in the EARS (Easy Approach to Requirements Syntax) form, and each has a key of the form `SOKF_VALIDATION_N`, where `N` is the number of the criterion."
  - heading: "1. Terminology"
    level: 2
    rule-id: section-1
    guidance: >
      This section defines each term that the specification adds to the terms of the format
      and schema specifications, one item for each term: the term's name in bold and a colon,
      then what it is. The type `sokf-subject` lists these terms and those of the two
      specifications, and the validator checks the subject of each criterion against it.
    blocks:
      - kind: prose
        max: 1
        each:
          text: "This specification uses the terms of the [format specification][sokf:sokf-format-spec] and the [schema specification][sokf:sokf-schema-spec], and adds these:"
      - kind: bullet-list
        each:
          template: '**${name}**: ${}'
          key: name
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - '**document check**: The deterministic check that the validator runs on every file in the working tree (§2).'
            - '**failure**: A finding that a knowledge store breaks a criterion, such as a label in `design-plumber` that resolves to no document.'
            - '**warning**: A finding that does not stop a knowledge store from conforming, such as a link from `design-plumber` with a custom relationship.'
  - heading: "2. Document check"
    level: 2
    rule-id: section-2
    guidance: >
      This section defines the document check: what the validator checks, and what each
      finding says. Its first criterion requires the validator to report a failure for each
      criterion of the format and schema specifications that a knowledge store breaks. The
      check therefore covers every rule of those specifications. The criteria after it say how
      the validator reports a finding, and what it checks beyond those rules.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A statement of what has to be true is a criterion in a list. Only a
          criterion has a key that the code and the tests cite.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'The validator runs one deterministic check, the document check, on the working tree, so it reports a broken link in `design-plumber` before the writer commits the design.'
            - 'The check also reads the markers in the plumber''s source files, such as `// sokf-implements: PLUMB-1_AC-2` in `match.c`, although those files lie outside every knowledge store.'
      - kind: bullet-list
        guidance: >
          The list contains the criteria of the document check, and SOKF_VALIDATION_1 comes
          first, because each criterion after it adds to that one.
        rules:
          - template: '${id:sokf-rule-id} [ubiquitous]: the validator SHALL report a failure for each criterion of the [format specification][sokf:sokf-format-spec] and the [schema specification][sokf:sokf-schema-spec] that the knowledge store breaks.'
            key: id
            unique: repository
            max: 1
            example: 'SOKF_VALIDATION_1 [ubiquitous]: the validator SHALL report a failure for each criterion of the [format specification][sokf:sokf-format-spec] and the [schema specification][sokf:sokf-schema-spec] that the knowledge store breaks.'
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_VALIDATION_45 [ubiquitous]: the validator SHALL report each finding with the path of its file, its line where one applies, and its severity.'
              - 'SOKF_VALIDATION_48 [ubiquitous]: the validator SHALL report each finding once.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a rule with an example, such as a finding, or a reference
          before and after the fix. The block is informative, because only a criterion states
          a rule.
        example: |
          /knowledge/designs/design-plumber.md:14: error: the label `sokf:contract-plumbin` resolves to no document (SOKF_FORMAT_61)
  - heading: "3. Fix"
    level: 2
    rule-id: section-3
    guidance: >
      This section defines the fix: the mechanical errors that the validator repairs in
      place, and the order of the repairs. A validator that offers the fix runs it before the
      document check, so that the check reports only what the fix cannot repair.
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        guidance: >
          Prose explains the rules of the section and gives their reasons, and it is
          informative. A statement of what has to be true is a criterion in a list. Only a
          criterion has a key that the code and the tests cite.
        each:
          pattern: '.'
          prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
          example:
            - 'After a writer changes the state of `design-plumber`, the fix moves the design to the directory of its new state. It then regenerates each link definition that points to the design''s former path.'
            - 'The fix leaves the path of a document in an include marker unchanged.'
      - kind: bullet-list
        optional: true
        repeat: true
        guidance: >
          A list is either a list of criteria or a named list, which takes the place of a
          table. A sentence before a named list says what the list contains, and each item
          names one thing in bold, followed by a colon, and then says what it is. A paragraph or a code block
          separates two lists, because two lists in a row read as one.
        rules:
          - format: sokf-criterion
            unique: repository
            min: 0
            example:
              - 'SOKF_VALIDATION_17 [event]: WHEN the validator fixes a tree, the validator SHALL move each document, keeping its filename, to the directory that its schema maps its current state to.'
              - 'SOKF_VALIDATION_37 [event]: WHEN the validator fixes a tree that its previous run already fixed, the validator SHALL rewrite no file.'
          - template: '**${}**: ${}'
            min: 0
            prohibited-patterns: [{pattern: '\b(SHALL|MUST|SHOULD|MAY|REQUIRED|RECOMMENDED|OPTIONAL|[Ss]hall|[Mm]ust|[Ss]hould|[Mm]ay)\b', message: 'The prose of a specification uses no requirement keyword, such as SHALL or must, because only a criterion states a requirement.'}]
            example:
              - '**Path link**: The fix rewrites an inline path link to a document, such as `[Plumbing](../contracts/contract-plumbing.md)`, as a label, such as `[Plumbing][sokf:contract-plumbing]`.'
              - '**Link target**: The fix replaces the `to` of a link with the `id` of its target, such as `contract-plumbing`, when the `to` is on a line of its own.'
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: >
          A code block illustrates a rule with an example, such as a finding, or a reference
          before and after the fix. The block is informative, because only a criterion states
          a rule.
        example: |
          before: [Plumbing](../contracts/contract-plumbing.md)
          after:  [Plumbing][sokf:contract-plumbing]
````
