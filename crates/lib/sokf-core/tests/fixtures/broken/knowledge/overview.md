---
type: Overview
id: overview
title: Overview
description: Reads the requirement.
links:
  - rel: references
    to: req-cli
---

# Overview

The CLI requirement is [CLI-1][sokf:#cli-1] in [the criteria][sokf:req-cli#acceptance-criteria].

<!-- sokf:include req-cli -->

# CLI requirement

## User Stories

CLI-1: AS A developer, I WANT configuration loaded from a file, SO THAT I don't need to specify options every time.

> Enables persistent configuration without repetitive CLI arguments.

## Acceptance Criteria

- CLI-1_AC-1 [event]: WHEN `--config <path>` names a readable file, the system SHALL load options from it.
- CLI-1_AC-2 [unwanted]: IF the config file fails to parse, THEN the system SHALL report the line and column.

<!-- /sokf:include -->

<!-- sokf:links -->

[sokf:#cli-1]: /knowledge/req-cli.md#cli-1
[sokf:req-cli#acceptance-criteria]: /knowledge/req-cli.md#acceptance-criteria
