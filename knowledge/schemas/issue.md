---
type: Schema
id: schema-issue
title: Issue schema
description: 'An issue records one piece of work (a feature, a bug, or a chore): what the work asks for, the plan for doing it, and its outcome.'
---

<!-- prettier-ignore -->
````yaml
line-limit: 0

guidance: >
  An issue records one piece of work: a feature, a bug, or a chore. Its readers are the
  person or the agent doing the work and anyone who later asks why the work was done. The
  issue begins with what the work has to achieve, as a GitHub issue does. The plan follows,
  with its open questions, its approach, its files, its steps, and its verification. After
  the work is done, the outcome follows the plan. An issue is history, not the authority.
  The stories, the contracts, the designs, and the code are the authority, and none of them
  cites an issue.

root: /knowledge/issues

# An issue is history, so no code or test cites one.
markers: []

# While an issue is open, each new file that its Files section links is planned,
# so a design can link and include the file before the code exists.
plans-paths:
  - section: files
    only: { state: [open] }

lifecycle:
  - key: state
    states: [open, closed]
    path: true
    archived: [closed]
    guidance: >
      An issue is open while its work remains to be done. It closes after the work is done
      or dropped, and a closed issue doesn't change. The issue's folder has the name of its
      state, and `sokf graph` shows a closed issue only with `--archived`.
  - key: kind
    states: [feature, bug, chore]
    guidance: >
      The kind is _feature_ for something that a user can't do yet, _bug_ for something
      that doesn't work as a contract says, and _chore_ for work that no user sees. The id
      contains the kind, so the kind never changes. An issue of the wrong kind closes, and a
      new issue of the right kind replaces it.

frontmatter:
  type:
    const: Issue
  id:
    template: "issue-${:seq}-${kind}-${:slug}"
  title:
    template: '${:action} ${}'
    guidance: >
      The title describes the work in a few words. It begins with a verb from the type
      `action` of the issue schema, followed by what the verb acts on. When no verb fits the
      work, the type `action` gains one.
    example: Report the pattern that stops a match
  description:
    guidance: The description says what the issue asks for, in one sentence.
    example: 'When a message with the attribute `explain=yes` matches no rule set, the error names the pattern that failed.'

links:
  - rel: depends-on
    to-type: Issue
    min: 0
    guidance: A depends-on link points to an issue that has to close before this one can.
  - rel: supersedes
    to-type: Issue
    min: 0
    guidance: A supersedes link points to an issue that this one replaces.
  - rel: relates-to
    to-type: Issue
    min: 0
    guidance: A relates-to link points to another issue whose work included this one's.

types:
  decision-id: 'D-[1-9][0-9]*'
  question-id: 'Q-[1-9][0-9]*'
  action: [Add, Allow, Change, Fix, Make, Move, Remove, Rename, Replace, Report, Split, Tighten]

sections:
  - heading: { from: title }
    level: 1
    guidance: The opening says who needs the work, why, and what the issue contains. A contents list follows it.
    blocks:
      - kind: prose
        max: 1
        guidance: >
          One paragraph says who needs what and why, adding to the description rather than
          repeating it. Its last sentence says what the issue contains.
        example: 'A user who writes plumbing rules needs to know which pattern stopped a message from matching, so that the user can correct the rule set rather than guess. This issue describes the need, the report that meets it, and the plan for the matcher and the file server.'
      - kind: bullet-list
        only: { state: [open], kind: [feature] }
        guidance: The contents list has one item for each level-2 section, in order, and each item links to its section's heading.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Problem](#problem)'
            - '[Proposed solution](#proposed-solution)'
            - '[Alternatives considered](#alternatives-considered)'
            - '[Acceptance criteria](#acceptance-criteria)'
            - '[Out of scope](#out-of-scope)'
            - '[Plan](#plan)'
      - kind: bullet-list
        only: { state: [open], kind: [bug] }
        guidance: The contents list has one item for each level-2 section, in order, and each item links to its section's heading.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Steps to reproduce](#steps-to-reproduce)'
            - '[Expected behaviour](#expected-behaviour)'
            - '[Actual behaviour](#actual-behaviour)'
            - '[Environment](#environment)'
            - '[Acceptance criteria](#acceptance-criteria)'
            - '[Out of scope](#out-of-scope)'
            - '[Plan](#plan)'
      - kind: bullet-list
        only: { state: [open], kind: [chore] }
        guidance: The contents list has one item for each level-2 section, in order, and each item links to its section's heading.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Motivation](#motivation)'
            - '[Acceptance criteria](#acceptance-criteria)'
            - '[Out of scope](#out-of-scope)'
            - '[Plan](#plan)'
      - kind: bullet-list
        optional: true
        only: { state: [closed], kind: [feature] }
        guidance: >
          The contents list has one item for each level-2 section, in order, and each item
          links to its section's heading. The list is optional in a closed issue, because a
          closed issue is history and keeps the form it had when it closed.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Problem](#problem)'
            - '[Proposed solution](#proposed-solution)'
            - '[Alternatives considered](#alternatives-considered)'
            - '[Acceptance criteria](#acceptance-criteria)'
            - '[Out of scope](#out-of-scope)'
            - '[Plan](#plan)'
            - '[Outcome](#outcome)'
      - kind: bullet-list
        optional: true
        only: { state: [closed], kind: [bug] }
        guidance: >
          The contents list has one item for each level-2 section, in order, and each item
          links to its section's heading. The list is optional in a closed issue, because a
          closed issue is history and keeps the form it had when it closed.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Steps to reproduce](#steps-to-reproduce)'
            - '[Expected behaviour](#expected-behaviour)'
            - '[Actual behaviour](#actual-behaviour)'
            - '[Environment](#environment)'
            - '[Acceptance criteria](#acceptance-criteria)'
            - '[Out of scope](#out-of-scope)'
            - '[Plan](#plan)'
            - '[Outcome](#outcome)'
      - kind: bullet-list
        optional: true
        only: { state: [closed], kind: [chore] }
        guidance: >
          The contents list has one item for each level-2 section, in order, and each item
          links to its section's heading. The list is optional in a closed issue, because a
          closed issue is history and keeps the form it had when it closed.
        each:
          template: '[${}](#${:slug})'
          example:
            - '[Motivation](#motivation)'
            - '[Acceptance criteria](#acceptance-criteria)'
            - '[Out of scope](#out-of-scope)'
            - '[Plan](#plan)'
            - '[Outcome](#outcome)'

  # The issue: a feature.
  - heading: Problem
    level: 2
    only: { kind: [feature] }
    guidance: >
      This section says what the user can't do yet, why that matters, and what the code does
      now, with a link to the code or to the section of the design concerned. It links the
      story that the issue serves by its key, such as [ST-3][sokf:#st-3]. When no story
      describes the need yet, a goal document gains one before the issue closes.
    blocks:
      - kind: prose
        example: 'A user who writes a rule set can''t tell why a message doesn''t match it. When a message matches no rule set, the write to `send` fails with the same error, `no matching plumb rule`, whichever pattern failed. The code of [`matchruleset`][sokf:design-plumber#plmb-1] and of the file server causes this:'
      - kind: bullet-list
        optional: true
        guidance: Each item states one finding, where the section has more than one.
        example:
          - '`matchruleset` stops at the first pattern that fails, returns nil, and discards the pattern.'
          - 'The file server turns every nil result into the same error.'
  - heading: Proposed solution
    level: 2
    only: { kind: [feature] }
    guidance: This section describes the change as the user sees it. The plan describes how the change is built.
    blocks:
      - kind: prose
        example: 'When a message with the attribute `explain=yes` matches no rule set, the `plumb` command prints an error. The error names the failed pattern of the last rule set, such as `arg isfile $0`.'

  # The issue: a bug.
  - heading: Steps to reproduce
    level: 2
    only: { kind: [bug] }
    guidance: This section lists the fewest steps that show the bug, with one action in each step.
    blocks:
      - kind: numbered-list
        example:
          - 'Use a rule set that matches a file name with the pattern `data matches ''[a-zA-Z0-9_\-./]+''` and sends it to the editor with `plumb to edit`.'
          - 'Run `plumb -a click=5 mem.c`, which sends the text `mem.c` with the click position at its end.'
  - heading: Expected behaviour
    level: 2
    only: { kind: [bug] }
    guidance: >
      This section says what should happen and links the contract criterion that says so.
      When no criterion says so, the contract has a gap: the contract gains a new criterion,
      and this section links it.
    blocks:
      - kind: prose
        example: 'The editor receives `mem.c`, because the click position lies at the end of the matched text, as [PLUMB-1_AC-4][sokf:contract-plumbing#plumb-1_ac-4] says.'
  - heading: Actual behaviour
    level: 2
    only: { kind: [bug] }
    guidance: This section says what happens instead, with the output where there is any. The Approach section of the plan states the root cause.
    blocks:
      - kind: prose
        example: 'The write to `send` fails with `no matching plumb rule`, and the editor receives nothing.'
      - kind: code
        optional: true
        lang: any
        example: |
          % plumb -a click=5 mem.c
          plumb: no matching plumb rule
  - heading: Environment
    level: 2
    optional: true
    only: { kind: [bug] }
    guidance: This section states the version and the platform, where they affect the bug.
    blocks:
      - kind: bullet-list
        example:
          - 'Plan 9 from Bell Labs, fourth edition'
          - 'A 386 terminal'

  # The issue: a chore.
  - heading: Motivation
    level: 2
    only: { kind: [chore] }
    guidance: >
      This section says two things: why the work is worth doing now, though no user sees
      it, and what the code does now. It links the story or the issue that the work serves.
    blocks:
      - kind: prose
        example: 'A new verb needs an entry in the list of verbs in `plumber.h`, a name in the parser in `rules.c`, and a case in `match.c`. Nothing checks that the three agree, so a verb that one of them lacks fails only when a message uses it.'
      - kind: bullet-list
        optional: true
        guidance: Each item states one finding, when there is more than one.
        example:
          - 'The parser in `rules.c` turns the name of a verb into its number.'
          - 'The matcher in `match.c` handles each verb in a case of its own.'

  # Every kind of issue.
  - heading: Alternatives considered
    level: 2
    optional: true
    guidance: This section lists each other way to do the work, with the reason the issue doesn't choose it.
    blocks:
      - kind: bullet-list
        example:
          - 'A log of every match in a file: the user has to find the log and read it after each failure.'
          - 'A command that tests one message against one rule set: it reports nothing about the other rule sets.'

  - heading: Acceptance criteria
    level: 2
    guidance: >
      This section is a checklist of what is true when the work is done, with one item for
      each criterion. A check that applies to every issue, such as a clean validation or a
      passing test suite, isn't a criterion, and the list leaves it out.
    blocks:
      - kind: bullet-list
        only: { state: [open] }
        guidance: >
          When the contract already contains a criterion, its item is the criterion's key as a
          link, without the criterion's wording. Any other item states its criterion in EARS
          (Easy Approach to Requirements Syntax), and the contract gains that criterion before
          the issue closes. An item is ticked when it's true.
        rules:
          - pattern: '^\[[ x]\] \[[A-Z]+(?:_[A-Z]+)*(?:-[0-9]+)?(?:_AC-[0-9]+|_P-[0-9]+|_[0-9]+)?\]\[sokf:[^\]]+\](?:(?:, and |, | and | to )\[[A-Z]+(?:_[A-Z]+)*(?:-[0-9]+)?(?:_AC-[0-9]+|_P-[0-9]+|_[0-9]+)?\]\[sokf:[^\]]+\])*$'
            min: 0
            example:
              - '[x] [PLUMB-1_AC-5][sokf:contract-plumbing#plumb-1_ac-5]'
              - '[ ] [PLUMB-1_AC-6][sokf:contract-plumbing#plumb-1_ac-6] and [PLUMB-1_AC-7][sokf:contract-plumbing#plumb-1_ac-7]'
          - pattern: '^\[[ x]\] (?:(?:WHEN|WHILE|WHERE) [^,]+, |IF .+, THEN )?[Tt]he .+ SHALL .+\.$'
            min: 0
            example:
              - '[ ] WHEN a message with the attribute `explain=yes` matches no rule set, the `plumb` command SHALL print the failed pattern of the last rule set.'
              - '[ ] WHEN a message matches no rule set, the file server SHALL fail the write to `send` with an error that begins with `no matching plumb rule`.'
      - kind: bullet-list
        only: { state: [closed] }
        guidance: >
          In a closed issue, an item that isn't ticked ends with `(not done:` and the reason.
          A criterion can leave the contract after the issue closes. The item then links the
          criterion by a permalink to the last commit that holds it.
        rules:
          - select-pattern: '^\[ \]'
            pattern: '\(not done: .+\)$'
            min: 0
            example: '[ ] [PLUMB-1_AC-6][sokf:contract-plumbing#plumb-1_ac-6] (not done: an error string of 9P, the Plan 9 file protocol, holds at most 128 bytes)'
          - pattern: '^\[[ x]\] \[[A-Z]+(?:_[A-Z]+)*(?:-[0-9]+)?(?:_AC-[0-9]+|_P-[0-9]+|_[0-9]+)?\](?:\[sokf:[^\]]+\]|\(https://github\.com/six5536(?:-private)?/sokf/blob/[0-9a-f]{40}/[^)]+\))(?:(?:, and |, | and | to )\[[A-Z]+(?:_[A-Z]+)*(?:-[0-9]+)?(?:_AC-[0-9]+|_P-[0-9]+|_[0-9]+)?\](?:\[sokf:[^\]]+\]|\(https://github\.com/six5536(?:-private)?/sokf/blob/[0-9a-f]{40}/[^)]+\)))*$'
            min: 0
            example: '[x] [PLUMB-1_AC-5][sokf:contract-plumbing#plumb-1_ac-5]'
          - pattern: '^\[[ x]\] (?:(?:WHEN|WHILE|WHERE) [^,]+, |IF .+, THEN )?[Tt]he .+ SHALL .+\.$'
            min: 0
            example: '[x] WHEN a message matches no rule set, the file server SHALL fail the write to `send` with an error that begins with `no matching plumb rule`.'

  - heading: Out of scope
    level: 2
    guidance: >
      This section lists what the issue leaves out, one thing in each item. An item links
      the issue that covers it, where there is one.
    blocks:
      - kind: bullet-list
        example:
          - 'A report on a message that matches the wrong rule set, because no pattern fails in that case.'
          - 'A change to the editor `acme`, because the error string reaches it unchanged through `libplumb`.'

  # The plan.
  - heading: Plan
    level: 2
    optional: true
    guidance: >
      This section says how the work is done, and it comes before the work starts. The
      preceding sections give its context.
    blocks:
      - kind: prose
        max: 1
        only: { state: [open] }
        guidance: One paragraph states the plan in outline.
        example: 'The plan changes the matcher and the file server and adds the test `nomatch.rc`. The matcher records the failed pattern, and the file server puts it in the error of a message with the attribute `explain=yes`.'
      - kind: prose
        max: 1
        optional: true
        only: { state: [closed] }
        guidance: One paragraph states the plan in outline.
        example: 'The plan changes the matcher and the file server and adds the test `nomatch.rc`. The matcher records the failed pattern, and the file server puts it in the error of a message with the attribute `explain=yes`.'
  - heading: Open questions
    level: 3
    optional: true
    only: { state: [open] }
    guidance: >
      This section lists the questions that the plan has to settle, each with a key (Q-1,
      Q-2, and so on) and the recommended answer. An answered question becomes a decision
      in the Approach section.
    blocks:
      - kind: bullet-list
        each:
          template: '${id:question-id} ${}? Recommended: ${}.'
          key: id
          example:
            - 'Q-1 Does the error name a pattern from every rule set, or only from the last one? Recommended: only the last one, because an error string of 9P, the Plan 9 file protocol, holds at most 128 bytes.'
            - 'Q-2 Does a message ask for the report with an attribute, or does every error carry it? Recommended: an attribute, `explain=yes`, because a program that reads the error expects `no matching plumb rule` alone.'
  - heading: Approach
    level: 3
    optional: true
    guidance: This section describes the chosen approach, then lists its decisions. A rejected option belongs in Alternatives considered.
    blocks:
      - kind: prose
        optional: true
        guidance: >
          The prose states the root cause of a bug, where it's known, then the approach. It
          links the design that records the decisions.
        example: 'The file server appends the failed pattern and the port of its rule set to the error, as in `no matching plumb rule: arg isfile $0 failed in the rule set for edit`. The [plumber''s design][sokf:design-plumber] records the decisions:'
      - kind: bullet-list
        guidance: >
          Each decision has a key (D-1, D-2, and so on), because a later issue cites a decision
          by its key. Each decision states its reason after _because_. A decision that refines
          an earlier one is a new decision that cites the earlier one.
        each:
          template: '${id:decision-id} ${}, because ${}.'
          key: id
          example:
            - 'D-1 `matchruleset` returns the number of the failed pattern through a new argument, because a nil result then means the same as before to every other caller.'
            - 'D-2 The error names the failed pattern of the last rule set only, because an error string of 9P holds at most 128 bytes.'
  - heading: Files
    level: 3
    optional: true
    rule-id: files
    guidance: >
      This section lists the files that the work changes or creates, and the existing code to
      reuse, one in each item. Each item gives the path as a link, a colon, and what changes.
      While the issue is open, a link to a file that doesn't exist yet plans that file, so a
      design can link and include it before the code exists. A removed file is in code font.
    blocks:
      - kind: bullet-list
        example:
          - '[`/sys/src/cmd/plumb/match.c`](/sys/src/cmd/plumb/match.c): `matchruleset` records the number of the pattern that failed.'
          - '[`/sys/src/cmd/plumb/fsys.c`](/sys/src/cmd/plumb/fsys.c): the file server adds the failed patterns to the error of a message with the attribute `explain=yes`.'
          - '[`/sys/src/cmd/plumb/test/nomatch.rc`](/sys/src/cmd/plumb/test/nomatch.rc): a test that sends a message with no matching rule set and reads the error.'
  - heading: Steps
    level: 3
    optional: true
    guidance: >
      This section lists the steps in the order of the work, with one action in each step. It
      says why the order matters where the reason isn't obvious.
    blocks:
      - kind: numbered-list
        example:
          - 'Add the new argument to `matchruleset`, and pass nil from every caller, so that the plumber still builds and behaves as before.'
          - 'Record the failed pattern in `matchruleset`.'
          - 'In the file server, add the failed patterns to the error of a message with the attribute `explain=yes`.'
  - heading: Verification
    level: 3
    optional: true
    guidance: >
      This section lists the checks that show the work is done, from end to end. Each check
      states an action and the result that shows the work is done. It also names the tests
      that cover the check.
      The section leaves out a check that applies to every issue, such as a clean validation
      or a passing test suite.
    blocks:
      - kind: bullet-list
        example:
          - 'Plumb the name of a file that doesn''t exist, such as `nosuch.c`, with the attribute `explain=yes`, and check that the error names the pattern `arg isfile $0`. The test `nomatch.rc` checks the same thing.'

  - heading: Outcome
    level: 2
    only: { state: [closed] }
    guidance: >
      This section is one paragraph about what the work delivered, with a link to each
      document that the work changed. The subsections Differs from the plan and Left for later
      follow it where they apply. When another issue's work included this one's, the paragraph
      says so, and the issue links the other issue with relates-to. The paragraph describes the
      result, because git records the steps, the counts, and the dates.
    blocks:
      - kind: prose
        max: 1
        each:
          prohibited-patterns: [{pattern: '\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b', message: 'The text contains no date, because git records when the work happened.'}, {pattern: '^Step [0-9]+:', message: 'An item begins with no step number, because the outcome describes the result, not the steps.'}]
        example: 'With the attribute `explain=yes`, the `plumb` command prints the pattern that failed in the last rule set, as [PLUMB-1_AC-5][sokf:contract-plumbing#plumb-1_ac-5] says. The [plumber''s design][sokf:design-plumber#plmb-1] describes the new argument of `matchruleset`.'

  - heading: Differs from the plan
    level: 3
    optional: true
    only: { state: [closed] }
    guidance: This section lists each place where the work departed from the plan, and why, one place in each item.
    blocks:
      - kind: bullet-list
        each:
          prohibited-patterns: [{pattern: '\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b', message: 'The text contains no date, because git records when the work happened.'}, {pattern: '^Step [0-9]+:', message: 'An item begins with no step number, because the outcome describes the result, not the steps.'}]
        example: 'The error names the failed pattern of the last rule set only, because an error string of 9P, the Plan 9 file protocol, holds at most 128 bytes.'
  - heading: Left for later
    level: 3
    optional: true
    only: { state: [closed] }
    guidance: This section lists the work that is still wanted but left undone, one thing in each item. An item links the issue that covers it, where there is one.
    blocks:
      - kind: bullet-list
        each:
          prohibited-patterns: [{pattern: '\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b', message: 'The text contains no date, because git records when the work happened.'}, {pattern: '^Step [0-9]+:', message: 'An item begins with no step number, because the outcome describes the result, not the steps.'}]
        example: 'A report of the failed pattern of every rule set. It needs a way to return more text than an error string of 9P, the Plan 9 file protocol, holds.'
````
