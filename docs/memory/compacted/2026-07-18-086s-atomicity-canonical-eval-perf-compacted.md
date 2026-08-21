---
title: "Completed task memory compaction: 086-S atomicity chain, 091.020-T, 091.016-T"
type: compacted-memory
date: 2026-08-21
status: complete
sources:
  - docs/archive/memory/2026-07-17/086-writer-atomicity-closure-memory.md
  - docs/archive/memory/2026-07-17/091-020-T-recall-denominator-memory.md
  - docs/archive/memory/2026-07-17/092-003-T-daemon-atomicity-closure-memory.md
  - docs/archive/memory/2026-07-17/092-004-T-handler-atomicity-closure-memory.md
  - docs/archive/memory/2026-07-17/092-reader-atomicity-closure-memory.md
  - docs/archive/memory/2026-07-18/091-016-T-prepass-perf-closure-memory.md
---

# Completed task memory compaction

## 086-S / 092-F — Writer+reader+daemon+handler workspace/config atomicity chain

A four-part atomicity remediation closed the 082-S adversarial "F4" residual
(non-atomic `(workspace, config)` paired reads/writes) across every code path:

1. **092.001-T** (PR #261, merge `106be1d`) — atomic `set_workspace_and_config`
   writer (both locks acquired in a fixed order, capacity check first, no
   partial publish).
2. **092.002-T** (PR #263, merge `4436d53`) — None-gating atomic reader
   `snapshot_workspace_and_config`, migrating `background_db_hydration` and
   `drain_pending_sync`; deliberately preserves skip-if-either-`None`
   semantics rather than defaulting config.
3. **092.003-T** (PR #269, merge `68a3378`) — migrated the four daemon
   background-sync closures (`run_with_shutdown`/`_v2`) through a new shared
   seam `snapshot_daemon_sync_context`, with a non-vacuous stress test
   asserting no torn pair.
4. **092.004-T** (PR #271, merge `23f4030`) — terminal link: migrated the four
   MCP tool handlers (`index_workspace`, `sync_workspace`, `map_code`,
   `impact_analysis`) through `snapshot_graph_handler_context`
   (`snapshot_dispatch_context`, which default-substitutes absent config,
   matching prior handler behavior). A handler-level test
   (`map_code_handler_never_observes_torn_pair`) was required — a seam-only
   test would stay green even if a handler bypassed the seam.

All four locks follow the same acquisition order (`active_workspace` then
`workspace_config`), so the chain is deadlock-free by construction. Each
step used GPT-5.6 Sol (xhigh) + Gemini adversarial review before Copilot per
operator directive, and each closed with a 4-point merge gate. 092.004-T was
the terminal item in the chain; no further atomicity follow-up remained.

## 091.020-T — Resolution-aware recall denominator (091-F canonical eval)

PR #265 (merge `ce3872a`) made the `resolution_recall` eval denominator
collapse equivalent call-site spellings only when index-time context is
proven fresh, closing four successive over-report vectors found across five
adversarial passes: syntax-derived unsafe-prefixes recomputed from a stale
file list → shared production helpers → still-live-disk recompute →
persisted whole-workspace `CanonicalWorkspace` snapshot (new Cozo relation,
written only by full-index or no-drift incremental sync) → per-file
use-graph/module-path still needing a content-hash freshness gate before
the collapse key is emitted. Final invariant: the metric may under-report
(safe direction) but never silently over-report; a malformed snapshot fails
loud rather than degrading silently.

## 091.016-T — Async pre-pass + parse-dedup (partial delivery)

PR #267 (merge `36a944a`) shipped Option 1 (async `tokio::fs` reads instead
of blocking) and Option 2 (hash-gated reuse of pre-pass-parsed
`ModulePath`/`UseGraph` in the main per-file symbol pass) of a canonical
pre-pass perf task. Canonical edge output stayed byte-identical
(pure performance). Copilot correctly identified that Option 2 eliminated
the duplicate *parse* but not the second *read*, and that the
full-index post-pass still re-parses staged files a third time — both
deferred to follow-up 091.021-T rather than expanded mid-task (partial,
documented delivery over scope creep).

## Preserved, not compacted

Deferred follow-ups remain blocked and are tracked in the backlog, not
memory: 091.015-T (blocked, backfill trigger design needs operator
input), 091.019-T, 091.021-T, 090.005-T, and the 041-F CozoDB
major-upgrade cluster. Status as of this compaction (2026-08-21, not the
source-checkpoint date): 091.017-T was independently rejected/archived
(`wont-fix`, refuted finding — see the 2026-07-31 drain-closeout
compaction); 087.005-T/087.006-T shipped and archived as part of shipment
100-S (PowerBI durability); 025-S was archived as abandoned. These three
are historical, not open follow-ups.
