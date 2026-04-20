---
type: stage-handoff
date: 2026-04-19
agent: stage
plan: docs/exec-plans/2026-04-19-cozodb-datalog-migration-plan.md
spike: docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md
shipment: 003-S
---

# Stage Handoff — CozoDB + Datalog Migration

## Shipment

- **ID:** `003-S`
- **Title:** CozoDB + Datalog migration
- **Status:** queued (ready for Ship agent claim)
- **Items:** 50 (1 root chore + 8 phase chores + 41 tasks)

## Hierarchy

```text
001-C   CozoDB + Datalog migration  (root chore)
├── 001.001-C  Phase 0 — Spike close-out and benchmarks       (5 tasks)
├── 001.002-C  Phase 1 — Parallel-DB scaffolding              (6 tasks)
├── 001.003-C  Phase 2 — Schema + symbol CRUD parity          (10 tasks)
├── 001.004-C  Phase 3 — Edge + traversal parity              (6 tasks)
├── 001.005-C  Phase 4 — Vector + hybrid parity               (5 tasks)
├── 001.006-C  Phase 5 — Auxiliary surfaces                   (4 tasks)
├── 001.007-C  Phase 6 — Cutover and operational closure      (3 tasks)
└── 001.008-C  Phase 7 — Removal of SurrealDB                 (2 tasks)
```

## Workflow Trail

| Step | Status | Artifact |
|---|---|---|
| 1. Triage | done | `docs/memory/2026-04-19/stage-triage-cozodb-checkpoint.md` |
| 2. Deliberation | satisfied by spike | `docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md` |
| 3. Impl-plan | done | plan file (33 KB) |
| 3.2 Plan-harden | done (inline) | plan §"Plan Hardening" — risky actions classified |
| 4. Plan-review | PASS attempt 1, 0 P0/P1, 2 P2 advisory | plan §"Plan Review" |
| 5. Harvest | done — 50 items created with deps wired | `files/harvest.ps1`, `files/harvest-ids.json` |
| 5.5. Shipment | done | `003-S` |

## Deferred Backlog (not in shipment)

These pre-existing queue items remain unchanged:

- **001-F** (parallel-DB scaffolding) — pre-existing feature, **superseded** by `001.002-C` (Phase 1). Recommend Ship agent close as duplicate after Phase 1 lands.
- **003-F** (vector parity hardening) — **superseded** by `001.005-C` (Phase 4). Plan task `001.007.002-T` (U6.2) carries close-instruction during Phase 6.
- **002-F** (markdown ingestion) — **independent**, unaffected by migration. Stays queued.
- **018-F**, **024-F**, **025-F** (installer / surface) — **independent**, unaffected.
- **0523404D**, **D715B3EE**, **47F34E2C** stash entries (TS / Java / C# parsers) — **sequencing-dependent**: should land after Phase 4 (vector parity) to avoid double-migration. Stay in stash.

## Operator Decisions Deferred to Phase 0

Captured in plan §"Unresolved operator decisions"; resolved by Phase 0 tasks:

- **U0.1** RocksDB vs SQLite-backed Cozo storage backend
- **U0.5** Specific 768-dim embedding model (deferred to micro-benchmark)
- **U7.1 + U7.2** Release-version anchor for SurrealDB removal (deferred to post-cutover)

Locked in spike §17.6 and encoded as `001.003.001-T` (U2.8): ship migration at **384-dim / `bge-small-en-v1.5`**.

## Estimated Effort

41 tasks × ~2hr each ≈ **82 person-hours**. Phases 2 (10 tasks) and 3 (6 tasks) are the longest.

## Ship Agent Handoff Token

```text
SHIPMENT_ID=003-S
ROOT_CHORE=001-C
PLAN=docs/exec-plans/2026-04-19-cozodb-datalog-migration-plan.md
SPIKE=docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md
ENTRY_POINT=001.001-C  # Phase 0 — start with U0.1, U0.4, U0.5 (parallelizable)
```

Ship agent: `backlogit shipment claim 003-S`, then process Phase 0 tasks in parallel where dep-free, then linearly through Phase 1 → 7.
