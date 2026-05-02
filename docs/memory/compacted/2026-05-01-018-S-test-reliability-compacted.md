---
type: compacted-memory
date: 2026-05-01
shipment: 018-S
feature: 036-F
pr: 66
merge_sha: d9209db15df617d0a425cc633451b7a3df19edf5
closure_pr: 67
status: shipped
sources:
  - docs/memory/2026-05-01/018-S-test-reliability-memory.md
  - docs/memory/2026-05-01/018-S-ship-session-memory.md
  - docs/memory/2026-05-01/018-S-post-merge-closure-memory.md
---

# Compacted: 018-S — Test Reliability and CozoDB Concurrent Stability

## Outcome

Shipped via PR #66 (merge `d9209db`). All 3 tasks done. Post-merge closure PR #67 created.

## Items Shipped

| ID | Title | Key Commit |
|---|---|---|
| 036.001-T | Fix CozoDB SQLite concurrent-open panic (U015-FLK1) | ae861d8 |
| 036.002-T | Stabilize flaky c018_06 policy-denied metrics test | 20a00b1 |
| 036.003-T | Make s_cs4 concurrent indexing test deterministic | 8ec3941 |

## Files Modified

| File | Change |
|---|---|
| `src/db/cozo_backend/mod.rs` | fd-lock advisory lock around `DbInstance::new`; `try_write()` + 50ms poll + 5s deadline in `spawn_blocking` |
| `tests/contract/atomic_policy_snapshot_test.rs` | `#[serial]` on c018_06 only; removed `clear_recent_events()` from c018_07 (self-isolating 3-field predicate) |
| `tests/integration/concurrent_sessions_test.rs` | 20 seeded `.rs` files; removed fallback `else`; 1 IndexInProgress assertion |
| `Cargo.toml` | Added `fd-lock` prod dep (was already in `src/daemon/lockfile.rs`); `serial_test = "3"` dev dep |
| `.github/workflows/ci.yml` | Retained `continue-on-error: true` on test step (intra-process schema bootstrap race; stash `C4E8F2A1`) |
| `.gitignore` | Added `**/*.db.lock` for fd-lock sidecar |
| `docs/architecture.md` | Added fd-lock advisory lock design note to Embedded Database section |

## Decisions

1. **fd-lock approach** (036.001-T): `fd_lock::RwLock::try_write()` + 50ms polling + 5s Instant deadline inside `spawn_blocking`. Lock held during `DbInstance::new` only; released before return. No external timeout wrapper (would leave blocking thread dangling after error).

2. **`#[serial]` on c018_06 only** (036.002-T): c018_07 already self-isolates via unique 3-field predicate (`tool_name + outcome + agent_role`); applying `#[serial]` to c018_07 was unnecessary. Only c018_06 needs serialization because it calls `clear_recent_events()` (process-global reset).

3. **Seed 20 files, no sleep** (036.003-T): 20 small `.rs` files (each with a struct + function) make indexing reliably outlast the IPC round-trip. Timing sleeps rejected per plan-review advisory.

4. **CI `continue-on-error` retained**: The fd-lock fixes the multi-process U015-FLK1 panic. An intra-process schema-bootstrap race was discovered post-ship (parallel tests hitting SQLITE_BUSY after lock release). `continue-on-error: true` is retained until stash `C4E8F2A1` (extend fd-lock scope) or `1092D3D6` (cozo 0.8+ upgrade) resolves the remaining race.

## Failed Approaches

- `tokio::time::timeout` wrapper around fd-lock acquire: rejected — would leave the `spawn_blocking` thread running after the caller received an `Err` (resource leak).
- `#[serial]` on c018_07: unnecessary — 3-field predicate already self-isolates.

## Quality Gates (all green)

`cargo fmt` ✅ | `cargo clippy -- -D warnings -D clippy::pedantic` ✅ | `cargo test` ✅

## Source Stash IDs

| Stash | Task |
|---|---|
| 685B097A | 036.001-T (U015-FLK1 bug) |
| 5B1EB1DF | 036.002-T (flaky c018_06) |
| 02E87E6E | 036.003-T (s_cs4 determinism) |

## Follow-up Stash

- `1092D3D6` — Upgrade cozo 0.8+, remove fd-lock workaround when upstream fix ships
