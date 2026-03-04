# Session Memory: 004-refactor-engram-server-as-plugin Phase 1.5

**Date**: 2026-03-04  
**Branch**: `004-refactor-engram-server-as-plugin`  
**Phase**: 1.5 — Prerequisites  

## Tasks Completed

### T088 — Remove mcp-sdk imports from tests (no-op)

**Status**: ✅ Complete (no-op)

Searched all files under `tests/` for patterns `mcp.sdk`, `mcp_sdk`, `McpSdk`. No references found. Phase 1 had already removed any mcp-sdk usage; tests use direct `serde_json::json!()` construction and `engram::` library types throughout.

### T089 — Process-based daemon test harness

**Status**: ✅ Complete

**Created**: `tests/helpers/mod.rs`  
**Cargo.toml entry added**: `[[test]] name = "helpers_daemon_harness" path = "tests/helpers/mod.rs"`

#### Implementation decisions

| Decision | Rationale |
|----------|-----------|
| `std::process::Command` (not `tokio::process`) | `Drop` must kill synchronously; mixing async child with sync drop requires `std::process::Child` which supports `.kill()` / `.wait()` in drop without a runtime |
| `#[cfg(not(windows))]` for Unix branch | Covers Linux, macOS, FreeBSD — all non-Windows platforms use Unix domain socket at `{workspace}/.engram/run/engram.sock` |
| `std::fs::metadata` for Windows pipe readiness | Named pipes at `\\.\pipe\*` return `Ok` from `metadata` once the server is listening; avoids `unsafe` WinAPI calls |
| `sha2` + `hex` imports inside `#[cfg(windows)]` block | Avoids unused-import warnings on non-Windows; both crates are in `[dependencies]` so available to test code |
| `const MAX_ATTEMPTS: u32 = 30` declared first in `spawn()` | Required by `clippy::items_after_statements` (pedantic) — consts must precede `let` bindings |
| `env!("CARGO_BIN_EXE_engram")` | Cargo sets this env var for test binaries in the same package as the binary target; ensures tests use the freshly built binary |
| Exponential backoff: 10ms start, 2× per step, 500ms max | Balances fast detection vs. CPU spinning; 30 attempts at max delay = ~15s max wait, generous for daemon startup |

#### IPC path naming (per ADR 0015)

- **Unix**: `{workspace}/.engram/run/engram.sock`
- **Windows**: `\\.\pipe\engram-{sha256_prefix_16}` — first 16 hex chars (8 bytes) of SHA-256 of canonical workspace path

#### Test coverage added

- `ipc_path_unix_format` — asserts Unix socket path is `{workspace}/.engram/run/engram.sock`  
- `ipc_path_windows_format` — asserts Windows pipe name format and 16-char hex suffix  
- `ipc_path_windows_unique_per_workspace` — asserts distinct workspaces produce distinct pipe names  

#### Gates passed

- ✅ `cargo check --all-targets`
- ✅ `cargo fmt --all -- --check`
- ✅ `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
- ✅ `cargo test` (all existing tests pass; 2 new harness tests pass)

## State at Session End

- Phase 1.5: **complete** (both tasks `[X]`)
- Phase 2: IPC transport and lockfile implementation — next up
- `DaemonHarness::spawn()` is implemented correctly but will error until Phase 2 delivers a working `engram daemon` subcommand (the daemon currently `todo!()` panics)
