---
type: Decision
id: decision-single-binary
title: Ship one binary
description: The CLI and daemon ship as one binary.
---

Ship one binary
===============

## Background

Two binaries doubled the release surface and users installed the wrong one.

## Decision

The CLI and daemon ship as one binary. The daemon is a subcommand.

## Consequences

- The project releases, signs, and versions one artefact.
