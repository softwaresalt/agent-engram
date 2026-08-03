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

- Source basis: HEAD `feb5f7c84dc189dfebce840a7811aec3acfe4b53`; current source equals main after temporary spike edits were removed.
- Evidence basis: `docs/research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md`, reviewed evidence commit `b5d5802e`.
- Package contract: `Cargo.toml` has `publish = false`; Engram ships a binary. No public Rust permit API is added.
- API decision: opaque `GenerationToken`, `OwnerPermit`, `OwnerKind`, and `WorkMask` are `pub(crate)` at most, with private fields. The permit is non-`Clone`, owns an `Arc<CoordinatorCell>`, and has mandatory non-awaiting RAII abandonment cleanup.
- Semver: no major bump or deprecation bridge. This is an internal source migration. A supported downstream Rust API discovered before source mutation is a hard stop requiring strategy B and a new decision; tokenless adapters are never accepted as a fallback.
- Repository tests: migrate direct owner setup to public tool behavior or co-located private tests. No public test-only seam.

## Requirements trace

| Requirement | Implementation action | Verification units |
|---|---|---|
| One authority | Replace owner flag, generation, mask, timestamp, and handoff with one `Arc<CoordinatorCell>` | `109.014-T` -> `109.015-T` |
| Drop/cancellation safety | Armed RAII permit republishes authoritative owned mask OR pending, clears exact owner, and wakes one | `109.014-T` / `109.015-T` |
| Successful finish disarms cleanup | Exact completion disarms old permit before Drop and writes timestamp once | `109.014-T` / `109.015-T` |
| Atomic generation retirement | One lock transition retires old identity, promotes owner mask OR pending, and publishes binding/cancel/floor | `109.016-T` -> `109.017-T` |
| Stale finish/Drop isolation | Old permit after rebind/replacement returns stale/no-op and cannot mutate current generation | `109.016-T` / `109.017-T` |
| Hydration owns before I/O | Pre-acquisition cancel is zero-permit; post-acquisition cancel relies on RAII | `109.018-T` -> `109.019-T` |
| Full-mask single successor | Completion or abandonment exposes one whole mask to one successor | `109.020-T` -> `109.021-T` |
| Write migration | Index/sync use guarded permits; exact queued JSON; no producer reacquire | `109.022-T` -> `109.023-T` |
| Startup/watcher arbitration | Typed guarded permits and one request linearization | `109.024-T` -> `109.025-T` |
| Compatibility tests | Replace tokenless setup with behavior/private harnesses | `109.026-T`, `109.027-T` |
| Visibility reduction | Retire tokenless owner, split pending, and companion mutators | `109.028-T` -> `109.030-T` |
| Process-abort boundary | No Drop claim on abort; startup reconciliation, intent reissue, full rollback | `109.031-T` |
| Runtime/release closure | Deterministic suite plus disposable Windows daemon validation | `109.031-T` |

## Authoritative design

### State and RAII ownership

```text
CoordinatorCell { state: std::sync::Mutex<SyncCoordinator>, notify: Notify }
SyncCoordinator {
  floor, binding_identity, next_sequence,
  owner: Option<OwnerRecord>, pending: Option<WorkMask>, last_indexed_at
}
GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerIdentity { generation, sequence, kind }
OwnerPermit { cell: Arc<CoordinatorCell>, identity, work_mask, cleanup_armed }
```

`AppState` owns one private `Arc<CoordinatorCell>`. Permits clone the cell, remain non-`Clone`, and expose no public fields. `Notify` stores neither work nor identity. The coordinator owner record, not the permit copy, is authoritative for the owned mask.

### Request transition

`request(token, mask, kind)` returns fallible `Acquired(permit) | Queued | Stale` after validating under the coordinator lock.

- Busy current producers OR a complete mask into the one pending slot; empty-mask waiters publish nothing.
- Stale requests mutate nothing.
- With no owner, a non-empty request atomically takes `pending OR requested` into one `Sync` permit; concurrent requesters cannot take it twice.
- With no owner, empty-mask Index/Hydration may acquire while pending remains, then transfer pending on exact completion. This preserves hydration-before-DB ordering on a fresh binding.
- Sequence exhaustion fails before mutation. No Drop/rebind path preallocates an owner whose permit has no recipient.

### Explicit completion transition

`complete(mut permit)` consumes the permit and compares generation, sequence, and kind under the mutex.

- Exact completion with pending clears pending and installs one armed `Sync` successor returned as `Transferred`.
- Exact completion without pending releases and returns `Released`; one waiter is notified after unlock.
- Both exact outcomes write `last_indexed_at` once in the same current-permit transition and set `cleanup_armed = false` before old-permit destruction.
- Stale completion disarms only the local permit and returns `Stale`; coordinator state, notification, and timestamp stay unchanged.

No standard mutex crosses `.await`. Keeping timestamp in coordinator state removes the prior transition-to-timestamp cancellation gap.

### Mandatory Drop transition

`OwnerPermit::drop` is the cancellation guard. If armed, it locks synchronously with poison recovery. Only exact owner identity may act. Exact Drop computes `recovery = owner.work_mask OR pending`, clears owner, stores non-empty recovery in the one pending slot, unlocks, and calls `notify_one` once. It never allocates sequence, awaits, spawns, updates timestamp, or panics. Stale Drop is a strict no-op and cannot clear or republish over a replacement.

The next current request is the sole possible successor. A sync-capable request takes recovery once; an empty-mask owner may run first and transfers recovery on completion. Cancellation cannot strand ownership and no caller-optional cleanup exists.

### One atomic generation/binding advance

Prepare the new workspace/config/cancellation tuple and validate capacity before mutation. Acquire binding write guards in documented order, then coordinator mutex; do not await afterward. One coordinator-locked publication:

1. removes and invalidates prior owner identity;
2. computes `promoted = prior_owner.work_mask OR prior_pending` from authoritative state;
3. publishes non-empty `promoted` as the new generation's one pending mask;
4. swaps workspace, config, and cancellation ownership behind held guards;
5. advances binding identity and floor together and resets hydration readiness; and
6. synchronously signals old cancellation.

It deliberately leaves `owner = None`; a successor without a recipient would strand ownership. After all guards drop, invoke `notify_one` at most once if retirement made progress possible, then return the new opaque token. Every set bit is promoted to the newest binding. Empty masks do not invent work; obsolete non-coalescible old-binding operations are cancelled.

Old owners retain an immutable old binding snapshot/cancellation receiver and cannot reread or operate against the new binding. Later finish returns `Stale`; later Drop sees no match. A partially completed old-binding operation may be reconciled on the new binding, but cannot mutate or launch current-generation work. Exactness means one owner/pending location and one driver launch per generation/binding, not exactly-once external side effects.

### Hydration and process-abort boundary

Hydration registers `Notified` before final request check and selects notification versus cancellation without a standard mutex. Cancellation before acquisition exits with no permit/mutation. After acquisition, normal/handled failure explicitly completes and disarms; task cancellation or panic relies on Drop. DB/file work begins only after `Acquired(Hydration)`.

Rust Drop is not claimed for process abort. Restart reconstructs in-memory authority; bind/hydration and offline-change detection reconcile durable files. Non-durable revalidate/backfill intent must be reissued. Runtime invariant failure uses full release-unit revert and daemon restart.

### Compatibility boundary

Safe observers may remain stable. Request-only publication may only be crate-private delegation to `request`; it cannot stage companion-only state. Tokenless claim/completion/generation-clear/producer-reacquire/split-take/companion setters retire. The exact queued result remains:

```json
{"status":"queued","message":"Sync queued; will run after current indexing completes"}
```

## Protected invariants

1. The coordinator cell is sole authority for binding floor, owner identity, full mask, handoff, and completion timestamp.
2. In-process, each `WorkMask` bit exists in exactly one location: owner or pending.
3. Rebind atomically retires old identity and promotes `owner mask OR pending` to the new binding.
4. Exact Drop republishes the same union, clears owner once, and wakes at most one; stale Drop changes nothing.
5. Exact completion disarms Drop and updates `last_indexed_at` once; stale completion/Drop update zero times.
6. No transition installs owner without returning its permit.
7. Hydration does zero DB/file I/O before ownership; pre-acquisition cancel has no permit to complete.
8. Old owners use immutable old binding snapshots and cannot mutate/launch new-generation work.
9. No mutex crosses await; Drop never awaits, spawns, or panics.
10. No second queue, double drain, split consumption, producer reacquire, sleeps, unsafe, or public test seam.
11. No CLI/MCP/wire/schema/persistence/config/queued-response regression.
12. No exactly-once or RAII claim crosses process abort; reconciliation, reissue, and full rollback are explicit.

## Implementation units

Every unit is `<=110 minutes`, touches `<=2` production files, changes fewer than five production functions, and has `<=4` deterministic scenarios. RED tasks change zero release behavior. GREEN starts only after its direct RED compiles and fails intended assertions. No partial migration is mergeable/releasable.

### 1. `109.014-T` — RED: permit completion/Drop lifecycle

- Files: `src/server/state.rs` only.
- Scenarios (4): current Drop republishes authoritative owned mask OR pending and clears owner once; one later requester takes that union once; exact completion disarms Drop and does not republish on destruction; exact transfer/release writes timestamp once while stale explicit completion writes zero.
- Assert one notification for exact abandonment/release and zero for stale Drop with a private deterministic collaborator.
- Dependency: `109.013-T` terminal, while status remains blocked until the existing Stage requeue gate.

### 2. `109.015-T` — GREEN: authoritative RAII coordinator core

- Files: `src/server/state.rs` only.
- At most four production functions cover cell initialization/request, explicit completion, Drop abandonment, and observer/update behavior.
- Implement authoritative owner mask, completion disarm, synchronous poison-safe Drop, and timestamp-in-transition. No caller-optional cleanup, async Drop work, second queue, or tokenless bridge.
- Dependency: `109.014-T`.

### 3. `109.016-T` — RED: coherent binding retirement and stale isolation

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (4): complete old/new binding-token-cancel-floor tuple; `u64::MAX` no mutation; advance with active owner `0b101` plus pending `0b010` retires old identity and exposes one new-generation `0b111` pending mask with at most one wake; after successor acquisition a two-fixture matrix proves stale old explicit finish and independently stale old Drop leave successor/mask/floor/timestamp unchanged.
- Dependency: `109.015-T`.

### 4. `109.017-T` — GREEN: atomic binding advance and owner retirement

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four functions cover prepared install, one coordinator publication, retirement/promotion, and lifecycle call.
- Acquire async binding guards before coordinator; no await under standard mutex. Signal old cancellation synchronously, return token after publication, notify at most one after unlock.
- Preserve `DispatchSnapshot`, wire, schema, config format, and persistence.
- Dependency: `109.016-T`.

### 5. `109.018-T` — RED: hydration admission and exact exit classes

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): held owner prevents pre-DB signal; pre-acquisition cancel exits with no permit/completion/Drop mutation/signal; acquired DB-connect failure explicitly completes its exact permit, disarms cleanup, and preserves transferred newer mask.
- Bind S3 to a private production collaborator, not a reference-only transition.
- Dependency: `109.017-T`.

### 6. `109.019-T` — GREEN: cancellation-aware hydration guard

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- At most four functions cover notify/retry admission, hydration body, and exact-exit finalization.
- Register `Notified` before recheck. Pre-acquisition cancellation exits without completion. After acquisition, normal/handled failure explicitly complete/disarm; cancellation/panic is recovered only by mandatory Drop.
- No DB/file work before permit and no cleanup convention left to callers.
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

### 9. `109.022-T` — RED: write ownership and queued contract

- Files: `src/server/state.rs`, `src/tools/write.rs`.
- Scenarios (3): full index exact guarded permit; busy sync exact queued JSON/full mask/no reacquire; stale qualified snapshot no execution.
- Preserve scan-progress shape; no public seam.
- Dependency: `109.021-T`.

### 10. `109.023-T` — GREEN: write caller migration

- Files: `src/server/state.rs`, `src/tools/write.rs`.
- At most four functions migrate index/sync/finalization to guarded permits. Explicit returns disarm; cancellation/panic cannot strand ownership.
- Delete producer reacquire and separate finish/drain. Preserve exact queued JSON and MCP/CLI errors.
- Dependency: `109.022-T`.

### 11. `109.024-T` — RED: startup and watcher arbitration

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- Scenarios (4): startup just before release transfers once; startup just after release acquires once; each watcher completes/disarms its permit; stale watcher finish/Drop cannot execute or clear current owner.
- Barriers/oneshots only.
- Dependency: `109.023-T`.

### 12. `109.025-T` — GREEN: startup and watcher guarded-permit migration

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- At most four functions cover watcher paths, startup request, and completion. Remove try-then-set and finish-then-drain. RAII protects every permit until explicit disarm.
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

### 18. `109.031-T` — Windows/runtime/release validation

- Production files: zero; closure/checklist docs only if needed.
- At most four scenarios: normal bind/hydration/complete-disarm; queued sync plus dropped-owner full-mask recovery/exact response; rebind with active owner/pending plus stale finish/Drop and forced DB failure; startup/watcher Windows named-pipe handoff.
- Deterministic fixtures prove races. Smoke is observational. Verify restart reconciliation and record non-durable intent reissue; never claim Drop on process abort.
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

- Private co-located harnesses access coordinator, Drop, notification, and admission seams; no public seam.
- Every RED first passes narrow `cargo test --lib --no-run <target>`, then fails only its named assertion. Missing symbols/visibility are not RED.
- Use direct transitions, `Barrier`, oneshot, or `Notify`, with private notification counters. No sleeps, yields, permission races, live-daemon race proof, or wall-clock assertions.
- Required proofs: `0b101 OR 0b010 == 0b111` promotion during active-owner rebind; stale old finish and Drop leave replacement byte-for-byte unchanged; current Drop clears owner/republishes once; one successor takes recovery once; explicit completion disarms Drop; timestamp writes once only for exact explicit completion.
- Hydration distinguishes pre-acquisition cancellation (zero permit/mutation) from post-acquisition RAII cleanup. Handled DB failure explicitly completes/disarms.
- Static inventory confirms no legacy symbols, caller-optional cleanup, or direct tokenless setup.
- Process abort is restart reconciliation/intent reissue, never a Drop path. Windows smoke checks liveness only.

## Migration and atomicity

Intermediate branch commits may checkpoint execution, but none is mergeable/releasable/runtime-valid. The final PR must contain coordinator cell, RAII Drop, binding retirement, every caller migration, and visibility cleanup. Tokenless completion, split-mask cache, second authority, or caller-optional cancellation cleanup stops the unit.

Binding transition is one logical publication: prepare/check first; acquire async binding guards in fixed order; then one no-await coordinator transition retires identity, unions/promotes masks, publishes binding/cancel/floor, and records whether one post-unlock wake is needed. Old and returned-new token are never current together. Stale finish/Drop cannot mutate replacement.

No storage, schema, config-format, or wire migration exists. Process death is outside Rust Drop: restart hydration/offline reconciliation covers durable files, non-durable companion intent is reissued, and invariant failure rolls back the complete unit then restarts. No exactly-once side-effect claim crosses process death or binding identity.

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

- deterministic S1-S4 and added timestamp/generation/cancellation controls pass;
- zero legacy caller inventory;
- exact queued JSON and MCP/CLI schema snapshots unchanged;
- no DB/file boundary before hydration permit;
- every acquired owner sequence has exactly one exact terminal transition in fixture logs: explicit completion or RAII abandonment;
- pending masks transfer whole and only once;
- no bounded-drain warning path remains;
- Windows named-pipe daemon bind, hydration, queued sync, cancellation, startup, and watcher smoke succeeds in a disposable workspace.

Operational closure records a manual monitoring checklist if no metric sink exists, a pre-deploy audit, a 15-minute Windows post-startup observation window owned by Ship, and full-release-unit rollback evidence. Roll back on wrong-generation execution, stale finish/Drop mutation, duplicate/missing executor, orphaned/full-mask loss, pre-permit I/O, stuck owner, timestamp count violation, queued response change, or Windows daemon/IPC liveness regression.

Rollback is the complete coordinator release-unit revert followed by daemon restart. Partial rollback after permit caller migration is forbidden. There is no data/schema rollback.

## Decisions and rationale

- Select A, not B: no supported published Rust contract; public permits would expand support burden.
- Select A, not C: private REDs and internal callers prove migration feasibility.
- Mutex, not packed atomic: full-width generations and readable transition diagnostics outweigh unproven lock-free benefit.
- Completing driver receives pending mask: guarantees one successor without a second queue or producer reacquire.
- Normalize transferred work to `OwnerKind::Sync`: queued public behavior is coalesced sync, not producer-affine execution.
- Private tests over public seams: protects API containment and deterministic scheduling.
- Full release-unit rollback: caller signatures and ownership semantics cannot be partially reverted safely.

## Risks and caveats

| Risk | Mitigation / stop condition |
|---|---|
| Hidden downstream Rust consumer | `publish = false` evidence; stop before source mutation if a supported contract is produced. |
| Partial migration exposes tokenless completion | No partial merge/release; final zero-caller inventory is gate-blocking. |
| Completion/Drop/timestamp gap | Timestamp is written in the exact coordinator transition; old permit is disarmed before destruction; deterministic completion-versus-Drop counter proof. |
| Hydration lost wake/cancellation | Register before recheck; pre-acquisition cancel is zero-permit; post-acquisition cancel is mandatory RAII Drop. |
| Successor duplication | Completion returns one successor; Drop/rebind never preallocate inaccessible owner and notify at most one; no producer reacquire. |
| Mask loss or companion orphan | Rebind/Drop use authoritative `owner mask OR pending` in one lock; one pending slot, no split APIs. |
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
11. caller-optional cleanup, async/spawn work in Drop, or stale Drop touching replacement owner;
12. an exactly-once or RAII recovery claim across process abort.

The source cutover is one release boundary. Width-safe tasks may checkpoint separately on the feature branch, but no checkpoint is eligible for PR merge or runtime use before `109.030-T` removes every legacy mutator and the aggregate source compiles. If the executor cannot stage the cutover without an unsafe compatibility bridge, it stops and returns the plan to Stage; it does not widen scope.

### Proposed actions

**ProposedAction PA-1**

- Summary: replace four independent synchronization authorities with one `Arc`-owned permit coordinator whose exact completion disarms mandatory RAII cleanup.
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
| Owner sequence balance | Structured acquire/complete/abandon logs | Every acquired sequence has one exact completion or abandonment; stale finish/Drop mutate nothing | Missing/duplicate terminal transition, wrong sequence, or stale mutation | Ship |
| Full-mask handoff | Deterministic counters plus transfer log fields (`generation`, `sequence`, `mask`) | One `0b111` transfer to one successor; pending empty at transfer | Orphan companion, split mask, duplicate successor, or pending retained after transfer | Ship |
| Hydration admission | Private pre-DB fixture counter and Windows hydration logs | Counter/log boundary remains zero before permit | Any DB/file boundary before `Acquired(Hydration)` | Ship |
| Startup/release execution | S4 counters and startup structured logs | Exactly one executor | Zero or more than one executor | Ship |
| Timestamp completion | Private timestamp-write counter and doctor health observation | One write per valid current completion, zero per stale/mismatch | Any missing, duplicate, or stale-triggered write | Ship |
| Queued contract | Contract snapshot of status/message and MCP schema | Byte-equivalent status/message and unchanged schema | Any response/schema/error mapping change | Ship |
| Windows runtime liveness | `daemon-status`, `workspace-status`, named-pipe command completion, logs | One daemon, responsive IPC, no stuck owner/drain warning | IPC timeout/hang, stuck owner, drain-bound warning, duplicate daemon, or uncompleted queued work | Ship |

### Pre-deploy audit

Before merge, Ship records:

- `publish = false` and no supported Rust-library contract introduced;
- all 18 replacement tasks complete and old tasks remain superseded;
- zero forbidden legacy callers, zero public permit/test seams, and zero caller-optional cleanup paths;
- no Cargo dependency, feature, schema, persistence, wire, or config delta;
- exact queued response/schema snapshots unchanged;
- deterministic suite and Windows disposable-runtime evidence attached;
- rollback commit/range and prior daemon restart procedure identified;
- `102-S`/`103-S` predecessor evidence remains intact and unrelated work untouched.

### Observation window and rollback

Ship owns an active 15-minute post-startup observation window on Windows after all deterministic gates pass. Exercise one normal bind/hydration, one queued sync, one rebind cancellation/forced DB-failure fixture, and one startup/watcher handoff. The live smoke observes liveness and logs only; deterministic fixtures remain the race proof.

Immediate rollback triggers are any wrong-generation execution, stale completion mutation, duplicate/missing executor, orphan/split mask, pre-permit DB/file I/O, stuck owner after cancel/failure, invalid timestamp count, queued contract drift, drain-bound warning, or named-pipe liveness regression. Rollback means revert the entire coordinator/caller/test release unit, restart the prior daemon, verify one healthy bind/status cycle, and record the outcome. No partial source rollback and no data/schema action.

### Operator and agent checkpoints

- Stage checkpoint now: docs/backlog only; no source/build/test/Git/shipment lifecycle action.
- Ship checkpoint 1: integrate findings and plan, verify source clean, then close only `106-S`/`109.013-T` after merge evidence exists.
- Stage checkpoint 2: execute the exact requeue transaction only after terminal 106 and positive 102/103 reads.
- Ship checkpoint 3: claim `104-S` only after Stage queues it; execute the coordinator release unit.
- Any unavailable index, ambiguous caller visibility, or non-terminal predecessor fails closed.


### PR #316 P1 hardening delta

- **Trigger:** owner lifecycle spans completion, cancellation/panic, generation replacement, and restart. `ActionRisk: high`.
- **Guardrail:** `OwnerPermit` holds `Arc<CoordinatorCell>` and armed cleanup; exact Drop is synchronous, poison-safe, non-panicking, and notifies only after unlock.
- **Atomicity:** generation install prepares/checks first, then one coordinator transition retires identity and publishes `old owner mask OR old pending` under new binding/floor. It never creates a successor without a recipient.
- **Verification:** four core and four binding scenarios cover Drop republish, one-successor take, completion disarm/timestamp once, active-owner rebind union, and stale finish/Drop isolation.
- **Abort boundary:** no Drop claim on process abort. Restart reconciliation handles durable state; non-durable flags are reissued; full-unit revert/restart is rollback.
- **ActionResult:** applied to plan and blocked task scopes; implementation remains unapproved until the existing post-106 requeue gate.

## Superseded Plan Review (pre-PR-P1 remediation)

The earlier PASS at PR head `897406cc` is superseded because `discussion_r3700752674` and `discussion_r3700752695` exposed two valid P1 lifecycle gaps.

## Plan Review

**Review cycle:** 1 (fresh review after accepted PR #316 P1 remediation)
**Model routing verification:** `.github/agents/stage.agent.md` declares `.Stage`, Tier 3/high reasoning, provider `anthropic`, family `claude-opus-4.8`; no model override was supplied.
**Cross-model status:** no subagent execution surface was available, so all required personas were applied in the caller review as permitted by the skill.
**Gate: PASS**
**Open P0: 0. Open P1: 0. Open P2: 0. Open P3: 0.**
**Plan hardening:** required, rerun, and satisfied.

### Gate rationale

The two P1s are closed in design rather than delegated. Generation/binding advance has one no-await coordinator linearization point that retires prior identity, moves authoritative `owner WorkMask OR pending WorkMask` to the new generation, publishes binding/cancellation/floor coherently, and schedules at most one post-unlock wake without creating an inaccessible owner. Stale old finish returns `Stale`; stale Drop is identity-mismatched and cannot mutate replacement.

Ownership is RAII. Exact completion disarms before destruction and writes timestamp once in the current-permit transition. Cancellation/panic Drop republishes authoritative union, clears exact owner once, and wakes one without await/spawn/panic. Pre-acquisition cancellation is a zero-permit path. Process abort is not misrepresented: startup reconciliation, non-durable intent reissue, and full-unit rollback/restart are named.

### Persona verdicts

- **Constitution Reviewer — PASS.** Stage changes only planning/docs/blocked backlog; TDD ordering, task width, role boundaries, and blocked shipment lifecycle remain explicit.
- **Rust Reviewer — PASS.** `Arc<CoordinatorCell>` makes RAII feasible from `&AppState`; Drop uses a recoverable synchronous mutex and synchronous `notify_one`; no async destructor or mutex across await. Private exact identity prevents stale cleanup touching replacement.
- **Scope Boundary Auditor — PASS.** No public API, wire/schema/config/persistence/Cargo change and no task over two files, four scenarios, or 110 minutes. Dependencies stay acyclic.
- **Learnings Researcher — PASS.** Whole-mask atomicity, complete release-site coverage, no tokenless bridge, and private harness guidance remain; abandonment is an authoritative release site, not a second drain.
- **Architecture Strategist — PASS.** Owner/pending union has one source/destination under one lock. Completion may return one successor; generation/Drop leave owner empty and wake at most one, preventing inaccessible permits and duplicate claims.
- **Agent-Native Parity Reviewer — PASS.** Exact queued JSON, CLI/MCP schemas/errors, health meaning, and startup behavior are frozen and verified deterministically plus at runtime.
- **Security Lens Reviewer — PASS.** No trust-boundary or sensitive-data expansion. Logs are limited to generation/sequence/kind/mask outcomes and exclude workspace contents.

### Internal consistency checks

- `109.014/015` own four completion/Drop scenarios; `109.016/017` own four generation-retirement scenarios.
- `109.018/019` distinguish pre-acquisition cancellation from post-acquisition RAII cleanup.
- `109.020/021` prove a dropped transferred permit cannot strand work or start a second drain.
- `109.031` treats process abort as restart reconciliation/reissue, not Drop.
- Dependency chain remains `109.013 -> 109.014 -> ... -> 109.031`; statuses remain blocked.

### Review decision

The remediated plan has zero open P0/P1 and is approved only as blocked replacement scope. PASS does not authorize source/test/Cargo work, closing `106-S`/`109.013-T`, requeueing `104-S`/`109-F`, shipment claim/closure, Git/PR operations, or thread resolution. Ship must commit/push these planning changes and reply to both threads; the existing post-106 Stage transaction remains the sole requeue gate.
