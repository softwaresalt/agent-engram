---
title: "106-S single-authority sync coordinator spike execution evidence"
type: spike-evidence
doc_type: research
source: "106-S / 109.013-T"
date: 2026-08-02
status: execution-evidence-for-stage-consumption
shipment: "106-S"
task: "109.013-T"
base_head: "f10ab572082bb93e9f68f65f25095d82edfa512a"
branch: "spike/106-sync-coordinator-proof"
tags: ["concurrency", "pending-sync", "red-proof", "compatibility", "single-authority"]
---

## Scope and disposition

This is raw Ship execution evidence for Stage. It is not the Stage-owned findings artifact, does not close `106-S` or `109.013-T`, and does not authorize `104-S`, `109-F`, or implementation work.

The spike used one core worktree and temporarily added four co-located `#[cfg(test)]` scenarios plus one private test helper in only:

* `src/server/state.rs`
* `src/tools/lifecycle.rs`

There were zero release-mode production-function changes, public test seams, sleeps, live daemons, operator-workspace actions, dependency changes, schema or protocol changes, second queues, double drains, mutex guards across await, or unsafe code. The source edits were removed after evidence capture. Elapsed time from the `106-S` claim through raw evidence capture was under 13 minutes, below the 110-minute bound.

Removal was verified against the pre-spike blobs:

```text
src/server/state.rs      3bfa22d93436364a2f06b592857e5f4c635277d6  MATCH
src/tools/lifecycle.rs   6e2f1d56d948ef2bceb11c0dad7a4184ca25351d  MATCH
git diff --name-only -- src tests                                 EMPTY
```

## Current authority inventory

Current `AppState` has four independent synchronization authorities:

1. `indexing_in_progress: AtomicBool`
2. `pending_sync: Mutex<PendingSyncState>`
3. `sync_generation: AtomicU64`
4. lifecycle-local acquisition, hydration, and drain control

The current code therefore cannot atomically answer generation validity, owner identity, complete-mask ownership, and completion handoff at one linearization point.

## Public compatibility table

Line references are from base head `f10ab572082bb93e9f68f65f25095d82edfa512a`.

| Public method | Current contract | Single-authority classification | Required mapping |
|---|---|---|---|
| `is_indexing(&self) -> bool` | Observe global indexing flag | Stable observer | Return whether the coordinator has any owner |
| `try_start_indexing(&self) -> bool` | Claim global owner without returning identity | Incompatible with sequenced completion | Replace ownership use with a permit-returning API; the boolean signature cannot identify the owner later |
| `finish_indexing(&self)` | Clear whichever global owner is current and update `last_indexed_at` | Incompatible with stale-completion rejection | Require `OwnerPermit` or an RAII guard; valid completion updates `last_indexed_at` exactly once after releasing the coordinator guard, while stale completion neither clears an owner nor updates time |
| `last_indexed_at(&self)` | Observe the latest completed indexing time | Stable observer | Read the timestamp recorded only by valid permit completion |
| `current_sync_generation(&self) -> u64` | Observe raw generation | Stable observer | Return the coordinator floor; internal callers use opaque `GenerationToken` |
| `set_pending_sync(&self)` | Arm routine bit at current generation | Stable public adapter | Atomically request a complete routine mask at the current floor |
| `publish_pending_sync(&self, bool, bool)` | Publish routine plus companions | Stable public adapter | Atomically merge one complete `WorkMask` |
| `publish_pending_sync_and_try_reacquire(&self, bool, bool) -> bool` | Publish, then claim without returning owner identity | Incompatible when `true` means ownership | Replace with permit-bearing `request`; a boolean acquisition recreates tokenless completion |
| `take_pending_sync(&self) -> bool` | Clear only the routine bit without returning the complete mask | Incompatible with whole-mask ownership | Replace with an API that returns a permit and complete `WorkMask`; no safe concurrent continuation exists for the split boolean calls |
| `has_pending_sync(&self) -> bool` | Peek routine bit | Stable observer | Observe whether a complete pending mask exists |
| `clear_pending_sync_for_generation(&self, u64)` | Accept an untrusted raw generation to clear work | Incompatible with opaque token qualification | Replace internal use with exact permit cancellation/completion; retaining this public mutator requires a separate compatibility decision |
| `set_pending_sync_revalidate(&self)` | Stage a companion-only intermediate state | Incompatible with the complete-mask invariant | Replace with one complete-mask request; retaining the old standalone behavior would require a forbidden second staging authority |
| `take_pending_sync_revalidate(&self) -> bool` | Continue a global split take without owner identity | Incompatible with whole-mask ownership | Consume from a returned `WorkMask`, not global state |
| `set_pending_sync_backfill_python(&self)` | Stage a companion-only intermediate state | Incompatible with the complete-mask invariant | Replace with one complete-mask request; retaining the old standalone behavior would require a forbidden second staging authority |
| `take_pending_sync_backfill_python(&self) -> bool` | Continue a global split take without owner identity | Incompatible with whole-mask ownership | Consume from a returned `WorkMask`, not global state |
| `begin_scan_generation(&self)` | Advance generation and replace cancellation channel | Signature-compatible lifecycle adapter | The new private install operation must publish binding, cancellation ownership, and floor together, then return an opaque token |

The original signature-preservation assumption is rejected. A tokenless `finish_indexing(&self)` cannot reject this deterministic sequence: owner A acquires and completes, owner B acquires, then a stale second completion from A clears B. Both owners have the same hidden kind, and the call carries no permit or sequence. Thread-local or side-map ownership would be a second authority and is not valid for async callers.

The split-take bridge has the same identity problem. A global legacy cursor cannot associate later companion calls with the caller that consumed the routine bit, while an extracted-mask cache is a forbidden second authority. The standalone companion setters also require a behavior change because companion-only staging violates the complete-mask invariant.

Stage must therefore choose an explicit compatibility path before implementation planning: a permit-bearing public API with a semver decision, removal or visibility reduction of the ownership mutators, or deferral of Option A. In-repository callers and tests can migrate, but that does not prove downstream compatibility.

## Production caller inventory

| Surface | Caller and line | Current role | Migration |
|---|---|---|---|
| `is_indexing` | `src/tools/write.rs:32` | `flush_state` rejection observer | Stable observer |
| `try_start_indexing` | `src/tools/write.rs:160` | Full index claim | `request(token, index mask, Index)` |
| `try_start_indexing` | `src/tools/write.rs:268` | Sync claim or queued response | `request(token, sync mask, Sync)` |
| publish/reacquire | `src/tools/write.rs:297-304` | Queued sync producer backstop | One atomic request; remove producer reacquire and separate finish/drain |
| `finish_indexing` | `src/tools/write.rs:470` | Shared index/sync finalizer | `complete(permit)` and act on transfer/release outcome |
| `last_indexed_at` | `src/tools/doctor.rs:115-135` | Distinguish not-yet-indexed from healthy session state | Stable observer; valid completion preserves the health timestamp |
| `begin_scan_generation` | `src/tools/lifecycle.rs:187` | Bind generation and cancellation | Move into coherent binding install returning `GenerationToken` |
| `try_start_indexing` | `src/tools/lifecycle.rs:250` | Hydration claim, currently ignored on failure | Cancellation-aware permit acquisition before any DB/file work |
| generation clear/finish | `src/tools/lifecycle.rs:274-275,301-302` | Hydration cancel and DB-failure exits | `complete` or cancel the exact sequenced hydration permit |
| `finish_indexing` | `src/tools/lifecycle.rs:402` | Normal hydration completion | `complete(permit)` |
| split takes/claim/finish | `src/tools/lifecycle.rs:420-463` | Coalesced drain | Replace with one complete transferred `WorkMask`; remove split consume and re-arm |
| pending observer | `src/tools/lifecycle.rs:484,494` | Bounded drain loop | Remove once completion owns handoff; no second drain |
| `try_start_indexing` | `src/daemon/ipc_server.rs:684,1135` | File/content watcher claims | Token-qualified `Watcher` permits |
| startup try/set | `src/daemon/ipc_server.rs:1210-1215` | Startup try-then-set arbitration | One atomic `Startup` request |
| finish/drain helper | `src/daemon/ipc_server.rs:1219-1224` | Startup/watcher release and drain | `complete(permit)` with one transfer/release outcome |

`src/server/state.rs` also calls its own raw-generation and pending helpers at lines `570`, `597`, `640-644`, `690`, and `711`; those public wrappers must delegate to the coordinator rather than retain primitive authority.

## External-test caller inventory

No external test calls the standalone companion setters or companion take methods.

| File | Direct calls | Compatibility constrained |
|---|---|---|
| `tests/contract/read_test.rs` | `try_start_indexing` at `302`, `324`, `346` | Migrate fixture ownership to a returned permit while preserving tool behavior |
| `tests/contract/write_test.rs` | `try_start_indexing` at `108`, `150`, `185`; `take_pending_sync` at `159` | Migrate fixtures and assert queued response through complete-mask observation |
| `tests/integration/indexing_resilience_test.rs` | `try_start_indexing` at `54`, `75`, `99`, `125`, `156`; `take_pending_sync` at `135`, `141`; `finish_indexing` at `157` | Migrate start/finish to permits and replace split-take assertions |

Co-located tests in `src/tools/lifecycle.rs` and `src/daemon/ipc_server.rs` also exercise generation clear, companion bits, producer reacquire, bounded drain, and startup helpers. They are migration tests, not justification for retaining internal legacy paths.

## Recommended authoritative API

Option A remains the recommended internal authority, using one private `Mutex<SyncCoordinator>` and one private `Notify`, but it cannot proceed until Stage resolves the public compatibility break:

```text
GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerPermit { generation, sequence, kind, work_mask }

request(token, work_mask, owner_kind)
  -> Acquired(OwnerPermit) | Queued | Stale

complete(owner_permit)
  -> Transferred(OwnerPermit) | Released | Stale
```

`OwnerPermit` must be opaque and sequence-qualified. `complete` accepts only the exact permit and moves the complete pending mask to one successor or releases and notifies. An async completion wrapper validates and transitions under the coordinator mutex, drops that guard, and updates `last_indexed_at` exactly once only for a valid completion; no mutex guard crosses the timestamp await. Stale completion does not update health state. Observers and request-only publishers can remain signature-compatible wrappers, but ownership, completion, generation-clear, and split-take mutators need an explicit API compatibility decision.

## State diagram

```text
Idle(floor, no pending)
  current request ------------------> Owned(seq, kind, complete mask)
  stale request --------------------> Idle unchanged

Owned(seq, kind, owner mask)
  current request ------------------> Owned + Pending(one complete ORed mask)
  stale request --------------------> Owned unchanged
  stale/mismatched complete --------> Owned unchanged
  exact complete, pending ----------> Owned(new seq, successor, full pending)
                                       and Pending becomes empty
  exact complete, no pending -------> Idle + notify waiters

Generation install
  validates overflow, publishes binding/cancel/floor in one no-await section,
  invalidates older tokens, and never lets an older completion clear new state.

Hydration
  registers notification, requests a permit, and waits cancellation-aware
  without a mutex guard; DB/file work starts only after Acquired.
```

## Compile-first RED evidence

All four narrow library harnesses compiled successfully before execution:

| Scenario | Exact compile command | Result |
|---|---|---|
| S1 | `cargo test --lib --no-run spike_s1_stale_wrapper_cannot_mutate_newer_mask` | PASS; test binary produced; initial clean build `3m 28s` |
| S2 | `cargo test --lib --no-run spike_s2_finish_transfers_full_mask_to_one_successor` | PASS in `1.71s` |
| S3 | `cargo test --lib --no-run spike_s3_hydration_does_no_db_work_before_ownership` | PASS in `1.58s` |
| S4 | `cargo test --lib --no-run spike_s4_startup_release_request_has_one_executor` | PASS in `1.59s` |

There were no compiler diagnostics, missing symbols, public visibility changes, or warnings reported by these commands.

## Intended assertion failures

Each exact run selected one test, failed its named invariant, and reported `507 filtered out`:

### S1 stale wrapper below advanced floor

Command:

```text
cargo test --lib server::state::tests::spike_s1_stale_wrapper_cannot_mutate_newer_mask -- --exact --nocapture
```

Observed assertion:

```text
S1: stale generation must not mutate the newer generation's complete mask
left:  (2, 7)
right: (2, 5)
```

Current `PendingSyncState::arm(1, revalidate)` ORs a stale companion into the generation-2 mask. The generation stays `2`, but its mask is mutated by stale work.

### S2 full-mask exactly-once handoff

Command:

```text
cargo test --lib server::state::tests::spike_s2_finish_transfers_full_mask_to_one_successor -- --exact --nocapture
```

Observed assertion:

```text
S2: completion must transfer the complete mask to exactly one successor
left:  (false, 7)
right: (true, 0)
```

`finish_indexing` releases ownership while the complete `0b111` mask remains pending. Current state has no representation for a successor permit owning that mask.

### S3 hydration zero work before ownership

Command:

```text
cargo test --lib tools::lifecycle::tests::spike_s3_hydration_does_no_db_work_before_ownership -- --exact --nocapture
```

Observed assertion:

```text
S3: hydration must not reach the pre-DB boundary before ownership
```

A private oneshot reference-transition helper reproducing the production preamble signalled the pre-DB boundary even though `try_start_indexing` returned false. No DB was opened. The replacement plan must bind this assertion to a private production admission helper or private collaborator so GREEN cannot pass by changing only test code.

### S4 startup release/request arbitration

Command:

```text
cargo test --lib server::state::tests::spike_s4_startup_release_request_has_one_executor -- --exact --nocapture
```

Observed assertion:

```text
S4: startup request and release must select exactly one executor
left:  0
right: 1
```

The deterministic order was: startup claim fails, current owner releases and observes no pending work, then startup sets pending. Neither side owns execution. One coordinator request removes this try-then-set gap.

## Actionable pivot and conditional replacement-plan slices

Each slice must remain at most two production files, four deterministic scenarios, and 110 minutes:

1. Stage compatibility decision: approve a permit-bearing API/semver break, reduce ownership-mutator visibility, or defer Option A
2. `state.rs`: coordinator types, token, permit, mask, request/complete transitions, valid-only completion timestamp, and S1/S2/S4 RED-to-GREEN
3. `state.rs` plus compatibility tests: stable observers/publishers and the approved ownership API; remove any proposed tokenless legacy owner bridge
4. `state.rs` plus `lifecycle.rs`: coherent generation install and cancellation-aware hydration permit; bind S3 to the private production admission seam
5. `state.rs` plus `lifecycle.rs`: whole-mask completion handoff; remove split consumption, re-arm, and bounded double-drain authority
6. `state.rs` plus `write.rs`: migrate index/sync callers and preserve queued JSON response without producer reacquire
7. `state.rs` plus `ipc_server.rs`: migrate startup and watcher callers to typed permits and one request linearization
8. Tests and closure: prove zero tokenless ownership callers, approved compatibility, monitoring, and full release-unit rollback

Dependency order is `1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8`. No implementation task may be harvested until decision slice 1 is approved and freshly reviewed.

## Monitoring and rollback obligations

The replacement plan must verify:

* zero stale-token acquisition, publication, or completion
* zero companion-only pending masks
* zero DB/file work before hydration permit
* exactly one executor after startup/release arbitration
* zero stale completion clearing a newer owner
* valid completion updates `last_indexed_at` exactly once; stale completion does not update it
* zero producer reacquire and bounded-drain exhaustion paths
* unchanged queued CLI/MCP response and the explicitly approved Rust API compatibility level

Rollback is a full coordinator release-unit revert followed by daemon restart. Partial rollback after caller migration is forbidden. There is no schema or data rollback.

## Remaining unknowns

* The S3 proof is a compiling private reference transition, not yet a production-coupled seam; the replacement plan must couple it without making anything public.
* Public ownership completion cannot remain tokenless while guaranteeing stale-completion rejection; Stage must choose the API/semver path.
* Split takes cannot be safely associated with an owner without a returned permit and complete mask.
* Standalone companion-setter behavior requires a compatibility decision because companion-only staging invalidates Option A's invariant.
* Exact successor selection policy between queued owner kinds remains a Stage planning decision, but it must be deterministic and cannot introduce a second queue.

## Recommendation

**PIVOT to a Stage-owned public compatibility decision before any replacement implementation plan. Confidence: high.**

All four required RED shapes compiled and failed only their intended deterministic assertions, so the current multi-authority defects and the private harness locations are proven. The spike rejects the assumption that every public ownership method can preserve its signature while also enforcing exact sequenced completion and whole-mask ownership. Option A is internally viable, but planning must pivot through an explicit permit-bearing API, visibility, and semver decision.

Stage must independently assess this evidence, author `docs/decisions/2026-08-01-post-105-sync-coordinator-spike-findings.md` with a PIVOT disposition, and decide whether to approve a permit-bearing compatibility change or defer Option A. Only an approved decision may seed a fresh hardened plan and zero-P0/P1 review. No blocked status changes before those gates.
