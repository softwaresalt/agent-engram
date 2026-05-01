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

## Resolution (018-S — permanent fix)

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

**Previous workaround (015-S — superseded):** `continue-on-error: true` on the CI test step.
Remove this from `.github/workflows/ci.yml` now that 018-S is merged.

**Approaches that did NOT work:**
- `cargo nextest run --test-threads 1`: Serializes tests globally, but daemon cleanup between tests still leaves SQLite locks briefly held. The failure shifts non-deterministically to whichever test runs next after a daemon teardown. Different test ordering exposes different victims.
- Exclusion lists (skip specific tests): Whack-a-mole — the flaky behavior appears on any test that happens to run immediately after a daemon teardown under lock.

## Prevention

- The `fd-lock` advisory lock in `connect_db` prevents the panic for all callers. When upgrading cozo beyond 0.7.x, verify the new version handles `SQLITE_BUSY` gracefully; if it does, the fd-lock workaround can be removed.
- Remove `continue-on-error` from CI after confirming the fd-lock fix holds consistently.
- Enabling SQLite WAL mode (`PRAGMA journal_mode=WAL`) would allow concurrent readers but still requires cozo's error handling to not panic.
- The `engram.db.lock` sidecar file: the advisory lock is released when the file descriptor closes (on process exit or when `_guard` is dropped). The file itself may remain on disk but is harmless — it is excluded by `.gitignore` and carries no meaningful state after lock release.
