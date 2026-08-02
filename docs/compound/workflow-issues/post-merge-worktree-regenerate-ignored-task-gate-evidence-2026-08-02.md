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
tags: [backlogit, ship-agent, post-merge, gate-evidence, worktree]
---

## Finding

Backlogit records `pre_task_completion_gate_passed` events under
`.backlogit/logs/`, which is intentionally ignored by Git. A clean post-merge
worktree therefore has the archived task Markdown but not the local gate events
created on the implementation worktree. `backlogit shipment ship` correctly
fails closed when those events are absent.

## Safe Recovery

On the clean post-merge branch, revalidate each already-merged task through the
normal state machine:

```text
backlogit move <task-id> --status active
backlogit move <task-id> --status done --json
```

The second command runs the configured pre-task-completion gate against the
merged tree and records fresh passing evidence. After every member has passing
evidence, run `backlogit shipment ship` with the verified merge SHA.

## Guardrails

- Never use `--force-gates` to manufacture closure evidence.
- Revalidate only after the merge SHA is confirmed in `origin/main`.
- Preserve the task body, dependencies, and references; only lifecycle status
  and timestamps should change.
- Commit the resulting Markdown archival on a dedicated post-merge branch.

## Impact

This pattern keeps post-merge closure fail-closed while allowing isolated
worktrees to reconstruct intentionally non-versioned gate evidence. It applies
to shipments whose implementation tasks were completed in another worktree.
