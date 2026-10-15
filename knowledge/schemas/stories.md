---
type: Schema
id: schema-stories
title: Stories schema
description: A Stories document holds the stories for one user goal, and each story links the contracts that deliver it.
---

<!-- prettier-ignore -->
````yaml
line-limit: 0
root: /knowledge/stories

guidance: >
  A Stories document lists the stories for one user goal, and each story states what one
  kind of user needs and why. Stories documents are in `/knowledge/stories`. Each document
  covers one broad goal. A goal that serves two different kinds of work splits into two
  documents, so that a reader who does one kind of work finds all of its stories in one
  place.

frontmatter:
  type:
    const: Stories
  id:
    template: "stories-${:slug}"
  title:
    guidance: The title names the user goal in a few words.
    example: Following references
  description:
    guidance: The description says what users with this goal want to do.
    example: Users click the name of a file or an image in any window, and the editor or the image viewer shows it.

types:
  story-id: 'ST-[1-9][0-9]*'
  article: [A, An]
  user: '[a-z]+(?: [a-z]+)*'

formats:
  user-story:
    patch: true
    key: null # the heading of the story holds its key
    template: 'AS ${:indefinite-article} [${}][sokf:${:ref}], I WANT ${}, SO THAT ${}.'
    example: 'AS A [user][sokf:glossary#user], I WANT to click a file name and line number and see that line in the editor, SO THAT I reach the code in one click.'

sections:
  - heading: { from: title }
    level: 1
    blocks:
      - kind: prose
        max: 1
        guidance: >
          The opening says which user goal the stories serve and what users with that goal
          want to do. It also says that each story links the contracts that deliver it.
        example: 'This document holds the stories of a user who follows references. The user clicks the name of a file or an image in any window, and the editor or the image viewer shows it. Each story links the contracts that deliver it.'
      - kind: bullet-list
        guidance: >
          A contents list follows the opening, with one item for each story, in order. Each
          item repeats the story's heading and links to the story by the story's key.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[ST-1 A user can open a file from its name in any window](#st-1)'
            - '[ST-2 A user can see an image from its file name](#st-2)'
            - '[ST-3 A user can change what a click opens while working](#st-3)'

  - heading:
      template: '${id:story-id} ${:article} ${:user} can ${}'
      key: id
      unique: knowledge
      example: ST-1 A user can open a file from its name in any window
    level: 2
    repeat: true
    rule-id: stories
    guidance: >
      Each section contains one story. Its heading begins with the story's key, which is
      `ST-` and a number. The numbers form one sequence across all the stories in the
      knowledge store. A new story takes the next unused number. A story's number never
      changes, because other documents cite the key. After the key, a short clause says
      what the user can do: an article, the kind of user, the word _can_, and a verb with
      its object. The clause is in the user's own words. A need that the system meets in
      two separate ways is two stories, one for each way, because each way has its own
      contracts.
    examples:
      - |
        ## ST-2 A user can see an image from its file name

        AS A [user][sokf:glossary#user], I WANT to click the name of an image file and see the image, SO THAT I look at a picture in one click.

        > An image named in a mail message or a directory listing is only text in the window that shows it.

        Contracts: [PLUMB][sokf:contract-plumbing].
      - |
        ## ST-3 A user can change what a click opens while working

        AS A [user][sokf:glossary#user], I WANT to change the rules and have the new rules apply to my next click, SO THAT I fix a rule and keep my windows open.

        Contracts: [PLUMB][sokf:contract-plumbing].
    blocks:
      - kind: prose
        max: 1
        guidance: >
          The story states what the user wants and why, in the user's terms. The contracts
          say how the system meets the need. The name of the kind of user links to its entry
          in the glossary, and the glossary defines each kind of user before a story uses it.
        each:
          format: user-story
      - kind: prose
        optional: true
        max: 1
        guidance: >
          A `Planned:` paragraph marks a story that isn't built yet. It links the open issue
          that owns the change, and the story keeps the paragraph until the issue closes.
        each:
          pattern: '^Planned: .*\[[^\]]+\]\[sokf:(?:[a-z0-9/-]+:)?issue-[a-z0-9-]+\].*\.$'
          example: 'Planned: When a click matches no rule set, the user sees which pattern failed, as [the issue on failed patterns][sokf:issue-001-feature-report-failed-pattern] describes.'
      - kind: blockquote
        optional: true
        max: 1
        guidance: >
          The blockquote states, in one line, the fact that causes the need. The clause after
          `SO THAT` states the benefit.
        example: A compiler error names the file and the line, and typing them into the editor again is slow and leads to mistakes.
      - kind: prose
        max: 1
        guidance: >
          The last paragraph lists the contracts that deliver the story, and the text of each
          link is the contract's code. The story cites each contract by link alone, because
          the contract states its own criteria. The paragraph links only the contracts of
          building blocks that a user works with. Any other contract traces to the story
          through the contracts that depend on it.
        each:
          pattern: '^Contracts: \[[A-Z]+\]\[sokf:[^\]]+\](?:, \[[A-Z]+\]\[sokf:[^\]]+\])*\.$'
          example: 'Contracts: [PLUMB][sokf:contract-plumbing].'
      - kind: prose
        optional: true
        max: 1
        guidance: >
          The paragraph links each flow of the architecture that realises the story, by the
          flow's key. Every flow needs a story that cites it.
        each:
          pattern: '^Flows: \[[^\]]+\]\[sokf:(?:[a-z0-9/-]+:)?architecture#[a-z0-9-]+\](?:, \[[^\]]+\]\[sokf:(?:[a-z0-9/-]+:)?architecture#[a-z0-9-]+\])*\.$'
          example: 'Flows: [Show an image][sokf:architecture#show-an-image].'
````
