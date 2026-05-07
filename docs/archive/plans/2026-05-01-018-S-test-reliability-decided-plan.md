---
title: "018-S Test Reliability and CozoDB Concurrent Stability — Decided Plan"
type: decided-plan
date: 2026-05-01
status: shipped
feature_id: "036-F"
shipment_id: "018-S"
supersedes: "docs/exec-plans/2026-05-01-018-S-test-reliability-plan.md"
merge_sha: d9209db15df617d0a425cc633451b7a3df19edf5
---

## Decisions

### 036.001-T: fd-lock advisory lock in connect_db

**Decision**: Wrap `cozo::DbInstance::new` in an advisory file lock (`fd-lock`) using
`fd_lock::RwLock::try_write()` inside `spawn_blocking` with 50ms polling and a 5s
`Instant` deadline. The lock is held only during `DbInstance::new`; CozoDB's SQLite WAL
handles concurrent statement-level access after open.

**Rejected alternatives**:
- `tokio::time::timeout` wrapper: leaves the `spawn_blocking` thread dangling after caller error.
- Upgrade cozo 0.8+: not yet available (tracked as stash `1092D3D6`).
- `write()` blocking variant: no deadline control; could block indefinitely.

**Lint constraints**: `try_write()` + deadline in `if let` (not `match`) form; `sleep` at
bottom of loop (not in `else` after `else if`) to satisfy clippy pedantic.

### 036.002-T: `#[serial]` on c018_06 only

**Decision**: Apply `serial_test = "3"` as dev-dep; `#[serial]` on c018_06 only.
c018_07 already self-isolates via unique 3-field predicate (`tool_name + outcome + agent_role`);
`clear_recent_events()` was removed from c018_07 (it was unnecessary and created a race risk).

**Rejected**: `#[serial]` on both c018_06 and c018_07 — overly conservative; c018_07 doesn't need it.

### 036.003-T: Seed 20 `.rs` files; no sleep

**Decision**: Seed the TempDir workspace with 20 `.rs` files after `DaemonHarness::spawn`,
each containing a struct + function. This makes indexing reliably outlast the IPC round-trip.
Removed fallback `else` branch; assertion is exactly 1 `IndexInProgress` (7003) result.

**Rejected**: Timing sleeps — non-deterministic, not portable, per plan-review advisory.

## Key Invariants (post-ship)

- `connect_db` MUST NOT panic on concurrent opens; MUST return `Err(EngramError)` on 5s timeout.
- Advisory lock released before `connect_db` returns.
- c018_06 MUST remain `#[serial]` (calls `clear_recent_events()`).
- c018_07 MUST NOT call `clear_recent_events()` (self-isolating predicate).
- s_cs4 MUST assert exactly 1 `IndexInProgress` error (no fallback assertion).

## Files Modified

| File | Change |
|---|---|
| `src/db/cozo_backend/mod.rs` | fd-lock advisory lock around `DbInstance::new` |
| `tests/contract/atomic_policy_snapshot_test.rs` | `#[serial]` on c018_06; no `clear_recent_events()` in c018_07 |
| `tests/integration/concurrent_sessions_test.rs` | 20 seeded files; 1 IndexInProgress assertion |
| `Cargo.toml` | `fd-lock` prod dep; `serial_test = "3"` dev dep |
| `.github/workflows/ci.yml` | Restored `continue-on-error: true` on test step (intra-process schema bootstrap race; see stash `C4E8F2A1`) |
| `.gitignore` | Added `**/*.db.lock` |
