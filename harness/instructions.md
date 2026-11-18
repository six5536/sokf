A repository with a `/.sokf/config.toml` holds Superdev Open Knowledge Format (SOKF) knowledge stores, as listed in that file.
Before you read or edit anything in a knowledge store, read the `sokf` skill.
Before modifying a knowledge store document, use `sokf guide <PATH>` to read the update guidance to follow when writing. Add `--state <KEY>=<STATE>` when changing document state.
Before you end a turn, run `sokf validate --fix` and fix every error it reports. A hook may tell you that errors remain; fix them before you finish the work.
