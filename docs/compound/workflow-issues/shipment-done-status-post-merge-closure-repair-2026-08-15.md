---
title: "Repair a shipment incorrectly marked done before post-merge closure"
doc_type: learning
source: "115-S post-merge reconciliation repair"
description: >-
  A shipment left in queue with status done cannot use the supported shipment
  closure transition; restore the releasable lifecycle state through backlogit
  before invoking shipment ship.
category: workflow-issues
date: 2026-08-15
confidence: high
evidence:
  - shipment: 115-S
  - pr: 339
  - merge_commit: 60cf6940e1ff50a1ddbfbd983c35392565f604dd
tags: [backlogit, ship-agent, post-merge, reconciliation, lifecycle]
---

## Finding

`backlogit shipment ship` supports the shipment lifecycle transition from
`active` to `shipped`. A shipment that was previously moved to generic status
`done` can remain in `.backlogit/queue/` while being ineligible for the
registered shipment closure operation, which returns a shipment status
conflict.

## Safe Recovery

1. Confirm the implementation PR is merged and its merge SHA is in the default
   branch history.
2. Run shipment reconciliation in pre mode with `expected_status: done`.
3. Confirm all manifest items are matched or validly pre-archived and that
   task-completion gate evidence is available.
4. If the shipment Markdown does not declare `active`, use the registered
   backlogit move operation to normalize only the shipment to `active`, then
   confirm that state through a shipment read.
5. Invoke `backlogit shipment ship` with the confirmed merge SHA, message, and
   author.
6. Run post reconciliation and verify the shipment and every manifest member
   are present in `.backlogit/archive/`.

## Guardrails

- Do not edit shipment status or paths directly in Markdown.
- Do not claim or mutate a successor shipment while repairing its predecessor.
- Do not use `--force-gates`.
- If task gate evidence is absent in a clean post-merge worktree, follow
  `post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`
  before retrying shipment closure.
- Preserve unrelated worktree changes and commit only the bounded closure
  artifacts.

## Result

This sequence repaired `115-S` without changing its manifest, archived the
shipment and released scope with merge commit
`60cf6940e1ff50a1ddbfbd983c35392565f604dd`, and left successor `116-S`
queued and unclaimed.
