# Changes: Shim + Daemon Bug Fixes (Live E2E Issues)

**Date**: 2026-03-07  
**Plan**: `.copilot-tracking/plans/2026-03-07-shim-daemon-fixes-plan.md`  
**Status**: Complete — all three fixes implemented, clippy clean, tests pass.

---

## Files Modified

### Fix 1 — Configurable Startup Timeout

**File**: `src/shim/lifecycle.rs`

**Summary**:
- Replaced `const READY_TIMEOUT_MS: u64 = 2_000` with `const DEFAULT_READY_TIMEOUT_MS: u64 = 10_000` (5× increase for slow CI environments and cold startup paths).
- Extracted parsing logic into a pure function `fn parse_timeout_ms(raw: Option<&str>) -> u64` that converts an optional env-var string to milliseconds, falling back to the default when absent or non-numeric.
- Added `fn ready_timeout_ms() -> u64` which reads `ENGRAM_READY_TIMEOUT_MS` and delegates to `parse_timeout_ms`.
- Updated `poll_until_ready` to call `ready_timeout_ms()` at the top of the function and pass the resulting value through to the `NotReady { timeout_ms }` error variant.
- Fixed stale doc-comment on `ensure_daemon_running` which still referenced the deleted constant name.
- Added 3 `#[cfg(test)]` unit tests:
  - `ready_timeout_default_is_10_seconds` — `parse_timeout_ms(None)` returns 10 000.
  - `ready_timeout_env_var_overrides_default` — valid numeric string is parsed correctly.
  - `ready_timeout_invalid_env_var_falls_back_to_default` — non-numeric string falls back to default.

**Deviation from plan**: Tests use `parse_timeout_ms()` directly instead of manipulating the environment with `set_var`/`remove_var`. In Rust 2024 edition, `std::env::set_var` and `remove_var` are classified as `unsafe` functions, and `#![forbid(unsafe_code)]` is in effect. The pure-function approach is safer, simpler, and tests the exact same parsing logic.

---

### Fix 2 — Static Tool Catalog for `tools/list`

**File created**: `src/shim/tools_catalog.rs`

**Summary**:
- New module containing a single public constant `TOOL_COUNT: usize = 35` and a public function `all_tools() -> Vec<Tool>` (using `rmcp::model::Tool`).
- All 35 tools from `src/tools/mod.rs` dispatch are represented with meaningful descriptions and typed JSON Schema `input_schema` objects.
- A private helper `fn schema(v: Value) -> Arc<Map<String, Value>>` converts JSON object literals into the `Arc<JsonObject>` required by `Tool::new`, using a safe match instead of `unwrap()`.
- 3 unit tests:
  - `tool_count_matches_dispatch` — `all_tools().len() == TOOL_COUNT` (35).
  - `tool_names_are_unique` — no duplicate tool names.
  - `all_dispatch_names_present` — spot-checks 10 key tool names from the dispatch table.

**File modified**: `src/shim/mod.rs`
- Added `pub mod tools_catalog;` alongside the existing module declarations.

**File modified**: `src/shim/transport.rs`
- Updated `ShimHandler::list_tools` to return `ListToolsResult { tools: crate::shim::tools_catalog::all_tools(), next_cursor: None, meta: None }` instead of `ListToolsResult::default()`.
- Added a doc comment explaining the static-catalog rationale (local response without daemon round-trip).

---

### Fix 3 — Stale PID Detection

**File modified**: `src/daemon/lockfile.rs`

**Summary**:
- Added imports: `use sysinfo::{Pid, System}` and `use tracing::warn`.
- Refactored `DaemonLock::acquire` from a monolithic method to a thin public wrapper that calls `acquire_inner(workspace, true)`.
- Extracted the full lock-acquisition body into `fn acquire_inner(workspace: &Path, allow_retry: bool) -> Result<DaemonLock, EngramError>`, a module-private free function.
- In the `WouldBlock` match arm, replaced the single-line `unwrap_or(0)` with a three-branch `match read_pid(&pid_path)`:
  - **PID alive** (`is_process_alive(pid)` returns `true`): `warn!(pid, "daemon lock held by live process, cannot start")` → return `AlreadyHeld`.
  - **PID dead, retry allowed**: `warn!(pid, "found stale lockfile, cleaning up")` → remove the file → call `acquire_inner(workspace, false)` to retry once.
  - **PID dead, retry already attempted**: `warn!(pid, "stale lockfile cleanup retry failed; lock still held")` → return `AlreadyHeld`.
  - **PID unreadable**: `warn!("lockfile held but PID unreadable")` → return `AlreadyHeld { pid: 0 }`.
- Added `fn is_process_alive(pid: u32) -> bool` using `sysinfo 0.30.x`:
  - Returns `false` immediately for `pid == 0`.
  - Creates a fresh `System`, calls `sys.refresh_process(Pid::from_u32(pid))`, and returns the `bool` result.

**Deviation from plan**: The plan suggested `ProcessesToUpdate::Some(&[...])` which is a sysinfo 0.31+ API. The project uses sysinfo 0.30.13, whose API for single-process refresh is `sys.refresh_process(Pid) -> bool`. The `from_u32` method is available in sysinfo 0.30. No existing tests were broken; the `s029_acquire_with_stale_pid_file_succeeds` test (which pre-writes an astronomically high PID and expects acquire to succeed) continues to pass because the OS had already released the lock — `try_write()` succeeds directly without hitting the `WouldBlock` branch.

---

## Test Results

### `cargo build` — ✅ Clean
```
Compiling engram v0.0.1
Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.33s
```

### `cargo clippy --all-targets -- -D warnings -D clippy::pedantic -A ...` — ✅ Clean
```
Checking engram v0.0.1
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.05s
```
No warnings, no errors.

### `cargo test --lib` — ✅ 80 tests pass (up from 74)
New tests added:
- `shim::lifecycle::tests::ready_timeout_default_is_10_seconds`
- `shim::lifecycle::tests::ready_timeout_env_var_overrides_default`
- `shim::lifecycle::tests::ready_timeout_invalid_env_var_falls_back_to_default`
- `shim::tools_catalog::tests::tool_count_matches_dispatch`
- `shim::tools_catalog::tests::tool_names_are_unique`
- `shim::tools_catalog::tests::all_dispatch_names_present`

### `cargo test --test unit_lockfile` — ✅ 3 tests pass
- `s027_acquire_on_fresh_workspace_succeeds_and_writes_pid`
- `s029_acquire_with_stale_pid_file_succeeds`
- `s032_acquire_drop_acquire_again_succeeds`

### `cargo test --test unit_proptest --test unit_proptest_serialization --test unit_parsing --test unit_ttl` — ✅ 49 tests pass

### Contract tests — ✅ 100 tests pass
- `contract_error_codes`: 8 passed
- `contract_lifecycle`: 9 passed
- `contract_quickstart`: 5 passed
- `contract_read`: 25 passed
- `contract_write`: 53 passed

### Integration tests (subset) — ✅ 12 tests pass
- `integration_connection`: 2 passed
- `integration_hydration`: 10 passed

---

## Architecture Notes

- The `parse_timeout_ms` helper is `pub(crate)` only within the `lifecycle` module via `use super::*` in the test module — it is not exposed from the crate boundary.
- `acquire_inner` is a module-private free function, not a method, which avoids borrowing the already-leaked `rw_lock` reference in the recursive retry case.
- The stale-lock retry is bounded to exactly one level of recursion via the `allow_retry: bool` flag, preventing any risk of infinite recursion even if OS behavior is unexpected.
- The `Box::leak` pattern for `RwLock<File>` is preserved exactly; in the stale-lock retry path, the first leaked allocation is abandoned (the file handle it holds is released when the leaked memory is claimed back by the process on exit). This is an acceptable one-allocation overhead per stale-lock detection event, consistent with the existing memory model documented in the module header.
