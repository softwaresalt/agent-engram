---
title: "Stage 142-F addendum: declare root slot shipments 143-S and 144-S as dag-root"
date: 2026-10-06
agent: stage
feature: "142-F"
plan: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
plan_revision: 18
amends: "docs/memory/2026-10-05-stage-142-f-asm-memory.md (assembly step 7, shipments and edges)"
branch: chore/stage-142-f-dag-roots
base: "origin/main @ ab6e8e6d9a366edd4670465ab6b974f3a8b9df93 (PR #413, ASM)"
status: complete-awaiting-pr-merge
---

# Stage 142-F addendum: dag-root correction

## Trigger

The Orchestrator's pre-claim topology gate for the first slot failed:
`autoharness gate pipeline-topology --mode agent --shipment 143-S --phase pre_claim --json`
returned exit 1 with `UNSEQUENCED_SHIPMENT` ("target 143-S has no explicit blocks
edge; record the real blocks edge or declare the shipment a root"). The gate
treats a shipment as a declared root only when its `labels` include `dag-root`.
Assembly (PR #413) recorded the 31 `blocks` edges but never applied that label
to the slots that have no predecessors.

## Verified facts (2026-10-06, on `main` at `ab6e8e6d`)

* Plan section 4 slot table: only Slot-01 (`142.060-T`, "none (E11)") and
  Slot-02a (`142.061a`, minted as `142.075-T`, "none") have no predecessors.
* `backlogit dep list` is empty for `143-S` (Slot-01) and `144-S` (Slot-02a).
  Every other queued slot shipment, `145-S` to `166-S`, has at least one
  `blocks` edge, so no other shipment is labeled.
* Neither `143-S` nor `144-S` had any labels before this change.

## Correction

* `backlogit update 143-S --labels dag-root` and
  `backlogit update 144-S --labels dag-root`, then `backlogit sync`.
* No `blocks` edge was added or removed. `queue_position` (10, 20) is unchanged,
  so Slot-01 is still claimed first (OD-1).

## Gate evidence

* On this branch the CLI gate stops at `branch_ownership` (`BRANCH_MISMATCH`,
  expected because the branch is not a slot branch). Output is in
  `logs/2026-10-06-topology-pre-claim-{143-S,144-S}-dag-root.json`.
* The same evaluator, run with only the current branch read as `main`, passes
  for both shipments (exit 0). `shipment_readiness` passes with
  `predecessor_source: declared_root`. Output is in
  `logs/2026-10-06-topology-pre-claim-{143-S,144-S}-dag-root-simulated-main.json`.
  The `logs/` files are git-ignored and stay local.
* The Orchestrator should rerun the real CLI gate on `main` after this PR merges.
