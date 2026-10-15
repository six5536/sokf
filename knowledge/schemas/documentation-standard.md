---
type: Schema
id: schema-documentation-standard
title: Documentation standard schema
description: A documentation standard sets the rules for one kind of documentation, in sections by topic after an opening, a contents list, and an optional order of precedence.
---

<!-- prettier-ignore -->
````yaml
line-limit: 200
root: /knowledge/documentation-standards

guidance: >
  A documentation standard sets the rules for one kind of documentation. Every writer of
  that documentation, an agent most of all, reads it before writing. The standard opens with
  what it covers and lists its sections. An optional Precedence section follows, and then
  one section for each topic.

frontmatter:
  type:
    const: DocumentationStandard
  id:
    template: "documentation-standard-${:slug}"

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says what the standard helps its reader do and what it contains: the
      documentation that its rules cover, the writers who read it, and its topics. It names
      the standards that the rules add to. When the rules come from a published guide, the
      opening cites the guide in a footnote and states its licence, because the standard
      copies the guide's words. The frontmatter lists the guide in `sources`.
    blocks:
      - kind: prose
        example: 'This standard sets the rules for the manual pages of the plumber, `plumber(4)`, `plumb(1)`, `plumb(2)`, and `plumb(6)`, and for the comments in the default rules file `/sys/lib/plumb/basic`. A writer, a person or an agent, reads it before changing a page or a comment. Its four sections state the order of precedence, then the rules for files, rules, and messages, for diagnostics, and for bugs.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each level-2 section, in
          order, because a reader, an agent most of all, reads the start of a document first.
          Each item links to the heading of its section.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Precedence](#precedence)'
            - '[Files, rules, and messages](#files-rules-and-messages)'
            - '[Diagnostics](#diagnostics)'
            - '[Bugs](#bugs)'
  - heading: Precedence
    level: 2
    optional: true
    guidance: >
      This section lists the sources of style in order of precedence, with one source in each
      item, so that a writer knows which source to follow when two disagree. One sentence
      that ends with a colon introduces the list.
    blocks:
      - kind: prose
        max: 1
        each:
          pattern: '^[A-Z].*:$'
          example: 'When two sources of style disagree, follow the one that comes first:'
      - kind: numbered-list
        example:
          - This standard
          - The page `plumber(4)`
          - The other pages of the plumber, `plumb(1)`, `plumb(2)`, and `plumb(6)`
  - heading: { template: '${}', example: 'Files, rules, and messages' }
    level: 2
    repeat: true
    guidance: >
      Each topic section states the rules for one topic, one rule in each item. Each rule
      addresses the writer as _you_, in the imperative. A rule that chooses one form among
      several carries its reason. With the reason, a writer can apply the rule to new cases. A self-evident rule carries no reason. One sentence that ends with a colon
      introduces the list.
    examples:
      - |
        ## Diagnostics

        Follow these rules for the errors that a page lists:

        - Quote each error exactly as the program writes it, such as `no matching plumb rule`, because a reader searches the page for that text.
        - Say which call fails with each error: a write to `/mnt/plumb/send` fails with `no matching plumb rule` when no rule set matches and the message names no port.
      - |
        ## Bugs

        Follow these rules for the BUGS section of a page:

        - State each limit plainly, as a fact about the program: "The plumber gives a sender no way to know when a message has been read."
        - Give the reason for a limit that the design chose, because a reader might otherwise report the limit as a fault, for example: "The design accepts the preceding limit, because in a user interface a write that never waits matters more than certain delivery."
    blocks:
      - kind: prose
        max: 1
        each:
          pattern: '^[A-Z].*:$'
          example: 'Follow these rules when a page refers to a file, a rule, or a message:'
      - kind: bullet-list
        example:
          - Name each file by its full path, such as `/mnt/plumb/rules`, because _rules_ alone could mean any rules file.
          - 'Show each rule as its object, verb, and argument, such as `plumb to edit`, because a reader copies the rule into a rules file in that form.'
          - Show each message as its six header lines followed by its data, as a program writes it to `send`.

````
