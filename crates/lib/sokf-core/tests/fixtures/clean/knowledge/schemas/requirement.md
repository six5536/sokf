---
type: Schema
id: schema-requirement
title: Requirement Schema
description: User stories with EARS criteria.
---

````yaml
line-limit: 0
frontmatter:
  type:
    const: Requirement
sections:
  - heading: { from: title }
    level: 1
    blocks: []
  - heading: User Stories
    level: 2
    rule-id: stories
    guidance: One story per capability, each with its rationale.
    blocks:
      - kind: group
        repeat: true
        blocks:
          - { kind: prose, max: { hard: 1, when-at-limit: "Write a second need as a group of its own." }, each: { format: user-story, unique: repository } }
          - { kind: blockquote, max: { hard: 1, when-at-limit: "Put the rest of the rationale in the story." }, example: A reader needs the rationale to weigh the story. }
  - heading: Acceptance Criteria
    level: 2
    guidance: EARS criteria, each keyed to the story it verifies.
    blocks:
      - kind: bullet-list
        each: { format: requirement-ears, unique: repository, key-derives-from: { rule: stories, separator: "_" }, example: "ST-1_AC-1 [event]: WHEN a reader clicks a file name, the editor SHALL open the file." }

types:
  requirement-id: '[A-Z]{1,10}-\d+(?:\.\d+)?'
  criterion-id: '[A-Z]{1,10}-\d+_AC-\d+'
  indefinite-article: [A, AN]

formats:
  user-story:
    key: id
    template: "${id:requirement-id}: AS ${:indefinite-article} ${}, I WANT ${}, SO THAT ${}."
    prohibited-patterns:
      [
        {
          pattern: '\b(SHALL|SHOULD|MUST)\b',
          message: "A user story states a need, so it holds no SHALL, SHOULD, or MUST.",
        },
      ]
    example: "ST-1: AS A user, I WANT a click on a file name to open the file in the editor, SO THAT I go from a compiler error to the code in one click."
  requirement-ears:
    key: id
    variants:
      ubiquitous:
        template: "${id:criterion-id} [ubiquitous]: the ${} SHALL ${}."
        example: "PLUMB-4_AC-1 [ubiquitous]: the plumber SHALL queue a message on an open port within 1 ms of the write to `send`."
      event:
        template: "${id:criterion-id} [event]: WHEN ${}, the ${} SHALL ${}."
        example: "PLUMB-1_AC-1 [event]: WHEN a program writes a message to `/mnt/plumb/send`, the plumber SHALL deliver the message to the port that the first matching rule set names."
      state:
        template: "${id:criterion-id} [state]: WHILE ${}, the ${} SHALL ${}."
        example: "PLUMB-2_AC-2 [state]: WHILE a program's queue for the port is empty, the plumber SHALL delay its reply to the program's read until a message arrives."
      unwanted:
        template: "${id:criterion-id} [unwanted]: IF ${}, THEN the ${} SHALL ${}."
        example: "PLUMB-1_AC-2 [unwanted]: IF no rule set matches and the message names no port, THEN the plumber SHALL fail the write with the error `no matching plumb rule`."
      optional:
        template: "${id:criterion-id} [optional]: WHERE ${}, the ${} SHALL ${}."
        example: "PLUMB-1_AC-4 [optional]: WHERE the message has a `click` attribute, the plumber SHALL replace the data with the text that the matching rule set's patterns match around the click position."
      complex-event:
        template: "${id:criterion-id} [complex]: WHILE ${}, WHEN ${}, the ${} SHALL ${}."
        example: "PLUMB-1_AC-3 [complex]: WHILE no program has the destination port open, WHEN a message for that port arrives, the plumber SHALL start the program that the rule set's `plumb client` action names."
      complex-unwanted:
        template: "${id:criterion-id} [complex]: WHILE ${}, IF ${}, THEN the ${} SHALL ${}."
        example: "PLUMB-3_AC-4 [complex]: WHILE a program has `/mnt/plumb/rules` open for writing, IF a second program opens it for writing, THEN the plumber SHALL fail the open with the error `file already open`."
    prohibited-patterns:
      - pattern: '\b([Ss]hould|[Mm]ust|[Ww]ill|[Mm]ay|[Mm]ight|[Cc]an)\b'
        message: "The line uses no weak modal verb, such as should, must, will, may, might, or can."
      - pattern: '\bSHALL\b.*\bSHALL\b'
        message: "The line holds one SHALL."
      - pattern: '\b([Ff]ast|[Qq]uickly|[Ee]asy|[Ee]asily|[Uu]ser-friendly|[Aa]ppropriate|[Aa]s needed)\b'
        message: "The line uses no vague word, such as fast, easy, appropriate, or as needed."
      - pattern: '\betc\.'
        message: "The line ends no list with etc."
      - pattern: "and/or"
        message: "The line uses no and/or."
      - pattern: '\bSHALL (NOT )?be (\w+ed|\w+[^p\W]en|\w+wn|sent|kept|built|left|lost|made|read|set|put|held|told|found|sold|run|done|shut|meant|spent|paid|laid|said|brought|bought|caught|taught|thought)\b'
        message: "A criterion states its response in the active voice."
````
