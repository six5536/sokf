---
type: Formats
id: sokf-formats
title: SOKF shared formats
resource: /knowledge/formats/sokf.formats.yaml
version: "0.4"
---

# SOKF shared formats

This document describes `sokf.formats.yaml`, so that a schema author can bind a format by name without reading the file. The formats describe the shapes of user stories, acceptance criteria, properties, and the other lines of requirements documents. sokf embeds the file in its binary. Line formats are a feature of the tool sokf, not part of SOKF, the Superdev Open Knowledge Format.

Each entry of the file states the shape of a line, or the shapes of its variants, as the line itself with a typed placeholder wherever the line varies. Each entry also lists the patterns that the line must not contain and an example of each shape. sokf shows the example when a line doesn't fit. The file defines the types of the placeholders too, such as `criterion-id`.

The document covers the types, the formats, how a schema binds a format, and the order in which sokf looks up a format name. `sokf info formats` prints every format and type. Where this document and the file differ, the file is the authority.

- [Types](#types)
- [Formats](#formats)
- [Binding a format](#binding-a-format)
- [Layers](#layers)

## Types

The file defines these types, in addition to the built-in types of the [schema specification][sokf:sokf-spec:sokf-schema-spec] §7:

- **`requirement-code`**: `[A-Z]{1,10}`, the code of a set of requirements or of a contract
- **`requirement-id`**: `CODE-n`, or `CODE-n.p` for a subrequirement
- **`criterion-id`**: `CODE-n_AC-m`
- **`property-id`**: `CODE_P-n`
- **`indefinite-article`**: `A` or `AN`
- **`property-shape`**: `round-trip`, `invariant`, `idempotent`, `order-independent`, `equivalent`, or `metamorphic`
- **`evolution-kind`**: `Versioning`, `Backwards compatibility`, or `Deprecation`
- **`sentence`**: a capital letter, any text, and a full stop
- **`predicate`**: a lowercase letter, any text, and a full stop: the rest of a sentence after its subject
- **`code-name`**: any text without a backtick: a name written in backticks
- **`channel-name`**: any text without a colon

## Formats

`user-story` has the shape `${id:requirement-id}: AS ${:indefinite-article} ${}, I WANT ${}, SO THAT ${}.`, written as one sentence. It prohibits `SHALL`, `SHOULD`, and `MUST`.

`requirement-ears` is the one format for a criterion of the Easy Approach to Requirements Syntax (EARS), `${id:criterion-id} [<tag>]: <clause>.`. It requires the keywords in upper case, `the` in lower case, and a full stop at the end of the clause:

- **`ubiquitous`**: `the <system> SHALL <response>.`
- **`event`**: `WHEN <trigger>, the <system> SHALL <response>.`
- **`state`**: `WHILE <state>, the <system> SHALL <response>.`
- **`unwanted`**: `IF <condition>, THEN the <system> SHALL <response>.`
- **`optional`**: `WHERE <feature>, the <system> SHALL <response>.`
- **`complex-event`**: `[complex]: WHILE <state>, WHEN <trigger>, the <system> SHALL <response>.`
- **`complex-unwanted`**: `[complex]: WHILE <state>, IF <condition>, THEN the <system> SHALL <response>.`

`requirement-ears` permits `SHALL NOT`. It prohibits the modals `should`, `must`, `will`, `may`, `might`, and `can`, a second `SHALL`, and the vague words `fast`, `quickly`, `easy`, `easily`, `user-friendly`, `appropriate`, `as needed`, `etc.`, and `and/or`. It also prohibits the passive `SHALL be <participle>`. A participle is a word that ends in `-ed`, `-en`, or `-wn`, or a common irregular participle such as `sent`, `kept`, or `made`.

`requirement-property` has the shape `${id:property-id} [${:property-shape}]: FOR ANY <inputs>, the <system> SHALL <relation>.` It prohibits what `requirement-ears` prohibits.

`requirement-evolution` has the shape `${:evolution-kind}: ${:sentence}`, and it consists of one or more sentences in the present tense. It prohibits `SHALL`, the modals and vague words of `requirement-ears`, `eventually`, and a passive such as `is <participle>`, with participles as `requirement-ears` defines them.

`requirement-channel` is a line about one input or output channel of a part, written as `${:channel-name}: <what>.`, or as `None.` alone. It prohibits `SHALL` and the modals and vague words of `requirement-ears`.

`requirement-summary` is one full sentence about a part, with the part as its subject. Its variant `named` starts with the name of the part in backticks, as in `` `sokf validate` checks… ``, and its variant `described` starts with `The`, as in `The rules file lets…`. It prohibits `SHALL`, a second sentence, and the modals and vague words of `requirement-ears`.

## Binding a format

A unit rule binds a format with the property `format`. A block rule that reads `each: { format: user-story }` requires every paragraph of the block to be a user story. sokf reports each paragraph that doesn't fit. The following schema binds two of the shared formats, `user-story` and `requirement-ears`:

```yaml
sections:
  - heading: User stories
    level: 2
    rule-id: stories
    guidance: The section has one story for each capability.
    blocks:
      - kind: group
        repeat: true
        blocks:
          - {
              kind: prose,
              max: 1,
              each: { format: user-story, unique: repository },
            }
          - kind: blockquote
            max: 1
            example: The click on a file name takes a user from a compiler error to the code.
  - heading: Acceptance criteria
    level: 2
    blocks:
      - kind: bullet-list
        each:
          format: requirement-ears
          unique: repository
          key-derives-from: { rule: stories, separator: "_" }
```

The property `unique: repository` requires each key to occur once in the whole repository. The property `key-derives-from` requires each criterion key to start with the key of a story from the rule `stories`, followed by `_`.

The following document follows this schema:

```markdown
## User stories

ST-1: AS A user, I WANT a click on a file name to open the file in the editor, SO THAT I go from a compiler error to the code in one click.

> The click on a file name takes a user from a compiler error to the code.

## Acceptance criteria

- ST-1_AC-1 [event]: WHEN a program writes a message with the data `mem.c` and the attribute `addr=27` to `/mnt/plumb/send`, the plumber SHALL deliver the message to the `edit` port.
- ST-1_AC-2 [unwanted]: IF no rule set matches and the message names no port, THEN the plumber SHALL fail the write to `send` with the error `no matching plumb rule`.
```

## Layers

sokf checks documents with tramlines, a validation engine. The core grammar of tramlines, the grammar that every schema follows, defines the shapes and variants of a format, its prohibited patterns, and its layers. §5 of the [tramlines specification](https://github.com/six5536/tramlines/blob/main/knowledge/spec/tramlines-spec.md#5-formats) describes them. sokf looks up a format name in three layers, nearest first, and uses the nearest definition:

- **Schema**: the `formats` and `types` of the schema
- **Repository**: the file `/.sokf/formats.yaml`, which has the same shape as this file
- **Shared**: this file

`sokf info formats --schema <id>` prints the formats and types that apply to a schema. For each entry, it shows the `source`: the layer of the entry's definition.

To change a format for the whole repository, edit `/.sokf/formats.yaml`, because an override in a schema changes the format for that schema alone.

<!-- sokf:links -->

[sokf:sokf-spec:sokf-schema-spec]: /sokf-spec/sokf-schema-spec.md
