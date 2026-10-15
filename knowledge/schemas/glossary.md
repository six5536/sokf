---
type: Schema
id: schema-glossary
title: Glossary schema
description: The terms that the documents of a knowledge store use, each defined once.
---

<!-- prettier-ignore -->
````yaml
line-limit: 2000

guidance: >
  The glossary defines each term that the documents of a knowledge store use, so that no
  other document defines it. A document links the first use of a term to the term's entry,
  for example `[port][sokf:#port]`. A reader who meets an unfamiliar term follows that link.
  A knowledge store has one glossary, and its id is always `glossary`. The glossary lists
  the terms of the domain, then the terms of each large area of the domain, and then the
  terms of the method.

frontmatter:
  type:
    const: Glossary
  id:
    const: glossary
  description:
    guidance: The description says whose terms the glossary defines.
    example: 'The terms that the plumber''s documents use for messages, ports, and rules, and the terms of the method.'

types:
  term: '[^*]+'
  definition: '[A-Z].*'
  area: '[A-Z][A-Za-z0-9 ]*'

sections:
  - heading: { from: title }
    level: 1
    guidance: >
      The opening says what the glossary defines, how a document cites a term, and which
      sections the glossary has.
    blocks:
      - kind: prose
        max: 1
        example: 'This glossary defines each term that the documents of the plumber use. A document links the first use of a term to the term''s entry. The Domain section defines the terms of the plumber, the Rules and Actions sections define the terms of the rule language, and the Method section defines the terms of the method.'
      - kind: bullet-list
        guidance: >
          The contents list follows the opening, with one item for each level-2 section, in
          order. The items for Domain and Method have the same text in every project, so
          that every glossary reads the same.
        rules:
          - text: '[Domain](#domain): the terms that are specific to this project.'
            max: 1
          - template: '[${:area}](#${:slug}): the terms of ${}, an area of the domain.'
            min: 0
            example:
              - '[Rules](#rules): the terms of the rule language, an area of the domain.'
              - '[Actions](#actions): the terms of the actions of a rule set, an area of the domain.'
          - text: '[Method](#method): the terms of the software development method.'
            max: 1

  - heading: Domain
    level: 2
    guidance: >
      This section defines the terms of the system and of the problem that the system
      solves. A domain term's name differs from the name of every term of the method,
      because each term has one entry in the knowledge store.
    blocks:
      - kind: bullet-list
        guidance: >
          Each item defines one term, and the items are in alphabetical order. Each term is
          in sentence case. A term's slug differs from the slug of every heading, because
          the term's anchor and the heading's anchor would otherwise be the same. Each
          definition is one or two sentences for a reader who is new to the system. An item
          ends with `Also known as …` when people use a different word for the same thing.
          An item ends with `Not to be confused with …` and a link when a reader might
          confuse the term with another. When a specification defines a term, the item
          summarises the definition in one sentence. A footnote and a `sources` entry cite
          the specification.
        each:
          template: '**${term:term}**: ${:definition}'
          key: term
          unique: knowledge
          example:
            - '**Click attribute**: The attribute `click`, whose value is the position of the cursor in the data of a message. When a message has this attribute, the plumber sends only the part of the data that the patterns of the matching rule set match around that position.'
            - '**Message**: The unit that the plumber passes between programs. It has six header lines, followed by the data. The header lines give the source application, the destination port, the working directory, the type of the data, the attributes, and the length of the data.'
            - '**Port**: A file in `/mnt/plumb`, such as `edit` or `image`, from which programs read the messages for one kind of work. Each program that has opened the port receives one copy of each message sent to the port.'

  - heading: { template: '${:area}', example: Rules }
    level: 2
    optional: true
    repeat: true
    guidance: >
      An area of the domain with four or more terms of its own, such as a rule language,
      has a section of its own. The section follows the rules of the Domain section. The
      terms of an area with fewer than four terms are in the Domain section, because a
      short section adds a heading without making its terms easier to find.
    examples:
      - |
        ## Actions

        - **Action**: A rule whose object is `plumb`. An action says what the plumber does with a message after the action's rule set matches the message.
        - **Plumb client**: An action that starts a program when no program has opened the destination port. The plumber holds the message until the new program opens the port.
        - **Plumb start**: An action that starts a program to handle a message when no program has opened the destination port. The file server in `fsys.c` starts the program.
        - **Plumb to**: An action that names the destination port of a message, such as `plumb to edit`.
    blocks:
      - kind: bullet-list
        each:
          template: '**${term:term}**: ${:definition}'
          key: term
          unique: knowledge
          example:
            - '**Object**: The first word of a rule, such as `data` or `type`. It names the part of a message that the rule tests or sets.'
            - '**Rule**: One line of a rule set, written as an object, a verb, and an argument, such as `data matches ''[a-zA-Z0-9_\-./]+''`.'
            - '**Rule set**: A group of rules, separated from the next group by a blank line. The plumber tries the rule sets in order and carries out the actions of the first set whose patterns all match the message.'
            - '**Verb**: The second word of a rule, such as `matches` or `to`. It says what the rule does with its object: `matches` tests the object, and `to` names the destination port.'

  - heading: Method
    level: 2
    guidance: >
      This section defines the terms of the software development method. Their text is the
      same in every project, so that a term of the method means the same in every knowledge
      store. A document links to the method with `[method][sokf:glossary#method]`.
    blocks:
      - kind: bullet-list
        each:
          template: '**${term:term}**: ${:definition}'
          key: term
          unique: knowledge
          example: '**Part**: A building block inside a subsystem.'
        sequence:
          - text: '**Acceptance criterion**: One promise that a contract makes, written as one sentence in a fixed form. A test can check it. Also known as a criterion or an EARS (Easy Approach to Requirements Syntax) requirement.'
          - text: '**Behaviour**: What a building block does at its interface: its results, its errors, and its limits. Acceptance criteria state the behaviour.'
          - text: '**Building block**: A unit of the system with one responsibility. Other units can use it only through its interface. Also known as a component in the C4 model (Context, Containers, Components, and Code).'
          - text: '**Contract**: The interface and the behaviour that a building block promises to its users. A public contract has users outside the system. An internal contract has users only in other subsystems.'
          - text: '**Data store**: A place where the system stores data between runs, for example a database, a file, or a cache.'
          - text: '**Decision**: A choice in a design, and the reason for it.'
          - text: '**Design**: How a building block implements its contract: its parts, how they work together, and the decisions about them.'
          - text: '**External system**: A system outside the described system that connects to it.'
          - text: '**Flow**: The sequence in which building blocks work together to do one task for a user.'
          - text: '**Group**: A set of parts that a diagram shows as one building block. A different diagram shows all the parts of the group.'
          - text: '**Host**: A machine, a container, or a service on which building blocks of the system run.'
          - text: '**Include**: A block of a document that sokf fills with the text of a code file or of another document, and keeps up to date. The document and its source always agree.'
          - text: '**Interface**: The operations, data, events, or screens through which users access a building block.'
          - text: '**Module**: A unit of code inside a part, for example a package, a namespace, or a file.'
          - text: '**Neighbour**: A building block of the system that lies outside the boundary of a diagram. The diagram shows it because the parts inside the boundary connect to it.'
          - text: '**Part**: A building block inside a subsystem.'
          - text: '**Property**: An acceptance criterion of a contract that holds for all valid inputs. A property-based test checks it against many generated inputs. Also known as an invariant.'
          - text: '**Quality**: An acceptance criterion about how well a building block works, not what it does: its speed, limits, security, availability, or compatibility. Also known as a non-functional requirement.'
          - text: '**Story**: A user''s need and the reason for it, in the form "AS A …, I WANT …, SO THAT …". Each section of a Stories document contains one story and links to the contracts that meet it.'
          - text: '**Subsystem**: One of the largest building blocks of a system. It can change without changes to the other subsystems, as long as its contracts stay the same.'
          - text: '**System**: The software that the documents describe, as its users see it.'
          - text: '**User**: A person, or the role of a person, whom the system serves. The person can use the system directly, or through software that acts for them, for example an artificial intelligence (AI) agent.'
````
