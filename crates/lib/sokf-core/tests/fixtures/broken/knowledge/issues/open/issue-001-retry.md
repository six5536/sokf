---
type: Issue
id: issue-001-retry
title: Retry fails twice
state: open
kind: bug
links:
  - rel: implements
    to: decision-single-binary
---

# Retry fails twice

The retry of [the one binary][sokf:decision-single-binary] runs twice.

## Reproduction

1. Run the daemon.
2. Kill the network.

<!-- sokf:links -->

[sokf:decision-single-binary]: /knowledge/decisions/decision-single-binary.md
