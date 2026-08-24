---
title: "Regenerate ignored task gate evidence before post-merge shipment archival"
doc_type: learning
source: "102-S post-merge closure"
description: >-
  A clean post-merge worktree lacks ignored task gate logs; regenerate evidence
  through normal active-to-done transitions before shipping the shipment.
category: workflow-issues
date: 2026-08-02
confidence: high
evidence:
  - shipment: 102-S
  - pr: 307
  - merge_commit: 89ce5419
  - shipment: 124-S
  - pr: 359
  - merge_commit: 8f9904a0
tags: [backlogit, ship-agent, post-merge, gate-evidence, worktree]
---

## Finding

Backlogit records `pre_task_completion_gate_passed` events under
`.backlogit/logs/`, which is intentionally ignored by Git. A clean post-merge
worktree therefore has the archived task Markdown but not the local gate events
created on the implementation worktree. `backlogit shipment ship` correctly
fails closed when those events are absent.

## Safe Recovery

Prefer **porting the original evidence**; fall back to regeneration only when
the implementation worktree is gone.

### Preferred: port the original gate evidence (124-S, 2026-08-23)

If the implementation worktree still exists, port its gate logs into the
post-merge worktree. Backlogit event streams are append-only, tool-managed
history (see the Data Ownership Rule in
`.github/instructions/backlogit.instructions.md`), so the port MUST fail closed
instead of replacing an existing destination stream:

```text
$src  = '<impl-worktree>\.backlogit\logs'
$dest = '<post-merge-worktree>\.backlogit\logs'

# Fail closed: never overwrite existing event history.
if (Test-Path $dest) {
    if (@(Get-ChildItem -Path $dest -Recurse -File).Count -gt 0) {
        throw "Destination already holds event history; port aborted. Use a supported backlogit merge/import path."
    }
} else {
    New-Item -ItemType Directory -Path $dest | Out-Null
}

# No -Force: the empty-destination precondition above is the only guard that
# makes this copy safe, and the command must stay non-overwriting.
Copy-Item -Path "$src\*" -Destination $dest -Recurse
```

Requiring an empty destination keeps the operation idempotent-safe: a rerun
after closure-time events exist aborts instead of erasing them. If the
destination is already populated and logs are still missing, do not copy over
it — reconcile through a supported backlogit merge/import path.

This preserves the **authentic** `pre_task_completion_gate_passed` events
recorded at execution time, including the real `head_sha` each gate actually
ran against. Because `.backlogit/logs/` is gitignored, the copy produces no
working-tree change — verify with `git status --short` before and after.

### Fallback: regenerate through the state machine

Only when the implementation worktree is unavailable, revalidate each
already-merged task on the clean post-merge branch:

```text
backlogit move <task-id> --status active
backlogit move <task-id> --status done --json
```

The second command runs the configured pre-task-completion gate against the
merged tree and records fresh passing evidence. After every member has passing
evidence, run `backlogit shipment ship` with the verified merge SHA.

Note the tradeoff: regeneration rewrites `updated_at` and records a `head_sha`
from closure time rather than execution time, so the archived history no longer
shows when the work actually passed its gate. Port first, regenerate second.

## Guardrails

- Never use `--force-gates` to manufacture closure evidence.
- Never port gate logs with `Copy-Item -Force` or any other overwriting copy
  into a non-empty `.backlogit/logs/`. Event streams are append-only,
  tool-managed history; an overwrite (or a rerun of the port) can erase
  closure-time events that were recorded after the original run.
- Revalidate only after the merge SHA is confirmed in `origin/main`.
- Preserve the task body, dependencies, and references; only lifecycle status
  and timestamps should change.
- Commit the resulting Markdown archival on a dedicated post-merge branch.
- Do not "fix" this by un-ignoring `.backlogit/logs/`; the logs are local
  runtime evidence, and versioning them would create merge conflicts on every
  parallel shipment.
- When the primary checkout holds unrelated operator changes, run the entire
  closure from an isolated `post-merge/` worktree created at the merge SHA and
  never mutate the primary working tree.

## Impact

This pattern keeps post-merge closure fail-closed while allowing isolated
worktrees to reconstruct intentionally non-versioned gate evidence. It applies
to shipments whose implementation tasks were completed in another worktree.
