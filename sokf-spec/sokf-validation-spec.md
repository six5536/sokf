---
type: SokfValidationSpecification
id: sokf-validation-spec
title: "SOKF: Validation specification"
description: How a validator checks a knowledge store, what it reports, and what its fix repairs.
version: "0.6"
status: draft
---

# SOKF: Validation specification

This specification defines validation in the Superdev Open Knowledge Format (SOKF): what a validator checks, reports, and repairs.

SOKF is a format for storing canonical knowledge, which agents and people both read and maintain.

A validator reads every document of a knowledge store and reports each rule that a document breaks. Its fix repairs the mechanical errors, so that a writer corrects only the errors that need judgement.

- [About this specification](#about-this-specification)
- [1. Terminology](#1-terminology)
- [2. Document check](#2-document-check)
- [3. Fix](#3-fix)

## About this specification

Each normative rule of this specification is a criterion in the EARS (Easy Approach to Requirements Syntax) form, and each has a key of the form `SOKF_VALIDATION_N`, where `N` is the number of the criterion.

## 1. Terminology

This specification uses the terms of the [format specification][sokf:sokf-format-spec] and the [schema specification][sokf:sokf-schema-spec], and adds these:

- <a id="document-check"></a>**document check**: The deterministic check that the validator runs on every file in the working tree (§2).
- <a id="fix"></a>**fix**: The automatic repair of mechanical errors, which a validator runs before the document check (§3).
- <a id="failure"></a>**failure**: A finding that a knowledge store breaks a criterion, so that the store does not conform.
- <a id="warning"></a>**warning**: A finding that leaves a knowledge store conforming.

## 2. Document check

The validator runs one deterministic check, the document check, on the working tree. The check covers each region of a file that an include block includes. It also reads the markers in the repository files outside every knowledge store ([format specification][sokf:sokf-format-spec] §9.5).

- <a id="sokf_validation_1"></a>SOKF_VALIDATION_1 [ubiquitous]: the validator SHALL report a failure for each criterion of the [format specification][sokf:sokf-format-spec] and the [schema specification][sokf:sokf-schema-spec] that the knowledge store breaks.
- <a id="sokf_validation_45"></a>SOKF_VALIDATION_45 [ubiquitous]: the validator SHALL report each finding with the path of its file, its line where one applies, and its severity.
- <a id="sokf_validation_46"></a>SOKF_VALIDATION_46 [ubiquitous]: the validator SHALL state in each finding the criterion that the knowledge store breaks, or the schema and the schema rule that it breaks.
- <a id="sokf_validation_47"></a>SOKF_VALIDATION_47 [ubiquitous]: the validator SHALL identify a schema rule by its `rule-id` or, when the rule has none, by its section and block position, its frontmatter key, or its `rel`.
- <a id="sokf_validation_48"></a>SOKF_VALIDATION_48 [ubiquitous]: the validator SHALL report each finding once.
- <a id="sokf_validation_49"></a>SOKF_VALIDATION_49 [unwanted]: IF the source of an include block contains an include that does not resolve, THEN the validator SHALL report a failure on the including file.
- <a id="sokf_validation_2"></a>SOKF_VALIDATION_2 [ubiquitous]: the validator SHALL treat LF and CRLF as the same line ending in each comparison.
- <a id="sokf_validation_3"></a>SOKF_VALIDATION_3 [unwanted]: IF a manifest declares a `sokf` version that the validator does not implement, THEN the validator SHALL report a failure.
- <a id="sokf_validation_4"></a>SOKF_VALIDATION_4 [unwanted]: IF a link uses a custom relationship, THEN the validator SHALL report a warning.
- <a id="sokf_validation_6"></a>SOKF_VALIDATION_6 [unwanted]: IF a document breaks a schema rule or entry that declares `severity: warn`, THEN the validator SHALL report a warning.
- <a id="sokf_validation_7"></a>SOKF_VALIDATION_7 [unwanted]: IF a document contains a `sokf:optional` region, THEN the validator SHALL report a warning.
- <a id="sokf_validation_40"></a>SOKF_VALIDATION_40 [unwanted]: IF a scaffold omits a part that its schema allows, THEN the validator SHALL report a warning on the schema.
- <a id="sokf_validation_41"></a>SOKF_VALIDATION_41 [unwanted]: IF the schema of a document does not conform to the grammar, THEN the validator SHALL report a failure on the document and run none of its schema checks.
- <a id="sokf_validation_8"></a>SOKF_VALIDATION_8 [unwanted]: IF a unit does not match a template, THEN the validator SHALL report the column where the text stops matching.
- <a id="sokf_validation_9"></a>SOKF_VALIDATION_9 [unwanted]: IF a unit does not match a template, THEN the validator SHALL report what the template expects at that column.
- <a id="sokf_validation_10"></a>SOKF_VALIDATION_10 [unwanted]: IF a unit does not match a template, THEN the validator SHALL report the example of the template.
- <a id="sokf_validation_11"></a>SOKF_VALIDATION_11 [ubiquitous]: the validator SHALL check an asset and its sidecar using only their filenames, their frontmatter, the schemas, and whether files exist.
- <a id="sokf_validation_12"></a>SOKF_VALIDATION_12 [ubiquitous]: the validator SHALL read the content of an asset only for an include block that refers to it.
- <a id="sokf_validation_13"></a>SOKF_VALIDATION_13 [ubiquitous]: the validator SHALL write no scaffold into a Schema document.
- <a id="sokf_validation_50"></a>SOKF_VALIDATION_50 [ubiquitous]: the validator SHALL read for markers each repository file outside every knowledge store, except a git-ignored or excluded file, or one with a NUL byte in its first 8 KB.
- <a id="sokf_validation_51"></a>SOKF_VALIDATION_51 [unwanted]: IF a marker uses a custom relationship, THEN the validator SHALL report a warning.
- <a id="sokf_validation_52"></a>SOKF_VALIDATION_52 [unwanted]: IF the first word of a line matches the tag of a marker only when the comparison ignores case, THEN the validator SHALL report a failure.

The document check reports each reference to a former path that the fix (§3) does not rewrite, such as a former path in a `resource` field.

## 3. Fix

The fix repairs mechanical errors, so that the document check reports only the errors that the fix cannot repair. A validator that offers the fix runs it before the document check.

- <a id="sokf_validation_14"></a>SOKF_VALIDATION_14 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite each stale definition block and each stale include block, in dependency order.
- <a id="sokf_validation_43"></a>SOKF_VALIDATION_43 [event]: WHEN the validator rewrites a definition block, the validator SHALL remove each line that follows the block.
- <a id="sokf_validation_15"></a>SOKF_VALIDATION_15 [event]: WHEN the validator fixes a tree, the validator SHALL expand each include block from the content that its source has after the fix.
- <a id="sokf_validation_16"></a>SOKF_VALIDATION_16 [unwanted]: IF the source of a block does not resolve or lies on a cycle, THEN the validator SHALL keep the block as it is.
- <a id="sokf_validation_17"></a>SOKF_VALIDATION_17 [event]: WHEN the validator fixes a tree, the validator SHALL move each document, keeping its filename, to the directory that the path-enforced states of the document determine.
- <a id="sokf_validation_54"></a>SOKF_VALIDATION_54 [event]: WHEN the validator fixes a tree, the validator SHALL move each document that lies outside the `root` of a schema with no path-enforced dimension, keeping its filename, into that root.
- <a id="sokf_validation_18"></a>SOKF_VALIDATION_18 [event]: WHEN the validator fixes a tree, the validator SHALL move a sidecar together with its asset, each keeping its filename.
- <a id="sokf_validation_19"></a>SOKF_VALIDATION_19 [event]: WHEN the validator fixes a tree, the validator SHALL regenerate each definition that points to the former path of a moved file.
- <a id="sokf_validation_20"></a>SOKF_VALIDATION_20 [unwanted]: IF a move by the fix would overwrite a file, THEN the validator SHALL leave every file of that move in place.
- <a id="sokf_validation_21"></a>SOKF_VALIDATION_21 [unwanted]: IF a move by the fix would overwrite a file, THEN the validator SHALL report a failure on the document.
- <a id="sokf_validation_22"></a>SOKF_VALIDATION_22 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite the `resource` of each sidecar to the root-relative path (starting with `/`) of its asset.
- <a id="sokf_validation_23"></a>SOKF_VALIDATION_23 [event]: WHEN the validator rewrites the `resource` of a sidecar, the validator SHALL leave every other byte of the frontmatter unchanged.
- <a id="sokf_validation_24"></a>SOKF_VALIDATION_24 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite each reference in its canonical form ([format specification][sokf:sokf-format-spec] §8), keeping its target.
- <a id="sokf_validation_25"></a>SOKF_VALIDATION_25 [event]: WHEN the validator fixes a tree, the validator SHALL replace each heading that a schema lists under `renamed-from` with the current heading.
- <a id="sokf_validation_26"></a>SOKF_VALIDATION_26 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite each heading, paragraph, or item bound by `{ from: KEY }` to match the value of the frontmatter key `KEY`.
- <a id="sokf_validation_27"></a>SOKF_VALIDATION_27 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite each setext heading as an ATX heading.
- <a id="sokf_validation_28"></a>SOKF_VALIDATION_28 [event]: WHEN the validator fixes a tree, the validator SHALL write each generated anchor where the [format specification][sokf:sokf-format-spec] §9 places it.
- <a id="sokf_validation_29"></a>SOKF_VALIDATION_29 [event]: WHEN the validator fixes a tree, the validator SHALL remove each anchor that matches no key, and each duplicate anchor.
- <a id="sokf_validation_30"></a>SOKF_VALIDATION_30 [event]: WHEN the validator fixes a tree, the validator SHALL preserve the anchors of a file through every other rewrite.
- <a id="sokf_validation_31"></a>SOKF_VALIDATION_31 [event]: WHEN the validator fixes a tree, the validator SHALL leave each `sokf:guidance` line as it is.
- <a id="sokf_validation_42"></a>SOKF_VALIDATION_42 [event]: WHEN the validator fixes a tree, the validator SHALL write no `sokf:guidance` line.
- <a id="sokf_validation_32"></a>SOKF_VALIDATION_32 [event]: WHEN the validator fixes a tree, the validator SHALL replace each fragment that uses the text of a key or heading with the slug of that key or heading.
- <a id="sokf_validation_33"></a>SOKF_VALIDATION_33 [event]: WHEN the validator fixes a tree, the validator SHALL write each missing index and rewrite each stale index, after the moves.
- <a id="sokf_validation_55"></a>SOKF_VALIDATION_55 [event]: WHEN a move by the fix creates a directory, the validator SHALL rewrite the index of each directory that gains the new directory as an entry.
- <a id="sokf_validation_34"></a>SOKF_VALIDATION_34 [event]: WHEN the validator fixes a tree, the validator SHALL create no file other than an index.
- <a id="sokf_validation_53"></a>SOKF_VALIDATION_53 [event]: WHEN the validator fixes a tree, the validator SHALL rewrite no marker.
- <a id="sokf_validation_35"></a>SOKF_VALIDATION_35 [event]: WHEN the validator rewrites a file, the validator SHALL use the line ending of its first line, or LF if the first line has none.
- <a id="sokf_validation_36"></a>SOKF_VALIDATION_36 [event]: WHEN the validator creates a file, the validator SHALL use LF line endings.
- <a id="sokf_validation_37"></a>SOKF_VALIDATION_37 [event]: WHEN the validator fixes a tree that its previous run already fixed, the validator SHALL rewrite no file.
- <a id="sokf_validation_38"></a>SOKF_VALIDATION_38 [event]: WHEN the validator ends a fix, the validator SHALL report every move, every file that it created, and every file that it rewrote.
- <a id="sokf_validation_39"></a>SOKF_VALIDATION_39 [event]: WHEN the validator ends a fix, the validator SHALL run the document check on the tree as the fix left it.

The fix rewrites these kinds of reference into canonical form ([format specification][sokf:sokf-format-spec] §8):

- **Store prefix**: The fix rewrites a label with the shortest store prefix when its prefix is longer than needed or is a former ID of the store.
- **Bare fragment**: The fix adds the prefix of the other store to a bare fragment that refers to one of that store's keys.
- **Path link**: The fix rewrites an inline path link to a document or an asset, or an inline image of an asset, as a `sokf:` label.
- **Link target**: The fix replaces the `to` of a link with the `id` of its target when the `to` is on a line of its own.
- **Include marker**: The fix replaces the path of an asset in an include marker with the `id` of the asset's sidecar.

The fix replaces the `resource` line of a sidecar, or adds one after the `id` line when the sidecar has none. The fix leaves the path in an include marker as it is when the marker includes a document.

<!-- sokf:links -->

[sokf:sokf-format-spec]: /sokf-spec/sokf-format-spec.md
[sokf:sokf-schema-spec]: /sokf-spec/sokf-schema-spec.md
