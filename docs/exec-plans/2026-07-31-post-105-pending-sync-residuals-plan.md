---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
updated: 2026-08-01
status: "reviewed (Stage startup-guardrail correction PASS)"
source: "docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue and the R2 producer/finisher backstop. Current code still separates `set_workspace_and_config` from `begin_scan_generation`, lets `write.rs` publish after a coherent workspace/config capture using a later generation read, lets lifecycle execute a stale full-mask claim after an acquired-lock race, and leaves startup publication after failed CAS without a deterministic RED pause point.

The operator-authoritative guardrail requires `109.004-T` to remain an `src/daemon/ipc_server.rs`-only GREEN capped at three private production functions. This correction preserves the eight-task chain and the earlier four-pair decomposition while moving compatibility safety back to the state-owning units: `109.006-T` may use its optional fourth pending-state total-order/floor helper, and `109.008-T` may finish state/lifecycle caller migration and documentation without a fifth function. Startup consumes the established qualified API but does not own state compatibility. Scope excludes `015-D`, `017-D`, `025-S`, `081-S`, schema, wire/response, persistence, and unrelated code-graph work. Stage changes planning artifacts only.

## Provenance and Supersession

- Source deliberation: `018-D`; source stash: `FF55E51A`, `88EB5FB1`, `1E70A289`.
- Archived `105-F` remains immutable.
- This cycle supersedes only the widened startup wording from the latest remediation. It preserves the corrected wrapper, guard, completion-inventory, and <=4 caps for the earlier state-owning GREENs while restoring the stronger Unit 8 one-file/three-private-function guardrail.
- `102-S` and `103-S` remain separate release units. `104-S` keeps `operator_order: 3`.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| G1 atomic rebind | State publishes workspace/config, checked next generation, cancel ownership, and queue floor in one transition and returns one crate-private token with the generation-specific cancel receiver; lifecycle consumes it once | `109.001-T` G1a/G1c |
| G2 fail-closed exhaustion | Generation uses `checked_add(1)`; `u64::MAX` returns an existing typed system error before any workspace, config, generation, cancel, queue, or hydration-ready mutation | `109.001-T` G1b |
| W1 qualified write producer | New crate-private `QualifiedSyncSnapshot` wraps an unchanged public `DispatchSnapshot` plus an opaque generation token captured in the same coherent read window | `109.005-T` W1a |
| W2 explicit publication | Token-taking publish/reacquire applies newer-replaces, equal-full-mask-OR, and older-ignored semantics; `write.rs` carries the captured token without retry or fallback | `109.005-T` W1b/W1c |
| L1 generation-owned claim | State claims pending/revalidate/backfill atomically as an opaque token; lifecycle validates against the exact qualified snapshot before acquired-lock work or republishes the exact claim after lost lock | `109.007-T` L1a/L1b/L1c |
| S1 startup handoff | Startup captures the qualified generation before CAS and uses token-taking R2 publication with exactly one finisher | `109.003-T` race/control |
| API containment | Public `DispatchSnapshot` gains no field; transition/generation/claim types are crate-private and token/claim internals are opaque | Structural gate |
| Compatibility | Compatibility is outside `109.002-T` and startup: `109.006-T` owns any needed floor-aware total-order helper; `109.008-T` owns state/lifecycle caller migration and deprecation documentation without a fifth function; final production search has zero unqualified publisher callers | `109.006-T`/`109.008-T` plus final structural gate |
| Contract/build safety | No normal response/error change; every GREEN leaves a buildable intermediate state and every touched production function counts | Per-task review |

## Task Table

| Order | ID | Responsibility | Files | RED scenarios | Touched production functions | Release-behavior production changes | Target |
|---:|---|---|---|---:|---:|---:|---|
| 1 | `109.001-T` | RED atomic transition/token/cancel/floor | `state.rs`, `lifecycle.rs` | 3 | exactly 1: cfg(test) pause in `lifecycle::set_workspace` | 0 | 80-95 min |
| 2 | `109.002-T` | GREEN atomic state+lifecycle transition only | `state.rs`, `lifecycle.rs` | 0 | <=4 | atomic transition only | 95-110 min |
| 3 | `109.005-T` | RED qualified snapshot and explicit write publish | `state.rs`, `write.rs` | 3 | exactly 1: cfg(test) pause in `write::sync_workspace` | 0 | 75-90 min |
| 4 | `109.006-T` | GREEN qualified snapshot, token handoff, and floor-aware compatibility primitive | `state.rs`, `write.rs` | 0 | <=4 | wrapper/publish/reacquire plus optional floor helper | 85-105 min |
| 5 | `109.007-T` | RED full-mask lost/acquired claim safety | `state.rs`, `lifecycle.rs` | 3 | exactly 1: cfg(test) pause in `lifecycle::drain_pending_sync` | 0 | 75-90 min |
| 6 | `109.008-T` | GREEN full-mask claim/validate/exact re-arm and state/lifecycle caller migration | `state.rs`, `lifecycle.rs` | 0 | <=4 | claim/validate/re-arm; migration/docs add no fifth function | 90-110 min |
| 7 | `109.003-T` | RED startup final-peek handoff | `ipc_server.rs` | 2 | exactly 1: cfg(test) pause in `try_start_startup_sync` | 0 | 65-80 min |
| 8 | `109.004-T` | GREEN startup token handoff only | `ipc_server.rs` | 0 | <=3 private functions | startup only; no state compatibility | 60-90 min |

Every task is <=2 files, <=3 scenarios, and <=110 minutes. The earlier state-owning GREENs remain capped at <=4 touched production functions; startup has the stronger one-file, <=3-private-function cap. A private cfg(test) seam changes the enclosing production function text and therefore counts exactly once on its RED card/table even though release-behavior production changes remain exactly zero. Each seam is private, cfg(test)-only, not serialized, not feature-enabled, and absent from release control flow.

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
- adapt the state publish/reacquire operation to require that exact token and apply newer-replaces, equal-full-mask-OR, and older-ignored under the pending mutex before CAS reacquire;
- if retained fresh-at-call public compatibility wrappers need floor safety, use the optional existing `PendingSyncState` total-order/floor helper as the fourth function slot so a transition that advances the floor before queue lock makes the older operation a no-op rather than a relabel; and
- have `write::sync_workspace` carry the captured token directly through failed-CAS publication/reacquire.

At most four production functions may be touched: qualified snapshot construction, token-taking publish/reacquire, `write::sync_workspace`, and only if necessary one existing pending-state total-order/floor helper. That optional helper owns retained-wrapper floor semantics; startup owns none of them. Wrapper fields may be crate-visible for transport, but the generation token inner value remains private to `state.rs`. No field is added to public `DispatchSnapshot`. No mismatch retry, retry budget, exhaustion error, later-generation fallback, response change, extra pre-snapshot lock, fifth function, or third file. Cap: <=4 functions, <=2 files, 85-105 minutes.

### Unit 5 - RED full-mask claim safety (`109.007-T`)

Add exactly three deterministic private lifecycle scenarios:

1. L1a claims the full G mask, advances G+1, loses the indexing lock, resumes exact-G re-arm, and proves zero stale pending/revalidate/backfill relabel or leak.
2. L1b claims G, advances the exact workspace/config/generation snapshot to G+1, then acquires the indexing lock and proves validation rejects G before path construction, routine sync, revalidation, or backfill.
3. L1c keeps the snapshot at G, loses the lock, re-arms the exact full mask, and drains it exactly once under the existing 64-pass bound.

The only production function touched by RED is `lifecycle::drain_pending_sync`, because the private cfg(test) pause is inside it between claim and lock-outcome/validation. Count: exactly one. Release-behavior production changes: zero. No sleep, real daemon/IPC, public seam, or fourth scenario. GREEN `109.008-T` must make all three scenarios pass.

### Unit 6 - GREEN full-mask claim/validate/re-arm (`109.008-T`)

In `src/server/state.rs` and `src/tools/lifecycle.rs` only:

- claim pending/revalidate/backfill in one locked state operation as a crate-private opaque `PendingSyncClaim` with private fields;
- on acquired lock, consume the claim through a state validator against the exact `QualifiedSyncSnapshot`; only a match yields crate-private work flags, and a stale claim performs no path construction, routine sync, revalidation, or backfill;
- on lost lock, consume the exact claim through exact re-arm: older ignored, equal full-mask OR-coalesced, newer ownership untouched;
- preserve same-G exactly-once drain, the bounded 64-pass driver, and every completion site in the inventory below; and
- complete state/lifecycle production-caller migration to qualified claim/re-arm APIs and module-level deprecation documentation for retained fresh-at-call wrappers. This structural migration must reuse the four listed touches and may not add a fifth function.

The four-function budget is: (1) claim API, (2) acquired-claim validator/work decoder, (3) exact-claim re-arm, and (4) `lifecycle::drain_pending_sync`. No claim accessor is added; the validator returns non-token work flags so claim fields remain private. Documentation outside function bodies does not consume a function slot. Cap: <=4 functions, <=2 files, 90-110 minutes. Stop for a fifth function, third file, public claim field, recursion, unbounded drain, pending mutex guard across await, or non-buildable intermediate state. If retained external-wrapper compatibility cannot be proven inside the existing Unit 4/Unit 6 caps, preserve the zero-internal-caller gate and record the residual proof as a non-blocking P2 follow-up; never transfer it to startup.

### Unit 7 - RED startup handoff (`109.003-T`)

Add exactly two private `ipc_server.rs` scenarios: the final-peek race and a no-contention control. The race captures qualified G before initial CAS, fails CAS, pauses between failure and publication, lets the holder release and finish its final empty peek, then resumes exact-G publication. It asserts exactly one guaranteed finisher without watcher/index/sync/timer help. The control acquires directly and never queues.

The only production function touched by RED is `try_start_startup_sync`, because the private cfg(test) pause is inside it exactly between failed CAS and publication. Count: exactly one. Release-behavior production changes: zero. The seam is not public, serialized, feature-enabled, or present in release control flow. GREEN `109.004-T` must make both scenarios pass.

### Unit 8 - GREEN startup handoff (`109.004-T`)

In `src/daemon/ipc_server.rs` only:

- obtain the qualified generation before initial startup CAS and pass that exact opaque token to the established token-taking publish/reacquire API after failure;
- return a private initial-owner, queued-under-holder, or producer-reacquired outcome; only the initial owner runs startup work, while a reacquirer releases and drains exactly once without duplicate startup work; and
- preserve the shared finish-and-drain helper used by startup and watcher, startup retry/backoff, registry ingestion, embedding backfill, flush, and responses.

At most three private production functions may be touched: `try_start_startup_sync`, private startup control flow in `run_with_shutdown_v2`, and only if needed one private outcome adapter. Cap: one production file, <=3 private functions, 60-90 minutes. Stop for `state.rs`, any second file, a fourth or non-private function, post-failure generation capture, duplicate startup body/drain, recursion, compatibility-wrapper work, public captured-token bypass, or response change.

## Guard and Lock-Order Contract

- The only async guard held across another lock-acquisition await is a Tokio `active_workspace` guard while acquiring `workspace_config` in explicit `workspace -> config` order. The transition may then acquire `scan_cancel` only under the single documented `workspace -> config -> cancel` order. These guards are held together only long enough to publish or clone the coherent tuple; all readers use the same workspace-before-config order.
- The qualified snapshot reader may hold the Tokio workspace read guard while awaiting the config read guard, read generation while both are stable, then drop both before return. The existing public snapshot reader keeps the same workspace-before-config order and public shape.
- No `std::sync::MutexGuard<PendingSyncState>` may cross an `.await`. The transition acquires the pending mutex only after all required async guards have been acquired and when no further await remains, updates the queue floor, and drops it. Claim/publish/re-arm copy state under the mutex, drop it, and only then perform snapshot, lock, I/O, sync, or drain awaits.
- No async or synchronous guard may cross DB/file I/O, routine sync, revalidation, backfill, or drain awaits. Any additional guard order or guard-across-await is a STOP.

## Production Publication and Completion Inventory

| Production site | Final disposition | Preservation/coverage map |
|---|---|---|
| `lifecycle::background_db_hydration` cancellation completion | Generation-scoped full-mask clear, then `finish_indexing`; intentionally no drain against torn-down state | G1a/G1c cancel/floor plus structural no-drain assertion |
| `lifecycle::background_db_hydration` DB-connect-failure completion | Generation-scoped full-mask clear, then `finish_indexing`; intentionally no drain without a DB | G1b/floor and existing DB-failure regression coverage |
| `lifecycle::background_db_hydration` normal completion | `finish_indexing`, then `drain_pending_sync_to_completion` | L1c and bounded-64-pass preservation |
| `lifecycle::drain_pending_sync` inner completion | After validated work, `finish_indexing`; outer bounded driver observes any re-arm | L1a/L1b/L1c cover claim, acquired validation, re-arm, and exactly-once drain |
| `write::sync_workspace` producer-reacquirer | Exact captured token publishes; reacquirer calls `finish_indexing` and bounded drain once | W1b/W1c; queued response remains frozen |
| `write::finalize_indexing_request` for `index_workspace` and `sync_workspace` | Always `finish_indexing`, invoke injected bounded drain, then finish progress | Existing finalizer tests plus targeted W1 gates; order unchanged |
| `ipc_server::finish_indexing_and_drain_pending_sync` from startup | Shared helper releases and bounded-drains after initial owner or producer-reacquirer outcome | S1 race/control and exactly-one-finisher assertion |
| `ipc_server::finish_indexing_and_drain_pending_sync` from watcher | Same shared helper after acquired watcher work; untouched by 109-F | Existing watcher completion regressions remain green; structural inventory proves route remains |
| Legacy unqualified `set_pending_sync` / `publish_pending_sync` / companion wrappers | Unit 4 supplies any needed floor-aware helper; Unit 6 migrates state/lifecycle callers and documentation without a fifth function; Unit 8 migrates only the startup call through existing qualified APIs; final production callers are zero | G1c/W1b/W1c, L1 structural migration, S1 startup migration, and final production-caller search |

Each GREEN introduces its crate-private primitive before a later consumer needs it. No task may leave a signature mismatch for its successor. Any unclassified production finisher, publisher, drain, or second queue returns the affected task and `104-S` blocked. If only retained external-wrapper compatibility proof remains after the earlier <=4 caps are exhausted, record that residual as a non-blocking P2 follow-up while keeping zero internal production callers; do not widen Unit 8.

## Compatibility API Migration Contract

- Internal write, lifecycle, and startup paths are token-qualified only. Final structural search after Unit 8 must find zero production callers of an unqualified pending publisher.
- Compatibility is not part of `109.002-T` or startup. No publish/reacquire signature, write caller, `DispatchSnapshot` shape, floor helper, or wrapper documentation may be attributed to the transition or startup units.
- Unit 4 (`109.006-T`) may use its optional fourth `PendingSyncState` total-order/floor helper so retained public wrappers represent fresh-at-call intent and an older operation loses to an advanced floor rather than being relabeled.
- Unit 6 (`109.008-T`) migrates remaining state/lifecycle production callers and completes module-level deprecation documentation within `state.rs` + `lifecycle.rs`, reusing its four function touches and adding no fifth.
- Unit 8 (`109.004-T`) changes only the startup caller in `ipc_server.rs` to consume the already-established qualified API; it performs no state compatibility work.
- Retained wrappers cannot accept or expose an opaque captured generation token and are not used by internal production. If their external compatibility proof cannot be completed within Unit 4/Unit 6 caps, keep the final zero-internal-caller requirement and record a non-blocking P2 compatibility advisory instead of widening startup.

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

1. public `DispatchSnapshot` is unchanged and qualified snapshot/token/claim types are crate-private with opaque token/claim internals;
2. no normal write retry, exhaustion error, later-generation fallback, or response change exists;
3. exact `workspace -> config` and transition-only `workspace -> config -> cancel` order is documented and no synchronous pending guard crosses await;
4. the completion inventory classifies lifecycle cancellation, DB failure, normal hydration, inner drain, write reacquirer/finalizer, and IPC startup/watcher;
5. each RED records exactly one touched production function for its cfg(test) seam and zero release-behavior production changes;
6. Unit 4 owns any floor-aware compatibility helper, Unit 6 owns state/lifecycle migration and documentation without a fifth function, and final production search after Unit 8 finds zero unqualified publisher callers;
7. Unit 8 touches only `src/daemon/ipc_server.rs` and no more than three private production functions; and
8. every task is <=2 files, <=3 scenarios, <=110 minutes, and within its production-function cap.

Any failure returns the affected task and `104-S` blocked. Stage does not run builds, tests, or linters.

## Risks, Rollback, and Monitoring

- Risk: torn binding/generation/cancel/floor. Mitigation: checked single transition token consumed once.
- Risk: stale write intent relabeled. Mitigation: qualified wrapper plus exact token publication, no retry/fallback.
- Risk: stale acquired lifecycle work mutates the new binding. Mitigation: exact qualified-snapshot validation before any path/sync/heavy action.
- Risk: stale lost-lock mask leaks. Mitigation: atomic full-mask claim and exact-token re-arm.
- Risk: startup lost wakeup or duplicate body. Mitigation: deterministic private seam and exactly one finisher.

Rollback is release-unit commit revert plus daemon restart. Do not partially revert one dependent unit. No workspace migration or repair is planned.

| SLI | Healthy | Block/rollback trigger | Owner/window |
|---|---|---|---|
| Transition fixtures | coherent token/cancel/floor and checked exhaustion | torn state or partial overflow mutation | Ship; targeted gate |
| Write producer | exact captured token; unchanged queued response | retry/error path, later-G fallback, relabel, or heavy leak | Ship; targeted gate |
| Lifecycle claim | all three scenarios pass | stale acquired work, stale leak, or same-G loss | Ship; targeted gate |
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

Protected invariants: checked transition ownership, opaque crate-private tokens, unchanged public snapshot, exact snapshot/claim equality, full-mask atomicity, explicit guard order, floor-aware retained wrappers owned by earlier state units, zero final internal unqualified callers, no retry-generated response behavior, bounded drain, one startup finisher, an `ipc_server.rs`-only <=3-private-function startup unit, buildable intermediate states, and frozen public/wire contracts.

**ProposedAction PA-1**
- summary: introduce atomic transition and qualified sync tokens
- targets: `state.rs`, `lifecycle.rs`, `write.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-2**
- summary: generation-validate lifecycle claims and complete internal publisher migration before startup
- targets: `state.rs`, `lifecycle.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-3**
- summary: finish startup handoff through established qualified APIs without state compatibility work
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

Keep `operator_order: 3`. Do not touch `025-S`, `081-S`, `015-D`, or `017-D`.

## Plan Review - Stage Startup-Guardrail Correction: PASS

**Review mode:** fresh complete Stage review at HEAD `59f53eed6a58e8f3119f7b13b680b95fc81d7863` under the configured `.Stage` model with no override. Plan hardening remains satisfied. Cross-model dispatch was unavailable, so all triggered lenses used the caller Stage model as permitted by the skill.

**Gate:** **PASS**. Open P0: none. Open P1: none. Open P2: none. Open P3: none.

### Guardrail disposition and evidence

- P1 regression resolved: `109.004-T` is restored to `src/daemon/ipc_server.rs` only, 60-90 minutes, and <=3 private startup production-function touches. `state.rs` and compatibility-wrapper work are explicit STOP conditions.
- P1 decomposition proof: current production publisher inventory has one write handoff in `write::sync_workspace`, one lifecycle lost-lock re-arm in `lifecycle::drain_pending_sync`, and one startup handoff in `ipc_server::try_start_startup_sync`. Units 4, 6, and 8 respectively migrate those paths in dependency order.
- P2 compatibility proof: Unit 4 may spend its existing optional fourth slot on one `PendingSyncState` total-order/floor helper. Unit 6 migrates the lifecycle call inside its already-counted `drain_pending_sync` touch and completes module-level documentation without a fifth function. Startup only consumes the resulting qualified API.
- P2 contingency: if retained external-wrapper compatibility proof later requires another earlier-state function, Ship records a non-blocking P2 follow-up while preserving zero internal production unqualified callers. It must not widen Unit 8.
- P3 structure: `104-S.custom_fields.items`, statuses, and dependencies remain unchanged and topological. Invalid checkpoint `checkpoint-20260801-065720.json` remains excluded.

### Persona decisions

- **Constitution Reviewer: PASS** - all four RED/GREEN pairs close immediately; every task is <=2 files, <=3 scenarios, and <=110 minutes, with the stronger Unit 8 one-file/three-private-function cap.
- **Rust/Concurrency Reviewer: PASS** - checked generation, opaque tokens, floor total order, exact claim validation/re-arm, no pending guard across await, and exactly-one-finisher startup are explicit.
- **Scope Boundary Auditor: PASS** - compatibility remains in state-owning Units 4/6; Unit 8 has no `state.rs`, schema, wire/response, persistence, unrelated backlog, or Ship-owned operation.
- **Learnings Researcher: PASS** - packed-mask atomic publication, take-before-lock re-arm, bounded drain, and all-finish-site coverage remain preserved.
- **Architecture Strategist: PASS** - qualified primitives and compatibility safety precede lifecycle and startup consumers in buildable states without a cap increase.
- **Agent-Native Parity Reviewer: PASS** - public `DispatchSnapshot`, queued status/message, CLI/MCP responses, and error behavior remain frozen.
- **Security Lens Reviewer:** not triggered.

**Decision:** keep `104-S`, `109-F`, and all eight tasks queued in the unchanged authoritative order. Return the affected task and shipment blocked on any file/function/scenario/time cap, API-containment, guard-order, inventory, buildability, zero-internal-caller, or RED-to-GREEN closure breach. A residual retained-wrapper proof gap is P2 follow-up only and never startup scope.
