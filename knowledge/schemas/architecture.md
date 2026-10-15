---
type: Schema
id: schema-architecture
title: Architecture schema
description: An architecture introduces a system and shows, in a hierarchy of diagrams, its context, its subsystems and their parts, its contracts, its flows, where it runs, and its technology.
---

<!-- prettier-ignore -->
````yaml
line-limit: 1500

guidance: >
  An architecture introduces one system to a reader who has never seen it. It shows the
  building blocks of the system and how they connect, in a hierarchy of diagrams: the
  system, its subsystems, and the parts of each subsystem. Each block's design describes
  how the block is built inside. Each diagram has at most nine nodes, because a larger one
  shows too much for one level.

frontmatter:
  type:
    const: Architecture
  id:
    const: architecture
  description:
    guidance: The description says what the system is, what it does, and what it provides to its users.
    example: 'The plumbing system passes text between the interactive programs of Plan 9. A file server, the plumber, delivers each message to the program that handles it, according to the user''s rules.'

# A story cites each flow, because a flow that no user needs is waste.
cited-by:
  - from-type: Stories
    keys: flows
types:
  link-text: '[^\]]+'
  link-name: '[A-Z][^\]]*'
  path: '[^) ]+'
  block-name: '[A-Z][^:"<]*'
  purpose: '[a-z][^"<]*'
  article: [a, an]
  part-path: '[A-Z][^:"<]*: [A-Z][^:"<]*'

formats:
  group-boundary:
    template: 'subgraph ${id:word}["${:phrase}<br/><i>[Group]</i>"]'
    example: '    subgraph fsys["File server<br/><i>[Group]</i>"]'
  legend-node:
    prohibited-patterns:
      - pattern: '^(graph|subgraph|end|class|classDef|style|linkStyle|click|call|href|default|direction)[\[({]'
        message: 'A node''s id is no Mermaid keyword.'
    variants:
      user:
        template: '${:word}(["${:label}"]):::user'
        example: '    user(["User"]):::user'
      block:
        template: '${:word}["${:label}"]:::block'
        example: '    block["Block"]:::block'
      neighbour:
        template: '${:word}["${:label}"]:::neighbour'
        example: '    neighbour["Neighbour"]:::neighbour'
      external:
        template: '${:word}["${:label}"]:::external'
        example: '    external["External system"]:::external'
      store:
        template: '${:word}[("${:label}")]:::store'
        example: '    store[("Data store")]:::store'
      contract:
        template: '${:word}{{"${:label}"}}:::contract'
        example: '    contract{{"Contract"}}:::contract'
      group:
        template: '${:word}[["${:label}"]]:::group'
        example: '    group[["Group"]]:::group'
  provides:
    template: '${from:word} -.- ${to:word}'
    example: '    fsys -.- plumbing'
  nav-link-subsystem:
    template: '[Subsystem: ${:link-text}][sokf:${:store}architecture#subsystem-${:slug}]'
    example: '[Subsystem: Plumber][sokf:architecture#subsystem-plumber]'
  nav-link-group:
    template: '[Group: ${:link-text}][sokf:${:store}architecture#group-${:slug}]'
    example: '[Group: File server][sokf:architecture#group-file-server]'
  design-link:
    template: '[${:link-name}][sokf:${:store}design-${:slug}] (${block:block-name}): ${:purpose}.'
    example: '[Plumber][sokf:design-plumber] (Plumber): how the plumber reads the rules, matches each message, and delivers it to a port.'
  part-design-link:
    template: '[${:link-name}][sokf:${:store}design-${:slug}] (${part:part-path}): ${:purpose}.'
    example: '[Rules reader][sokf:design-rules-reader] (File server: Rules reader): how the file server parses a rules file and reports the line of each error.'
  contract-link:
    template: '[${name:link-name}][sokf:${:store}contract-${:slug}] (${:block-name}): ${:purpose}.'
    example: '[Plumbing][sokf:contract-plumbing] (Plumber): the files in `/mnt/plumb`, the format of a message, and what a program that sends or reads a message can rely on.'

sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        max: 1
        guidance: >
          The opening says what the architecture helps its reader do and what it contains.
        each:
          pattern: '^[A-Z].*\.$'
          example: 'This architecture helps a developer find the part of the plumbing system that a change touches. It shows the system in its context, its three subsystems and their parts, its contracts, two flows, where it runs, and the technology it depends on.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each level-2 section of
          the architecture, in order. Each item links to the section's heading.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Overview](#overview)'
            - '[Context](#context)'
            - '[Subsystems](#subsystems)'
            - '[Contracts](#contracts)'
            - '[Designs](#designs)'
            - '[Flows](#flows)'
            - '[Deployment](#deployment)'
            - '[Technology](#technology)'
            - '[Legend](#legend)'

  - heading: Overview
    level: 2
    guidance: >
      The overview introduces the system. It says what the system provides, how it
      provides it, and what it connects to. Its last sentence links to the non-goals. The
      Subsystems section says what the system is built from, and the Technology section
      says what it builds on.
    blocks:
      - kind: prose
        max: 4
        example:
          - 'The plumbing system passes text between the interactive programs of Plan 9. A program writes a message to `/mnt/plumb/send`, and the plumber delivers it to the port that the user''s rules choose. A program can send text without knowing which program handles it.'
          - 'The plumbing system connects the window system, the editor, the image viewer, and the mail file system. It reads each user''s rules from the Plan 9 file server.'
          - 'The [non-goals][sokf:non-goals] say what the plumbing system doesn''t do, and why.'

  - heading: Context
    level: 2
    guidance: >
      This section shows who uses the system at run time and which external systems it
      depends on or serves. The Deployment section shows how the system is delivered and
      hosted.
    blocks:
      - kind: group
        max: 1
        guidance: The section has one caption and one diagram of the system in its context.
        blocks:
          - kind: prose
            guidance: >
              This paragraph is the caption of the diagram that follows it, and serves as the
              diagram's title, because a reader reads it first. It states the diagram's main
              point: how the system relates to the people and systems around it.
            max: 1
            each:
              format: caption
              example: A user clicks on text in an interactive program, and the plumbing system delivers the text to the program that handles it. The plumbing system reads each user's rules from the Plan 9 file server.
          - kind: code
            guidance: >
              The diagram shows only what a reader needs at this level. The people whom the
              system serves appear as users. An artificial intelligence (AI) agent or other
              software that acts for them appears as an external system. In a crowded
              diagram, one user represents each kind of person, and one external system
              represents each kind of system, so that the context fits in one diagram.
            lang: mermaid
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                flowchart LR
                accTitle: The plumbing system in its context
                  subgraph canvas[" "]
                    user(["User<br/><i>a person</i>"]):::user
                    plumbing["Plumbing system<br/><i>C</i>"]:::block
                    programs["Interactive programs<br/><i>editor, image viewer, window system, mail file system</i>"]:::external
                    fileserver["Plan 9 file server<br/><i>a separate machine</i>"]:::external
                    user -->|"Clicks on<br/><i>text</i>"| programs
                    programs -->|"Send<br/><i>messages</i>"| plumbing
                    plumbing -->|"Delivers<br/><i>messages</i>"| programs
                    plumbing -->|"Reads over 9P<br/><i>rules</i>"| fileserver
                  end
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
                - format: flowchart-type
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - format: node
                  min: 2
                  max: 9
                - select-pattern: ':::block$'
                  min: 1
                  max: 7
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
                - format: ground-end
                  min: 1
                  max: 1

  - heading: Subsystems
    level: 2
    guidance: >
      This section shows the subsystems that make up the system, the external systems they
      connect to, and the stores they use. Its diagram shows no contracts, because each
      subsystem's diagram shows the contracts of that subsystem.
    blocks:
      - kind: group
        max: 1
        guidance: The section has one caption and one diagram, which shows every subsystem.
        blocks:
          - kind: prose
            guidance: >
              This paragraph is the caption of the diagram that follows it, and serves as the
              diagram's title, because a reader reads it first. It says how the system is
              divided and what each subsystem is responsible for.
            max: 1
            each:
              format: caption
              example: The plumbing system consists of the plumber, the plumbing library `libplumb`, and the plumb command. The window system, the mail file system, and the plumb command send messages through the library. The image viewer reads messages through it, and the editor does both. The library reaches the plumber over 9P, the Plan 9 file protocol, and the plumber reads the user's rules from `lib/plumbing`.
          - kind: code
            guidance: >
              The diagram shows only what a reader needs at this level. A crowded diagram
              means that the system is divided too finely, and the system then needs fewer,
              larger subsystems, whose parts appear in their own sections.
            lang: mermaid
            rule-id: subsystems
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                flowchart LR
                accTitle: The subsystems of the plumbing system
                  subgraph canvas[" "]
                    subgraph system["Plumbing system<br/><i>[System]</i>"]
                      plumber["Plumber<br/><i>/sys/src/cmd/plumb</i>"]:::block
                      libplumb["Plumbing library<br/><i>/sys/src/libplumb</i>"]:::block
                      plumbcmd["Plumb command<br/><i>/sys/src/cmd/plumb/plumb.c</i>"]:::block
                    end
                    rio["Window system<br/><i>rio</i>"]:::external
                    editor["Editor<br/><i>acme</i>"]:::external
                    viewer["Image viewer<br/><i>page</i>"]:::external
                    mailfs["Mail file system<br/><i>upas/fs</i>"]:::external
                    rules[("Plumbing rules<br/><i>lib/plumbing on the file server</i>")]:::store
                    rio -->|"Sends through<br/><i>message</i>"| libplumb
                    editor -->|"Sends and reads through<br/><i>messages</i>"| libplumb
                    viewer -->|"Reads through<br/><i>message</i>"| libplumb
                    mailfs -->|"Sends through<br/><i>message</i>"| libplumb
                    plumbcmd -->|"Sends through<br/><i>message</i>"| libplumb
                    libplumb -->|"Writes and reads over 9P<br/><i>messages</i>"| plumber
                    plumber -->|"Reads over 9P<br/><i>rules</i>"| rules
                  end
                  style system fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
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
                - format: flowchart-type
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - format: node
                  min: 2
                  max: 9
                - select-pattern: ':::block$'
                  min: 1
                  max: 7
                  pattern: '^\w+\["(?P<name>[^"<]+)'
                  key: name
                  example: '      plumber["Plumber<br/><i>/sys/src/cmd/plumb</i>"]:::block'
                - select-pattern: ':::contract$'
                  min: 0
                  max: 0
                - format: arrow
                  min: 1
                  max: 9
                - select-pattern: ' -\.- '
                  min: 0
                  max: 0
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
      - kind: prose
        max: 1
        each:
          text: 'Each subsystem has a section of its own, which shows its parts:'
      - kind: bullet-list
        guidance: The list links to the section of each subsystem, which shows the subsystem's parts.
        each:
          format: nav-link-subsystem
          example:
            - '[Subsystem: Plumber][sokf:architecture#subsystem-plumber]'
            - '[Subsystem: Plumbing library][sokf:architecture#subsystem-plumbing-library]'
            - '[Subsystem: Plumb command][sokf:architecture#subsystem-plumb-command]'

  - heading: { template: 'Subsystem: ${name}', key: name, example: "Subsystem: Plumber" }
    level: 3
    repeat: true
    for-each-key-of: subsystems
    rule-id: subsystem
    guidance: >
      Each subsystem section shows the parts of one subsystem and how a request moves
      between them. Each part appears once in the diagram, as a block. A part of another
      subsystem appears as a neighbour when the subsystem's parts call it. The diagram connects each
      contract of the subsystem to the part that provides it. A contract that the subsystem
      only uses appears in its provider's diagram.
    examples:
      - |
        ### Subsystem: Plumbing library

        The editor opens a port with the port opener, and sends and reads messages through the sender and the receiver. The sender and the receiver call the codec, which converts a `Plumbmsg` to and from text. The port opener, the sender, and the receiver reach the plumber's file server over 9P.

        ```mermaid
        %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
        flowchart LR
        accTitle: The parts of the plumbing library
          subgraph canvas[" "]
            subgraph libplumb["Plumbing library<br/><i>[Subsystem]</i>"]
              opener["Port opener<br/><i>plumbopen</i>"]:::block
              sender["Sender<br/><i>plumbsend</i>"]:::block
              receiver["Receiver<br/><i>plumbrecv</i>"]:::block
              codec["Codec<br/><i>plumbpack and plumbunpack</i>"]:::block
            end
            editor["Editor<br/><i>acme</i>"]:::external
            fsys["File server<br/><i>plumber</i>"]:::neighbour
            editor -->|"Calls<br/><i>plumbopen</i>"| opener
            editor -->|"Calls<br/><i>plumbsend</i>"| sender
            editor -->|"Calls<br/><i>plumbrecv</i>"| receiver
            sender -->|"Calls<br/><i>plumbpack</i>"| codec
            receiver -->|"Calls<br/><i>plumbunpack</i>"| codec
            opener -->|"Opens over 9P<br/><i>port</i>"| fsys
            sender -->|"Writes over 9P<br/><i>message</i>"| fsys
            receiver -->|"Reads over 9P<br/><i>message</i>"| fsys
          end
          style libplumb fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
          classDef user fill:#ffffff,stroke:#666666,color:#1a1a1a
          classDef block fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
          classDef neighbour fill:#eaf2fb,stroke:#a9c4e0,color:#1a1a1a
          classDef external fill:#eeeeee,stroke:#999999,color:#1a1a1a
          classDef store fill:#ffe6cc,stroke:#d79b00,color:#1a1a1a
          classDef contract fill:#e1d5e7,stroke:#9673a6,color:#1a1a1a
          classDef group fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
        ```
      - |
        ### Subsystem: Plumb command

        The message builder sets the fields of a message from the command's arguments, and calls the input reader when the `-i` option takes the data from standard input. The message builder then sends the message through the sender of the plumbing library.

        ```mermaid
        %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
        flowchart LR
        accTitle: The parts of the plumb command
          subgraph canvas[" "]
            subgraph plumbcmd["Plumb command<br/><i>[Subsystem]</i>"]
              builder["Message builder<br/><i>main</i>"]:::block
              input["Input reader<br/><i>gather</i>"]:::block
            end
            shell["Shell<br/><i>rc</i>"]:::external
            sender["Sender<br/><i>plumbing library</i>"]:::neighbour
            shell -->|"Runs<br/><i>plumb</i>"| builder
            builder -->|"Calls<br/><i>gather</i>"| input
            builder -->|"Calls<br/><i>plumbsend</i>"| sender
          end
          style plumbcmd fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
          classDef user fill:#ffffff,stroke:#666666,color:#1a1a1a
          classDef block fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
          classDef neighbour fill:#eaf2fb,stroke:#a9c4e0,color:#1a1a1a
          classDef external fill:#eeeeee,stroke:#999999,color:#1a1a1a
          classDef store fill:#ffe6cc,stroke:#d79b00,color:#1a1a1a
          classDef contract fill:#e1d5e7,stroke:#9673a6,color:#1a1a1a
          classDef group fill:#dae8fc,stroke:#6c8ebf,color:#1a1a1a
        ```
    blocks:
      - kind: group
        max: 1
        guidance: >
          The section has one caption and one diagram. When a subsystem has too many parts
          for one diagram, the diagram draws some of them together as one group, and a
          later Group section shows what is inside each group.
        blocks:
          - kind: prose
            guidance: >
              The first paragraph is the caption of the diagram that follows it, and serves
              as the diagram's title, because a reader reads it first. It says how the parts
              of the subsystem work together. A `Planned:` paragraph can follow the caption.
              It describes a part that isn't built yet, and links the open issue that owns
              the change.
            sequence:
              - format: caption
                example: The file server collects each message that a program writes, and calls the matcher. The matcher tests the message against the rule sets in memory, which the rules reader parsed at startup. The file server then queues the message on the destination port.
              - pattern: '^Planned: [A-Z].*\[[^\]]+\]\[sokf:([a-z0-9-]+:)?issue-[0-9]+-[a-z0-9-]+\].*\.$'
                optional: true
                example: 'Planned: The matcher reports which pattern failed when a message with the attribute `explain=yes` matches no rule set, as [the issue on the failed pattern][sokf:issue-001-feature-report-failed-pattern] describes.'
          - kind: code
            guidance: >
              The diagram shows only what a reader needs at this level. It shows each
              group with the group's inputs and outputs, so that each level fits in
              one diagram.
            lang: mermaid
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                flowchart LR
                accTitle: The parts of the plumber
                  subgraph canvas[" "]
                    subgraph plumber["Plumber<br/><i>[Subsystem]</i>"]
                      startup["Startup<br/><i>plumber.c</i>"]:::block
                      reader["Rules reader<br/><i>rules.c</i>"]:::block
                      matcher["Matcher<br/><i>match.c</i>"]:::block
                      fsys[["File server<br/><i>fsys.c</i>"]]:::group
                      rules[("Rule sets<br/><i>in memory</i>")]:::store
                    end
                    editor["Editor<br/><i>acme</i>"]:::external
                    plumbing{{"Plumbing<br/><i>public</i>"}}:::contract
                    editor -->|"Writes<br/><i>message</i>"| fsys
                    startup -->|"Calls<br/><i>readrules</i>"| reader
                    startup -->|"Starts<br/><i>file server</i>"| fsys
                    reader -->|"Writes<br/><i>rule sets</i>"| rules
                    fsys -->|"Calls<br/><i>matchruleset</i>"| matcher
                    matcher -->|"Reads<br/><i>rule sets</i>"| rules
                    fsys -.- plumbing
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
                - format: flowchart-type
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - format: node
                  min: 2
                  max: 9
                - select-pattern: ':::(block|group)$'
                  min: 1
                  max: 7
                - select-pattern: ':::block$'
                  rule-id: parts
                  min: 0
                  pattern: '^\w+\["(?P<name>[^"<]+)<br/>'
                  key: name
                  example: '      startup["Startup<br/><i>plumber.c</i>"]:::block'
                - select-pattern: ':::group$'
                  rule-id: groups
                  min: 0
                  pattern: '^\w+\[\["(?P<name>[^"<]+)<br/>'
                  key: name
                  example: '      fsys[["File server<br/><i>fsys.c</i>"]]:::group'
                - select-pattern: ':::contract$'
                  rule-id: contracts
                  min: 0
                  pattern: '\{\{"(?P<name>[^"<]+)<br/>'
                  key: name
                  example: '    plumbing{{"Plumbing<br/><i>public</i>"}}:::contract'
                - format: arrow
                  min: 1
                  max: 9
                - format: provides
                  min: 0
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
      - kind: group
        optional: true
        max: 1
        blocks:
          - kind: prose
            max: 1
            each:
              text: 'Each group in the diagram has a Group section of its own:'
          - kind: bullet-list
            guidance: The list links to the Group section of each group in the subsystem's diagram.
            each:
              format: nav-link-group

  - heading: { template: 'Group: ${name}', key: name, example: "Group: File server" }
    level: 4
    repeat: true
    for-each-key-of: groups
    parent: subsystem
    guidance: >
      Each Group section shows what is inside one group of the parent subsystem, and
      is named after the group. A store appears inside the group when only the
      group's parts use it. A store that other parts also use appears in the
      subsystem's diagram.
    blocks:
      - kind: group
        max: 1
        guidance: >
          The section has one caption and one diagram. When a group has too many parts
          for one diagram, the subsystem's diagram shows some of those parts outside the
          group.
        blocks:
          - kind: prose
            guidance: >
              The first paragraph is the caption of the diagram that follows it, and serves
              as the diagram's title, because a reader reads it first. It says what each part
              inside the group does. A `Planned:` paragraph can follow the caption. It
              describes a part that isn't built yet, and links the open issue that owns the
              change.
            sequence:
              - format: caption
                example: The request handler answers each 9P request and passes each complete message to the dispatcher. The dispatcher queues the message on the destination port, and the request handler takes the message from the queue when a program reads the port.
              - pattern: '^Planned: [A-Z].*\[[^\]]+\]\[sokf:([a-z0-9-]+:)?issue-[0-9]+-[a-z0-9-]+\].*\.$'
                optional: true
                example: 'Planned: The request handler returns an error that names the failed pattern to the program that writes to `send`, as [the issue on the failed pattern][sokf:issue-001-feature-report-failed-pattern] describes.'
          - kind: code
            guidance: >
              The diagram shows only the group's contents, inside a boundary named after
              the group, because the subsystem's diagram already shows the group's
              inputs and outputs.
            lang: mermaid
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                flowchart LR
                accTitle: The parts of the file server
                  subgraph canvas[" "]
                    subgraph fsys["File server<br/><i>[Group]</i>"]
                      handler["Request handler<br/><i>fsys.c</i>"]:::block
                      dispatcher["Dispatcher<br/><i>dispose</i>"]:::block
                      queues[("Port queues<br/><i>in memory</i>")]:::store
                    end
                    handler -->|"Passes<br/><i>message</i>"| dispatcher
                    dispatcher -->|"Appends<br/><i>message</i>"| queues
                    handler -->|"Takes<br/><i>message for a read</i>"| queues
                  end
                  style fsys fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
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
                - format: flowchart-type
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - format: node
                  min: 2
                  max: 9
                - select-pattern: ':::block$'
                  rule-id: group-parts
                  min: 1
                  max: 7
                  pattern: '^\w+\["(?P<name>[^"<]+)<br/>'
                  key: name
                  example: '      handler["Request handler<br/><i>fsys.c</i>"]:::block'
                - select-pattern: ':::(user|neighbour|external|contract|group)$'
                  min: 0
                  max: 0
                - format: arrow
                  min: 1
                  max: 9
                - select-pattern: ' -\.- '
                  min: 0
                  max: 0
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
                - format: group-boundary
                  min: 1
                  max: 1
                - format: boundary-style
                  min: 1
                  max: 1
                - format: ground-end
                  min: 2
                  max: 2

  - heading: Contracts
    level: 2
    guidance: >
      This section lists each contract in the diagrams of the subsystems, with the
      building block that provides it. Each contract appears once, and no other section of
      the architecture lists contracts.
    blocks:
      - kind: bullet-list
        guidance: >
          Each item links to the Contract document, names in parentheses the block or the
          group that the diagram attaches the contract to, and says what the contract
          defines, so that a reader knows where the link leads before following it. The
          Designs section lists a design for each block named here.
        rules:
          - format: contract-link
            key: name
            example: '[Plumbing][sokf:contract-plumbing] (Plumber): the files in `/mnt/plumb`, the format of a message, and what a program that sends or reads a message can rely on.'
            for-each-key-of: contracts
          - select-pattern: '\] \('
            pattern: '\] \((?P<block>[^)]+)\): '
            key: block
            key-repeats: true
            rule-id: providers
            min: 1
            example: '[Plumbing][sokf:contract-plumbing] (Plumber): the files in `/mnt/plumb`, the format of a message, and what a program that sends or reads a message can rely on.'

  - heading: Designs
    level: 2
    guidance: >
      This section lists the design of each subsystem, of each group, and of each block
      that provides a contract, because those are the blocks whose inside a reader needs
      explained. A part of a group can have a design of its own when the group's design
      would otherwise grow too long.
    blocks:
      - kind: bullet-list
        guidance: >
          Each item links to the Design document, names in parentheses the subsystem, the
          group, or the block that the design covers, and says what the design explains. A
          part's design names its group, a colon, and the part.
        rules:
          - format: design-link
            key: block
            for-each-key-of: [subsystems, groups, providers]
            example: '[Plumber][sokf:design-plumber] (Plumber): how the plumber reads the rules, matches each message, and delivers it to a port.'
          - format: part-design-link
            key: part
            key-derives-from: { rule: groups, separator: ': ' }
            min: 0
            example: '[Rules reader][sokf:design-rules-reader] (File server: Rules reader): how the file server parses a rules file and reports the line of each error.'

  - heading: Flows
    level: 2
    guidance: >
      This section shows how the subsystems work together in each important request, with
      one subsection for each request.
    blocks: []
  - heading: { template: 'Flow: ${name}', key: name, example: "Flow: Open a file from an error message" }
    level: 3
    repeat: true
    rule-id: flows
    guidance: >
      Each flow follows one important request, from the first call to the response. A
      story realises each flow and cites it by its key, because a flow that no user needs
      is waste.
    examples:
      - |
        ### Flow: Show an image

        The user clicks on `fig1.jpg` in the editor when no image viewer is running. The plumber starts the image viewer with a `plumb client` action, and holds the message until the image viewer opens the `image` port.

        ```mermaid
        %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
        sequenceDiagram
        accTitle: Showing an image
          box rgb(246, 248, 250)
          participant editor as Editor
          participant plumber as Plumber
          participant viewer as Image viewer
          end
          editor->>plumber: write(send, message)
          plumber->>viewer: start page
          plumber-->>editor: the write returns
          viewer->>plumber: open(image)
          plumber-->>viewer: read(image) returns the message
        ```
    blocks:
      - kind: group
        guidance: Each diagram in a flow has its own caption before it.
        blocks:
          - kind: prose
            guidance: >
              This paragraph is the caption of the diagram that follows it, and serves as the
              diagram's title, because a reader reads it first. It says what happens in the
              request, step by step.
            max: 1
            each:
              format: caption
              example: The user clicks on `mem.c:27` in a compiler's error message. The window system sends the text to the plumber, the plumber matches it against the rules and delivers it to the `edit` port, and the editor shows line 27 of `mem.c`.
          - kind: code
            lang: mermaid
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                sequenceDiagram
                accTitle: Opening a file from an error message
                  box rgb(246, 248, 250)
                  participant rio as Window system
                  participant plumber as Plumber
                  participant editor as Editor
                  end
                  editor->>plumber: read(edit), which waits
                  rio->>plumber: write(send, message)
                  plumber->>plumber: matchruleset(message, rule set)
                  plumber-->>editor: read(edit) returns the message
                  plumber-->>rio: the write returns
              lines:
                - format: theme
                  min: 1
                  max: 1
                - format: sequence-type
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - format: sequence-message
                  min: 1
                  max: 15
                - format: box
                  min: 1
                  max: 1
                - format: participant
                  min: 2
                  max: 9
                - format: ground-end
                  min: 1
                  max: 1

  - heading: Deployment
    level: 2
    guidance: >
      This section shows where each part of the system runs and which hosted services the
      system relies on.
    blocks:
      - kind: group
        guidance: >
          Each environment that the system runs in, such as test and production, has one
          caption and one diagram, and the caption names the environment.
        blocks:
          - kind: prose
            guidance: >
              This paragraph is the caption of the diagram that follows it, and serves as the
              diagram's title, because a reader reads it first. It says where the system runs
              and what it relies on there.
            max: 1
            each:
              format: caption
              example: On each user's terminal, one plumber runs beside the window system, the editor, the image viewer, and the mail file system. The plumber reads the rules from the file server, and the mail file system reads the mailbox from it.
          - kind: code
            guidance: >
              The diagram shows one environment, and only what a reader needs at this level.
              The parts appear again in each environment's diagram, because each environment
              runs them.
            lang: mermaid
            each:
              example: |
                %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
                flowchart LR
                accTitle: Where the plumbing system runs
                  subgraph canvas[" "]
                    subgraph terminal["Terminal<br/><i>[Host]</i>"]
                      plumber["Plumber<br/><i>one process for each user</i>"]:::block
                      rio["Window system<br/><i>rio</i>"]:::external
                      editor["Editor<br/><i>acme</i>"]:::external
                      viewer["Image viewer<br/><i>page</i>"]:::external
                      mailfs["Mail file system<br/><i>upas/fs</i>"]:::external
                    end
                    subgraph server["File server<br/><i>[Host]</i>"]
                      files[("User files<br/><i>lib/plumbing and mail</i>")]:::store
                    end
                    plumber -->|"Reads over 9P<br/><i>rules</i>"| files
                    mailfs -->|"Reads over 9P<br/><i>mailbox</i>"| files
                  end
                  style terminal fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
                  style server fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
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
                - format: flowchart-type
                  min: 1
                  max: 1
                - format: acc-title
                  min: 1
                  max: 1
                - format: acc-descr
                  min: 0
                  max: 1
                - format: node
                  min: 2
                  max: 9
                - select-pattern: ':::block$'
                  min: 1
                  max: 7
                - format: arrow
                  min: 1
                  max: 9
                - format: provides
                  min: 0
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
                - format: boundary-style
                  min: 1
                - format: ground-end
                  min: 2
                  max: 8

  - heading: Technology
    level: 2
    guidance: >
      This section lists the technologies that shape the system, such as its languages,
      runtimes, stores, and libraries. The build manifests record the versions and the
      other libraries.
    blocks: []
  - heading: { template: 'Technology: ${:block-name}', example: "Technology: Plumber" }
    level: 3
    repeat: true
    guidance: >
      Each subsection lists the technology choices for one subsystem, in the order of the
      Subsystems section. The subsection _Technology: Build_ lists the choices that belong
      to no single subsystem. The heading begins with _Technology_, so that its anchor
      differs from the anchor of the subsystem's section.
    examples:
      - |
        ### Technology: Build

        - **mk**: The `mk` command builds the plumber from the `mkfile` in its directory. Why: `mk` builds every Plan 9 program, so the plumber needs no build tool of its own.
    blocks:
      - kind: bullet-list
        guidance: >
          Each item begins with the name of the choice as its label. A sentence with the
          choice as its subject says what the choice does, and the reason for the choice
          follows `Why:`. Where a design describes the choice in depth, the item links to the
          design.
        each:
          template: '**${}**: ${}. Why: ${}.'
          example:
            - '**C**: C is the language of the plumber and of the programs that send it messages. Why: the plumber shares its types, such as `Plumbmsg`, with those programs through `libplumb`.'
            - '**9P**: 9P, the Plan 9 file protocol, carries every read and write of the files in `/mnt/plumb`. Why: every Plan 9 program can already read and write files, on its own machine or across the network ([the design of the plumber][sokf:design-plumber]).'
            - '**regexp**: The `regexp` library matches the patterns of the rules. Why: it finds the longest leftmost match, which the `click` attribute needs.'

  - heading: Legend
    level: 2
    guidance: >
      The legend is the same in every architecture, so that each class of node has one
      shape and one colour in every diagram. Its one sentence is the same in every
      architecture too.
    blocks:
      - kind: prose
        max: 1
        each:
          text: Every diagram in this document uses these shapes and colours.
      - kind: code
        lang: mermaid
        each:
          example: |
            %%{init: {"theme": "neutral", "themeVariables": {"darkMode": false, "background": "#f6f8fa", "clusterBkg": "#f6f8fa", "clusterBorder": "#d0d0d0", "edgeLabelBackground": "#f6f8fa", "lineColor": "#555555", "textColor": "#1a1a1a", "primaryTextColor": "#1a1a1a", "actorBkg": "#ffffff", "actorBorder": "#6c8ebf", "actorLineColor": "#767676", "actorTextColor": "#1a1a1a", "signalColor": "#555555", "signalTextColor": "#1a1a1a", "labelBoxBkgColor": "#f5f5f5", "labelBoxBorderColor": "#6c8ebf", "labelTextColor": "#1a1a1a", "classText": "#1a1a1a"}}}%%
            flowchart LR
            accTitle: The classes and the colours of these diagrams
              subgraph canvas[" "]
                subgraph boundary["Boundary<br/><i>[Level noun]</i>"]
                  block["Block"]:::block
                end
                user(["User"]):::user
                neighbour["Neighbour"]:::neighbour
                external["External system"]:::external
                store[("Data store")]:::store
                contract{{"Contract"}}:::contract
                group[["Group"]]:::group
                user -->|"Verb<br/><i>payload</i>"| block
                block -.- contract
              end
              style boundary fill:#f6f8fa,stroke:#999999,stroke-dasharray: 5 5
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
            - format: flowchart-type
              min: 1
              max: 1
            - format: acc-title
              min: 1
              max: 1
            - format: acc-descr
              min: 0
              max: 1
            - format: legend-node
              min: 7
              max: 7
            - format: arrow
              min: 1
              max: 1
            - format: provides
              min: 1
              max: 1
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
      - kind: bullet-list
        guidance: >
          The list says what each class of node in the diagram stands for, in the order of
          the diagram, and links the glossary term of the class, so that the glossary
          defines each kind of node once.
        rules:
          - template: '**${:phrase}**: ${:article} [${:link-text}][sokf:${:store}glossary#${:slug}].'
            example:
            - '**Block**: a [building block][sokf:glossary#building-block].'
            - '**User**: a [user][sokf:glossary#user].'
            - '**Neighbour**: a [neighbour][sokf:glossary#neighbour].'
            - '**External system**: an [external system][sokf:glossary#external-system].'
            - '**Data store**: a [data store][sokf:glossary#data-store].'
            - '**Contract**: a [contract][sokf:glossary#contract].'
            - '**Group**: a [group][sokf:glossary#group].'
          - select-pattern: 'glossary#building-block\]'
            min: 1
            max: 1
          - select-pattern: 'glossary#user\]'
            min: 1
            max: 1
          - select-pattern: 'glossary#neighbour\]'
            min: 1
            max: 1
          - select-pattern: 'glossary#external-system\]'
            min: 1
            max: 1
          - select-pattern: 'glossary#data-store\]'
            min: 1
            max: 1
          - select-pattern: 'glossary#contract\]'
            min: 1
            max: 1
          - select-pattern: 'glossary#group\]'
            min: 1
            max: 1

````
