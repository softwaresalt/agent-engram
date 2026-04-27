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
is incomplete if `<sha>` is a merge commit. Running it without flags causes two issues:

1. Git aborts with: `error: commit <sha> is a merge but no -m option was given`
2. Even if it did not error, Git would open an editor to capture the revert commit message

## Solution

Always use `--no-edit -m 1` for merge commit reverts in rollback procedures:

```text
git revert --no-edit -m 1 <merge-sha>
git push origin main
```

* `-m 1` selects the first parent (mainline) as the tree to revert to
* `--no-edit` suppresses the editor prompt, making the command non-interactive

## When This Applies

Whenever a closure artifact's rollback procedure references a merge commit SHA.
All PRs in this repo use merge commits (P-009), so every closure artifact's rollback
command should use this form.

## Detection

Copilot review repeatedly flagged `git revert <sha>` without `-m 1` in closure docs.
This is a reliable pattern — apply `--no-edit -m 1` preemptively when writing rollback
procedures for merge commits.
