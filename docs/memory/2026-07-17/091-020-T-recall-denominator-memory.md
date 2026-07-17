---
title: "091.020-T resolution-aware recall denominator - session memory"
type: session-memory
date: 2026-07-17
task: 091.020-T
feature: 091-F
pr: 265
merge_commit: ce3872ab45c61bf935ac4d19cd7c69b358c9e50f
status: done
---

## What shipped

091.020-T - resolution-aware recall denominator for the canonical-resolution eval
metric. The `resolution_recall` denominator collapses two call-site spellings into
one unit ONLY when both are proven to share a `(caller_id, target_id)` edge in the
numerator's resolved-edge set. The numerator (real indexed edges) is byte-unchanged,
so the change to the recall METRIC VALUE is confined to the eval denominator. The
code change is NOT purely eval-local, however: it also adds a new Cozo relation and
writes a canonical-workspace snapshot on the production full-index and no-drift
incremental-sync paths (additive and guarded — see Files modified). PR #265 called
out this broader production-indexing scope explicitly.

The load-bearing invariant is about the METRIC, not the denominator's raw count.
Recall = resolved / call_sites, so syntax-only counting OVER-counts the denominator
(it treats two equivalent spellings as distinct), which makes recall UNDER-report —
the safe direction. The metric MUST NEVER OVER-report. Over-report happens when a
genuine index-time miss is collapsed onto a coincidental edge: that UNDER-counts the
denominator, inflating recall toward a false 1.0, because eval mis-computed the
canonical target.

## The adversarial resolution: four remediation passes + a validation pass

Over-report is only possible when eval's canonical-resolution context DIVERGES from
the index-time context that produced `resolved_edges`. Each Sol remediation pass found
one more live-recomputed resolution input that could diverge under a stale index:

* **Cycle 1 (e078c91):** eval rebuilt `unsafe_module_prefixes` from the indexed file
  list, a strict subset of production's discovered-file set. Fixed by reusing the
  production `discover_files` + `unsafe_module_prefixes` helpers.
* **Cycle 2 (6c1b9b0):** the shared helpers still recompute from CURRENT disk, not
  index-time. Persisted the unsafe-prefix snapshot so eval stops walking live disk.
* **Cycle 3 (2255f7d):** generalized the persist to the WHOLE index-time
  `CanonicalWorkspace {crates, unsafe_prefixes}` as one JSON snapshot in a new Cozo
  relation `index_canonical_workspace_snapshot`. Collapse is ENABLED iff the snapshot
  is present; written only by a successful full index or a no-drift incremental sync
  (delete-at-start + write-at-success); eval builds its ENTIRE CanonicalWorkspace from
  the snapshot with zero live-disk reads; absent snapshot -> syntax-only.
* **Cycle 4 (174b0af):** the per-file `UseGraph`/`ModulePath` was still reparsed
  from CURRENT caller source at eval time. Closed with a per-file FRESHNESS GATE using
  existing 084.003-T infra: emit the collapse key only when the caller file is fresh
  (`!recorded_hash.is_empty() && !is_index_stale(source, recorded_hash)`); stale or
  unknown-hash files fall back to syntax-only. This commit also hardened the full
  reindex path.

After 174b0af, no live-recomputed resolution input remains: every canonical-resolution
input is either persisted (crates + unsafe_prefixes) or per-file freshness-gated. A
fifth, validation-only pass (no commit) confirmed cross-file target-side staleness is
NOT a vector (eval never reparses target files for canonicalization). Clean verdict ->
merge authorized.

## Files modified (merged diff)

* `src/services/retrieval_eval.rs` - resolution-aware denominator; per-file freshness
  gate in `accumulate_call_sites`; `count_call_sites_resolution_aware` is the sole
  collapse path (one call site, inside the gated function); builds CanonicalWorkspace
  entirely from the persisted snapshot.
* `src/tools/eval.rs` - wiring for the persisted-snapshot lookup.
* `src/services/parsing/canonical/mod.rs` and
  `src/services/parsing/canonical/module_path.rs` - canonical module-path plumbing
  used by the resolution-aware denominator.
* `src/db/cozo_queries.rs` - clear/replace/load for
  `index_canonical_workspace_snapshot`; `is_missing_relation_error` shared helper.
* `src/db/cozo_backend/schema.rs` - the new snapshot relation.
* `src/services/code_graph.rs` - persist the CanonicalWorkspace at full index
  (clear-at-start + write-on-success) and incremental sync (clear-at-start +
  write-only-if prev==current); removed the unused full-index pre-load (FIX B) so a
  malformed snapshot row can no longer abort a repair reindex.
* `tests/integration/retrieval_eval_graph_test.rs` and eval unit tests - regression
  tests for drift-shrink, partial-absent, rename-drift, and use-graph-drift, each with
  a load-bearing fresh/empty control.

## Key decisions

* **Persist-then-freshness-gate beats persist-each-input.** Persisting individual
  resolution inputs one at a time was whack-a-mole (crates, prefixes, use-graph are
  all index-time inputs). The terminating design persists the workspace-global inputs
  as one snapshot and freshness-gates the per-file source-derived input. See the
  compound learning captured this session.
* **Fail-closed on the reachable paths; fail-loud on corruption.** Absent snapshot,
  missing relation, empty hash, or stale file all degrade to syntax-only counting,
  which under-reports but never over-reports. The one exception is deliberate: a
  present-but-malformed snapshot is propagated as an error and aborts eval (fail-loud)
  rather than degrading, since the writer should never emit corrupt JSON.
* **Sol (GPT-5.6 xhigh) as KEY reviewer before Copilot.** Five adversarial passes on
  successive HEADs drove all P1s to closure pre-Copilot, per operator directive. All 6
  Copilot threads on #265 were then triaged (T1 safe-direction, T2 superseded, T3/T5/6th
  scope -> PR-body rewrite, T4 -> FIX B) and resolved. 4-point merge gate green on
  174b0af; merged with a merge commit (P-009).

## Backlog outcome

* 091.020-T -> done -> archived; full merge SHA ce3872a is recorded in the archived
  item's `commit:` frontmatter field (`.backlogit/archive/091.020-T.md`) for tracked
  traceability (the backlogit comment log is gitignored, so it is not durable).
* 091.015-T -> blocked. The ID-preserving canonical_path backfill is technically
  feasible, but WHEN to run it (auto-on-startup vs opt-in vs force-sync) is an open
  product+performance decision with blast radius on large already-indexed workspaces -
  the exact area where the operator removed the A8 forced-reindex as unsafe. Recall is
  already recoverable via a normal re-index (latency, not correctness). Needs operator
  input on the trigger design.

## Next steps

* Drive this closure PR through the 4-point Copilot merge gate; merge.
* Continue the safe queue: 091.016-T (perf, correctness-neutral, fail-closed) ->
  091.019-T (A4 re-export map; own Sol-xhigh review, STOP-and-defer if the precision
  gate can't hold) -> 091.017-T (physical-target-file dual-identity; likely reduce to a
  proving fixture) -> 090.005-T (dispatch registry parity) -> 092.003-T (daemon-reader
  atomicity migration). Keep deferred: 087.005-T / 087.006-T (PowerBI durability),
  025-S/041-F cluster (CozoDB major upgrade), operator branch cluster (081-S/088-F).
