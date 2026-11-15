---
type: Requirement
id: req-cli
title: CLI requirement
description: What the command line must do.
---

# CLI requirement

## User Stories

<a id="cli-1"></a>CLI-1: AS A developer, I WANT configuration loaded from a file, SO THAT I don't need to specify options every time.

> Enables persistent configuration without repetitive CLI arguments.

## Acceptance Criteria

- <a id="cli-1_ac-1"></a>CLI-1_AC-1 [event]: WHEN `--config <path>` names a readable file, the system SHALL load options from it.
- <a id="cli-1_ac-2"></a>CLI-1_AC-2 [unwanted]: IF the config file fails to parse, THEN the system SHALL report the line and column.
