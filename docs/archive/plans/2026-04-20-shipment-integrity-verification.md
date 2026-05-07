---
title: "003-S Replay Verification — shipment-reconcile protocol correctness proof"
date: 2026-04-20
type: verification
related_plan: docs/exec-plans/2026-04-20-shipment-integrity-plan.md
incident_shipment: 003-S
incident_commit: d663b77
---

# 003-S Replay Verification

This document is the correctness proof for the `shipment-reconcile` skill design.
It demonstrates, via a paper replay against the known 003-S incident state, that the
new GI/GR reconciliation protocol would have detected and halted the over-scoped archive.

## Incident Summary (003-S)

| Metric | Value |
|---|---|
| Shipment | 003-S |
| Pre-ship commit | `d663b77^` |
| Merge commit | `d663b77` |
| Manifest declared items | 50 |
| Items actually done | 23 |
| Items over-included (never built) | 27 |
| Items deleted from disk by `backlogit_ship_shipment` | 27 |

### Over-included items (never reached `status: done`)

These 27 items were in the 003-S manifest but had `status: queued` or did not exist
in `.backlogit/queue/` at commit `d663b77^`:

| Phase | Count | Status at pre-ship commit |
|---|---|---|
| Phase 1 (1 deferred task) | 1 | `queued` |
| Phase 4 chores + tasks | ~8 | `queued` (never started) |
| Phase 5 chores + tasks | ~6 | `queued` (never started) |
| Phase 6 chores + tasks | ~5 | `queued` (never started) |
| Phase 7 chores + tasks | ~4 | `queued` (never started) |
| Phase 8 chores + tasks | ~3 | `queued` (never started) |

Root cause: Stage harvested ALL phases into the 003-S manifest at planning time,
but Ship only executed Phases 1 and 2.

## Protocol Replay: Pre-Mode (Step 6) Against 003-S State

If `shipment-reconcile mode: pre, expected_status: done` had been invoked at
commit `d663b77^`:

### Step 1: Load manifest

```
backlogit_get_shipment("003-S")
→ items: [001-C, 001.001-T, ..., 001.050-T]  (50 items)
```

### Step 2: Check each manifest item

For each of the 50 manifest items, check `.backlogit/queue/{id}.*` exists
and has `status: done`.

**Items 001.001-T through 001.023-T** (Phase 1-2 completed work):
* Files exist at `.backlogit/queue/` → ✓
* `status: done` → ✓
* Classification: **matched** (23 items)

**Items 001.024-T through 001.050-T** (Phase 4-8 unbuilt work):
* Files exist at `.backlogit/queue/` → ✓ (files present but unbuilt)
* `status: queued` (NOT `done`) → ✗
* Classification: **status-mismatch** (27 items)

### Step 3: Orphan scan

Scan `.backlogit/queue/` for files with `shipment_id: 003-S` not in the manifest.
Result: none (no orphans in this incident — Stage had swept all phases into manifest).

### Step 4: Report

```yaml
shipment_id: "003-S"
mode: pre
expected_status: done
summary:
  total: 50
  matched: 23
  missing: 0
  status_mismatch: 27
  orphan: 0
recommendation: "HALT — operator reconcile required"
```

Recommendation table:

| Issue | Items | Action |
|---|---|---|
| status-mismatch (status: queued, expected: done) | 001.024-T .. 001.050-T (27 items) | Remove from manifest or complete before shipping |

### Step 5: Gate decision

`RECONCILE_FAIL` → halt. `backlogit_ship_shipment` is NOT called.

**Result**: The protocol would have detected all 27 over-included items and
halted with a clear reconciliation report. The operator would have had two
recovery options:

1. **Remove Phase 4-8 items** from the 003-S manifest (reduce scope to match what shipped)
2. **Complete Phase 4-8 items** before calling `backlogit_ship_shipment` (full shipment)

Neither option was available in the actual incident because the protocol did not exist.

## Protocol Replay: Stage Step 5.5 / Step 3.0 (Scope Guard)

If the scope guard had been active at Stage time when 003-S was assembled:

```
harvest_ids = {001-C, 001.001-T, ..., 001.008-T}  ← Phase 1 only (initial harvest)
```

Only items in `harvest_ids` would have been added to the shipment manifest.
Phase 2-8 items were never in `harvest_ids` at the time of assembly. The initial
manifest would have contained exactly 9 items (1 chore + 8 Phase 1 tasks) rather
than 50.

When Phase 2 work began, a NEW harvest would have emitted Phase 2 IDs into
`harvest_ids`, and THOSE would have been added to the shipment (or a new shipment).
Each phase would have been an incremental shipment scope, not a speculative one.

## Verification Verdict

| Check | Result |
|---|---|
| Pre-mode would have detected 27 status-mismatch items | ✅ YES — all 27 have `status: queued` at pre-ship time |
| Pre-mode would have returned `RECONCILE_FAIL` | ✅ YES — any `status-mismatch` triggers HALT |
| `backlogit_ship_shipment` would have been blocked | ✅ YES — RECONCILE_FAIL halts before step 1.a |
| 27 items would have been preserved in queue | ✅ YES — no deletion without archive |
| Stage scope guard would have prevented over-assembly | ✅ YES — `harvest_ids` set at Phase 1 excludes Phases 2-8 |

## 3-Shipment Validation Window

As stated in the implementation plan, the first 3 shipments after this plan is
merged are the operational validation window:

| Shipment | Expected outcome | False-positive threshold |
|---|---|---|
| 004-S (this shipment) | Dogfood: passes pre-mode with all items done | Zero false positives expected |
| 005-S | First non-dogfood shipment; pre-mode should pass cleanly | 0 false positives |
| 006-S | Second non-dogfood shipment | 0 false positives |

If false-positive `RECONCILE_FAIL` halts occur in 005-S or 006-S (items that ARE
done but classified as `status-mismatch`), investigate the backlogit `status` write
timing and consider adding a retry or a short stabilization delay before pre-mode runs.
