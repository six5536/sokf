---
type: Schema
id: schema-grammar
title: Grammar schema
description: The usage guide that sits beside the grammar file of the schema extension of SOKF (Superdev Open Knowledge Format) and shows how to write text that the grammar accepts.
---

<!-- prettier-ignore -->
````yaml
line-limit: 0

asset-files: /sokf-spec/sokf.tramlines.yaml

guidance: >
  A grammar guide explains the grammar file next to it one construct at a time. A writer
  then writes text that the grammar accepts without first reading the grammar.
  The guide opens by saying who it is for and what the grammar governs, and a contents
  list follows. Each section then explains one construct of the grammar with an example
  of text that the construct accepts. A guide usually ends with two sections: the common
  mistakes, and a list of every keyword of the grammar. The grammar file is the authority,
  and where the guide and the grammar differ, the grammar is right.

frontmatter:
  type:
    const: Grammar
  version:
    template: '${:int}.${:int}'
    example: "0.4"
    guidance: The version is the value of `version` in the grammar file, so that a reader knows which version of the grammar the guide describes.

# A section body holds any of the following blocks, in any order and any
# number of times. Every fence has a language tag.
sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening's heading is the frontmatter title. The opening says who the guide is for,
      what the grammar governs, and which documents define what the grammar leaves out.
    blocks:
      - kind: prose
        example:
          - 'This guide is for anyone who writes plumbing rules, the rules that say where the plumber sends each message.'
          - 'The file `/mnt/plumb/rules` contains the rules in use, and a write to that file changes them at once. The rules are written in a pattern-action language. This guide explains that language one construct at a time. Its last section is a reference table of the guide''s verbs and their objects.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each section in order, so
          that a reader sees the whole guide before reading any section. Each item links to
          the anchor of its section, which is the section's number.
        each:
          template: '[${}](#${:int})'
          example:
            - '[1. Rule sets](#1)'
            - '[2. Objects and verbs](#2)'
            - '[3. Patterns](#3)'
            - '[4. Variables](#4)'
            - '[5. Actions](#5)'
            - '[6. Included rules](#6)'
            - '[7. Pitfalls](#7)'
            - '[8. Reference](#8)'

  - heading: { template: '${n:int}. ${}', key: n, example: "1. Rule sets" }
    level: 2
    repeat: true
    subheadings-open: true
    guidance: >
      Each section explains one construct of the grammar, with an example of text that the
      construct accepts. The heading starts with the section's number, because the number
      is the section's key: the guide cites a section as §5, and a link points to it with
      the anchor `#5`.
    examples:
      - |
        ## 2. Objects and verbs

        A rule has three words: an object, a verb, and an argument. The object identifies what the rule acts on, the verb says what the rule does with it, and the argument is the value that the verb uses. In `type is text`, the object is `type`, the verb is `is`, and the argument is `text`. The objects are these:

        - `src`, `dst`, `wdir`, `type`, `attr`, and `data` identify five of the six header lines and the data of the message.
        - `arg` identifies the rule's own argument, so a rule such as `arg isfile $1` tests the argument without changing the message.
        - `plumb` identifies the action of the rule set, as in `plumb to edit` (§5).
      - |
        ## 3. Patterns

        A pattern rule tests the message, and the rule set fails at the first pattern rule that fails. `is` compares the object with the argument exactly, and `matches` tests whether a regular expression matches the whole object:

        ```plumbing
        data matches '[a-zA-Z0-9_\-./]+'
        ```

        When the message has a `click` attribute, the plumber finds the longest text around the click position that the expression matches, and that text becomes the data of the message.
      - |
        ## 4. Variables

        A variable holds text that an earlier rule of the set matched or found, and a later rule of the set can use it. After a `matches` rule, `$0` holds the whole match, and `$1` to `$9` hold the parenthesised parts. After an `isfile` rule, `$file` holds the full path of the file.
      - |
        ## 5. Actions

        The plumber carries out an action rule only when every pattern rule of its set has succeeded. `plumb to edit` sends the message to the port `edit`, and each program that has opened the port receives one copy. When no program has opened the port, `plumb start` starts one, and `plumb client` starts one and holds the message until the new program opens the port.

        > A write to `send` returns before any program reads the message, because in a user interface a write that never waits matters more than delivery that is certain.
      - |
        ## 6. Included rules

        `include basic` inserts the rule sets of the file `/sys/lib/plumb/basic` in place of the line. A user's file usually begins with the user's own rule sets and ends with this line, so that the plumber tries them before the basic ones.
      - |
        ## 7. Pitfalls

        - A blank line inside a rule set ends the set, and the rules after it form a set of their own.
        - When a general rule set comes before a specific one, the plumber routes every message that both sets match by the general set. The plumber stops at the first matching set.
      - |
        ## 8. Reference

        The following table lists the verbs that this guide uses, with the objects that each verb takes:

        | Verb      | Objects         | Meaning                                             |
        | --------- | --------------- | --------------------------------------------------- |
        | `is`      | any but `plumb` | The object equals the argument.                     |
        | `matches` | any but `plumb` | The regular expression matches the whole object.    |
        | `isfile`  | `arg`           | The argument names a file.                          |
        | `set`     | any but `plumb` | The argument replaces the value of the object.      |
        | `add`     | `attr`          | The plumber adds the argument to the attributes.    |
        | `to`      | `plumb`         | The message goes to the port.                       |
        | `start`   | `plumb`         | The plumber runs the argument as a command.         |
        | `client`  | `plumb`         | The plumber runs the command and holds the message. |
    blocks-ordered: false
    blocks:
      - kind: prose
        optional: true
        repeat: true
        example:
          - 'A rule set is a group of rules on consecutive lines, and a blank line ends the set. The plumber tries the rule sets in the order of the file. The action rules of the first set whose patterns all succeed say where the message goes.'
      - kind: bullet-list
        optional: true
        repeat: true
        example:
          - 'A pattern rule, such as `type is text`, tests the message (§3).'
          - 'An action rule, such as `plumb to edit`, says where the message goes (§5).'
          - 'A line that begins with `#` is a comment, and the plumber ignores it.'
      - kind: numbered-list
        optional: true
        repeat: true
        example:
          - 'The plumber selects the next rule set of the file.'
          - 'The plumber applies each pattern rule of the set to the message, in order.'
          - 'When every pattern rule succeeds, the plumber carries out the action rules of the set and stops.'
      - kind: blockquote
        optional: true
        repeat: true
        example: 'When no rule set matches and the message names no port, the write to `/mnt/plumb/send` fails with the error `no matching plumb rule`.'
      - { kind: table, optional: true, repeat: true, columns: [], columns-open: true }
      - kind: code
        optional: true
        repeat: true
        lang: any
        guidance: A code block shows text that the grammar accepts, and its tag names the language of that text.
        example: |
          # a file name and a line number go to the editor
          type is text
          data matches '([a-zA-Z0-9_\-./]+):([0-9]+)'
          arg isfile $1
          data set $file
          attr add addr=$2
          plumb to edit

````
