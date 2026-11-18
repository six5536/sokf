---
type: Schema
id: schema-task
title: Task Schema
description: A task, laid out by state.
---

````yaml
line-limit: 0
root: /knowledge/tasks
guidance: A task records one piece of work for one person.
prohibited-patterns:
  - pattern: '\bTODO\b'
    message: The text holds no TODO, because the task itself records what remains.
lifecycle:
  - key: state
    states: [open, done]
    path: true
    guidance: A task is open while its work remains.
frontmatter:
  type:
    const: Task
  id:
    template: "task-${:seq}-${:slug}"
  tags:
    list: true
    optional: true
    max: { soft: 3, when-at-limit: 'Keep the tags that a reader searches for.' }
    example: kitchen
  owner:
    optional: true
    only: { state: [open] }
    guidance: The owner is the person who does the work.
    example: ada
links:
  - rel: depends-on
    to-type: Task
    min: 0
    max: { hard: 2, when-at-limit: 'Merge the tasks that this one waits for.' }
    guidance: A depends-on link points to a task that has to finish first.
sections:
  - heading: { from: title }
    level: 1
    guidance: The opening says why the work matters.
    blocks:
      - kind: prose
        guidance: The summary says what the work changes.
        max: { soft: 1, when-at-limit: 'Put the details in the steps.' }
        example: The kettle needs descaling before it breaks.
  - heading: Steps
    level: 2
    rule-id: steps
    only: { state: [open] }
    guidance: This section lists the steps in order.
    blocks:
      - kind: numbered-list
        guidance: Each step holds one action.
        each:
          pattern: '^[A-Z]'
          prohibited-patterns: [{pattern: '\bthen\b', message: 'A step holds no "then", because each step is one action.'}]
          example: Buy vinegar.
  - heading: Notes
    level: 3
    parent: steps
    optional: true
    only: { state: [open] }
    guidance: This section holds what a later step needs to know.
    blocks:
      - kind: prose
        example: The vinegar is under the sink.
  - heading: Outcome
    level: 2
    only: { state: [done] }
    guidance: This section says what the work delivered.
    blocks:
      - kind: prose
        example: The kettle boils in half the time.
  - heading: Follow-up
    level: 2
    optional: true
    only: { state: [done] }
    guidance: This section names the work that the task leaves.
    blocks:
      - kind: bullet-list
        max: { hard: 3, when-at-limit: 'File a task for the rest.' }
        example: Descale the other kettle.
````
