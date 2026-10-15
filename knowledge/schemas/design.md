---
type: Schema
id: schema-design
title: Design schema
description: "How one building block is built, for a developer who changes its code: the ideas it rests on, its modules, how they work together, the interfaces between them, its data, the rules across its modules, its limits, the rules that a change keeps, and the decisions behind it."
---

<!-- prettier-ignore -->
````yaml
line-limit: 2000
root: /knowledge/designs
# A `Planned:` paragraph promises work that an open issue owns, so it links no
# closed issue.
current-links: ['^Planned: ']

guidance: >
  A design describes how one building block is built, so that a developer can change its
  code safely. It covers one level of the block. It states the ideas that the block rests
  on, then describes the block's modules and how they work together. It then describes the
  interfaces between the modules, the block's data, the rules across its modules, its
  limits, and the rules that a change keeps. The decisions behind the block come last. The architecture describes
  how the block connects to the other blocks, and the contract states what the block
  promises, so the design cites both rather than restating them. Where the design presents
  existing code, it includes the code rather than copying it, because an include changes
  with the code.

frontmatter:
  type:
    const: Design
  id:
    template: "design-${:slug}"
  code:
    template: '${:requirement-code}'
    guidance: >
      The code is 1 to 10 capital letters, and it begins every key in the design, such as
      PLMB-1 and PLMB_P-1. It differs from the code of the contract, so that the design's
      keys never collide with the contract's keys.
    example: 'PLMB'
  resource:
    list: true
    min: 1
    guidance: >
      The resource lists the paths of the block's code, one directory or file for each
      module, so that `sokf graph` finds the design from any of those paths. A path that
      an open issue's Files section links counts before the file exists.
    example: '/sys/src/cmd/plumb'
  description:
    guidance: The description says how the block does its work, in one phrase that begins with _How_.
    example: 'How the plumber matches each message against the rules and delivers it to a port.'

# The architecture overview lists every design in its Designs section.
cited-by:
  - from-type: Architecture
links:
  - rel: implements
    to-type: Contract
    min: 0
    guidance: The design has one implements link to each contract that the block implements, and none when the block implements no contract.

types:
  module-name: '[A-Z][^*`:]*|`[^`]+`'
  topic: '[A-Z][^*`:]*|`[^`]+`'
  labelled-text: '[A-Z`].*\.'

formats:
  decision:
    template: '**${:topic}**: ${}. Why: ${}.'
    example: '**File server**: The plumber is a file server, and programs send and receive messages by writing and reading its files. Why: a program needs no new mechanism to use the plumber.'
  planned:
    pattern: '^Planned: [A-Z`].*\[[^\]]+\]\[sokf:[^\]]*issue-[^\]]+\].*\.$'
    example: 'Planned: `matchruleset` also returns the number of the failed pattern, as [the issue on the failed pattern][sokf:issue-001-feature-report-failed-pattern] describes.'

prohibited-patterns:
  - pattern: '\*\*[^*]+\*\*([^:]|$)'
    message: 'The body uses bold only for a label that a colon ends.'

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says what the design helps its reader do and what it contains. It then says
      what the block does, who uses it, and what its contract promises. Last, it says what the
      block doesn't do and which block does that instead.
    blocks:
      - kind: prose
        sequence:
          - pattern: '^[A-Z].*\.$'
            example: 'This design helps a developer change the plumber without breaking it. It describes the ideas that the plumber rests on, its modules, how they handle a message, their interfaces, its data, its limits, and the decisions behind it.'
          - pattern: '^[A-Z].*\[[^\]]+\]\[sokf:([a-z0-9-]+:)?architecture#[a-z0-9_-]+\].*\.$'
            example: 'The [plumber][sokf:architecture#plumber] passes messages between interactive programs. A program writes a message to `/mnt/plumb/send`, and the plumber delivers it to the port that the first matching rule set names, such as `edit` for the text editor. The plumber implements the [Plumbing][sokf:contract-plumbing] contract. The contract promises each program that has opened a port one copy of each message sent to that port. It also promises that a write to `send` fails when no rule set matches and the message names no port.'
          - pattern: '^[A-Z`].*\.$'
            example: 'The plumber doesn''t interpret the data that it passes, beyond what the rules say, and it doesn''t confirm that a message was read. Each program decides what to do with a message that it reads. The sender of a message fills in the message''s working directory.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each section of the design,
          in order. Each item links to the section's heading.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Approach](#approach)'
            - '[Structure](#structure)'
            - '[Runtime behaviour](#runtime-behaviour)'
            - '[Interfaces](#interfaces)'
            - '[Data](#data)'
            - '[Cross-cutting concerns](#cross-cutting-concerns)'
            - '[Limits](#limits)'
            - '[Changing the plumber](#changing-the-plumber)'
            - '[Decisions](#decisions)'

  - heading: Approach
    level: 2
    guidance: >
      This section states the two or three ideas that the block rests on, in one short
      paragraph, or in a sentence and a list. The Decisions section gives the advantages of
      each idea and the reasons for it.
    blocks:
      - kind: prose
        max: 1
        each:
          pattern: '^[A-Z`].*[.:]$'
          example: 'The plumber rests on three ideas:'
      - kind: bullet-list
        optional: true
        max: 3
        guidance: Each item states one idea in one sentence.
        each:
          pattern: '^[A-Z`].*\.$'
          example:
            - 'The plumber is a file server, and programs pass messages by writing and reading its files.'
            - 'Rules kept in one file decide where every message goes, and a user changes them while the plumber runs.'
            - 'A write to `send` never waits for the message to be read.'

  - heading: Structure
    level: 2
    guidance: >
      This section describes the modules that make up the block, one paragraph for each,
      and not the smaller pieces inside them. A module whose description needs more than one
      paragraph has a design of its own.
    blocks:
      - kind: prose
        guidance: >
          The first paragraph says where the block's code is and which type or function other
          blocks call. Each module then has one paragraph, which begins with the module's name
          in bold as its label. A descriptive name is plain bold, and a name that is an
          identifier in the code is in code font as well. The sentence after the label has
          the module as its subject. The paragraph says what the module is responsible for and
          what only that module does. Where it matters, the paragraph also says how the module
          behaves under load or failure. The paragraph ends with a link to the module's code,
          which can be a file that an open issue's Files section plans.
        rules:
          - pattern: '^[A-Z].*\.$'
            min: 1
            max: 1
            prohibited-patterns: [{pattern: '^`', message: 'The opening paragraph begins with a word, not with code.'}, {pattern: ' Code: \[', message: 'The opening paragraph links no module''s code, because each module''s paragraph links its own.'}]
            example: 'The plumber has four modules, in [`/sys/src/cmd/plumb`](/sys/src/cmd/plumb). Programs reach the plumber only through the files that the file server serves in `/mnt/plumb`, never by calling its functions.'
          - template: '**${name:module-name}**: ${:labelled-text} Code: [${}](/${}).'
            key: name
            min: 1
            example:
              - '**Main module**: The main module reads the user''s rules from `lib/plumbing` in the home directory, creates each port that a rule set names, and starts the file server. It prints `parse error` and exits when the rules have a syntax error. Code: [`plumber.c`](/sys/src/cmd/plumb/plumber.c).'
              - '**Rules reader**: The rules reader parses the text of the rules into a list of `Ruleset`s, and it is the only module that understands the syntax of the rules. It turns the name of each verb into the verb''s number. Code: [`rules.c`](/sys/src/cmd/plumb/rules.c).'
              - '**Matcher**: The matcher tests whether a message matches a rule set, and rewrites the message as the rule set says. When the message has a `click` attribute, the matcher narrows the data to the text that the patterns match around the click position. Code: [`match.c`](/sys/src/cmd/plumb/match.c).'
              - '**File server**: The file server serves the files in `/mnt/plumb` over 9P, the Plan 9 file protocol, and collects each message that a program writes to `send`. It keeps a queue of messages for each port, and starts the program that a `plumb start` or `plumb client` action names. Code: [`fsys.c`](/sys/src/cmd/plumb/fsys.c).'
      - kind: code
        optional: true
        guidance: >
          A diagram of the modules comes last. The design leaves it out when the architecture
          already has a diagram of them. The diagram has at most nine nodes, because a larger
          one shows too much for one level.
        lang: mermaid
        each:
          example: |
            %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
            flowchart LR
            accTitle: The modules of the plumber
              subgraph canvas[" "]
                subgraph plumber["Plumber<br/><i>[Block]</i>"]
                  main["Main module<br/><i>C</i>"]:::block
                  reader["Rules reader<br/><i>C</i>"]:::block
                  matcher["Matcher<br/><i>C</i>"]:::block
                  fsys["File server<br/><i>C</i>"]:::block
                  rules[("Rule sets<br/><i>in memory</i>")]:::store
                end
                main -->|"Calls<br/><i>readrules</i>"| reader
                main -->|"Starts<br/><i>file server</i>"| fsys
                fsys -->|"Passes<br/><i>rules text</i>"| reader
                reader -->|"Writes<br/><i>rule sets</i>"| rules
                fsys -->|"Calls<br/><i>matchruleset</i>"| matcher
                matcher -->|"Reads<br/><i>rule sets</i>"| rules
              end
              style plumber fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
              classDef user fill:#ffffff,stroke:#666666,color:#1a1a1a
              classDef block fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
              classDef neighbour fill:#eaf2fb,stroke:#a9c4e0,color:#1a1a1a
              classDef external fill:#eeeeee,stroke:#999999,color:#1a1a1a
              classDef store fill:#ffe6cc,stroke:#d79b00,color:#1a1a1a
              classDef contract fill:#e1d5e7,stroke:#9673a6,color:#1a1a1a
              classDef group fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
          lines:
            - format: theme
              min: 1
              max: 1
            - format: acc-title
              min: 1
              max: 1
            - format: acc-descr
              min: 0
              max: 1
            - format: flowchart-type
              min: 1
              max: 1
            - format: node
              min: 2
              max: 9
              prohibited-patterns:
                - pattern: ':::(user|neighbour|external|contract|group)$'
                  message: A node of the parts diagram is a block or a store of this design, so it takes neither the user, neighbour, external, contract, nor group class, although the diagram declares each class.
            - format: arrow
              min: 1
              max: 9
            - format: class-style
              min: 7
              max: 7
            - select-pattern: '^classDef user '
              min: 1
              max: 1
            - select-pattern: '^classDef block '
              min: 1
              max: 1
            - select-pattern: '^classDef neighbour '
              min: 1
              max: 1
            - select-pattern: '^classDef external '
              min: 1
              max: 1
            - select-pattern: '^classDef store '
              min: 1
              max: 1
            - select-pattern: '^classDef contract '
              min: 1
              max: 1
            - select-pattern: '^classDef group '
              min: 1
              max: 1
            - format: canvas
              min: 1
              max: 1
            - format: boundary
              min: 1
              max: 1
            - format: boundary-style
              min: 1
              max: 1
            - format: ground-end
              min: 2
              max: 2

  - heading: Runtime behaviour
    level: 2
    optional: true
    rule-id: runtime
    guidance: >
      This section explains how the modules work together to carry out the block's main
      tasks. When a task spans more than one building block, the section cites the
      architecture's flow rather than repeating its diagram.
    blocks: []

  - heading: { template: '${}', example: Delivering a message }
    level: 3
    repeat: true
    parent: runtime
    guidance: >
      Each subsection follows one task through the block, from the request that starts it
      to its result, and says which module does each step.
    examples:
      - |
        ### Replacing the rules

        A user changes the rules by writing to `/mnt/plumb/rules`, for example with `cp /usr/$user/lib/plumbing /mnt/plumb/rules`, and the plumber matches the next message against the new rules.

        A write to the rules file passes through the plumber in three steps:

        1. The file server clears the rules in use when a program opens the rules file with truncation.
        2. The file server passes the text of each write to the rules reader, which parses every complete rule set in the text written so far. The rules reader returns a syntax error, such as `malformed rule`, as the error of the write.
        3. The rules reader parses the last rule set when the program closes the file, and the file server creates each port that a rule set names.
    blocks:
      - kind: prose
        guidance: >
          A paragraph says what starts the task and what it produces. A sentence that ends
          with a colon introduces the steps.
        rules:
          - pattern: '^[A-Z`].*\.$'
            min: 0
            example:
              - 'A program such as the editor or the `plumb` command writes a message to `/mnt/plumb/send`, and the write returns when the plumber has queued the message or rejected it.'
          - pattern: '^[A-Z`].*:$'
            min: 0
            max: 1
            example:
              - 'A message follows one path through the plumber:'
      - kind: numbered-list
        optional: true
        guidance: Each step says which module acts and what the module does. Where the reason for a step isn't obvious, the step gives it.
        each:
          pattern: '^[A-Z`].*\.$'
          example:
            - 'The file server collects the bytes of the write into a `Plumbmsg`, and joins a message that arrives in several writes. A write that can''t be part of a message fails with the error for a bad message.'
            - 'The file server calls `matchruleset` for each rule set in order, and stops at the first set whose patterns all succeed.'
            - 'The matcher rewrites the message as the matching rule set says. It sets the destination port and adds attributes such as `addr=27`. When the message has a `click` attribute, the matcher replaces the data with the text that the patterns matched around the click position.'
            - 'The file server queues the message on the port when a program has opened the port. Otherwise it calls `startup` to start a program, and for a `plumb client` action it holds the message until the new program opens the port.'
            - 'The write to `send` returns. When no rule set matched and the message named no port, the write fails with `no matching plumb rule`.'
      - kind: code
        optional: true
        guidance: >
          A sequence diagram or an activity flowchart can follow the steps, and the
          task's name is the diagram's `accTitle`.
        lang: mermaid
        each:
          example: |
            %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
            sequenceDiagram
            accTitle: Delivering a message
              participant sender as Sending program
              box rgb(246, 248, 250)
              participant fsys as File server
              participant match as Matcher
              end
              participant editor as Editor
              sender->>fsys: write(send, message)
              fsys->>match: matchruleset(message, rule set)
              match-->>fsys: Exec, or nil
              alt a rule set matches
              fsys->>fsys: queue the message, or start a program
              fsys-->>editor: read(edit) returns the message
              end
              fsys-->>sender: the write returns, or fails
          lines-open: true
          lines:
            - format: theme
              min: 1
              max: 1
            - format: acc-title
              min: 1
              max: 1
            - format: acc-descr
              min: 0
              max: 1
            - select-pattern: '^(sequenceDiagram|flowchart TD)$'
              min: 1
              max: 1
            - format: participant
              min: 0
              max: 9
            - format: sequence-message
              min: 0
              max: 15
            - select-pattern: '.'
              min: 3
              max: 30

  - heading: Interfaces
    level: 2
    optional: true
    rule-id: interfaces
    guidance: >
      This section has one subsection for each interface that one module of the block offers
      and another module relies on. The contract defines the interface that the block offers
      its users, so the design cites the contract's criteria by their keys rather than
      restating them.
    blocks: []

  - heading:
      template: '${id:requirement-id} ${:part-name}'
      key: id
      unique: knowledge
      key-derives-from: { from: code, separator: '-' }
      example: PLMB-1 `matchruleset`
    level: 3
    repeat: true
    parent: interfaces
    rule-id: interface-parts
    guidance: >
      Each subsection describes one interface. Its heading is the interface's name in the
      code, in backticks, or a noun phrase that describes the interface.
    examples:
      - |
        ### PLMB-2 `readrules`

        `readrules` parses the text of the rules into a list of `Ruleset`s, and returns the first syntax error, or nil.

        <!-- sokf:include /sys/src/cmd/plumb/rules.c#readrules -->
        <!-- /sokf:include -->

        - PLMB-2_AC-1 [unwanted]: IF the text has a syntax error, THEN the rules reader SHALL return the error, such as `malformed rule`, as the error of the write.
    blocks:
      - kind: prose
        max: 1
        guidance: One sentence, with the interface as its subject, says what the interface does.
        each:
          format: requirement-summary
          example: '`matchruleset` matches a message against one rule set, and returns an `Exec` with the matched text and the changes to the message, or nil when a pattern fails.'
      - kind: include
        optional: true
        repeat: true
        source-pattern: '^/'
        guidance: >
          An include shows the code of the interface. Before the code exists, the include
          names a file that an open issue's Files section plans and holds a sketch of the
          code, and the fix replaces the sketch with the code after the file exists.
        example: |
          <!-- sokf:include /sys/src/cmd/plumb/match.c#matchruleset -->
          <!-- /sokf:include -->
      - kind: group
        optional: true
        repeat: true
        guidance: >
          A `Planned:` paragraph describes a change to the interface that isn't built yet.
          The paragraph links the open issue that owns the change. A sketch after it shows
          only that addition. When the code exists, the include shows it, and the paragraph
          and the sketch are removed.
        blocks:
          - kind: prose
            max: 1
            each:
              format: planned
              example: 'Planned: `matchruleset` takes a new argument, through which it returns the number of the failed pattern, as [the issue on the failed pattern][sokf:issue-001-feature-report-failed-pattern] describes.'
          - kind: code
            lang: any
            example: |
              Exec* matchruleset(Plumbmsg *m, Ruleset *rs, int *failed);
      - kind: bullet-list
        optional: true
        guidance: >
          The list states the promises that another module relies on and the code doesn't
          show. Each promise is a criterion in EARS (Easy Approach to Requirements Syntax), or
          a property that holds for all inputs.
        rules:
          - format: requirement-ears
            unique: knowledge
            key-derives-from: { rule: interface-parts, separator: '_' }
            min: 0
            example:
              - 'PLMB-1_AC-1 [unwanted]: IF a pattern of the rule set fails, THEN the matcher SHALL return nil and leave the message unchanged.'
              - 'PLMB-1_AC-2 [optional]: WHERE the message has a `click` attribute, the matcher SHALL return nil unless every pattern matches the same text around the click position.'
          - format: requirement-property
            unique: knowledge
            key-derives-from: { from: code, separator: '_' }
            min: 0
            example:
              - 'PLMB_P-1 [idempotent]: FOR ANY message and rule set, the matcher SHALL return the same result each time while the rules and the files that the patterns test are unchanged.'

  - heading: Data
    level: 2
    optional: true
    guidance: >
      This section describes the data that the block works with, and the states of each
      piece of data. An include shows each definition from the code. A diagram
      shows what an include can't: how the pieces of data relate to one another, and how
      the data moves from one state to the next.
    blocks:
      - kind: prose
        optional: true
        guidance: >
          The paragraphs before the definitions explain what the code doesn't show, such as
          why a structure has a field or what a state means for the rest of the block. A
          sentence that ends with a colon introduces the definitions.
        rules:
          - pattern: '^[A-Z`].*\.$'
            min: 1
            prohibited-patterns: [{pattern: '^Planned: ', message: 'A caption describes code that exists, not a planned change.'}]
            example: 'A `Ruleset` keeps its patterns and its actions in two lists. It keeps the port that its `plumb to` action names in a field of its own, so the plumber creates every port before any message arrives.'
          - pattern: '^[A-Z`].*:$'
            min: 1
            max: 1
            example:
              - 'The matcher reads two structures and builds a third:'
      - kind: include
        optional: true
        repeat: true
        source-pattern: '^/'
        example: |
          <!-- sokf:include /sys/src/cmd/plumb/plumber.h#rules -->
          <!-- /sokf:include -->
      - kind: group
        optional: true
        repeat: true
        guidance: >
          A one-sentence caption comes before a class, entity-relationship, or state diagram.
          In a class diagram, each box contains only the class's name, because the include
          shows the class's fields and methods. The diagram has at most 24 lines, because a
          larger one shows too much for one diagram.
        blocks:
          - kind: prose
            max: 1
            each:
              format: caption
              prohibited-patterns: [{pattern: '^Planned: ', message: 'A caption describes code that exists, not a planned change.'}]
              example: 'A `Ruleset` contains one `Rule` for each pattern and each action, and `matchruleset` builds one `Exec` for each `Plumbmsg` that it matches.'
          - kind: code
            lang: mermaid
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                classDiagram
                accTitle: The structures of the plumber
                  namespace plumber {
                    class Ruleset
                    class Rule
                    class Exec
                    class Plumbmsg
                  }
                  Ruleset *-- Rule : patterns and actions
                  Exec --> Plumbmsg : rewrites
                  style Ruleset fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
                  style Rule fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
                  style Exec fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
                  style Plumbmsg fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
              lines-open: true
              lines:
                - format: theme
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - select-pattern: '^(classDiagram|erDiagram|stateDiagram-v2)$'
                  min: 1
                  max: 1
                - format: class-namespace
                  min: 0
                  max: 1
                - format: class-declare
                  rule-id: data-classes
                  min: 0
                  key: name
                - format: class-box-style
                  min: 0
                  key: name
                  for-each-key-of: data-classes
                - select-pattern: '^class \w+ *\{'
                  min: 0
                  max: 0
                - select-pattern: '^\w+ : '
                  min: 0
                  max: 0
                - select-pattern: '.'
                  min: 3
                  max: 24
      - kind: group
        optional: true
        repeat: true
        guidance: >
          A `Planned:` paragraph describes a change to the data that isn't built yet. The
          paragraph links the open issue that owns the change. A sketch after it shows only
          that addition. When the code exists, the include shows it, and the paragraph and the
          sketch are removed.
        blocks:
          - kind: prose
            max: 1
            each:
              format: planned
              example: 'Planned: A message can carry the attribute `explain=yes`, which asks the plumber to name the failed pattern in its error, as [the issue on the failed pattern][sokf:issue-001-feature-report-failed-pattern] describes.'
          - kind: code
            lang: any
            example: |
              addr=27 explain=yes

  - heading: Cross-cutting concerns
    level: 2
    optional: true
    rule-id: concerns
    guidance: This section explains the rules that affect more than one module of the block.
    blocks: []

  - heading: { template: '${}', example: Errors }
    level: 3
    repeat: true
    parent: concerns
    guidance: Each subsection covers one concern, such as errors, concurrency, caching, or persistence.
    examples:
      - |
        ### Ports

        The rule sets decide which ports exist. At startup, the main module creates each port that a rule set names. Closing the rules file creates each port that a new rule set names. The file server keeps a queue of messages for each reader of a port, and discards a reader's queue when the reader exits. When a program opens a port, it receives only the messages that arrive from then on.
      - |
        ### Order of the rule sets

        The order of the rule sets in the rules file decides where a message goes. The first set whose patterns all succeed decides the destination, so a rule set takes a message from every set after it. Two modules keep the rule sets in that order:

        - The rules reader keeps the rule sets in the order of the rules file.
        - The file server passes the rule sets to the matcher in the same order.

        `cat /mnt/plumb/rules` prints the rules in use, including the rule set that sends a C header file to the editor:

        ```text
        type is text
        data matches '([a-zA-Z0-9]+\.h)(:([0-9]+))?'
        arg isfile /sys/include/$1
        data set /sys/include/$1
        attr add addr=$3
        plumb to edit
        ```
    blocks:
      - kind: prose
        example:
          - 'The plumber reports every error as the error string of a 9P request. A write to `send` fails with `no matching plumb rule` when no rule set matches and the message names no port. A write to `rules` with a syntax error fails with the rules reader''s error, such as `malformed rule`.'
          - 'A syntax error in the rules at startup is different: the main module prints `parse error`, and the plumber exits.'
          - 'A failed write leaves no message behind:'
      - kind: bullet-list
        optional: true
        example:
          - 'A write of a malformed message fails, and the file server drops what it had collected for that message.'
          - 'A write of a message that matches no rule set fails, and the file server queues the message on no port.'
      - kind: group
        optional: true
        repeat: true
        guidance: >
          A worked example shows what prose can't, such as the output of a command on a test
          fixture. One sentence before the example says what runs, and ends with a colon. A
          test can read the example from the design and compare it with the real output.
        blocks:
          - kind: prose
            max: 1
            each:
              pattern: '^[A-Z`].*:$'
              example: 'The test `nomatch.rc` sends a message that matches no rule set, and checks that the write fails with this error:'
          - kind: code
            lang: any
            example: |
              no matching plumb rule

  - heading: Limits
    level: 2
    optional: true
    guidance: >
      This section states plainly what the block doesn't do well, how large the effect is,
      and what bounds the effect, so that a developer doesn't mistake a known limit for a
      bug.
    blocks:
      - kind: prose
        each:
          pattern: '^[A-Z`].*\.$'
          example:
            - 'The plumber can''t tell a sender when a message has been read.'
            - 'A message is lost when its reader exits with messages still in the queue. The user notices the failure immediately, because the program that should show the message isn''t there.'
            - 'The rules match a file by its name, not by its contents. An image file without a `.jpg` suffix therefore goes to the editor rather than the image viewer.'
            - 'A write to `rules` reports no error in its last rule set unless the text ends with a blank line. The rules reader parses that rule set only when the file is closed, after the write has returned.'

  - heading: { template: 'Changing the ${}', example: Changing the plumber }
    level: 2
    optional: true
    guidance: >
      This section lists the rules that a change to the block has to keep. Neither the
      compiler nor the tests enforce them. Each item states its rule and the reason for it.
    blocks:
      - kind: prose
        max: 1
        each:
          pattern: '^[A-Z`].*:$'
          example: 'A change that breaks one of these rules causes no compiler error and no failing test:'
      - kind: bullet-list
        each:
          pattern: '^[A-Z`].*\.$'
          example:
            - 'A new verb needs an entry in the list of verbs in `plumber.h`, a name in the parser in `rules.c`, and a case in `match.c`, because nothing checks that the three agree.'
            - 'A change to the six header lines needs a matching change to `libplumb` and to every program that sends or reads messages, because the header is part of the plumbing contract.'
            - 'A change to `plumbmsg.schema.json` goes into the `Plumbmsg` type, because `jsonschema.rc` generates the file from that type and loses a hand edit at the next run.'

  - heading: Decisions
    level: 2
    guidance: >
      This section explains the main decisions behind the block, and the reason for each.
      Each idea of the Approach section has a decision here, with its advantages. The section
      includes a decision that affects more than one file and would be costly to undo, and a
      decision made over a real alternative. A comment in a file explains a decision that
      affects only that file.
    blocks:
      - kind: bullet-list
        guidance: >
          Each item begins with its topic in bold as its label, then states the decision, and
          gives the reason after `Why:`. When a real alternative was rejected, the item ends
          with `Rejected:` and that alternative, then `Why not:` and the reason. An item states
          the decision without citing the plan that proposed it, and changes when the decision
          changes. `sokf validate` warns when a design has more than 15 decisions, because one
          of the modules of such a block belongs in a design of its own.
        rules:
          - format: decision
            min: 1
            max: 50
            severity: warn
            example:
              - '**File server**: The plumber is a file server, and programs send and receive messages by writing and reading its files. Why: a program needs no new mechanism to use the plumber.'
              - '**Rules in one file**: Rules kept in one file decide where every message goes, and the plumber serves that file as `/mnt/plumb/rules`. Why: a user changes the rules while the plumber runs, and the next message follows the new rules.'
              - '**Writes never wait**: A write to `send` returns without waiting for the message to be read. Why: in a user interface, a write that never waits matters more than certain delivery. Rejected: a write that returns only after a program reads the message. Why not: the sender would wait as long as its slowest reader.'
````
