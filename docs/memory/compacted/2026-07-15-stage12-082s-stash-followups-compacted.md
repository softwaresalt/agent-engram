---
title: "Completed release-unit memory compaction: 12-stash triage, 082-S, stash-followups"
type: compacted-memory
date: 2026-08-21
status: complete
sources:
  - docs/archive/memory/2026-07-14/stage-12-stash-triage-memory.md
  - docs/archive/memory/2026-07-15/082-S-ship-memory.md
  - docs/archive/memory/2026-07-15/stage-stash-followups-memory.md
---

# Completed release-unit memory compaction

## Stage — 12-stash triage & release planning (2026-07-14)

Processed all 12 active stash entries into a reviewed, queued backlog: 5
features (086-F reliability, 087-F DAX/PBI, 088-F recall (HIGH), 089-F
durability, 090-F parity), 20 tasks, deliberation 013-D, plan-review gate
088.001-R (PASS), and 5 queued shipments in order 081-S→082-S→083-S→084-S→
085-S. Key decisions: 088-F resolver uses qualified-name exact/singleton-only
(013-D Option A), release-gated by a recall/precision eval (088.005-T);
086.003-T fails closed on shared/external data-dir migrate-down; 087.004-T
uses symlink_metadata + visited-set containment. `088.001-T` (incremental
post-pass) and `090.004-T` (parity gap-closing) were deferred pending a perf
spike and an audit respectively. Full detail:
`docs/decisions/2026-07-14-stage-12-stash-triage-release-plan.md`.

## Ship — 082-S runtime reliability & concurrency hardening (086-F)

Shipped via PR #249 (merge `8adde5e`). All four tasks (086.001–086.004-T)
delivered test-first: SQLITE_BUSY-tolerant retry core for `calls_edge`
migrate/rollback, bounded reopen-retry in `connect_db` with
`catch_busy_panic` (cozo 0.7.x SQLITE_BUSY/LOCKED reopen is a **panic**, not
an Err — Err-only retry is inert), fail-closed guard on destructive
migrate-down against shared/external `ENGRAM_DATA_DIR`, and an atomic
`snapshot_dispatch_context()` read in `get_workspace_status` with
workspace_id-guarded stale write-back. Adversarial review (5-model) plus 7
Copilot rounds surfaced and remediated two gating P1s (retry falsely
succeeding on `:replace` "already exists"; Err-only retry vs panic) before
PR. Git isolation: the ship worktree was based on Stage commit `a6b0925`,
not the 081/088-F resolver HEAD, so main received accurate blocked-pipeline
state without leaking unmerged resolver code (PR #248 unaffected).

## Stage — stash-followups (081-S/082-S residuals, 2026-07-15)

Working from `origin/main` directly (the backlogit MCP server was pinned to
a stale root worktree; backlogit CLI used as the registry-declared
fallback). Disposed all 4 residual stash entries (all archived, none
deleted): 8CCB9CC3 + B6DF4AD1 consolidated into blocked feature 091-F
(Option C canonical-identity, gated on spike 091.001-T); 6870ECDF deferred
as blocked task 091.002-T (reconciles 088.005-T, not mutable in Stage
scope); 32DAA85B promoted to independently executable feature 092-F (writer
-side workspace+config atomicity, the 086-F "F4" residual) with task
092.001-T and gate 092.001-R (PASS), packaged as new shipment 086-S —
recommended ahead of 083-S/084-S/085-S since it closes an observable race
in already-shipped 086-F. Adversarial + plan review gate: PASS, 0 P0
(`docs/closure/2026-07-15-stage-followups-adversarial-review.md`).

## Preserved, not compacted

083-S, 084-S, 085-S manifests were unchanged by this session and remained
queued; their execution memory is compacted separately once each ships.
