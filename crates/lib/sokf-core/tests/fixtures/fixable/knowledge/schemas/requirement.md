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
  - heading: User stories
    level: 2
    rule-id: stories
    guidance: One story per capability, each with its rationale.
    blocks:
      - kind: group
        repeat: true
        blocks:
          - { kind: prose, max: 1, each: { format: user-story, unique: repository } }
          - { kind: blockquote, max: 1, example: A reader needs the rationale to weigh the story. }
  - heading: Acceptance criteria
    level: 2
    guidance: EARS criteria, each keyed to the story it verifies.
    blocks:
      - kind: bullet-list
        each: { format: requirement-ears, unique: repository, key-derives-from: { rule: stories, separator: "_" }, example: "ST-1_AC-1 [event]: WHEN a reader clicks a file name, the editor SHALL open the file." }
````
