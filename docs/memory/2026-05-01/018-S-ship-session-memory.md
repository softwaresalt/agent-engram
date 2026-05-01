---
title: "018-S Ship Session Memory"
date: 2026-05-01
shipment: 018-S
feature: 036-F
branch: staging/018-S-test-reliability
pr: 66
status: pr-ready
tasks_completed:
  - 036.002-T
  - 036.003-T
  - 036.001-T
---

# Ship Session: 018-S — Test Reliability and CozoDB Concurrent Stability

## Items Completed

| ID | Title | Commit |
|---|---|---|
| 036.002-T | Stabilize flaky c018_06 policy-denied metrics test | 20a00b1 |
| 036.003-T | Make s_cs4 concurrent indexing test deterministic | 8ec3941 |
| 036.001-T | Fix CozoDB SQLite concurrent-open panic (U015-FLK1) | ae861d8 |
| 036-F | Test reliability and CozoDB concurrent stability | — |

Fmt fix commit: 5ef0d26
Backlog state commit: 04da4f1

## Files Modified

| File | Change |
|---|---|
| `Cargo.toml` | Added `serial_test = "3"` to `[dev-dependencies]` |
| `tests/contract/atomic_policy_snapshot_test.rs` | Added `use serial_test::serial;` + `#[serial]` on c018_06 |
| `tests/integration/concurrent_sessions_test.rs` | Seeded workspace with 20 `.rs` files; removed fallback `else` branch; tightened assertion to exactly 1 IndexInProgress error |
| `src/db/cozo_backend/mod.rs` | Wrapped `cozo::DbInstance::new` in fd-lock advisory lock via `spawn_blocking` + 5s timeout; added unit test `concurrent_connect_db_does_not_panic` |

## Decisions and Rationale

### 036.002-T: `#[serial]` applied to c018_06 only (not c018_07)
Per plan review advisory R4: c018_07 already has a 3-field predicate
(`tool_name + outcome + agent_role`) that self-isolates from concurrent tests.
c018_06 lacks a unique discriminator and relies on the process-global
`RECENT_EVENTS` ledger, so `#[serial]` is the correct and minimal fix.
Rationale documented in test doc comment per plan review L1 advisory.

### 036.003-T: No timing sleep added
Per plan review advisory S1: deterministic workspace sizing is preferred over
timing-based sleeps. 20 seeded `.rs` files (each containing a struct + function)
are sufficient to make indexing reliably outlast the IPC round-trip.

### 036.001-T: fd-lock instead of fs2
`fd-lock = "4"` is already a project dependency (used in `src/daemon/lockfile.rs`).
No new dependency was needed. The operator note about checking for a cozo upgrade
was considered — cozo 0.7.6 remains the latest stable; Option A (file lock) proceeded.
Used `fd_lock::RwLock::write()` inside `spawn_blocking` + `tokio::time::timeout(5s)`.
Lock is held only during `DbInstance::new`, then released — CozoDB's own WAL handles
subsequent concurrent access.

## Quality Gates

| Gate | Status |
|---|---|
| `cargo fmt --all -- --check` | ✅ Pass |
| `cargo clippy -- -D warnings -D clippy::pedantic` | ✅ Pass |
| `cargo test --lib` | ✅ 87 passed |
| `cargo test --test contract_atomic_policy_snapshot` | ✅ 7 passed |
| `cargo test --test integration_cozo_crud` | ✅ 11 passed |
| Review gate | ✅ No P0/P1 findings (reviewer false-positive on fd-lock crate name clarified) |

## Branch State

Branch: `staging/018-S-test-reliability`
PR: #66 (open, pushed to origin)
Status: All tasks done, full quality gates pass, awaiting operator merge approval.

## Next Steps

1. Operator reviews and approves PR #66
2. Merge to `main` (merge commit, not squash/rebase)
3. Run post-merge closure: backlog archival, knowledge graduation

## Blocked Conditions

None.
