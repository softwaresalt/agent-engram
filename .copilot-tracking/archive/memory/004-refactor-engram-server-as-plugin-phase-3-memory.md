# Phase 3 Memory: US1+US2 — Shim Lifecycle + Workspace Isolation (T020–T034)

**Spec**: `004-refactor-engram-server-as-plugin`
**Phase**: 3
**Date**: 2026-03-04
**Status**: COMPLETE — All 15 tasks done, all gates pass, committed

---

## Task Overview

Phase 3 implements the shim-side lifecycle management and workspace isolation for the plugin
architecture introduced in Phases 1–2. The shim process (which runs as an rmcp stdio server
inside the IDE) must: (1) check if a daemon is already running for the workspace, (2) spawn one
if not, (3) wait for it to be ready, then (4) run the rmcp MCP server over stdio forwarding
tool calls to the daemon via IPC.

**User Stories**: US1 (Shim Lifecycle Management), US2 (Workspace Isolation)  
**Tasks**: T020–T034 (15 total)

---

## Current State

### All Tasks Complete

| Task | Description | Status |
|------|-------------|--------|
| T020 | Write shim lifecycle contract tests | ✅ Done |
| T021 | Write shim error handling integration tests | ✅ Done |
| T022 | Write multi-workspace isolation integration tests | ✅ Done |
| T023 | Write lockfile unit tests | ✅ Done |
| T024 | Write IPC protocol contract tests (already existed Phase 2) | ✅ Done |
| T025 | Write proptest serialization tests | ✅ Done |
| T026 | Implement `src/shim/ipc_client.rs` | ✅ Done |
| T027 | Implement `src/shim/lifecycle.rs` health check | ✅ Done |
| T028 | Implement `src/shim/lifecycle.rs` spawn + backoff | ✅ Done |
| T029 | Implement `src/shim/transport.rs` ShimHandler + run_shim | ✅ Done |
| T030 | Wire `src/shim/mod.rs` run() workspace resolution | ✅ Done |
| T031 | Wire `src/shim/mod.rs` run() daemon + shim startup | ✅ Done |
| T032 | IPC workspace scoping (done Phase 2, verified) | ✅ Done |
| T033 | `_health` handler (done Phase 2, verified) | ✅ Done |
| T034 | All tests pass | ✅ Done |

### Files Created/Modified

| File | Change |
|------|--------|
| `src/shim/ipc_client.rs` | NEW: `send_request()`, platform-specific `connect()`, JSON-RPC framing |
| `src/shim/lifecycle.rs` | NEW: `check_health()`, `ensure_daemon_running()`, `spawn_daemon()`, `poll_until_ready()` |
| `src/shim/transport.rs` | NEW: `ShimHandler`, `run_shim()`, rmcp stdio server |
| `src/shim/mod.rs` | MODIFIED: fully implemented `run()` — was `todo!()` |
| `src/daemon/ipc_server.rs` | MODIFIED: `ipc_endpoint()` made `pub` |
| `tests/helpers/mod.rs` | MODIFIED: `.git/HEAD` stub + `check_health()` readiness probe |
| `tests/contract/shim_lifecycle_test.rs` | NEW: 9 tests S001–S008 |
| `tests/integration/shim_error_test.rs` | NEW: 6 tests S009, S010 |
| `tests/integration/multi_workspace_test.rs` | NEW: 6 tests S088–S091 |
| `Cargo.toml` | MODIFIED: 3 `[[test]]` entries added |
| `specs/.../tasks.md` | MODIFIED: T020–T034 marked `[X]` |

### Test Results (all pass)

All 26 test suites pass, 0 failures.  
Notable suites: contract_shim_lifecycle (9), integration_shim_error (6), integration_multi_workspace (6), unit_proptest (16), unit_proptest_serialization (15), contract_ipc_protocol (23).

---

## Important Discoveries

### Windows Named Pipe Readiness Detection

`std::fs::metadata(pipe_path)` does NOT detect whether a Windows named pipe server is actually
listening — it always fails with "The system cannot find the file". The correct approach is to
attempt an actual IPC connection (`check_health()`) and wait for it to succeed. The harness was
updated to use this pattern.

### Canonical Path Hash Alignment

`std::fs::canonicalize()` on Windows returns `\\?\`-prefixed paths. Both the daemon's
`canonicalize_workspace()` and the test harness must call `std::fs::canonicalize()` on the same
path for the SHA-256 database namespace hash to match. The `DaemonHarness` passes the TempDir
path through `canonicalize()` before setting `ENGRAM_WORKSPACE`.

### rmcp 0.13.0 API

Despite `Cargo.toml` specifying `"1.1"`, the crate resolves to 0.13.0. Key API:
- `rmcp::serve_server(handler, transport).await` → `RunningService<RoleServer, S>`
- `rmcp::transport::io::stdio()` → `(tokio::io::Stdin, tokio::io::Stdout)` used as transport
- `running.waiting().await` — awaits session end, returns `Result`

### interprocess 2.3.1 Client Pattern

- Unix: `Name::from(path).to_fs_name::<GenericFilePath>()` → `Stream::connect(name).await`
- Windows: strip `\\.\pipe\` prefix, `to_ns_name::<GenericNamespaced>()` → `Stream::connect(name).await`
- `stream.split()` → `(RecvHalf: AsyncRead, SendHalf: AsyncWrite)`

### Adversarial Review Fixes Applied (F-1 HIGH, F-2/F-3/F-5 MEDIUM/LOW)

- **F-1 HIGH**: Added `MAX_RESPONSE_BYTES = 1 MiB` constant, applied via `.take()` on `recv_half`
  before `BufReader` in `ipc_client.rs`. Prevents unbounded allocation from misbehaving daemon.
- **F-2 MEDIUM**: After `read_line()`, check `n == 0` and return specific "daemon closed connection"
  error instead of falling through to a misleading JSON parse error.
- **F-3 MEDIUM**: `running.waiting().await` now uses `.map_err(...)` to propagate MCP session errors
  instead of silent `let _ = ...` discard in `transport.rs`.
- **F-5 LOW**: `shim/mod.rs` workspace resolution now returns `EngramError::Workspace` instead of
  silently yielding an empty string when `current_dir()` fails.
- **F-4 MEDIUM (deferred)**: `list_tools` returns empty list — tool discovery proxying to daemon
  deferred to a future phase. The shim's IPC dispatch uses method-name routing, not MCP discovery.

### Debug Build Timing

The `READY_TIMEOUT_MS = 2_000` spec SLA applies to release builds. In debug builds, process startup
takes 5–10 seconds. The timing assertion in `shim_lifecycle_test.rs` uses a 10s threshold
(not the 2s spec SLA) specifically for debug-mode CI compatibility.

---

## Next Steps (Phase 4)

**Phase 4: US4 — Real-Time File System Awareness (T035–T047, ~12 tasks)**

- Implement `src/daemon/watcher.rs` using `notify 9.0.0-rc.2` + `notify-debouncer-full 0.7.0`
- Debounce file change events from the workspace root
- Notify connected clients via the IPC broadcast channel when `.engram/` files change
- Watch for new/modified/deleted `.engram/*.md` task files and re-hydrate

**Known dependency concern**: `notify 9.0.0-rc.2` is pre-release; check Cargo.toml for
conflicts with `notify-debouncer-full 0.7.0` before starting Phase 4.

---

## Context to Preserve

- Commit for Phase 3: see git log
- Branch: `004-refactor-engram-server-as-plugin`
- `src/daemon/ipc_server.rs` → `ipc_endpoint()` is now `pub` (needed by shim)
- `tests/helpers/mod.rs` → `DaemonHarness` creates `.git/HEAD` stub; uses `check_health()` for readiness
- The timing threshold in `shim_lifecycle_test.rs` is intentionally 10s (not 2s spec SLA)
- F-4 (list_tools empty) is a known deferred finding — acceptable for Phase 3 scope
