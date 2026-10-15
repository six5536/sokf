A repository with a `/.sokf/config.toml` holds Superdev Open Knowledge Format (SOKF) knowledge stores, as listed in that file.
Before you read or edit anything in a knowledge store, read the `sokf` skill.
When you end a turn, a hook runs `sokf validate --fix` on the knowledge stores; if no hook runs it, run it yourself before you finish. Fix every error that `sokf validate --fix` reports.
