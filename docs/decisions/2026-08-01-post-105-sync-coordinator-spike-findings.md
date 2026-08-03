---
title: "Post-105 single-authority sync coordinator spike findings"
type: decision
doc_type: decision
source: "106-S / 109.013-T"
date: 2026-08-02
status: decided
disposition: PIVOT
feature: "109-F"
shipment: "104-S"
evidence_commit: "b5d5802e930bb0f88b79dbe03f910e326ea7f604"
tags: ["concurrency", "pending-sync", "compatibility", "single-authority", "pivot", "raii", "cancellation-safety"]
---

## Executive disposition

**PIVOT.** The spike proved the four current defects with deterministic compile-then-fail REDs, but it disproved the prior signature-preservation premise. Option A remains viable only as an **opaque, identity-bearing, crate-private permit API with visibility reduction and full internal caller migration**. This is selected strategy **A**. Strategy B is unnecessary because Engram is distributed as a binary and `Cargo.toml` sets `publish = false`; strategy C is unnecessary because every critical production caller found at the examined HEAD is internal and can migrate in one release unit.

This decision does not close `106-S` or `109.013-T`, does not requeue `104-S` or `109-F`, and does not authorize implementation. A fresh hardened plan must remain blocked under the marker `ready_after_106_closure` until Ship integrates these findings and closes `106-S`.

## Evidence and citations

The controlling execution record is [106-S single-authority sync coordinator spike execution evidence](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md), reviewed at commit `b5d5802e930bb0f88b79dbe03f910e326ea7f604`.

- [Current authority inventory](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#current-authority-inventory) identifies four independent authorities: generation, owner flag, pending mask, and lifecycle-local admission/drain state.
- [S1](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#s1-stale-wrapper-below-advanced-floor) compiled and failed because stale generation 1 changed the generation-2 mask from `(2,5)` to `(2,7)`.
- [S2](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#s2-full-mask-exactly-once-handoff) compiled and failed because completion produced `(owner=false,pending=7)` instead of exactly one successor owning the full mask.
- [S3](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#s3-hydration-zero-work-before-ownership) compiled and failed because hydration reached the private pre-DB boundary without ownership.
- [S4](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#s4-startup-releaserequest-arbitration) compiled and failed because startup/release selected zero executors instead of exactly one.
- [Public compatibility](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#public-compatibility-table) proves that tokenless `finish_indexing(&self)` cannot reject a stale completion and split boolean takes cannot preserve owner identity or whole-mask ownership.
- [Production caller inventory](../research/2026-08-02-106-sync-coordinator-spike-execution-evidence.md#production-caller-inventory) and a Phase 5C reinspection at HEAD `feb5f7c84dc189dfebce840a7811aec3acfe4b53` agree on the four-file migration surface.

## Facts and assumptions

### Facts

| ID | Fact | Basis |
|---|---|---|
| F1 | All four narrow RED harnesses compiled before execution and failed only their intended assertions. | Spike evidence compile and failure tables. |
| F2 | Current generation, owner flag, pending mask, and lifecycle admission/drain are separate authorities. | `state.rs` and `lifecycle.rs`; spike inventory. |
| F3 | `finish_indexing(&self)` carries no owner identity; a stale second completion can clear a newer owner. | Signature and S2 analysis. |
| F4 | Split `take_pending_sync*() -> bool` calls cannot transfer one complete mask to one owner. | Current API and S2. |
| F5 | Hydration may call `connect_db` after `try_start_indexing()` returned false. | `src/tools/lifecycle.rs:250-305`; S3. |
| F6 | Startup is a try-then-set sequence and can select neither executor. | `src/daemon/ipc_server.rs:1210-1217`; S4. |
| F7 | Critical production callers are confined to `state.rs`, `write.rs`, `lifecycle.rs`, `ipc_server.rs`, with the stable timestamp observer in `doctor.rs`. | Phase 5C targeted caller reinspection. |
| F8 | Direct non-production callers are repository contract/integration tests; no standalone companion setter/take is called by an external test. | Phase 5C targeted caller reinspection and spike inventory. |
| F9 | The Rust package is not published (`publish = false`) and the product contract is the released daemon/CLI/MCP binary. | `Cargo.toml` and repository operating model. |
| F10 | `102-S` and `103-S` retain positive terminal evidence: archived at commits `89ce54193ad8c1340e5b8b440f9190a276b72196` and `5c9d466ebff883ae8ae6e71008968f986707e882`. | Exact backlog shipment reads on 2026-08-02. |
| F11 | `106-S` and `109.013-T` remain active; `104-S`, `109-F`, and `109.001-T` through `109.012-T` remain blocked. | Exact backlog reads on 2026-08-02. |
| F12 | Exact no-pending `Running` completion selected `Released` without notifying empty Hydration/Startup/Watcher waiters queued behind the owner. | Copilot P1 `discussion_r3701238147` / `PRRT_kwDORJEduc6V25-i` at HEAD `2f267d9c617243dd70cbaac9837826a4fd0358e9`. |

### Assumptions and fail-closed treatment

| ID | Assumption | Confidence and treatment |
|---|---|---|
| A1 | There is no supported downstream Rust-library consumer outside this repository. | High: `publish = false`, binary distribution, and no repository contract naming a supported Rust API. If a supported downstream contract is produced before implementation, stop before source mutation and replace this decision with strategy B; do not silently retain tokenless methods. |
| A2 | The Phase 5C caller inventory is exhaustive for the examined source. | High: targeted recursive inspection covered all `.rs` files under `src/` and `tests/`. Ship repeats the zero-legacy-caller inventory before visibility reduction. |
| A3 | One coalesced successor may execute queued routine/revalidate/backfill work regardless of which producer requested it. | High: current queued CLI/MCP semantics promise coalescing, not producer-specific execution. The successor permit is normalized to `OwnerKind::Sync`. |
| A4 | No schema or persisted-data migration is required. | High: the authority is in-memory process state. Any discovered persistence change is a stop condition and requires a new hardened plan. |
| A5 | Rust `Drop` runs for task cancellation and unwind, but not process abort. | Certain language/runtime boundary. In-process cleanup is RAII; process death is recovered by restart/bind hydration and offline-change reconciliation, while non-durable revalidate/backfill intent must be reissued. No exactly-once claim crosses process death. |

The decision is not open: the current facts select strategy A. The fail-closed clauses address changed evidence, not unresolved current ambiguity.

## Caller and visibility decision

### Production migration inventory

| Surface | Current callers | Decision |
|---|---|---|
| Observe indexing | `write.rs:32` | Keep a read-only observer, reduce to crate-private unless a stable public behavior requires it. |
| Claim/complete index or sync | `write.rs:160,268,470` | Migrate to cancellation-bearing RAII permits around the complete DB/file-capable future; ordinary completion disarms, active-rebind exit acknowledges only after quiescence, and task unwind uses Drop. |
| Queue/reacquire | `write.rs:297-304` | Replace with one request; preserve exact queued JSON; remove producer reacquire. |
| Generation install | `lifecycle.rs:187` | Install coherent binding/cancellation/floor plus one RetirementBarrier when active; signal old cancellation after publication and never admit a successor before ack. |
| Hydration | `lifecycle.rs:250-402` | Pre-acquisition cancellation is zero-permit; acquired Hydration observes permit cancellation, drops/joins mutation-capable work, then explicitly or by Drop acknowledges retirement. |
| Drain | `lifecycle.rs:420-494` | Consume only a transferred full mask; remove split takes, re-arm, bounded loop, and double drain. |
| Watchers | `ipc_server.rs:684,1135` | Both legacy/v2 watcher drivers use cancellation-bearing permits and immutable snapshots; cancellation suppresses later phases and ack follows DB/file-capable exit. |
| Startup | `ipc_server.rs:1210-1224` | Use one guarded request; Startup receives cancellation and cannot overlap a successor because retirement ack gates release. |
| Health timestamp | `doctor.rs:115-135` | Preserve observer behavior; only current-permit completion updates it. |

### Repository-test migration

`tests/contract/read_test.rs`, `tests/contract/write_test.rs`, and `tests/integration/indexing_resilience_test.rs` directly call tokenless methods today. They are repository tests, not evidence of a published library contract. Migrate observable assertions to public tool behavior. Move authority invariants to co-located private tests in `state.rs`, `lifecycle.rs`, and `ipc_server.rs`. No `pub`, feature-gated public, or test-only public ownership seam is permitted.

## PR #316 lifecycle remediation decision

The earlier lifecycle and binding comments remain valid and accepted: exact completion must be RAII-safe; same-binding refresh preserves the authoritative union; distinct binding carries zero old-workspace bits; stale finish/Drop is isolated; process abort uses reconciliation.

Review-fix cycle 2/3 at exact HEAD `d6321504137445a94b4134718355b87cceb75db6` adds valid P1 `discussion_r3701136926` / thread `PRRT_kwDORJEduc6V2olJ`. Targeted source inspection confirms the reported gap: the current generation cancellation receiver is passed only to `background_db_hydration`, while `index_workspace`, `sync_workspace`, the legacy watcher, and the v2 watcher hold no receiver. The previous proposed rebind cleared owner and exposed promoted work immediately, so a successor could run while a retired driver still touched the same database.

Strategy A remains selected, but owner retirement is now a **cancellation-plus-acknowledgment quiescence barrier**:

1. every acquired Index, Sync, Hydration, Startup, and Watcher permit receives the current generation `watch::Receiver<bool>` and an immutable binding snapshot;
2. active rebind advances the visible binding/floor and signals old cancellation, but replaces `Running` with one `RetirementBarrier` rather than clearing ownership for a successor;
3. for the same binding, `owner 0b101 OR pending 0b010 = 0b111` moves exactly once into the barrier's new-generation `deferred` field; for a distinct binding, `deferred` starts empty and no old-workspace bit transfers;
4. current-token requests during retirement cannot acquire and OR only into `deferred`; empty waiters publish no work and wait for the mandatory acknowledgment notification;
5. only exact explicit terminal or armed Drop after all DB/file-capable work exits acknowledges retirement, moves deferred to ordinary pending, clears the barrier, and invokes `notify_one` exactly once after unlock;
6. a later terminal is stale and changes nothing. A non-quiescent driver leaves the barrier closed; no timeout admits overlap.

A second rebind while waiting retargets the same barrier. Equal target binding preserves/retags deferred; distinct target binding discards superseded-target work. The original retired identity remains the sole acknowledgment key, so there is never a stack of retired owners or an unowned gap.

This maintains the accepted binding rules. New-binding startup/hydration/offline detection still reconciles durable state after distinct-binding acknowledgment, and non-durable companion intent still requires a request with the latest token. Process abort still uses restart reconciliation and full-unit rollback, not a false Drop/ack claim.

### Final review-fix cycle 3/3: successful release and empty-waiter baton

Exact Copilot P1 `discussion_r3701238147` / thread `PRRT_kwDORJEduc6V25-i` is valid. A current empty Hydration/Startup/Watcher request can receive `Queued` while another owner is `Running`. The prior no-pending success path cleared the owner and returned `Released` but emitted no notification, so no later transition was guaranteed to wake any waiter.

The selected strategy therefore adds these exact liveness rules without changing work authority:

1. every empty waiter creates and enables `Notified` before its final request/recheck; `Queued` awaits that registration, and every wake enables a fresh registration before rechecking;
2. exact no-pending `Running` completion clears owner, disarms and timestamps once, selects `Released`, drops the mutex, invokes `notify_one` exactly once, then returns the selected outcome;
3. one notification allows at most one mutex-authorized empty acquisition. Remaining waiters stay registered, and the acquired empty owner completes `Released` once to pass the baton;
4. if non-empty work wins, the empty waiter stays queued and blocks on its fresh registration. No polling, second queue, duplicate mask, notification authority, or concurrent driver is introduced;
5. exactness refers to one post-unlock `notify_one` call. Tokio may resume at most one waiter or retain/coalesce a permit, so no exact resumed-task count is claimed. Deterministic fixtures guarantee eventual finite baton progress instead.

Running armed Drop and retirement acknowledgment retain their accepted one-call post-unlock wake. Stale terminals remain zero-mutation/zero-notification no-ops. All same/distinct binding, cancellation/quiescence, successful-disarm, and process-abort contracts remain unchanged.

## Selected API strategy: A

Use one crate-private, `Arc`-owned coordinator cell. All fields remain private:

```text
CoordinatorCell { state: Mutex<SyncCoordinator>, notify: Notify }
BindingIdentity { workspace_uuid, workspace_id }
SyncCoordinator {
  floor, binding_identity, next_sequence,
  phase: Idle | Running(OwnerRecord) | Retiring(RetirementBarrier),
  pending, generation_cancel, last_indexed_at
}
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerIdentity { generation, sequence, kind }
AdmissionGuard { cell, token, binding_snapshot, cancel_rx, enabled_notification }
OwnerPermit {
  cell, token, binding_snapshot, cancel_rx,
  identity, work_mask, cleanup_armed
}
DriverTaskGuard { join_handle, abort_handle, terminal_state }
RetirementBarrier {
  retired_identity, retired_binding, target_generation, target_binding,
  deferred
}
```

`request(admission, work_mask, owner_kind)` consumes a non-cloneable guard and returns `Acquired(OwnerPermit) | Waiting(AdmissionGuard) | Enqueued | Stale`. In `Running` or `Retiring`, non-empty current work is committed to the authoritative pending/deferred mask before `Enqueued`; an empty internal waiter receives `Waiting` with the same receiver and already-enabled pinned `OwnedNotified`. In `Idle`, direct Index/Sync preserves its requested kind, drops the enabled waiter registration, and moves the remaining cancellation/binding ownership into the acquired permit; only a completion-transferred coalesced successor normalizes to Sync. The coordinator clones a receiver only when minting AdmissionGuard, callers cannot extract/clone it, and acquisition/transfer moves ownership without retaining a registration that could steal a release wake. A Waiting loop selects its owned notification against its owned receiver and re-arms before recheck.

### Atomic active rebind

Prepare/capacity-check the complete tuple, acquire binding guards then coordinator mutex, and perform no await under the standard mutex. If an owner is running:

- move same-binding `owner mask OR pending` to barrier deferred under the new generation, or move zero old bits for a distinct binding;
- retain the old identity as `retired_identity`, clear ordinary pending, publish the new binding/floor/cancellation channel, and install no successor;
- unlock, then synchronously signal the old generation cancellation sender. Do not notify the coordinator at rebind.

The old permit has no current-work authority after publication, but its exact identity remains the barrier key until the driver exits. The barrier deferred field is the sole authoritative current-generation union while waiting. Requests for the current token OR there and cannot run. Rebind while retiring retargets that one barrier by equality with the previous target binding.

### Exit acknowledgment and RAII

Each owner wraps its entire DB/file-capable driver future in the permit lifetime and observes cancellation. Explicit acknowledgment is allowed only after mutation-capable child work is dropped or joined. Armed Drop may acknowledge only after stack unwinding has dropped that work; any detached mutation-capable child is forbidden.

An exact terminal against `Running` with pending work transfers one successor. With no pending it clears owner, disarms and timestamps once, selects `Released`, drops the mutex, and calls `notify_one` exactly once before returning. An exact terminal against `Retiring` is `RetirementAcknowledged`: move deferred to the latest generation ordinary pending, clear the barrier, disarm when explicit, write no timestamp, and call `notify_one` exactly once after unlock. It returns no successor permit to the retired driver. Retiring Drop performs the same transition without OR-ing the old mask a second time. Identity mismatch is a no-op.

The completion/rebind race is lock-linearized: completion first is ordinary; rebind first makes that terminal the unique ack. If cancellation is ignored or a driver hangs, the barrier remains closed and rollback/restart is required. This favors safety over bind progress and prevents overlapping old/new drivers.

## Protected invariants

1. One coordinator phase is the only authority for binding/floor, owner or retirement identity, complete current-generation mask, cancellation generation, and timestamp.
2. In `Retiring`, the barrier deferred mask is the sole authoritative current-generation work location; no successor permit exists.
3. Every `OwnerKind` receives cancellation and must acknowledge DB/file-capable exit before any successor can acquire or run.
4. Same-binding active rebind stores `0b101 OR 0b010 = 0b111` in the barrier; distinct binding stores zero old bits and uses new-binding reconciliation/reissue.
5. Current requests coalesce into the barrier. Exact ack publishes deferred, clears the barrier, and invokes `notify_one` once after unlock; a stuck driver keeps the barrier closed.
6. Rebind while retiring preserves same-target deferred work or discards distinct superseded-target work without adding a retired owner.
7. Running completion disarms/timestamps once. Its no-pending `Released` path clears owner then invokes `notify_one` once after unlock; running Drop republishes and invokes once. Retiring ack never timestamps or re-adds the old mask.
8. Empty Hydration/Startup/Watcher waiters enable notification before final request/recheck. One empty acquisition at a time passes the release baton; actual waiter resumption is at most one per call and notification is never authority.
9. Stale request, finish, acknowledgment, and Drop mutate and notify zero times.
10. Every driver retains its immutable binding snapshot; no detached DB/file/workspace mutator survives acknowledgment.
11. Hydration performs zero DB/file I/O before a permit. No mutex crosses await and Drop never awaits/spawns/panics.
12. No second queue, split/double drain, producer reacquire, sleep, unsafe, public seam, busy loop, or forced barrier timeout.
13. CLI/MCP, wire/schema/persistence/config, exact queued JSON, and process-abort reconciliation remain unchanged.

## Compatibility and semver disposition

Strategy A is an internal Rust source migration, not a public semver break: the package is non-published and distributed as a binary. No version-major bump or deprecation bridge is required. The release unit must preserve CLI commands, MCP tool schemas, JSON status/message text, health meaning, startup behavior, and persisted files. Release notes may describe a concurrency correctness fix but must not advertise a new Rust API.

Strategy B is rejected because it would create and support a public permit-bearing Rust API without product evidence requiring one. Strategy C is rejected because all critical callers are migratable internal code and private deterministic harnesses are proven. If the facts change, implementation fails closed before source edits rather than shipping an unsafe compatibility adapter.

## Replacement-plan input and disposition

The old residual plan and tasks assume signature preservation, `Reacquired`, split claims, and bounded double-drain behavior. Those scopes remain superseded. The replacement plan keeps every task at `<=110 minutes`, `<=2` production files, `<5` production functions, and `<=3` scenarios in the existing strict chain. Core RED/GREEN adds the cancellation receiver and ordinary RAII lifecycle. Binding RED/GREEN adds one retirement barrier, request coalescing, repeated-rebind retargeting, and exact ack publication. Hydration, Index/Sync, Startup, and both Watcher task pairs each prove cancellation observation and mutation-capable exit before ack. Parameterized OwnerKind/relation/terminal matrices preserve the scenario caps while proving same-binding `0b111`, distinct-binding zero carryover, no successor-before-ack, one post-unlock notification call, max one DB driver, no post-ack old work, and stale-terminal isolation. Existing fixtures also add owner-success/single-empty-waiter and multi-waiter baton rows for Hydration/Startup/Watcher, asserting exact call counts, at-most-one acquisition per call, finite progress, and no spin or queue duplication. Pre-acquisition cancellation remains zero-permit, and non-durable companion work still requires a latest-binding token.

Final disposition: **PIVOT to strategy A with high confidence.** Final configured Stage review-fix cycle 3/3 passes with P0/P1/P2/P3 = `0/0/0/0`. Implementation remains blocked until Ship closes `106-S`, Stage performs the documented fail-closed requeue transaction, and all existing gates pass.


## Phase 5E residual cancellation-handle decision

### Finding and validity

PR #316 comments `r3701318733` and `r3701318749`, plus the exact Ship backlog comment on `109-F` at `c6f2b06174b10724ed9527601cd4ad6448c1433d`, identify a valid P1. The selected API gave cancellation only to an acquired permit. A bare queued empty waiter could not observe generation cancellation, and rebind deliberately emits no coordinator notification. With no later owner transition, the waiter could remain blocked forever. All earlier PASS claims are superseded for this gate.

### Selected ownership contract

`GenerationToken` is an opaque identity value, not a standalone wait capability. A caller must hold a non-cloneable `AdmissionGuard` containing the token, exact binding snapshot, coordinator cell, one receiver clone minted by the coordinator, and a notification registration enabled before request. Callers cannot extract or clone the receiver.

`request` consumes the guard and has four distinct internal outcomes:

1. `Acquired(OwnerPermit)`: the enabled waiter registration is dropped, then receiver, token, snapshot, and cell ownership move into one armed non-cloneable permit; no stale registration can consume its release wake.
2. `Waiting(AdmissionGuard)`: only an empty internal waiter receives this result; it owns the already-enabled notification and selects it against cancellation before re-arming for a recheck.
3. `Enqueued`: only a non-empty busy producer receives this after the complete mask is coordinator-owned in ordinary pending or barrier deferred; caller return cannot lose the work.
4. `Stale`: zero mutation and no retained obligation.

Rebind signals the old channel for both waiting admission guards and acquired permits, including an idle/no-owner row where no coordinator notification or later owner transition occurs. Direct idle Index/Sync requests preserve their requested kind; only completion-transferred coalesced work becomes Sync.

### Complete cleanup ownership

- Pre-acquisition cancellation drops only the admission guard and mutates nothing.
- Post-acquisition normal/handled-failure paths consume and disarm the permit; `?`, early return, panic, or caller future abort reaches mandatory armed Drop.
- A transferred successor receives the moved cancellation/binding ownership and complete mask. Loss or abort before execution republishes that entire mask once.
- Every spawned Hydration/Startup/Watcher or progress-mutator task has a parent-retained `DriverTaskGuard`. Raw JoinHandle drop/detach is forbidden; normal shutdown joins, guard Drop aborts, and the mutation-capable child future is gone before permit Drop or retirement acknowledgment.
- A mutation-capable non-cancellable child that cannot be joined is a stop-and-replan condition. Authority-free CPU parsing is the only child-outliving-parent exception.
- Process abort remains restart reconciliation and qualified intent reissue, never an RAII guarantee.

The cleanup guard is therefore explicit, non-optional, and continuous from waiting admission through acquired and transferred ownership. No detached cancellation receiver can become the only route to release a permit or full `WorkMask`.

### Deterministic acceptance

The replacement task matrices must compile before failing and prove, without sleeps or public seams: idle rebind cancels a pre-acquisition waiter without any owner wake; acquisition removes its enabled registration and moves cancellation/binding ownership exactly once; post-acquisition early return and abort recover; raw spawned-handle loss cannot detach a mutation-capable owner; Hydration and both Watchers are supervised; progress helpers end before owner terminal; full-mask transfer loss republishes once; and aggregate active-driver count never exceeds one. The existing `109.014-T` through `109.031-T` chain carries these rows within `<=3` scenarios, `<=2` files, and `<=110` minutes per task.
