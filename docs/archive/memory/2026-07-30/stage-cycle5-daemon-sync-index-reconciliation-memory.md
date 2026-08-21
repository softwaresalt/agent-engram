# Stage cycle-5 memory — Daemon sync/index reconciliation cluster (105-F)

- **Date:** 2026-07-30 (UTC; local 2026-07-29 evening)
- **Agent:** Stage
- **Session:** stash-to-backlog pipeline for Cluster A (daemon sync/index
  reconciliation & pending-sync lifecycle correctness) + 015-D fold evaluation

## What shipped out of Stage this session (backlog, not code)

- **Feature 105-F** (queued): "Daemon sync/index reconciliation & pending-sync
  lifecycle correctness (PR #297/#299 residual races)". Covering feature for three
  PR #297/#299 Copilot-thread residual races on the shipped-104-F/103-F subsystem.
- **Tasks (all queued, medium, TDD RED→GREEN, ≤~2h, width-isolated):**
  - **105.001-T** (R1, from stash **B7F52777**, bug) — Generation-scoped
    pending-sync clear. `clear_all_pending_sync` (lifecycle.rs:255 cancel / :280
    DB-fail) is a whole-queue wipe; must become generation-scoped so an old
    generation's clear can't erase a newer generation's `--revalidate-code-graph`
    / `--backfill-python-canonical` intent.
  - **105.002-T** (R2, from stash **0B5AAAD2**, task) — Pending-sync drain
    producer/consumer handoff state machine. `drain_pending_sync_to_completion`
    (lifecycle.rs:457–480) snapshot loop can't close the TOCTOU; needs an atomic
    producer→lock-holder handoff observed on release. **Depends on 105.001-T.**
  - **105.003-T** (R3, from stash **7A317008**, task) — Forced-index
    reconciliation ordering. Move the `force||!any_hash_skipped`-gated
    `indexed−discovered` eviction (code_graph.rs:1926–1948) AHEAD of the singleton
    post-pass `reresolve_calls_edges_with_canonical_context` (code_graph.rs:1867),
    + duplicate-callee-then-excluded recall-RECOVERY test. Independent width.
- **Shipment 097-S** (queued) — covering_feature 105-F + members
  105.001-T/105.002-T/105.003-T. Step 5.5 scope guard PASS (only the coherent
  feature + its width-isolated children). **Ready for Ship to claim.**

## Dependencies + links wired

- **105.002-T `blocks`-depends-on 105.001-T** (real execution-ordering: the R2
  handoff must be layered on R1's generation ownership or it re-introduces the
  cross-generation wipe; same AppState struct).
- 105.003-T is independent (code-graph width) — no execution-blocking dep.
- Links: 105-F `related_to` {015-D, 104-F, 103-F}; 105.003-T `related_to` 015-D.

## 015-D / 5765BAAB fold decision — DO NOT FOLD

Evaluated folding 5765BAAB's non-persist portion into 105.003-T (R3). **NOT
folded.** 5765BAAB / 015-D remain ACTIVE (un-harvested) with a non-fold note on
the stash entry + a comment on the 015-D deliberation. Rationale:
1. **Distinct root cause/trigger** — R3 is a pinned ordering bug requiring a
   duplicate callee in an excluded-but-indexed file; the 015-D repro is a fresh
   workspace with a unique def (no excluded file, no duplicate callee), so R3 is
   not even triggered by it.
2. **015-D root cause is UNPINNED** (H1 vs H4 open); a fix would violate 013-D.
3. **015-D IPC-hang portion is out-of-width** (architectural async response).
4. The 015-D spike recommends DEFER to a Ship-owned/instrumented runtime spike.
- **Revisit trigger:** if the runtime spike pins the non-persist to the same
  post-pass/commit boundary R3 touches, fold then.

## Stash disposition

- **Harvested (consumed):** B7F52777 → 105.001-T, 0B5AAAD2 → 105.002-T,
  7A317008 → 105.003-T (provenance in each task's `source_stash_id`).
- **Left active (untouched):** 5765BAAB (015-D, with non-fold note), 99AFF44B
  (017-D cozo bump), A85DC0E3 (harvest-provenance doc reconciliation), 05EA3D39
  (lz4_flex monitor).

## Artifacts

- Plan: `docs/exec-plans/2026-07-30-daemon-sync-index-reconciliation-plan.md`
  (impl-plan + plan-harden H1–H5 + plan-review GATE: PASS + fold decision).
- This memory: `docs/memory/2026-07-30/stage-cycle5-daemon-sync-index-reconciliation-memory.md`.

## Handoff to Ship

- Claim shipment **097-S**. Execution order: **105.001-T → 105.002-T** (blocked),
  **105.003-T** independent (can run in parallel with the R1/R2 chain).
- TDD is non-negotiable: each task's AC1 specifies the failing RED test that must
  fail against pre-fix code before the GREEN implementation.
- Residual risk flagged for Ship: R1/R2 may need to touch the AppState pending-sync
  flag representation (generation-tagged packed atomic / mutex tri-state) — confirm
  the minimal primitive; contained to `src/tools/lifecycle.rs` + `src/tools/write.rs`.
- R3 must preserve fail-closed + 082/094/096/101/103-F post-pass certify-order
  invariants (dangling-edge sweep + generation marker stay AFTER the post-pass).

## Pre-flight caveat respected

- `M start.ps1` (operator state) NOT touched/staged/committed; main NOT pushed.
  Stage artifacts committed to a dedicated feature branch only.
