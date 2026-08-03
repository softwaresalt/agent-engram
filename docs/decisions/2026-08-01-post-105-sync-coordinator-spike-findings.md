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
tags: ["concurrency", "pending-sync", "compatibility", "single-authority", "pivot"]
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

### Assumptions and fail-closed treatment

| ID | Assumption | Confidence and treatment |
|---|---|---|
| A1 | There is no supported downstream Rust-library consumer outside this repository. | High: `publish = false`, binary distribution, and no repository contract naming a supported Rust API. If a supported downstream contract is produced before implementation, stop before source mutation and replace this decision with strategy B; do not silently retain tokenless methods. |
| A2 | The Phase 5C caller inventory is exhaustive for the examined source. | High: targeted recursive inspection covered all `.rs` files under `src/` and `tests/`. Ship repeats the zero-legacy-caller inventory before visibility reduction. |
| A3 | One coalesced successor may execute queued routine/revalidate/backfill work regardless of which producer requested it. | High: current queued CLI/MCP semantics promise coalescing, not producer-specific execution. The successor permit is normalized to `OwnerKind::Sync`. |
| A4 | No schema or persisted-data migration is required. | High: the authority is in-memory process state. Any discovered persistence change is a stop condition and requires a new hardened plan. |

The decision is not open: the current facts select strategy A. The fail-closed clauses address changed evidence, not unresolved current ambiguity.

## Caller and visibility decision

### Production migration inventory

| Surface | Current callers | Decision |
|---|---|---|
| Observe indexing | `write.rs:32` | Keep a read-only observer, reduce to crate-private unless a stable public behavior requires it. |
| Claim/complete index or sync | `write.rs:160,268,470` | Migrate to a non-cloneable `OwnerPermit`; remove tokenless completion. |
| Queue/reacquire | `write.rs:297-304` | Replace with one request; preserve exact queued JSON; remove producer reacquire. |
| Generation install | `lifecycle.rs:187` | Install coherent binding/cancellation/floor and return opaque `GenerationToken`. |
| Hydration | `lifecycle.rs:250-402` | Cancellation-aware permit wait before DB/file I/O; complete the exact permit on every exit. |
| Drain | `lifecycle.rs:420-494` | Consume only a transferred full mask; remove split takes, re-arm, bounded loop, and double drain. |
| Watchers | `ipc_server.rs:684,1135` | Use typed watcher permits and exact completion. |
| Startup | `ipc_server.rs:1210-1224` | Replace try-then-set with one token-qualified request and one completion outcome. |
| Health timestamp | `doctor.rs:115-135` | Preserve observer behavior; only current-permit completion updates it. |

### Repository-test migration

`tests/contract/read_test.rs`, `tests/contract/write_test.rs`, and `tests/integration/indexing_resilience_test.rs` directly call tokenless methods today. They are repository tests, not evidence of a published library contract. Migrate observable assertions to public tool behavior. Move authority invariants to co-located private tests in `state.rs`, `lifecycle.rs`, and `ipc_server.rs`. No `pub`, feature-gated public, or test-only public ownership seam is permitted.

## Selected API strategy: A

Use one private `std::sync::Mutex<SyncCoordinator>` and one private `tokio::sync::Notify`. The notify is only a wake mechanism, never a queue. All identity-bearing types and mutators are `pub(crate)` at most, with private fields:

```text
GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerKind { Index, Sync, Hydration, Startup, Watcher }
OwnerPermit { generation, sequence, kind, work_mask }  // non-Clone

request(token, work_mask, owner_kind)
  -> Acquired(OwnerPermit) | Queued | Stale

complete(owner_permit)
  -> Transferred(OwnerPermit) | Released | Stale
```

`SyncCoordinator` owns the generation floor, monotonically increasing owner sequence, current owner, and zero-or-one complete pending `WorkMask`. Busy sync/startup/watcher producers merge into that one mask. Hydration registers `Notified` before retrying but does not create queued work. On exact completion with pending work, the completing driver receives exactly one new `OwnerKind::Sync` permit containing the full mask; pending becomes empty in the same critical section. With no pending work, completion releases and notifies waiters. Requests arriving after release acquire directly. Stale or mismatched permits make no mutation.

`complete` consumes a non-cloneable permit, validates and transitions under the coordinator mutex, drops that guard, then records `last_indexed_at` exactly once through a non-awaiting private timestamp lock before returning a successful outcome. A stale/mismatched completion never changes the timestamp. No standard mutex guard crosses `.await`.

## Protected invariants

1. One coordinator is the only authority for generation floor, owner identity/permit, complete pending mask, and hydration/drain handoff.
2. A stale token cannot acquire, queue, replace, clear, or relabel current work.
3. A stale, duplicated, wrong-kind, wrong-generation, or wrong-sequence permit cannot mutate owner, mask, notification, or timestamp state.
4. A non-empty pending mask is one complete `WorkMask`; companion-only state is impossible.
5. Exact completion transfers the full mask to exactly one successor or releases; never both and never neither.
6. Hydration performs zero DB or file I/O until it holds its permit.
7. Startup/release arbitration selects exactly one executor.
8. Every successful current-permit completion records `last_indexed_at` exactly once; rejected completion records it zero times.
9. No mutex guard crosses `.await`.
10. No second queue, split drain, double drain, producer reacquire, timing sleep, unsafe code, or public test-only seam is introduced.
11. CLI/MCP methods, wire/schema/persistence formats, and the exact queued sync JSON remain unchanged.

## Compatibility and semver disposition

Strategy A is an internal Rust source migration, not a public semver break: the package is non-published and distributed as a binary. No version-major bump or deprecation bridge is required. The release unit must preserve CLI commands, MCP tool schemas, JSON status/message text, health meaning, startup behavior, and persisted files. Release notes may describe a concurrency correctness fix but must not advertise a new Rust API.

Strategy B is rejected because it would create and support a public permit-bearing Rust API without product evidence requiring one. Strategy C is rejected because all critical callers are migratable internal code and private deterministic harnesses are proven. If the facts change, implementation fails closed before source edits rather than shipping an unsafe compatibility adapter.

## Replacement-plan input and disposition

The old residual plan and tasks assume signature preservation, `Reacquired`, split claims, and bounded double-drain behavior. Those scopes are not accurate and must be superseded, not reused. The replacement plan must pair RED before GREEN, keep every task at `<=110 minutes`, `<=2` production files, `<5` production functions, and `<=4` scenarios, and decompose `state.rs`, `lifecycle.rs`, `write.rs`, and `ipc_server.rs` in a strict dependency chain.

Final disposition: **PIVOT to strategy A with high confidence.** Implementation remains blocked until Ship closes `106-S`, Stage performs the documented fail-closed requeue transaction, and the fresh plan has a zero-P0/P1 PASS.
