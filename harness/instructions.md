If a repo contains `/.sokf/config.toml` it holds SOKF knowledge stores:
- Before doing anything in a knowledge store, read the `sokf` skill.
- Before modifying or to explain a knowledge store document, use `sokf guide <PATH> (--state <KEY>=<STATE>)`, to read the update guidance to follow when writing.
- Before you end a turn, run `sokf validate --fix` and fix every error it reports.
