---
title: "Upstream Issue: backlogit_ship_shipment lacks per-item state validation"
date: 2026-04-20
tool: backlogit
affected_operation: backlogit_ship_shipment
severity: high
discovered_in: shipment 003-S
status: submitted
upstream_issue: https://github.com/softwaresalt/backlogit/issues/63
submitted_in: shipment 010-S (2026-04-23)
---

# Upstream Issue: `backlogit_ship_shipment` Lacks Per-Item State Validation

## Summary

`backlogit_ship_shipment` archives a shipment manifest without verifying that
each declared item is actually complete (`status: done`) or even present on
disk. This causes silently corrupted shipment records where unbuilt items are
marked as shipped.

## Reproduction

### Environment

* backlogit version: (check your version)
* Repository: any repo with `.backlogit/` directory

### Steps to Reproduce

1. Create a shipment manifest (`backlogit_create_shipment`) with 10 items.
2. Complete only 5 of those items (mark them `status: done`).
3. Call `backlogit_ship_shipment(shipment_id, merge_sha)`.
4. Inspect `.backlogit/archive/{shipment_id}.md`.

### Expected Behavior

The tool should:
* Verify each manifest item's `status: done` before archiving.
* Skip (or reject) items that are not `done`, removing them from the manifest.
* Emit a per-item action log: `[archived|skipped-not-done|missing] {id}`.
* Refuse to archive if any manifest item file is missing from disk.

### Actual Behavior

The tool:
* Moves each manifest item's queue file to `.backlogit/archive/` without
  checking its `status` field (does not validate `status: done` before archiving).
* Deletes the on-disk archive copies after the internal move — the known workaround
  is to run `git restore .backlogit/archive/` when `git status` reports deletions.
* Archives the shipment as if all manifest items were shipped, regardless of
  their actual completion state.

## Impact

In our incident (shipment 003-S, commit `d663b77`):

* Manifest declared 50 items
* Only 23 items were actually completed
* 27 incomplete items were **deleted from disk** with no archive copy
* Discovery required a post-hoc audit comparing git history against the
  archived manifest
* Recovery required manual `git checkout {pre-ship-sha} -- .backlogit/queue/{id}.md`
  for each of the 27 deleted files

Reference: `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`

## GI/GR Double-Entry Analogy

A shipment manifest is like a goods invoice (GI). The queue state is the
goods receipt (GR). Before closing a shipment, GI and GR should reconcile:
every item on the invoice must have a corresponding "received" record (done
status + file on disk). Without this check, the accounting is inaccurate.

## Requested Fix

We request the following changes to `backlogit_ship_shipment`:

1. **Pre-archive validation**: Before deleting any queue file, verify the
   item's frontmatter `status: done`. Items that are NOT done should be
   skipped (left in queue) or cause the operation to fail with an explicit
   error listing the non-done items.

2. **Per-item archive**: Move each completed item's markdown file from
   `.backlogit/queue/` to `.backlogit/archive/` individually (do not just
   delete them).

3. **Missing-file guard**: If a manifest item has no file in queue or archive,
   refuse to ship and surface an integrity error:
   `INTEGRITY_ERROR: manifest item {id} has no file in queue or archive`.

4. **Per-item action log**: Emit a log line for each item:
   `[archived] {id}`, `[skipped-not-done] {id}`, or `[missing] {id}`.

5. **Return value**: Include a summary of per-item outcomes in the return value
   so callers can verify results programmatically.

## Workaround (harness-side)

While awaiting the upstream fix, we have implemented a harness-side
`shipment-reconcile` skill (`.github/skills/shipment-reconcile/SKILL.md`)
that runs before and after `backlogit_ship_shipment`. This provides the
GI/GR check as a compensating control:

* `mode: pre` — verifies all manifest items are present and `status: done`
  before `backlogit_ship_shipment` is called; halts on any discrepancy
* `mode: post` — verifies archive integrity after the call, including
  detecting the known deletion-without-archive behavior

Reference: `docs/exec-plans/2026-04-20-shipment-integrity-plan.md`

## Related

* Compound learning: `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`
* Implementation plan: `docs/exec-plans/2026-04-20-shipment-integrity-plan.md`
* Reconciliation schema: `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md`
* Harness skill: `.github/skills/shipment-reconcile/SKILL.md`
