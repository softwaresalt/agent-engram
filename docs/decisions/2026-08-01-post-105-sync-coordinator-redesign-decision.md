---
title: "Post-105 sync coordinator redesign"
type: decision
date: 2026-08-01
status: "decided-with-spike-gate"
feature: "109-F"
shipment: "104-S"
branch: "107-stage-102-104-integration"
head_examined: "538e0ab95ce1ad2ecb77925950f89e63d6d74f58"
tags: ["daemon", "pending-sync", "generation", "ownership", "concurrency"]
---

# Post-105 sync coordinator redesign

## Executive decision

Freeze the current 109-F implementation plan and retain all existing provenance. The next executable work is one bounded proof task, `109.013-T`, for a single authoritative synchronization coordinator. Shipment `104-S`, feature `109-F`, and the twelve existing implementation tasks remain blocked until that proof closes the public compatibility and deterministic RED questions and a revised implementation plan receives a fresh PASS.

The recommended design is Option A: one mutex-protected `SyncCoordinator` in `AppState` that owns the generation floor, indexing owner, and complete pending mask. Workspace/config rebinding publishes one opaque generation token at a single no-await linearization point. Internal producers use token-qualified requests and opaque owner permits. Hydration performs no DB work until it holds a permit. Owner completion atomically transfers the full mask or releases and notifies; there is no second queue, producer reacquire loop, split companion-bit claim, or double drain.

This decision is planning only. Stage did not modify source, tests, build output, Git history, branches, shipments through Ship lifecycle actions, or pull requests.

## Evidence examined

### Current source contracts

- `src/server/state.rs:157-211` stores `{generation, flags}` in `PendingSyncState`, but `arm` retains an existing owner and ORs bits even when passed an older generation.
- `src/server/state.rs:215-270` keeps `indexing_in_progress`, `pending_sync`, and `sync_generation` as separate authorities.
- `src/server/state.rs:526-645` separates owner CAS, generation capture, queue publication, and producer reacquisition.
- `src/server/state.rs:651-723` exposes split pending and companion-bit consumers, so full-mask ownership is not represented as one value.
- `src/server/state.rs:397-417` publishes workspace/config before `src/tools/lifecycle.rs:176-185` calls `begin_scan_generation`; a sync can observe the new binding under the old generation.
- `src/tools/lifecycle.rs:229-405` calls `connect_db` even when `try_start_indexing` returned false. Hydration therefore has no exclusive pre-DB ownership contract.
- `src/tools/lifecycle.rs:419-466` consumes pending before acquiring indexing, then separately consumes companion bits and re-arms on a lost race.
- `src/tools/write.rs:237-322` uses publish-then-reacquire as a producer backstop and preserves the public queued response.
- `src/daemon/ipc_server.rs:1210-1225` still uses try-then-set for startup and drains through a separate helper.
- External contract and integration tests call public `try_start_indexing`, `take_pending_sync`, and the queued response path. Compatibility cannot be inferred away.

### Prior learnings applied

- `docs/compound/best-practices/packed-atomic-clear-requires-atomic-publish-2026-07-29.md`: the full mask must publish and clear as one operation.
- `docs/compound/concurrency-issues/atomicbool-drain-race-take-before-lock-2026-05-09.md`: consuming work before ownership creates a loss window.
- `docs/compound/concurrency-issues/pending-sync-drain-must-cover-all-finish-indexing-sites-2026-05-09.md`: release and drain obligations must be co-located.
- `docs/compound/best-practices/pub-visibility-for-external-test-harness-2026-04-20.md`: external tests constrain public visibility; RED proof should prefer co-located private unit tests.
- `docs/compound/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md`: after three materially identical review failures, freeze the rejected contract and change the design or proof strategy.

## Root cause of the repeated review failure

The rejected plan tried to repair five symptoms while retaining four independent authorities:

1. an atomic indexing owner;
2. a mutex-protected pending generation and mask;
3. a separate atomic generation floor; and
4. lifecycle-local hydration and drain ownership.

That leaves no single linearization point that can answer all of these questions together: which generation a request belongs to, whether it may acquire, who owns the full mask, who must execute or hand off the work, and when hydration may touch the DB. Additional wrappers and RED/GREEN pairs made the prose narrower but did not remove that ambiguity. The same P1s therefore reappeared after each review cycle.

## Gate-blocking areas and required closure

| P1 area | Current gap | Required closure |
|---|---|---|
| Public wrapper qualification | Generation is loaded outside the queue lock; stale operations can arrive below an advanced floor | Every retained public wrapper delegates to the same coordinator and qualifies at its linearization point; stale token-qualified internal work is rejected without owner or queue mutation |
| Hydration ownership | DB connect can begin without indexing ownership | Hydration waits cancellation-aware for an opaque permit and performs zero DB or file work before permit acquisition |
| Arbitration-mask ownership | Owner, pending bit, and companion bits are acquired or consumed separately | One `WorkMask` value moves atomically between pending state and exactly one owner permit |
| Deterministic RED proof | Existing tests use yields, real DB failure, or behavior that may not compile against a proposed API | Every RED first passes `cargo test --no-run`, then fails an assertion on current behavior using barriers, oneshots, or direct same-module state transitions; missing symbols and public test-only seams do not count |
| Predecessor/index proof | Prose allows stale or unavailable index state to be mistaken for completion | Before any 104-S claim, index sync must succeed and exact 102-S and 103-S records must show positive terminal shipped or merged evidence; warning, absence, blocked, queued, unknown, or unqueryable stays blocked |

## Option A: mutex-protected SyncCoordinator and owner permits

### Ownership model

`AppState` contains one private `std::sync::Mutex<SyncCoordinator>` plus one private `tokio::sync::Notify`. The coordinator owns:

- the current generation floor;
- the current owner identity and owner kind;
- zero or one complete pending `WorkMask` for the current generation; and
- a monotonically increasing owner sequence used to reject stale completion.

The `Notify` is a wake mechanism, not a work queue. No coordinator guard crosses `.await`.

### Generation linearization point

A crate-private async install operation acquires workspace, config, and cancellation write guards in the documented order. It validates generation overflow before mutation. With all async guards held, it locks the coordinator only for the final non-awaiting critical section that publishes workspace, config, cancellation ownership, and the new generation floor. It returns an opaque `GenerationToken` and cancellation receiver.

Internal snapshot consumers use a crate-private qualified snapshot carrying that token. The public `DispatchSnapshot` shape remains unchanged.

### Request, handoff, and mask ownership

A token-qualified request enters one coordinator operation and returns one of:

- `Acquired(OwnerPermit, WorkMask)`;
- `Queued`;
- `Stale`.

There is no producer `Reacquired` branch. If the owner completes while work is pending, completion atomically hands the full mask to one successor permit or returns it to the same bounded driver. If no work is pending, completion clears the owner and notifies waiters. A request arriving after release sees no owner and acquires directly. This removes the final-peek lost wake rather than backstopping it with a second acquisition.

### Public compatibility

Retained public `AppState` methods keep their signatures and become adapters over the coordinator:

- fresh-at-call setters qualify under the coordinator lock;
- public publish methods merge a complete mask;
- public owner methods map to a legacy owner kind within the same coordinator;
- split take methods require an explicitly documented compatibility mapping that never exposes an orphan companion mask.

All production internal callers must migrate to the token and permit API. Final source inventory must show zero internal use of split legacy publication or take methods. The bounded spike must prove the exact compatibility mapping before implementation planning resumes.

### Startup hydration handoff

Hydration registers `Notified` before rechecking acquisition, then awaits either notification or cancellation without a coordinator guard. It exits stale or cancelled without DB work. After acquiring a permit, every normal, cancelled, and DB-failure exit completes the permit, clears only exact-generation state, and transfers or releases ownership.

### Deterministic RED seam

Use co-located `#[cfg(test)]` modules in at most `state.rs` and `lifecycle.rs`. They can see private coordinator state and do not require public test APIs. Synchronize with `Barrier`, `Notify`, or oneshot channels. No sleeps, permission races, real daemon, or publicly exported test hook is allowed.

### Migration and rollback

Migrate primitive, public adapters, write, lifecycle drain, hydration, and startup in dependency order while keeping public CLI and MCP responses stable. There is no persistence or schema migration. Rollback is a full release-unit revert and daemon restart; partial rollback after internal callers adopt permits is forbidden.

## Option B: packed atomic synchronization word

### Ownership model

Replace the separate owner, generation, and flags with one CAS-managed `AtomicU64` word containing generation epoch bits, owner state, and the complete work mask. An opaque token captures the epoch. Requests and completion use compare-exchange loops, and a private `Notify` wakes hydration waiters.

### Generation and handoff

Workspace/config install publishes the next representable epoch only after capacity checks. Request CAS validates the token, merges the full mask, and acquires only when the owner bit is clear. Completion CAS either transfers the mask to the next owner state or clears ownership and notifies.

### Compatibility, testing, and rollback

Public methods remain adapters around CAS loops. Co-located tests drive exact word transitions and ABA boundaries. No second queue or mutex-across-await is needed. Rollback is also a full release-unit revert.

### Trade-off

This option is viable and can be faster, but it adds bit-allocation limits, epoch-wrap proof, more difficult public-adapter reasoning, and harder debugging. The current defect is correctness-sensitive and low-throughput; lock-free complexity has no demonstrated value.

## Comparison

| Criterion | Option A: SyncCoordinator mutex | Option B: packed atomic word |
|---|---|---|
| Authoritative owner | One explicit coordinator | One packed CAS word |
| Generation linearization | Full `u64` token in a no-await critical section | Bounded packed epoch with wrap proof |
| Startup hydration | Permit wait with private Notify | CAS wait with private Notify |
| Arbitration mask | First-class `WorkMask` moved whole | Packed mask moved whole |
| Deterministic RED | Direct private state plus barriers | Exact CAS word transitions plus barriers |
| Public compatibility | Readable adapters; spike still required for split take contract | Compact but more difficult adapter semantics |
| Migration scope | Moderate, four runtime modules | Moderate to high, same modules plus encoding proof |
| Rollback | Full release-unit revert, no data migration | Full release-unit revert, no data migration |
| Main risk | Adapter bridge accidentally retains dual authority | ABA, bit exhaustion, and opaque CAS failures |

## Recommendation

Choose Option A. It makes authority visible in the type model, eliminates the extra producer reacquire and double-drain patterns, preserves deterministic wakeup without timing, and gives review one place to verify generation, ownership, and mask invariants. It also keeps full-width generations and makes failure diagnostics understandable.

Do not yet re-queue the implementation plan. The exact public compatibility bridge and compiling RED shape are source-coupled unknowns that Stage cannot honestly certify without a bounded build-capable proof.

## Protected invariants and proof obligations

1. Exactly one authority stores generation floor, owner, and pending mask.
2. A workspace/config snapshot and its `GenerationToken` are coherent.
3. Stale tokens never acquire, enqueue, replace, or clear current work.
4. Every non-empty pending mask contains the routine owner bit.
5. A complete mask is owned by at most one permit and is never split during arbitration.
6. Hydration performs zero DB or file work before a permit.
7. Every owner exit transfers or releases and notifies exactly once.
8. No standard mutex or pending guard crosses `.await`.
9. No second queue, recursive drain, producer reacquire, or public test-only seam is introduced.
10. Public CLI, MCP, schema, persistence, queued response, and startup caller contracts stay stable unless a separately approved compatibility decision says otherwise.
11. Every RED compiles first and then fails only on the intended assertion.
12. Every implementation task remains at most two production files, four deterministic scenarios, and 110 minutes.
13. 104-S predecessor and index validation fails closed.

## Objective restart criteria for 104-S

All conditions are mandatory:

1. `109.013-T` is done with a findings artifact that proves or rejects the public adapter mapping and private deterministic RED strategy.
2. A revised implementation plan adopts one authoritative model and maps every internal caller in `state.rs`, `write.rs`, `lifecycle.rs`, and `ipc_server.rs`.
3. Plan hardening records no second queue, no double drain, no public test seam, no mutex across await, and whole-mask permit ownership.
4. Fresh plan review of the revised implementation plan returns PASS with zero open P0/P1. A PASS on the spike plan alone is not sufficient.
5. All implementation tasks are harvested after that PASS and satisfy the width limits.
6. Backlog index sync succeeds immediately before claim.
7. Exact shipment reads show `102-S` and `103-S` exist and have positive terminal shipped or merged evidence. Any warning, absence, blocked, queued, unknown, omitted, or unqueryable result keeps `104-S` blocked.
8. Stage explicitly moves `104-S`, `109-F`, and the implementation tasks back to queued. Ship must not infer readiness from this decision artifact.
