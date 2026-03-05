# Phase 5 Memory: US3 — Automatic Lifecycle Management (T045–T055)

**Spec**: `004-refactor-engram-server-as-plugin`
**Phase**: 5
**Date**: 2026-03-05
**Status**: COMPLETE — All 11 tasks done, all gates pass

---

## Task Overview

Phase 5 implements automatic lifecycle management for the workspace daemon (US3). The daemon
tracks idle time via a TTL timer, shuts down gracefully on expiry or signal, and recovers
cleanly from crashes (stale lockfile detection, socket cleanup).

**User Story**: US3 (Automatic Lifecycle Management)
**Tasks**: T045–T055 (11 total)

---

## Current State

### All Tasks Complete

| Task | Description | Status |
|------|-------------|--------|
| T045 | TTL timer unit tests (S045, S046-S047, S049, S051) | ✅ Done |
| T046 | Daemon lifecycle integration tests (graceful shutdown, restart) | ✅ Done |
| T047 | Crash recovery integration tests (SIGKILL, stale lock) | ✅ Done |
| T048 | TTL timer implementation in src/daemon/ttl.rs | ✅ Done |
| T049 | TTL reset wired into IPC request handler | ✅ Done |
| T050 | TTL reset wired into watcher event handler | ✅ Done |
| T051 | Graceful shutdown sequence in src/daemon/mod.rs | ✅ Done |
| T052 | `_shutdown` IPC handler | ✅ Done |
| T053 | Stale lock/socket cleanup in lockfile.rs | ✅ Done |
| T054 | SIGTERM/SIGINT (ctrl_c) handler | ✅ Done |
| T055 | All tests pass (29 suites, 0 failures) | ✅ Done |

### Files Modified

| File | Change |
|------|--------|
| `src/daemon/ttl.rs` | IMPLEMENTED: TtlTimer with Arc<Mutex<Instant>>, reset(), run_until_expired() |
| `src/daemon/mod.rs` | MODIFIED: run() with TTL + shutdown channel + signal + watcher integration |
| `src/daemon/ipc_server.rs` | MODIFIED: TTL reset on accept, _shutdown handler, run_with_shutdown() |
| `src/daemon/lockfile.rs` | MODIFIED: clean_stale_socket() called after lock acquisition |
| `tests/unit/ttl_test.rs` | NEW: 5 unit tests using tokio::time::pause + advance |
| `tests/integration/daemon_lifecycle_test.rs` | NEW: 7 integration tests S037-S040 |
| `Cargo.toml` | MODIFIED: 2 [[test]] entries added |
| `specs/.../tasks.md` | MODIFIED: T045–T055 marked [X] |

---

## Important Discoveries

### cfg(test) doesn't apply to external test binaries
`#[cfg(test)]` on constants in library code only activates during `cargo test --lib`, not
when external `tests/unit/ttl_test.rs` links the library. Fix: adaptive check interval
`min(CHECK_INTERVAL, timeout/2).max(1ms)` — no cfg needed.

### tokio::spawn + time::pause requires pre-yield before advance()
A spawned task doesn't execute until the spawner yields. If `advance(300ms)` is called before
the spawned task has run even once, the task starts during the advance and registers its sleep
at T=300ms+100ms=400ms — past the advance window. Fix: `yield_now().await` immediately after
`spawn()` to let the task register its initial sleep at T=100ms.

### TTL task must spawn AFTER daemon is ready
Spawning TTL before SurrealDB init caused premature expiry during startup. Fix: moved TTL task
spawn inside `run_with_shutdown()`, after `bind_listener()` + initial `ttl.reset()`.

### Shutdown coordination
- `tokio::sync::watch::channel(false)` used for shutdown signaling
- `shutdown_tx.send(true)` triggered by: TTL expiry, `_shutdown` IPC, SIGTERM/ctrl_c
- IPC server's accept loop watches `shutdown_rx.changed()` via `tokio::select!`
- `_shutdown` handler returns `{"status":"shutting_down","flush_started":true}` before signaling

### Windows crash recovery
On Windows, OS releases the fd-lock when process dies (TerminateProcess). The stale socket
(named pipe) is auto-cleaned by Windows when the server handle closes. `clean_stale_socket()`
is a no-op on Windows but cleans up `.engram/run/engram.sock` on Unix.

---

## Next Steps (Phase 6)

**Phase 6: US5 — Plugin Installation & Management (T056–T059+)**

Tasks: install command, update/reinstall/uninstall, installer with running daemon
Key files: `src/installer/mod.rs`, `src/installer/templates.rs`
Scenarios: S067-S078

---

## Context to Preserve

- `TtlTimer::reset()` is `pub(crate)` — called from ipc_server.rs and mod.rs
- `TtlTimer::new(Duration::ZERO)` = never expires (run forever — S049)
- Shutdown channel: `tokio::sync::watch::channel(false)` → `Sender<bool>` + `Receiver<bool>`
- `_shutdown` IPC method → `{"jsonrpc":"2.0","id":...,"result":{"status":"shutting_down","flush_started":true}}`
- `clean_stale_socket(workspace)` in lockfile.rs removes Unix socket; no-op on Windows
- Branch: `004-refactor-engram-server-as-plugin`
