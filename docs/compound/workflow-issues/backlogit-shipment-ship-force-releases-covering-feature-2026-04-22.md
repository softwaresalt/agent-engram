---
title: "backlogit shipment ship force-releases excluded covering feature"
description: "backlogit shipment ship can mark a covering feature done and remove it from queue even when the reconciled shipment manifest excludes that feature"
problem_type: "shipment archive drift"
category: "workflow-issues"
component: "backlogit shipment closure"
root_cause: "The shipment ship command still derives release scope from the covering feature relationship instead of honoring the reconciled manifest items list as the sole archive source of truth"
resolution_type: "workaround"
severity: "high"
message: "shipment ship archived_ids includes covering feature not present in manifest; queue feature file disappears without intended archive scope"
file_path: ".backlogit/queue/006-S.md"
citations:
  - "docs/closure/2026-04-22-006-s-closure.md"
  - ".backlogit/reconcile/006-S-pre-20260422-074546.md"
  - ".backlogit/reconcile/006-S-post-20260422-075028.md"
tags:
  - "backlogit"
  - "shipment"
  - "archive"
  - "workflow"
---

## Problem

During post-merge closure for shipment `006-S`, the reconciled shipment manifest
contained only the 12 completed B1 chores/tasks. Even so, `backlogit shipment
ship 006-S` force-released covering feature `029-F`, marked it done, and removed
`.backlogit\queue\029-F.md` from queue. The command output reported `029-F` in
`archived_ids` even though the manifest explicitly excluded it.

## Root Cause

The shipment close command still expands archive scope from the feature/shipment
relationship instead of treating the reconciled `custom_fields.items` list on
the shipment record as the authoritative release scope. That means manifest
pruning alone is not enough to protect a partially shipped parent feature.

## Resolution

1. Run the normal pre-mode shipment reconciliation and confirm the manifest is correct
2. After `backlogit shipment ship`, immediately inspect the result for unintended parent-feature release
3. If the covering feature was force-released, restore the queue source-of-truth file from Git:
   * `git restore -- .backlogit\queue\029-F.md`
4. Re-sync backlogit so the index reflects the repaired queue state:
   * `backlogit sync`
5. Record the incident in the post-mode reconcile report and closure artifact
6. Stash a follow-up bug to fix the backlogit command itself

## Prevention

Treat `backlogit shipment ship` as capable of overshipping a covering feature
until the tool is fixed. Do not assume a reconciled manifest alone is enough.
Always verify both:

* the intended archived item files exist under `.backlogit\archive\`
* the deferred covering feature still exists under `.backlogit\queue\`

If the deferred feature disappears, restore it before committing the closure
branch.
