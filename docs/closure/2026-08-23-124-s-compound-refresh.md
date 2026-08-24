---
title: "124-S compound refresh — knowledge graduation"
doc_type: closure
date: 2026-08-23
shipment_id: "124-S"
status: done
---

## Candidate Assessment

| Candidate learning | Graduate? | Disposition |
|---|---|---|
| Gitignored gate evidence blocks post-merge `shipment ship` | Already exists | **Refreshed** existing doc with a better recovery method |
| Merge-commit-only verification by parent count | Fold into existing | Captured in closure + memory; too small for a standalone learning |
| Merge-tree equivalence justifies skipping a redundant rebuild | Fold into existing | Captured in runtime verification + memory |
| Cozo cold-start degrades Engram diagnostics | Not a learning | Open defect; evidence attributed to spike `002-SP` |
| Pre-initialize compat window + rollback runbook | Already shipped | Documented in-tree by commit `3d90d5a7` |

No new compound document was created. Creating one would duplicate an existing
learning rather than add knowledge.

## Refreshed Learning

`docs/compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`

The 2026-08-02 doc (from 102-S) correctly diagnosed the cause but prescribed
only one recovery: regenerate gate evidence by cycling each task
`active` → `done` on the post-merge branch. This session found a strictly
better path and the doc now reflects it.

### What changed

1. **New preferred recovery — port the original evidence.** When the
   implementation worktree still exists, copy `.backlogit/logs/*` from it into
   the post-merge worktree. This preserves the authentic
   `pre_task_completion_gate_passed` events and the real `head_sha` each gate
   actually ran against (here `3d90d5a7…`). Because the path is gitignored,
   the copy leaves `git status --short` empty — verified before and after.

2. **Regeneration demoted to fallback**, for when the implementation worktree
   is gone, with its tradeoff now stated explicitly: regeneration rewrites
   `updated_at` and stamps a closure-time `head_sha`, so the archived history
   stops showing when the work actually passed its gate.

3. **Two guardrails added**: do not "fix" this by un-ignoring
   `.backlogit/logs/` (local runtime evidence; versioning it would generate
   merge conflicts across parallel shipments), and run the whole closure from
   an isolated `post-merge/` worktree whenever the primary checkout holds
   unrelated operator changes.

4. **Evidence extended** with `shipment: 124-S`, `pr: 359`,
   `merge_commit: 8f9904a0`, so the doc now cites two independent
   reproductions (102-S and 124-S) — raising it from single-instance to
   recurring-pattern status.

## Why This Matters

The refusal message (`130.001-T remains active`) contradicts the artifact's
own `status: done`, which invites an agent to "repair" a status that was never
broken. Recording the correct diagnosis and the non-destructive recovery
prevents future sessions from mutating archived backlog state or reaching for
`--force-gates`.
