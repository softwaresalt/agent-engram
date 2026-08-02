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
| `try_start_indexing(&self) -> bool` | Claim global owner without token or kind | Stable legacy claim adapter | One atomic current-floor request with `OwnerKind::LegacyPublic`; production callers migrate to token-qualified permits |
| `finish_indexing(&self)` | Clear global owner and update timestamp | Stable legacy completion adapter | Complete only the sequenced legacy owner in the coordinator; stale or mismatched completion cannot clear a newer owner |
| `current_sync_generation(&self) -> u64` | Observe raw generation | Stable observer | Return the coordinator floor; internal callers use opaque `GenerationToken` |
| `set_pending_sync(&self)` | Arm routine bit at current generation | Stable public adapter | Atomically request a complete routine mask at the current floor |
| `publish_pending_sync(&self, bool, bool)` | Publish routine plus companions | Stable public adapter | Atomically merge one complete `WorkMask` |
| `publish_pending_sync_and_try_reacquire(&self, bool, bool) -> bool` | Publish, then attempt producer reacquire | Stable public adapter with changed implementation | One atomic `request` maps `Acquired` to `true` and `Queued` or `Stale` to `false`; no second acquisition occurs |
| `take_pending_sync(&self) -> bool` | Clear only the routine bit | Stable legacy claim adapter, internal use forbidden | Atomically move the complete mask into `OwnerKind::LegacySplitTake`; never leave a companion-only pending mask |
| `has_pending_sync(&self) -> bool` | Peek routine bit | Stable observer | Observe whether a complete pending mask exists |
| `clear_pending_sync_for_generation(&self, u64)` | Generation-scoped queue clear | Stable legacy adapter, internal use forbidden | Convert the raw generation to a guarded cancellation/complete action that cannot clear a newer floor or owner |
| `set_pending_sync_revalidate(&self)` | Stage a revalidate companion before routine | Stable only for the documented paired-call contract | Atomically request routine plus revalidate; exact standalone companion-only behavior is intentionally not representable |
| `take_pending_sync_revalidate(&self) -> bool` | Clear one companion bit | Stable legacy claim continuation | Consume the bit from the coordinator's `LegacySplitTake` owner mask |
| `set_pending_sync_backfill_python(&self)` | Stage a backfill companion before routine | Stable only for the documented paired-call contract | Atomically request routine plus backfill; exact standalone companion-only behavior is intentionally not representable |
| `take_pending_sync_backfill_python(&self) -> bool` | Clear one companion bit | Stable legacy claim continuation | Consume the bit from the coordinator's `LegacySplitTake` owner mask |
| `begin_scan_generation(&self)` | Advance generation and replace cancellation channel | Signature-compatible lifecycle adapter | The new private install operation must publish binding, cancellation ownership, and floor together, then return an opaque token |

The split-take bridge is viable only as an owner kind inside the same coordinator. A separate extracted-mask cache would be a forbidden second authority. All in-repository production callers must stop using split takes, raw generation clears, and legacy claims before GREEN is complete.

The two standalone companion setters have no in-repository production or external-test caller. Their documented paired-call intent can be preserved by atomically adding the routine bit. Stage should explicitly record this safety-strengthening compatibility decision and deprecate companion-only sequencing.

## Production caller inventory

| Surface | Caller and line | Current role | Migration |
|---|---|---|---|
| `is_indexing` | `src/tools/write.rs:32` | `flush_state` rejection observer | Stable observer |
| `try_start_indexing` | `src/tools/write.rs:160` | Full index claim | `request(token, index mask, Index)` |
| `try_start_indexing` | `src/tools/write.rs:268` | Sync claim or queued response | `request(token, sync mask, Sync)` |
| publish/reacquire | `src/tools/write.rs:297-304` | Queued sync producer backstop | One atomic request; remove producer reacquire and separate finish/drain |
| `finish_indexing` | `src/tools/write.rs:470` | Shared index/sync finalizer | `complete(permit)` and act on transfer/release outcome |
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
| `tests/contract/read_test.rs` | `try_start_indexing` at `302`, `324`, `346` | Boolean legacy claim remains available for read-while-indexing contracts |
| `tests/contract/write_test.rs` | `try_start_indexing` at `108`, `150`, `185`; `take_pending_sync` at `159` | Index-in-progress and queued-response behavior; routine take returns true once |
| `tests/integration/indexing_resilience_test.rs` | `try_start_indexing` at `54`, `75`, `99`, `125`, `156`; `take_pending_sync` at `135`, `141`; `finish_indexing` at `157` | Read availability, queued sync, one-shot take, and post-finish release |

Co-located tests in `src/tools/lifecycle.rs` and `src/daemon/ipc_server.rs` also exercise generation clear, companion bits, producer reacquire, bounded drain, and startup helpers. They are migration tests, not justification for retaining internal legacy paths.

## Recommended authoritative API

Proceed with Option A, using one private `Mutex<SyncCoordinator>` and one private `Notify`:

```text
GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher, LegacyPublic, LegacySplitTake }
OwnerPermit { generation, sequence, kind, work_mask }

request(token, work_mask, owner_kind)
  -> Acquired(OwnerPermit) | Queued | Stale

complete(owner_permit)
  -> Transferred(OwnerPermit) | Released | Stale
```

`OwnerPermit` must be opaque and sequence-qualified. `complete` accepts only the exact permit and moves the complete pending mask to one successor or releases and notifies. Public wrappers enter this same coordinator; none stores ownership or a mask elsewhere.

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

## Actionable replacement-plan slices

Each slice must remain at most two production files, four deterministic scenarios, and 110 minutes:

1. `state.rs`: coordinator types, token, permit, mask, request/complete transitions, and S1/S2/S4 RED-to-GREEN
2. `state.rs` plus compatibility tests: public observer, legacy claim, publish, and split-take adapters; explicit companion-setter decision
3. `state.rs` plus `lifecycle.rs`: coherent generation install and cancellation-aware hydration permit; bind S3 to the private production admission seam
4. `state.rs` plus `lifecycle.rs`: whole-mask completion handoff; remove split consumption, re-arm, and bounded double-drain authority
5. `state.rs` plus `write.rs`: migrate index/sync callers and preserve queued JSON response without producer reacquire
6. `state.rs` plus `ipc_server.rs`: migrate startup and watcher callers to typed permits and one request linearization
7. Tests and closure: prove zero internal legacy callers, external compatibility, monitoring, and full release-unit rollback

Dependency order is `1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7`. Stage may split RED and GREEN milestones further but must not reorder callers ahead of the primitive and adapters.

## Monitoring and rollback obligations

The replacement plan must verify:

* zero stale-token acquisition, publication, or completion
* zero companion-only pending masks
* zero DB/file work before hydration permit
* exactly one executor after startup/release arbitration
* zero stale completion clearing a newer owner
* zero producer reacquire and bounded-drain exhaustion paths
* unchanged queued CLI/MCP response and external Rust compatibility

Rollback is a full coordinator release-unit revert followed by daemon restart. Partial rollback after caller migration is forbidden. There is no schema or data rollback.

## Remaining unknowns

* The S3 proof is a compiling private reference transition, not yet a production-coupled seam; the replacement plan must couple it without making anything public.
* The `LegacySplitTake` adapter needs focused GREEN tests proving it remains inside the coordinator and auto-releases when its mask is exhausted.
* Standalone companion-setter behavior must be documented as a safety-strengthening complete-mask request; any demand for companion-only staging requires a semver decision and would invalidate Option A's invariant.
* Exact successor selection policy between queued owner kinds remains a Stage planning decision, but it must be deterministic and cannot introduce a second queue.

## Recommendation

**PROCEED to Stage-owned findings and a fresh hardened replacement plan; do not proceed directly to implementation. Confidence: medium-high.**

All four required RED shapes compiled and failed only their intended deterministic assertions. The current code validates the single-authority assumptions, and the public signatures can map to one coordinator if split takes become a legacy owner kind and standalone companion setters atomically request a complete mask. The remaining S3 coupling and legacy-adapter details are bounded implementation-plan obligations, not reasons to widen this spike.

Stage must independently assess this evidence, author `docs/decisions/2026-08-01-post-105-sync-coordinator-spike-findings.md`, and run fresh hardening and zero-P0/P1 review before changing any blocked status.
