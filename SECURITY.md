# Security policy

This policy helps you report a vulnerability in sokf safely. It says which
reports are in scope, how to report a vulnerability privately, and which
versions receive security fixes.

- [Scope](#scope)
- [Reporting a vulnerability](#reporting-a-vulnerability)
- [Supported versions](#supported-versions)

## Scope

sokf moves, links, and, under `conflict = "replace"`, deletes files, so its
safety model is part of its security. sokf follows these rules:

- It touches only the files that you name, with one exception.
- It refuses to change anything under a protected root.
- It runs as root only with `--allow-root`.
- It asks you to confirm a delete that can't be undone.

The exception is `backup_keep`. `backup_keep` scans a target's parent
directory for that target's own backups, which are named `NAME.TIMESTAMP.bak`.
`NAME` is the target's filename, and `TIMESTAMP` is the time of the backup.
The scan finds the backups so that `backup_keep` can delete the oldest. Those
backups are the only files that the scan can delete.

The "Safety" section of the README documents the whole model. Any report that
shows a way around these rules is in scope.

## Reporting a vulnerability

Report a security issue privately, through GitHub's private vulnerability
reporting:

1. Open the [Security tab](https://github.com/six5536/sokf/security) of the
   repository.
2. To start a private advisory, click **Report a vulnerability**.

Don't open a public issue for a suspected vulnerability, because a public issue
discloses it before a fix exists.

The maintainer acknowledges the report and agrees a fix and a disclosure date
with you.

## Supported versions

sokf is pre-1.0. Security fixes go into the latest release and the `main`
branch only. To pick up a fix, upgrade to the latest release.
