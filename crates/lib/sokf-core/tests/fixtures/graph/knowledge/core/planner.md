---
type: Subsystem
id: planner
title: Planner
description: The planner is a pure planning stage.
resource: /crates/lib/example-core/src/planner.rs
links:
  - rel: depends-on
    to: config
    note: Reads logging configuration.
  - rel: references
    to: planer
---

# Planner

Plans from the [configuration][sokf:config]; see also [planer][sokf:planer].

<!-- sokf:include /crates/app/sokf/src/main.rs#cli -->

```rust
fn cli() {}
```

<!-- /sokf:include -->

<!-- sokf:links -->

[sokf:config]: /knowledge/config.md
