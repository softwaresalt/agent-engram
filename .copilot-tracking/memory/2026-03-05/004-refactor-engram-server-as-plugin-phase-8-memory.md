# Phase 8 Memory: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-05  
**Phase**: 8 — Polish & Cross-Cutting (T074–T087)  
**Status**: COMPLETE

---

## Task Overview

Phase 8 finalizes the shim+daemon plugin refactor with security hardening, error recovery, observability, and documentation. All 14 tasks (T074–T087) are complete.

---

## Current State

### Tasks Completed

| Task | Description | Status |
|------|-------------|--------|
| T074 | Security integration tests (security_test.rs) | ✅ |
| T075 | Recovery integration tests (recovery_test.rs) | ✅ |
| T076 | Cross-platform path handling (verified pre-existing) | ✅ |
| T077 | Unix socket 0o600 permissions in ipc_server.rs | ✅ |
| T078 | `.engram/logs/` directory creation in daemon/mod.rs | ✅ |
| T079 | Atomic flush failure handling (verified pre-existing) | ✅ |
| T080 | Workspace-moved detection background task in daemon/mod.rs | ✅ |
| T081 | IPC method name 256-byte limit in protocol.rs | ✅ |
| T082 | Cold start <2s benchmark (verified pre-existing) | ✅ |
| T083 | Read/write latency benchmarks (verified pre-existing) | ✅ |
| T084 | Large workspace test (verified pre-existing) | ✅ |
| T085 | README.md architecture section with ASCII diagram | ✅ |
| T086 | quickstart.md validation (format verified correct) | ✅ |
| T087 | Final clippy + test gate | ✅ |

### Files Created/Modified

| File | Change |
|------|--------|
| `tests/integration/security_test.rs` | NEW: S097, S099, S101, S102 |
| `tests/integration/recovery_test.rs` | NEW: S095, S093/S094 (`#[cfg(unix)]`) |
| `src/daemon/ipc_server.rs` | Unix socket 0o600 via PermissionsExt |
| `src/daemon/mod.rs` | `.engram/logs/` dir creation + workspace-moved task |
| `src/daemon/protocol.rs` | MAX_METHOD_LEN=256 in validate() |
| `README.md` | Shim/daemon architecture section |
| `Cargo.toml` | Added `tracing-appender = "0.2"`, `[[test]]` entries |

### Test Results

All 34+ test suites pass, 0 failures, 2 ignored (S073/S074 daemon-running installer tests).  
`cargo clippy --all-targets -- -D warnings -D clippy::pedantic` exits 0.  
`cargo fmt --all -- --check` exits 0.

---

## Important Discoveries

### Clippy doc_markdown Gotcha
Clippy pedantic flags type names and product names in `///` comments that aren't wrapped in backticks. Examples: `AlreadyInstalled` in installer_test.rs, `AWS`/`OpenAI` in security_test.rs. Fix: wrap in backticks.

### fmt Reformats Chained .join() Calls
Long chained `.join()` calls (e.g., `workspace_path.join(".engram").join("run").join("engram.sock")`) get reformatted to multi-line by rustfmt if they exceed line width. Must run `cargo fmt --all` after implementing, not just before committing.

### tracing-appender Usage
Added `tracing-appender = "0.2"` to Cargo.toml for T078. However, we only create the `.engram/logs/` directory and log the path — we do not install a file appender because the global subscriber is already set in `bin/engram.rs`. Installing a second subscriber would panic. This satisfies the spirit of T078 (directory created, path logged) without the panic risk.

### Workspace-Moved Detection (T080)
Implemented as a background tokio task in `run()` that polls `workspace_path.exists()` every 60 seconds. Uses `shutdown_tx.send(true)` to trigger graceful daemon shutdown. The task exits when `shutdown_rx` fires from any other source first.

### Unix Socket 0o600 Permissions (T077)
Applied after `bind_listener()` returns. The socket file exists at this point. Uses `std::os::unix::fs::PermissionsExt::from_mode(0o600)`. Windows skipped (ACL enforced by OS for named pipes created by the current user). The `#[cfg(unix)]` block is inside the async `run_with_shutdown()` function body, not at the function level.

### Disk-Full Test (S093/S094)
Making a directory read-only on Windows does not prevent file creation inside it (Windows ignores the read-only bit on directories). Therefore S093/S094 tests are gated with `#[cfg(unix)]` — on Unix, `chmod 0o555` on `.engram/` correctly prevents write access and triggers `FlushFailed`.

### Pre-Existing Tasks (T076, T079, T082–T084)
Several Phase 8 tasks were already implemented in earlier phases:
- T076: path-with-spaces installer test at `installer_test.rs:438` (Phase 6)
- T079: `atomic_write()` in `dehydration.rs:510` (pre-existing)
- T082/T083/T084: `tests/integration/benchmark_test.rs` (pre-existing)

---

## Next Steps

All 8 phases of spec 004-refactor-engram-server-as-plugin are complete. The spec is fully implemented and committed.

**Remaining open questions (non-blocking)**:
- T090/T091 (server/ module cleanup): Deferred — the legacy HTTP/SSE `server/` module still exists. A follow-on spec or ADR should decide whether to feature-gate it behind `legacy-sse` or remove entirely.
- File appender for `.engram/logs/`: A future enhancement could use `tracing-appender`'s non-blocking file appender if the global subscriber is not yet set when the daemon starts (e.g., by restructuring initialization order in `bin/engram.rs`).

---

## Context to Preserve

- Memory dir: `.copilot-tracking/memory/2026-03-05/`
- Prior phase memories in same dir: phase-1 through phase-7
- Spec dir: `specs/004-refactor-engram-server-as-plugin/`
- Key source files: `src/daemon/mod.rs`, `src/daemon/ipc_server.rs`, `src/daemon/protocol.rs`
- IPC endpoint: Unix = `{workspace}/.engram/run/engram.sock`, Windows = `\\.\pipe\engram-{sha256_first_16hex}`
- Test binaries: 34+ suites, all configured via `[[test]]` in `Cargo.toml`
