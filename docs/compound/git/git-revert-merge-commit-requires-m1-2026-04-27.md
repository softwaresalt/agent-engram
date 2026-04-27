---
title: "git revert on merge commits requires --no-edit -m 1"
date: 2026-04-27
category: git
confidence: high
evidence:
  - docs/closure/2026-04-26-034-F-sql-parser-post-merge-closure.md (PR #36 review cycle)
  - docs/closure/2026-04-26-autoharness-tune-v1.3.2-closure.md (PR #31 review cycle)
  - multiple review cycles flagging the same omission
---

## Problem

When a closure artifact documents a rollback using `git revert <sha>`, the command
is incomplete if `<sha>` is a merge commit. Two distinct failure modes exist:

1. Omitting `-m`: Git aborts immediately with:
   `error: commit <sha> is a merge but no -m option was given`
   No editor is opened — the command simply fails.
2. Providing `-m 1` but omitting `--no-edit`: Git opens an interactive editor for
   the revert commit message, making the command non-scriptable.

## Solution

Always use `--no-edit -m 1` for merge commit reverts in rollback procedures:

```text
git revert --no-edit -m 1 <merge-sha>
git push origin main
```

* `-m 1` selects the first parent (mainline) as the tree to revert to
* `--no-edit` suppresses the editor prompt, making the command non-interactive

## When This Applies

When a closure artifact's rollback procedure references a merge commit SHA. Not every
rollback targets a merge commit — single-commit fixes produce non-merge SHAs that can
be reverted with plain `git revert`. Identify the commit type first:

* `git show --no-patch --format="%P" <sha>` — if output has two SHAs (two parents),
  it is a merge commit; use `--no-edit -m 1`.
* If output has one SHA (one parent), it is a regular commit; plain `git revert` works.

In this repo, the top-level feature/chore PR merge commits are always merge commits
(P-009), so rollbacks of those always need `--no-edit -m 1`.

## Detection

Copilot review repeatedly flagged `git revert <sha>` without `-m 1` in closure docs.
This is a reliable pattern — apply `--no-edit -m 1` preemptively when writing rollback
procedures for merge commits.
