# Implementation Plan: Shim + Daemon Bug Fixes (Live E2E Issues)

**Date**: 2026-03-07  
**Priority**: HIGH (Bug 1), MEDIUM (Bugs 2 & 3)  
**Sources**: `.copilot-tracking/research/2026-03-07-shim-daemon-fixes.md`, live E2E test failures  

---

## Problem Statement

Three bugs were discovered during live end-to-end testing of the engram shim+daemon MCP system:

1. **Shim startup timeout too short** — `READY_TIMEOUT_MS = 2_000` is hardcoded. On Windows, SurrealDB embedded (surrealkv/RocksDB) initialization + named pipe binding regularly takes 1–4 seconds, causing the shim to report `DaemonError::NotReady` before the daemon is actually ready.

2. **`tools/list` returns empty** — `ShimHandler::list_tools` is a stub that always returns `ListToolsResult::default()` (an empty tool list). MCP clients (Claude Desktop, Cursor, GitHub Copilot) cannot discover any tools and show an empty capability surface.

3. **Stale PID `0` logged on startup** — In `DaemonLock::acquire`, the `WouldBlock` branch calls `read_pid().unwrap_or(0)`. If the PID file is empty during a race window (e.g., the winning daemon has the lock but hasn't written its PID yet, or the PID file was just truncated), this emits `LockError::AlreadyHeld { pid: 0 }` — a confusing and misleading diagnostic.

---

## Approach Summary

- **Bug 1**: Replace the compile-time constant with a runtime env-var override (`ENGRAM_READY_TIMEOUT_MS`), raising the default from 2 s to 10 s. Pattern mirrors `ENGRAM_IDLE_TIMEOUT_MS` already in the codebase.
- **Bug 2**: Create `src/shim/tools_catalog.rs` containing a static `fn all_tools() -> Vec<Tool>` with one `Tool` entry per `dispatch()` match arm (35 tools). Wire it into `ShimHandler::list_tools`. Add a unit test asserting count + names.
- **Bug 3**: In the `WouldBlock` branch, replace `unwrap_or(0)` with a `sysinfo`-based liveness check (already a dependency). Emit a `warn!` log when the PID is unreadable; emit an `info!` log with the live process's PID when readable. This makes `AlreadyHeld { pid }` always carry a real PID for a live process.

---

## Task List

### Task 1 — Configurable Startup Timeout (Bug 1)

**ID**: FIX-001  
**Priority**: HIGH  
**File**: `src/shim/lifecycle.rs`

#### Exact Change

Replace lines 19–28:

```rust
// ── Backoff constants ─────────────────────────────────────────────────────────

/// Maximum number of health-check poll attempts after spawning the daemon.
const MAX_BACKOFF_ATTEMPTS: u32 = 30;
/// Initial delay before the first poll (milliseconds).
const INITIAL_BACKOFF_MS: u64 = 10;
/// Maximum delay cap per backoff step (milliseconds).
const MAX_BACKOFF_MS: u64 = 500;
/// Total wall-clock budget allowed for the ready-wait loop (milliseconds).
const READY_TIMEOUT_MS: u64 = 2_000;
```

With:

```rust
// ── Backoff constants ─────────────────────────────────────────────────────────

/// Maximum number of health-check poll attempts after spawning the daemon.
const MAX_BACKOFF_ATTEMPTS: u32 = 30;
/// Initial delay before the first poll (milliseconds).
const INITIAL_BACKOFF_MS: u64 = 10;
/// Maximum delay cap per backoff step (milliseconds).
const MAX_BACKOFF_MS: u64 = 500;
/// Default wall-clock budget allowed for the ready-wait loop (milliseconds).
///
/// On Windows, SurrealDB embedded init + named pipe binding can take 1–4 s.
/// Override with `ENGRAM_READY_TIMEOUT_MS` for operator-tunable deployments.
const DEFAULT_READY_TIMEOUT_MS: u64 = 10_000;

/// Read the ready-wait timeout from the `ENGRAM_READY_TIMEOUT_MS` environment
/// variable, falling back to [`DEFAULT_READY_TIMEOUT_MS`].
fn ready_timeout_ms() -> u64 {
    std::env::var("ENGRAM_READY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_READY_TIMEOUT_MS)
}
```

Then update `poll_until_ready` (line 138) to use the function instead of the constant:

```rust
async fn poll_until_ready(endpoint: &str) -> Result<(), EngramError> {
    let timeout_ms = ready_timeout_ms();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    // ... (rest of body unchanged)

    // In the error return at the bottom, pass `timeout_ms` instead of
    // the removed `READY_TIMEOUT_MS` constant:
    Err(EngramError::Daemon(DaemonError::NotReady { timeout_ms }))
}
```

**Also update the doc-comment** on `poll_until_ready` to reference `DEFAULT_READY_TIMEOUT_MS` and `ENGRAM_READY_TIMEOUT_MS` instead of the removed `READY_TIMEOUT_MS`.

#### Acceptance Criteria

- `cargo build` passes with zero warnings.
- `READY_TIMEOUT_MS` constant no longer exists; `DEFAULT_READY_TIMEOUT_MS = 10_000` is used when the env var is absent.
- Setting `ENGRAM_READY_TIMEOUT_MS=5000` causes `ready_timeout_ms()` to return `5000`.
- Existing `shim_lifecycle_test.rs` tests still pass (they use 10 s harness timeout, unaffected by production constant change).
- New unit test `t_ready_timeout_env_var_override` passes (see Test Plan below).

---

### Task 2 — Static Tool Catalog (Bug 2)

**ID**: FIX-002  
**Priority**: MEDIUM  
**Files**:  
- `src/shim/tools_catalog.rs` (new file)  
- `src/shim/mod.rs` (add `pub mod tools_catalog;`)  
- `src/shim/transport.rs` (wire `list_tools`)

#### Exact Change — New File `src/shim/tools_catalog.rs`

Create a new file with 35 static tool entries matching every arm in `src/tools/mod.rs::dispatch()`. Each entry uses `rmcp::model::{Tool, ToolInputSchema}` with `serde_json::json!` for the schema.

Structure:
```rust
//! Static tool catalog for the engram shim.
//!
//! This module defines the complete set of MCP tools that the shim
//! advertises via `tools/list`. Every entry MUST correspond to a match
//! arm in `src/tools/mod.rs::dispatch()`. A unit test in this module
//! verifies the count matches the dispatch table.
//!
//! # Maintenance
//!
//! When adding or removing a tool in `dispatch()`, update this catalog
//! and the `TOOL_COUNT` constant in the same commit.

use rmcp::model::{Tool, ToolInputSchema};
use serde_json::json;

/// Expected number of tools — keep in sync with `src/tools/mod.rs::dispatch()`.
pub const TOOL_COUNT: usize = 35;

/// Return the complete list of MCP tools supported by the engram daemon.
///
/// This list is generated at call-time from a fixed data set; no allocation
/// is shared across calls. Intended for use in `ShimHandler::list_tools`.
pub fn all_tools() -> Vec<Tool> {
    vec![
        tool(
            "set_workspace",
            "Bind the daemon connection to a workspace path and trigger \
             hydration of tasks and context from `.engram/` files.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the workspace root" }
                },
                "required": ["path"]
            }),
        ),
        tool(
            "get_daemon_status",
            "Report daemon health: uptime, active connection count, and \
             bound workspace paths.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_workspace_status",
            "Report workspace state: task counts by status, context note \
             count, last flush time, and staleness indicators.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "create_task",
            "Create a new task in the active workspace with a title, \
             optional description, and initial status.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "status": { "type": "string" }
                },
                "required": ["title"]
            }),
        ),
        tool(
            "update_task",
            "Change a task's status or description. Records a context note \
             for every transition.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "status": { "type": "string" },
                    "description": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "add_blocker",
            "Block a task by linking it to a blocker task with an optional \
             reason. The blocked task cannot be claimed until unblocked.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "blocker_id": { "type": "string" },
                    "reason": { "type": "string" }
                },
                "required": ["task_id", "blocker_id"]
            }),
        ),
        tool(
            "register_decision",
            "Record an architectural or implementation decision as a \
             context note in the workspace.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["title", "content"]
            }),
        ),
        tool(
            "flush_state",
            "Serialize the current in-memory workspace state to `.engram/` \
             Markdown files on disk.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_task_graph",
            "Return the dependency graph rooted at a task, up to a \
             configurable depth.",
            json!({
                "type": "object",
                "properties": {
                    "root_id": { "type": "string" },
                    "depth": { "type": "integer" }
                }
            }),
        ),
        tool(
            "check_status",
            "Batch lookup of status for one or more work items by ID.",
            json!({
                "type": "object",
                "properties": {
                    "work_item_ids": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["work_item_ids"]
            }),
        ),
        tool(
            "query_memory",
            "Semantic (embedding-based) search across context notes and \
             task descriptions.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer" }
                },
                "required": ["query"]
            }),
        ),
        tool(
            "get_ready_work",
            "List tasks in the active workspace that have no pending \
             blockers and are ready to be started.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "add_label",
            "Add a text label to a task for categorisation or filtering.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "label": { "type": "string" }
                },
                "required": ["task_id", "label"]
            }),
        ),
        tool(
            "remove_label",
            "Remove a previously applied label from a task.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "label": { "type": "string" }
                },
                "required": ["task_id", "label"]
            }),
        ),
        tool(
            "add_dependency",
            "Add a dependency edge from one task to another, indicating \
             that the dependent task cannot start until its dependency is done.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "depends_on_id": { "type": "string" }
                },
                "required": ["task_id", "depends_on_id"]
            }),
        ),
        tool(
            "get_compaction_candidates",
            "List context notes that exceed the staleness threshold and are \
             eligible for compaction into a summary.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "apply_compaction",
            "Compact selected context notes into a summary note, reducing \
             context window usage.",
            json!({
                "type": "object",
                "properties": {
                    "note_ids": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "summary": { "type": "string" }
                },
                "required": ["note_ids", "summary"]
            }),
        ),
        tool(
            "claim_task",
            "Claim a task for exclusive execution. Prevents other agents \
             from starting the same task concurrently.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "release_task",
            "Release a previously claimed task, making it available for \
             other agents to claim.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "defer_task",
            "Mark a task as deferred until a specified time or condition.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "reason": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "undefer_task",
            "Remove the deferred state from a task, making it eligible for \
             ready-work selection again.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "pin_task",
            "Pin a task to prevent automatic status changes or compaction.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "unpin_task",
            "Remove the pinned state from a task.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"]
            }),
        ),
        tool(
            "get_workspace_statistics",
            "Return aggregate statistics for the active workspace: task \
             counts by status, context note totals, and code graph metrics.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "batch_update_tasks",
            "Apply status or description updates to multiple tasks in a \
             single atomic operation.",
            json!({
                "type": "object",
                "properties": {
                    "updates": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "task_id": { "type": "string" },
                                "status": { "type": "string" },
                                "description": { "type": "string" }
                            },
                            "required": ["task_id"]
                        }
                    }
                },
                "required": ["updates"]
            }),
        ),
        tool(
            "add_comment",
            "Add a free-text comment to a task, recorded as a context note.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "comment": { "type": "string" }
                },
                "required": ["task_id", "comment"]
            }),
        ),
        tool(
            "index_workspace",
            "Trigger a full re-index of the workspace source files into the \
             code graph.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "sync_workspace",
            "Incrementally sync changed workspace files into the code graph \
             without a full re-index.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "link_task_to_code",
            "Associate a task with a code symbol (function, struct, module) \
             by path and symbol name.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "file_path": { "type": "string" },
                    "symbol": { "type": "string" }
                },
                "required": ["task_id", "file_path"]
            }),
        ),
        tool(
            "unlink_task_from_code",
            "Remove an existing task-to-code symbol association.",
            json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "file_path": { "type": "string" },
                    "symbol": { "type": "string" }
                },
                "required": ["task_id", "file_path"]
            }),
        ),
        tool(
            "map_code",
            "Return the code graph (files, symbols, dependencies) for a \
             path or subtree.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "depth": { "type": "integer" }
                }
            }),
        ),
        tool(
            "list_symbols",
            "List code symbols (functions, structs, enums, traits) defined \
             within a given file or directory path.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
        ),
        tool(
            "get_active_context",
            "Return the most recent context notes for the active workspace, \
             ordered by recency.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" }
                }
            }),
        ),
        tool(
            "unified_search",
            "Search across tasks, context notes, and code symbols \
             simultaneously using a single query string.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer" }
                },
                "required": ["query"]
            }),
        ),
        tool(
            "impact_analysis",
            "Analyse the potential impact of modifying a code symbol: \
             returns dependent tasks, callers, and related context notes.",
            json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "symbol": { "type": "string" }
                },
                "required": ["file_path"]
            }),
        ),
    ]
}

/// Construct a [`Tool`] with a JSON object input schema.
///
/// `schema_value` must be a `serde_json::Value` of type `object`.
/// The function converts it into a [`ToolInputSchema`] via round-trip
/// deserialization so the rmcp model type is satisfied.
fn tool(name: &str, description: &str, schema_value: serde_json::Value) -> Tool {
    let input_schema: ToolInputSchema = serde_json::from_value(schema_value)
        .expect("static schema must be valid; fix the schema literal in tools_catalog.rs");
    Tool {
        name: name.into(),
        description: Some(description.into()),
        input_schema,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_count_matches_dispatch() {
        assert_eq!(
            all_tools().len(),
            TOOL_COUNT,
            "tools_catalog::all_tools() length must equal TOOL_COUNT ({TOOL_COUNT}); \
             update TOOL_COUNT or add/remove entries to match dispatch() in src/tools/mod.rs"
        );
    }

    #[test]
    fn all_tool_names_are_unique() {
        let tools = all_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let original_len = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), original_len, "tool catalog must not contain duplicate names");
    }

    #[test]
    fn dispatch_tools_present_in_catalog() {
        // Canonical list of all dispatch() arm names from src/tools/mod.rs.
        // Update this array when dispatch() is updated.
        let dispatch_names = [
            "set_workspace",
            "get_daemon_status",
            "get_workspace_status",
            "create_task",
            "update_task",
            "add_blocker",
            "register_decision",
            "flush_state",
            "get_task_graph",
            "check_status",
            "query_memory",
            "get_ready_work",
            "add_label",
            "remove_label",
            "add_dependency",
            "get_compaction_candidates",
            "apply_compaction",
            "claim_task",
            "release_task",
            "defer_task",
            "undefer_task",
            "pin_task",
            "unpin_task",
            "get_workspace_statistics",
            "batch_update_tasks",
            "add_comment",
            "index_workspace",
            "sync_workspace",
            "link_task_to_code",
            "unlink_task_from_code",
            "map_code",
            "list_symbols",
            "get_active_context",
            "unified_search",
            "impact_analysis",
        ];

        let catalog_names: std::collections::HashSet<&str> =
            all_tools().iter().map(|t| t.name.as_str()).collect();

        for name in dispatch_names {
            assert!(
                catalog_names.contains(name),
                "tool '{name}' is in dispatch() but missing from tools_catalog::all_tools()"
            );
        }
    }
}
```

#### Exact Change — `src/shim/mod.rs`

Add `pub mod tools_catalog;` after the existing `pub mod transport;` line:

```rust
pub mod ipc_client;
pub mod lifecycle;
pub mod tools_catalog;   // ← add this line
pub mod transport;
```

#### Exact Change — `src/shim/transport.rs`

Replace the `list_tools` method body (lines 117–123):

```rust
// Before:
/// Return an empty tool list (tools are enumerated by the daemon).
async fn list_tools(
    &self,
    _request: Option<PaginatedRequestParams>,
    _cx: RequestContext<RoleServer>,
) -> Result<ListToolsResult, ErrorData> {
    Ok(ListToolsResult::default())
}

// After:
/// Return the static tool catalog for the engram daemon.
///
/// The catalog is defined in [`crate::shim::tools_catalog`] and mirrors
/// every dispatch arm in `src/tools/mod.rs`. No IPC round-trip is needed
/// because the tool list is compile-time-determined.
async fn list_tools(
    &self,
    _request: Option<PaginatedRequestParams>,
    _cx: RequestContext<RoleServer>,
) -> Result<ListToolsResult, ErrorData> {
    Ok(ListToolsResult {
        tools: crate::shim::tools_catalog::all_tools(),
        next_cursor: None,
    })
}
```

> **Note**: Verify the exact field names for `ListToolsResult` in the `rmcp` crate version in use. If the struct has a different shape (e.g., uses a builder), adapt accordingly. The intent is: `tools` field = `all_tools()`, no pagination cursor.

#### Acceptance Criteria

- `cargo build` passes with zero warnings.
- Calling `list_tools` on `ShimHandler` returns exactly 35 tools.
- Unit tests in `tools_catalog.rs` pass: count == 35, no duplicate names, all dispatch names present.
- `all_tools()` does not panic (the `expect()` in `tool()` helper only fires if a static schema literal is malformed).

---

### Task 3 — Fix Stale PID 0 on Lock Contention (Bug 3)

**ID**: FIX-003  
**Priority**: MEDIUM  
**File**: `src/daemon/lockfile.rs`

#### Exact Change

Replace lines 129–131 in the `WouldBlock` branch:

```rust
// Before (lines 129–131):
Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    let pid = read_pid(&pid_path).unwrap_or(0);
    Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
}
```

With:

```rust
// After:
Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    // Another process holds the lock. Read its PID and verify liveness.
    // `read_pid` returns None if the file is empty or non-numeric — this
    // can occur in the brief window between file-open and PID write.
    let pid = match read_pid(&pid_path) {
        Some(p) if is_process_alive(p) => {
            tracing::info!(pid = p, "lock held by live process");
            p
        }
        Some(p) => {
            // PID file contains a number but the process is gone — stale.
            // The OS lock prevents us from acquiring, so something else
            // holds the fd-lock (e.g., another daemon that hasn't written
            // its PID yet, or a process that survived but the PID was
            // recycled). Report the stale PID for diagnostics.
            tracing::warn!(
                pid = p,
                path = %pid_path.display(),
                "lock held but PID appears dead (possible PID recycle or race)"
            );
            p
        }
        None => {
            // PID file is empty or unreadable — lock holder hasn't written
            // its PID yet (startup race). Report 0 only as a last resort,
            // with an explicit warning so it's not silently misleading.
            tracing::warn!(
                path = %pid_path.display(),
                "lock held but PID file is empty or unreadable (daemon may be initializing)"
            );
            0
        }
    };
    Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
}
```

**Add the `is_process_alive` helper** after the `read_pid` function (line 153 currently):

```rust
/// Check whether process `pid` is currently alive using `sysinfo`.
///
/// Uses a minimal `System` refresh scoped to the single process to avoid
/// the cost of a full system scan. Returns `false` if the process cannot
/// be found (dead or never existed).
fn is_process_alive(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        ProcessRefreshKind::new(),
    );
    sys.process(Pid::from_u32(pid)).is_some()
}
```

> **Note**: Verify the exact `sysinfo` 0.30 API — `ProcessesToUpdate::Some` and `ProcessRefreshKind::new()` are the 0.30 patterns. If the API differs, use `System::new_all()` with `sys.refresh_process(Pid::from_u32(pid))` as a fallback. The key requirement is: call `sys.refresh_process` or equivalent before calling `sys.process`.

#### Acceptance Criteria

- `cargo build` passes with zero warnings.
- When a live process holds the lock and the PID file is readable: `AlreadyHeld { pid }` contains the correct PID, `info!` log is emitted.
- When the PID file is empty (race window): `AlreadyHeld { pid: 0 }` is returned with a `warn!` log (not a silent `0`).
- When the PID file contains a number for a dead process: `AlreadyHeld { pid }` contains that number with a `warn!` log.
- Existing lockfile unit tests (`s027`, `s029`, `s030`, `s032`) still pass.
- New unit test `s031_would_block_with_empty_pid_file_logs_warning` passes (see Test Plan).

---

## Test Plan

### Tests to Run (Existing)

| Test file | Tests | Why relevant |
|-----------|-------|-------------|
| `tests/contract/shim_lifecycle_test.rs` | All 6 tests | Validates daemon startup, health check, timeout; Bug 1 change must not break them |
| `tests/unit/lockfile_test.rs` | `s027`, `s029`, `s030`, `s032` | Validates lockfile acquisition paths; Bug 3 must not regress them |
| `tests/integration/daemon_lifecycle_test.rs` | All | Cross-module lifecycle; catches regressions from any of the three fixes |

Run all with: `cargo test --workspace`

### New Tests to Add

#### FIX-001: Timeout env var (add inline to `src/shim/lifecycle.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_timeout_defaults_to_10_seconds_when_env_var_absent() {
        // Remove env var if set by another test (tests run in the same process)
        std::env::remove_var("ENGRAM_READY_TIMEOUT_MS");
        assert_eq!(ready_timeout_ms(), DEFAULT_READY_TIMEOUT_MS);
        assert_eq!(ready_timeout_ms(), 10_000);
    }

    #[test]
    fn ready_timeout_reads_from_env_var() {
        std::env::set_var("ENGRAM_READY_TIMEOUT_MS", "3000");
        let result = ready_timeout_ms();
        std::env::remove_var("ENGRAM_READY_TIMEOUT_MS"); // clean up
        assert_eq!(result, 3000);
    }

    #[test]
    fn ready_timeout_falls_back_to_default_for_invalid_value() {
        std::env::set_var("ENGRAM_READY_TIMEOUT_MS", "not_a_number");
        let result = ready_timeout_ms();
        std::env::remove_var("ENGRAM_READY_TIMEOUT_MS");
        assert_eq!(result, DEFAULT_READY_TIMEOUT_MS);
    }
}
```

> **Warning**: `std::env::set_var` is not safe in multithreaded tests. Use `#[serial_test::serial]` if the `serial_test` crate is available, or use `cargo test -- --test-threads=1` for this module. Alternatively, test `ready_timeout_ms` via a helper that accepts an explicit `Option<&str>` override so tests don't need env mutation.

#### FIX-002: Tool catalog (inline in `src/shim/tools_catalog.rs`)

Three tests already specified in the exact change above:
- `tool_count_matches_dispatch` — count == `TOOL_COUNT`
- `all_tool_names_are_unique` — no duplicates
- `dispatch_tools_present_in_catalog` — every dispatch() arm name in catalog

#### FIX-003: Lockfile stale PID (add to `tests/unit/lockfile_test.rs`)

```rust
// S031: WouldBlock with empty PID file returns AlreadyHeld pid=0 and emits warning
// This test cannot directly hold the fd-lock from another thread (DaemonLock isn't
// Send). Instead, verify the behaviour by inspecting the WouldBlock arm indirectly:
// pre-create an empty PID file + hold a write lock from a child thread, then attempt
// acquire from the main thread and check the error.
#[test]
fn s031_would_block_with_empty_pid_file_returns_already_held() {
    // Implementation note: holding an fd_lock from one thread while calling
    // acquire from another requires DaemonLock to be Send, which it is not
    // (the guard is not Send on Windows). Use a subprocess approach or
    // document as a manual test. See risk note R4.
    //
    // Minimal verifiable assertion: the WouldBlock match arm exists and the
    // is_process_alive helper doesn't panic for PID 0 or PID u32::MAX.
    //
    // Placeholder — implementor must choose the subprocess or mock approach
    // appropriate for the CI environment.
}
```

> **Note**: The concurrent lock test is difficult to write without subprocess spawning (covered by integration harness). The immediate priority is that the existing tests still pass and that `is_process_alive` compiles and works. A full concurrent test is tracked as a follow-up (see Risk R4).

#### FIX-003: Integration test — no stale PID 0 in error (add to `tests/integration/daemon_lifecycle_test.rs`)

Verify that when a daemon is running and a second daemon tries to acquire the same lock, the error contains a real PID (not 0):

```rust
// Spawn a daemon harness, then attempt a second acquire on the same workspace.
// Assert: EngramError::Lock(LockError::AlreadyHeld { pid }) where pid > 0.
```

---

## Risk Notes

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|------------|
| R1 | `ENGRAM_READY_TIMEOUT_MS=10000` causes 10 s hangs when daemon binary is missing | Low | Medium | The `spawn_daemon` error path already returns `SpawnFailed` immediately before `poll_until_ready` is called. If the binary exists but never binds, the 10 s timeout will elapse. This is acceptable — it is always better to wait than to fail prematurely on a slow machine. |
| R2 | Static tool catalog drifts from `dispatch()` | Medium | High | The `dispatch_tools_present_in_catalog` unit test is the primary mitigation. Also add a CI note: "When adding a tool to `dispatch()`, update `tools_catalog.rs` in the same PR." |
| R3 | `sysinfo` process refresh adds 50–100 ms on lock contention | Low | Low | Only called on the `WouldBlock` path (rare in production). `ProcessesToUpdate::Some` scoped refresh is much faster than `new_all()`. |
| R4 | Concurrent lockfile test requires subprocess or `#[serial]` | Medium | Low | Skip the multithreaded lock test for now; rely on integration test via `DaemonHarness`. |
| R5 | `rmcp::model::ListToolsResult` field names differ from expectation | Low | Medium | Confirm field names from the rmcp crate source before writing `transport.rs`. The `tools` field and `next_cursor: None` pattern are standard MCP, but verify with `cargo doc --open`. |
| R6 | `sysinfo` 0.30 API for `ProcessesToUpdate::Some` may differ | Low | Medium | Fall back to `sys.refresh_process(Pid::from_u32(pid))` if `ProcessesToUpdate` is not available. Check `sysinfo` 0.30 changelog before implementing. |
| R7 | `tool()` helper uses `expect()` — violates constitution Principle I for library code | Low | Low | The `expect()` fires only on a statically authored schema literal. This is equivalent to a compile-time assertion. Acceptable — document it. |

---

## Constitution Check

| Principle | Relevance | Status | Notes |
|-----------|-----------|--------|-------|
| **I. Safety-First Rust** | `unwrap_or(0)` in Bug 3, `expect()` in catalog helper | ⚠️ Partial | The `unwrap_or(0)` is removed (fix is strictly better). The `expect()` in `tools_catalog::tool()` is acceptable as it fires only on static, compile-time data — equivalent to a compile-time assertion. No `unsafe` code is introduced. |
| **II. MCP Protocol Fidelity** | Bug 2 directly violates "tools MUST be unconditionally visible" | 🔴 Violated (currently) → ✅ Fixed | Returning an empty `tools/list` prevents any MCP client from discovering capabilities. FIX-002 resolves this violation. |
| **III. Test-First Development** | Tests must be written before/alongside implementation | ✅ Compliant | Test plan is specified above for all three fixes. Unit tests go in the implementation files. The unit tests for FIX-002 are written as part of the new file. |
| **IV. Workspace Isolation** | No path operations introduced | ✅ Not affected | These fixes do not touch workspace path handling. |
| **V. Structured Observability** | Bug 3 fix adds `warn!` and `info!` tracing | ✅ Improved | The fix explicitly emits structured log events for the empty-PID and live-PID branches, improving observability over the silent `unwrap_or(0)`. |
| **VI. Single-Binary Simplicity** | `sysinfo` already a dependency | ✅ Compliant | No new dependencies introduced. `sysinfo = "0.30"` is already in `Cargo.toml`. |
| **VII. CLI Workspace Containment** | No filesystem writes outside workspace | ✅ Not affected | |
| **VIII. Git-Friendly Persistence** | No `.engram/` format changes | ✅ Not affected | |

**Complexity Tracking** (justified deviations):

| Violation | Justification | Simpler Alternative Rejected |
|-----------|--------------|------------------------------|
| `expect()` in `tools_catalog::tool()` (Principle I) | The schema literals are compile-time constants authored by the implementor. A panic here indicates a bug in the code itself, not a runtime condition. Using `Result` and propagating through `all_tools()` would require `ListToolsResult` to return `Result<ListToolsResult, _>`, changing the trait signature. | Convert schemas to validated types at compile time using `schemars` (rejected: adds a new dependency violating Principle VI). |

---

## Implementation Order

These three tasks are **independent** and can be implemented in any order or in parallel. The recommended order for a single implementor:

1. **FIX-001** (30 min) — smallest change, highest priority, easiest to test
2. **FIX-003** (45 min) — medium complexity, improves daemon diagnostics
3. **FIX-002** (2–3 h) — most content to write (35 tool entries), but mechanical

After all three: run `cargo test --workspace` and `cargo clippy -- -D warnings`.

---

## Validation Commands

```bash
# Build (zero warnings required by .cargo/config.toml -Dwarnings)
cargo build

# Lint
cargo clippy -- -D warnings

# All tests
cargo test --workspace

# Scoped tests for each fix
cargo test shim::lifecycle       # FIX-001
cargo test shim::tools_catalog   # FIX-002
cargo test unit::lockfile        # FIX-003
```
