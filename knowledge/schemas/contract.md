---
type: Schema
id: schema-contract
title: Contract schema
description: The interface and the behaviour that a building block promises to its users, for any kind of interface.
---

<!-- prettier-ignore -->
````yaml
line-limit: 0
root: /knowledge/contracts
# A `Planned:` paragraph promises work that an open issue owns, so it links no
# closed issue.
current-links: ['^Planned: ']

guidance: >
  A contract defines the interface of one building block and the acceptance criteria that
  the interface meets. The block's users read it to learn what they can rely on. The
  block's developers read it to learn what the code does and what the tests check. A
  contract covers any kind of interface, such as a library, a network protocol, a command
  line, a file format, or a user interface. The stories say why users need the interface,
  and the design says how the block is built.

lifecycle:
  - key: scope
    states: [public, internal]
    guidance: >
      A public contract has users outside the system. An internal contract has users only
      in other subsystems, and it covers a major interface between two subsystems. An
      internal contract holds only the criteria that its users depend on, because each
      criterion needs its own code and its own test.

frontmatter:
  type:
    const: Contract
  id:
    template: "contract-${:slug}"
  code:
    template: '${:requirement-code}'
    guidance: >
      The code begins every key in the contract, such as PLUMB-1 and PLUMB_P-1. It is 1
      to 10 capital letters, because a longer code makes every key long. No other contract
      uses the code.
    example: PLUMB
  description:
    guidance: The description says what the interface lets its users do.
    example: 'The plumbing files let programs send messages to one another, read the messages sent to them, and change the rules that route the messages.'

# A story cites every public contract.
cited-by:
  - from-type: Stories
    only: { scope: [public] }
    severity: warn
  # Code implements each criterion of a part, and a test checks each quality criterion
  # and each property. The issue that builds a part holds the part's criteria until the
  # code exists.
  - from-marker: implements
    keys: criteria
  - from-marker: tests
    keys: quality-criteria
  - from-marker: tests
    keys: properties

types:
  quality-kind: [Performance, Capacity, Reliability, Security, Compatibility, Usability, Portability]

formats:
  requirement-channel:
    patch: true
    variants:
      channel:
        template: '**${:channel-name}**: ${}.'
        example: '**Read**: the rules in use, as text.'
  requirement-evolution:
    patch: true
    template: '**${:evolution-kind}**: ${:sentence}'
    example: '**Versioning**: The contract has a version number of its own, and the changelog records what each version changes.'

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says what the interface does and who uses it, with the interface as the
      subject of its first sentence. The interface's name links to the contract's node in
      the architecture. That node shows the block that provides the contract. A last
      sentence says that the contract defines the interface's elements, qualities, and
      properties for the developers and testers who rely on the interface. In a public
      contract, the sentence also names the interface's evolution.
    blocks:
      - kind: prose
        max: 1
        each:
          pattern: '^[A-Z].*\[[^\]]+\]\[sokf:([a-z0-9-]+:)?architecture#[a-z0-9_-]+\].*\. This contract defines .*\.$'
          example: 'The [plumbing files][sokf:architecture#plumbing] in `/mnt/plumb` carry messages between interactive programs, such as the editor `acme` and the image viewer `page`. A program writes a message to `/mnt/plumb/send`, and each program that has opened the destination port reads one copy of it. This contract defines the elements, qualities, properties, and evolution of the plumbing files, for the developers and testers who rely on them.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each section of the
          contract, in order. Each item links to the section's heading, so that a reader sees
          every section of the contract before reading any of them.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Interface](#interface)'
            - '[Qualities](#qualities)'
            - '[Properties](#properties)'
            - '[Evolution](#evolution)'

  - heading: Interface
    level: 2
    rule-id: interface
    guidance: >
      This section has one subsection for each element of the interface: an operation, an
      endpoint, a command, a message, a file, a resource, a policy, a metric, or a screen.
      The elements and the qualities share one sequence of keys, PLUMB-1, PLUMB-2, and so
      on, so that each key identifies one element or one quality.
    blocks:
      - kind: prose
        max: 1
        each:
          text: 'This section fully defines the [interface][sokf:#interface] and its functional behaviour with [acceptance criteria][sokf:#acceptance-criterion]. Where a code file defines the interface, a sokf [include][sokf:#include] copies the code verbatim.'

  - heading:
      template: '${id:requirement-id} ${:part-name}'
      key: id
      unique: knowledge
      key-derives-from: { from: code, separator: '-' }
      example: PLUMB-1 `/mnt/plumb/send`
    level: 3
    repeat: true
    parent: interface
    rule-id: parts
    guidance: >
      Each subsection describes one element of the interface. Its heading names the element
      as users see it. A name that is code is in backticks, such as `/mnt/plumb/send` or
      `plumb -j`, and any other name is a noun phrase, such as the title of a screen. A
      sentence on what the element does comes first, and Inputs, Outputs, and Criteria
      follow it, in that order.
    examples:
      - |
        ### PLUMB-2 `/mnt/plumb/PORT`

        `/mnt/plumb/PORT` gives each program with the file open a copy of each message that the plumber delivers to the port `PORT`, such as `edit` or `image`.

        #### Inputs

        - **Open**: a call that opens the file for reading and makes the program a reader of the port.
        - **Read**: a read of the file, which asks for the next message on the program's queue.

        #### Outputs

        - **Message**: the next message on the program's queue, as the six header lines followed by the data.

        #### Criteria

        - PLUMB-2_AC-1 [event]: WHEN the plumber delivers a message to a port, the plumber SHALL give one copy of the message to each program that has the port open.
        - PLUMB-2_AC-2 [state]: WHILE a program's queue for the port is empty, the plumber SHALL delay its reply to the program's read until a message arrives.
      - |
        ### PLUMB-3 `/mnt/plumb/rules`

        `/mnt/plumb/rules` contains the rules in use, and a write to the file changes them at once.

        #### Inputs

        - **Rules**: the text of the rule sets, separated by blank lines, in one or more writes.

        #### Outputs

        - **Read**: the rules in use, as text.
        - **Error**: `malformed rule` for a syntax error in the written text.

        #### Criteria

        - PLUMB-3_AC-1 [event]: WHEN a program opens `/mnt/plumb/rules` with truncation, the plumber SHALL clear the rules in use.
        - PLUMB-3_AC-2 [unwanted]: IF the written text has a syntax error, THEN the plumber SHALL fail the write with the error `malformed rule`.
        - PLUMB-3_AC-3 [event]: WHEN a program closes `/mnt/plumb/rules`, the plumber SHALL create each port that a rule set names.
    blocks:
      - kind: prose
        max: 1
        guidance: >
          One sentence, whose subject is the element, says what the element does. The
          sentence names the element in backticks, or describes it in a phrase that begins
          with _The_. The criteria describe the element's behaviour in detail.
        each:
          format: requirement-summary
          example: '`/mnt/plumb/send` accepts a message from any program and passes it to the port that the first matching rule set names.'

  - heading: Inputs
    level: 4
    parent: parts
    guidance: >
      This subsection lists what the block receives through this element, with one item for
      each channel. Each item begins with the channel's name in bold and a colon. The list
      of an element that receives nothing has `None.` as its only item.
    blocks:
      - kind: bullet-list
        guidance: >
          The channels depend on the kind of interface. A library takes the parameters, the
          state of the receiver, and the callbacks. An interface over HTTP or remote
          procedure calls takes the method, the path, the query, the headers, the body, and
          the credentials. A command line takes the arguments, standard input, the
          environment, the files that the command reads, and the configuration. An event
          interface takes the message that it consumes and its headers. A data or file
          format takes the file. Authentication and authorisation take the principal, the
          action, the resource, and the context. Telemetry takes the event that triggers the
          signal. Configuration takes the keys. A user interface takes the user's actions,
          the route, and the parameters.
        each:
          format: requirement-channel
          example:
            - '**Message**: the six header lines, which give the source application, the destination port, the working directory, the type of the data, the attributes, and the length of the data in bytes, followed by the data.'
            - '**Rules**: the rule sets in use, as the last write to `/mnt/plumb/rules` left them.'
      - kind: include
        optional: true
        repeat: true
        guidance: >
          An include of the file that defines each channel follows the list. The include
          identifies a source file by its path, and an asset by the id of its sidecar, the
          document that describes the asset. When no file defines a channel, the contract
          includes a file written for it, such as a JSON Schema, an OpenAPI file, or a
          protobuf file. When the code does not read that file, a test checks the code
          against it.
        source-pattern: '^(/|[a-z0-9])'
        example: |
          <!-- sokf:include /sys/include/plumb.h#plumbmsg -->
          <!-- /sokf:include -->
      - kind: group
        optional: true
        repeat: true
        guidance: >
          A `Planned:` paragraph says what a channel adds before the channel is built. The
          paragraph links the open issue that owns the change. A sketch after it shows only that addition.
          When the code exists, an include of the code replaces the paragraph and the sketch.
        blocks:
          - kind: prose
            max: 1
            each:
              pattern: '^Planned: .*\[[^\]]+\]\[sokf:(?:[a-z0-9/-]+:)?issue-[a-z0-9-]+\].*\.$'
              example: 'Planned: A message can have the attribute `explain=yes`, which asks the plumber to report which pattern failed, as [the issue on failed patterns][sokf:issue-001-feature-report-failed-pattern] describes.'
          - kind: code
            lang: any
            example: |
              explain=yes

  - heading: Outputs
    level: 4
    parent: parts
    guidance: >
      This subsection lists what the block returns or causes through this element, with one
      item for each channel. Each item begins with the channel's name in bold and a colon. A
      side effect, such as a file written, data stored, or an event sent, is an output. The
      list of an element that returns nothing has `None.` as its only item.
    blocks:
      - kind: bullet-list
        guidance: >
          The channels depend on the kind of interface. A library produces the return value,
          the errors, the changes to state, and the input and output that the call performs.
          An interface over HTTP or remote procedure calls produces the status, the headers,
          the body, the stored data, and the events sent. A command line produces standard
          output, standard error, the exit code, and the files that the command writes. An
          event interface produces the message, its topic, and its headers. A data or file
          format usually produces nothing. Authentication and authorisation produce the
          decision, the reason, and the audit record. Telemetry produces the metric, span, or
          log record and its attributes. Configuration produces nothing, because the
          contracts of the blocks that read the configuration define its effects. A user
          interface produces the screen, the navigation, and the requests sent.
        each:
          format: requirement-channel
          example:
            - '**Queue**: the message, rewritten as the matching rule set specifies, on the queue of the destination port.'
            - '**Program**: the program that a `plumb start` or `plumb client` action names, started when no program has the port open.'
            - '**Error**: `no matching plumb rule` when no rule set matches and the message names no port.'
      - kind: include
        optional: true
        repeat: true
        guidance: >
          An include of the file that defines each channel follows the list, as for the
          inputs.
        source-pattern: '^(/|[a-z0-9])'
        example: |
          <!-- sokf:include /sys/man/4/plumber#errors -->
          <!-- /sokf:include -->
      - kind: group
        optional: true
        repeat: true
        guidance: >
          A `Planned:` paragraph says what an output adds before the output is built. The
          paragraph links the open issue that owns the change. A sketch after it shows only that addition.
          When the code exists, an include of the code replaces the paragraph and the sketch.
        blocks:
          - kind: prose
            max: 1
            each:
              pattern: '^Planned: .*\[[^\]]+\]\[sokf:(?:[a-z0-9/-]+:)?issue-[a-z0-9-]+\].*\.$'
              example: 'Planned: With the attribute `explain=yes`, the error also names the failed pattern of the last rule set, as [the issue on failed patterns][sokf:issue-001-feature-report-failed-pattern] describes.'
          - kind: code
            lang: any
            example: |
              no matching plumb rule: arg isfile $0 failed in the rule set for edit
      - kind: image
        optional: true
        repeat: true
        guidance: >
          An image shows a screen, a wireframe, or a sequence diagram that the included files
          cannot show. The contract embeds the image by the id of its sidecar, the document
          that describes the image.
        example: '![The path of a message from the editor through the plumber to the image viewer][sokf:plumb-message-path]'

  - heading: Criteria
    level: 4
    parent: parts
    rule-id: part-criteria
    guidance: >
      This subsection says what the element does on success, on each error, and at each
      limit. Each criterion describes code that exists, because a `sokf-implements:` marker
      in that code cites the criterion's key. Until the code exists, the issue that plans
      the element contains the element's criteria. Each criterion has one trigger and one
      response, so "prints X and exits 0" is two criteria. Every criterion uses the same
      name for the system and says what the system does rather than how. A criterion uses
      _it_ only where the word has one possible referent. A criterion's tag matches the
      meaning of its condition, not its keyword: "WHEN the file is absent" is a state, and
      "WHEN the file does not parse" is an error.
    blocks:
      - kind: bullet-list
        guidance: >
          Each item contains one criterion in EARS, the Easy Approach to Requirements
          Syntax, with one of six tags for its kind. [event] marks a response that a trigger
          starts (WHEN ... SHALL). [state] marks a response that holds while a state lasts
          (WHILE ... SHALL). [unwanted] marks the response to an error or an unwanted
          condition (IF ... THEN ... SHALL). [optional] marks a response that applies only
          where a feature is present (WHERE ... SHALL). [complex] marks a response that
          combines WHILE with WHEN or IF. [ubiquitous] marks a response that always holds and
          needs none of these keywords.
        each:
          format: requirement-ears
          unique: repository
          rule-id: criteria
          key-derives-from: { rule: parts, separator: '_' }
          example:
            - 'PLUMB-1_AC-1 [event]: WHEN a program writes a message to `/mnt/plumb/send`, the plumber SHALL deliver the message to the port that the first matching rule set names.'
            - 'PLUMB-1_AC-2 [unwanted]: IF no rule set matches and the message names no port, THEN the plumber SHALL fail the write with the error `no matching plumb rule`.'
            - 'PLUMB-1_AC-3 [state]: WHILE no program has the destination port open, the plumber SHALL start the program that the matching rule set''s `plumb start` or `plumb client` action names.'
            - 'PLUMB-1_AC-4 [optional]: WHERE the message has a `click` attribute, the plumber SHALL replace the data with the text that the matching rule set''s patterns match around the click position.'

  - heading: Qualities
    level: 2
    rule-id: qualities
    guidance: >
      This section has one subsection for each kind of quality that the contract promises.
      The subsections follow the order Performance, Capacity, Reliability, Security,
      Compatibility, Usability, and Portability, so that a reader finds each kind in the
      same place in every contract. Each criterion promises a result that a test can check,
      such as a time, a size, or an error.
    blocks:
      - kind: prose
        max: 1
        each:
          text: 'This section fully defines the non-functional behaviour of this contract. Each [quality][sokf:#quality] is an acceptance criterion about how well the interface works: its speed, limits, security, availability, or compatibility.'

  - heading:
      template: '${id:requirement-id} ${:quality-kind}'
      key: id
      unique: knowledge
      key-derives-from: { from: code, separator: '-' }
      example: PLUMB-4 Performance
    level: 3
    repeat: true
    parent: qualities
    rule-id: quality-kinds
    guidance: >
      Each subsection covers one of the seven kinds of quality. Performance covers time and throughput.
      Capacity covers limits and sizes. Reliability covers availability, recovery, and data
      safety. Security covers who can do what, and what is protected. Compatibility covers
      the versions, formats, and platforms that the interface supports. Usability covers
      what the user sees, such as messages and accessibility. Portability covers where the
      interface runs, such as line endings and encodings.
    examples:
      - |
        ### PLUMB-5 Reliability

        - PLUMB-5_AC-1 [unwanted]: IF a program exits with messages in its queue for a port, THEN the plumber SHALL discard those messages.
      - |
        ### PLUMB-6 Compatibility

        - PLUMB-6_AC-1 [ubiquitous]: the plumber SHALL serve its files over 9P, the Plan 9 file protocol.
      - |
        ### PLUMB-7 Usability

        - PLUMB-7_AC-1 [unwanted]: IF the rules file that the plumber reads at startup has a syntax error, THEN the plumber SHALL exit with the error `parse error`.
    blocks:
      - kind: bullet-list
        guidance: >
          A criterion states a number and a unit where the quality has one, such as 50 ms or
          8192 bytes.
        each:
          format: requirement-ears
          unique: repository
          rule-id: quality-criteria
          key-derives-from: { rule: quality-kinds, separator: '_' }
          example: 'PLUMB-4_AC-1 [ubiquitous]: the plumber SHALL return from a write to `/mnt/plumb/send` without waiting for a program to read the message.'

  - heading: Properties
    level: 2
    guidance: >
      This section states the properties that hold for every valid input. The words after
      FOR ANY name the inputs exactly, because the property-based test generates those
      inputs. The properties have the keys PLUMB_P-1, PLUMB_P-2, and so on.
    blocks:
      - kind: prose
        max: 1
        each:
          text: 'This section fully defines the [properties][sokf:#property] of this contract. A property is an acceptance criterion that holds for all valid inputs. A property-based test checks it.'
      - kind: bullet-list
        guidance: >
          Each property has one of six tags for its kind. [round-trip] marks an operation
          that, followed by its inverse, returns the input. [invariant] marks something that stays
          true, or stays within a bound. [idempotent] marks an action that has the same effect
          done twice as done once. [order-independent] marks actions that give the same
          result in any order. [equivalent] marks two methods that give the same result, and
          one of them is often a simple model. [metamorphic] marks a known change to the
          input that gives a known change to the output.
        each:
          format: requirement-property
          unique: repository
          rule-id: properties
          key-derives-from: { from: code, separator: '_' }
          example:
            - 'PLUMB_P-1 [invariant]: FOR ANY message and any set of programs that have its destination port open, the plumber SHALL give each of those programs exactly one copy of the message.'
            - 'PLUMB_P-2 [idempotent]: FOR ANY valid rules text, the plumber SHALL have the same rule sets in use after two writes of the text to `/mnt/plumb/rules` as after one.'

  - heading: Evolution
    level: 2
    only: { scope: [public] }
    guidance: >
      This section says how users tell the versions of the interface apart, what stays the
      same in every version, and how long an old version keeps working, with a number and a
      unit. Each item is a statement in the present tense rather than a SHALL criterion,
      because it is a rule for changing the contract, not a promise that code keeps.
    blocks:
      - kind: prose
        max: 1
        each:
          text: 'This section fully defines how this contract can change. A change that these rules do not allow is a breaking change.'
      - kind: bullet-list
        each:
          format: requirement-evolution
        sequence:
          - template: '**Versioning**: ${}'
            example: '**Versioning**: The contract has a version number of its own, and the changelog records what each version changes.'
          - template: '**Backwards compatibility**: ${}'
            example: '**Backwards compatibility**: A release keeps the six header lines in their order and adds new information about a message as an attribute. A program written for an earlier release reads every message.'
          - template: '**Deprecation**: ${}'
            example: '**Deprecation**: A deprecated verb of the rules language keeps working for one more release, and the plumber prints a warning for each rule set that uses it.'
````
