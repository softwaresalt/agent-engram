---
title: "Post-105 single-authority sync coordinator permit migration"
type: impl-plan
doc_type: plan
date: 2026-08-02
status: blocked
source: "docs/decisions/2026-08-01-post-105-sync-coordinator-spike-findings.md"
feature: "109-F"
shipment: "104-S"
tags: ["concurrency", "pending-sync", "permit", "internal-api", "raii", "cancellation-safety", "ready-after-106-closure"]
---

## Pipeline gate

This fresh replacement plan is **`ready_after_106_closure`**, represented by the supported plan body marker and backlog label `ready-after-106-closure`; its actual status remains `blocked`. It does not authorize implementation or a status change now. `106-S` and `109.013-T` remain active. `104-S`, `109-F`, and every implementation task remain blocked until Ship integrates the findings and closes `106-S`, then Stage executes the exact requeue transaction in this plan.

This plan supersedes `docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md` and tasks `109.001-T` through `109.012-T`. None of those task scopes remains accurate: they retain tokenless completion, `Reacquired`, split mask takes, or bounded double-drain behavior rejected by the spike.

## Problem frame

Engram currently uses separate synchronization authorities for generation, an `AtomicBool` owner, a mutex-protected pending mask, and lifecycle-local hydration/drain behavior. Four deterministic REDs proved that stale work mutates newer masks, completion releases without transferring pending work, hydration reaches DB admission without ownership, and startup/release can select zero executors.

The replacement is strategy A from the findings: one private `SyncCoordinator` owns generation floor, sequenced owner identity, complete pending `WorkMask`, and hydration/drain handoff. All critical production callers are internal to the non-published crate. Tokenless ownership mutators are removed or reduced after caller/test migration. Public CLI/MCP, wire, schema, persistence, queued-response, and startup behavior stay compatible.

## Source and compatibility decision

- Source basis: exact review HEAD `d6321504137445a94b4134718355b87cceb75db6`; targeted source reinspection confirms that only hydration consumes the existing generation-cancellation receiver, while Index/Sync and both watcher drivers do not.
- Evidence basis: `docs/research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md`, reviewed evidence commit `b5d5802e`.
- Package contract: `Cargo.toml` has `publish = false`; Engram ships a binary. No public Rust permit API is added.
- API decision: opaque `GenerationToken`, `OwnerPermit`, `OwnerKind`, and `WorkMask` are `pub(crate)` at most, with private fields. The permit is non-`Clone`, owns an `Arc<CoordinatorCell>`, and has mandatory non-awaiting RAII abandonment cleanup.
- Semver: no major bump or deprecation bridge. This is an internal source migration. A supported downstream Rust API discovered before source mutation is a hard stop requiring strategy B and a new decision; tokenless adapters are never accepted as a fallback.
- Repository tests: migrate direct owner setup to public tool behavior or co-located private tests. No public test-only seam.

## Requirements trace

| Requirement | Implementation action | Verification units |
|---|---|---|
| One authority | Replace owner flag, generation, mask, timestamp, and handoff with one `Arc<CoordinatorCell>` | `109.014-T` -> `109.015-T` |
| Drop/cancellation safety | Every permit carries the current generation cancellation receiver; armed Drop either recovers a running owner or acknowledges a retirement barrier once | `109.014-T` / `109.015-T` |
| Successful finish disarms cleanup | Exact completion disarms old permit before Drop and writes timestamp once | `109.014-T` / `109.015-T` |
| Binding-aware generation retirement | One lock transition advances binding/floor but retains the retired driver behind a quiescence barrier; same-binding work lives in the barrier while distinct-binding carries zero old bits | `109.016-T` -> `109.017-T` |
| Quiescence and stale isolation | No successor acquires before the exact retired permit acknowledges exit; later finish/Drop is stale and cannot mutate or wake | `109.016-T` / `109.017-T`; driver matrices in `109.018-T`-`109.025-T` and `109.031-T` |
| Hydration owns before I/O | Pre-acquisition cancel is zero-permit; post-acquisition cancel relies on RAII | `109.018-T` -> `109.019-T` |
| Full-mask single successor | Completion or abandonment exposes one whole mask to one successor | `109.020-T` -> `109.021-T` |
| Write migration | Index/sync use guarded permits; exact queued JSON; no producer reacquire | `109.022-T` -> `109.023-T` |
| Startup/watcher arbitration | Typed guarded permits and one request linearization | `109.024-T` -> `109.025-T` |
| Compatibility tests | Replace tokenless setup with behavior/private harnesses | `109.026-T`, `109.027-T` |
| Visibility reduction | Retire tokenless owner, split pending, and companion mutators | `109.028-T` -> `109.030-T` |
| Process-abort boundary | No Drop claim on abort; startup reconciliation, intent reissue, full rollback | `109.031-T` |
| Runtime/release closure | Deterministic suite plus disposable Windows daemon validation | `109.031-T` |

## Authoritative design

### State, cancellation, and RAII ownership

```text
CoordinatorCell { state: std::sync::Mutex<SyncCoordinator>, notify: Notify }
BindingIdentity { workspace_uuid, workspace_id }  // private exact equality; workspace_id includes path/branch
SyncCoordinator {
  floor, binding_identity: BindingIdentity, next_sequence,
  phase: Idle | Running(OwnerRecord) | Retiring(RetirementBarrier),
  pending: Option<WorkMask>, generation_cancel, last_indexed_at
}
GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerIdentity { generation, sequence, kind }
OwnerRecord { identity, binding_identity, work_mask }
RetirementBarrier {
  retired_identity, retired_binding, target_generation, target_binding,
  deferred: WorkMask
}
OwnerPermit {
  cell: Arc<CoordinatorCell>, identity, binding_snapshot, work_mask,
  cancel_rx: watch::Receiver<bool>, cleanup_armed
}
```

`AppState` owns one private `Arc<CoordinatorCell>`. Permits clone the cell, remain non-`Clone`, and expose no public fields. The existing `tokio::sync::watch` facility becomes the per-generation cancellation broadcast; every acquired `OwnerPermit` receives a receiver clone, and pre-acquisition generation waiters receive the same signal. `Notify` stores neither work nor identity.

The coordinator phase is the sole authority. `pending` is used only in `Idle`/`Running`. During `Retiring`, `RetirementBarrier.deferred` is the sole current-generation work location; the retired permit retains an immutable old binding snapshot and a non-authoritative mask copy only so it can stop safely. No current permit exists and no successor can acquire until the barrier is acknowledged.

Every driver owns its permit around the complete DB/file-capable future. Rebind signals its receiver. The driver either observes cancellation and explicitly acknowledges only after its DB/file-capable future has been dropped or joined, or task cancellation/panic unwinds that future before armed `Drop` acknowledges. A detached task with DB, file, coordinator, or workspace-mutation authority is forbidden; discovering one is a stop-and-replan condition. A CPU-only parse worker may finish after its parent future is dropped only when it holds no such authority.

### Request transition

`request(token, mask, kind)` returns fallible `Acquired(permit) | Queued | Stale` after validating under the coordinator lock.

- In `Running`, a current producer ORs one complete mask into ordinary `pending`; an empty-mask waiter publishes no work. Stale requests mutate nothing.
- In `Retiring`, no request can acquire. A current-token non-empty request ORs into `RetirementBarrier.deferred`; an empty-mask waiter publishes no work and waits for the mandatory acknowledgment notification. The caller receives `Queued`. Thus same-binding `0b111` and all later current-generation requests remain in one authoritative barrier slot. Old-token requests remain stale.
- A rebind that arrives while already retiring retargets the same barrier, never creates another retired owner: equal target binding preserves and retags `deferred`; a distinct target binding discards the previous target-binding work before accepting later requests for the newest token.
- In `Idle`, a non-empty request atomically takes `pending OR requested` into one `Sync` permit; concurrent requesters cannot take it twice. An empty-mask Index/Hydration request may acquire and transfer pending on exact completion.
- Each acquisition clones the current generation cancellation receiver into the permit. Sequence exhaustion fails before mutation. No path installs an owner without returning its permit.

Public behavior remains mapped at the caller boundary: busy Index retains `IndexInProgress`, busy Sync retains the exact queued JSON, and internal Hydration/Startup/Watcher paths wait on `Notify` then recheck. `Queued` never authorizes execution.

### Explicit completion and retirement acknowledgment

`complete(mut permit)` consumes the permit and compares generation, sequence, and kind under the mutex.

- An exact `Running` match follows the accepted rule: pending installs one armed `Sync` successor returned as `Transferred`; no pending returns `Released`. Both write `last_indexed_at` once and disarm the old permit.
- An exact `Retiring` match is not ordinary success and not stale: it is `RetirementAcknowledged`. Only after the driver has quiesced does this terminal move `deferred` to the latest target generation ordinary pending slot, clear the barrier, disarm the old permit, and perform exactly one unconditional post-unlock `notify_one`. It writes no completion timestamp and never returns a successor permit to the retired driver.
- Any later or identity-mismatched terminal disarms only its local guard and returns `Stale`. It cannot mutate phase, pending/deferred mask, floor, binding, notification count, or timestamp.

The coordinator lock linearizes completion versus rebind. Completion first is a regular current terminal before publication. Rebind first converts that same terminal into the unique retirement acknowledgment. No standard mutex crosses `.await`.

### Mandatory Drop transition

`OwnerPermit::drop` is the mandatory terminal guard. If armed, it locks synchronously with poison recovery.

- An exact `Running` match retains the accepted abandonment rule: compute authoritative `owner.work_mask OR pending`, clear the owner, publish the non-empty union once, unlock, and notify once.
- An exact `Retiring` match acknowledges quiescence. It does not OR the old permit mask again because that mask already lives in `RetirementBarrier.deferred`. It publishes deferred to ordinary pending, clears the barrier, and performs the same single post-unlock wake as explicit acknowledgment, without a timestamp.
- Any identity mismatch is a strict no-op. Drop never allocates a sequence, awaits, spawns, or panics.

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

Only exact explicit terminal or armed Drop of the retired permit acknowledges exit. The acknowledgment atomically publishes the latest deferred mask to ordinary pending, clears the barrier, and wakes exactly once after unlock when work or a waiter exists. The successor then competes through normal `request` and cannot overlap the old driver. Same-binding replay may reconcile work already partly attempted, but there is never duplicate authoritative ownership or concurrent same-database execution. Distinct-binding acknowledgment exposes only requests issued for the latest new binding; durable state still uses new-binding startup/hydration/offline-change reconciliation and non-durable intent still requires a new-token request.

Every Index, Sync, Hydration, Startup, and both legacy/v2 Watcher driver must hold its permit and immutable binding snapshot for its whole DB/file-capable future, observe cancellation, and reach one terminal. Cancellation may drop a cancellation-safe operation future; any non-cancellation-safe or detached DB/file mutator must be joined before acknowledgment. Old normal completion after retirement is the acknowledgment and writes no timestamp. Any terminal after acknowledgment is stale and harmless.

### Hydration, driver cancellation, and process-abort boundary

Hydration registers `Notified` before final request check and selects notification versus its generation cancellation receiver without a standard mutex. Cancellation before acquisition exits with no permit or acknowledgment. After acquisition, Hydration follows the same barrier rule as every other owner: no DB/file boundary before `Acquired`, and on rebind it drops or joins DB/file-capable work before explicit acknowledgment or armed Drop.

Index/Sync wrap the complete write driver in the permit lifetime. Startup and both watcher loops do the same for each acquired execution. They check cancellation before starting another phase and select it against cancellable waits. A cancellation observation suppresses flush, drain, or any later phase; acknowledgment occurs only when the current DB/file-capable phase has actually ended. Deterministic activity counters, not sleeps, prove `max_active_db_drivers == 1` and zero old-driver work after acknowledgment for every `OwnerKind`.

Rust Drop is not claimed for process abort. Restart reconstructs in-memory authority; bind/hydration and offline-change detection reconcile durable files. Non-durable revalidate/backfill intent must be reissued. Runtime invariant failure or a stuck retirement barrier uses full release-unit revert and daemon restart.

### Compatibility boundary

Safe observers may remain stable. Request-only publication may only be crate-private delegation to `request`; it cannot stage companion-only state. Tokenless claim/completion/generation-clear/producer-reacquire/split-take/companion setters retire. The exact queued result remains:

```json
{"status":"queued","message":"Sync queued; will run after current indexing completes"}
```

## Protected invariants

1. The coordinator cell is sole authority for binding floor, exact binding identity, owner phase, full mask, handoff, cancellation generation, and completion timestamp.
2. In `Idle`/`Running`, each set `WorkMask` bit exists in exactly one of owner or ordinary pending; in `Retiring`, all current-generation work exists only in `RetirementBarrier.deferred`.
3. Every `OwnerKind` receives and observes generation cancellation. No successor permit can be acquired or run until the exact retired permit acknowledges that all DB/file-capable work has exited.
4. Same-binding active-owner advance atomically moves `owner mask OR pending` into the barrier under the new generation; `0b101 OR 0b010 = 0b111` is mandatory and new requests coalesce there.
5. Distinct-binding advance moves zero old routine/revalidate/backfill bits; only latest-binding requests may accumulate behind the barrier, durable state uses new-binding reconciliation, and non-durable intent requires new-token reissue.
6. A second rebind during retirement retargets one barrier: same target binding preserves deferred work; distinct target binding discards superseded-target work; no second retired owner or successor appears.
7. Exact retirement acknowledgment, whether explicit terminal or Drop, publishes deferred/clears the barrier once and performs exactly one post-unlock wake. It never timestamps or returns a successor to the retired driver.
8. Exact running Drop republishes authoritative owner mask OR pending, clears owner once, and wakes at most one. Exact running completion disarms Drop and timestamps once.
9. Stale request, finish, acknowledgment, and Drop mutate and notify zero times.
10. Hydration does zero DB/file I/O before acquisition. Index/Sync/Hydration/Startup/Watcher retain immutable binding snapshots and permits for their complete DB/file-capable futures.
11. No detached DB/file/workspace mutator can survive acknowledgment; failure to prove quiescence blocks the release unit. A stuck driver keeps the barrier closed.
12. No mutex crosses await; Drop never awaits, spawns, or panics.
13. No second queue, double drain, split consumption, producer reacquire, sleeps, unsafe, or public test seam.
14. No CLI/MCP/wire/schema/persistence/config/queued-response regression.
15. No exactly-once or RAII claim crosses process abort; restart reconciliation, qualified reissue, and full rollback remain explicit.

## Implementation units

Every unit is `<=110 minutes`, touches `<=2` production files, changes fewer than five production functions, and has `<=4` deterministic scenarios. RED tasks change zero release behavior. GREEN starts only after its direct RED compiles and fails intended assertions. No partial migration is mergeable/releasable.

### 1. `109.014-T` — RED: permit cancellation, completion, and Drop lifecycle

- Files: `src/server/state.rs` only.
- Scenarios (4): an `OwnerKind` fixture matrix proves every acquired permit carries and observes its generation cancellation receiver; exact current Drop republishes authoritative owned mask OR pending and clears owner once; one later requester takes that union once; a terminal matrix proves exact completion disarms Drop/timestamps once while stale completion/Drop mutates and wakes zero times.
- Use private deterministic cancellation and notification counters; no driver source or release behavior changes.
- Dependency: `109.013-T` terminal, while status remains blocked until the existing Stage requeue gate.

### 2. `109.015-T` — GREEN: cancellation-bearing RAII coordinator core

- Files: `src/server/state.rs` only.
- Exactly four production-function touches: existing AppState construction initializes the cell; request clones the receiver; complete handles exact running terminals; OwnerPermit Drop handles running abandonment.
- Implement `Idle/Running` authority, a generation cancellation receiver in every permit, authoritative owner mask, completion disarm, synchronous poison-safe running Drop, and timestamp-in-transition. The `Retiring` branch is added only by `109.017-T` after its RED.
- No caller-optional cleanup, async Drop work, second queue, tokenless bridge, or owner without a permit.
- Dependency: `109.014-T`.

### 3. `109.016-T` — RED: active rebind quiescence and stale isolation

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (4 maximum): complete old/new binding-token-cancel-floor tuple; `u64::MAX` no mutation; one parameterized active-rebind matrix over same/distinct binding, every `OwnerKind`, and explicit/Drop acknowledgment; one repeated-rebind/stale-terminal matrix.
- The active matrix seeds owner `0b101` plus pending `0b010`. Same binding must expose `0b111` only in `RetirementBarrier.deferred`; distinct binding must expose zero old bits. Current-token requests coalesce in deferred, cancellation reaches the retired permit, and attempted successor acquisition remains blocked until exit acknowledgment.
- Ack must move the latest deferred mask to ordinary pending, clear the barrier, and wake exactly once; deterministic activity counters remain at one and old work count cannot increase after ack. Repeated same-target rebind preserves deferred; distinct retarget discards superseded-target work. Later finish/Drop is stale and changes nothing.
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
- Scenarios (3): held owner prevents pre-DB signal; a phase matrix covers pre-acquisition cancellation plus same/distinct active rebind after Hydration acquisition; acquired DB-connect failure explicitly completes/disarms.
- The active rows prove cancellation is observed, no new permit starts before Hydration drops/joins DB/file-capable work and acknowledges, acknowledgment wakes once, and no old work occurs after ack. Pre-acquisition cancellation has no permit or ack.
- Bind the harness to a private production collaborator; no live DB timing.
- Dependency: `109.017-T`.

### 6. `109.019-T` — GREEN: cancellation-aware hydration guard

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four functions cover notify/retry admission, permit cancellation observation, hydration body, and exact-exit finalization.
- Register `Notified` before recheck. Pre-acquisition generation cancellation exits without ack. After acquisition, select cancellation at safe boundaries; drop or join all DB/file-capable work before explicit retirement ack or armed Drop. Normal/handled failure completes/disarms.
- No DB/file work before permit, detached mutator, forced barrier timeout, or caller-optional cleanup.
- Dependency: `109.018-T`.

### 7. `109.020-T` — RED: transferred-mask lifecycle driver

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): full `0b111` executes once under one successor; request on either side of release yields one executor; dropping/cancelling a transferred successor republishes full mask, clears owner, and lets one later requester take it without a second drain.
- No bounded loop/yield, timing assertion, or optional cleanup call.
- Dependency: `109.019-T`.

### 8. `109.021-T` — GREEN: single completion/RAII handoff driver

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four functions replace split takes, re-arm, `drain_pending_sync`, and bounded drain-to-completion with one transferred-mask driver.
- Explicit success consumes/disarms; cancellation/panic relies on Drop. The next current request is the only recovery successor. No recursive/double drain.
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
- Scenarios (4 maximum): a startup-before/after-release fixture; Startup active same/distinct rebind; legacy/v2 Watcher × same/distinct active-rebind matrix; stale watcher terminal matrix.
- Active rows prove cancellation observation, immutable snapshots, no successor acquisition before DB/file-capable exit ack, `max_active_db_drivers == 1`, one ack wake, and zero old work after ack. Both watcher loops are covered.
- Stale finish/Drop cannot clear or execute a replacement. Barriers/oneshots/counters only.
- Dependency: `109.023-T`.

### 12. `109.025-T` — GREEN: Startup and both Watcher cancellation migration

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- At most four functions cover Startup, the legacy watcher, the v2 watcher, and shared terminal handling.
- Every acquired Startup/Watcher permit wraps its full DB/file-capable execution and observes cancellation. Retirement suppresses later flush/drain phases, quiesces mutation-capable work, then acknowledges; ordinary success completes/disarms.
- Remove try-then-set and finish-then-drain. No detached mutator, forced barrier clear, overlap, or response/schema change.
- Dependency: `109.024-T`.

### 13. `109.026-T` — compatibility migration: contract tests

- Files: `tests/contract/read_test.rs`, `tests/contract/write_test.rs`; production files: zero.
- At most four scenarios: read rejection via public behavior; queued exact response; queued complete-mask observable behavior; idle control.
- Remove tokenless setup; add no public fixture API.
- Dependency: `109.025-T`.

### 14. `109.027-T` — compatibility migration: resilience tests

- Files: `src/server/state.rs`, `tests/integration/indexing_resilience_test.rs`; one production file with test-only private assertions.
- At most four scenarios: stale finish/Drop rejection; dropped-owner successor recovery; current completion disarm/timestamp once; release control.
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
- At most four scenarios: normal bind/Hydration/complete-disarm; queued Sync plus running Drop recovery/exact response; one deterministic `OwnerKind × same/distinct binding × explicit/Drop ack` active-rebind matrix with forced DB-failure rows; Startup/legacy-watcher/v2-watcher Windows named-pipe handoff smoke.
- The matrix proves cancellation delivery, same-binding `0b111` location, distinct-binding zero carryover, request coalescing during retirement, no acquisition before ack, exactly one ack publication/wake, `max_active_db_drivers == 1`, zero old work after ack, and stale-terminal no-op. Fixture rows stay one scenario.
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
- One parameterized relation/kind/terminal matrix proves: every `OwnerKind` receives cancellation; same-binding `0b101 OR 0b010 == 0b111` lives only in the barrier; distinct binding starts with zero old bits; current requests coalesce behind the barrier; no successor acquires before explicit/Drop ack; ack publishes/wakes exactly once; `max_active_db_drivers == 1`; old work count cannot increase after ack; later terminals are no-ops.
- A repeated-rebind fixture proves same-target preservation and distinct-target discard while retaining one retired identity.
- Ordinary current Drop clears owner/republishes once; one successor takes recovery once; ordinary completion disarms and timestamps once. Hydration distinguishes zero-permit pre-acquisition cancellation from acquired retirement acknowledgment.
- Static inventory confirms no legacy symbols, caller-optional cleanup, detached DB/file mutator, or direct tokenless setup.
- Process abort is restart reconciliation/intent reissue, never a Drop path. Windows smoke checks liveness only.

## Migration and atomicity

Intermediate branch commits may checkpoint execution, but none is mergeable/releasable/runtime-valid. The final PR must contain coordinator cell, cancellation-bearing RAII permits, the retirement barrier, every driver migration, and visibility cleanup. Tokenless completion, split-mask cache, second authority, caller-optional cancellation cleanup, or a driver that cannot prove quiescence stops the unit.

Binding transition is one logical publication. It advances binding/config/floor and installs a new cancellation channel under the fixed lock order. With an active owner it moves same-binding union or zero distinct-binding old bits into one `RetirementBarrier.deferred` slot, leaves the retired identity as the barrier key, then signals the old channel after unlock. It performs no coordinator wake and exposes no successor permit. Current requests coalesce behind the barrier. Only exact explicit terminal or Drop after DB/file-capable exit publishes deferred to ordinary pending, clears the barrier, and wakes once. A later rebind retargets the same barrier rather than bypassing it.

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
- same-binding barrier holds one `0b111` union, distinct binding carries zero old bits, requests coalesce in the barrier, and ack publishes/wakes exactly once;
- zero legacy caller and detached mutation-capable child inventory;
- exact queued JSON and MCP/CLI schema snapshots unchanged;
- no DB/file boundary before hydration permit; pending masks transfer whole and only once; no bounded-drain warning remains;
- Windows named-pipe bind, hydration, queued sync, cancellation, startup, and both watcher smoke paths succeed in a disposable workspace.

Operational closure records cancellation/ack traces, barrier duration, a manual monitoring checklist if no metric sink exists, a pre-deploy audit, a 15-minute Windows post-startup observation window owned by Ship, and full-release-unit rollback evidence. Roll back on missing cancellation delivery, acknowledgment before mutation-capable exit, successor-before-ack, active-driver count above one, work after ack, stuck barrier, duplicate/missing wake, wrong-generation execution, stale terminal mutation, mask loss, pre-permit I/O, timestamp violation, queued response change, or Windows IPC regression.

Rollback is the complete coordinator release-unit revert followed by daemon restart. Partial rollback after permit caller migration is forbidden. There is no data/schema rollback.

## Decisions and rationale

- Select A, not B: no supported published Rust contract; public permits would expand support burden.
- Select A, not C: private REDs and internal callers prove migration feasibility.
- Mutex, not packed atomic: full-width generations and readable transition diagnostics outweigh unproven lock-free benefit.
- Ordinary completion may return one transferred successor; retirement acknowledgment never does. It publishes pending and wakes one only after quiescence, so no successor overlaps the retired driver.
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
| Successor duplication/overlap | A RetirementBarrier blocks every request; only exact exit ack clears it and emits one wake. Activity counters prove no successor-before-ack and max one DB driver. |
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
6. more than two production files, four production functions, four scenarios, or 110 minutes in one task;
7. a wire, schema, persistence, config, or queued-response change;
8. a partial merge, deployment, or rollback of the caller migration;
9. source mutation before exact visibility/caller reinspection confirms `publish = false` and no new supported Rust contract;
10. failure to compile a RED before executing it;
11. caller-optional cleanup, async/spawn work in Drop, stale Drop touching replacement state, or any OwnerKind without a cancellation receiver and deterministic observation proof;
12. clearing/bypassing a retirement barrier before exact driver exit acknowledgment, or acknowledging while a DB/file/workspace mutation-capable child remains live;
13. a timeout that force-opens a stuck barrier or any successor-before-ack path;
14. an exactly-once or RAII recovery claim across process abort.

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
| Driver quiescence | Cancellation/ack/activity counters for all OwnerKind rows | Cancellation delivered; no successor before ack; max active DB drivers 1; no old work after ack; one wake | Missing observation/ack, overlap, post-ack work, duplicate wake, or stuck barrier | Ship |
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

Immediate rollback triggers are missing cancellation or acknowledgment, acknowledgment before DB/file-capable exit, successor-before-ack, more than one active DB driver, post-ack old work, stuck barrier, duplicate/missing wake, wrong-generation or cross-binding execution, stale mutation, orphan/split mask, pre-permit I/O, timestamp/queued-contract drift, or named-pipe regression. Rollback means revert the entire coordinator/caller/test release unit, restart the prior daemon, verify one healthy bind/status cycle, and record the outcome. No partial source rollback and no data/schema action.

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
- **Acknowledgment rule:** explicit terminal or armed Drop clears the barrier only after mutation-capable work exits, publishes deferred, and wakes exactly once. Stale terminals remain no-ops; a stuck driver keeps the barrier closed.
- **Driver proof:** fixture matrices cover Index, Sync, Hydration, Startup, legacy Watcher, and v2 Watcher under same/distinct rebind, explicit/Drop acknowledgment, repeated rebind, and forced failure without exceeding four scenarios per task.
- **Abort boundary:** no Drop claim on process abort. Restart reconciliation handles durable state; applicable non-durable flags are reissued; full-unit revert/restart is rollback.
- **ActionResult:** applied to plan, findings, and blocked task scopes; implementation remains unapproved until the existing post-106 requeue gate.

## Superseded Plan Reviews

- The pre-cycle-1 PASS at PR head `897406cc` was superseded by `discussion_r3700752674` and `discussion_r3700752695`.
- The unconditional cross-binding promotion review was superseded at head `436436587d7383bf4f97a2699b8aa473703d37df` by six binding-isolation comments.
- The binding-aware PASS recorded at base HEAD `d6321504137445a94b4134718355b87cceb75db6` was superseded by P1 `discussion_r3701136926` / `PRRT_kwDORJEduc6V2olJ` because it cleared owner before Index/Sync/Watcher exit.

## Plan Review

**Review-fix cycle:** 2/3
**Fresh plan-review revision:** 3 (quiescence-barrier remediation)
**Model routing verification:** `.github/agents/stage.agent.md` declares `.Stage`, Tier 3/high reasoning, provider `anthropic`, family `claude-opus-4.8`; no override was supplied.
**Execution surface:** no subagent invocation surface was exposed, so `.Stage` directly applied all always-on and triggered personas under the configured routing, as allowed by the skill fallback.
**Gate: PASS**
**Open P0: 0. Open P1: 0. Open P2: 0. Open P3: 0.**
**Plan hardening:** required, rerun, and satisfied.

### Gate rationale

The reported P1 is valid: current generation cancellation is hydration-only, and the prior plan would expose promoted work while retired Index/Sync/Watcher code could still use the same database. The revised state machine closes that gap. Active rebind advances binding/floor but changes `Running` to one `Retiring` barrier, signals the receiver held by every owner kind, and admits no successor. Same-binding `0b111` lives only in barrier deferred; distinct binding carries zero old bits. Current requests coalesce there. Only exact explicit terminal or armed Drop after DB/file-capable exit publishes deferred, clears the barrier, and performs one unconditional wake. Repeated rebind retargets the same barrier and stale terminals remain no-ops.

The driver-specific RED/GREEN pairs cover Hydration, Index/Sync, Startup, legacy Watcher, and v2 Watcher with relation/kind/terminal fixture matrices. They prove no successor-before-ack, max one DB driver, no old work after ack, and one publication/wake while retaining every existing time, file, function, scenario, status, and dependency cap. A non-quiescent driver fails closed behind the barrier; process abort remains restart reconciliation/full rollback.

### Persona verdicts

- **Constitution Reviewer — PASS.** Only the existing decision, blocked plan/backlog scopes, and existing handoff memory are changed. TDD order, `<=110 minutes`, `<=2` production files, `<5` production functions, `<=4` scenarios, and Stage role boundaries remain explicit.
- **Rust Reviewer — PASS.** `watch::Receiver` is already available and can be cloned into non-Clone permits. The `Idle | Running | Retiring` mutex state gives one authority; rebind/terminal races are lock-linearized; no standard mutex crosses await; Drop performs only synchronous exact-identity transitions.
- **Scope Boundary Auditor — PASS.** Quiescence is allocated to the existing state/lifecycle, hydration, write, IPC, and final-validation tasks. Matrices add rows, not scenarios; no Cargo, source, test, schema, config, wire, persistence, task-status, or dependency expansion occurs in Stage.
- **Learnings Researcher — PASS.** The barrier preserves whole-mask atomicity and removes producer reacquire/double drain while respecting newer-binding replacement and the prior take-before-lock/finish-site learnings. No second queue or companion authority is introduced.
- **Architecture Strategist — PASS.** `RetirementBarrier.deferred` is the exact sole union location while quiescing. Request coalescing, repeated-rebind retargeting, exact ack publication, unconditional single wake, and fail-closed stuck-driver behavior prevent inaccessible owners and old/new overlap.
- **Agent-Native Parity Reviewer — PASS.** Busy Index behavior, exact queued Sync JSON, MCP/CLI schemas/errors, health meaning, startup behavior, and persisted formats remain frozen. Internal `Queued` never authorizes execution.
- **Security Lens Reviewer — PASS.** Distinct-binding retirement transfers zero old-workspace bits and no old snapshot can authorize new-binding work. No credential, secret, trust-boundary, or sensitive logging expansion appears.

### Internal consistency checks

- `109.014/015` add the receiver and ordinary completion/Drop lifecycle within four scenarios/functions.
- `109.016/017` own four tuple/overflow/active-rebind/repeated-rebind matrices and the exact barrier/ack transition.
- `109.018/019` cover Hydration; `109.022/023` cover Index/Sync; `109.024/025` cover Startup and both watcher loops; `109.031` consolidates all-owner deterministic/runtime evidence.
- Same binding stores `0b111` only in deferred; distinct binding stores zero old bits; current requests coalesce; ack publishes/wakes once; stale terminals mutate zero state.
- Dependency chain remains `109.013 -> 109.014 -> ... -> 109.031`. `106-S`/`109.013-T` remain active; `104-S`/`109-F`/all old and replacement tasks remain blocked and unclaimed.

### Review decision

The cancellation-plus-acknowledgment plan has no open P0-P3 findings and passes only as blocked replacement scope. PASS does not authorize source/test/Cargo work, closing `106-S`/`109.013-T`, requeueing `104-S`/`109-F`, shipment claim/closure, Git/PR operations, replies, or thread resolution. Ship may later commit these uncommitted planning changes and post the suggested reply; the existing post-106 Stage transaction remains the sole requeue gate.
