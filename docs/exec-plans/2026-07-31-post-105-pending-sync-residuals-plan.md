---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
updated: 2026-08-01
status: "superseded-blocked (current-head adversarial architecture failure; redesign spike required)"
source: "docs/decisions/2026-08-01-post-105-sync-coordinator-redesign-decision.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue. Current code still allows retained public wrapper publication to race below the queue floor, separates token capture from startup acquisition, permits stale claim paths to hold indexing, and lets background hydration reach DB work before it owns indexing. The prior plan incorrectly treated wrapper floor proof as P2 and combined claim repair with hydration completion.

This correction makes wrapper floor safety P1 and closes it in `109.009-T` and `109.010-T`. It gives write and startup one token-qualified start-or-publish arbitration with explicit `Acquired`, `Reacquired`, `Queued`, and `Stale` outcomes. It keeps `109.007-T` and `109.008-T` claim-only, adds a dedicated hydration ownership pair, and leaves public signatures, responses, and startup callers unchanged. Stage changes planning and backlog artifacts only.

## Provenance, Scope, and Batch Contract

- Source deliberation: `018-D`; source stash: `FF55E51A`, `88EB5FB1`, `1E70A289`.
- Archived `105-F` remains immutable. Excluded scopes `015-D`, `017-D`, `025-S`, `081-S`, blocked work, and stash work remain untouched.
- `102-S`, `103-S`, and `104-S` share `custom_fields.operator_batch: 102-104-integration`, preserve `operator_order` 1, 2, and 3, and carry exact `operator_predecessors` lists `[]`, `[102-S]`, and `[102-S, 103-S]`.
- Predecessor validation is fail closed. Every listed predecessor ID must exist and have positive terminal shipment evidence such as shipped/merged. Absence, omission, blocked, unknown, unqueryable, or non-terminal state is never inferred as complete.
- Every task is `<=2` production files, `<=3` deterministic scenarios, and `<=110 minutes`. Every state GREEN is `<=4` touched production functions. Startup is one file and `<=3` private functions.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| Atomic transition | Publish workspace/config, checked generation, cancel ownership, and queue floor in one token | `109.001-T` -> `109.002-T` |
| Public wrapper floor safety | Retained wrappers use older-ignore, equal-OR, newer-replace; stale rejection cannot reacquire | `109.009-T` -> `109.010-T` |
| Qualified arbitration | One queue-floor-mutex arbitration returns `Acquired/Reacquired/Queued/Stale` | `109.005-T` -> `109.006-T` |
| No stale initial acquisition | Check captured token against floor before initial CAS | `109.005-T` W2 |
| No duplicate routine body | Consume routine-only startup claim on `Reacquired`; preserve owning bit for companions | `109.005-T` W3 and startup pair |
| Claim-only safety | Atomic full-mask claim, combined validate/re-arm, stale acquired release and continuation | `109.007-T` -> `109.008-T` |
| Hydration ownership | Zero DB work before ownership; lost-wake-safe Notify wait/acquire | `109.011-T` -> `109.012-T` |
| Hydration completion | Exact-old clear, release/notify, bounded drain of surviving newer work | `109.011-T` H2/H3 |
| Startup compatibility | Private synchronous bool mapping; public callers untouched | `109.003-T` -> `109.004-T` |
| Contract containment | No public signature, response, wire, schema, or second-queue change | Structural gate |

## Authoritative Task Table

| Order | ID | Responsibility | Production files | Scenarios | Function cap | Target |
|---:|---|---|---|---:|---:|---:|
| 1 | `109.001-T` | RED atomic transition/token/cancel/floor | `state.rs`, `lifecycle.rs` | 3 | RED seam only | `<=110m` |
| 2 | `109.002-T` | GREEN atomic transition | `state.rs`, `lifecycle.rs` | 0 | `<=4` | `<=110m` |
| 3 | `109.009-T` | RED public wrapper floor safety | `state.rs` | 3 | RED seams only | `<=110m` |
| 4 | `109.010-T` | GREEN cap-safe wrapper publication | `state.rs` | 0 | `<=4` | `<=110m` |
| 5 | `109.005-T` | RED token-qualified start-or-publish | `state.rs`, `write.rs` | 3 | RED seams only | `<=110m` |
| 6 | `109.006-T` | GREEN qualified arbitration and write | `state.rs`, `write.rs` | 0 | `<=4` | `<=110m` |
| 7 | `109.007-T` | RED claim-only ownership safety | `state.rs`, `lifecycle.rs` | 3 | RED seam only | `<=110m` |
| 8 | `109.008-T` | GREEN claim-only validation/drain | `state.rs`, `lifecycle.rs` | 0 | `<=3` | `<=110m` |
| 9 | `109.011-T` | RED hydration ownership/completion | `state.rs`, `lifecycle.rs` | 3 | RED seams only | `<=110m` |
| 10 | `109.012-T` | GREEN Notify wait and hydration | `state.rs`, `lifecycle.rs` | 0 | `<=4` | `<=110m` |
| 11 | `109.003-T` | RED private startup outcomes | `ipc_server.rs` | 3 | `<=3` private | `<=110m` |
| 12 | `109.004-T` | GREEN synchronous bool mapping | `ipc_server.rs` | 0 | `<=3` private | `<=110m` |

## Implementation Units

### 1. RED atomic transition (`109.001-T`)

Exactly three barrier-driven state/lifecycle scenarios prove a coherent old-or-new binding tuple, checked `u64::MAX` failure before mutation, and consecutive transition token/cancel/floor ownership. No publisher, claim, hydration ownership, or startup behavior enters this task.

### 2. GREEN atomic transition (`109.002-T`)

In `state.rs` and `lifecycle.rs`, validate generation capacity before mutation, publish workspace/config/generation/cancel/floor coherently, return one opaque crate-private transition token and exact cancel receiver, and consume it once in lifecycle. No pending guard crosses await. At most four production functions and `<=110 minutes`.

### 3. RED retained-wrapper floor (`109.009-T`)

In `state.rs` only, add exactly three deterministic private scenarios:

1. An older generation reaches each retained publication path after the queue floor advanced. Queue generation/mask remain unchanged and stale `publish_pending_sync_and_try_reacquire` returns no ownership.
2. Equal-generation routine and companion/heavy flags OR together and retain the owning pending bit.
3. A newer generation replaces the previous generation and mask.

Coverage includes retained `set_pending_sync`, `publish_pending_sync`, and `publish_pending_sync_and_try_reacquire` through stable public signatures. No release behavior changes in RED.

### 4. GREEN retained-wrapper floor (`109.010-T`)

In `state.rs` only:

- make `PendingSyncState::arm` and `PendingSyncState::publish` apply one cap-safe rule: older ignored, equal-generation full-mask OR, newer replaces;
- make `AppState::publish_pending_sync_and_try_reacquire` observe the publication result before CAS reacquisition and reject stale work without acquisition; and
- keep retained public wrapper signatures and response contracts stable.

Expected touches are those three functions, with at most one supporting function and no more than four total. Same-generation companion/heavy flags always retain an owning pending bit. No retry, fallback, second queue, or public contract change.

### 5. RED qualified start-or-publish (`109.005-T`)

In `state.rs` and `write.rs`, add exactly three deterministic scenarios:

1. A coherent qualified workspace/config/token snapshot reaches a free slot and reports `Acquired` while qualified write behavior remains frozen.
2. Generation advances after token capture but before initial CAS. The result is `Stale`: no initial acquisition, publication, queue mutation, or reacquisition.
3. Same-generation holder controls report `Queued` while live and `Reacquired` after release. Reacquisition consumes the routine-only startup claim once. Surviving revalidate/backfill bits retain an owning pending bit for bounded drain.

State exposes explicit outcomes and does not collapse them to bool.

### 6. GREEN qualified start-or-publish (`109.006-T`)

In `state.rs` and `write.rs`, preserve unchanged public `DispatchSnapshot` shape and add:

1. crate-private qualified snapshot construction;
2. synchronous opaque current-generation-token capture;
3. one token-qualified start-or-publish operation under the queue-floor mutex; and
4. direct `write::sync_workspace` adoption.

The arbitration checks floor before initial CAS and returns `Acquired`, `Reacquired`, `Queued`, or `Stale`. Older is ignored, equal masks OR, and newer replaces. `Reacquired` consumes only the routine-only startup claim so unchanged caller completion does not run a duplicate routine body. Companion/heavy work remains owned by a pending bit. These are the complete four production-function slots.

### 7. RED claim-only safety (`109.007-T`)

Exactly three state/lifecycle scenarios cover stale lost-lock exact re-arm, stale acquired-lock rejection with surviving newer work, and same-generation full-mask exactly-once bounded drain. Stale acquired rejection performs no path, sync, revalidation, backfill, or DB work; it releases indexing and returns to the existing bounded driver. Hydration cancellation and DB failure are excluded.

### 8. GREEN claim-only safety (`109.008-T`)

Exactly/at most three touched production functions:

1. atomic full-mask claim;
2. combined acquired validation and exact lost-lock re-arm; and
3. `lifecycle::drain_pending_sync`.

Stale acquired rejection always calls `finish_indexing` and continues bounded driving. Same-generation companion/heavy flags retain owning pending state. `background_db_hydration`, cancellation completion, and DB-connect failure are untouched here.

### 9. RED hydration ownership (`109.011-T`)

In `state.rs` and `lifecycle.rs`, add exactly three deterministic barrier/injection scenarios:

1. Hold indexing, start successor hydration, and prove zero DB connect/query/file work while non-acquired. Release/notify, then prove DB work begins only after ownership.
2. Cancel an acquired old-generation owner while newer-generation work survives. Clear exact old state, release/notify, and bounded-drain only surviving newer work.
3. Force DB-connect failure for an acquired owner while newer-generation work survives, with the same exact-old clear and release/notify/drain discipline.

The scenarios cover acquired and non-acquired ownership without sleeps, permissions, or a real daemon.

### 10. GREEN hydration ownership (`109.012-T`)

In `state.rs` and `lifecycle.rs`, exactly/at most four touched production functions:

1. `AppState::with_options` initializes one private `Notify` field;
2. `finish_indexing` releases indexing and notifies waiters;
3. one private cancellation-aware wait/acquire method registers `Notified` before every CAS attempt to prevent lost wake; and
4. `background_db_hydration` waits for ownership before any DB work.

On cancellation, forced DB-connect failure, or normal completion, clear only exact old state where applicable, call finish/notify, then use the existing bounded drain for surviving newer work. No pending mutex crosses await. No second work queue, public contract, recursion, timing test, or fifth function.

### 11. RED private startup outcomes (`109.003-T`)

In `ipc_server.rs` only, exactly three deterministic private scenarios:

1. generation advance before initial CAS -> `Stale`, no body;
2. holder final-peek release -> producer `Reacquired`, one body; and
3. stable/queued controls -> free slot `Acquired`, live holder `Queued`.

Use the exact synchronously captured pre-CAS token from `109.006-T`. Public callers remain untouched.

### 12. GREEN startup bool mapping (`109.004-T`)

Keep `try_start_startup_sync` synchronous. Map `Acquired` and `Reacquired` to `true`; map `Queued` and `Stale` to `false`. The `Reacquired` routine-only claim is already consumed, so unchanged caller completion cannot duplicate the startup routine and drains only surviving companion/newer work. Touch only `ipc_server.rs` and no more than three private production functions. Public callers and contracts remain unchanged.

## Guard and Completion Contract

- Transition lock order is `workspace -> config -> cancel`; pending mutex acquisition occurs only when no further await remains.
- The token-qualified start-or-publish decision linearizes floor validation, initial acquisition eligibility, publication, and reacquisition under one state arbitration slot.
- No `PendingSyncState` guard crosses await or DB/file work.
- Hydration registers `Notified` before CAS, then waits cancellation-aware without holding pending state.
- Every indexing owner exit releases and notifies. Stale acquired claim rejection releases and continues the bounded driver.
- Cancellation and DB failure clear exact old-generation state before release/drain. Torn old work never executes.
- No second queue, recursion, unbounded drain, timing test, or public bypass is permitted.

## Compatibility Contract

- Public `DispatchSnapshot`, retained pending-sync wrapper signatures, startup callers, queued status/message, CLI/MCP responses, errors, schema, and persistence shapes remain stable.
- Internal write and startup paths use the token-qualified arbitration outcomes.
- Final production search must find no internal path that bypasses floor-safe publication.
- Wrapper floor safety is closed P1 in `109.009/109.010` and cannot be deferred or reclassified.
- Only `tools/mod.rs` shared-seam rustdoc clarification may remain explicit non-blocking P2. It does not authorize a third file, fifth function, or contract widening.

## Dependency Graph

```text
109.001-T -> 109.002-T -> 109.009-T -> 109.010-T
    -> 109.005-T -> 109.006-T -> 109.007-T -> 109.008-T
    -> 109.011-T -> 109.012-T -> 109.003-T -> 109.004-T
```

This order is authoritative in dependencies and `104-S.custom_fields.items` after `109-F`.

## Verification and Blocked-Return Gates

Ship records compiling RED evidence before each immediate GREEN. Stage performs artifact validation only.

1. Each RED is fully GREEN in its immediate successor.
2. Every task is `<=2` production files, `<=3` deterministic scenarios, and `<=110 minutes`.
3. State GREEN tasks are `<=4` functions; `109.008-T` is `<=3`; startup is one file and `<=3` private functions.
4. Older/equal/newer wrapper behavior and no stale reacquisition are proven through retained public contracts.
5. Token-qualified arbitration exposes all four outcomes, forbids stale initial acquisition, consumes routine-only reacquisition exactly once, and preserves companion ownership.
6. Claim-only work has no hydration completion edit. Hydration performs zero DB work before ownership and uses register-before-CAS notification.
7. Public signatures, responses, startup callers, schemas, and persistence remain unchanged.
8. Shipment predecessor proof exists and validates fail closed.

Return the affected task and `104-S` blocked on any failed P0/P1 gate. Do not infer completion from missing, blocked, unknown, or unqueryable predecessor state.

## Risks, Rollback, and Monitoring

- Wrapper stale publication: blocked by cap-safe arm/publish and stale no-reacquire proof.
- Stale initial startup acquisition: blocked by floor check before CAS.
- Duplicate routine body: blocked by routine-only claim consumption on `Reacquired`.
- Orphan heavy flags: blocked by owning pending-bit preservation.
- Stranded claim work: blocked by unconditional stale release and bounded continuation.
- Pre-ownership DB work or lost wake: blocked by cancellation-aware Notify registration before CAS.

Rollback is a release-unit commit revert plus daemon restart. Do not partially revert one dependent unit. Monitoring uses deterministic targeted fixtures and existing bounded-drain warnings; no workspace migration or repair is planned.

## Plan Hardening

This is elevated concurrency work. Protected invariants are wrapper floor safety, exact opaque generation ownership, no stale acquisition, explicit arbitration outcomes, one routine body, full-mask ownership, lost-wake-safe hydration acquisition, zero DB work before ownership, exact-old cleanup, release/notify on every owner exit, bounded continuation, buildable intermediate states, and frozen public contracts.

**ProposedAction PA-1**: atomic transition and cap-safe retained wrappers in `state.rs` and `lifecycle.rs`. ActionRisk: moderate. ActionResult: approved for Ship execution, not executed by Stage.

**ProposedAction PA-2**: token-qualified write/start arbitration and claim-only drain in `state.rs`, `write.rs`, and `lifecycle.rs`. ActionRisk: moderate. ActionResult: approved for Ship execution, not executed by Stage.

**ProposedAction PA-3**: ownership-gated hydration and private startup bool mapping in `state.rs`, `lifecycle.rs`, and `ipc_server.rs`. ActionRisk: moderate. ActionResult: approved for Ship execution, not executed by Stage.

## Harvest Shape

`104-S` and `109-F` remain queued. Manifest order:

1. `109-F`
2. `109.001-T`
3. `109.002-T`
4. `109.009-T`
5. `109.010-T`
6. `109.005-T`
7. `109.006-T`
8. `109.007-T`
9. `109.008-T`
10. `109.011-T`
11. `109.012-T`
12. `109.003-T`
13. `109.004-T`

## Plan Review - Authorized Review-Fix Cycle: PASS

**Review mode:** current content and queued state, configured Stage model, no override.

**Gate:** PASS WITH ONE NON-BLOCKING P2. Open P0: none. Open P1: none. Open P2: `tools/mod.rs` shared-seam rustdoc clarification only. Open P3: none.

- Constitution: PASS. Every task meets file, scenario, time, and function caps.
- Rust/Concurrency: PASS. Wrapper floor P1 is substantive, arbitration is explicit, stale initial/reacquisition is forbidden, claims always release, and hydration is ownership-gated with lost-wake-safe notification.
- Scope: PASS. Claim and hydration concerns are isolated; startup is private and one-file; public contracts stay frozen.
- Architecture: PASS. Every producer primitive precedes its consumer in the authoritative dependency order.
- Agent parity: PASS. CLI/MCP responses and public startup callers remain unchanged.
- Batch proof: PASS. Exact predecessor lists are structured and fail closed.

**Decision:** retain all items queued for Ship. No P1 remains. The rustdoc-only P2 cannot widen implementation scope.


## Supersession and quarantine - 2026-08-01

This historical implementation plan is preserved but no longer executable. Operator-provided current-head adversarial review failed three cycles on the same architecture P1s:

1. retained public wrapper and generation qualification contract;
2. hydration ownership before DB work;
3. one authoritative arbitration and full-mask owner;
4. compiling deterministic RED proof; and
5. fail-closed predecessor and backlog-index checks.

The prior content-only PASS and all task provenance above remain as history, but accepted review `109.001-R` is superseded and rejected. Shipment `104-S`, feature `109-F`, and tasks `109.001-T` through `109.012-T` are blocked.

The redesign decision is `docs/decisions/2026-08-01-post-105-sync-coordinator-redesign-decision.md`. Only bounded proof task `109.013-T` is queued, outside `104-S`, under `docs/exec-plans/2026-08-01-post-105-sync-coordinator-spike-plan.md`.

Do not re-queue implementation from this plan. Restart requires spike findings, a revised single-authority implementation plan, plan hardening, a fresh zero-P0/P1 PASS, width-safe harvest, successful backlog index sync, and exact positive terminal shipped or merged evidence for predecessors `102-S` and `103-S`.
