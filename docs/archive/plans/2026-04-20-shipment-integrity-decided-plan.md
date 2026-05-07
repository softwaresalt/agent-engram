---
title: "Decided Plan — Shipment Manifest Integrity (GI/GR Reconciliation)"
date: 2026-04-20
release_unit: 004-S
source_plan: docs/archive/plans/2026-04-20-shipment-integrity-plan.md
status: delivered
---

# Decided Plan: Shipment Manifest Integrity (GI/GR Reconciliation)

## Problem

003-S archived 50 manifest items; only 23 were actually implemented. Two root gaps:

1. **Stage over-inclusion** — Step 5.5/3 swept all unassigned queue items into the
   shipment, not just those from the current harvest hierarchy.
2. **Ship blind archive** — Step 6 called `backlogit_ship_shipment` without verifying
   per-item state; undone items were archived as if complete.

## Decision: Harness-only defense-in-depth (Option A)

Implement GI/GR reconciliation entirely in the agent harness (Stage + Ship prompts +
new skill). Do not wait for upstream `backlogit` changes. Option B (Datalog query
adapter) was rejected as out-of-scope. Option C (external CI gate) was rejected as
fragile.

## Delivered Work

| Unit | File | Change |
|---|---|---|
| U-1 | `.github/agents/stage.agent.md` | Step 5.5/3: `harvest_ids` scope guard — only items emitted by the current harvest may be added to the shipment |
| U-2+3+12 | `.github/skills/shipment-reconcile/SKILL.md` | NEW — full GI/GR reconciliation skill (pre + post modes, lock integration) |
| U-4 | `.github/agents/ship.agent.md` | Step 6.1.0: invoke `shipment-reconcile mode: pre, expected_status: done` before `backlogit_ship_shipment`; halt on `RECONCILE_FAIL` |
| U-5 | `.github/agents/ship.agent.md` | Step 6.1.c: invoke `shipment-reconcile mode: post` after archive + restore; attach report to closure |
| U-6 | `.github/agents/ship.agent.md` | Step 0.5: intake pre-mode (`expected_status: queued`) — checks presence, not status, at build-start |
| U-7 | `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` | Resolution section added linking skill and agent changes |
| U-8 | `docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md` | Upstream issue draft for backlogit maintainers |
| U-9 | `.github/instructions/backlogit.instructions.md` | Shipment Reconciliation section: `backlogit_ship_shipment` MUST NOT be called without `shipment-reconcile mode: pre` first |
| U-10 | `docs/exec-plans/2026-04-20-shipment-integrity-verification.md` | 003-S replay proof — 27 items would have been halted; 23 valid |
| U-11 | `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md` | Report schema (YAML/JSON): `matched`, `missing`, `orphan`, `status-mismatch`, `pre-archived` |
| U-12 | (merged into U-2) | Single-writer lock on `{shipment_id}.md` during Ship Step 6; lock path fixed to `.md` (not `.S.md`) |
| U-13 *(010-S)* | `.github/skills/shipment-reconcile/SKILL.md` | `pre-archived` classification: items found in archive but not queue are valid; distinct from `matched` so they remain visible in reports |
| U-14 *(010-S)* | `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md` | Added `summary.pre_archived` counter and `pre-archived` to classification table and examples |
| U-15 *(010-S)* | `docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md` | Status updated to `submitted`; upstream issue filed at softwaresalt/backlogit#63 |

## Key Constraints

* **No auto-prune** — reconciliation is report-and-halt only. Operator manually fixes via
  `backlogit_*` tools, then re-invokes Ship Step 6.
* **`expected_status` parameter** — single `pre` mode handles both intake (`queued`) and
  pre-ship (`done`); no hidden `--intake` variant.
* **Pre-mode `pre-archived` classification** — archive fallback is permitted but ONLY to
  produce a distinct `pre-archived` classification (never `matched`). Items found in archive
  but not queue are surfaced explicitly in the report. This resolves the original concern
  about silent passage: `pre-archived` items are visible and identifiable, and PROCEED is
  still safe because they were already shipped. Implemented in shipment 010-S.
* **`git restore` is conditional** — only run when `git status` shows archive deletions
  (the `backlogit_ship_shipment` deletion quirk).

## Rejected Alternatives

* `--allow-prune` flag: rejected (auto-mutation of manifest is too dangerous)
* File-touch heuristic for `orphan` detection: rejected (objective state checks only)
* Separate `--intake` CLI variant: rejected (parameterize via `expected_status` instead)

## Known Follow-Up

* ~~**Pre-mode `pre-archived` spec gap**~~ — resolved in shipment 010-S (U-13/U-14 above).
* ~~**Dogfood validation**~~ — reconcile gates verified during shipments 010-S (stash `CC8DD4AF`).
* ~~**Upstream escalation**~~ — filed as [softwaresalt/backlogit#63](https://github.com/softwaresalt/backlogit/issues/63) in shipment 010-S (stash `73DD2A8D`).
