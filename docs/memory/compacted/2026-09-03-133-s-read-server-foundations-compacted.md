---
title: "133-S read-server foundations — compacted session memory"
date: 2026-09-03
type: session-memory-compacted
doc_type: memory
agent: ship
shipment: 133-S
feature: 142-F
status: done
sources:
  - docs/archive/memory/2026-09-03-ship-pr-372-stage-133-s-merge-closure.md
  - docs/archive/memory/2026-09-03-ship-133-s-mid-session-checkpoint.md
  - docs/archive/memory/2026-09-03-ship-133-s-pr-ready-checkpoint.md
---

## Scope

Shipment `133-S` (feature `142-F`) delivered read-server foundations:
F00 (49 placeholder test-manifest registrations), F01 (storage feasibility
spike, GO verdict, Windows durability residual risk accepted), F02 (strict
`DaemonMode` mode-contract parser), F03 (immutable `mode` field on
`AppState`, temporary `with_mode` constructor, existing constructors
forward unchanged), F12a (`crates/engram-indexer` empty stub crate +
workspace membership). F04 (constructor call-site migration) and
F06-F09/F12 (real generation storage + indexer logic) are explicitly
deferred to later shipments.

## Timeline

1. **PR #372** (staging-gate closure, 2026-09-03T05:29:41Z, merge commit
   `23865522fcfa5ee7e145beeafc896fe4cb46ac45`): opened the Orchestrator
   staging gate for `133-S` by landing `.backlogit/queue/133-S.md` on
   `main` in `queued` status with its 11-item manifest (`142-F`,
   `142.001-T` + 5 subtasks, `142.006-T`, `142.004-T`, `142.002-T`,
   `142.007-T`). Shipment not claimed in that session; no build occurred.
   Identified a process gap: the mandatory unconditional P-020
   `compact-context` call was skipped in that closure — corrected in this
   session.
2. **Build session** (this shipment's claim through PR #376): all 10
   task-level manifest items completed and marked `done`
   (`142.002-T`→`461e0e2d`, `142.001-T`+subtasks→`3f890662`,
   `142.006-T`→`7b96b641`, `142.007-T`→`815e593f`, `142.004-T` completed
   last with the storage-feasibility spike and its decision doc). Quality
   gates green throughout (`cargo check`, `clippy -D warnings -D
   clippy::pedantic`, `fmt --check`, `cargo dev-test`). `cargo ci
   --all-features` pre-existing opentelemetry compile break
   (unrelated, stash `7B270F79`) and full-suite flakiness under parallel
   execution (unrelated, stash `58B33C45`) both confirmed pre-existing and
   out of scope.
3. **PR #376 ready** (HEAD `9ccbfffa60b1d00d56af08f5ab7143cdf1901fcd`):
   local review `READY_WITH_FOLLOWUPS` (8 personas, 0 P0/P1, 1 P2 fixed
   in-diff, 2 P2 deferred as stash `A7C0BA5F`/`5A7FBC37`); Copilot review
   flagged rename-durability gaps at HEAD `4c8cc253`, fixed
   (`interrupted_rename_never_yields_a_torn_destination`,
   `sync_parent_dir`), re-reviewed clean at `9ccbfffa`; P-009 merge-commit
   only confirmed at repo level.
4. **This session — merge + post-merge closure**: re-verified all gates at
   exact HEAD `2005b3db94752dbe37946a98532c46dde1aad674` (local readiness,
   P-018 Copilot `SATISFIED` zero unresolved threads, CI green, mergeable
   clean, P-009). Merged via `gh pr merge 376 --merge` → merge SHA
   `33a0a41e345cef8965b707346728d44fa5492daf`, confirmed
   `MERGE_CONFIRMED` via `git merge-base --is-ancestor`. Ran full runtime
   verification (build, MCP catalog/contract tests, pre-existing-failure
   isolation via temporary diagnostic worktree at pre-merge `main` tip
   `c66d320e`) — see
   `docs/closure/133-S-2026-09-03-runtime-verification.md`.

## Blocking finding at post-merge shipment closure (carried forward, not resolved by this session)

`backlogit shipment ship 133-S` (the only CLI path to `status: shipped` in
backlogit 1.10.1 — direct `move --status shipped` is unconditionally
rejected by the 144-F `ErrShipmentShippedRequiresEnvelope` guard) cannot be
safely invoked: covering feature `142-F` is an explicit manifest member
with 59 direct task children and 28 nested subtask descendants (87 total,
verified by direct file enumeration under the `142.*` ID namespace); the
manifest contains only 5 of each (10 of 87), leaving 77 descendants outside
scope for `133-S` (a multi-shipment feature). The cascade would force-mark
`142-F` `done` and
force-requeue-and-detach the other 77 descendants — forbidden by this
workspace's own P-015
fully-covered-root test. See
`docs/closure/133-S-2026-09-03-post-merge-closure.md` for the full
evidence chain and recommended remediation (Stage should remove `142-F`
from `133-S`'s `custom_fields.items`, relying on `parent_id` resolution,
after which safe archival becomes possible). `133-S` remains `active`
(not `shipped`) pending that resolution — `134-S` must not be claimed
until it clears.

## Follow-up stash entries (all pre-existing at time of this compaction, verified present, none re-created)

`A7C0BA5F`, `5A7FBC37`, `58B33C45`, `7B270F79`, `F2E84E15` (Windows
directory-entry durability residual risk — accepted, tracked for later
F07/F08 shipments).

## Verbose originals

Archived to `docs/archive/memory/`:
`2026-09-03-ship-pr-372-stage-133-s-merge-closure.md`,
`2026-09-03-ship-133-s-mid-session-checkpoint.md`,
`2026-09-03-ship-133-s-pr-ready-checkpoint.md`.
