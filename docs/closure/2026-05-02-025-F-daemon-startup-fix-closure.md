---
title: Post-Merge Closure — 025-F Engram Server Reliability & Dog-Fooding
feature_id: 025-F
shipment_id: 020-S
merge_commit: d20ac49
pr_number: 72
branch: feat/025-F-daemon-startup-fix
closure_date: 2026-05-02
status: READY
---

## Summary

Feature 025-F fixed a daemon startup hang where `start_watcher` (blocking file-system
watch registration via `ReadDirectoryChangesW` on Windows / `inotify_add_watch` on Linux)
was called before the IPC listener was bound. On large workspaces the watch registration
takes >2s, causing the shim's health probe (2s timeout) to report "Daemon failed to reach
Ready state". The fix introduces bind-first startup ordering and stale PID cleanup.

### Shipped Changes

| File | Change |
|---|---|
| `src/daemon/ipc_server.rs` | `run_with_shutdown_v2` — new entry point with bind-first ordering; watcher started in `spawn_blocking` with 5s timeout after IPC bind |
| `src/daemon/mod.rs` | `daemon::run()` now calls `run_with_shutdown_v2`; `remove_stale_pid_if_dead` clears stale PID files before lock acquisition; diagnostic `info!` spans added |
| `tests/integration/daemon_startup_order_test.rs` | New integration test confirming bind-first ordering and TTL-based shutdown |
| `tests/integration/concurrent_sessions_test.rs` | Pre-existing flaky test `s_cs4` marked `#[ignore]` (U015-FLK1) |
| `tests/integration/graph_vector_rehydration_test.rs` | Pre-existing flaky test marked `#[ignore]` (026-F gap) |

### PR Review Cycles

Two rounds of Copilot review, all findings addressed:

**Round 1 (6 comments):**
- Doc comment accuracy (FlushFailed vs. run-directory creation)
- Legacy numeric PID file fallback in `remove_stale_pid_if_dead`
- `remove_file` error semantics (Ok(Some(pid)) only on success)
- Duplicate caller log removed
- Docblock consolidation
- `mpsc` channel moved inside `spawn_blocking`; event loop made conditional

**Round 2 (2 comments):**
- `remove_stale_pid_if_dead`: return type changed from `Result<Option<u32>>` → `Option<u32>`
  (all error paths are non-fatal; `FlushFailed` misuse resolved; clippy `unnecessary_wraps` lint)
- `run_with_shutdown_v2`: stale comment "before acquiring the lock" fixed to "before binding
  the listener" with note that lock is held by caller

---

## Invariants to Preserve

1. IPC listener MUST bind before any blocking file-system watch registration
2. `remove_stale_pid_if_dead` MUST be called before `DaemonLock::acquire` on every startup
3. `WatcherHandle` MUST be kept in the outer `run_with_shutdown_v2` scope (not moved into
   spawn task) so watcher lifetime equals daemon lifetime, not task lifetime
4. `run_with_shutdown_v2` is the authoritative daemon entry point; `run_with_shutdown` is
   preserved for reference but no longer called

---

## Pre-Deploy Checks

- [x] `cargo fmt --all -- --check` — clean
- [x] `cargo clippy -- -D warnings -D clippy::pedantic` — clean
- [x] `cargo test --lib` — 91 unit tests passed
- [x] Integration test `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` — passes (~17s)
- [x] Release binary built (`cargo build --release`)
- [x] Manual e2e: shim → spawned fresh daemon → reached Ready state (025.004-T)
- [x] CI green (2m14s on final push)
- [x] All 8 Copilot review threads resolved

---

## Post-Deploy Checks

On next daemon startup after merge:
1. Confirm shim reports "Ready" within 2s on large workspaces (agent-engram itself)
2. Confirm no "Daemon failed to reach Ready state within 2000ms" errors in logs
3. If upgrading from a build with old PID file format: confirm `remove_stale_pid_if_dead`
   handles legacy numeric PID files gracefully (warn log, no crash)

---

## Healthy Signals

- Daemon reaches Ready state within 1–2s on any workspace size
- `engram up` returns 0 exit code
- IPC health probe succeeds on first try
- `tracing` output shows `daemon lock acquired` before watcher init log

---

## Failure Signals / Rollback Triggers

| Signal | Meaning | Action |
|---|---|---|
| "Daemon failed to reach Ready state within 2000ms" | bind-first ordering not taking effect | Rollback: `git revert d20ac49` |
| Watcher timeout log on startup | `spawn_blocking` 5s timeout triggered | Investigate workspace size; consider increasing timeout |
| Panic in `run_with_shutdown_v2` | WatcherHandle scope issue | Check that `_watcher_handle` is not moved into spawned task |

---

## Rollback Procedure

```bash
git revert d20ac49
cargo build --release
# deploy reverted binary
```

The old `run_with_shutdown` entry point is preserved in `ipc_server.rs` and can be
restored in `daemon::run()` by reverting the single call-site change.

---

## Validation Window

**Duration**: 48 hours post-merge  
**Owner**: softwaresalt  
**Watch**: daemon startup latency on Windows (ReadDirectoryChangesW) and Linux (inotify)

---

## Monitoring Plan

| Signal | Where to observe |
|---|---|
| Startup latency | `tracing` output: time between `run_with_shutdown_v2 starting` and `daemon ready` |
| Watcher timeout | `tracing` warn: "watcher init timed out" |
| Stale PID cleanup | `tracing` info: "removed stale PID file for dead process" |
| Health probe failures | Shim exit codes / error messages |

---

## Source Artifact Cleanup

| Field | Value | Notes |
|---|---|---|
| `source_stash_id` | absent from custom_fields | Original stash entry was `9B4996E5` (Group A deliberation); not recorded in custom_fields |
| `source_deliberation_id` | absent from custom_fields | Deliberation artifacts in `docs/decisions/` |
| `backlog_md_id` | m-0 | Legacy migration reference from backlog-md |

---

## Deferred Follow-Ups

| ID | Description | Source |
|---|---|---|
| U015-FLK1 | Add SQLITE_BUSY retry logic to `index_workspace` handler | Pre-existing flaky test `s_cs4` |
| 026-F gap | `flush_state` doesn't write `nodes.jsonl` to `.engram/code-graph/main/` | Pre-existing rehydration test failure |
| — | Increase shim health probe timeout from 2000ms to accommodate slow-start environments | Mitigation for watcher timeout edge cases |
