<!-- markdownlint-disable-file -->
# Implementation Review: Shim + Daemon Bug Fixes

**Review Date**: 2026-03-07  
**Related Plan**: `2026-03-07-shim-daemon-fixes-plan.md`  
**Related Changes**: `2026-03-07-shim-daemon-fixes-changes.md`  
**Related Research**: None

---

## Review Summary

Three bug fixes were reviewed: (1) configurable startup timeout in `shim/lifecycle.rs`, (2) static tool
catalog in the new `shim/tools_catalog.rs` wired into `shim/transport.rs`, and (3) stale PID detection
with sysinfo liveness check in `daemon/lockfile.rs`. All fixes were verified to be correct and complete.
All 80 lib tests pass (6 new tests added), all 3 lockfile integration tests pass, and clippy pedantic
is clean.

---

## Implementation Checklist

### Fix 1 — Configurable Startup Timeout (`src/shim/lifecycle.rs`)

* [x] `DEFAULT_READY_TIMEOUT_MS = 10_000` (not 2_000)
  * Source: `lifecycle.rs` line 28
  * Status: Verified
  * Evidence: `const DEFAULT_READY_TIMEOUT_MS: u64 = 10_000;`

* [x] `ENGRAM_READY_TIMEOUT_MS` env var is read correctly
  * Source: `lifecycle.rs` lines 44–46
  * Status: Verified
  * Evidence: `std::env::var("ENGRAM_READY_TIMEOUT_MS").ok().as_deref()` passed to `parse_timeout_ms`

* [x] Invalid env var values fall back to default (no panic)
  * Source: `lifecycle.rs` lines 34–37
  * Status: Verified
  * Evidence: `s.parse::<u64>().ok()` returns `None` on parse failure; `.unwrap_or(DEFAULT_READY_TIMEOUT_MS)` supplies the fallback — no `unwrap()` call

* [x] `NotReady { timeout_ms }` uses the runtime value
  * Source: `lifecycle.rs` lines 157 and 182–184
  * Status: Verified
  * Evidence: `let timeout_ms = ready_timeout_ms();` at line 157; `DaemonError::NotReady { timeout_ms }` at line 183

* [x] Unit tests: default, override, invalid-fallback all present
  * Source: `lifecycle.rs` lines 194–210
  * Status: Verified
  * Evidence: `ready_timeout_default_is_10_seconds`, `ready_timeout_env_var_overrides_default`, `ready_timeout_invalid_env_var_falls_back_to_default` — all pass

* [x] No `unwrap()`/`expect()` in non-test code
  * Status: Verified
  * Evidence: `grep` found zero matches across `lifecycle.rs`

---

### Fix 2 — Static Tool Catalog (`src/shim/tools_catalog.rs` + `transport.rs`)

* [x] `all_tools()` returns non-empty Vec
  * Source: `tools_catalog.rs` lines 33–666
  * Status: Verified
  * Evidence: 35 `Tool::new(` call sites confirmed by grep count

* [x] Tool count ≥ 35 — all 35 dispatch arms covered
  * Source: `tools_catalog.rs` line 18 (`pub const TOOL_COUNT: usize = 35`)
  * Status: Verified
  * Evidence: Cross-check below confirms all 35 dispatch names present in catalog (see Completeness Check)

* [x] No duplicate tool names
  * Status: Verified
  * Evidence: `tool_names_are_unique` test passes; code review confirmed all names are distinct

* [x] `ShimHandler::list_tools` returns the catalog (not empty default)
  * Source: `transport.rs` lines 121–131
  * Status: Verified
  * Evidence: `tools: crate::shim::tools_catalog::all_tools()` replaces former `ListToolsResult::default()`

* [x] Tests: count, uniqueness, spot-check present
  * Source: `tools_catalog.rs` lines 674–720
  * Status: Verified
  * Evidence: `tool_count_matches_dispatch`, `tool_names_are_unique`, `all_dispatch_names_present` — all pass

* [x] Tool descriptions are meaningful (not empty strings)
  * Status: Verified
  * Evidence: Every `Tool::new(name, description, schema)` call has a non-empty, descriptive second argument; all 35 reviewed manually

* [x] `pub mod tools_catalog` added to `src/shim/mod.rs`
  * Source: `mod.rs` line 9
  * Status: Verified
  * Evidence: `pub mod tools_catalog;` present alongside existing module declarations

---

### Fix 3 — Stale PID Detection (`src/daemon/lockfile.rs`)

* [x] `unwrap_or(0)` is GONE from the `WouldBlock` arm
  * Status: Verified
  * Evidence: `grep` found zero `unwrap` calls in `lockfile.rs`; the arm now uses `read_pid(&pid_path)` returning `Option<u32>`

* [x] Liveness check using sysinfo is present
  * Source: `lockfile.rs` lines 189–195
  * Status: Verified
  * Evidence: `fn is_process_alive(pid: u32) -> bool` using `sysinfo::System::refresh_process(Pid::from_u32(pid))`

* [x] Live PID → returns appropriate error with real PID
  * Source: `lockfile.rs` lines 155–157
  * Status: Verified
  * Evidence: `Some(pid) if is_process_alive(pid) => { warn!(...); Err(EngramError::Lock(LockError::AlreadyHeld { pid })) }`

* [x] Dead PID → cleans up stale file AND retries/proceeds
  * Source: `lockfile.rs` lines 159–165
  * Status: Verified
  * Evidence: `std::fs::remove_file(&pid_path)` followed by `acquire_inner(workspace, false)` recursive call

* [x] Empty/unreadable PID file → handled gracefully (no panic)
  * Source: `lockfile.rs` lines 172–174
  * Status: Verified
  * Evidence: `None => { warn!(...); Err(EngramError::Lock(LockError::AlreadyHeld { pid: 0 })) }`

* [x] Retry logic is bounded (not infinite loop)
  * Source: `lockfile.rs` lines 78, 159, 165
  * Status: Verified
  * Evidence: `allow_retry: bool` parameter; first call passes `true`, recursive retry passes `false`, so at most one level of recursion

* [x] `is_process_alive` helper is present
  * Source: `lockfile.rs` lines 185–195
  * Status: Verified
  * Evidence: Module-private function with doc-comment, pid-0 guard, fresh `System` allocation, returns `sys.refresh_process(...)` bool directly

---

## Validation Results

### Convention Compliance

* No `unsafe` code: **Passed** — grep across all five changed files returned zero matches
* No `unwrap()`/`expect()` in production paths: **Passed**
  * `transport.rs` line 100 uses `.unwrap_or(Value::Null)` (safe fallback, not `unwrap()`)
  * All other potential panic sites use `?`, `ok()`, `unwrap_or`, or `map_err`
* `#![forbid(unsafe_code)]`: **Respected** — `set_var`/`remove_var` avoided in tests (uses pure `parse_timeout_ms` directly); change log documents this deviation from plan explicitly

### Validation Commands

* `cargo test --lib` (80 tests): **Passed** — 80 passed, 0 failed; 6 new tests included
  * `shim::lifecycle::tests::ready_timeout_default_is_10_seconds` ✅
  * `shim::lifecycle::tests::ready_timeout_env_var_overrides_default` ✅
  * `shim::lifecycle::tests::ready_timeout_invalid_env_var_falls_back_to_default` ✅
  * `shim::tools_catalog::tests::tool_count_matches_dispatch` ✅
  * `shim::tools_catalog::tests::tool_names_are_unique` ✅
  * `shim::tools_catalog::tests::all_dispatch_names_present` ✅

* `cargo test --test unit_lockfile` (3 tests): **Passed** — 3 passed, 0 failed
  * `s027_acquire_on_fresh_workspace_succeeds_and_writes_pid` ✅
  * `s029_acquire_with_stale_pid_file_succeeds` ✅
  * `s032_acquire_drop_acquire_again_succeeds` ✅

* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic [allowances]`: **Passed** — no warnings, no errors

---

## Completeness Check — Dispatch vs Catalog

All 35 names from `src/tools/mod.rs` match arm verified against `tools_catalog.rs`:

| # | Dispatch name | In catalog |
|---|---------------|-----------|
| 1 | `set_workspace` | ✅ |
| 2 | `get_daemon_status` | ✅ |
| 3 | `get_workspace_status` | ✅ |
| 4 | `create_task` | ✅ |
| 5 | `update_task` | ✅ |
| 6 | `add_blocker` | ✅ |
| 7 | `register_decision` | ✅ |
| 8 | `flush_state` | ✅ |
| 9 | `get_task_graph` | ✅ |
| 10 | `check_status` | ✅ |
| 11 | `query_memory` | ✅ |
| 12 | `get_ready_work` | ✅ |
| 13 | `add_label` | ✅ |
| 14 | `remove_label` | ✅ |
| 15 | `add_dependency` | ✅ |
| 16 | `get_compaction_candidates` | ✅ |
| 17 | `apply_compaction` | ✅ |
| 18 | `claim_task` | ✅ |
| 19 | `release_task` | ✅ |
| 20 | `defer_task` | ✅ |
| 21 | `undefer_task` | ✅ |
| 22 | `pin_task` | ✅ |
| 23 | `unpin_task` | ✅ |
| 24 | `get_workspace_statistics` | ✅ |
| 25 | `batch_update_tasks` | ✅ |
| 26 | `add_comment` | ✅ |
| 27 | `index_workspace` | ✅ |
| 28 | `sync_workspace` | ✅ |
| 29 | `link_task_to_code` | ✅ |
| 30 | `unlink_task_from_code` | ✅ |
| 31 | `map_code` | ✅ |
| 32 | `list_symbols` | ✅ |
| 33 | `get_active_context` | ✅ |
| 34 | `unified_search` | ✅ |
| 35 | `impact_analysis` | ✅ |

**Missing tools**: None.

---

## Additional or Deviating Changes

* **`src/shim/lifecycle.rs`** — Tests use `parse_timeout_ms()` directly rather than manipulating the
  environment with `set_var`/`remove_var`.
  * Reason: Documented in changes log. `std::env::set_var` is `unsafe` in Rust 2024 edition and the
    crate has `#![forbid(unsafe_code)]`. The pure-function approach tests identical parsing logic with
    no safety compromise. Deviation is intentional and correct.

* **`src/daemon/lockfile.rs`** — `is_process_alive` uses `sys.refresh_process(Pid::from_u32(pid))`
  (sysinfo 0.30 API) rather than `ProcessesToUpdate::Some(&[...])` (sysinfo 0.31+ API).
  * Reason: Documented in changes log. Crate depends on sysinfo 0.30.13; the 0.30 API is the only
    correct choice. Deviation is intentional and correct.

---

## Missing Work

None identified.

---

## Follow-Up Work

### Identified During Review

* **Stale-lock test with a live PID** — `unit_lockfile` tests do not exercise the `is_process_alive`
  live-process branch (i.e., the case where `WouldBlock` fires and the recorded PID is of a running
  process). This is inherently hard to test without two concurrent processes, but a future improvement
  could fork a child, verify `acquire()` returns `AlreadyHeld`, then reap the child.
  * Context: Minor coverage gap; existing tests do confirm the dead-PID (stale) and clean-start paths.
  * Recommendation: Track as a low-priority test-coverage task; functional behavior is correct.

* **`all_dispatch_names_present` test is a spot-check (10 of 35)** — the test verifies only 10 names.
  A future improvement would verify all 35 names to make the contract test exhaustive.
  * Context: Minor — the `tool_count_matches_dispatch` test confirms count parity; adding full name
    coverage would eliminate any risk of two tools swapping names while count stays the same.
  * Recommendation: Expand the `required` array in the test to all 35 names.

---

## Review Completion

**Overall Status**: Complete  
**Critical Findings**: 0  
**Major Findings**: 0  
**Minor Findings**: 0 (two follow-up items noted above are improvements, not defects)  

**Reviewer Notes**: All three fixes are implemented correctly and completely. The two documented
deviations from the plan are improvements over the plan (unsafe avoidance, correct sysinfo API
version). The codebase is clean: no unsafe, no panicking unwraps in production paths, clippy pedantic
passes, and all 83 relevant tests pass. No rework is required.
