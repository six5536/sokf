---
type: Schema
id: schema-recipe
title: Recipe schema
description: "A recipe explains how to do one task on the project: an opening that says what the task is and when to do it, a contents list, and one section for each step, in order."
---

<!-- prettier-ignore -->
````yaml
line-limit: 800
root: /knowledge/recipes

guidance: >
  A recipe explains how to do one task on the project, for a reader who is doing that
  task. It has an opening, a contents list, and one section for each step, in the order in
  which the reader takes the steps.

frontmatter:
  type:
    const: Recipe
  id:
    template: "recipe-${:slug}"

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says which task the recipe is for, when a reader uses the recipe, and
      which steps it covers.
    blocks:
      - kind: prose
        example: 'This recipe explains how to add a rule set that sends a PostScript file to the image viewer, and how to test the rule set while the plumber runs. A developer uses it when the plumber has to route a new kind of message. The recipe covers four steps: writing the rule set, loading the rules, testing the rule set, and restoring the old rules.'
      - kind: bullet-list
        guidance: >
          The contents list has one item for each level-2 section, in order. Each item links
          to the heading of its section.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Write the rule set](#write-the-rule-set)'
            - '[Load the rules](#load-the-rules)'
            - '[Test the rule set](#test-the-rule-set)'
            - '[Restore the old rules](#restore-the-old-rules)'
  - heading: { template: '${}', example: Write the rule set }
    level: 2
    rule-id: step
    repeat: true
    guidance: >
      Each section is one step, in the order in which the reader takes it. The heading is a
      bare verb phrase that names the step. A paragraph says what the step does, and a code
      block follows where the step runs a command or writes a file. A step can also contain
      a table of measurements, a list, or the instructions to give an agent. The paragraph
      says why the step matters only where the step has a cost or would otherwise seem
      arbitrary. A reader who knows that reason can adapt the step when the project
      changes.
    examples:
      - |
        ## Load the rules

        A write to `/mnt/plumb/rules` changes the rules in use at once, so the new rules take effect without a restart of the plumber. The shell opens the file with truncation, which clears the old rules, and the write then loads every rule set in `lib/plumbing`:

        ```rc
        cat $home/lib/plumbing > /mnt/plumb/rules
        ```

        A rule set with a syntax error makes the write fail with `malformed rule`.
      - |
        ## Test the rule set

        The plumb command sends a message from the shell. A message that names a PostScript file in the working directory tests the new rule set:

        ```rc
        cd /usr/rob/doc
        plumb plumb.ps
        ```

        The image viewer shows `plumb.ps`. When no rule set matches, the plumb command prints `no matching plumb rule`.
      - |
        ## Restore the old rules

        When the new rule set sends another kind of message to the image viewer, the developer removes the rule set from `lib/plumbing` and loads the rules again:

        ```rc
        cat $home/lib/plumbing > /mnt/plumb/rules
        ```
    blocks-ordered: false
    blocks:
      - kind: prose
        repeat: true
        example: 'The rule set goes in `lib/plumbing` in the home directory, before the rule set that sends text files to the editor. The plumber uses the first rule set whose patterns all succeed, so a later rule set never receives a PostScript file:'
      - kind: code
        lang: any
        optional: true
        repeat: true
        example: |
          type is text
          data matches '[a-zA-Z0-9_\-./]+'
          data matches '([a-zA-Z0-9_\-./]+)\.(ps|eps)'
          arg isfile $0
          plumb to image
          plumb client page -wi
      - kind: bullet-list
        optional: true
        repeat: true
      - kind: numbered-list
        optional: true
        repeat: true
        each:
          blocks:
            - kind: code
              lang: any
              optional: true
      - kind: table
        optional: true
        repeat: true
        columns-open: true
        columns: []
      - kind: blockquote
        optional: true
        repeat: true

  - heading: { template: '${}', example: Write the rule for PostScript files }
    level: 3
    parent: step
    optional: true
    repeat: true
    guidance: >
      A long step has a subsection for each of its parts, in the order in which the reader
      takes them. Each heading is a bare verb phrase.
    blocks-ordered: false
    blocks:
      - kind: prose
        repeat: true
      - kind: code
        lang: any
        optional: true
        repeat: true
      - kind: bullet-list
        optional: true
        repeat: true
      - kind: numbered-list
        optional: true
        repeat: true
        each:
          blocks:
            - kind: code
              lang: any
              optional: true
      - kind: table
        optional: true
        repeat: true
        columns-open: true
        columns: []
      - kind: blockquote
        optional: true
        repeat: true

````
