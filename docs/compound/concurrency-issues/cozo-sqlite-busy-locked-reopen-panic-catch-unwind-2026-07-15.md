---
title: "cozo 0.7.x SQLITE_BUSY/LOCKED reopen transient is a PANIC, not an Err — retry must catch_unwind"
description: "A bounded reopen-retry around DbInstance::new that only inspects the Err channel is INERT for the transient it targets: cozo 0.7.6 unwraps internally, so a rapid sequential reopen busy/lock surfaces as a panic. The fix catches the panic, classifies SQLite-specific busy/lock markers, converts to a retryable Err, and re-raises everything else."
problem_type: upstream_bug
category: concurrency-issues
component: db/cozo_backend
root_cause: "cozo-0.7.6/src/storage/sqlite.rs:49 calls unwrap() on SQLite open; SQLITE_BUSY/SQLITE_LOCKED panic the thread instead of returning an Err, so an Err-only retry never fires."
resolution_type: local_mitigation
severity: high
message: "thread '...' panicked at cozo-0.7.6/src/storage/sqlite.rs:49: called `Result::unwrap()` on an `Err` value: SqliteFailure(Error { code: DatabaseBusy, extended_code: 5 }, Some(\"database is locked\"))"
file_path: src/db/cozo_backend/mod.rs
shipment: 082-S
feature: 086-F
tasks:
  - 086.002-T
citations:
  - "docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md"
  - "docs/closure/2026-07-15-082-s-runtime-reliability-adversarial-review.md"
  - "PR #249 (merge 8adde5e); adversarial review F2; Copilot review rounds 1 and 5"
tags:
  - cozo
  - sqlite
  - SQLITE_BUSY
  - SQLITE_LOCKED
  - panic
  - catch_unwind
  - reopen-retry
  - U015-FLK1
  - 041.002-T
---

## Problem

`connect_db` opens a fresh `cozo::DbInstance` per branch DB. On a rapid **sequential**
reopen of the same SQLite file (Windows lock-release lag; crash-restart), SQLite returns
`SQLITE_BUSY`/`SQLITE_LOCKED`. cozo 0.7.6 (`storage/sqlite.rs:49`) calls `unwrap()` on the
open, so that transient **panics the thread** instead of returning an `Err`.

A first attempt at 086.002-T added a bounded reopen-retry that classified only the **Err**
channel (`is_retryable_open_error`) and validated it against a *fabricated* `Err("database is
locked")`. Because the real transient is a **panic**, that retry was **inert** for its target:
the panic unwound *past* the retry loop (0 retries) and was only contained downstream by
`spawn_blocking` + `JoinError`, which surfaces the open as a failure rather than absorbing it.
The adversarial review (F2) and Copilot both caught this by cross-referencing the repo's own
prior note (`cozodb-sqlite-lock-panic-2026-05-01.md`).

## Root cause

`SQLITE_BUSY` (code 5, message "database is locked") and `SQLITE_LOCKED` (code 6, message
"database table is locked") are unwrapped inside cozo 0.7.x. An `Err`-only classifier cannot
observe a value that never reaches the `Err` arm.

## Resolution (082-S / 086.002-T)

Wrap the open in `std::panic::catch_unwind(AssertUnwindSafe(...))`, then:

```rust
match std::panic::catch_unwind(AssertUnwindSafe(|| DbInstance::new("sqlite", path, Default::default()))) {
    Ok(Ok(db))   => Ok(db),
    Ok(Err(e))   => Err(e.to_string()),          // non-panic Err -> retry loop classifies it
    Err(payload) => {
        let msg = panic_payload_message(payload.as_ref());
        if is_sqlite_busy_or_locked_panic(&msg) { // SQLite-specific markers ONLY
            Err(msg)                               // convert busy/lock PANIC -> retryable Err
        } else {
            std::panic::resume_unwind(payload);    // re-raise unrelated panics unchanged
        }
    }
}
```

Then a bounded, jittered exponential back-off (`open_db_with_retry`) retries the resulting
`Err` while it classifies as busy/locked, giving up with a clear `EngramError`.

### Non-obvious lessons (cost the review its rounds)

1. **Panic vs Err is the whole game.** Test the panic path with a real `panic!(...)` payload,
   not a fabricated `Err`, or the retry can be green-but-inert. A live cozo busy panic was
   observed **absorbed with zero test failures** during the hermetic suite — the true
   validation.
2. **Classify panics with SQLite-specific markers, never bare "busy"/"locked".** A generic
   substring match would swallow an unrelated `panic!("worker is busy ...")` and mask a real
   bug. Match `database is locked` / `database is busy` / `sqlite_busy` / `databasebusy` (BUSY)
   **and** `database table is locked` / `sqlite_locked` / `databaselocked` (LOCKED). Keep the
   broader "busy"/"locked" match only on the ERROR path, whose source is already a CozoDB open.
3. **Always re-raise non-matching panics** (`resume_unwind`) so genuine invariant violations
   still propagate.

## Prevention / removal

This is an **interim** mitigation tracked by blocked **041.002-T**. Removal is TWO distinct
steps — do not conflate them:

1. **Remove the `catch_unwind` wrapper** once cozo ≥ 0.8 returns an `Err` (no internal
   `unwrap`) for `SQLITE_BUSY`/`SQLITE_LOCKED`. This only changes the transient's SURFACE
   (panic → `Err`); the transient itself still occurs.
2. **Remove the reopen-retry** (`open_db_with_retry`) ONLY after verifying that upstream
   (cozo/SQLite — e.g. an internal busy-timeout) actually ABSORBS the transient, so a rapid
   sequential reopen no longer surfaces busy/lock at all. Cozo merely returning an `Err` does
   NOT satisfy this: `open_db_with_retry` still retries that `Err` channel, so removing the
   retry prematurely would reintroduce the startup failures this mitigation prevents.

Until removed, the handled busy panic logs a `panicked at .../sqlite.rs:49` line via the
default panic hook even though it is absorbed — expected, not a failure.
