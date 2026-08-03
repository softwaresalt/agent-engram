---
title: "Post-105 single-authority sync coordinator permit migration"
type: impl-plan
doc_type: plan
date: 2026-08-02
status: accepted
source: "docs/decisions/2026-08-01-post-105-sync-coordinator-spike-findings.md"
feature: "109-F"
shipment: "104-S"
tags: ["concurrency", "pending-sync", "permit", "internal-api", "raii", "cancellation-safety", "ready-after-106-closure"]
---

## Pipeline gate

The historical replacement gate was **`ready_after_106_closure`**, represented by the supported plan body marker and backlog label `ready-after-106-closure`. That gate is now satisfied: `106-S` and `109.013-T` are archived, Stage completed its exact fail-closed requeue transaction, and `104-S`, `109-F`, and replacement tasks `109.014-T` through `109.031-T` are queued. Ship may implement only after revalidating the archived prerequisites, exact shipment manifest/predecessors, branch gate, and claim lifecycle.

This plan supersedes `docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md` and tasks `109.001-T` through `109.012-T`. None of those task scopes remains accurate: they retain tokenless completion, `Reacquired`, split mask takes, or bounded double-drain behavior rejected by the spike.

## Problem frame

Engram currently uses separate synchronization authorities for generation, an `AtomicBool` owner, a mutex-protected pending mask, and lifecycle-local hydration/drain behavior. Four deterministic REDs proved that stale work mutates newer masks, completion releases without transferring pending work, hydration reaches DB admission without ownership, and startup/release can select zero executors.

The replacement is strategy A from the findings: one private `SyncCoordinator` owns generation floor, sequenced owner identity, complete pending `WorkMask`, and hydration/drain handoff. All critical production callers are internal to the non-published crate. Tokenless ownership mutators are removed or reduced after caller/test migration. Public CLI/MCP, wire, schema, persistence, queued-response, and startup behavior stay compatible.

## Source and compatibility decision

- Source basis: exact final review-fix HEAD `2f267d9c617243dd70cbaac9837826a4fd0358e9`; the accepted cancellation/quiescence design is unchanged, and exact Copilot P1 `discussion_r3701238147` identifies the remaining successful-release wake gap for empty Hydration/Startup/Watcher waiters.
- Evidence basis: `docs/research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md`, reviewed evidence commit `b5d5802e`.
- Package contract: `Cargo.toml` has `publish = false`; Engram ships a binary. No public Rust permit API is added.
- API decision: opaque `GenerationToken`, `AdmissionGuard`, `OwnerPermit`, `DriverTaskGuard`, `OwnerKind`, and `WorkMask` are `pub(crate)` at most, with private fields. Admission and permits are non-Clone; cancellation ownership moves between them, and permit/task guards provide mandatory non-awaiting abandonment cleanup.
- Semver: no major bump or deprecation bridge. This is an internal source migration. A supported downstream Rust API discovered before source mutation is a hard stop requiring strategy B and a new decision; tokenless adapters are never accepted as a fallback.
- Repository tests: migrate direct owner setup to public tool behavior or co-located private tests. No public test-only seam.

## Requirements trace

| Requirement | Implementation action | Verification units |
|---|---|---|
| One authority | Replace owner flag, generation, mask, timestamp, and handoff with one `Arc<CoordinatorCell>` | `109.014-T` -> `109.015-T` |
| Drop/cancellation safety | Every permit carries the current generation cancellation receiver; armed Drop either recovers a running owner or acknowledges a retirement barrier once | `109.014-T` / `109.015-T` |
| Successful finish disarms cleanup | Exact completion disarms old permit before Drop and writes timestamp once | `109.014-T` / `109.015-T` |
| Empty-waiter release liveness | An exact no-pending `Running` completion selects `Released`, clears owner, unlocks, and invokes `notify_one` once; registered empty waiters recheck and pass a one-owner baton | `109.014-T` / `109.015-T`; caller rows in `109.018-T`-`109.025-T` and `109.031-T` |
| Binding-aware generation retirement | One lock transition advances binding/floor but retains the retired driver behind a quiescence barrier; same-binding work lives in the barrier while distinct-binding carries zero old bits | `109.016-T` -> `109.017-T` |
| Quiescence and stale isolation | No successor acquires before the exact retired permit acknowledges exit; later finish/Drop is stale and cannot mutate or wake | `109.016-T` / `109.017-T`; driver matrices in `109.018-T`-`109.025-T` and `109.031-T` |
| Hydration owns before I/O | Pre-acquisition cancel is zero-permit; post-acquisition cancel relies on RAII | `109.018-T` -> `109.019-T` |
| Full-mask single successor | Completion or abandonment exposes one whole mask to one successor; an empty acquired owner releases a waiter baton without creating work | `109.020-T` -> `109.021-T` |
| Write migration | Index/sync use guarded permits; exact queued JSON; no producer reacquire | `109.022-T` -> `109.023-T` |
| Startup/watcher arbitration | Typed guarded permits and one request linearization | `109.024-T` -> `109.025-T` |
| Compatibility tests | Replace tokenless setup with behavior/private harnesses | `109.026-T`, `109.027-T` |
| Visibility reduction | Retire tokenless owner, split pending, and companion mutators | `109.028-T` -> `109.030-T` |
| Process-abort boundary | No Drop claim on abort; startup reconciliation, intent reissue, full rollback | `109.031-T` |
| Runtime/release closure | Deterministic suite plus disposable Windows daemon validation | `109.031-T` |

## Authoritative design

### State, cancellation, and RAII ownership

```text
CoordinatorCell { state: std::sync::Mutex<SyncCoordinator>, notify: Arc<Notify> }
BindingIdentity { workspace_uuid, workspace_id }  // private exact equality; workspace_id includes path/branch
SyncCoordinator {
  floor, binding_identity: BindingIdentity, next_sequence,
  phase: Idle | Running(OwnerRecord) | Retiring(RetirementBarrier),
  pending: Option<WorkMask>, generation_cancel, last_indexed_at
}
GenerationToken { floor, binding_identity }
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerIdentity { generation, sequence, kind }
OwnerRecord { identity, binding_identity, work_mask }
RetirementBarrier {
  retired_identity, retired_binding, target_generation, target_binding,
  deferred: WorkMask
}
AdmissionGuard {
  cell: Arc<CoordinatorCell>, token, binding_snapshot,
  cancel_rx: watch::Receiver<bool>,
  enabled_notification: Pin<Box<OwnedNotified>>
}
OwnerPermit {
  cell: Arc<CoordinatorCell>, token, binding_snapshot,
  cancel_rx: watch::Receiver<bool>, identity, work_mask, cleanup_armed
}
DriverTaskGuard { join_handle, abort_handle, terminal_state }
```

`AppState` owns one private `Arc<CoordinatorCell>`. The coordinator is the only receiver-clone minting point. Each non-Clone `AdmissionGuard` owns one receiver, exact snapshot, cell, and pinned enabled `OwnedNotified`; `Arc<Notify>` avoids a self-referential guard. Acquisition consumes the guard, drops the enabled registration, and moves only the receiver/snapshot/cell/token ownership into a non-Clone `OwnerPermit`. Callers cannot extract or clone the receiver. `Notify` stores neither work nor identity, is never ownership authority, and only triggers a mutex-protected guarded recheck.

The coordinator phase is the sole authority. `pending` is used only in `Idle`/`Running`. During `Retiring`, `RetirementBarrier.deferred` is the sole current-generation work location; the retired permit retains an immutable old binding snapshot and a non-authoritative mask copy only so it can stop safely. No current permit exists and no successor can acquire until the barrier is acknowledged.

Every driver owns its permit around the complete DB/file-capable future. Rebind signals its receiver. The driver either observes cancellation and explicitly acknowledges only after its DB/file-capable future has been dropped or joined, or task cancellation/panic unwinds that future before armed `Drop` acknowledges. A detached task with DB, file, coordinator, or workspace-mutation authority is forbidden; discovering one is a stop-and-replan condition. A CPU-only parse worker may finish after its parent future is dropped only when it holds no such authority.

### Request transition

`request(admission, mask, kind)` consumes the non-cloneable guard and returns fallible `Acquired(permit) | Waiting(admission) | Enqueued | Stale` after validating under the coordinator lock.

- In `Running`, a current non-empty producer ORs one complete mask into ordinary `pending` and receives `Enqueued`; a current empty-mask Hydration/Startup/Watcher waiter publishes no work and receives `Waiting` with the same admission guard. Stale requests mutate nothing.
- In `Retiring`, no request can acquire. A current-token non-empty request ORs into `RetirementBarrier.deferred` and receives `Enqueued`; an empty-mask waiter publishes no work and receives guarded `Waiting`. Thus same-binding `0b111` and all later current-generation requests remain in one authoritative barrier slot. Old-token requests remain stale.
- A rebind that arrives while already retiring retargets the same barrier, never creates another retired owner: equal target binding preserves and retags `deferred`; a distinct target binding discards the previous target-binding work before accepting later requests for the newest token.
- In `Idle`, a direct non-empty request atomically takes `pending OR requested` while preserving the requested `Index` or `Sync` kind; only completion-transferred/coalesced work is normalized to `Sync`. Concurrent requesters cannot take it twice. A current empty-mask Hydration/Startup/Watcher request may acquire one empty permit while ordinary pending remains authoritative for later transfer.
- Each acquisition consumes `AdmissionGuard`, removes its enabled notification registration, and moves the receiver/snapshot/cell/token into the permit. The acquired permit cannot retain or consume a later release wake. Only guard minting clones the current channel receiver. Sequence exhaustion fails before mutation. No path installs an owner without returning its armed permit.

Every empty-mask wait loop owns an `AdmissionGuard` whose `OwnedNotified` registration is enabled before the final `request`/recheck. `Acquired` cancels that registration before returning the permit and moves the remaining cancellation/binding ownership; `Waiting` selects the owned registration against the owned receiver; `Stale` or generation cancellation drops the guard and exits. After a wake, the guard is re-armed before rechecking. This closes the release-versus-registration gap without letting an acquired owner steal the release wake and without a second queue.

Public behavior remains mapped at the caller boundary: busy Index retains `IndexInProgress`; internal `Enqueued` maps to the exact busy Sync queued JSON only after coordinator ownership of the full mask; and guarded `Waiting` is internal to Hydration/Startup/Watcher rechecks. Neither a waiting notification nor an enqueued response authorizes execution.

### Explicit completion and retirement acknowledgment

`complete(mut permit)` consumes the permit and compares generation, sequence, and kind under the mutex.

- An exact `Running` match with pending work installs one armed `Sync` successor returned as `Transferred`. With no pending work it clears the owner and selects `Released`. Both paths write `last_indexed_at` once and disarm the old permit. Because an empty waiter may exist whenever this path was `Running`, the `Released` path conservatively drops the mutex, invokes `notify_one` exactly once, then returns the selected outcome so a registered waiter can compete; no separate waiter queue or exact resumed-task claim is added.
- An exact `Retiring` match is not ordinary success and not stale: it is `RetirementAcknowledged`. Only after the driver has quiesced does this terminal move `deferred` to the latest target generation ordinary pending slot, clear the barrier, disarm the old permit, drop the mutex, and invoke `notify_one` exactly once. It writes no completion timestamp and never returns a successor permit to the retired driver.
- Any later or identity-mismatched terminal disarms only its local guard and returns `Stale`. It cannot mutate phase, pending/deferred mask, floor, binding, notification count, or timestamp.

The coordinator lock linearizes completion versus rebind. Completion first is a regular current terminal before publication. Rebind first converts that same terminal into the unique retirement acknowledgment. No standard mutex crosses `.await`.

### Mandatory Drop transition

`OwnerPermit::drop` is the mandatory terminal guard. If armed, it locks synchronously with poison recovery.

- An exact `Running` match retains the accepted abandonment rule: compute authoritative `owner.work_mask OR pending`, clear the owner, publish any non-empty union once, unlock, and invoke `notify_one` exactly once. The call also passes the baton when an empty owner is abandoned.
- An exact `Retiring` match acknowledges quiescence. It does not OR the old permit mask again because that mask already lives in `RetirementBarrier.deferred`. It publishes deferred to ordinary pending, clears the barrier, and performs the same single post-unlock `notify_one` as explicit acknowledgment, without a timestamp.
- Any identity mismatch is a strict no-op. Drop never allocates a sequence, awaits, spawns, or panics.

The exact count above is the number of `notify_one` invocations made after the linearized owner-clearing transition, not a claim that exactly one task resumes. Tokio `Notify` may wake at most one registered waiter or retain one permit when none is registered; coalescing and waiter cancellation make resumed-task counting unsuitable as authority. Progress comes from pre-registered recheck loops. With multiple empty waiters, one wake permits at most one mutex-protected acquisition; remaining waiters stay registered. The acquired empty owner completes `Released` and emits the next post-unlock notification, passing the baton one owner at a time. If another producer wins, the empty waiter remains queued and blocks on its fresh registration. There is no polling, work-mask duplication, or concurrent driver.

The DB/file-capable child future must be gone before the permit may Drop. If a driver hangs or ignores cancellation, the barrier deliberately stays closed and no timeout force-clears it; monitoring triggers full-unit rollback/restart instead of overlapping drivers.

### One atomic generation/binding advance and quiescence barrier

Prepare the complete workspace/config/new-cancellation tuple, derive exact `BindingIdentity`, and validate capacity before mutation. Acquire binding write guards in documented order, then the coordinator mutex; do not await afterward. One publication advances the visible binding and floor but never makes a successor runnable while an old driver remains:

1. replace the current generation cancellation sender and retain the old sender for synchronous signaling after publication;
2. if phase is `Running`, remove ordinary pending and convert the exact owner to `Retiring`:
   - **same binding:** initialize `deferred = owner.work_mask OR pending` under the new generation, including `0b101 OR 0b010 = 0b111`;
   - **distinct binding:** initialize `deferred = 0` and transfer no old routine/revalidate/backfill bit;
3. if phase is already `Retiring`, retain the same retired identity: an equal new target binding preserves/retags deferred, while a distinct new target discards deferred from the superseded target; never stack barriers or allocate a successor;
4. if phase is `Idle`, same-binding pending is retagged under the new generation and distinct-binding pending is discarded;
5. swap workspace/config, advance binding identity and floor together, reset hydration readiness, and install the new generation cancellation channel.

After all guards release, signal the old generation cancellation sender. This reaches a receiver held by every active `OwnerKind` and by pre-acquisition waiters. **Do not notify a successor at rebind.** In the active-owner case, `RetirementBarrier.deferred` is the sole authoritative current-generation work location while the retired driver unwinds against its immutable old snapshot. Current-token requests coalesce into that field and cannot acquire.

Only exact explicit terminal or armed Drop of the retired permit acknowledges exit. The acknowledgment atomically publishes the latest deferred mask to ordinary pending, clears the barrier, and invokes `notify_one` exactly once after unlock. At most one waiter resumes; coordinator state, not the notification, decides whether it acquires. The successor then competes through normal `request` and cannot overlap the old driver. Same-binding replay may reconcile work already partly attempted, but there is never duplicate authoritative ownership or concurrent same-database execution. Distinct-binding acknowledgment exposes only requests issued for the latest new binding; durable state still uses new-binding startup/hydration/offline-change reconciliation and non-durable intent still requires a new-token request.

Every Index, Sync, Hydration, Startup, and both legacy/v2 Watcher driver must hold its permit and immutable binding snapshot for its whole DB/file-capable future, observe cancellation, and reach one terminal. Cancellation may drop a cancellation-safe operation future; any non-cancellation-safe or detached DB/file mutator must be joined before acknowledgment. Old normal completion after retirement is the acknowledgment and writes no timestamp. Any terminal after acknowledgment is stale and harmless.

### Hydration, driver cancellation, and process-abort boundary

Hydration, Startup, and both watcher wait paths create and enable `Notified` before each final request/recheck and select notification versus their generation cancellation receiver without a standard mutex. Cancellation before acquisition exits with no permit or acknowledgment. After acquisition, Hydration follows the same barrier rule as every other owner: no DB/file boundary before `Acquired`, and on rebind it drops or joins DB/file-capable work before explicit acknowledgment or armed Drop.

Index/Sync wrap the complete write driver in the permit lifetime. Startup and both watcher loops do the same for each acquired execution. They check cancellation before starting another phase and select it against cancellable waits. A cancellation observation suppresses flush, drain, or any later phase; acknowledgment occurs only when the current DB/file-capable phase has actually ended. Deterministic activity counters, not sleeps, prove `max_active_db_drivers == 1` and zero old-driver work after acknowledgment for every `OwnerKind`.

Rust Drop is not claimed for process abort. Restart reconstructs in-memory authority; bind/hydration and offline-change detection reconcile durable files. Non-durable revalidate/backfill intent must be reissued. Runtime invariant failure or a stuck retirement barrier uses full release-unit revert and daemon restart.

### Compatibility boundary

Safe observers may remain stable. Request-only publication may only be crate-private delegation to `request`; it cannot stage companion-only state. Tokenless claim/completion/generation-clear/producer-reacquire/split-take/companion setters retire. The exact queued result remains:

```json
{"status":"queued","message":"Sync queued; will run after current indexing completes"}
```

## Protected invariants

1. The coordinator cell is sole authority for binding floor, exact binding identity, owner phase, full mask, handoff, cancellation generation, and completion timestamp.
2. In `Idle`/`Running`, the owner mask is the in-flight attempt and ordinary pending is the sole queued-rerun slot; a repeated request may therefore set the same bit in both, and terminal/rebind transitions consume their union exactly once. In `Retiring`, all current-generation work exists only in `RetirementBarrier.deferred`.
3. Every `OwnerKind` receives and observes generation cancellation. No successor permit can be acquired or run until the exact retired permit acknowledges that all DB/file-capable work has exited.
4. Same-binding active-owner advance atomically moves `owner mask OR pending` into the barrier under the new generation; `0b101 OR 0b010 = 0b111` is mandatory and new requests coalesce there.
5. Distinct-binding advance moves zero old routine/revalidate/backfill bits; only latest-binding requests may accumulate behind the barrier, durable state uses new-binding reconciliation, and non-durable intent requires new-token reissue.
6. A second rebind during retirement retargets one barrier: same target binding preserves deferred work; distinct target binding discards superseded-target work; no second retired owner or successor appears.
7. Exact retirement acknowledgment, whether explicit terminal or Drop, publishes deferred/clears the barrier once and invokes `notify_one` exactly once after unlock. It never timestamps or returns a successor to the retired driver.
8. Exact running Drop republishes authoritative owner mask OR pending, clears owner once, and invokes `notify_one` exactly once after unlock. Exact no-pending running completion clears owner, disarms/timestamps once, selects `Released`, unlocks, and also invokes `notify_one` exactly once.
9. Every empty Hydration/Startup/Watcher waiter enables notification before its final request/recheck. One released waiter may acquire; multiple waiters progress through empty-owner `Released` baton passes. Each notification resumes at most one waiter and never authorizes execution.
10. Stale request, finish, acknowledgment, and Drop mutate and notify zero times.
11. Hydration does zero DB/file I/O before acquisition. Index/Sync/Hydration/Startup/Watcher retain immutable binding snapshots and permits for their complete DB/file-capable futures.
12. No detached DB/file/workspace mutator can survive acknowledgment; failure to prove quiescence blocks the release unit. A stuck driver keeps the barrier closed.
13. No mutex crosses await; Drop never awaits, spawns, or panics.
14. No second queue, double drain, split consumption, producer reacquire, sleeps, unsafe, or public test seam.
15. No CLI/MCP/wire/schema/persistence/config/queued-response regression.
16. No exactly-once or RAII claim crosses process abort; restart reconciliation, qualified reissue, and full rollback remain explicit.

## Implementation units

Every unit is `<=110 minutes`, touches `<=2` production files, changes fewer than five production functions, and has `<=3` deterministic scenarios. RED tasks change zero release behavior. GREEN starts only after its direct RED compiles and fails intended assertions. No partial migration is mergeable/releasable.

### 1. `109.014-T` — RED: permit cancellation, completion, and Drop lifecycle

- Files: `src/server/state.rs` only.
- Scenarios (3 maximum): an `OwnerKind` cancellation fixture; an exact terminal matrix covering completion/Drop, pending/empty, timestamp/disarm, and stale no-op; and a parameterized owner-success empty-waiter matrix covering one and multiple Hydration/Startup/Watcher waiters plus baton progress and acquired-registration removal.
- The release rows require notification enabled before final request, no-pending completion to select `Released`, owner clear before one post-unlock `notify_one`, at most one acquisition, and one empty-owner release per remaining waiter. Use private deterministic cancellation/notification counters; no driver source or release behavior changes.
- Dependency: `109.013-T` terminal, while status remains blocked until the existing Stage requeue gate.

### 2. `109.015-T` — GREEN: cancellation-bearing RAII coordinator core

- Files: `src/server/state.rs` only.
- Exactly four production-function touches: existing AppState construction initializes the cell; request clones the receiver and supports empty queued/acquired outcomes; complete handles exact running terminals plus post-unlock `Released` notification; OwnerPermit Drop handles running abandonment/baton.
- Implement `Idle/Running` authority, a generation cancellation receiver in every permit, authoritative owner mask, completion disarm, synchronous poison-safe running Drop, and timestamp-in-transition. The `Retiring` branch is added only by `109.017-T` after its RED.
- No caller-optional cleanup, async Drop work, second queue, tokenless bridge, or owner without a permit.
- Dependency: `109.014-T`.

### 3. `109.016-T` — RED: active rebind quiescence and stale isolation

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3 maximum): a complete old/new binding-token-cancel-floor tuple including `u64::MAX` no mutation; one parameterized active-rebind matrix over same/distinct binding, every `OwnerKind`, and explicit/Drop acknowledgment; one repeated-rebind/stale-terminal matrix.
- The active matrix seeds owner `0b101` plus pending `0b010`. Same binding must expose `0b111` only in `RetirementBarrier.deferred`; distinct binding must expose zero old bits. Current-token requests coalesce in deferred, cancellation reaches the retired permit, and attempted successor acquisition remains blocked until exit acknowledgment.
- Ack must move the latest deferred mask to ordinary pending, clear the barrier, and invoke `notify_one` exactly once after unlock; deterministic activity counters remain at one and old work count cannot increase after ack. Repeated same-target rebind preserves deferred; distinct retarget discards superseded-target work. Later finish/Drop is stale and changes nothing.
- Fixture rows do not increase the scenario count. No live DB/file work or release behavior.
- Dependency: `109.015-T`.

### 4. `109.017-T` — GREEN: atomic binding advance and quiescence barrier

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four production functions cover prepared identity/install, one coordinator publication, retirement acknowledgment, and lifecycle wait/reissue entry.
- A running-owner rebind advances tuple/floor, installs one `RetirementBarrier`, moves same-binding union or zero distinct-binding old bits into `deferred`, installs the new generation cancellation channel, and signals the old channel only after unlock. It never clears the barrier, notifies a successor, or permits acquisition before exact ack.
- Current requests coalesce behind the barrier. Exact explicit terminal or Drop publishes/wakes once; stale terminals do nothing. A rebind while retiring retargets the same barrier by target-binding equality.
- Acquire async binding guards before coordinator; no standard mutex across await. Preserve dispatch snapshot, wire, schema, config, persistence, and new-binding reconciliation/reissue rules.
- Dependency: `109.016-T`.

### 5. `109.018-T` — RED: hydration admission, cancellation, and quiescent ack

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): held owner prevents pre-DB signal; a phase matrix covers pre-acquisition cancellation, owner-success/no-pending release to one or multiple pre-registered empty Hydration waiters, and same/distinct active rebind after acquisition; acquired DB-connect failure explicitly completes/disarms.
- Release rows prove no missed wake, one mutex-authorized empty acquisition at a time, and baton progress without polling. Active rows prove cancellation is observed, no new permit starts before Hydration drops/joins DB/file-capable work and acknowledges, acknowledgment invokes one post-unlock notification, and no old work occurs after ack. Pre-acquisition cancellation has no permit or ack.
- Bind the harness to a private production collaborator; no live DB timing.
- Dependency: `109.017-T`.

### 6. `109.019-T` — GREEN: cancellation-aware hydration guard

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four functions cover notify/retry admission, permit cancellation observation, hydration body, and exact-exit finalization.
- `AdmissionGuard` owns and enables notification before every final request/recheck. On guarded `Waiting`, select that registration against its receiver; after wake, re-arm before rechecking. Pre-acquisition generation cancellation exits without ack. After acquisition, select cancellation at safe boundaries; drop or join all DB/file-capable work before explicit retirement ack or armed Drop. Normal/handled failure completes/disarms.
- No DB/file work before permit, detached mutator, forced barrier timeout, or caller-optional cleanup.
- Dependency: `109.018-T`.

### 7. `109.020-T` — RED: transferred-mask lifecycle driver

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): full `0b111` executes once under one successor; a release-side matrix covers one and multiple pre-registered empty Hydration/Startup/Watcher waiters, owner-success `Released`, one-at-a-time acquisition, and empty-owner baton passing; dropping/cancelling a transferred successor republishes full mask, clears owner, and lets one later requester take it without a second drain.
- No bounded loop/yield, timing assertion, or optional cleanup call.
- Dependency: `109.019-T`.

### 8. `109.021-T` — GREEN: single completion/RAII handoff driver

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four functions replace split takes, re-arm, `drain_pending_sync`, and bounded drain-to-completion with one transferred-mask driver.
- Explicit success consumes/disarms; no-pending success invokes the post-unlock release notification, and an acquired empty owner completes once to pass the baton. Cancellation/panic relies on Drop. The next current request is the only recovery successor. No recursive/double drain or busy loop.
- Dependency: `109.020-T`.

### 9. `109.022-T` — RED: Index/Sync ownership, active rebind, and queued contract

- Files: `src/server/state.rs`, `src/tools/write.rs`.
- Scenarios (3): an Index/Sync × same/distinct-binding active-rebind matrix; busy Sync exact queued JSON/full mask/no reacquire; stale qualified snapshot no execution.
- The matrix proves both owner kinds receive cancellation, retain immutable snapshots, drop/join DB/file-capable work before ack, never overlap a successor, never perform work after ack, and preserve same-binding barrier union versus distinct-binding zero carryover.
- Preserve scan progress and existing Index error behavior. Narrow targets compile before intended failures; no public seam.
- Dependency: `109.021-T`.

### 10. `109.023-T` — GREEN: write caller cancellation and permit migration

- Files: `src/server/state.rs`, `src/tools/write.rs`.
- At most four functions migrate Index/Sync admission, cancellation-aware driver lifetime, and finalization to exact permits.
- Each complete DB/file-capable future is inside the permit lifetime and selected against permit cancellation. On retirement, stop later phases, drop/join mutation-capable work, then acknowledge; ordinary success completes/disarms. No successor starts before ack.
- Delete producer reacquire and separate finish/drain. Preserve busy Index error, exact queued Sync JSON, progress, and MCP/CLI errors.
- Dependency: `109.022-T`.

### 11. `109.024-T` — RED: Startup/Watcher arbitration and active rebind

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- Scenarios (3 maximum): a Startup/Hydration/legacy-Watcher/v2-Watcher owner-success plus single/multiple empty-waiter release matrix; one parameterized Startup/legacy-Watcher/v2-Watcher × same/distinct active-rebind matrix; stale watcher terminal matrix.
- Release rows prove pre-registration, `Released` post-unlock notification, one-at-a-time empty acquisition, multi-waiter baton progress, and no spin. Active rows prove cancellation observation, immutable snapshots, no successor acquisition before DB/file-capable exit ack, `max_active_db_drivers == 1`, one ack notification, and zero old work after ack. Both watcher loops are covered.
- Stale finish/Drop cannot clear or execute a replacement. Barriers/oneshots/counters only.
- Dependency: `109.023-T`.

### 12. `109.025-T` — GREEN: Startup and both Watcher cancellation migration

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- At most four functions cover Startup, the legacy watcher, the v2 watcher, and shared terminal handling.
- Every acquired Startup/Watcher permit wraps its full DB/file-capable execution and observes cancellation. Empty wait loops enable notification before every final request/recheck; a no-pending successful owner release invokes `notify_one` after unlock so at most one competitor resumes, and each acquired empty owner passes the baton by completing once. Retirement suppresses later flush/drain phases, quiesces mutation-capable work, then acknowledges; ordinary success completes/disarms.
- Remove try-then-set and finish-then-drain. No detached mutator, forced barrier clear, overlap, or response/schema change.
- Dependency: `109.024-T`.

### 13. `109.026-T` — compatibility migration: contract tests

- Files: `tests/contract/read_test.rs`, `tests/contract/write_test.rs`; production files: zero.
- At most three scenarios: read rejection via public behavior; queued exact response plus complete-mask observable behavior; idle control.
- Remove tokenless setup; add no public fixture API.
- Dependency: `109.025-T`.

### 14. `109.027-T` — compatibility migration: resilience tests

- Files: `src/server/state.rs`, `tests/integration/indexing_resilience_test.rs`; one production file with test-only private assertions.
- At most three scenarios: a stale finish/Drop plus dropped-owner recovery lifecycle matrix; current completion disarm/timestamp once; release control.
- Dependency: `109.026-T`.

### 15. `109.028-T` — retire tokenless ownership mutators

- Files: `src/server/state.rs` only; at most four functions: retire/reduce `try_start_indexing`, `finish_indexing`, producer reacquire, and generation clear.
- Prove zero callers; no public/test-only permit export.
- Dependency: `109.027-T`.

### 16. `109.029-T` — retire split pending API

- Files: `src/server/state.rs` only; at most four functions: retire/reduce set/publish/take/observer where safe.
- Prove no split consumption or second-authority wrapper.
- Dependency: `109.028-T`.

### 17. `109.030-T` — retire companion-only API and final inventory

- Files: `src/server/state.rs` only; four companion set/take functions.
- Verify zero tokenless owner, generation-clear, split-take, companion-only, producer-reacquire, bounded/double-drain, and caller-optional cleanup paths across `src/` and `tests/`.
- Dependency: `109.029-T`.

### 18. `109.031-T` — deterministic quiescence, Windows runtime, and release validation

- Production files: zero; closure/checklist docs only if needed.
- At most three scenarios: normal bind/Hydration/complete-disarm plus owner-success/empty-waiter rows; queued Sync plus running Drop recovery/exact response and a multi-empty-waiter baton fixture; one deterministic `OwnerKind × same/distinct binding × explicit/Drop ack` active-rebind matrix with forced DB-failure and Startup/legacy-watcher/v2-watcher Windows named-pipe handoff rows.
- Release fixtures prove notification registration before recheck, removal on acquisition, exact one post-unlock `notify_one` call on `Released`, at-most-one resumed/acquired waiter per call, finite baton progress, no busy loop, no work duplication, and max one owner. The active matrix preserves cancellation delivery, same-binding `0b111` location, distinct-binding zero carryover, request coalescing during retirement, no acquisition before ack, one ack notification call, `max_active_db_drivers == 1`, zero old work after ack, and stale-terminal no-op. Fixture rows stay within the three-scenario cap.
- Smoke is observational; deterministic fixtures prove races. Verify restart reconciliation and qualified non-durable reissue; never claim Drop on process abort.
- Dependency: `109.030-T`.

## Dependency graph

```text
109.013-T (Ship closes spike)
  -> 109.014 -> 109.015
  -> 109.016 -> 109.017
  -> 109.018 -> 109.019
  -> 109.020 -> 109.021
  -> 109.022 -> 109.023
  -> 109.024 -> 109.025
  -> 109.026 -> 109.027
  -> 109.028 -> 109.029 -> 109.030
  -> 109.031
```

This chain is intentionally narrow. The four production modules migrate by responsibility: state authority, binding/hydration, lifecycle handoff, write, then IPC. The complete release unit is the only merge/deploy/rollback boundary. Partial migration is never shipped.

## Deterministic test strategy

- Private co-located harnesses access coordinator phase, cancellation delivery, Drop/ack, notification, and driver activity counters; no public seam.
- Every RED first passes narrow `cargo test --lib --no-run <target>`, then fails only its named assertion. Missing symbols/visibility are not RED.
- Use direct transitions, `Barrier`, oneshot, or `Notify`. No sleeps, yields, permission races, live-daemon race proof, or wall-clock assertions.
- One parameterized relation/kind/terminal matrix proves: every `OwnerKind` receives cancellation; same-binding `0b101 OR 0b010 == 0b111` lives only in the barrier; distinct binding starts with zero old bits; current requests coalesce behind the barrier; no successor acquires before explicit/Drop ack; ack publishes and invokes `notify_one` exactly once; `max_active_db_drivers == 1`; old work count cannot increase after ack; later terminals are no-ops.
- Owner-success/empty-waiter fixtures enable each `Notified` before the final request, cover one and multiple Hydration/Startup/Watcher waiters, and assert exact notification-call counts but only at-most-one waiter resumption/acquisition per call. Each acquired empty owner releases once to pass the baton; blocked waiters never poll and all state decisions come from the coordinator mutex.
- A repeated-rebind fixture proves same-target preservation and distinct-target discard while retaining one retired identity.
- Ordinary current Drop clears owner/republishes once; one successor takes recovery once; ordinary completion disarms and timestamps once. Hydration distinguishes zero-permit pre-acquisition cancellation from acquired retirement acknowledgment.
- Static inventory confirms no legacy symbols, caller-optional cleanup, detached DB/file mutator, or direct tokenless setup.
- Process abort is restart reconciliation/intent reissue, never a Drop path. Windows smoke checks liveness only.

## Migration and atomicity

Intermediate branch commits may checkpoint execution, but none is mergeable/releasable/runtime-valid. The final PR must contain coordinator cell, cancellation-bearing RAII permits, the retirement barrier, every driver migration, and visibility cleanup. Tokenless completion, split-mask cache, second authority, caller-optional cancellation cleanup, or a driver that cannot prove quiescence stops the unit.

Binding transition is one logical publication. It advances binding/config/floor and installs a new cancellation channel under the fixed lock order. With an active owner it moves same-binding union or zero distinct-binding old bits into one `RetirementBarrier.deferred` slot, leaves the retired identity as the barrier key, then signals the old channel after unlock. It performs no coordinator wake and exposes no successor permit. Current requests coalesce behind the barrier. Only exact explicit terminal or Drop after DB/file-capable exit publishes deferred to ordinary pending, clears the barrier, and invokes `notify_one` once after unlock. Ordinary exact no-pending completion uses the same post-unlock notification discipline for `Released` empty-waiter progress. A later rebind retargets the same barrier rather than bypassing it.

Each driver captures its immutable binding snapshot before DB/file execution and never rereads the current binding for old work. Cancellation-aware wrappers may drop cancellation-safe futures; non-cancellation-safe mutation work must be joined. No timeout opens a stuck barrier. This fail-closed wait is required to prevent same-database overlap and duplicate authoritative work.

No storage, schema, config-format, or wire migration exists. Process death is outside Rust Drop: restart hydration/offline reconciliation covers durable files, non-durable companion intent is reissued only for the applicable current binding, and invariant failure rolls back the complete unit then restarts. No exactly-once side-effect claim crosses process death or binding identity.

## Exact Stage requeue transaction after Ship closes 106

Stage performs this transaction only after a new session starts and index sync succeeds:

1. Call `backlogit_sync_index`; any warning/failure blocks requeue.
2. Read `106-S` and `109.013-T` exactly. Require terminal archived/shipped/done evidence and a merge commit containing the findings; active, blocked, missing, or unqueryable fails closed.
3. Re-read `102-S` and `103-S`. Require archived terminal records with commits `89ce54193ad8c1340e5b8b440f9190a276b72196` and `5c9d466ebff883ae8ae6e71008968f986707e882`. Do not infer predecessor completion.
4. Re-read `104-S`, `109-F`, old tasks `109.001-T`–`109.012-T`, and replacements `109.014-T`–`109.031-T`. Require old tasks blocked/superseded and replacements blocked with the exact chain above.
5. Remove old tasks `109.001-T`–`109.012-T` from `104-S` using the supported blocked-return manifest operation with reason `superseded by reviewed single-authority permit plan`; leave them blocked. Do not mutate 102/103.
6. Add replacement tasks `109.014-T`–`109.031-T` to `104-S` after the already-present parent feature `109-F`. Keep terminal spike provenance if the manifest projection retains it.
7. Move `109-F` and replacement tasks to `queued`, then move `104-S` to `queued` last. Do not claim it.
8. Sync the index again and re-read all statuses/dependencies. Any mismatch rolls the transaction back to blocked/queued-safe state and halts for operator review.

## Runtime verification and operational closure

Ship owns implementation and closure. Required proof before merge:

- deterministic S1-S4 plus generation/cancellation/quiescence controls pass;
- every Index, Sync, Hydration, Startup, legacy Watcher, and v2 Watcher permit receives cancellation and has exactly one terminal: ordinary completion/abandonment or retirement acknowledgment;
- no successor acquisition/start precedes ack, `max_active_db_drivers == 1`, and no old DB/file-capable work occurs after ack;
- same-binding barrier holds one `0b111` union, distinct binding carries zero old bits, requests coalesce in the barrier, and ack publishes plus invokes `notify_one` exactly once;
- successful no-pending release invokes `notify_one` once after unlock so at most one pre-registered empty waiter resumes, and multi-waiter Hydration/Startup/Watcher fixtures pass a finite one-owner baton;
- zero legacy caller and detached mutation-capable child inventory;
- exact queued JSON and MCP/CLI schema snapshots unchanged;
- no DB/file boundary before hydration permit; pending masks transfer whole and only once; no bounded-drain warning remains;
- Windows named-pipe bind, hydration, queued sync, cancellation, startup, and both watcher smoke paths succeed in a disposable workspace.

Operational closure records cancellation/ack traces, barrier duration, a manual monitoring checklist if no metric sink exists, a pre-deploy audit, a 15-minute Windows post-startup observation window owned by Ship, and full-release-unit rollback evidence. Roll back on missing cancellation delivery, acknowledgment before mutation-capable exit, successor-before-ack, active-driver count above one, work after ack, stuck barrier, duplicate/missing notification call, wrong-generation execution, stale terminal mutation, mask loss, pre-permit I/O, timestamp violation, queued response change, or Windows IPC regression.

Rollback is the complete coordinator release-unit revert followed by daemon restart. A stranded empty waiter, missing `Released` notification, non-progressing baton, spin, or concurrent empty owners is a release blocker. Partial rollback after permit caller migration is forbidden. There is no data/schema rollback.

## Decisions and rationale

- Select A, not B: no supported published Rust contract; public permits would expand support burden.
- Select A, not C: private REDs and internal callers prove migration feasibility.
- Mutex, not packed atomic: full-width generations and readable transition diagnostics outweigh unproven lock-free benefit.
- Ordinary completion may return one transferred successor; retirement acknowledgment never does. It publishes pending and invokes `notify_one` only after quiescence; at most one waiter resumes and no successor overlaps the retired driver.
- Normalize transferred work to `OwnerKind::Sync`: queued public behavior is coalesced sync, not producer-affine execution.
- Private tests over public seams: protects API containment and deterministic scheduling.
- Full release-unit rollback: caller signatures and ownership semantics cannot be partially reverted safely.

## Risks and caveats

| Risk | Mitigation / stop condition |
|---|---|
| Hidden downstream Rust consumer | `publish = false` evidence; stop before source mutation if a supported contract is produced. |
| Partial migration exposes tokenless completion | No partial merge/release; final zero-caller inventory is gate-blocking. |
| Completion/Drop/timestamp gap | Timestamp is written in the exact coordinator transition; old permit is disarmed before destruction; deterministic completion-versus-Drop counter proof. |
| Hydration or driver cancellation gap | Every permit receives the generation receiver; pre-acquisition cancellation is zero-permit and active drivers must quiesce DB/file work before acknowledgment. |
| Successor duplication/overlap | A RetirementBarrier blocks every request; only exact exit ack clears it and emits one notification. Activity counters prove no successor-before-ack and max one DB driver. |
| Successful release strands empty waiters | Every empty waiter pre-registers before recheck; exact no-pending completion unlocks then invokes `notify_one` once. Single/multi-waiter fixtures prove one-at-a-time acquisition and finite baton progress without treating notifications as authority. |
| Mask loss or cross-binding companion leak | Same-binding union and new requests live only in barrier.deferred; distinct retarget discards old/superseded-target masks; ack publishes the latest deferred mask once. |
| Stuck or non-quiescent driver | No timeout bypass; barrier remains closed, structured logs/observation detect it, and Ship performs full-unit rollback/restart. |
| Windows-only runtime behavior | Deterministic tests plus disposable Windows named-pipe validation. |
| Review drift toward old design | Old tasks marked superseded; forbidden-symbol inventory and P0/P1 gate. |

## Plan hardening signals

| Signal | Present? | Reason |
|---|---|---|
| Public API, schema, or contract change | Yes | Rust public visibility narrows, though package is non-published; CLI/MCP contract is frozen. |
| Security/auth/compliance | No | No trust boundary, credentials, or authorization behavior changes. |
| Migration/backfill/destructive/irreversible | Yes | In-memory source API migration is atomic and not partially reversible; no data migration. |
| External integration/operator checkpoint | Yes | Ship closure and Stage requeue are explicit checkpoints; Windows runtime is required. |
| High runtime/rollout/rollback risk | Yes | Daemon-wide concurrency and startup/hydration ownership change. |

**Requires plan hardening: yes.**


## Plan Hardening

**Hardening required: yes — applied.** The triggers are daemon-wide concurrency ownership, internal Rust visibility reduction, a coordinated four-module source migration, startup/hydration runtime behavior, Windows IPC validation, and all-or-nothing rollback.

### Reinforcing instructions and learnings

- `.github/instructions/strict-safety.instructions.md`
- `.github/instructions/release-observability.instructions.md`
- `.github/instructions/concurrency.instructions.md`
- `.github/instructions/circuit-breaker.instructions.md`
- `docs/compound/best-practices/packed-atomic-clear-requires-atomic-publish-2026-07-29.md`
- `docs/compound/concurrency-issues/atomicbool-drain-race-take-before-lock-2026-05-09.md`
- `docs/compound/concurrency-issues/pending-sync-drain-must-cover-all-finish-indexing-sites-2026-05-09.md`
- `docs/compound/best-practices/pub-visibility-for-external-test-harness-2026-04-20.md`

The older take-before-lock and finish-site learnings explain why the current re-arm/drain repair exists; the new plan supersedes that repair by moving the full mask with ownership at one completion linearization point. The external-test visibility learning is satisfied by migrating repository tests before narrowing visibility, never by exporting a permit.

### Reinforced stop conditions

Stop and return the release unit blocked if any implementation requires:

1. a tokenless completion bridge claimed to be stale-safe;
2. a second active owner, work queue, extracted-mask cache, split companion state, or producer reacquire;
3. a standard mutex guard across await;
4. a public, feature-gated-public, or test-only-public token/permit/seam;
5. sleeps, yields, permissions, or live daemon timing as correctness proof;
6. more than two production files, four production functions, three scenarios, or 110 minutes in one task;
7. a wire, schema, persistence, config, or queued-response change;
8. a partial merge, deployment, or rollback of the caller migration;
9. source mutation before exact visibility/caller reinspection confirms `publish = false` and no new supported Rust contract;
10. failure to compile a RED before executing it;
11. caller-optional cleanup, async/spawn work in Drop, stale Drop touching replacement state, or any OwnerKind without a cancellation receiver and deterministic observation proof;
12. clearing/bypassing a retirement barrier before exact driver exit acknowledgment, or acknowledging while a DB/file/workspace mutation-capable child remains live;
13. a timeout that force-opens a stuck barrier or any successor-before-ack path;
14. an exactly-once or RAII recovery claim across process abort;
15. an empty Hydration/Startup/Watcher wait path that requests before enabling notification, a successful `Released` path without a post-unlock notification, waiter polling, duplicated queue/work state, or a claim that `Notify` resumes exactly one task.

The source cutover is one release boundary. Width-safe tasks may checkpoint separately on the feature branch, but no checkpoint is eligible for PR merge or runtime use before `109.030-T` removes every legacy mutator and the aggregate source compiles. If the executor cannot stage the cutover without an unsafe compatibility bridge, it stops and returns the plan to Stage; it does not widen scope.

### Proposed actions

**ProposedAction PA-1**

- Summary: replace four independent synchronization authorities with one `Arc`-owned permit coordinator whose per-generation cancellation receiver and exact exit-ack barrier prevent old/new driver overlap.
- Targets: `src/server/state.rs`, then responsibility-isolated migrations in `lifecycle.rs`, `write.rs`, and `ipc_server.rs`.
- Change kind: high-blast-radius shared runtime edit.
- ActionRisk: high.
- Approval required: reviewed plan plus Ship execution safeguards; no Stage implementation.
- Rollback: revert the complete release unit and restart the daemon.
- ActionResult: planned.

**ProposedAction PA-2**

- Summary: reduce/remove tokenless Rust ownership and split-mask methods after every internal and repository-test caller migrates.
- Targets: `state.rs` and the three named repository test files.
- Change kind: internal contract/visibility migration.
- ActionRisk: high.
- Approval required: zero-legacy-caller proof and `publish = false` recheck.
- Rollback: full release-unit revert only; no deprecation bridge.
- ActionResult: planned.

**ProposedAction PA-3**

- Summary: execute deterministic compile-then-fail harnesses and later GREEN validation in disposable test state.
- Targets: co-located private tests plus named contract/integration tests.
- Change kind: non-destructive test-first verification.
- ActionRisk: moderate.
- Approval required: normal Ship task execution.
- Rollback: remove failed harness changes or return task blocked; never alter public visibility to make a test compile.
- ActionResult: planned.

**ProposedAction PA-4**

- Summary: validate the finished daemon on Windows named pipes in a disposable workspace and observe it for 15 minutes.
- Targets: local release candidate runtime, structured logs, health/workspace status, disposable files only.
- Change kind: runtime validation.
- ActionRisk: high.
- Approval required: Ship runtime-verification protocol; never use the operator workspace.
- Rollback: stop the candidate, revert the full release unit, restart the prior daemon.
- ActionResult: planned.

### Monitoring plan

There is no required new metrics sink. Ship records a structured manual checklist and existing tracing output in operational closure.

| SLI / signal | Query or observation | Healthy baseline | Alert / rollback threshold | Owner |
|---|---|---|---|---|
| Owner sequence balance | Structured acquire/complete/abandon/retire-ack logs | Every sequence has one ordinary terminal or one retirement acknowledgment; later terminals mutate nothing | Missing/duplicate terminal, wrong identity, or stale mutation | Ship |
| Full-mask/barrier location | Deterministic fields for phase, binding, generation, sequence, mask | Same binding: one 0b111 barrier mask then one pending publication; distinct binding: zero inherited bits | Mask in two locations, early publication, split/lost mask, or cross-binding companion | Ship |
| Hydration admission | Private pre-DB fixture counter and Windows hydration logs | Counter/log boundary remains zero before permit | Any DB/file boundary before `Acquired(Hydration)` | Ship |
| Driver quiescence | Cancellation/ack/activity counters for all OwnerKind rows | Cancellation delivered; no successor before ack; max active DB drivers 1; no old work after ack; one post-unlock notification call | Missing observation/ack, overlap, post-ack work, duplicate call, or stuck barrier | Ship |
| Empty-waiter liveness | Deterministic single/multi-waiter release fixtures | Notification enabled before request; `Released` invokes once; at most one acquisition per call; baton drains all registered rows without spin | Stranded waiter, missed registration, no baton progress, busy loop, duplicate authority, or concurrent owners | Ship |
| Timestamp completion | Private timestamp-write counter and doctor health observation | One write per valid current completion, zero per stale/mismatch | Any missing, duplicate, or stale-triggered write | Ship |
| Queued contract | Contract snapshot of status/message and MCP schema | Byte-equivalent status/message and unchanged schema | Any response/schema/error mapping change | Ship |
| Windows runtime liveness | daemon-status, workspace-status, named-pipe completion, barrier logs | One daemon, responsive IPC, no stuck retirement/drain warning | IPC timeout/hang, stuck barrier, duplicate daemon, or uncompleted queued work | Ship |

### Pre-deploy audit

Before merge, Ship records:

- `publish = false` and no supported Rust-library contract introduced;
- all 18 replacement tasks complete and old tasks remain superseded;
- zero forbidden legacy callers, zero public permit/test seams, zero caller-optional cleanup, and cancellation plus quiescent acknowledgment coverage for every OwnerKind;
- no Cargo dependency, feature, schema, persistence, wire, or config delta;
- exact queued response/schema snapshots unchanged;
- deterministic suite and Windows disposable-runtime evidence attached;
- rollback commit/range and prior daemon restart procedure identified;
- `102-S`/`103-S` predecessor evidence remains intact and unrelated work untouched.

### Observation window and rollback

Ship owns an active 15-minute post-startup observation window on Windows after all deterministic gates pass. Exercise one normal bind/hydration, one queued sync, the deterministic all-OwnerKind same/distinct active-rebind and forced-DB-failure matrix, and one Startup/legacy-Watcher/v2-Watcher handoff. The live smoke observes liveness and barrier logs only; deterministic fixtures remain the race proof.

Immediate rollback triggers are missing cancellation or acknowledgment, acknowledgment before DB/file-capable exit, successor-before-ack, more than one active DB driver, post-ack old work, stuck barrier, duplicate/missing notification call, a stranded empty waiter, failed baton progress, busy-loop polling, wrong-generation or cross-binding execution, stale mutation, orphan/split mask, pre-permit I/O, timestamp/queued-contract drift, or named-pipe regression. Rollback means revert the entire coordinator/caller/test release unit, restart the prior daemon, verify one healthy bind/status cycle, and record the outcome. No partial source rollback and no data/schema action.

### Operator and agent checkpoints

- Stage checkpoint now: docs/backlog only; no source/build/test/Git/shipment lifecycle action.
- Ship checkpoint 1: integrate findings and plan, verify source clean, then close only `106-S`/`109.013-T` after merge evidence exists.
- Stage checkpoint 2: execute the exact requeue transaction only after terminal 106 and positive 102/103 reads.
- Ship checkpoint 3: claim `104-S` only after Stage queues it; execute the coordinator release unit.
- Any unavailable index, ambiguous caller visibility, or non-terminal predecessor fails closed.


### PR #316 lifecycle hardening delta

- **Trigger:** discussion `r3701136926` proves that generation cancellation currently reaches only Hydration while Index/Sync and both watcher drivers can outlive owner clearing. `ActionRisk: high`.
- **Guardrail:** every `OwnerPermit` receives the generation cancellation receiver and retains its immutable binding snapshot for the complete DB/file-capable future. Exact Drop remains synchronous, poison-safe, and non-panicking.
- **Barrier rule:** active rebind advances binding/floor but converts the owner to one `RetirementBarrier`. Same binding stores `0b101 OR 0b010 = 0b111` in `deferred`; distinct binding stores zero old bits. Current requests coalesce there and no request acquires.
- **Acknowledgment rule:** explicit terminal or armed Drop clears the barrier only after mutation-capable work exits, publishes deferred, and invokes `notify_one` exactly once after unlock. Stale terminals remain no-ops; a stuck driver keeps the barrier closed.
- **Driver proof:** fixture matrices cover Index, Sync, Hydration, Startup, legacy Watcher, and v2 Watcher under same/distinct rebind, explicit/Drop acknowledgment, repeated rebind, and forced failure without exceeding three scenarios per task.
- **Abort boundary:** no Drop claim on process abort. Restart reconciliation handles durable state; applicable non-durable flags are reissued; full-unit revert/restart is rollback.
- **ActionResult:** applied to plan, findings, and blocked task scopes; implementation remains unapproved until the existing post-106 requeue gate.

### PR #316 successful-release waiter hardening delta

- **Trigger:** P1 `discussion_r3701238147` / `PRRT_kwDORJEduc6V25-i` shows that exact `Running` completion with no pending work selected `Released` but emitted no wake, stranding empty Hydration/Startup/Watcher waiters. `ActionRisk: high`.
- **Release rule:** exact no-pending completion clears owner, disarms and timestamps once, selects `Released`, drops the mutex, invokes `notify_one` exactly once, then returns. Running Drop and retirement acknowledgment retain the same one-call post-unlock discipline.
- **Registration rule:** each empty waiter owns an `AdmissionGuard` with `Notified` enabled before its final request/recheck. `Waiting(AdmissionGuard)` selects that registration against cancellation; every wake re-arms before rechecking.
- **Baton rule:** one notification call allows at most one mutex-authorized empty acquisition. Remaining waiters stay registered; the acquired empty owner completes `Released` and emits the next wake. A competing producer merely keeps the waiter queued. No polling, second queue, duplicate work state, notification authority, or concurrent drivers.
- **Counting rule:** tests assert exactly one `notify_one` invocation per qualifying transition, but only at-most-one resumed/acquired waiter because Tokio may coalesce or retain a permit. Progress, not an impossible exact resumed-task count, is the contract.
- **Bounded proof:** owner-success/single-empty-waiter and multi-waiter baton rows are folded into existing matrices/fixtures in `109.014-T`, `109.018-T`, `109.020-T`, `109.024-T`, and `109.031-T`; no task exceeds three scenarios, two production files, four production functions, or 110 minutes.
- **ActionResult:** applied in final review-fix cycle 3/3; all lifecycle/status/dependency/ownership caps remain unchanged.

## Superseded Plan Reviews

- The pre-cycle-1 PASS at PR head `897406cc` was superseded by `discussion_r3700752674` and `discussion_r3700752695`.
- The unconditional cross-binding promotion review was superseded at head `436436587d7383bf4f97a2699b8aa473703d37df` by six binding-isolation comments.
- The binding-aware PASS recorded at base HEAD `d6321504137445a94b4134718355b87cceb75db6` was superseded by P1 `discussion_r3701136926` / `PRRT_kwDORJEduc6V2olJ` because it cleared owner before Index/Sync/Watcher exit.
- The cycle-2 quiescence-barrier PASS at HEAD `2f267d9c617243dd70cbaac9837826a4fd0358e9` was superseded by P1 `discussion_r3701238147` / `PRRT_kwDORJEduc6V25-i` because successful no-pending `Running` completion did not wake empty waiters.

## Plan Review

**Review-fix cycle:** 3/3 (final allowed cycle)
**Fresh plan-review revision:** 4 (successful-release empty-waiter remediation)
**Model routing verification:** `.github/agents/stage.agent.md` declares `.Stage`, Tier 3/frontier, high reasoning, provider `anthropic`, family `claude-opus-4.8`; no override was supplied.
**Execution surface:** no subagent invocation surface was exposed, so `.Stage` directly applied all always-on and triggered personas under the skill fallback.
**Gate: PASS**
**Open P0: 0. Open P1: 0. Open P2: 0. Open P3: 0.**
**Plan hardening:** required, rerun for release/waiter liveness, and satisfied.

### Gate rationale

Copilot P1 `discussion_r3701238147` is valid. Empty Hydration/Startup/Watcher requests can return `Queued` behind a running owner, but the prior exact no-pending completion selected `Released` without notifying, so every registered waiter could remain blocked forever. The final plan closes the lost-wake gap: waiters enable `Notified` before final request/recheck; exact no-pending completion clears owner and selects `Released` under the mutex, unlocks, then invokes `notify_one` exactly once.

Multiple waiters progress by a finite baton. One notification permits at most one mutex-protected empty acquisition; remaining waiters stay registered. The acquired empty owner completes once, selects `Released`, and emits the next post-unlock notification. If a producer wins, the waiter remains queued and blocks on its fresh registration. Tests assert exact notification-call counts, not an invalid exact resumed-task count. Coordinator state remains authority; no work state is duplicated, no loop polls, and no two drivers acquire concurrently. All prior binding, cancellation, quiescence, RAII, stale-terminal, and process-abort contracts remain intact.

### Persona verdicts

- **Constitution Reviewer — PASS.** Only existing plan/decision/memory/backlog artifacts change. TDD order and the `<=110 minutes`, `<=2` production files, `<5` production functions, and `<=3` scenario limits remain unchanged.
- **Rust Reviewer — PASS.** A pinned `OwnedNotified` enabled before recheck closes the registration race, and acquisition drops that registration before returning a permit so it cannot steal a release wake. `notify_one` occurs only after dropping the standard mutex. Exactness applies to the call, while waiter resumption is correctly stated as at most one.
- **Scope Boundary Auditor — PASS.** Single/multi-waiter coverage is folded into existing fixtures for the coordinator, Hydration, lifecycle handoff, Startup/Watcher, and final validation; no new task, dependency, file family, status, or cap is introduced.
- **Learnings Researcher — PASS.** The rule preserves whole-mask ownership and the no-second-queue/no-producer-reacquire decisions while adding only liveness triggering.
- **Architecture Strategist — PASS.** `Notify` remains a hint, never authority. Mutex-protected request chooses one owner; empty-owner `Released` passes the baton; competing work safely delays but cannot strand waiters.
- **Agent-Native Parity Reviewer — PASS.** Busy Index behavior, exact queued Sync JSON, MCP/CLI schemas/errors, health meaning, startup behavior, and persisted formats remain frozen.
- **Security Lens Reviewer — PASS.** Same-binding `0b111` remains only in `RetirementBarrier.deferred`; distinct binding carries zero old bits; no trust-boundary or sensitive-data surface changes.

### Internal consistency checks

- `109.014/015` own exact `Released`/Drop notification semantics and core single/multi-waiter fixtures within three scenarios.
- `109.018/019`, `109.020/021`, and `109.024/025` require pre-registration, blocked waits, and baton behavior for Hydration/Startup/both Watchers.
- `109.031` folds owner-success and multi-waiter rows into its existing three fixtures and distinguishes exact notification calls from at-most waiter resumption.
- Retirement acknowledgment and armed Drop still notify once after unlock; success disarms; stale terminals notify zero; process abort uses reconciliation.
- Dependency chain remains `109.013 -> 109.014 -> ... -> 109.031`. `106-S`/`109.013-T` remain active; `104-S`/`109-F`/all old and replacement tasks remain blocked and unclaimed.

### Review decision

Final cycle 3/3 passes with zero open P0-P3 findings only as blocked replacement scope. PASS does not authorize source/test/Cargo work, closing `106-S`/`109.013-T`, requeueing `104-S`/`109-F`, shipment claim/closure, Git/PR operations, replies, or thread resolution. Ship may commit these uncommitted planning changes and post the suggested reply; the existing post-106 Stage transaction remains the sole requeue gate.


## Phase 5E authoritative cancellation-ownership amendment

### Residual P1 disposition

PR #316 discussion `r3701318733`, companion findings comment `r3701318749`, and the Ship comment on blocked `109-F` at commit `c6f2b06174b10724ed9527601cd4ad6448c1433d` are **valid**. The prior contract gave a cancellation receiver only to `OwnerPermit`, while a bare `Queued` result and floor-only `GenerationToken` left a pre-acquisition empty waiter with nothing to select against. A rebind intentionally does not notify the coordinator, so an idle/no-later-owner-transition row could wait forever. The earlier PASS is superseded and is not evidence for requeue.

This section is authoritative over every earlier reference to a bare token or bare `Queued` outcome. The internal API uses a non-cloneable ownership chain:

```text
AdmissionGuard {
  cell, token: GenerationToken, binding_snapshot,
  cancel_rx, enabled_notification
}
RequestOutcome = Acquired(OwnerPermit) | Waiting(AdmissionGuard) | Enqueued | Stale
OwnerPermit {
  cell, token, binding_snapshot, cancel_rx,
  identity, work_mask, cleanup_armed
}
DriverTaskGuard { join_handle, abort_handle, terminal_state }
```

The coordinator may clone the current watch receiver only while minting a new `AdmissionGuard`; callers cannot extract or clone it. `request` consumes the guard. Acquisition drops the enabled notification registration, then moves the same receiver, binding snapshot, token, and coordinator cell into one armed `OwnerPermit`; the permit retains no waiter registration. An empty internal waiter receives `Waiting(AdmissionGuard)` carrying the already-enabled notification and receiver. A non-empty busy producer receives `Enqueued` only after the whole `WorkMask` is authoritative in `pending` or `RetirementBarrier.deferred`; the caller owns no mask or cancellation obligation after that return. `Stale` mutates nothing.

Every empty wait iteration enables its notification before `request`. `Waiting` selects that owned registration against its owned cancellation receiver. A notification consumes the registration and the guard is re-armed before recheck; generation cancellation exits by dropping the pre-acquisition guard with zero coordinator mutation. Rebind signals the old channel even when phase is `Idle`, creates no coordinator notification, and therefore deterministically cancels a waiter when no later owner transition occurs.

Direct idle `Index` and `Sync` acquisitions preserve the requested `OwnerKind`. Only a successor created from coalesced pending work is normalized to `Sync`. A completion transfer moves, rather than clones or exposes, cancellation/binding ownership into the successor permit. Dropping or aborting that successor before execution invokes its armed Drop and republishes the entire mask once.

### Mandatory cleanup ownership matrix

| Exit class | Required owner | Required terminal behavior |
|---|---|---|
| Pre-acquisition cancellation or caller return | `AdmissionGuard` / `Waiting(AdmissionGuard)` | Drop guard; no completion, acknowledgment, work mutation, or notification. Enqueued non-empty work is already coordinator-owned. |
| Post-acquisition normal/error/`?`/early return | Armed `OwnerPermit` lexically outside the complete mutation-capable future | Explicit completion consumes/disarms on handled success/failure; any escaping path drops the exact permit and performs running recovery or retirement acknowledgment. |
| Completion-transferred permit | Successor `OwnerPermit` containing the moved guard ownership and full mask | Execute once or, on loss/cancel/abort, Drop republishes the full mask once and wakes once after unlock. |
| Spawned Hydration/Startup/Watcher/progress helper | Parent-retained `DriverTaskGuard`; spawned future owns the `OwnerPermit` and an inner mutation-capable future | Raw `JoinHandle` loss/detach is forbidden. Normal shutdown consumes and joins. Guard Drop aborts. The inner DB/file/workspace future is dropped or joined before permit Drop/ack; the barrier stays closed until that terminal runs. |
| Caller future aborted while inline driver runs | The inline future owns `OwnerPermit` outside an inner operation scope | Operation future drops first; armed permit drops second. No caller cleanup call is required or available. |
| Process abort | none | No RAII claim. Restart hydration/offline reconciliation, qualified intent reissue, full-unit rollback/restart. |

The current ignored `_task` for Hydration and ignored `tokio::spawn` handles for both watcher loops are explicit migration targets. The retained supervisor slot or outer daemon scope must own `DriverTaskGuard`; replacing or dropping a slot aborts the old task without admitting a successor, and normal shutdown joins it. The write-path scan-progress child is also joined or abort-on-drop before the owner terminal so it cannot mutate workspace progress after permit acknowledgment. Any mutation-capable `spawn_blocking` or other child that cannot be cancelled and joined before acknowledgment is a stop-and-replan condition; CPU-only authority-free parse workers are the only allowed exception.

No API permits `cancel_rx` extraction, bare floor-only admission, caller-optional cleanup, raw detached driver handles, or owner installation without returning an armed permit. No standard mutex crosses await; Drop remains synchronous, poison-safe, non-panicking, non-spawning, and non-allocating. There is still one coordinator, one pending/deferred mask location, no second queue, no sleeps or public test seam, and no unsafe code.

### Phase 5E deterministic proof allocation

The existing 18-task chain remains sufficient; no new replacement IDs are required. Scenario matrices are parameterized rows, not additional scenarios.

- `109.014-T` / `109.015-T`: pre-acquisition rebind with no owner transition/no notify; guard-to-permit move; `?`/early-return/Drop terminal matrix; direct-kind preservation and no owner-without-permit.
- `109.016-T` / `109.017-T`: old-channel signaling reaches both waiting admission guards and every active permit, including idle/no-successor rebind, while active retirement remains acknowledgment-gated.
- `109.018-T` / `109.019-T`: retained Hydration task guard, dropped/aborted parent-handle rows, operation-before-permit-drop ordering, and DB failure/early return.
- `109.020-T` / `109.021-T`: transferred permit moves cancellation ownership and full mask; loss/abort/early return republishes once without a second drain.
- `109.022-T` / `109.023-T`: Index/Sync `?`/caller abort plus joined/abort-on-drop progress child; busy Sync `Enqueued` keeps exact public JSON.
- `109.024-T` / `109.025-T`: parent-retained and shutdown-joined/abort-on-drop Startup and both Watcher task handles; no detached permit owner.
- `109.030-T`: structural zero-inventory for bare admission, receiver extraction, optional cleanup, ignored/raw driver handles, and detached mutation-capable children.
- `109.031-T`: three parameterized scenarios cover pre-acquisition no-wake cancellation, post-acquisition early return, transferred-permit abort, and spawned-handle loss/abort.

Each task remains `<=2` files, `<=3` scenarios, and `<=110` minutes. A task that cannot preserve those caps or prove child-before-permit destruction returns blocked to Stage.


## Plan Hardening — Phase 5E

**Hardening required: yes — freshly applied after the residual PR #316 P1.** The added risks are pre-acquisition lost cancellation, guard transfer discontinuity, Tokio JoinHandle detachment, caller abort/early return, and mutation-capable child work outliving permit acknowledgment.

### Reinforcing context

- `.github/instructions/strict-safety.instructions.md`
- `.github/instructions/release-observability.instructions.md`
- `.github/instructions/concurrency.instructions.md`
- `.github/instructions/circuit-breaker.instructions.md`
- `docs/compound/best-practices/packed-atomic-clear-requires-atomic-publish-2026-07-29.md`
- `docs/compound/concurrency-issues/atomicbool-drain-race-take-before-lock-2026-05-09.md`
- `docs/compound/concurrency-issues/pending-sync-drain-must-cover-all-finish-indexing-sites-2026-05-09.md`

The old learnings reinforce atomic whole-mask publication and complete terminal coverage. Phase 5E extends the same rule to cancellation and spawned-task ownership: the authority chain is incomplete if a receiver, JoinHandle, or transferred mask becomes caller-detached.

### Protected hardening invariants

1. Exactly one non-cloneable admission/permit guard owns each caller-side cancellation receiver; coordinator minting is the only clone point.
2. A pre-acquisition waiter can exit on rebind with no owner transition and no coordinator wake.
3. Acquisition drops the enabled waiter registration and moves cancellation/binding ownership into the permit without an unguarded owner interval; completion transfers only permit ownership.
4. A non-empty Enqueued return occurs only after the complete mask is coordinator-owned.
5. Raw JoinHandle drop/detach is forbidden for Hydration, Startup, Watchers, progress, or any mutation-capable child; parent scopes retain DriverTaskGuard.
6. Normal task shutdown joins; guard Drop aborts; permit terminal remains the only barrier-release authority and runs only after inner mutation-capable work ends.
7. Caller `?`, return, panic, or future abort requires no optional cleanup call and cannot strand owner/mask.
8. No mutex across await, second queue, sleep-based proof, public seam, unsafe, forced barrier timeout, or process-abort RAII claim.

### ProposedAction PA-5

- Summary: replace bare queued/token cancellation with continuous AdmissionGuard to OwnerPermit ownership and supervise every permit-capable spawned task with parent-retained abort-on-drop/join-on-normal guards.
- Targets: the accepted plan, findings, and tasks `109.014-T` through `109.031-T`; later Ship source targets remain the four already-declared modules.
- Change kind: high-blast-radius internal concurrency contract amendment.
- ActionRisk: high.
- Approval required: operator Phase 5E instruction plus fresh zero-P0/P1 plan review; Ship still owns implementation and runtime approval.
- Rollback: return `104-S`, `109-F`, and replacements to blocked; restore planning contracts; no partial source rollout.
- ActionResult: applied to planning/backlog contracts; implementation not started.

### Verification, monitoring, and rollback delta

Deterministic RED rows must prove the no-later-wake pre-acquisition case, moved guard ownership, early-return/abort recovery, transferred-mask loss, and parent-handle loss/abort. Structural inventory must prove zero receiver extraction, bare queued waiter, ignored raw driver handle, optional cleanup, or unjoined mutation-capable child. Existing owner sequence, barrier, full-mask, timestamp, queued JSON, Windows named-pipe, 15-minute observation, and full-unit rollback signals remain mandatory.

Ship stops before source mutation if a task exceeds two files, three scenarios, or 110 minutes, or if the reusable task guard cannot fit the declared function cap. Ship stops during execution on any detached mutation-capable task, child-outliving-ack path, guard gap, successor-before-ack, or inability to join/cancel a mutation-capable child. No unresolved operator decision remains for Stage; requeue still requires the fresh review and exact terminal prerequisite transaction.


## Plan Review — Phase 5E fresh gate

**Review epoch:** fresh post-PR-316 residual review, revision 5; this does not revive rejected `109.001-R` or rely on any superseded PASS.

**Model routing verification:** `.github/agents/stage.agent.md` declares `.Stage`, Tier 3/frontier, high reasoning, provider `anthropic`, family `claude-opus-4.8`; no override was supplied.

**Execution surface:** no subagent invocation surface was exposed. Under the plan-review fallback, `.Stage` directly applied all always-on and triggered personas using the configured model. The Security lens was included conservatively because the internal API controls daemon-wide concurrency authority.

**Hardening required:** yes.

**Hardening satisfied:** yes; the fresh Phase 5E section classifies PA-5, protects continuous guard/task ownership, adds deterministic proof, retains monitoring/rollback, and names fail-closed stop conditions.

**Gate: PASS**

**Accepted review artifact:** `109.002-R`

**Open findings: P0 0 / P1 0 / P2 0 / P3 0.**

### Gate rationale

The residual P1 is valid and is fixed in contract rather than rebutted. A floor-only token and bare queued result are replaced by a consumed non-cloneable `AdmissionGuard`. The guard owns the enabled notification and receiver before acquisition; request either drops the registration and moves cancellation/binding ownership into an armed permit, returns guarded Waiting, commits a full non-empty mask before Enqueued, or returns Stale. Idle rebind signals cancellation even without an owner transition or coordinator wake.

Ownership remains continuous after acquisition. Direct Index/Sync kind is preserved. Completion transfers permit cancellation/binding ownership and the full mask into an armed successor. Every fallible/early-return/abort path is inside mandatory RAII scope. Hydration, both Watchers, Startup, and progress mutation use parent-retained join-on-normal/abort-on-drop task guards, and inner mutation-capable work ends before permit Drop or acknowledgment. The retirement barrier still prevents a successor until that exact terminal. No detached receiver, permit, task handle, or full mask remains.

The proof is width-safe: existing RED/GREEN matrices absorb the new rows, each task remains at most two files, three scenarios, and 110 minutes, and dependencies remain the single `109.013-T -> 109.014-T -> ... -> 109.031-T` chain. No source/test/Cargo, public seam, second queue, mutex-across-await, sleep proof, forced timeout, unsafe, wire/schema/persistence change, or process-abort RAII claim is authorized.

### Persona verdicts

- **Constitution Reviewer — PASS.** Planning/backlog-only scope is preserved; RED precedes each GREEN; every amended task has acceptance criteria, one concern, and bounded files/scenarios/time.
- **Rust Reviewer — PASS.** Receiver cloning is confined to guard minting; ownership moves through non-Clone guards; synchronous Drop and abort handles do not await; operation-before-permit destruction and the no-unsafe rule are explicit.
- **Scope Boundary Auditor — PASS.** No new task is needed. The reusable DriverTaskGuard is introduced in `109.019-T` before write/IPC reuse, while all tasks remain within two files, fewer than five production functions, and three scenarios.
- **Learnings Researcher — PASS.** Whole-mask atomic publication, complete finish-site coverage, and take-before-lock lessons are retained; the amendment extends them consistently to cancellation/task ownership.
- **Architecture Strategist — PASS.** Coordinator phase remains sole authority. `Waiting` carries no work, `Enqueued` carries no caller ownership, direct kinds remain diagnostic, and transferred work has one guarded successor.
- **Agent-Native Parity Reviewer — PASS.** Busy Index behavior, exact busy Sync queued JSON, MCP/CLI schemas/errors, startup contract, health meaning, and persistence remain frozen.
- **Security Lens Reviewer — PASS.** Binding snapshots and same/distinct retirement rules remain fail-closed; detached mutation authority, stale terminals, cross-binding mask carryover, and forced barrier bypass are forbidden.

### Internal consistency checks

- Main design, findings, and amended tasks use `Acquired | Waiting | Enqueued | Stale`; no live bare-Queued admission contract remains.
- `109.014/015` own pre-acquisition no-wake cancellation and core move-only guard semantics.
- `109.018/019` define/reuse DriverTaskGuard and supervise Hydration before later write/IPC tasks depend on it.
- `109.020/021` own transferred-guard/full-mask loss recovery.
- `109.022/023` own caller early returns and progress-child termination.
- `109.024/025` own Startup/legacy-Watcher/v2-Watcher handle retention, join, and abort.
- `109.030/031` provide structural zero-inventory and aggregate deterministic/runtime stop evidence.
- Existing binding barrier, full-mask, empty-waiter baton, timestamp, process-abort, monitoring, and full-unit rollback requirements remain unchanged.

### Review decision

The amended plan is accepted for the exact Stage restart transaction only after positive terminal reads for `106-S`, `109.013-T`, `102-S`, and `103-S`. PASS authorizes backlog/manifest requeue, not implementation, shipment claim/closure, source/test/Cargo edits, builds, Git operations, PR actions, or worktree changes. A fresh accepted review artifact must be created under `109-F`; rejected `109.001-R` remains rejected and superseded.
