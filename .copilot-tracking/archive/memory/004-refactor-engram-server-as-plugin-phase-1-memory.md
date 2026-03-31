# Session Memory: 004-refactor-engram-server-as-plugin Phase 1

**Date**: 2026-03-04  
**Phase**: 1 — Setup (Dependencies, Stubs)  
**Spec**: specs/004-refactor-engram-server-as-plugin/  
**Branch**: `004-refactor-engram-server-as-plugin`

---

## Task Overview

Phase 1 established the structural scaffolding for the plugin architecture refactor:
- Removed old `mcp-sdk 0.0.3` dependency, replaced with `rmcp 1.1`
- Added IPC, lockfile, and file-watching dependencies
- Restructured the binary entrypoint with clap subcommands
- Created stub module trees for `shim`, `daemon`, and `installer`
- Extended the error hierarchy with 5 new error types and 14 new error codes

No behavioral logic was implemented — all public API functions contain `todo!()`.

---

## Current State

### Tasks Completed

- **T001**: Added `rmcp = { version = "1.1", features = ["server", "transport-io"] }`, `interprocess = { version = "2", features = ["tokio"] }`, `fd-lock = "4"`, `notify = "9.0.0-rc.2"`, `notify-debouncer-full = "0.7.0"` to Cargo.toml
- **T002**: Removed `mcp-sdk = "0.0.3"` from Cargo.toml (no existing code referenced it at all — grep confirmed zero usage in src/ and tests/)
- **T003**: Rewrote `src/bin/engram.rs` with clap subcommands: `shim` (default), `daemon --workspace`, `install`, `update`, `reinstall`, `uninstall [--keep-data]`
- **T004**: Created `src/shim/` stubs: `mod.rs`, `transport.rs`, `ipc_client.rs`, `lifecycle.rs`
- **T005**: Created `src/daemon/` stubs: `mod.rs`, `ipc_server.rs`, `watcher.rs`, `debounce.rs`, `ttl.rs`, `lockfile.rs`
- **T006**: Created `src/installer/` stubs: `mod.rs`, `templates.rs`
- **T007**: Added to `src/errors/mod.rs`: `IpcError`, `DaemonError`, `LockError`, `WatcherError`, `InstallError` enums; 5 new `EngramError` variants; complete `to_response()` match arms for all new variants
- **T008**: Added 14 new constants to `src/errors/codes.rs`: 8xxx range (IPC/daemon: 8001–8009) and 9xxx range (installer: 9001–9005)
- **T009**: `cargo check --all-targets` passes (exit 0)

### Verification Gates Passed

- `cargo check --all-targets`: ✅ exit 0
- `cargo fmt --all -- --check`: ✅ exit 0 (auto-fixed one brace style in errors/mod.rs)
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: ✅ exit 0 (added `#[allow(clippy::unused_async)]` to 6 stub async functions)
- `cargo test --lib`: ✅ 74 tests pass
- `cargo test` (full suite): ✅ ~282 tests across all suites, 0 failures

### Files Modified

- `Cargo.toml` — removed mcp-sdk, added 5 new dependencies
- `Cargo.lock` — updated (new deps: rmcp, rmcp-macros, interprocess, fd-lock, notify 9.0.0-rc.2, notify-debouncer-full 0.7.0, and their transitive deps)
- `src/lib.rs` — added `pub mod daemon;`, `pub mod installer;`, `pub mod shim;`
- `src/bin/engram.rs` — complete rewrite (clap subcommands)
- `src/errors/mod.rs` — 5 new error types, 5 new EngramError variants, new to_response() match arms
- `src/errors/codes.rs` — 14 new constants

### Files Created

- `src/shim/mod.rs` — shim module with `run()` stub
- `src/shim/transport.rs` — rmcp ServerHandler stub placeholder
- `src/shim/ipc_client.rs` — IPC client stub placeholder
- `src/shim/lifecycle.rs` — daemon health-check + spawn stub placeholder
- `src/daemon/mod.rs` — daemon module with `run()` stub
- `src/daemon/ipc_server.rs` — IPC server stub placeholder
- `src/daemon/watcher.rs` — file watcher stub placeholder
- `src/daemon/debounce.rs` — debouncer stub placeholder
- `src/daemon/ttl.rs` — TTL timer stub placeholder
- `src/daemon/lockfile.rs` — lockfile stub placeholder
- `src/installer/mod.rs` — installer with 4 async stub functions
- `src/installer/templates.rs` — MCP config template stub placeholder
- `docs/adrs/0015-rmcp-migration-and-plugin-architecture.md` — ADR for rmcp migration decision
- `specs/004-refactor-engram-server-as-plugin/tasks.md` — T001–T009 marked `[X]`

---

## Important Discoveries

### 1. notify Version Mismatch (Phase 2/4 Planning Issue)

`notify-debouncer-full 0.7.0` (latest) depends on `notify 8.2.0`, NOT
`notify 9.0.0-rc.2`. Both versions appear in the dependency tree during Phase 1
(stubs don't reference either). When Phase 4 implements the watcher, must
choose ONE of:

- **Option A**: Downgrade `notify` to `"8"` → works with `notify-debouncer-full 0.7.0`
- **Option B**: Keep `notify = "9.0.0-rc.2"` → must implement debouncing manually
  (drop `notify-debouncer-full`)
- **Option C**: Wait for `notify-debouncer-full` to release a version supporting
  notify 9.x

Documented in ADR 0015.

### 2. notify = "9" Cargo SemVer

`notify = "9"` is rejected by cargo's SemVer resolver because v9 is still RC.
Must specify `notify = "9.0.0-rc.2"` explicitly in Cargo.toml. Cargo accepts
this as a pinned pre-release version.

### 3. clippy::unused_async on Stub Functions

All stub `async fn` functions that contain only `todo!()` trigger
`clippy::unused_async` under `-D clippy::pedantic`. Applied
`#[allow(clippy::unused_async)]` to each:
- `shim::run`
- `daemon::run`
- `installer::install`, `installer::update`, `installer::reinstall`,
  `installer::uninstall`

This suppression should be removed in Phase 3–6 when implementations are added.

### 4. to_response() Pattern: reason Field in details

The `to_response()` match arms for errors with both a `path` and `reason` field
(e.g., `IpcError::ConnectionFailed`, `LockError::AcquisitionFailed`,
`WatcherError::InitFailed`) include only the `path` in `details`, relying on
`inner.to_string()` (which does include `reason`) for the `message` field.
The `reason` field is not duplicated in `details`. This is intentional — the
full error message already contains the reason; `details` provides the structured
data that clients may want to act on programmatically.

### 5. No mcp-sdk References Existed in src/ or tests/

Confirmed via grep before removing mcp-sdk: zero references in any Rust source
file. The old server code used `mcp-sdk` but had already been updated to not
reference it in the existing test suite.

---

## Next Steps

### Phase 1.5: Prerequisites

- **T088**: Update existing contract/integration test files to remove mcp-sdk
  imports (they were confirmed absent, but verify no test uses mcp-sdk types)
- **T089**: Build process-based test harness for spawning daemon processes

### Phase 2: Foundational (IPC Transport & Lockfile)

- Implement `IpcRequest`, `IpcResponse`, `IpcError` structs with serde in
  `src/daemon/protocol.rs`
- Implement `DaemonState` and `DaemonStatus` enum
- Implement daemon lockfile management (`fd-lock`, PID file, stale detection)
- Implement IPC server (LocalSocketListener accept loop, JSON-RPC framing)
- Wire IPC server into `daemon::run()` lifecycle

### Known Open Issues

- **notify version decision** (Phase 4): Must choose between notify 8.x+debouncer
  or notify 9.x+manual debouncing before implementing the watcher.
- **Windows named pipe complexity**: Platform-specific endpoint naming and accept
  loop may need conditional compilation (`#[cfg(windows)]`).
- The stub modules have no `#[cfg(test)]` blocks — test modules will be added in
  Phase 1.5 (T088, T089) and Phase 2 (T010–T012).

---

## Context to Preserve

- **Branch**: `004-refactor-engram-server-as-plugin` (all work on this branch)
- **Spec dir**: `specs/004-refactor-engram-server-as-plugin/`
- **ADR**: `docs/adrs/0015-rmcp-migration-and-plugin-architecture.md`
- **Error code ranges**: 8xxx = IPC/daemon, 9xxx = installer
- **clippy allow**: `#[allow(clippy::unused_async)]` on stub async functions —
  remove when implementations are added in Phases 2–6
- **Test count baseline**: ~282 tests, all passing after Phase 1
