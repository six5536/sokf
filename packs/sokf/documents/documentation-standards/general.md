---
type: DocumentationStandard
id: documentation-standard-general
title: General documentation standard
description: How to write the prose of this project's documents, modelled on The C Programming Language by Kernighan and Ritchie.
sources:
  - id: kr
    resource: https://www.informit.com/store/c-programming-language-9780131103627
    title: The C Programming Language, second edition (Kernighan, Ritchie), Prentice Hall, 1988
  - id: plan9-licence
    resource: https://github.com/plan9foundation/plan9/blob/main/LICENSE
    title: Plan 9 from Bell Labs licence (MIT)
  - id: plan9-names
    resource: https://github.com/plan9foundation/plan9/blob/main/sys/doc/names.ms
    title: The Use of Name Spaces in Plan 9 (Pike, Presotto, Thompson, Trickey, Winterbottom)
  - id: plan9-plumb
    resource: https://github.com/plan9foundation/plan9/blob/main/sys/doc/plumb.ms
    title: Plumbing and Other Utilities (Pike)
  - id: google-style
    resource: https://developers.google.com/style
    title: Google developer documentation style guide
  - id: google-word-list
    resource: https://developers.google.com/style/word-list
    title: Google developer documentation style guide word list
---

# General documentation standard

This standard sets the rules for the prose of every document in this project, for a writer, a person or an agent, who reads it before writing. The model is _The C Programming Language_, by Brian Kernighan and Dennis Ritchie.[^kr] The book describes a whole language in 272 pages. Each of its sentences states one fact in plain words, an example comes before the rule that it shows, and a reason follows only where a reader would ask for one.

The sections run from the document to the sentence, then cover reasons, systems, words, and instructions. The [formatting standard][sokf:documentation-standard-formatting] covers punctuation, headings, text formatting, lists, code, links, numbers, notices, and images.

This standard quotes its examples from the second edition of the book and cites each by section. They remain the authors' and the publisher's copyright. This standard quotes them under fair dealing for criticism and review, and the repository's licence doesn't extend to them. Three examples come from the papers of Plan 9 from Bell Labs, where the book makes no comparable point. The Plan 9 Foundation publishes those papers under the MIT licence.[^plan9-licence] The word rules come from the Google developer documentation style guide, licensed Creative Commons Attribution 4.0 (CC BY 4.0).[^google-style]

- [Precedence](#precedence)
- [Documents](#documents)
- [Paragraphs](#paragraphs)
- [Sentences](#sentences)
- [Reasons](#reasons)
- [Describing a system](#describing-a-system)
- [Words](#words)
- [Instructions](#instructions)

## Precedence

When two sources of style disagree, follow the one that comes first:

1. This standard
2. The [formatting standard][sokf:documentation-standard-formatting]
3. British spelling with _-ise_ endings, and _New Hart's Rules_ for general style
4. The Google developer documentation style guide and its word list[^google-word-list]

## Documents

Judge the document before its paragraphs and its sentences, because a sentence can pass every rule while the document around it does nothing for the reader:

- Open with what the document is for, before any rule or detail. A document that guides a task opens with what it helps the reader do. A document that describes a system, a block, or a promise opens with what it describes. Both then say what they contain. The two openings differ because a reader at work on a task needs to know whether the document serves the task, and a reader who studies a structure needs to know what the structure is. The book's preface opens the first way: "This book is meant to help the reader learn how to program in C. It contains a tutorial introduction to get new users started as soon as possible, separate chapters on each major feature, and a reference manual." (Preface to the first edition)[^kr] Its reference manual opens the second way: "This manual describes the C language specified by the draft submitted to ANSI on 31 October, 1988 …" (A.1)[^kr]
- Write for a reader who meets the document for the first time. That reader has to understand the document as it stands, without the code it describes.
- Put what the subject is first, then how it works, then why it was built that way. The book shows a working program before any rule and the tutorial before the reference manual: "Our aim is to show the essential elements of the language in real programs, but without getting bogged down in details, rules, and exceptions." (1)[^kr]
- Give each section one job, named by its heading. Move misplaced material to the section with that job, because a reader looks for a rule under its heading.
- After the opening, list the sections with a link to each. A reader, an agent most of all, reads the start of a document first and needs to see what it covers.
- Don't count the items of a list that the document shows, such as "its four sections" before the contents list, because the list already shows them, and the count goes stale when an item is added. A count of reasons is the exception that the [Reasons](#reasons) section gives, because a reader can't see the reasons before they are given.
- In a contract, a schema, a glossary, or a criterion, state one fact in each sentence, in the order in which a reader looks things up, as the book's reference manual does: "char \*strcpy(s,ct) copy string ct to string s, including '\0'; return s." (B.3)[^kr]
- Keep the voice of the book's chapters in the architecture, a design, and an introduction, because their reader follows an argument.

## Paragraphs

Build each paragraph around one idea:

- Follow one line of thought, so that each sentence builds on the one before: "Variables and constants are the basic data objects manipulated in a program. Declarations list the variables to be used, and state what type they have and perhaps what their initial values are. Operators specify what is to be done to them." (2)[^kr]
- State the point first, then explain it: "A C program, whatever its size, consists of functions and variables. A function contains statements that specify the computing operations to be done, and variables store values used during the computation." (1.1)[^kr]
- When a reader might take the point in a wrong sense, say what it doesn't mean: "All resources in Plan 9 look like file systems. That does not mean that they are repositories for permanent files on disk, but that the interface to them is file-oriented: finding files (resources) in a hierarchical name tree, attaching to them by name, and accessing their contents by read and write calls."[^plan9-names]
- Use a list for parallel items and prose for a line of thought. Turn a paragraph that defines several things in a row into a list: "The lines are, in order: The source application, the name of the program generating the message. The destination port, the name of the port to which the messages should be sent. The working directory in which the message was generated."[^plan9-plumb]
- Split a paragraph that does two jobs, and cut one that only restates the paragraph before it.

## Sentences

Write each sentence so that a reader understands it on the first reading:

- Give each sentence one job, and make the thing it's about its subject: "A pointer is a variable that contains the address of a variable." (5.1)[^kr]
- Use the verb that says what happens, in the active voice and the present tense: "Each time it is called, getchar reads the next input character from a text stream and returns that as its value." (1.5)[^kr]
- Check that the subject can do what the verb says. Write "The design describes the modules", not "The design gives the modules", because a design describes and gives nothing.
- Use the verb, not a noun made from it: write "sokf validates each document", not "sokf performs the validation of each document".
- Give a sentence at most one relative clause, because each _which_ or _that_ asks the reader to hold one more thing. When a second would follow, start a new sentence.
- Keep most sentences under 20 words, and rewrite one past 30 unless it lists items, because a reader holds one clause at a time.
- Say the point outright: "C is not a big language, and it is not well served by a big book." (Preface)[^kr]
- Say what a thing does, not what it doesn't: write "The harness records a hash of each part that it writes", not "The harness keeps no copy of what it wrote, only a hash of it".
- Say each thing once, in as few words as a whole sentence needs: "Functions break large computing tasks into smaller ones, and enable people to build on what others have done instead of starting over from scratch." (4)[^kr]
- Cut every word and phrase that adds nothing, but keep the articles and verbs of a whole sentence.
- Avoid ambiguous pronouns: give a bare _this_ or _it_ a clear referent.
- Describe a function, a command, or a part in a full sentence with its name as the subject: "The function putchar prints a character each time it is called" (1.5),[^kr] not "Prints a character each time it is called".

## Reasons

Give a reason where a reader would ask why, and nowhere else:

- Give the reason for a design decision, for a rule that would otherwise seem arbitrary, for a limit, and for an instruction with a cost: "Normally you are at liberty to give functions whatever names you like, but 'main' is special - your program begins executing at the beginning of main. This means that every program must have a main somewhere." (1.1)[^kr]
- State a fact without a reason, because a fact is complete in itself: "A text stream is a sequence of characters divided into lines; each line consists of zero or more characters followed by a newline character." (1.5)[^kr] A definition, a criterion, a description of what a thing does, and a step whose order is plain need no _because_.
- Give a reason once, in the document of the decision, and let every later sentence state its fact. A reader who wants the reason follows the link to the decision.
- When you give several reasons, say how many, then give them in order: "Pointers are much used in C, partly because they are sometimes the only way to express a computation, and partly because they usually lead to more compact and efficient code than can be obtained in other ways." (5)[^kr]
- Let most sentences end with their fact. A paragraph in which every sentence ends with _because_ or _so that_ reads as a defence rather than a description.

## Describing a system

When you describe a system, in an architecture, a contract, or a design, follow these rules:

- Say how a thing is built, in the order in which a reader meets its parts: "Input and output uses the read and write system calls, which are accessed from C programs through two functions called read and write. For both, the first argument is a file descriptor." (8.2)[^kr]
- State a design decision, then its advantages: "The plumber has an unusual architecture: it is a language-driven file server. This design has distinct advantages. It makes plumbing easy to add to an existing, Unix-like command environment; it guarantees uniform handling of inter-application messages; it off-loads from those applications most of the work of extracting and dispatching messages; and it works transparently across a network."[^plan9-plumb]
- Give facts and numbers, not claims such as _fast_ or _small_: "on others an int is 16 bits, with a maximum value of 32767, and it would take relatively little input to overflow an int counter." (1.2)[^kr]
- Report a limit or a mistake plainly: "A pointer is a variable, so pa=a and pa++ are legal. But an array name is not a variable; constructions like a=pa and a++ are illegal." (5.3)[^kr]
- Write timelessly: describe what exists, and avoid _currently_, _now_, _new_, and _latest_ unless the time matters.
- Describe a change that isn't built yet only in a paragraph beginning with `Planned:`. The paragraph links the open issue that owns the change. This rule has two reasons: a reader can't tell an old promise from a fact, and the issue closes the promise. Code that doesn't exist yet needs no such paragraph: link the file in the Files section of the open issue, and any document, such as a design or a contract, can then link the file and include it with a sketch of the code.

## Words

Choose words that the reader already knows:

- Introduce a term when you first use it, with its other name where it has one, and define it in the same sentence: "A string constant, or string literal, is a sequence of zero or more characters surrounded by double quotes" (2.3).[^kr]
- Use the plain, ordinary word for a thing. Don't coin a word for something that has a name, and don't stretch a word to mean something else, such as _names_ for _identifies_.
- Define or avoid jargon.
- Spell out an abbreviation on first use, except one that every reader of code knows, such as API, CLI, HTTP, JSON, URL, and YAML.
- Write for a global audience: no idioms, slang, or culture-specific references.
- Don't anthropomorphise software: a program neither _thinks_ nor _wants_.
- Use _for example_, _that is_, and _and so on_, not _e.g._, _i.e._, and _etc._, because not every reader knows the Latin.
- Use British spelling (_organise_, _colour_, _centre_, _licence_ as a noun), so that one search finds every use of a word.
- Use inclusive language, such as _allowlist_ and _denylist_, or _primary_ and _replica_, and avoid gendered defaults and ableist terms.
- Use _after_, not _once_, for a sequence, and _because_, not _since_, for a cause, because _once_ and _since_ each have a second meaning.
- Use _can_ for ability and _might_ for possibility, not _may_, because _may_ also means permission.
- Write _through_, not _via_, _use_, not _utilize_, and _to_, not _in order to_.
- Refer to another part of a document as _preceding_ or _following_, not _above_ or _below_, because a screen reader has no above or below.
- Check the official word list for anything else.[^google-word-list]

## Instructions

When you tell the reader what to do, follow these rules:

- Address the reader as _you_, in the imperative, and give the reason with an instruction that has a cost: "It's bad practice to bury 'magic numbers' like 300 and 20 in a program; they convey little information to someone who might have to read the program later, and they are hard to change in a systematic way." (1.4)[^kr]
- Put the condition before the instruction, because a reader acts as soon as they read the instruction: "To delete a file, click **Delete**", not "Click **Delete** to delete a file".
- Don't call a task _easy_, _simple_, or _just_, because the word tells a reader who finds the task hard that the fault is theirs.
- Don't write _please_, because an instruction is not a favour.
- Write _we_ only for this project's maintainers, because a reader otherwise can't tell who _we_ are.

[^kr]: The C Programming Language, second edition (Kernighan, Ritchie), Prentice Hall, 1988

[^plan9-licence]: Plan 9 from Bell Labs licence (MIT)

[^plan9-names]: The Use of Name Spaces in Plan 9 (Pike, Presotto, Thompson, Trickey, Winterbottom)

[^plan9-plumb]: Plumbing and Other Utilities (Pike)

[^google-style]: Google developer documentation style guide

[^google-word-list]: Google developer documentation style guide word list

<!-- sokf:links -->

[sokf:documentation-standard-formatting]: /knowledge/documentation-standards/formatting.md
