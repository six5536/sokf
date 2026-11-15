---
type: DocumentationStandard
id: documentation-standard-formatting
title: Formatting documentation standard
description: How to punctuate and format the prose of this project's documents, condensed from the Google developer documentation style guide.
sources:
  - id: google-style
    resource: https://developers.google.com/style
    title: Google developer documentation style guide
---

# Formatting documentation standard

This standard sets the rules for the punctuation, headings, text formatting, lists, code, user interface (UI) elements, links, numbers, notices, and images of every document, so that a writer formats each document the same way. The rules are condensed from the Google developer documentation style guide, licensed Creative Commons Attribution 4.0 (CC BY 4.0).[^google-style] The [general documentation standard][sokf:documentation-standard-general] covers the prose itself and takes precedence over this one.

- [Punctuation](#punctuation)
- [Capitalisation and headings](#capitalisation-and-headings)
- [Text formatting](#text-formatting)
- [Lists, procedures, and tables](#lists-procedures-and-tables)
- [Code, commands, and placeholders](#code-commands-and-placeholders)
- [UI elements](#ui-elements)
- [Links and cross-references](#links-and-cross-references)
- [Numbers, dates, and units](#numbers-dates-and-units)
- [Notes and notices](#notes-and-notices)
- [Images](#images)

## Punctuation

Follow these rules for punctuation in prose, but not in code, commands, or literal strings:

- Use the Oxford comma before the last item of a list of three or more, _red, green, and blue_, because the comma stops the last two items from reading as a pair.
- Use an em dash without spaces, and prefer a comma, a colon, or a new sentence, because a dash interrupts the sentence.
- Don't use an ampersand (&) for _and_, including in a heading, unless you quote a label that uses one.
- Avoid semicolons and slashes where a simpler construction works. Rephrase "and/or", for example.
- Put full stops and commas outside quotation marks unless they belong to the quoted text (British style).
- Keep quotation marks and end punctuation outside link text.
- Use no exclamation marks.
- Write a contraction (_don't_, _it's_) or its full form as the sentence reads best, and avoid an awkward one (_it'll_, _should've_). A contract, a criterion, a glossary entry, and a specification use no contractions, because their voice is a rule's.

## Capitalisation and headings

Write titles and headings like this:

- Use sentence case for titles, headings, and navigation, because it reads as prose and leaves no word to decide about.
- Start a task heading with a bare verb ("Create a bucket"), and give a conceptual heading a short noun phrase ("Error location").
- Don't skip heading levels, and don't leave a heading empty or followed at once by another heading.
- Don't capitalise a term because it's important. Capitalise a product name as its makers do.

## Text formatting

Format text for its meaning, not for decoration:

- Use bold only for a label that starts a paragraph, such as **Note:**, and for the name of a UI element. Bold draws the eye, and it loses that force when it's common.
- Write bold with `**` and italic with `_`, so that every document uses one marker for each.
- Use italic sparingly: to introduce or define a term, for a word used as a word, for the title of a full-length work, for a mathematical variable, and for a version variable (1.4._x_).
- Let the wording carry emphasis. Where you need more, use italic, not bold. Underline nothing, because a reader takes underlined text for a link.
- Use code font for a filename, a path, a command, console output, user input, a placeholder, an HTTP status code, and the name of a class, a method, or a variable.
- Use a fenced code block for a sample and for multi-line code.
- Don't set the font, size, or colour of text yourself. Use Markdown's own formatting instead.

## Lists, procedures, and tables

Choose the form that fits the content:

- Use a numbered list for steps that happen in order, and a bulleted list for items in no particular order.
- Introduce a list in prose with a complete sentence ending in a colon, because the reader needs to know what the items are. A contents list, and a list that is the whole body of a section, are part of the document's structure, like its headings, and need no sentence: the opening introduces the one and the heading the other.
- Keep list items parallel in structure, capitalise the first word, and end each item with a full stop if the items are sentences.
- Give each step of a procedure one action, or closely related actions.
- Don't number a procedure that has only one step.
- Use a table for data with several dimensions, with introductory text and a header row, and never for layout.

## Code, commands, and placeholders

Write code, commands, and placeholders so that a reader can copy them:

- Write a placeholder in `UPPER_SNAKE_CASE` and code font, and explain each one after the sample, because the case marks the word that the reader replaces.
- Make code samples correct, runnable, and minimal, and follow the language's own style guide.
- Leave the shell prompt out of a command that a reader copies, unless the output is shown too, because a copied prompt is a failed command.
- Use reserved example domains and names (`example.com`, `example.org`, `example.net`) and the Internet Protocol (IP) ranges that a Request for Comments (RFC) reserves.

## UI elements

Refer to the elements of a user interface by their exact names:

- Write the name of a UI element in bold, matching the UI's exact text and capitalisation.
- Use _click_ for buttons and links, _select_ for options and checkboxes, _enter_ for typed input, and _turn on_ and _turn off_ for toggles.
- Describe an element by name, not by position or appearance ("Click **Save**", not "Click the blue button on the right").

## Links and cross-references

Make each link tell the reader where it leads:

- Use descriptive link text that makes sense out of context, never "click here" or "this page".
- Match link text to the target page title, or describe the destination.
- Don't put a link in a heading, because a heading is itself a link target.

## Numbers, dates, and units

Write numbers, dates, and units so that no reader can misread them:

- Spell out zero to nine in body text, and use numerals from 10. Use numerals for every measurement, version, and technical value, because a numeral stands out where a reader scans for a value.
- Write a date unambiguously, such as "15 January 2026", never in an all-numeric form like 01/02/2026.
- Put a space between a number and its unit (`10 GB`), except before `%` and `°`.
- Use 24-hour times, and give the time zone when readers are in more than one.

## Notes and notices

Use a notice only for something the reader mustn't miss:

- Use a notice (**Note:**, **Caution:**, **Warning:**, **Important:**) sparingly, and don't stack them.
- Reserve **Warning:** for the risk of data loss, a security issue, or harm.

## Images

Make each image useful to every reader, including one who can't see it:

- Give every image alt text that says what it shows, and put no essential text only in an image. A reader who can't see the image has only the text.
- Don't rely on colour, position, or an image alone to convey meaning.
- Prefer a high-resolution or vector image.
- Show code in a code block, never as an image.

[^google-style]: Google developer documentation style guide

<!-- sokf:links -->

[sokf:documentation-standard-general]: /knowledge/documentation-standards/general.md
