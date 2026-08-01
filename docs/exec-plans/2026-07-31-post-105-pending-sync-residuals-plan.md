---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
updated: 2026-08-01
status: "reviewed (Stage review cycle 3 PASS; residual P2 follow-up)"
source: "docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue and the R2 producer/finisher backstop. Current code still separates `set_workspace_and_config` from `begin_scan_generation`, lets `write.rs` publish after a coherent workspace/config capture using a later generation read, lets lifecycle execute a stale full-mask claim after an acquired-lock race, can strand newer work when old hydration cancels or DB connection fails, and leaves startup publication after failed CAS without a deterministic RED pause point.

The operator-authoritative guardrails keep `109.004-T` in `src/daemon/ipc_server.rs` only with at most three private production-function touches and no public `run_with_shutdown` caller changes. `109.006-T` establishes a synchronous crate-private opaque current-generation-token capture method plus qualified snapshot/publication primitives. `try_start_startup_sync` remains synchronous and bool-compatible: it captures the token before CAS, uses token-taking publish/reacquire after failure, and returns `true` for either the initial owner or producer-reacquirer so existing callers run the startup body once and their existing completion helper releases and bounded-drains. `109.008-T` owns claim resolution plus cancellation/DB-failure release-and-drain in `background_db_hydration`. Scope excludes `015-D`, `017-D`, `025-S`, `081-S`, schema, wire/response, persistence, and unrelated code-graph work. Stage changes planning artifacts only.

## Provenance and Supersession

- Source deliberation: `018-D`; source stash: `FF55E51A`, `88EB5FB1`, `1E70A289`.
- Archived `105-F` remains immutable.
- This third current-content review cycle supersedes the prior startup-outcome, compatibility-wrapper, and lifecycle completion wording. The artifact content and queued state are reviewed; acceptance is not bound to a Git HEAD.
- `102-S`, `103-S`, and `104-S` are separate release units in shared `custom_fields.operator_batch: 102-104-integration`, preserving `operator_order` 1, 2, and 3.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| G1 atomic rebind | State publishes workspace/config, checked next generation, cancel ownership, and queue floor in one transition and returns one crate-private token with the generation-specific cancel receiver; lifecycle consumes it once | `109.001-T` G1a/G1c |
| G2 fail-closed exhaustion | Generation uses `checked_add(1)`; `u64::MAX` returns an existing typed system error before any workspace, config, generation, cancel, queue, or hydration-ready mutation | `109.001-T` G1b |
| W1 qualified tokens | A crate-private qualified snapshot wraps unchanged public `DispatchSnapshot`; a separate synchronous crate-private method captures an opaque current-generation token for startup before CAS | `109.005-T` W1a plus `109.003-T` S1 |
| W2 explicit publication | Token-taking publish/reacquire applies newer-replaces, equal-full-mask-OR, and older-ignored semantics; write and startup carry exact captured tokens without retry/fallback | `109.005-T` W1b/W1c; `109.003-T` race/control |
| L1 generation-owned claim | State atomically claims the full mask and resolves an acquired claim against the exact qualified snapshot or exact-rearms it after lost lock | `109.007-T` L1a/L1b/L1c |
| L2 no stranded newer work | Stale acquired rejection releases indexing and returns to the bounded driver; old-generation cancellation/DB failure clears old state, releases indexing, and bounded-drains only surviving newer-generation work | `109.007-T` L1b plus completion inventory |
| S1 bool-compatible startup | Synchronous `try_start_startup_sync` returns true for initial owner and producer-reacquirer, false only when queued under a live holder; public startup callers remain untouched | `109.003-T` race/control |
| API containment | Public `DispatchSnapshot` and public `run_with_shutdown` callers remain unchanged; transition/generation/claim types are crate-private and opaque | Structural gate |
| Compatibility | Final internal production search has zero unqualified publisher callers. Retained external wrappers and a `tools/mod.rs` shared-seam rustdoc correction may remain an explicit non-blocking P2 if proof/fix would exceed a cap | Final structural gate |
| Contract/build safety | No normal response/error change; every GREEN leaves a buildable intermediate state and every touched production function counts | Per-task review |

## Task Table

| Order | ID | Responsibility | Files | RED scenarios | Touched production functions | Release-behavior production changes | Target |
|---:|---|---|---|---:|---:|---:|---|
| 1 | `109.001-T` | RED atomic transition/token/cancel/floor | `state.rs`, `lifecycle.rs` | 3 | exactly 1: cfg(test) pause in `lifecycle::set_workspace` | 0 | 80-95 min |
| 2 | `109.002-T` | GREEN atomic state+lifecycle transition only | `state.rs`, `lifecycle.rs` | 0 | <=4 | atomic transition only | 95-110 min |
| 3 | `109.005-T` | RED qualified snapshot and explicit write publish | `state.rs`, `write.rs` | 3 | exactly 1: cfg(test) pause in `write::sync_workspace` | 0 | 75-90 min |
| 4 | `109.006-T` | GREEN qualified snapshot, synchronous token capture, and token-taking publication | `state.rs`, `write.rs` | 0 | <=4 | snapshot/capture/publish/write | 85-105 min |
| 5 | `109.007-T` | RED full-mask lost/acquired claim safety | `state.rs`, `lifecycle.rs` | 3 | exactly 1: cfg(test) pause in `lifecycle::drain_pending_sync` | 0 | 75-90 min |
| 6 | `109.008-T` | GREEN claim resolution and all completion-path release/drain | `state.rs`, `lifecycle.rs` | 0 | <=4 | two state functions plus two lifecycle functions | 90-110 min |
| 7 | `109.003-T` | RED startup final-peek handoff | `ipc_server.rs` | 2 | exactly 1: cfg(test) pause in `try_start_startup_sync` | 0 | 65-80 min |
| 8 | `109.004-T` | GREEN synchronous bool-compatible startup handoff | `ipc_server.rs` | 0 | <=3 private functions | private helper only; public callers untouched | 60-90 min |

Every task is <=2 files, <=3 scenarios, and <=110 minutes. The state-owning GREENs remain capped at <=4 touched production functions; startup has the stronger one-file, <=3-private-function cap. A private cfg(test) seam changes the enclosing production function text and therefore counts exactly once on its RED card/table even though release-behavior production changes remain exactly zero. Each seam is private, cfg(test)-only, not serialized, not feature-enabled, and absent from release control flow.

## Implementation Units

### Unit 1 - RED atomic transition (`109.001-T`)

Add exactly three deterministic private state/lifecycle scenarios:

1. G1a pauses actual `lifecycle::set_workspace` after the pre-GREEN binding/config write but before generation/cancel/floor publication and proves the torn tuple. After GREEN, the same pause is after the single transition, so observers see the complete old tuple or complete new tuple.
2. G1b seeds private test state at `sync_generation == u64::MAX`, attempts transition, and proves checked failure returns before binding, config, generation, cancel sender, hydration-ready, or queue floor changes.
3. G1c performs consecutive transitions and proves the returned token owns the exact generation-specific receiver, the old receiver is cancelled, the new receiver remains live, and the queue floor advances without stale flags.

The only production function touched by RED is `lifecycle::set_workspace`, because the private cfg(test) pause is inside it. Count: exactly one. Release-behavior production changes: zero. Use barriers/explicit state steps only; no sleeps, real daemon/IPC, process-global mutation, or public seam. GREEN `109.002-T` must make all three scenarios pass without deferred follow-up.

### Unit 2 - GREEN atomic state+lifecycle transition (`109.002-T`)

In `src/server/state.rs` and `src/tools/lifecycle.rs` only:

- add one crate-private transition that validates capacity, loads the generation and applies `checked_add(1)` before mutation, then coherently publishes workspace/config, new generation, new cancel sender, and pending queue floor;
- return one crate-private transition token containing the opaque generation token and its generation-specific cancel receiver;
- have `lifecycle::set_workspace` consume the token exactly once when spawning hydration, with no generation reread or separate begin call; and
- keep qualified dispatch snapshots, explicit publish/reacquire, claim/re-arm, and legacy publisher compatibility out of this unit.

The maximum four production functions are: (1) the new state transition, (2) one transition-token consumer/into-parts method, (3) retiring the separate `begin_scan_generation` path, and (4) `lifecycle::set_workspace`. Every added, modified, removed, renamed, or visibility-changed production function counts. `set_workspace_and_config` may remain only as an existing test/library compatibility seam; production lifecycle no longer calls it. Cap: <=4 functions, <=2 files, 95-110 minutes. Stop for a fifth function, third file, unchecked increment, partial mutation on exhaustion, public token internals, publisher migration, or non-buildable intermediate state.

### Unit 3 - RED qualified sync snapshot/publish (`109.005-T`)

Add exactly three deterministic private state/write scenarios:

1. W1a races rebind against the new crate-private qualified sync snapshot and proves its wrapped public `DispatchSnapshot` workspace/config pair and opaque generation token all belong to one transition. The public `DispatchSnapshot` shape remains unchanged.
2. W1b pauses `write::sync_workspace` after failed CAS and before publication, advances G+1, resumes with captured G, and proves older publication is ignored with no relabel, heavy leak, or spurious reacquire.
3. W1c is a stable same-G control proving full-mask OR coalescing, producer-reacquire ownership, and the existing queued status/message and revalidate/backfill behavior.

The only production function touched by RED is `write::sync_workspace`, because the private cfg(test) pause is inside it exactly between failed CAS and publication. Count: exactly one. Release-behavior production changes: zero. No sleep, daemon/IPC, process-global mutation, public seam, mismatch retry, exhaustion, or new-error scenario. GREEN `109.006-T` must make all three scenarios pass.

### Unit 4 - GREEN qualified sync snapshot/publish (`109.006-T`)

In `src/server/state.rs` and `src/tools/write.rs` only:

- leave public `DispatchSnapshot` and `snapshot_dispatch_context` source-compatible and unchanged in shape;
- add a crate-private `QualifiedSyncSnapshot` or equivalent result that wraps `DispatchSnapshot` plus an opaque generation token captured while the ordered workspace/config read guards are held;
- add a synchronous crate-private method that captures an opaque token for the current generation, allowing startup to capture before CAS without changing any public caller or awaiting;
- adapt one state publish/reacquire operation to require that exact token and apply newer-replaces, equal-full-mask-OR, and older-ignored before CAS reacquire; and
- have `write::sync_workspace` carry its qualified-snapshot token directly through failed-CAS publication/reacquire.

The four-function budget is: (1) qualified snapshot construction, (2) synchronous current-generation-token capture, (3) token-taking publish/reacquire, and (4) `write::sync_workspace`. The generation token inner value remains private to `state.rs`. This task does not have a mandatory requirement to modify either retained public legacy arm/publish wrapper, much less both. Final internal production search after Unit 8 must still find zero unqualified publisher callers. A complete floor-safe proof for retained external wrappers, or correction of `tools/mod.rs` public shared-seam rustdoc if qualified sync uses a direct crate-private state wrapper, is a non-blocking P2 follow-up when it would require a fifth function or third production file. No field is added to public `DispatchSnapshot`; do not add `tools/mod.rs`. No mismatch retry, retry budget, exhaustion error, later-generation fallback, response change, extra pre-snapshot lock, fifth function, or third file. Cap: <=4 functions, <=2 files, 85-105 minutes.

### Unit 5 - RED full-mask claim safety (`109.007-T`)

Add exactly three deterministic private lifecycle scenarios:

1. L1a claims the full G mask, advances G+1, loses the indexing lock, resumes exact-G re-arm, and proves zero stale pending/revalidate/backfill relabel or leak.
2. L1b claims G, advances the exact workspace/config/generation snapshot to G+1 with newer-generation work pending, then acquires the indexing lock. Validation must reject G before path construction, routine sync, revalidation, or backfill, always release `indexing_in_progress`, and return to the existing bounded driver so the surviving newer work drains; no permanent indexing flag or stranded pending work survives.
3. L1c keeps the snapshot at G, loses the lock, re-arms the exact full mask, and drains it exactly once under the existing 64-pass bound.

The only production function touched by RED is `lifecycle::drain_pending_sync`, because the private cfg(test) pause is inside it between claim and lock-outcome/validation. Count: exactly one. Release-behavior production changes: zero. No sleep, real daemon/IPC, public seam, or fourth scenario. GREEN `109.008-T` must make all three scenarios pass.

### Unit 6 - GREEN full-mask claim/validate/re-arm (`109.008-T`)

In `src/server/state.rs` and `src/tools/lifecycle.rs` only:

- atomically claim pending/revalidate/backfill as a crate-private opaque `PendingSyncClaim` with private fields;
- consolidate state-side claim handling into at most two functions total: one atomic claim function and one claim-resolution function that validates an acquired claim against the exact `QualifiedSyncSnapshot` or exact-rearms a lost-lock claim with older-ignore, equal-full-mask-OR, and newer-untouched semantics;
- in `lifecycle::drain_pending_sync`, reject stale acquired claims before path/sync/heavy work, always call `finish_indexing`, and return control to the existing bounded 64-pass driver so surviving newer-generation work continues draining; preserve same-G exactly-once behavior and hold no pending guard across await; and
- in `lifecycle::background_db_hydration`, make cancellation and DB-connect-failure completion clear only the old generation, release indexing, and invoke the bounded drain for any surviving newer-generation pending work. The old torn generation must be cleared before release/drain and must never execute.

The four-function budget is: (1) atomic claim, (2) combined claim validation/exact re-arm, (3) `lifecycle::drain_pending_sync`, and (4) `lifecycle::background_db_hydration`. No claim accessor is added; validation returns only non-token work flags. Caller migration must fit these touches. Cap: <=4 functions, <=2 files, 90-110 minutes. Stop for a fifth function, third file, public claim field, recursion, unbounded drain, any permanent `indexing_in_progress` flag, torn-old-state drain, pending mutex guard across await, or non-buildable intermediate state. Retained external-wrapper proof and `tools/mod.rs` rustdoc may remain non-blocking P2 rather than violating caps.

### Unit 7 - RED startup handoff (`109.003-T`)

Add exactly two private `ipc_server.rs` scenarios: the final-peek race and a no-contention control. The race synchronously captures opaque qualified G before initial CAS, fails CAS, pauses between failure and publication, lets the holder release and finish its final empty peek, then resumes exact-G token-taking publication/reacquire. It asserts that `try_start_startup_sync` returns `true` to the producer-reacquirer, the unchanged caller runs the startup body once, and the existing completion helper releases and bounded-drains. The control acquires directly and returns `true` without queueing; a queued-under-live-holder observation returns `false`.

The only production function touched by RED is `try_start_startup_sync`, because the private cfg(test) pause is inside it exactly between failed CAS and publication. Count: exactly one. Release-behavior production changes: zero. The seam is not public, serialized, feature-enabled, or present in release control flow. GREEN `109.004-T` must make both scenarios pass.

### Unit 8 - GREEN startup handoff (`109.004-T`)

In `src/daemon/ipc_server.rs` only:

- keep `try_start_startup_sync` synchronous and bool-compatible;
- synchronously capture the opaque current-generation token established by `109.006-T` before the initial startup CAS;
- on CAS failure, call the established token-taking publish/reacquire and return `true` for a producer-reacquirer, while returning `false` only when work remains queued under a live holder; and
- leave public `run_with_shutdown`/startup callers untouched. Existing callers therefore run the startup body exactly once for either initial ownership or producer-reacquisition and use their existing completion helper to release indexing and bounded-drain. Retry/backoff, registry ingestion, embedding backfill, flush, watcher flow, and responses remain unchanged.

Only the private `try_start_startup_sync` production helper is required to change. At most private test/outcome helpers may consume the remaining slots; no public function or caller changes. Cap: one production file, <=3 private functions, 60-90 minutes. Stop for `state.rs`, any second file, a fourth or non-private function, asynchronous token capture, post-CAS generation capture, public caller edit, duplicate startup body/drain, recursion, compatibility-wrapper work, public token bypass, or response change.

## Guard and Lock-Order Contract

- The only async guard held across another lock-acquisition await is a Tokio `active_workspace` guard while acquiring `workspace_config` in explicit `workspace -> config` order. The transition may then acquire `scan_cancel` only under the single documented `workspace -> config -> cancel` order. These guards are held together only long enough to publish or clone the coherent tuple; all readers use the same workspace-before-config order.
- The qualified snapshot reader may hold the Tokio workspace read guard while awaiting the config read guard, read generation while both are stable, then drop both before return. The existing public snapshot reader keeps the same workspace-before-config order and public shape.
- No `std::sync::MutexGuard<PendingSyncState>` may cross an `.await`. The transition acquires the pending mutex only after all required async guards have been acquired and when no further await remains, updates the queue floor, and drops it. Claim/publish/re-arm copy state under the mutex, drop it, and only then perform snapshot, lock, I/O, sync, or drain awaits.
- No async or synchronous guard may cross DB/file I/O, routine sync, revalidation, backfill, or drain awaits. Any additional guard order or guard-across-await is a STOP.

## Production Publication and Completion Inventory

| Production site | Final disposition | Preservation/coverage map |
|---|---|---|
| `lifecycle::background_db_hydration` cancellation completion | Clear only the old generation, `finish_indexing`, then bounded-drain any surviving newer-generation pending work; never execute torn old state | G1a/G1c plus L2 structural/targeted coverage |
| `lifecycle::background_db_hydration` DB-connect-failure completion | Clear only the old generation, `finish_indexing`, then bounded-drain any surviving newer-generation pending work even though the old hydration DB connect failed | G1b/floor plus L2 DB-failure coverage |
| `lifecycle::background_db_hydration` normal completion | `finish_indexing`, then `drain_pending_sync_to_completion` | L1c and bounded-64-pass preservation |
| `lifecycle::drain_pending_sync` validated completion | After validated work, `finish_indexing`; outer bounded driver observes any re-arm | L1a/L1c |
| `lifecycle::drain_pending_sync` stale acquired rejection | No path/sync/heavy work; always `finish_indexing` and return to bounded driver so surviving newer work drains | L1b; no permanent indexing flag |
| `write::sync_workspace` producer-reacquirer | Exact captured token publishes; reacquirer calls `finish_indexing` and bounded drain once | W1b/W1c; queued response remains frozen |
| `write::finalize_indexing_request` for `index_workspace` and `sync_workspace` | Always `finish_indexing`, invoke injected bounded drain, then finish progress | Existing finalizer tests plus targeted W1 gates; order unchanged |
| `ipc_server::try_start_startup_sync` | Synchronous bool helper returns true for initial owner/reacquirer and false for queued-under-holder | S1 race/control; public callers untouched |
| `ipc_server::finish_indexing_and_drain_pending_sync` from startup/watcher | Existing shared helper releases and bounded-drains; unchanged | Existing regressions plus S1 exactly-one-finisher assertion |
| Retained external unqualified wrappers | No internal production callers after Unit 8. Complete floor-safe proof and `tools/mod.rs` shared-seam rustdoc correction may be deferred as explicit non-blocking P2 if caps would be exceeded | Final production-caller search |

Each GREEN introduces its crate-private primitive before a later consumer needs it. No task may leave a signature mismatch for its successor. Any unclassified production finisher, publisher, drain, or second queue returns the affected task and `104-S` blocked. P2 compatibility/doc follow-up never relaxes zero internal production callers or widens Units 4, 6, or 8.

## Compatibility API Migration Contract

- Internal write, lifecycle, and startup paths are token-qualified only. Final structural search after Unit 8 must find zero production callers of an unqualified pending publisher.
- `109.006-T` establishes the qualified snapshot, synchronous current-generation-token capture, token-taking publish/reacquire, and direct write caller within four functions. It is not required to modify either retained public legacy arm/publish wrapper.
- `109.008-T` fits atomic claim plus combined validation/exact-rearm into two state functions and uses its two lifecycle slots for `drain_pending_sync` and `background_db_hydration`; no fifth function is available for wrapper compatibility.
- `109.004-T` changes only private `try_start_startup_sync`; public `run_with_shutdown` callers remain untouched.
- Retained external wrappers cannot accept/expose opaque captured tokens and are unused by internal production. If complete floor-safe proof, deprecation wording, or `tools/mod.rs` public shared-seam rustdoc correction requires a fifth function or third production file, record an explicit non-blocking P2 follow-up. Do not add `tools/mod.rs`, widen a task, or weaken the zero-internal-caller gate.

## Dependency Graph

```text
109.001-T -> 109.002-T -> 109.005-T -> 109.006-T
    -> 109.007-T -> 109.008-T -> 109.003-T -> 109.004-T
```

This order is authoritative and is also the order stored in `104-S.custom_fields.items` after the covering feature.

## Verification Plan

Stage validates artifact structure only. Ship records compiling RED evidence before each paired GREEN and must not advance until the full RED is green in its immediate successor:

1. `109.001-T` is fully green in `109.002-T`.
2. `109.005-T` is fully green in `109.006-T`.
3. `109.007-T` is fully green in `109.008-T`.
4. `109.003-T` is fully green in `109.004-T`.

Structural review must prove:

1. public `DispatchSnapshot` and public startup callers are unchanged; qualified snapshot/token/claim types are crate-private and opaque;
2. `109.006-T` includes synchronous current-generation-token capture and no mandatory legacy-wrapper edits; no retry, exhaustion error, later-generation fallback, or response change exists;
3. exact `workspace -> config` and transition-only `workspace -> config -> cancel` order is documented and no synchronous pending guard crosses await;
4. stale acquired rejection always releases indexing and continues the bounded drain, while cancellation/DB-failure completion clears old state, releases, and drains only surviving newer work;
5. each RED records exactly one touched production function for its cfg(test) seam and zero release-behavior production changes;
6. Unit 6 uses at most two state functions plus `drain_pending_sync` and `background_db_hydration`; final production search after Unit 8 finds zero unqualified publisher callers;
7. Unit 8 touches only `src/daemon/ipc_server.rs`, no more than three private production functions, keeps `try_start_startup_sync` synchronous/bool-compatible, and leaves public callers untouched;
8. every task is <=2 files, <=3 scenarios, <=110 minutes, and within its production-function cap; and
9. `102-S`, `103-S`, and `104-S` share `custom_fields.operator_batch: 102-104-integration` while preserving `operator_order` 1/2/3.

Any P0/P1 failure returns the affected task and `104-S` blocked. The explicit retained-wrapper/`tools/mod.rs` P2 does not authorize cap widening. Stage does not run builds, tests, or linters.

## Risks, Rollback, and Monitoring

- Risk: torn binding/generation/cancel/floor. Mitigation: checked single transition token consumed once.
- Risk: stale write intent relabeled. Mitigation: qualified wrapper plus exact token publication, no retry/fallback.
- Risk: stale acquired lifecycle work mutates the new binding or leaves indexing held. Mitigation: exact qualified-snapshot validation before work, unconditional release on rejection, and bounded-driver continuation.
- Risk: stale lost-lock mask leaks. Mitigation: atomic full-mask claim and exact-token re-arm.
- Risk: cancellation/DB failure strands newer work. Mitigation: clear only old generation, release indexing, then bounded-drain surviving newer work without executing torn old state.
- Risk: startup lost wakeup or duplicate body. Mitigation: synchronous bool-compatible private helper, pre-CAS token capture, and unchanged caller/completion flow.

Rollback is release-unit commit revert plus daemon restart. Do not partially revert one dependent unit. No workspace migration or repair is planned.

| SLI | Healthy | Block/rollback trigger | Owner/window |
|---|---|---|---|
| Transition fixtures | coherent token/cancel/floor and checked exhaustion | torn state or partial overflow mutation | Ship; targeted gate |
| Write producer | exact captured token; unchanged queued response | retry/error path, later-G fallback, relabel, or heavy leak | Ship; targeted gate |
| Lifecycle claim/completion | all three scenarios and cancel/DB-failure completion pass | stale work, stranded newer work, held indexing flag, or same-G loss | Ship; targeted gate |
| Startup | exactly one finisher; seam test-only | stranded request, duplicate body, or release seam | Ship; targeted gate |
| Runtime drain | no attributable bound warning within existing 30-second debug budget | warning or controlled restart over budget | Ship; three restarts plus 15 min |

## Constitution Check

- **Safety-First Rust:** uses existing typed `EngramError` propagation for exhaustion, adds no unsafe code or dependency, and keeps token internals crate-private.
- **Test-First Development:** four compiling RED cards precede and are fully closed by their immediate GREEN successors.
- **Workspace isolation / CLI containment:** runtime changes remain inside daemon state/lifecycle/write/IPC; Stage changes only scoped repository artifacts.
- **Structured observability:** deterministic scenarios, complete finisher/publisher inventory, monitoring thresholds, rollback, and blocked-return gates are explicit.
- **Single responsibility / task granularity:** every task is one concern, <=2 files, <=3 scenarios, and <=110 minutes; state-owning GREENs remain <=4 functions while startup is `ipc_server.rs`-only and <=3 private functions.
- **Git-friendly persistence / context efficiency:** plan, backlog, review, checkpoint, and memory stay human-readable and queryable through backlogit.
- **Merge history:** no merge operation is part of Stage scope. Ship retains the repository merge-commit rule.

No constitutional exception or justified violation is required.

## Plan Hardening

Hardening is required because partial concurrency changes can silently run work for the wrong binding. Reinforcing context: strict-safety and concurrency instructions; packed-atomic-clear, all-finish-site drain, and take-before-lock compound learnings.

Protected invariants: checked transition ownership, opaque crate-private tokens, synchronous pre-CAS token capture, unchanged public snapshot and startup callers, exact snapshot/claim equality, full-mask atomicity, explicit guard order, unconditional indexing release on stale rejection, cancellation/DB-failure drain of surviving newer work only, zero final internal unqualified callers, no retry-generated response behavior, bounded drain, one startup finisher, buildable intermediate states, and frozen public/wire contracts. Retained external-wrapper floor proof and `tools/mod.rs` shared-seam rustdoc are explicit non-blocking P2 only when cap-safe completion is impossible.

**ProposedAction PA-1**
- summary: introduce atomic transition and qualified sync tokens, including synchronous current-generation-token capture
- targets: `state.rs`, `lifecycle.rs`, `write.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-2**
- summary: generation-resolve lifecycle claims and make cancellation/DB-failure completion release then drain surviving newer work
- targets: `state.rs`, `lifecycle.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-3**
- summary: update only private synchronous startup arbitration to use established token APIs while public callers remain unchanged
- targets: `ipc_server.rs`
- change_kind: bounded daemon startup coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

## Runtime Verification and Closure

Ship uses targeted fixtures and controlled daemon restarts only. Stage performed no implementation or runtime verification. Monitoring and rollback records must carry into closure; no operator workspace mutation is required or authorized by this plan.

## Harvest Shape

Existing `104-S` and `109-F` remain. No new task or shipment is created. Manifest and dependency order:

1. `109-F`
2. `109.001-T`
3. `109.002-T`
4. `109.005-T`
5. `109.006-T`
6. `109.007-T`
7. `109.008-T`
8. `109.003-T`
9. `109.004-T`

Keep `custom_fields.operator_batch: 102-104-integration` matching `102-S` and `103-S`, and preserve `operator_order: 3`. Do not touch `025-S`, `081-S`, `015-D`, or `017-D`.

## Plan Review - Third Current-Content Review Cycle: PASS

**Review mode:** fresh complete Stage review of the current artifact content and queued state under the configured `.Stage` model with no override. Acceptance is not bound to a Git HEAD. Plan hardening remains satisfied.

**Gate:** **PASS WITH NON-BLOCKING P2 FOLLOW-UP**. Open P0: none. Open P1: none. Open P2: retained external-wrapper floor proof and possible `tools/mod.rs` shared-seam rustdoc correction, cap-dependent. Open P3: none.

### Guardrail disposition and evidence

- P1 startup resolved: `109.006-T` first establishes synchronous crate-private opaque current-generation-token capture. `109.004-T` remains `src/daemon/ipc_server.rs` only, <=3 private functions, synchronous/bool-compatible, captures before CAS, returns true for initial owner/reacquirer, and leaves public startup callers/completion flow untouched.
- P1 lifecycle resolved: `109.008-T` uses at most two state functions plus `drain_pending_sync` and `background_db_hydration`. Stale acquired rejection always releases indexing and continues the bounded drain. Cancellation/DB failure clears old-generation state, releases indexing, and drains surviving newer work without executing torn old state.
- P1 wrapper-cap conflict resolved: `109.006-T` is not required to modify both public legacy wrappers. Final internal production search remains zero unqualified callers.
- P2 explicit follow-up: retained external-wrapper floor-safe proof and `tools/mod.rs` public shared-seam rustdoc correction may be deferred if they require a fifth function or third production file. No task cap is widened and `tools/mod.rs` is not added.
- P3 structure: `102-S`, `103-S`, and `104-S` share `custom_fields.operator_batch: 102-104-integration`; `operator_order` remains 1/2/3. Statuses/dependencies remain unchanged and topological. Invalid checkpoint `checkpoint-20260801-065720.json` remains excluded.

### Persona decisions

- **Constitution Reviewer: PASS** - every task is <=2 files, <=3 scenarios, <=110 minutes, and within its function cap.
- **Rust/Concurrency Reviewer: PASS** - pre-CAS opaque capture, exact claim resolution, unconditional stale-release, bounded continuation, and no pending guard across await are explicit.
- **Scope Boundary Auditor: PASS** - startup is one production file/private-only; lifecycle is two files/four functions; no third production file or public caller edit.
- **Learnings Researcher: PASS** - packed-mask atomicity, take-before-lock re-arm, bounded drain, and all-finish-site release/drain coverage are preserved.
- **Architecture Strategist: PASS** - qualified primitives precede lifecycle/startup consumers in buildable task order.
- **Agent-Native Parity Reviewer: PASS** - public snapshots, startup callers, queued status/message, CLI/MCP responses, and errors remain frozen.
- **Security Lens Reviewer:** not triggered.

**Decision:** keep `104-S`, `109-F`, and all eight tasks queued in the unchanged authoritative order. Return the affected task and shipment blocked on any P0/P1 cap, API-containment, guard-order, completion, buildability, zero-internal-caller, or RED-to-GREEN breach. Track the stated P2 without widening implementation scope.
