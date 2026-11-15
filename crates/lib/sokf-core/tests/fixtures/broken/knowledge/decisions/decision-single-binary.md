---
type: Decision
id: decision-single-binary
title: Ship one binary
description: The CLI and daemon ship as one binary.
---

# Ship one binary

## Context

Two binaries doubled the release surface and users installed the wrong one.

## Decision

The CLI and daemon ship as one binary; the daemon is a subcommand.

## Consequences

- One artifact to release, sign and version.
- The binary grows by the daemon's size for CLI-only users.
