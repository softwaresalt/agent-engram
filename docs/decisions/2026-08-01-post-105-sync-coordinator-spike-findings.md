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
| Claim/complete index or sync | `write.rs:160,268,470` | Migrate to an RAII `OwnerPermit`; explicit completion disarms, while cancellation/panic Drop recovers exact ownership. Remove tokenless completion. |
| Queue/reacquire | `write.rs:297-304` | Replace with one request; preserve exact queued JSON; remove producer reacquire. |
| Generation install | `lifecycle.rs:187` | Install coherent binding/cancellation/floor and return opaque `GenerationToken`. |
| Hydration | `lifecycle.rs:250-402` | Cancellation-aware wait before DB/file I/O; pre-acquisition cancellation exits without a permit, normal/handled post-acquisition exits complete/disarm, and cancellation/panic uses Drop. |
| Drain | `lifecycle.rs:420-494` | Consume only a transferred full mask; remove split takes, re-arm, bounded loop, and double drain. |
| Watchers | `ipc_server.rs:684,1135` | Use typed RAII watcher permits; exact completion disarms and cancellation/panic Drop recovers. |
| Startup | `ipc_server.rs:1210-1224` | Replace try-then-set with one token-qualified guarded request and one completion-or-Drop terminal transition. |
| Health timestamp | `doctor.rs:115-135` | Preserve observer behavior; only current-permit completion updates it. |

### Repository-test migration

`tests/contract/read_test.rs`, `tests/contract/write_test.rs`, and `tests/integration/indexing_resilience_test.rs` directly call tokenless methods today. They are repository tests, not evidence of a published library contract. Migrate observable assertions to public tool behavior. Move authority invariants to co-located private tests in `state.rs`, `lifecycle.rs`, and `ipc_server.rs`. No `pub`, feature-gated public, or test-only public ownership seam is permitted.

## PR #316 P1 remediation decision

The exact unresolved Copilot comments at PR head `897406cc79896eef90d3a44645804d691c3aff96` are accepted as valid P1 findings:

1. `discussion_r3700752674`: generation install could stale an active permit while leaving its owner record able to block the new generation.
2. `discussion_r3700752695`: a merely non-cloneable permit is not cancellation-safe because task cancellation or panic can drop it before explicit completion.

The selected remediation keeps strategy A and strengthens it with an evidence-backed RAII permit. `AppState` is already shared as `Arc<AppState>` across spawned work, `state.rs` already uses recoverable synchronous mutex critical sections, and `tokio::sync::Notify::notify_one` is synchronous. `AppState` therefore owns an `Arc<CoordinatorCell>` and each `OwnerPermit` owns a clone of that cell. `Drop` can perform the abandonment transition without awaiting, spawning, or caller cleanup. No public type or test seam is introduced.

The exactness claim is scoped to in-process coordinator ownership: every `WorkMask` bit is in exactly one of current owner or pending, and only one current-generation permit may launch a driver. An old driver retains its immutable old binding snapshot and is cancelled on rebind; it cannot reread or operate against the new binding. A partially completed idempotent old-binding scan may finish or be retried on the new binding, but it cannot complete, clear, or launch work in the new generation. The design does not claim exactly-once external side effects or RAII execution after process abort.

## Selected API strategy: A

Use one crate-private, `Arc`-owned coordinator cell. All fields remain private:

```text
CoordinatorCell { state: Mutex<SyncCoordinator>, notify: Notify }
SyncCoordinator { floor, binding_identity, next_sequence, owner, pending, last_indexed_at }
GenerationToken(u64)
WorkMask { routine, revalidate, backfill_python }
OwnerIdentity { generation, sequence, kind }
OwnerPermit { cell, identity, work_mask, cleanup_armed }  // non-Clone, RAII
```

`request(token, work_mask, owner_kind)` still returns `Acquired(OwnerPermit) | Queued | Stale`; `complete(owner_permit)` still returns `Transferred(OwnerPermit) | Released | Stale`. `Notify` remains only a wake mechanism. A busy producer ORs one complete mask into the single pending slot. With no owner, a non-empty request atomically takes `pending OR requested` into one `Sync` permit. An empty-mask Index/Hydration request may acquire while pending remains, and its completion transfers pending. Mutex serialization prevents two requesters taking the mask.

### Atomic generation/binding advance

Prepare and capacity-check the complete new binding/cancellation tuple before mutation. Acquire existing binding write guards in documented order, then take the coordinator lock; perform no await after that. One coordinator-locked publication:

1. captures and invalidates the prior `OwnerIdentity`;
2. computes `promoted = prior_owner.work_mask OR prior_pending` from the authoritative owner record;
3. removes the old owner and publishes non-empty `promoted` as the new generation's one pending mask;
4. swaps workspace/config/cancellation ownership behind the held guards;
5. advances binding identity and generation floor together and resets hydration readiness; and
6. synchronously signals old cancellation.

It leaves no inaccessible successor owner record. After all guards release, it calls `notify_one` at most once when retirement made progress possible, then returns the new token. The rule is **promote all three set bits to the newest coherent binding**. Empty masks invent no work; obsolete non-coalescible old-binding operations are cancelled. The old permit retains only its old binding identity. Later finish returns `Stale`; later Drop finds no exact identity. Both mutate nothing.

### Explicit completion and RAII abandonment

`complete(mut permit)` validates generation/sequence/kind under the mutex. Exact completion performs one transfer-or-release transition, writes `last_indexed_at` once inside that successful transition, and disarms the old permit before destruction. Pending transfer returns one armed successor; release wakes at most one after unlock. Stale explicit completion disarms only the local guard and returns `Stale` without coordinator mutation.

`OwnerPermit::drop` is mandatory cleanup. If armed and identity still exactly matches, Drop atomically computes `owner.work_mask OR pending`, republishes the non-empty union, clears owner once, unlocks, and calls `notify_one` once. It never allocates a sequence, awaits, spawns, updates the timestamp, or panics; poison recovery uses the existing `PoisonError::into_inner` pattern. Identity mismatch, including after generation advance or replacement acquisition, is a strict no-op. The next current request is the sole possible successor and takes the union once.

Task cancellation and panic unwind therefore release live ownership automatically. Process abort does not run Drop: restart reconstructs the coordinator, bind/hydration plus offline-change detection reconciles durable state, and non-durable revalidate/backfill intent is reissued. Full release-unit rollback/restart remains the runtime-invariant response.

## Protected invariants

1. One coordinator cell is the only authority for generation/binding identity, owner identity, complete pending mask, handoff, and completion timestamp.
2. During a live process, each set `WorkMask` bit is in exactly one authoritative location: owner or pending.
3. Generation advance moves `old owner mask OR old pending` to the new pending slot while retiring old identity in the same lock transition.
4. No generation/Drop transition installs an inaccessible owner; each wakes at most one after unlocking.
5. Stale token and stale finish/Drop paths cannot mutate current owner, mask, floor, binding, notify, or timestamp.
6. Exact completion disarms Drop, transfers/releases once, and updates `last_indexed_at` once; stale completion and abandonment update it zero times.
7. Exact Drop republishes the authoritative owned mask plus pending and clears owner once; one later request can take that union once.
8. Hydration does zero DB/file I/O until ownership; pre-acquisition cancellation has no permit and no coordinator mutation.
9. Every owner uses its immutable binding snapshot; old permits cannot authorize new-binding work.
10. No mutex crosses `.await`; Drop never awaits, spawns, or panics.
11. No second queue, split/double drain, producer reacquire, sleep, unsafe, or public test seam.
12. CLI/MCP, wire/schema/persistence/config formats, and exact queued JSON remain unchanged.
13. RAII and logical exactly-once ownership are not claimed across process abort; reconciliation, reissue, and rollback are explicit.

## Compatibility and semver disposition

Strategy A is an internal Rust source migration, not a public semver break: the package is non-published and distributed as a binary. No version-major bump or deprecation bridge is required. The release unit must preserve CLI commands, MCP tool schemas, JSON status/message text, health meaning, startup behavior, and persisted files. Release notes may describe a concurrency correctness fix but must not advertise a new Rust API.

Strategy B is rejected because it would create and support a public permit-bearing Rust API without product evidence requiring one. Strategy C is rejected because all critical callers are migratable internal code and private deterministic harnesses are proven. If the facts change, implementation fails closed before source edits rather than shipping an unsafe compatibility adapter.

## Replacement-plan input and disposition

The old residual plan and tasks assume signature preservation, `Reacquired`, split claims, and bounded double-drain behavior. Those scopes are not accurate and must be superseded, not reused. The replacement plan must pair RED before GREEN, keep every task at `<=110 minutes`, `<=2` production files, `<5` production functions, and `<=4` scenarios, and decompose `state.rs`, `lifecycle.rs`, `write.rs`, and `ipc_server.rs` in a strict dependency chain. Core RED/GREEN proves mandatory RAII abandonment and successful-completion disarm; binding RED/GREEN proves active-owner-plus-pending promotion and stale finish/Drop isolation. Pre-acquisition cancellation remains a zero-permit path.

Final disposition: **PIVOT to strategy A with high confidence.** Implementation remains blocked until Ship closes `106-S`, Stage performs the documented fail-closed requeue transaction, and the fresh plan has a zero-P0/P1 PASS.
