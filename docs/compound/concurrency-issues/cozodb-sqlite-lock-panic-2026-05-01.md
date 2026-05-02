---
title: "CozoDB 0.7.6 SQLite Unwrap Panic on Concurrent Daemon Access"
description: "cozo-0.7.6 panics with 'database is locked' when multiple processes open the same SQLite file concurrently; mitigated in 018-S via fd-lock advisory lock in connect_db"
problem_type: "upstream_bug"
category: "concurrency-issues"
component: "db/cozo_backend"
root_cause: "cozo-0.7.6/src/storage/sqlite.rs:49 calls unwrap() on SQLite open, which panics instead of returning an error when the database is locked"
resolution_type: "local_mitigation"
severity: "high"
message: "thread '...' panicked at ...: database is locked"
file_path: "src/db/cozo_backend/mod.rs"
citations:
  - "docs/closure/2026-05-01-015-s-cozodb-phase5-6-closure.md"
  - "docs/closure/2026-05-01-018-S-test-reliability-closure.md"
  - ".backlogit/archive/015-S.md"
  - ".backlogit/archive/018-S.md"
  - "src/db/cozo_backend/mod.rs — connect_db (lines 72-145)"
supersedes_workaround: "continue-on-error: true in .github/workflows/ci.yml (015-S)"
tags:
  - "cozo"
  - "sqlite"
  - "concurrency"
  - "daemon"
  - "integration-tests"
  - "CI"
  - "U015-FLK1"
  - "fd-lock"
---

## Problem

Integration tests that spawn CozoDB daemon processes fail non-deterministically with:

```
thread 'test_name' panicked at cozo-0.7.6/src/storage/sqlite.rs:49:
called `Result::unwrap()` on an `Err` value: SqliteFailure(Error { code: DatabaseBusy, extended_code: 5 }, Some("database is locked"))
```

This manifests in three scenarios:
1. **Cross-binary parallelism**: `cargo test --all-targets` runs test binaries concurrently. Each binary spawns its own daemon. If two binaries try to open the same SQLite path simultaneously, the second panics.
2. **Crash-restart tests**: Tests that kill a daemon process then immediately start a new one. The OS may not release the SQLite file lock before the new daemon opens the file.
3. **Intentional concurrent tests**: Tests designed to exercise concurrent daemon access (`s_cs1`, `s_cs4`).

The error originates in the upstream cozo crate, not project code.

## Root Cause

`cozo-0.7.6/src/storage/sqlite.rs:49` uses `unwrap()` on the SQLite open call. When SQLite returns `SQLITE_BUSY` (database locked), the `unwrap()` panics the process rather than propagating an error. This is an upstream defect (tracked as U015-FLK1).

## Resolution (018-S — local mitigation)

Applied an advisory `fd-lock` file lock around `DbInstance::new` in `connect_db`
(`src/db/cozo_backend/mod.rs`):

```rust
let lock_path = db_path.with_extension("db.lock");
let lock_file = std::fs::OpenOptions::new()
    .write(true).create(true).open(&lock_path)?;
let mut flock = fd_lock::RwLock::new(lock_file);
let deadline = Instant::now() + Duration::from_secs(5);
let _guard = loop {
    if let Ok(g) = flock.try_write() {
        break g;
    }
    if Instant::now() >= deadline {
        return Err(EngramError::DatabaseError(...));
    }
    std::thread::sleep(Duration::from_millis(50));
};
let db = DbInstance::new(...)?;
// _guard dropped here — lock released after open
```

The lock is held only for the duration of `DbInstance::new`. CozoDB's SQLite WAL handles
concurrent access after the database is open. Returns `Err(EngramError)` on 5-second timeout
rather than panicking.

**Intra-process variant (RESOLVED — 037-F, commits e85ee80 + 6f64eb7):** The fd-lock now covers
both `DbInstance::new()` AND `run_schema_bootstrap` (hold the guard through bootstrap). Additionally,
the startup auto-sync task in `ipc_server.rs` retries `sync_workspace` up to 10 times with
50 ms → 500 ms exponential back-off on SQLITE_BUSY, covering the third layer where the two resulting
handles race on actual write transactions after bootstrap completes.

`continue-on-error: true` is retained in CI due to pre-existing unrelated failures:
- `integration_graph_vector_rehydration`: startup-index timeout (timing-sensitive on slow CI runners)
- `integration_query_perf_observability`: timing-stat buckets not populated

**Previous workaround (015-S — superseded for multi-process case):** `continue-on-error: true` on
the CI test step. Retained for unrelated pre-existing failures noted above.

**Approaches that did NOT work:**
- `cargo nextest run --test-threads 1`: Serializes tests globally, but daemon cleanup between tests still leaves SQLite locks briefly held. The failure shifts non-deterministically to whichever test runs next after a daemon teardown. Different test ordering exposes different victims.
- Exclusion lists (skip specific tests): Whack-a-mole — the flaky behavior appears on any test that happens to run immediately after a daemon teardown under lock.

## Prevention

- The `fd-lock` advisory lock in `connect_db` serialises both `DbInstance::new()` AND
  `run_schema_bootstrap` across processes and threads (037-F). The startup auto-sync task in
  `ipc_server.rs` retries on SQLITE_BUSY covering post-bootstrap write races. When upgrading
  cozo beyond 0.7.x, verify the new version handles `SQLITE_BUSY` gracefully; if it does, both
  the fd-lock workaround and the retry loops can be removed.
- **Do NOT** remove `continue-on-error` from CI until the pre-existing
  `integration_graph_vector_rehydration` and `integration_query_perf_observability` failures are
  also resolved (separate from U015-FLK1).
- Enabling SQLite WAL mode (`PRAGMA journal_mode=WAL`) would allow concurrent readers but still requires cozo's error handling to not panic.
- The `engram.db.lock` sidecar file: the advisory lock is released when the file descriptor closes (on process exit or when `_guard` is dropped). The file itself may remain on disk but is harmless — it is excluded by `.gitignore` and carries no meaningful state after lock release.
