---
type: Recipe
id: recipe-rewrite-a-document
title: Rewrite a document from the top down
description: The instructions that an agent follows to rewrite a badly written document, from the whole document down to its sentences, and the checks to run afterwards.
---

# Rewrite a document from the top down

With this recipe, an agent rewrites a document whose English is poor so that it follows the documentation standards. It covers two passes, a top-down rewrite and a sentence sweep, and the checks to run after them. Use it on one document at a time, and read the result yourself before you commit it. An agent that follows the recipe fixes the structure of a document as well as its sentences, as it did on the sokf skill, the agent reference for the sokf command.

- [The instructions](#the-instructions)
- [The sentence sweep](#the-sentence-sweep)
- [Why the order matters](#why-the-order-matters)
- [After the rewrite](#after-the-rewrite)

## The instructions

Give a fresh agent these instructions, with the documentation standards as its only other instructions:

```text
Rewrite the English of FILE so that it follows the documentation standards you were given as your instructions (general, formatting, and schema documentation standards). Work from the top down, in this order, and finish each level before the next:

1. The document. The reader of this file is READER. Decide what that reader needs first, and in what order. Check that each section is in the right place and that the document opens with a real introduction: what the subject is, and what this document helps the reader do. Move material that is in the wrong place, such as rules sitting in the introduction, to where it belongs.
2. Each section and each paragraph. Give each paragraph one idea, with its point first. Introduce each term before you use it. Split a paragraph that does two jobs.
3. Each sentence. Only now fix sentences: use plain verbs in their ordinary meaning, give each sentence a clear subject that does what the verb says, and expand compressed phrases that need a second reading.

Rules:
- Keep every fact, command, option, key, rule, and example. Don't add claims. If you're unsure what something means, read the code or the specifications that the document describes before you change it.
- Keep the frontmatter keys, every heading or line that a test or a schema checks, and every code block's content except comments.
- Edit only this file. Don't build, test, or commit.

Return a short report, because git records every change: the structural changes in a few lines (what moved where, and why), each fact that you corrected against the code, and anything you were unsure of. Don't list sentence changes.
```

Replace `FILE` with the path of the document, and `READER` with who reads it and why.

## The sentence sweep

The top-down rewrite leaves some sentence-level habits behind. Give a fresh agent these instructions, and run them again until a pass changes little. On the design schema, the second and third passes each found a dozen more sentences.

```text
Check each sentence of the English in FILE against the documentation standards you were given as your instructions. Don't change the structure: an earlier pass has already reorganised it. Read every sentence, and for each one ask:

1. Does the subject really do what the verb says? A design doesn't "give" its parts. It describes or explains them. Check every verb whose subject is a thing, such as SUBJECTS: gives, shows, holds, carries, names, takes, keeps, sees, lives, sits, decides, moves, pulls, and so on. Use the verb that says what happens.
2. Is any word stretched past its ordinary meaning, or is any word chatty or informal, such as "pulls in" or "remedy"?
3. Does any phrase need a second reading because it is compressed?
4. Does a passive sentence hide who acts, where naming the actor would be clearer? Does any pronoun, such as "them" or "it", lack a clear referent?
5. Is the sentence in the voice that the standards require for this kind of text?
6. Does the sentence say anything that the text around it doesn't already say? If it doesn't, delete it.
7. Is each noun the exact word for the thing it refers to, and does it avoid a word that the glossary defines with another meaning?
8. Does the sentence say what something doesn't do, where it could say what it does? Write "The harness records a hash of each part that it writes", not "The harness keeps no copy of what it wrote, only a hash of it".
9. Can a word, a phrase, or a whole sentence go without losing meaning or grammar? Cut it, but keep the articles and verbs that make the sentence whole.

Change only the sentences that fail, and keep every fact and rule. When you rewrite a sentence, check your new sentence against the same questions before you keep it. Don't replace one fault with another. Some sentences have a shape that the validator or a format requires, such as a one-sentence summary with a named system as its subject. Keep those sentences in that shape. Never change anything that a test or the validator reads. Edit only this file. Don't build, test, or commit.

Return a short report, because git records every change: the number of sentences that you changed, and, as before-and-after pairs, only the changes that could alter a meaning or that you were unsure of.
```

Replace `FILE` with the path of the document, and `SUBJECTS` with the kinds of thing the document talks about, such as a design, a section, a list, or a diagram.

A sweep can also lose meaning while it fixes a verb. On the design schema, "one part moves into its own design" became "one of them has its own design", and the rewrite dropped the move. Later, "one module is moved into a design of its own rather than left out of the diagram" became "one module belongs in a design of its own rather than in the diagram". That rewrite dropped the choice against leaving the module out, and it reads as if the module should not be in the diagram. Read the diff of each sweep for meaning, not only for style, and check that the contrast or the cause in the original survives.

## Why the order matters

The [general documentation standard][sokf:documentation-standard-general#documents] asks you to judge the document before its paragraphs and its sentences. A rewrite that works sentence by sentence leaves a badly organised document badly organised. The old opening of the sokf skill showed this fault: its sentences were correct, but it opened with a rule about dates in frontmatter instead of an introduction. An agent that judges the document first, then each paragraph, then each sentence, catches both kinds of fault.

## After the rewrite

Read the result before you commit it:

1. Read the opening paragraph, and check that it introduces the subject before it gives any rule.
2. Read each sentence whose subject is a thing, such as a format, a file, or a key, and check that the thing really does what the verb says. "SOKF keeps knowledge" fails, because a format keeps nothing.
3. Look for a paragraph that defines many terms in a row, and turn it into a list.
4. Look for words stretched past their meaning, such as "decide", "names", "carries", or "lives".
5. Read each claim about how something is built and check it against the code, because a sentence can pass every question of the sentence sweep and still be false. On the CLI design, "the parser's definition is the only source of the man page" read well. But `man.rs` renders the parser's definition and then appends hand-written roff, so the page comes from two sources. A sweep judges the English. Only a reader who checks the code catches a claim that the code contradicts.
6. Run the tests and `sokf validate`, and fix what they report.

<!-- sokf:links -->

[sokf:documentation-standard-general#documents]: /knowledge/documentation-standards/general.md#documents
