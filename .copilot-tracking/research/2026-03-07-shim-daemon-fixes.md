<!-- markdownlint-disable-file -->
# Task Research: Shim/Daemon Bug Fixes (Three Live E2E Issues)

Three bugs discovered during live end-to-end testing of the `agent-engram` Rust project.
This document captures root-cause analysis, exact code locations, and recommended fix approaches for all three.

## Task Implementation Requests

* **Issue 1 (HIGH):** Shim startup timeout too short on Windows — `READY_TIMEOUT_MS = 2000` is insufficient for SurrealDB init + named pipe binding.
* **Issue 2 (MEDIUM):** `tools/list` returns empty from shim — `ShimHandler::list_tools` returns `ListToolsResult::default()` (empty) instead of proxying to the daemon.
* **Issue 3 (MEDIUM):** Stale PID file blocks daemon restart — `LockError::AlreadyHeld { pid: 0 }` reported because the shim does not handle the stale-lock case.

## Scope and Success Criteria

* **Scope:** `src/shim/lifecycle.rs`, `src/shim/transport.rs`, `src/daemon/lockfile.rs`, `src/daemon/ipc_server.rs`, `src/tools/mod.rs`, and related error types in `src/errors/mod.rs`.
* **Assumptions:** The daemon binary is the same executable invoked with `daemon --workspace` subcommand. SurrealDB startup is the primary bottleneck on Windows. No external config file changes are required for fixes.
* **Success Criteria:**
  * Shim waits long enough for daemon startup on Windows (target: 10–30 s configurable).
  * `tools/list` MCP calls from any client return the daemon's full tool roster (~35 tools).
  * A new shim succeeds even when a stale PID file without an OS-level lock exists.

---

## Outline

1. [Issue 1 — Startup Timeout](#issue-1--startup-timeout-too-short-on-windows)
2. [Issue 2 — tools/list Empty](#issue-2--toolslist-returns-empty-from-shim)
3. [Issue 3 — Stale PID File Blocks Restart](#issue-3--stale-pid-file-blocks-daemon-restart)
4. [Risk Register](#risk-register)
5. [Test Coverage Gaps](#test-coverage-gaps)

---

## Research Executed

### File Analysis

* `src/shim/lifecycle.rs` (167 lines) — all constants, health check, spawn, `poll_until_ready`
* `src/shim/transport.rs` (163 lines) — `ShimHandler`, `call_tool`, `list_tools`, `run_shim`
* `src/shim/mod.rs` (49 lines) — `shim::run()`, endpoint derivation, timeout used for IPC
* `src/shim/ipc_client.rs` (154 lines) — `send_request`, platform connect helpers
* `src/daemon/ipc_server.rs` (429 lines) — `run_with_shutdown`, `bind_listener`, `_health` handler, `process_request`, `accept_loop`
* `src/daemon/lockfile.rs` (199 lines) — `DaemonLock::acquire`, `read_pid`, `clean_stale_socket`
* `src/daemon/mod.rs` (218 lines) — `daemon::run()` startup sequence steps 1–8
* `src/daemon/protocol.rs` — IPC wire types (`IpcRequest`, `IpcResponse`)
* `src/tools/mod.rs` (99 lines) — `dispatch()` match arms, all 33 registered tool names
* `src/db/mod.rs` (100 lines) — `connect_db`, `ensure_schema` (10 schema queries on first connection)
* `src/tools/lifecycle.rs` (179 lines) — `set_workspace` (DB connect + hydrate called during daemon init)
* `src/models/config.rs` (313 lines) — `PluginConfig`, `ENGRAM_IDLE_TIMEOUT_MS` env var
* `src/config/mod.rs` (114 lines) — `Config` struct; no shim startup timeout field
* `src/errors/mod.rs` — `DaemonError::NotReady`, `LockError::AlreadyHeld { pid }`
* `tests/contract/shim_lifecycle_test.rs` — S005 already relaxed to 10 s in test harness
* `tests/unit/lockfile_test.rs` — S029 confirms stale PID (no OS lock) → acquire succeeds

### Code Search Results

* `READY_TIMEOUT_MS` — only in `src/shim/lifecycle.rs:28`
* `list_tools` — only in `src/shim/transport.rs:117–123` (returns `default()`)
* `AlreadyHeld` — emitted in `src/daemon/lockfile.rs:130–131`, error type `src/errors/mod.rs:169`
* `_health` — handled in `src/daemon/ipc_server.rs:230–237`, sent from `src/shim/lifecycle.rs:39–43`
* `set_workspace` — called from `ipc_server::run_with_shutdown` line 310 (the blocking init path)
* `ENGRAM_IDLE_TIMEOUT_MS` — env-var override in `src/daemon/mod.rs:122`; no analog for startup timeout

---

## Issue 1 — Startup Timeout Too Short on Windows

### Current Code

**`src/shim/lifecycle.rs` lines 19–28:**
```rust
const MAX_BACKOFF_ATTEMPTS: u32 = 30;
const INITIAL_BACKOFF_MS: u64 = 10;
const MAX_BACKOFF_MS: u64 = 500;
const READY_TIMEOUT_MS: u64 = 2_000;   // ← THE PROBLEM
```

**`src/shim/lifecycle.rs` lines 138–166 — `poll_until_ready`:**
```rust
async fn poll_until_ready(endpoint: &str) -> Result<(), EngramError> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(READY_TIMEOUT_MS);
    let mut delay_ms = INITIAL_BACKOFF_MS;

    for attempt in 0..MAX_BACKOFF_ATTEMPTS {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        delay_ms = (delay_ms * 2).min(MAX_BACKOFF_MS);

        if check_health(endpoint).await {
            info!(attempt, "daemon reached ready state");
            return Ok(());
        }

        if tokio::time::Instant::now() >= deadline {
            debug!(attempt, "ready-wait deadline exceeded");
            break;
        }
    }
    // Final check omitted from snippet for brevity…
    Err(EngramError::Daemon(DaemonError::NotReady { timeout_ms: READY_TIMEOUT_MS }))
}
```

**`src/shim/lifecycle.rs` line 46 — health check timeout:**
```rust
crate::shim::ipc_client::send_request(endpoint, &request, Duration::from_millis(500))
```

### Daemon Startup Sequence (What Must Finish Before `_health` Responds)

The startup timeline on Windows (from `src/daemon/mod.rs` and `src/daemon/ipc_server.rs`):

```
daemon::run()
  Step 1: fs::canonicalize(workspace)                    — fast
  Step 2: DaemonLock::acquire()                          — fast
  Step 3: PluginConfig::load() + env var parse           — fast
  Step 4: TtlTimer::new(), watch::channel()              — fast
  Step 5: signal handler spawn                           — fast
  Step 6: start_watcher (notify crate)                   — fast
  ipc_server::run_with_shutdown()
    Step A: fs::canonicalize again                       — fast
    Step B: create_dir_all(.engram/run/)                 — fast
    Step C: AppState::new(1)                             — fast
    Step D: tools::lifecycle::set_workspace()            — SLOW ★
              → connect_db() — Surreal::new::<SurrealKv>(db_path)  [Windows: ~1-3s]
              → ensure_schema() — 10 × db.query()        [~0.2-0.5s]
              → hydrate_into_db()                         [variable]
              → hydrate_code_graph()                      [variable]
    Step E: ipc_endpoint() — SHA-256 hash               — fast
    Step F: bind_listener() — named pipe creation        — fast after Step D
    Step G: info!("IPC listener bound")  ← READY to answer _health
    Step H: ttl.reset()
    Step I: spawn TTL task
    Step J: accept_loop                  ← _health now answerable
```

The critical bottleneck is **Step D**: `tools::lifecycle::set_workspace` triggers `connect_db` which calls `Surreal::new::<SurrealKv>(db_path)`. On Windows, SurrealKV (RocksDB/TiKV-backed embedded store) commonly takes 1–4 s to initialize cold. The named pipe is not bound until Step F, which occurs **after** Step D completes. The entire init chain must finish before the shim's first poll can receive a `status: ready`.

With `READY_TIMEOUT_MS = 2000` and per-poll health-check timeout of 500 ms, the effective wall-clock budget is only ~2.0 s. This is frequently insufficient.

### Root Cause

`READY_TIMEOUT_MS` is a compile-time constant with no runtime override. `2000` ms was chosen for fast Unix environments; Windows SurrealDB initialization regularly exceeds this.

### Proposed Fix

**Option A (Recommended): Environment-variable override (mirrors `ENGRAM_IDLE_TIMEOUT_MS` pattern)**

Add `ENGRAM_READY_TIMEOUT_MS` environment variable read at shim startup, with `10_000` ms (10 s) as the new default:

```rust
// src/shim/lifecycle.rs

/// Default wall-clock budget for daemon ready-wait (milliseconds).
/// On Windows, SurrealDB init + named pipe binding typically takes 1–4 s.
/// Override with the `ENGRAM_READY_TIMEOUT_MS` environment variable.
const DEFAULT_READY_TIMEOUT_MS: u64 = 10_000;

fn ready_timeout_ms() -> u64 {
    std::env::var("ENGRAM_READY_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_READY_TIMEOUT_MS)
}

async fn poll_until_ready(endpoint: &str) -> Result<(), EngramError> {
    let timeout_ms = ready_timeout_ms();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    // … rest unchanged …
    Err(EngramError::Daemon(DaemonError::NotReady { timeout_ms }))
}
```

**Option B: Platform-conditional constant**
```rust
#[cfg(windows)]
const READY_TIMEOUT_MS: u64 = 15_000;
#[cfg(not(windows))]
const READY_TIMEOUT_MS: u64 = 5_000;
```
Simpler, no env var needed, but less operator-tunable.

**Option C: Read from `.engram/config.toml` via `PluginConfig`**
Requires the shim to parse config before the daemon starts — adds coupling and a new config field. Higher complexity, not recommended.

**Recommendation: Option A** — mirrors the existing `ENGRAM_IDLE_TIMEOUT_MS` pattern, is operator-tunable without recompile, defaults are safe on all platforms.

**Also fix:** `MAX_BACKOFF_ATTEMPTS` should scale with the timeout or be removed as the deadline check is already authoritative. Currently 30 attempts × 500 ms cap = 15 s of sleep if the deadline weren't checked. The deadline check at line 151 makes the attempt count redundant as a time bound, but keeping it as a safety cap is fine.

**Test to update:** `tests/contract/shim_lifecycle_test.rs:48-68` (`t020_s001_s005_daemon_becomes_healthy_within_2_seconds`) already uses `Duration::from_secs(10)` in the test harness, so it will pass. The production constant change does not break the test.

---

## Issue 2 — `tools/list` Returns Empty from Shim

### Current Code

**`src/shim/transport.rs` lines 117–123:**
```rust
/// Return an empty tool list (tools are enumerated by the daemon).
async fn list_tools(
    &self,
    _request: Option<PaginatedRequestParams>,
    _cx: RequestContext<RoleServer>,
) -> Result<ListToolsResult, ErrorData> {
    Ok(ListToolsResult::default())   // ← returns [] always
}
```

The comment "tools are enumerated by the daemon" indicates the intent, but the implementation does **not** proxy to the daemon — it unconditionally returns `ListToolsResult::default()` which is `{ tools: [] }`.

### Daemon Side: Tool Registry

Tools are statically registered in `src/tools/mod.rs` via the `dispatch()` match arm. There is **no `list_tools` IPC method** on the daemon. The daemon does not expose its tool catalog over IPC — it only dispatches by name.

**Tools currently registered in `dispatch()` (`src/tools/mod.rs` lines 33–97):**

| # | Tool name | Handler |
|---|-----------|---------|
| 1 | `set_workspace` | `lifecycle::set_workspace` |
| 2 | `get_daemon_status` | `lifecycle::get_daemon_status` |
| 3 | `get_workspace_status` | `lifecycle::get_workspace_status` |
| 4 | `create_task` | `write::create_task` |
| 5 | `update_task` | `write::update_task` |
| 6 | `add_blocker` | `write::add_blocker` |
| 7 | `register_decision` | `write::register_decision` |
| 8 | `flush_state` | `write::flush_state` |
| 9 | `get_task_graph` | `read::get_task_graph` |
| 10 | `check_status` | `read::check_status` |
| 11 | `query_memory` | `read::query_memory` |
| 12 | `get_ready_work` | `read::get_ready_work` |
| 13 | `add_label` | `write::add_label` |
| 14 | `remove_label` | `write::remove_label` |
| 15 | `add_dependency` | `write::add_dependency` |
| 16 | `get_compaction_candidates` | `read::get_compaction_candidates` |
| 17 | `apply_compaction` | `write::apply_compaction` |
| 18 | `claim_task` | `write::claim_task` |
| 19 | `release_task` | `write::release_task` |
| 20 | `defer_task` | `write::defer_task` |
| 21 | `undefer_task` | `write::undefer_task` |
| 22 | `pin_task` | `write::pin_task` |
| 23 | `unpin_task` | `write::unpin_task` |
| 24 | `get_workspace_statistics` | `read::get_workspace_statistics` |
| 25 | `batch_update_tasks` | `write::batch_update_tasks` |
| 26 | `add_comment` | `write::add_comment` |
| 27 | `index_workspace` | `write::index_workspace` |
| 28 | `sync_workspace` | `write::sync_workspace` |
| 29 | `link_task_to_code` | `write::link_task_to_code` |
| 30 | `unlink_task_from_code` | `write::unlink_task_from_code` |
| 31 | `map_code` | `read::map_code` |
| 32 | `list_symbols` | `read::list_symbols` |
| 33 | `get_active_context` | `read::get_active_context` |
| 34 | `unified_search` | `read::unified_search` |
| 35 | `impact_analysis` | `read::impact_analysis` |

**33 unique tools** (vs. "~35" mentioned in the issue — the count is 33 match arms excluding the `_` wildcard).

### Root Cause

`ShimHandler::list_tools` was a placeholder/stub. MCP protocol requires `list_tools` (the `tools/list` request) to enumerate all available tools with their descriptions and input schemas so that MCP clients (Claude Desktop, cursor, etc.) can display them. Without this, clients cannot know what tools are available.

There are two sub-problems:
1. **No `list_tools` IPC method on the daemon** — the daemon only dispatches by name; it has no endpoint to query for the catalog.
2. **Tool descriptions and JSON schemas are not stored anywhere** — `dispatch()` is a plain `match` with no metadata.

### Proposed Fix Approaches

**Option A (Recommended): Static tool catalog in the shim**

Since the tool list is determined at compile time (the `match` in `dispatch()` is exhaustive), embed a static tool catalog directly in the shim. Each tool entry includes name, description, and `inputSchema`. This avoids a round-trip to the daemon and is always consistent with the compiled binary.

Implementation sketch:
```rust
// src/shim/tools_catalog.rs  (new file)
use rmcp::model::{Tool, ToolInputSchema};
use serde_json::json;

pub fn all_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "create_task".into(),
            description: Some("Create a new task in the workspace.".into()),
            input_schema: ToolInputSchema { /* … */ },
        },
        // … one entry per tool in dispatch() …
    ]
}
```

Then in `ShimHandler::list_tools`:
```rust
async fn list_tools(
    &self,
    _request: Option<PaginatedRequestParams>,
    _cx: RequestContext<RoleServer>,
) -> Result<ListToolsResult, ErrorData> {
    Ok(ListToolsResult { tools: crate::shim::tools_catalog::all_tools() })
}
```

**Advantages:** No IPC round-trip on every `tools/list`, always consistent, no new daemon endpoint needed.  
**Disadvantage:** The catalog must be kept in sync with `dispatch()` manually. Mismatch is a latent bug risk.

**Option B: Add `_list_tools` IPC method to the daemon**

Add a new match arm in `src/daemon/ipc_server.rs`/`process_request`:
```rust
"_list_tools" => IpcResponse::success(id, json!({ "tools": tools::catalog() })),
```

And define `tools::catalog()` that returns serialized tool metadata. The shim `list_tools` proxies this call.

**Advantages:** Single source of truth, catalog always matches dispatch.  
**Disadvantage:** Every `tools/list` call requires an IPC round-trip. Tool catalog must still be defined somewhere (same work as Option A, just in the daemon).

**Option C: Hybrid — static catalog with compile-time assertion**

Use `static` or `const` tool catalog in `src/tools/mod.rs`, exposed as `pub` and shared by shim. A unit test or `const_assert!` verifies the catalog matches the `dispatch()` arms.

**Recommendation: Option A** for immediate fix velocity, then migrate to **Option C** to prevent drift. The catalog is genuinely compile-time data. IPC round-trips for metadata (Option B) add latency to every MCP session init without benefit.

**Impact on `call_tool`:** `ShimHandler::call_tool` already works correctly (proxies to daemon via IPC). Only `list_tools` is broken.

---

## Issue 3 — Stale PID File Blocks Daemon Restart

### Current Code

**`src/daemon/lockfile.rs` lines 84–137 — `DaemonLock::acquire`:**
```rust
match rw_lock.try_write() {
    Ok(mut guard) => {
        // … truncate, write PID, call clean_stale_socket …
        Ok(Self { _guard: guard, path: pid_path, pid })
    }
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
        let pid = read_pid(&pid_path).unwrap_or(0);  // ← returns 0 if unreadable
        Err(EngramError::Lock(LockError::AlreadyHeld { pid }))  // ← "AlreadyHeld by PID 0"
    }
    Err(e) => Err(EngramError::Lock(LockError::AcquisitionFailed { … })),
}
```

**`src/daemon/lockfile.rs` lines 153–157 — `read_pid`:**
```rust
fn read_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}
```

**`src/daemon/lockfile.rs` lines 129–131:**
```rust
Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    let pid = read_pid(&pid_path).unwrap_or(0);
    Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
```

### Root Cause Analysis

#### Why "PID 0"?

`fd-lock`'s `try_write()` returns `Err(WouldBlock)` when **an OS-level write lock is already held on the file**. When this happens, `read_pid` attempts to read the PID from the file. If:
- The file is **empty** (e.g., created by `OpenOptions` with `create(true)` but never written), `read_pid` returns `None` → `unwrap_or(0)` → PID 0.
- The file is **locked by another process that has not yet written its PID** (narrow race window).
- The file is **corrupted** (non-numeric content), same result.

**The issue title says "stale PID file blocks restart"** — but `lockfile.rs` comment at lines 44–47 states:
> _"If the file exists but the owning process is dead (stale lock), the OS already released the lock, so `try_write()` succeeds and we overwrite the stale PID."_

This is the designed behavior on Unix (where `fd-lock` uses `flock(2)`). The OS **automatically releases** advisory file locks when the holding process exits (crashed or killed).

#### Windows Behavior Difference

On Windows, `fd-lock` uses `LockFileEx`/`UnlockFileEx`. The OS **does** release the lock on process death — but there is a subtlety:

- If the daemon process exits uncleanly (e.g., force-kill via Task Manager), Windows may not immediately release the lock if the file handle is still in a "zombie" state in the kernel. In practice this is resolved within milliseconds, but the shim may poll before the OS cleans up.

**More likely scenario for "PID 0":** The daemon process had a panic or error after `OpenOptions::new().open()` but **before** the first `guard.write_all(pid_str)` call (lines 104–109). The file exists and is unlocked (process died), so `try_write()` succeeds, but this is actually a **success path** — the acquire succeeds and overwrites the empty file. This doesn't match "PID 0" in an error.

**Most likely actual scenario:** Two shims race to spawn a daemon. The first shim's daemon is in the process of starting (between `open()` and the first `write_all(pid_str)`) while a second daemon's `try_write()` fails with `WouldBlock`. `read_pid` reads an empty file → PID 0. The second daemon's `acquire()` fails with `AlreadyHeld { pid: 0 }`.

#### How the Shim Handles `AlreadyHeld`

**`src/shim/lifecycle.rs` lines 83–94:**
```rust
pub async fn ensure_daemon_running(workspace: &Path) -> Result<(), EngramError> {
    let endpoint = ipc_endpoint(workspace)?;

    if check_health(&endpoint).await {
        info!("daemon already running and healthy");
        return Ok(());
    }

    spawn_daemon(workspace)?;   // ← spawns daemon; daemon may get AlreadyHeld

    poll_until_ready(&endpoint).await  // ← waits for _health to respond
}
```

The shim does **not** handle `LockError::AlreadyHeld` itself — it passes to the daemon process via `spawn_daemon`. The spawned daemon calls `DaemonLock::acquire()` at `src/daemon/mod.rs:98`. If acquire fails, the daemon exits immediately with an error (no listener bound). The shim then polls for health and times out.

The shim's `spawn_daemon` (`lifecycle.rs:97–130`) ignores the daemon's exit status (it's detached). The only feedback is health-check timeout.

#### `clean_stale_socket` — Does It Help on Windows?

`src/daemon/lockfile.rs` lines 168–198:
```rust
fn clean_stale_socket(run_dir: &Path) {
    #[cfg(unix)]
    { /* removes engram.sock */ }
    #[cfg(not(unix))]
    let _ = run_dir;  // no-op on Windows
}
```

**No**, it does nothing on Windows. Named pipes are cleaned up by the OS automatically when the server process dies — which is correct. The stale PID file issue is unrelated to socket/pipe cleanup.

### Proposed Fix

**Option A (Recommended): Check if the PID is alive before reporting `AlreadyHeld`**

Before returning `AlreadyHeld`, verify that the PID in the file corresponds to a live process. If the PID is dead (or 0, or unreadable), treat it as a stale lock and attempt to clean up.

The challenge: `try_write()` returned `WouldBlock`, meaning the OS lock IS held. If the OS holds it, the process should still be alive. The PID-0 case most likely means the file is empty (daemon hasn't written its PID yet). In that case, the lock IS genuinely held by a live process — we should retry rather than steal.

**Better fix: Retry after brief sleep when PID is 0 (or process check is ambiguous)**

```rust
// src/daemon/lockfile.rs
Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    let pid = read_pid(&pid_path).unwrap_or(0);
    // PID 0 = file is empty (daemon starting) or corrupt.
    // A live process holds the OS lock; report it clearly.
    let reported_pid = if pid == 0 {
        // Re-read after a short settle delay — the daemon may not have written its PID yet.
        std::thread::sleep(std::time::Duration::from_millis(50));
        read_pid(&pid_path).unwrap_or(0)
    } else {
        pid
    };
    Err(EngramError::Lock(LockError::AlreadyHeld { pid: reported_pid }))
}
```

This doesn't fix the root cause but improves the error message from "PID 0" to the actual PID.

**Option B (Recommended as primary): Shim retries `spawn_daemon` if daemon exits quickly**

The shim should detect the "daemon failed to acquire lock" scenario and respond gracefully. Currently `spawn_daemon` fires and forgets. A better approach: if `poll_until_ready` sees continuous connection-refused errors (never gets even a WouldBlock-level response) within the first second, assume the daemon exited and re-check health (in case another daemon spawned by a concurrent shim is now ready).

The existing "final check" at `lifecycle.rs:157–160` already handles this race:
```rust
// Final check: a concurrent shim may have raced and won the spawn.
if check_health(endpoint).await {
    info!("daemon ready (concurrent shim won the spawn race)");
    return Ok(());
}
```

**The real fix is in `DaemonLock::acquire`** in the daemon: when `WouldBlock` is returned but PID is 0 (file empty), the daemon should retry the lock acquisition a few times with exponential backoff rather than failing immediately. The first daemon to bind the pipe wins; subsequent ones should back off gracefully.

**Option C: Add stale-lock detection with process liveness check**

```rust
// src/daemon/lockfile.rs — in WouldBlock arm
Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    let pid = read_pid(&pid_path).unwrap_or(0);
    if pid > 0 && !is_process_alive(pid) {
        // Stale: the holding process is dead but the OS lock wasn't released.
        // This can happen on Windows when the process is being cleaned up.
        // Force-remove and retry. (Rare — the OS should release on death.)
        tracing::warn!(pid, "stale lock detected — PID no longer alive; will retry");
        // Drop the RwLock (leaked, but this is an error path — process will exit or retry)
        // Attempt: wait briefly for OS cleanup
        std::thread::sleep(Duration::from_millis(100));
        return DaemonLock::acquire(workspace); // recursive retry
    }
    Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
}
```

`is_process_alive(pid)`:
```rust
#[cfg(windows)]
fn is_process_alive(pid: u32) -> bool {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // OpenProcess returns NULL if PID doesn't exist
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h == 0 { return false; }
        windows_sys::Win32::Foundation::CloseHandle(h);
        true
    }
}
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
```

This requires the `windows-sys` or `sysinfo` crate dependency. `sysinfo` is already used in `src/tools/lifecycle.rs`.

**Using `sysinfo` (already a dependency):**
```rust
fn is_process_alive(pid: u32) -> bool {
    use sysinfo::{Pid, System};
    let mut sys = System::new();
    sys.refresh_process(Pid::from_u32(pid));
    sys.process(Pid::from_u32(pid)).is_some()
}
```

**Recommendation:**
1. **Immediate fix:** Option B (shim final check already handles the race; increase `READY_TIMEOUT_MS` per Issue 1 fix to give more time for the winning daemon to start).
2. **Proper fix:** Option C using `sysinfo` for liveness check in `DaemonLock::acquire`. This definitively breaks the "stale OS lock" scenario if it ever occurs on Windows.
3. **Error message fix (always do):** Replace `unwrap_or(0)` with a more informative fallback: emit a `warn!` log stating the PID file was empty/unreadable.

---

## Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| R1 | `DEFAULT_READY_TIMEOUT_MS = 10_000` causes 10 s hangs when daemon binary is missing/corrupt | Low | Medium | Check daemon binary exists before spawning; fail fast with `SpawnFailed` |
| R2 | Static tool catalog drifts from `dispatch()` over time | Medium | High | Add integration test that calls `tools/list` and verifies all known tool names appear |
| R3 | `is_process_alive` via `sysinfo` adds 50–100 ms on lock contention (sysinfo refresh is slow) | Low | Low | Only call on `WouldBlock` path, which is rare in production |
| R4 | Recursive `DaemonLock::acquire` retry leaks the `Box::leak`ed `RwLock` allocation | Low | Low | Add `drop` before retry or limit recursion depth to 1 |
| R5 | Increasing `READY_TIMEOUT_MS` makes shim tests slower if daemon binary is absent | Low | Medium | CI should run with a real binary; mock harness already uses separate timeout |
| R6 | `tools/list` schema definitions may not match actual tool parameter requirements | Medium | Medium | Schema should be derived from existing parameter structs using `schemars` |

---

## Test Coverage Gaps

### Issue 1 (Timeout)
- **Gap:** No test exercises the `READY_TIMEOUT_MS` deadline path — there is no test for "daemon takes too long" → `DaemonError::NotReady`.
- **Gap:** `t020_s001_s005_daemon_becomes_healthy_within_2_seconds` uses a 10 s harness timeout but the production constant remains 2 s. The test does **not** test the production timeout.
- **Recommended tests:**
  - Test that `poll_until_ready` respects `ENGRAM_READY_TIMEOUT_MS` env var.
  - Test that `DaemonError::NotReady` is returned when daemon never binds.

### Issue 2 (tools/list)
- **Gap:** No test calls `tools/list` via the shim and asserts a non-empty result.
- **Gap:** No test verifies that the catalog matches `dispatch()` arms.
- **Recommended tests:**
  - Contract test: start daemon + shim, call MCP `tools/list`, assert all 33 tool names present.
  - Unit test: assert `all_tools().len() == 33` and names match the dispatch match arm list.

### Issue 3 (Stale PID)
- **Gap:** `tests/unit/lockfile_test.rs:s029` covers stale PID with **no OS lock** (correct path). But there is no test for the "WouldBlock + empty file = PID 0" scenario.
- **Gap:** No test for concurrent shim race (two shims spawning simultaneously).
- **Recommended tests:**
  - Unit test: hold an `fd-lock` write lock on an empty file, call `acquire()` from another thread, assert error is `AlreadyHeld` with a PID ≥ 1 (not 0) after the fix.
  - Integration test: spawn two shims for the same workspace simultaneously; assert exactly one daemon runs and both shims succeed.

---

## Summary Table

| Issue | File | Line(s) | Root Cause | Fix Complexity |
|-------|------|---------|------------|----------------|
| 1 — Timeout | `src/shim/lifecycle.rs` | 28 (`READY_TIMEOUT_MS`), 139 (deadline) | Hardcoded 2 s budget; SurrealDB init on Windows takes 1–4 s | Low — change constant + add env var |
| 2 — tools/list | `src/shim/transport.rs` | 117–123 (`list_tools`) | Stub returns `default()` (empty); no daemon catalog endpoint | Medium — build static catalog |
| 3 — Stale PID | `src/daemon/lockfile.rs` | 129–131 (`WouldBlock` arm), 155 (`read_pid`) | Empty PID file during race → `unwrap_or(0)` → misleading "PID 0" error; fix: retry or liveness check | Medium — add `sysinfo` liveness check |
