---
title: "Post-105 single-authority sync coordinator permit migration"
type: impl-plan
doc_type: plan
date: 2026-08-02
status: blocked
source: "docs/decisions/2026-08-01-post-105-sync-coordinator-spike-findings.md"
feature: "109-F"
shipment: "104-S"
tags: ["concurrency", "pending-sync", "permit", "internal-api", "ready-after-106-closure"]
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
- API decision: opaque `GenerationToken`, `OwnerPermit`, `OwnerKind`, and `WorkMask` are `pub(crate)` at most, with private fields. The permit is non-`Clone`.
- Semver: no major bump or deprecation bridge. This is an internal source migration. A supported downstream Rust API discovered before source mutation is a hard stop requiring strategy B and a new decision; tokenless adapters are never accepted as a fallback.
- Repository tests: migrate direct owner setup to public tool behavior or co-located private tests. No public test-only seam.

## Requirements trace

| Requirement | Implementation action | Verification units |
|---|---|---|
| One authority | Replace owner flag, generation, mask, and lifecycle handoff with one coordinator | `109.014-T` -> `109.015-T` |
| Stale/mismatched no mutation | Sequence-qualified non-cloneable permit and exact transition checks | `109.014-T` / `109.015-T` |
| Coherent generation install | Publish binding, cancellation, and floor at one no-await point | `109.016-T` -> `109.017-T` |
| Hydration owns before I/O | Cancellation-aware notify/retry; no DB/file boundary before permit | `109.018-T` -> `109.019-T` |
| Full-mask single successor | Completion transfers one whole mask to one `Sync` permit | `109.020-T` -> `109.021-T` |
| Write migration | Index/sync use permits; queued JSON unchanged; no producer reacquire | `109.022-T` -> `109.023-T` |
| Startup/watcher arbitration | Typed permits and one request linearization | `109.024-T` -> `109.025-T` |
| Compatibility tests | Replace direct tokenless setup with behavior/private harnesses | `109.026-T`, `109.027-T` |
| Visibility reduction | Retire tokenless owner, split pending, and companion mutators | `109.028-T` -> `109.030-T` |
| Timestamp exactly once | Successful current-permit completion records once; stale records zero | `109.014-T` / `109.015-T` |
| Windows/runtime/release closure | Deterministic suite plus disposable Windows daemon validation and observation | `109.031-T` |

## Authoritative design

### State

```text
SyncCoordinator {
  floor: u64,
  next_sequence: u64,
  owner: Option<OwnerRecord>,
  pending: Option<WorkMask>
}

GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerPermit { generation, sequence, kind, work_mask }
```

`AppState` holds one private `std::sync::Mutex<SyncCoordinator>` and one private `tokio::sync::Notify`. `Notify` is a wakeup only. It stores neither work nor owner identity.

### Request transition

`request(token, mask, kind) -> Result<RequestOutcome, EngramError>` validates the token under the coordinator lock; `RequestOutcome` is `Acquired(permit) | Queued | Stale`. A current request with no owner allocates one sequence and permit. A busy producer with a non-empty sync mask merges one complete mask into the single pending slot and returns `Queued`. A busy `Index` or hydration request uses an empty coalesced mask, returns `Queued` without publishing work, and maps respectively to the existing in-progress error or notify/cancel wait. Sequence exhaustion returns an existing typed system error before mutation. Stale requests mutate nothing.

### Completion transition

`complete(permit) -> Result<CompletionOutcome, EngramError>` consumes the non-cloneable permit; `CompletionOutcome` is `Transferred(permit) | Released | Stale`. Wrong generation, sequence, or kind is stale and mutates nothing. Exact completion with pending work clears pending and creates exactly one new `OwnerKind::Sync` permit carrying the full mask for the completing driver. Exact completion without pending work releases and notifies waiters. A request that linearizes after release acquires directly, which closes startup/release as exactly one executor.

A valid completion updates `last_indexed_at` once before returning its successful outcome. Validation/transition occurs under the coordinator mutex; the guard is dropped before the non-awaiting timestamp lock is taken. A rejected completion updates zero times. No mutex guard crosses `.await`.

### Binding and hydration

The binding install validates checked generation/sequence capacity before mutation. Workspace, config, cancellation ownership, and floor are published coherently in documented lock order; the coordinator critical section contains no await. Hydration registers notification before the final request check and selects notification versus cancellation without holding a standard mutex. DB connect, query, hydration, file scan, and progress work begin only after `Acquired(Hydration)`.

### Compatibility boundary

Safe observers may remain stable. Request-only publication may exist only as crate-private delegation to `request`; it cannot stage companion-only state. Tokenless claim, completion, generation clear, producer reacquire, split takes, and companion-only setters are retired. The exact queued result remains:

```json
{"status":"queued","message":"Sync queued; will run after current indexing completes"}
```

## Protected invariants

1. The coordinator is the sole authority for generation floor, owner permit, full mask, and handoff.
2. Stale token or mismatched permit changes no owner, mask, floor, notify, or timestamp state.
3. Every pending mask is complete; companion-only state is unrepresentable.
4. Completion transfers the full mask to exactly one successor or releases.
5. Hydration performs zero DB/file I/O before ownership.
6. Startup/release selects exactly one executor.
7. `last_indexed_at` changes exactly once for successful current-permit completion and zero times for stale completion.
8. No standard mutex guard crosses await.
9. No second queue, double drain, split consumption, producer reacquire, sleeps, unsafe code, or public test seam.
10. No CLI/MCP/wire/schema/persistence/queued-response regression.

## Implementation units

Every unit is `<=110 minutes`, touches `<=2` production files, changes fewer than five production functions, and has `<=4` deterministic scenarios. RED harness tasks make release-mode behavior changes: zero. GREEN does not start until its direct RED predecessor compiles and fails only intended assertions. No task-level partial migration may be merged, released, or runtime-validated; rollback and release operate on the complete coordinator release unit.

### 1. `109.014-T` — RED: coordinator identity, mask, and timestamp

- Files: `src/server/state.rs` only.
- Posture: test-first, co-located private tests.
- Scenarios (4): stale token cannot mutate newer mask; mismatched/stale permit cannot clear newer owner; exact completion transfers `0b111` to exactly one successor; valid completion increments a private timestamp-write counter once while stale completion increments zero.
- Synchronization: direct private transitions, barriers, oneshots, or `Notify`; no sleeps.
- Exit: narrow library compile succeeds, exact tests fail intended assertions against current behavior.
- Dependency: `109.013-T` terminal, but task remains blocked until Stage requeue.

### 2. `109.015-T` — GREEN: authoritative coordinator core

- Files: `src/server/state.rs` only.
- Functions: at most four production functions covering initialization, request, completion, and timestamp observer/update.
- Implement opaque token/mask/kind/permit and one coordinator. Checked sequence overflow fails before mutation using an existing typed system error.
- Make all `109.014-T` tests pass. No async guard crossing, second queue, or tokenless compatibility bridge.
- Dependency: `109.014-T`.

### 3. `109.016-T` — RED: coherent binding and generation floor

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): observers see complete old or complete new binding/token/cancel/floor tuple; `u64::MAX` fails before mutation; old token after rebind is stale and cannot enqueue/acquire.
- No release-mode behavior change or public pause seam.
- Dependency: `109.015-T`.

### 4. `109.017-T` — GREEN: binding install and cancellation ownership

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Functions: at most four, including coherent install, retirement of separate begin-generation, and lifecycle binding call.
- Preserve documented async lock order; acquire coordinator only for final no-await publication.
- Make `109.016-T` green without changing `DispatchSnapshot`, wire, schema, or persistence.
- Dependency: `109.016-T`.

### 5. `109.018-T` — RED: hydration admission and exact exits

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): held owner prevents the private pre-DB signal; cancellation before acquisition exits without signal/mutation; acquired DB-connect failure completes only its exact permit and preserves transferred newer mask.
- Bind S3 to a private production admission helper/collaborator; no reference-only test transition.
- Dependency: `109.017-T`.

### 6. `109.019-T` — GREEN: cancellation-aware hydration permit

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Functions: at most four covering private notify/retry admission, hydration body, and exact-exit finalization.
- Register `Notified` before request retry; wait on notify/cancel with no mutex guard. No DB/file work before permit.
- All normal, cancelled, and DB-failure exits consume exactly one permit and honor transfer/release outcome.
- Dependency: `109.018-T`.

### 7. `109.020-T` — RED: transferred-mask lifecycle driver

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Scenarios (3): full `0b111` mask executes once under one successor; request at either side of release yields exactly one executor; cancellation/failure cannot launch a second drain or strand a transferred mask.
- No bounded-loop/yield or timing assertion.
- Dependency: `109.019-T`.

### 8. `109.021-T` — GREEN: single completion/handoff driver

- Files: `src/server/state.rs`, `src/tools/lifecycle.rs`.
- Functions: at most four. Replace split takes, re-arm, `drain_pending_sync`, and bounded `drain_pending_sync_to_completion` with one transferred-mask driver.
- The completing driver either executes its returned successor permit or releases; no recursive/double drain.
- Dependency: `109.020-T`.

### 9. `109.022-T` — RED: write ownership and queued contract

- Files: `src/server/state.rs`, `src/tools/write.rs`.
- Scenarios (3): full index uses one exact permit; busy sync returns exact queued JSON and queues full mask without reacquire; stale qualified snapshot performs no index/sync work.
- Preserve scan-progress outcome shape; no public test seam.
- Dependency: `109.021-T`.

### 10. `109.023-T` — GREEN: write caller migration

- Files: `src/server/state.rs`, `src/tools/write.rs`.
- Functions: at most four: index entry, sync entry, finalizer, and one private helper if needed.
- Migrate index/sync to permits. Delete producer reacquire and separate finish/drain calls. Handle transferred permit exactly once.
- Exact queued status/message and MCP/CLI errors remain unchanged.
- Dependency: `109.022-T`.

### 11. `109.024-T` — RED: startup and watcher arbitration

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- Scenarios (4): startup request just before release transfers once; startup just after release acquires once; each watcher path completes only its own permit; stale watcher token cannot execute or clear current owner.
- Co-located private harness; barriers/oneshots only.
- Dependency: `109.023-T`.

### 12. `109.025-T` — GREEN: startup and watcher permit migration

- Files: `src/server/state.rs`, `src/daemon/ipc_server.rs`.
- Functions: at most four covering the two watcher paths, startup request helper, and completion helper.
- Remove try-then-set and finish-then-drain. Preserve startup, debounce, flush, ingestion, backfill, and response behavior.
- Dependency: `109.024-T`.

### 13. `109.026-T` — compatibility migration: contract tests

- Files: `tests/contract/read_test.rs`, `tests/contract/write_test.rs`; production files: zero.
- Scenarios (4 maximum): read rejection while indexing through public tool behavior; queued sync exact response; queued complete-mask semantics through observable behavior; normal idle control.
- Remove direct tokenless owner/take setup. Do not add a public fixture API.
- Dependency: `109.025-T`.

### 14. `109.027-T` — compatibility migration: resilience tests

- Files: `src/server/state.rs`, `tests/integration/indexing_resilience_test.rs`.
- Production files: one, test-only co-located assertions in `state.rs`; release behavior changes: zero.
- Scenarios (4 maximum): stale completion rejection, exactly-once transfer, current completion timestamp once, normal release control.
- Move private authority assertions co-located; retain integration assertions only for public tool/runtime behavior.
- Dependency: `109.026-T`.

### 15. `109.028-T` — retire tokenless ownership mutators

- Files: `src/server/state.rs` only.
- Functions (4 maximum): retire/reduce `try_start_indexing`, `finish_indexing`, `publish_pending_sync_and_try_reacquire`, and `clear_pending_sync_for_generation`.
- Structural proof: zero production/test call sites; no public or test-only permit export.
- Dependency: `109.027-T`.

### 16. `109.029-T` — retire split pending API

- Files: `src/server/state.rs` only.
- Functions (4 maximum): retire/reduce `set_pending_sync`, `publish_pending_sync`, `take_pending_sync`, and `has_pending_sync` where no longer required as safe observers.
- Structural proof: no split consumption or request-only wrapper can become a second authority.
- Dependency: `109.028-T`.

### 17. `109.030-T` — retire companion-only API and final inventory

- Files: `src/server/state.rs` only.
- Functions (4): retire `set/take_pending_sync_revalidate` and `set/take_pending_sync_backfill_python`.
- Verify zero tokenless owner, generation-clear, split-take, companion-only, producer-reacquire, bounded-drain, and double-drain callers across `src/` and `tests/`.
- Dependency: `109.029-T`.

### 18. `109.031-T` — Windows/runtime/release validation

- Production files: zero. Closure/checklist docs only if required.
- Scenarios (4 maximum): normal bind/hydration/complete; queued sync with full mask and exact response; rebind cancellation plus forced DB failure; startup/watcher handoff on Windows named pipe runtime.
- Run deterministic unit/contract/integration coverage first, then a disposable Windows workspace smoke. Runtime timing is observational only, never the race proof. No operator workspace.
- Capture structured logs, owner sequence balance, timestamp evidence, queued response, and rollback readiness.
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

- Co-located private harnesses access private coordinator and admission seams.
- Every RED first passes narrow `cargo test --lib --no-run <target>` compilation, then fails only its named assertion. Missing symbols or visibility failures are not RED.
- Ordering uses direct transition steps, `Barrier`, oneshot, or `Notify`; no sleeps, `yield_now` correctness, permission races, live-daemon race proof, or wall-clock assertions.
- GREEN runs the exact RED first, then affected contract/integration targets, then the full release-unit suite after visibility cleanup.
- Static inventory confirms no forbidden legacy symbols/callers.
- Windows runtime validation is a final smoke over a disposable workspace and cannot replace deterministic tests.

## Migration and atomicity

The source migration may use intermediate branch commits for executor checkpointing, but no intermediate commit is mergeable, releasable, or runtime-valid. There is no compatibility deployment window and no partial rollout. The final PR must contain every caller migration and visibility cleanup. Any need to retain tokenless completion, a split-mask cache, or a second active authority stops the release unit.

No storage, schema, config, or wire migration exists. The only compatibility work is repository-test migration and internal visibility reduction. The exact CLI/MCP queued response and current public tool schemas are frozen.

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
- every acquired owner sequence has exactly one valid completion in fixture logs;
- pending masks transfer whole and only once;
- no bounded-drain warning path remains;
- Windows named-pipe daemon bind, hydration, queued sync, cancellation, startup, and watcher smoke succeeds in a disposable workspace.

Operational closure records a manual monitoring checklist if no metric sink exists, a pre-deploy audit, a 15-minute Windows post-startup observation window owned by Ship, and full-release-unit rollback evidence. Roll back on wrong-generation execution, duplicate/missing executor, orphaned/full-mask loss, pre-permit I/O, stuck owner, timestamp count violation, queued response change, or Windows daemon/IPC liveness regression.

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
| Completion/timestamp cancellation gap | Non-awaiting timestamp update after coordinator transition and before successful return; exact counter test. |
| Hydration lost wake | Register `Notified` before request recheck; await only after guards drop. |
| Successor duplication | Completion alone creates transferred permit; producer never reacquires. |
| Mask loss or companion orphan | One `WorkMask` value, one pending slot, no split APIs. |
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
10. failure to compile a RED before executing it.

The source cutover is one release boundary. Width-safe tasks may checkpoint separately on the feature branch, but no checkpoint is eligible for PR merge or runtime use before `109.030-T` removes every legacy mutator and the aggregate source compiles. If the executor cannot stage the cutover without an unsafe compatibility bridge, it stops and returns the plan to Stage; it does not widen scope.

### Proposed actions

**ProposedAction PA-1**

- Summary: replace four independent synchronization authorities with one permit-bearing coordinator.
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
| Owner sequence balance | Structured coordinator acquire/complete outcome logs in disposable run | Every acquired sequence has exactly one valid completion; stale rejects may occur but mutate nothing | Any missing/duplicate valid completion or completion of the wrong sequence | Ship |
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
- zero forbidden legacy callers and zero public permit/test seams;
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


## Plan Review

**Review cycle:** 0 (no remediation cycle required)  
**Review model:** configured `.Stage` frontmatter — Anthropic `claude-opus-4.8`, Tier 3/high reasoning; no override  
**Cross-model status:** unavailable in this execution surface; every required persona was applied with the caller model  
**Gate: PASS**  
**Open P0: 0. Open P1: 0. Open P2: 0. Open P3: 0.**  
**Plan hardening:** required and satisfied.

### Gate rationale

The plan makes the compatibility decision rather than deferring it, represents all synchronization authority in one coordinator, and converts every unsafe owner/mask surface to an explicit migration or removal task. Five production changesets are preceded by compiling deterministic RED tasks. The four runtime modules are separated by responsibility with a strict acyclic chain; every unit is capped at two production files, fewer than five production functions, four scenarios, and 110 minutes. The release unit cannot be requeued before terminal 106 evidence and cannot be partially merged or rolled back.

### Persona verdicts

- **Constitution Reviewer — PASS (0 findings).** TDD ordering, Rust safety, width limits, role boundaries, and blocked shipment lifecycle are explicit. Stage performs no implementation, build, test, Git, or shipment claim/closure action.
- **Rust Reviewer — PASS (0 findings).** The API is fallible on sequence exhaustion, permits are opaque/non-cloneable, stale outcomes are non-mutating, timestamp writes are valid-only, and no standard mutex guard crosses await. External-test visibility is handled by test migration rather than a public seam.
- **Scope Boundary Auditor — PASS (0 findings).** State authority, lifecycle binding/hydration/handoff, write, IPC, test compatibility, and closure are isolated. No schema, persistence, config, dependency, operator-workspace, or unrelated backlog scope is admitted.
- **Learnings Researcher — PASS (0 findings).** Whole-mask atomic publication, ownership-before-consume, complete release-site coverage, external-test visibility, and review-circuit-breaker learnings are incorporated. The plan explicitly supersedes the old re-arm/double-drain repair rather than combining both models.
- **Architecture Strategist — PASS (0 findings).** Generation floor, sequenced owner, pending mask, and transfer are one authority. Completion alone creates the successor; `Notify` is not a queue; producer reacquire and split-mask caches are forbidden. The dependency graph is acyclic and the full release unit is the only rollout boundary.
- **Agent-Native Parity Reviewer — PASS (0 findings).** CLI and MCP schemas, errors, commands, health meaning, startup behavior, and exact queued JSON are frozen and covered by contract/runtime verification.
- **Security Lens Reviewer — PASS (0 findings).** Triggered conservatively by API/runtime surface changes. No auth, secret, trust-boundary, external-service, or sensitive-data expansion exists. Structured logs use generation/sequence/kind/mask only and must not include paths or workspace contents.

### Requirements and risk coverage

- Spike facts S1-S4 map to named deterministic units and final runtime checks.
- Compatibility/semver, full caller migration, rollback, release observability, Windows named-pipe validation, and no-public-seam constraints are explicit.
- Strict-safety `ProposedAction`, `ActionRisk`, approval, rollback, and `ActionResult` records are complete.
- Monitoring names signals, observations, baselines, thresholds, owner, pre-deploy audit, and a 15-minute observation window.
- The `ready_after_106_closure` marker is carried in supported body/tag/label surfaces while actual backlog and plan statuses remain blocked.
- Exact positive terminal evidence for 102-S and 103-S is preserved and must be re-read before requeue.

### Review decision

The plan is approved for harvest into **blocked** replacement tasks only. PASS does not authorize source work, closing `106-S`/`109.013-T`, requeueing `104-S`/`109-F`, claiming a shipment, or changing any unrelated artifact. Stage may create the replacement hierarchy, mark old tasks superseded, and record the post-106 transaction. Ship must first integrate the findings and close 106; a later Stage session performs the fail-closed requeue.
