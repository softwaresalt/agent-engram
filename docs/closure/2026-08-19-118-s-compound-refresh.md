---
title: "118-S post-merge compound refresh"
date: 2026-08-19
shipment: 118-S
feature: 122-F
mode: apply
---

## Scope

Reviewed only the compound entries directly implicated by linked-worktree
admission, startup bounding, and post-merge closure sequencing.

## Classifications

| Entry | Classification | Evidence |
|---|---|---|
| `dark-mode-single-worktree-disk-admission-gates-2026-08-02.md` | keep | Its one-worktree and cleanup gate remains the broader operational rule around worktree admission. |
| `daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md` | keep | Its startup-order lesson remains valid as historical context for bounded startup. |
| `shipment-done-status-post-merge-closure-repair-2026-08-15.md` | keep | Its post-merge lifecycle repair and archive-verification guidance still applies to shipment closure. |

## Applied Maintenance

No existing compound entry required rewriting, consolidation, replacement, or
archival. The new linked-worktree learning was captured as a separate entry so
the older workflow-issue notes remain intact and sourceable.

## Notes

The refresh scope intentionally excluded unrelated compound entries and any
cosmetic rewrite of the three kept notes.
