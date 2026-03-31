# Phase 2 Session Memory: Foundational IPC Transport & Lockfile
## Spec: 004-refactor-engram-server-as-plugin
## Date: 2026-03-04

---

## Task Overview

Phase 2 implemented the foundational IPC transport and lockfile management needed before any user story work can begin. All 10 tasks (T010–T019) completed.

**Objective**: Implement `src/daemon/protocol.rs` (IPC wire types), `src/daemon/lockfile.rs` (fd-lock PID file), `src/daemon/ipc_server.rs` (LocalSocketListener accept loop + endpoint naming), and wire `daemon::run()` to `ipc_server::run()`. Write contract tests (T010), unit tests (T011, T012) first per TDD discipline.

---

## Current State

### Tasks Completed
- [X] T010: `tests/contract/ipc_protocol_test.rs` — 23 contract tests (S014–S025)
- [X] T011: `tests/unit/lockfile_test.rs` — 3 unit tests (S027, S029, S032)
- [X] T012: `tests/unit/proptest_models.rs` — 4 new proptest round-trips added (IpcRequest, IpcResponse, IpcError, DaemonStatus)
- [X] T013: `src/daemon/protocol.rs` — IpcRequest, IpcResponse, IpcError wire types with validation
- [X] T014: `src/daemon/mod.rs` — DaemonStatus enum, DaemonState struct, `pub mod protocol`
- [X] T015: `src/daemon/lockfile.rs` — DaemonLock with fd-lock + PID file
- [X] T016: `src/daemon/ipc_server.rs` — LocalSocketListener accept loop, connection handling, process_request dispatch
- [X] T017: `src/daemon/ipc_server.rs` — `ipc_endpoint()` function (Unix: `.engram/run/engram.sock`, Windows: `\\.\pipe\engram-{sha256_first16hex}`)
- [X] T018: `src/daemon/mod.rs::run()` → delegates to `ipc_server::run()`
- [X] T019: All tests pass (26 new + full suite)

### Files Created/Modified
- **NEW** `src/daemon/protocol.rs` — IpcRequest, IpcResponse, IpcError (wire-format JSON-RPC 2.0)
- **MODIFIED** `src/daemon/mod.rs` — added DaemonStatus, DaemonState, `pub mod protocol`, wired run()
- **MODIFIED** `src/daemon/lockfile.rs` — full DaemonLock implementation with PID write + truncate fix
- **MODIFIED** `src/daemon/ipc_server.rs` — full implementation (endpoint naming, bind, accept loop, dispatch)
- **NEW** `tests/contract/ipc_protocol_test.rs` — 23 contract tests
- **NEW** `tests/unit/lockfile_test.rs` — 3 unit tests
- **MODIFIED** `tests/unit/proptest_models.rs` — 4 proptest round-trips appended
- **MODIFIED** `Cargo.toml` — `[[test]]` entries for `contract_ipc_protocol` and `unit_lockfile`
- **MODIFIED** `specs/004-refactor-engram-server-as-plugin/tasks.md` — T010–T019 marked `[X]`

### Test Results
- `contract_ipc_protocol`: 23/23 pass
- `unit_lockfile`: 3/3 pass
- Full suite: all binaries pass, 0 failures
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: clean
- `cargo fmt --all -- --check`: clean

---

## Important Discoveries

### 1. Box::leak pattern for fd-lock lifetime
`fd_lock::RwLock::try_write()` returns a guard whose lifetime is bounded by the `RwLock`. To store both in the same `DaemonLock` struct, we use `Box::leak()` to obtain a `'static` mutable reference. This is sound because: (a) no unsafe blocks, (b) OS releases the flock when the guard drops OR when the process dies, (c) one daemon = one lock = one allocation.

**Caution**: `Box::leak` currently happens before `try_write()` completes, so a `WouldBlock` failure leaks a tiny allocation. This is acceptable given daemon startup frequency.

### 2. PID file corruption bug (fixed)
Original implementation: `seek(SeekFrom::Start(0))` + `write_all(pid_bytes)` without truncating first. If the previous PID was longer (e.g., "12345678"), writing a shorter PID (e.g., "99") leaves stale bytes: file reads as "9945678". 
**Fix**: Added `guard.set_len(0)?` before seek. `File::set_len` accessible via Deref on `RwLockWriteGuard`.

### 3. IpcError naming collision
`crate::errors::IpcError` is the domain error enum (connection failures, timeouts).
`crate::daemon::protocol::IpcError` is the wire-format struct (code: i32, message: String, data: Option<Value>).
Both coexist. `ipc_server.rs` uses: `use crate::daemon::protocol::{IpcError as WireError, ...}` and `use crate::errors::{IpcError as DomainIpcError, ...}`.

### 4. interprocess 2.x tokio API
The listener API in interprocess 2 uses:
- `ListenerOptions::new().name(name).create_tokio()` → `Listener`
- `listener.accept().await` → `Stream`
- `stream.split()` → `(RecvHalf, SendHalf)`
- `BufReader::new(recv_half).read_line(&mut line)` for line framing

On Unix: `"socket/path".to_fs_name::<GenericFilePath>()` then `ListenerOptions`
On Windows: pipe name WITHOUT `\\.\pipe\` prefix → `pipe_name.to_ns_name::<GenericNamespaced>()`

### 5. Stale socket removal fix
Original: `if path.exists() { let _ = remove_file(...); }` — silently discards errors, including permissions failures. Fixed to propagate non-NotFound errors with full diagnostic.

### 6. read_line size cap
Added `const MAX_REQUEST_BYTES: usize = 1024 * 1024` and match arm `Ok(n) if n > MAX_REQUEST_BYTES` returning a parse error response. Prevents pathological clients from causing unbounded allocation.

### 7. _shutdown not fully wired (T052 deferral)
`_shutdown` IPC method returns `{"status": "shutting_down", "flush_started": true}` per contract but does NOT actually halt the accept loop. The `accept_loop` only breaks on Ctrl-C. Full shutdown requires a `CancellationToken` or `oneshot` channel — deferred to T052 (Phase 8).

### 8. Windows named pipe naming
IPC endpoint: `\\.\pipe\engram-{sha256_first16hex}`
But `to_ns_name::<GenericNamespaced>()` expects the name WITHOUT `\\.\pipe\`.
Fixed in `bind_listener_impl(windows)`: `let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);`

---

## Next Steps (Phase 3: US1+US2 Shim Lifecycle + Isolation)

### Phase 3 task overview (T020–T034)
- T020–T025: Tests for shim cold/warm start, error forwarding, multi-workspace isolation
- T026: Shim IPC client (`src/shim/ipc_client.rs`)
- T027: Shim lifecycle (`src/shim/lifecycle.rs`) — health check, spawn, exponential backoff
- T028: Daemon spawn guard (detect existing daemon before spawning)
- T029: rmcp `StdioTransport` + `ServerHandler` in `src/shim/transport.rs`
- T030: Wire shim subcommand in `src/bin/engram.rs`
- T031: Wire daemon subcommand in `src/bin/engram.rs`
- T032: Workspace-scoped IPC addressing (T017 already implements this)
- T033: `_health` IPC handler (basic version already in `process_request`)
- T034: `cargo test` for Phase 3

### Open Questions for Phase 3
1. **rmcp StdioTransport API**: `rmcp 1.1` with `transport-io` feature — exact API for serving stdio in the shim. Check rmcp crate for `serve_client_with_ct` or equivalent.
2. **Tool list in shim**: The shim's rmcp ServerHandler must implement `list_tools` with a compiled-in registry. Should this enumerate `tools::dispatch` matches statically, or query the daemon via a new `_list_tools` IPC method?
3. **DaemonHarness readiness**: `DaemonHarness::spawn()` polls for socket/pipe file. With Phase 2 IPC working, this should connect successfully. T020 (cold start test) will be the first real integration test.

---

## Context to Preserve

- `src/daemon/protocol.rs` → all IPC wire types live here
- `src/daemon/lockfile.rs` → `DaemonLock::acquire(workspace: &Path)` is the entry point
- `src/daemon/ipc_server.rs` → `ipc_endpoint()`, `bind_listener()`, `run()` are the public-facing functions
- `tests/helpers/mod.rs` → `DaemonHarness::spawn()` polls IPC socket for readiness (from Phase 1.5)
- Windows pipe naming: strip `\\.\pipe\` before calling `to_ns_name`
- interprocess 2 split: `stream.split()` gives `(RecvHalf, SendHalf)` — these are NOT tokio traits directly but work with `BufReader::new()` and `write_all()` via `AsyncRead`/`AsyncWrite` impls

## Previous Phase Commits
- Phase 1 commit: `0495204`
- Phase 1.5 commit: `7e40ee8`
- Phase 2 commit: (pending — see Step 11)
