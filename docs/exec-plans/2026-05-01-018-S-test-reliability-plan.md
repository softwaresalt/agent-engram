---
title: "018-S Test Reliability and CozoDB Concurrent Stability"
type: impl-plan
date: 2026-05-01
status: reviewed
feature_id: "036-F"
shipment_id: "018-S"
tasks:
  - "036.001-T"
  - "036.002-T"
  - "036.003-T"
source_ref: "Shipment 018-S stash entries"
tags:
  - test-reliability
  - cozodb
  - flaky-tests
  - concurrency
---

# Implementation Plan: Test Reliability and CozoDB Concurrent Stability

## Primary Objective

Fix three test reliability issues that cause non-deterministic CI failures:

1. CozoDB SQLite concurrent-open panic (production safety)
2. Flaky policy-denied metrics assertion (test isolation)
3. Non-deterministic concurrent indexing test (test design)

## Constitution Check

| Principle | Compliance | Notes |
|---|---|---|
| I. Safety-First Rust | ✅ | Replaces `unwrap()`-inducing panic path with proper error propagation |
| II. Test-First | ✅ | Characterization tests already exist; fixes make them deterministic |
| III. Workspace Isolation | ✅ | No filesystem changes outside workspace |
| VII. Destructive Approval | N/A | No destructive operations |

## Requires plan hardening

no

---

## Task 036.001-T: Fix CozoDB SQLite Concurrent-Open Panic (U015-FLK1)

### Problem

`cozo::DbInstance::new("sqlite", path, ...)` panics via internal `unwrap()` when
multiple daemon processes attempt to open the same SQLite file concurrently. This
surfaces in multi-process test scenarios and potentially in production when a
stale lockfile allows two daemon instances.

The current code at `src/db/cozo_backend/mod.rs:92` calls:

```rust
let db = cozo::DbInstance::new("sqlite", db_path_str, Default::default())
    .map_err(|e| map_db_err(format!("cannot open CozoDB SQLite store: {e}")))?;
```

The `DbInstance::new` call returns `Result`, but internally CozoDB 0.7.x may
panic on SQLite lock contention rather than returning `Err`.

### Approach

**Option A (implemented): Add process-level file lock before opening CozoDB.**

Wrap the `DbInstance::new` call with an advisory file lock (`fd-lock`) on a
`.lock` sidecar file next to the database. This serializes concurrent open
attempts at the process level, preventing the internal panic.

Steps:

1. Add `fd-lock` dependency (already used elsewhere in the workspace)
2. Before `DbInstance::new`, acquire an exclusive lock on `{db_dir}/engram.db.lock`
3. Release the lock immediately after `DbInstance::new` returns (lock held during
   open only; CozoDB's own SQLite WAL handles concurrent access once the handle is open)
4. No external timeout is applied — the OS advisory lock is fast to acquire once
   any peer finishes opening, and wrapping `spawn_blocking` in a timeout would leave
   the blocking thread running after the caller received an error (resource leak with
   no recovery path)

**Option B (deferred): Upgrade cozo to 0.8+ if/when it fixes the internal panic.**

Not viable today — cozo 0.8 is not released and 0.7.6 is the latest stable.

### Files Modified

| File | Change |
|---|---|
| `Cargo.toml` | Add `fd-lock` dependency |
| `src/db/cozo_backend/mod.rs` | Add lock acquisition in `connect_db` via `spawn_blocking`; lock released after open |

### Acceptance Criteria

- [ ] Two concurrent `connect_db` calls to the same path do NOT panic
- [ ] Second caller receives `DatabaseError` with "locked by another process" message
- [ ] Lock file is released when `CozoDb` is dropped
- [ ] Existing single-process tests continue to pass unchanged
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` passes

### Subtask Decomposition

| Subtask | Scope | Est. |
|---|---|---|
| 036.001.001-ST | Add `fd-lock` dep and lock acquisition logic in `connect_db` | 1h |
| 036.001.002-ST | Add unit test for concurrent-open error path | 0.5h |
| 036.001.003-ST | Verify integration tests pass with lock in place | 0.5h |

---

## Task 036.002-T: Stabilize Flaky c018_06 Policy Denied Metrics Test

### Problem

`c018_06_policy_denied_call_records_metrics_with_denied_outcome` in
`tests/contract/atomic_policy_snapshot_test.rs` uses a global static
`RECENT_EVENTS` ledger shared across all tests in the process. When tests run
in parallel, a concurrent test may:

1. Insert its own denied event between `clear_recent_events()` and
   `recent_events()` assertion
2. Clear events written by c018_06 before the assertion runs

The `metrics::clear_recent_events()` call at line 265 is a process-wide reset
that races with any other test using the same static.

### Approach

**Isolate the assertion by filtering on a unique discriminator** rather than
relying on a clean global ledger.

The test already filters by `tool_name == "list_symbols"` and
`outcome == "denied"`. Add a unique `_test_nonce` field to the request metadata
that flows through to the `UsageEvent`, then filter on that nonce in the
assertion. This eliminates sensitivity to concurrent test activity.

Alternatively (simpler): use `#[serial]` from the `serial_test` crate on the
two metrics-sensitive tests (c018_06, c018_07) to prevent parallel execution.

**Chosen approach: `#[serial]`** — minimal code change, no production code
modification, directly addresses the root cause (shared mutable global state in
tests).

### Steps

1. Add `serial_test = "3"` as a dev-dependency in `Cargo.toml`
2. Add `use serial_test::serial;` import in `atomic_policy_snapshot_test.rs`
3. Add `#[serial]` attribute to `c018_06_policy_denied_call_records_metrics_with_denied_outcome`
4. Add `#[serial]` attribute to `c018_07_denied_metrics_event_carries_agent_role`

### Files Modified

| File | Change |
|---|---|
| `Cargo.toml` | Add `serial_test = "3"` to `[dev-dependencies]` |
| `tests/contract/atomic_policy_snapshot_test.rs` | Add `#[serial]` to c018_06 and c018_07 |

### Acceptance Criteria

- [ ] `cargo test c018_06` passes deterministically across 50 consecutive runs
- [ ] `cargo test c018_07` passes deterministically across 50 consecutive runs
- [ ] No other test requires modification
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` passes

### Subtask Decomposition

| Subtask | Scope | Est. |
|---|---|---|
| 036.002.001-ST | Add serial_test dep + #[serial] annotations | 0.5h |
| 036.002.002-ST | Run repeated test validation (50 iterations) | 0.5h |

---

## Task 036.003-T: Make s_cs4 Concurrent Indexing Test Deterministic

### Problem

`s_cs4_concurrent_indexing_serialised_by_in_progress_flag` in
`tests/integration/concurrent_sessions_test.rs` issues two concurrent
`index_workspace` calls expecting one to fail with `IndexInProgress` (7003).
However, the test workspace created by `DaemonHarness` is nearly empty (only a
`.git/HEAD` file), so indexing completes almost instantly. The second
`index_workspace` call often arrives after the first has already finished,
resulting in both succeeding.

The test currently handles this with a fallback assertion (`success_count == 2`
at line 383), making it non-deterministic — it tests different behavior on each
run.

### Approach

**Seed the workspace with enough indexable content so that indexing takes
measurably longer than the IPC round-trip**, ensuring the two barrier-released
calls reliably overlap.

Steps:

1. After `DaemonHarness::spawn`, write 10–20 small `.rs` files into the
   workspace `TempDir`, each containing a struct and a function (enough for
   tree-sitter to parse). This ensures `index_workspace` has real work to do.
2. Optionally add a small `tokio::time::sleep(Duration::from_millis(50))` inside
   the barrier wait to reduce platform timing variance.
3. Remove the `success_count == 2` fallback assertion — the test should now
   reliably produce exactly one `IndexInProgress` error.

### Files Modified

| File | Change |
|---|---|
| `tests/integration/concurrent_sessions_test.rs` | Add workspace seeding before barrier, tighten assertion |

### Acceptance Criteria

- [ ] Test reliably produces one success + one `IndexInProgress` (7003) error
- [ ] Runs deterministically across 20 consecutive iterations on CI runner
- [ ] No flaky fallback path remains in the test
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` passes

### Subtask Decomposition

| Subtask | Scope | Est. |
|---|---|---|
| 036.003.001-ST | Seed workspace with indexable files, tighten assertions | 1h |
| 036.003.002-ST | Run repeated test validation (20 iterations) | 0.5h |

---

## Dependency Graph

```text
036.001-T (CozoDB lock) — independent
036.002-T (metrics serial) — independent
036.003-T (s_cs4 seeding) — independent
```

All three tasks are independent and can be executed in any order or in parallel.

## Execution Order (Suggested)

1. **036.002-T** — smallest change, fastest validation, unblocks CI noise reduction
2. **036.003-T** — moderate change, test-only
3. **036.001-T** — production code change, needs most careful review

## Total Estimated Effort

| Task | Estimate |
|---|---|
| 036.001-T | 2h |
| 036.002-T | 1h |
| 036.003-T | 1.5h |
| **Total** | **4.5h** |

All tasks are within the 2-hour individual task limit.

---

## Plan Review

**Reviewed**: 2026-05-01
**Gate Decision**: ADVISORY

The plan is well-structured, correctly scoped, and constitutionally compliant. Two moderate gaps require attention before implementation but do not block the gate.

---

### Persona 1: Constitution Reviewer

| # | Severity | Finding |
|---|----------|---------|
| C1 | P3 | **Constitution Check table is incomplete.** Principle VI (Single Responsibility) should be addressed — two new dependencies (`fs2`, `serial_test`) are introduced. Both are justified by concrete requirements, so the check would pass, but the table should document it explicitly. |
| C2 | P3 | **Test-first nuance for 036.001-T.** The plan states "characterization tests already exist" but Task 036.001-T introduces a new error path (lock timeout returning `DatabaseError`). The subtask decomposition correctly places the test (036.001.002-ST) but does not explicitly state that the test must be written and observed to fail before the lock logic is implemented. Clarify the red-green sequence. |

**Verdict**: PASS — no constitutional violations. Findings are advisory.

---

### Persona 2: Rust Reviewer

| # | Severity | Finding |
|---|----------|---------|
| R1 | P2 | **`fs2` lacks a timed lock API.** The plan states "If the lock cannot be acquired within 5 seconds, return error." However, `fs2::FileExt::lock_exclusive()` blocks indefinitely and `try_lock_exclusive()` fails immediately with `WouldBlock`. The plan must specify the implementation strategy: either (a) a retry loop with `try_lock_exclusive()` + `std::thread::sleep` polling, or (b) wrapping the blocking call in `tokio::task::spawn_blocking` with a `tokio::time::timeout`. Option (b) is preferred for async compatibility. Without this detail, the implementer may introduce a busy-wait or an indefinite block. |
| R2 | P3 | **`fs2` version pinning.** `fs2 = "0.4"` is correct (0.4.3 is latest and last release). The crate is in maintenance mode. Consider documenting that `fd-lock` is the alternative if `fs2` ever becomes unmaintained. Low priority — no action required now. |
| R3 | P3 | **Lock file `Drop` semantics.** The plan says "On drop, the lock is automatically released." This is correct for `fs2` (dropping the `File` handle releases the advisory lock on all platforms). However, the struct should store `Option<File>` rather than bare `File` to allow explicit early release in graceful shutdown paths if needed. Minor — `Drop` behavior is sufficient for the stated requirements. |
| R4 | P2 | **`serial_test` scope for c018_07 may be unnecessary.** The compound library (`global-metrics-store-concurrent-test-isolation-2026-04-23.md`) documents that c018_07 was previously fixed by expanding the find predicate to three fields. Current code confirms c018_07 already uses `tool_name + outcome + agent_role` as a unique discriminator. Adding `#[serial]` to c018_07 is harmless but masks whether the predicate fix alone is sufficient. Recommend: apply `#[serial]` only to c018_06 (which lacks a unique discriminator), and if c018_07 still flakes in the 50-run validation, add it then. |

**Verdict**: ADVISORY — R1 needs implementation detail before the subtask is actionable.

---

### Persona 3: Scope Boundary Auditor

| # | Severity | Finding |
|---|----------|---------|
| S1 | P3 | **Task 036.003-T optional sleep is timing-dependent.** The plan mentions "optionally add a small `tokio::time::sleep(Duration::from_millis(50))`" to reduce timing variance. Timing-based synchronization is fragile across platforms. Prefer a deterministic approach: make the workspace large enough that indexing reliably exceeds the IPC round-trip, and use the barrier as the sole synchronization primitive. If the 20-iteration validation passes without the sleep, omit it. |
| S2 | P3 | **036.001-T touches production code; others are test-only.** The scope boundaries are correct and well-declared. No scope creep detected. The production change (file locking) is strictly additive and does not change the existing API surface. |

**Verdict**: PASS — all tasks stay within their declared scope.

---

### Persona 4: Learnings Researcher

| # | Severity | Finding |
|---|----------|---------|
| L1 | P2 | **Compound library has prior art for the metrics isolation problem.** `docs/compound/test-failures/global-metrics-store-concurrent-test-isolation-2026-04-23.md` documents the exact root cause and a predicate-expansion solution. The plan chooses `#[serial]` instead, which is valid but addresses the symptom (parallelism) rather than the root cause (ambiguous predicate). For c018_06, `#[serial]` is justified because there is no unique discriminator available without modifying production code. Document this rationale in the task description so the implementer understands why `#[serial]` was chosen over the predicate approach used for c018_07. |
| L2 | P3 | **Compound library confirms the CozoDB panic root cause.** `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md` confirms the upstream `unwrap()` at `cozo-0.7.6/src/storage/sqlite.rs:49`. The plan's file-lock approach is validated — it prevents the upstream panic path from being reached. The compound entry also notes that `nextest --test-threads 1` did NOT reliably fix the issue, which supports the file-lock approach. |
| L3 | P3 | **RwLock TOCTOU compound learning is not directly relevant** but demonstrates this project's pattern of fixing concurrency issues with structural guarantees rather than timing assumptions. The plan's approach for 036.001-T (structural file lock) is consistent with this established pattern. |

**Verdict**: ADVISORY — L1 recommends documenting the rationale for choosing `#[serial]` over predicate expansion.

---

### Summary of Recommendations

1. **(R1, P2)** Specify how the 5-second lock timeout will be implemented given `fs2`'s API. Recommend `tokio::task::spawn_blocking` + `tokio::time::timeout` wrapping `lock_exclusive()`.
2. **(R4, P2)** Consider applying `#[serial]` only to c018_06 initially; validate c018_07 independently.
3. **(L1, P2)** Add a rationale note to Task 036.002-T explaining why `#[serial]` was chosen over predicate expansion for c018_06.
4. **(S1, P3)** Prefer deterministic workspace sizing over timing-based sleeps in 036.003-T.
5. **(C2, P3)** Clarify red-green TDD sequence for the new lock-timeout error path in 036.001-T.

### Gate Outcome

**ADVISORY** — No P0/P1 findings. Three P2 findings require minor plan refinement but do not block harvest. The plan may proceed to implementation with the recommendations above addressed inline during execution.
