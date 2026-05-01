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
| `tests/contract/atomic_policy_snapshot_test.rs` | Added `use serial_test::serial;` + `#[serial]` on c018_06; removed `clear_recent_events()` from c018_07 (self-isolates via unique 3-field predicate) |
| `tests/integration/concurrent_sessions_test.rs` | Seeded workspace with 20 `.rs` files; removed fallback `else` branch; tightened assertion to exactly 1 IndexInProgress error |
| `src/db/cozo_backend/mod.rs` | Wrapped `cozo::DbInstance::new` in fd-lock advisory lock via `spawn_blocking` + `try_write()` with 5s deadline; added unit test `concurrent_connect_db_does_not_panic` |

## Decisions and Rationale

### 036.002-T: `#[serial]` applied to c018_06 only; c018_07 self-isolates
c018_06 lacks a unique discriminator and calls `clear_recent_events()`, so
`#[serial]` is required. c018_07 uses a unique 3-field predicate (`tool_name +
outcome + agent_role`) and no longer calls `clear_recent_events()`, so it runs
in parallel safely without serialisation.

### 036.003-T: No timing sleep added
Per plan review advisory S1: deterministic workspace sizing is preferred over
timing-based sleeps. 20 seeded `.rs` files (each containing a struct + function)
are sufficient to make indexing reliably outlast the IPC round-trip.

### 036.001-T: fd-lock with try_write + 5s deadline
`fd-lock = "4"` is already a project dependency (used in `src/daemon/lockfile.rs`).
No new dependency was needed. Used `fd_lock::RwLock::try_write()` in a 50 ms
polling loop with a 5-second deadline inside `spawn_blocking` — the task itself
enforces the timeout (no dangling background thread). Lock held only during
`DbInstance::new`, then released — CozoDB's own WAL handles subsequent concurrent
access.

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
