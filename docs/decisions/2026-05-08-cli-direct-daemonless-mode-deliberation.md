---
title: "CLI-Direct (Daemonless) Mode for Indexing Operations"
description: "Deliberation on whether engram CLI should support standalone database indexing without requiring a running daemon"
topic: "CLI-direct daemonless mode"
depth: "standard"
decision_status: "decided"
promoted_to: "queue"
linked_artifacts:
  - ".backlogit/queue/044-F.md"
tags:
  - "cli"
  - "architecture"
  - "indexing"
  - "daemon"
---

## Problem Frame

All engram CLI subcommands currently route through the daemon via IPC (`run_tool()` → `send_request()` → daemon handler). There is no direct-to-DB path. This creates friction in two scenarios:

1. **`start.ps1` / `start.sh` preloading**: The startup script wants to call `engram sync` to pre-populate the code graph before launching Copilot. Currently this auto-spawns a daemon, waits for it to reach "ready" state (which includes `background_db_hydration`), and then the sync IPC request competes with ongoing indexing — hitting either a 30s timeout or an `IndexInProgress` error.

2. **One-shot CLI usage**: A developer runs `engram index` from the terminal expecting a simple "parse, index, exit" workflow — like `cargo build` or `git status`. Instead they get a persistent daemon process left running, with its watcher, IPC server, and idle timeout.

The operator wants the ability to run `engram sync` or `engram index` as a standalone process that opens the database directly, performs the operation synchronously, and exits cleanly — without spawning or requiring a daemon.

### Success Criteria

- `engram sync` and `engram index` can run without a daemon
- No orphaned daemon processes after CLI-direct operations
- Concurrent safety: CLI-direct and daemon cannot corrupt the database
- Existing daemon-based workflow continues to work unchanged
- Exit code semantics preserved (0 = success, 1 = tool error, 2 = invocation)

### Scope Boundaries

- **In scope**: `sync`, `index`, `flush` subcommands
- **Out of scope**: Read-only query commands (`search`, `symbols`, `map-code`, `impact`, `stats`) — these depend on an indexed database, which implies either a prior indexing run or a running daemon. Daemonless reads are a future extension if this pattern proves successful.
- **Out of scope**: File watcher functionality — that's inherently a daemon concern.

## Research Findings

### Architecture Analysis

The service layer is already decoupled from the daemon:

| Function | Location | Parameters | Daemon Required? |
|---|---|---|---|
| `connect_db()` | `src/db/cozo_backend/mod.rs` | `(data_dir, branch)` | No |
| `index_workspace()` | `src/services/code_graph.rs` | `(ws_path, data_dir, branch, config, force)` | No |
| `sync_workspace()` | `src/services/code_graph.rs` | `(ws_path, data_dir, branch, config)` | No |
| `hydrate_code_graph()` | `src/services/hydration.rs` | `(canonical, data_dir, branch, queries)` | No |
| `parse_config()` | `src/services/config.rs` | `(path)` | No |
| `resolve_data_dir()` | `src/db/workspace.rs` | `(workspace_root)` | No |
| `resolve_git_branch()` | `src/db/workspace.rs` | `(workspace_root)` | No |

**Key finding**: Every function needed for indexing accepts plain path/config parameters — none require `AppState`, `Arc<AppState>`, or any daemon runtime. The daemon wraps these functions with concurrency guards (`try_start_indexing()`, `finish_indexing()`) and state management, but the core operations are standalone.

### Concurrency Safety

CozoDB uses SQLite storage with WAL mode. The existing `connect_db()` already implements:

1. **File-level advisory lock** (`engram.db.lock`) with 30s timeout polling
2. **Schema bootstrap serialization** — lock held through `run_schema_bootstrap`
3. **WAL concurrency** — CozoDB's SQLite WAL handles multiple readers after bootstrap

**Risk**: If the daemon is running AND a CLI-direct process opens the same DB, both hold separate `CozoDb` handles. SQLite WAL allows this (multiple readers, single writer), but concurrent writes cause `SQLITE_BUSY`. The existing `run_script_busy_retry_mutable` retries writes up to 5 times with 500ms backoff, which provides some protection.

### Prior Art

- `cargo` operates as a standalone CLI — no daemon
- `git` status/log/diff work without a daemon; `git fsmonitor--daemon` is optional
- `ripgrep`, `fd` — pure standalone
- SQLite's own CLI (`sqlite3`) opens the DB directly

## Options Evaluated

### Option A: `--no-daemon` Flag

Add a `--no-daemon` flag to `sync` and `index` subcommands. When present, the CLI opens CozoDB directly, calls `index_workspace()` / `sync_workspace()` from the services layer, prints the result, and exits.

**Implementation:**
- Add `--no-daemon` to `GlobalFlags` (applies to sync/index only)
- Create `src/cli/direct.rs` — a lightweight runner that resolves workspace, opens DB, runs the service function, formats output
- When `--no-daemon` is set, bypass `run_tool()` entirely
- Check daemon lockfile: if daemon is running, warn but proceed (SQLite WAL allows it)

**Pros:**
- Backward-compatible — default behavior unchanged
- Clear opt-in semantics
- Minimal code: ~100 lines for the direct runner
- Uses existing, tested service functions

**Cons:**
- Two code paths for the same operation (daemon vs direct)
- User must remember to pass the flag
- `--no-daemon` is a double-negative name (confusing)

**Effort:** Low

### Option B: Auto-Detect Mode

Make `sync` and `index` auto-detect whether a daemon is running. If yes, route through IPC. If no, run directly without spawning one.

**Implementation:**
- Check daemon PID file / named pipe before calling `ensure_daemon_running()`
- If daemon detected: use existing IPC path
- If no daemon: use direct runner from Option A
- No new flags needed

**Pros:**
- Zero configuration — "just works"
- No behavioral change for users with daemon running
- Aligns with `git`'s model (fsmonitor optional)

**Cons:**
- Implicit behavior may surprise users who expect daemon auto-spawn
- Harder to test — behavior depends on runtime state
- Edge case: daemon crashes mid-operation, next CLI call silently switches mode
- Loses the `ensure_daemon_running()` auto-spawn convenience

**Effort:** Medium

### Option C: Separate Binary / Subcommand

Create a new subcommand (`engram run-direct sync`) or alias (`engram-sync`) that always runs in direct mode.

**Pros:**
- No ambiguity — separate entry point
- Easy to document

**Cons:**
- More surface area to maintain
- Confusing: "when do I use `engram sync` vs `engram run-direct sync`?"
- Unnecessary complexity for a flag-level feature

**Effort:** Medium

### Option D: `--direct` Flag (Recommended)

Add a `--direct` flag (positive naming) to `sync` and `index`. When present, bypass the daemon and operate directly on the database. Default behavior (no flag) preserves the daemon IPC path with auto-spawn.

**Implementation:**
- Add `--direct` to the `Sync` and `Index` subcommand structs (not `GlobalFlags` — only applies to indexing commands)
- Create `src/cli/direct.rs`:
  - Resolve workspace root, data dir, branch, config
  - Check daemon lockfile: if daemon is running, emit a warning that concurrent writes may cause retries
  - Call `connect_db()` → `index_workspace()` or `sync_workspace()`
  - Format and print the result JSON
- Support `ENGRAM_DIRECT=1` env var for scripted usage (start.ps1)
- Add integration test: run `engram sync --direct` on a test workspace

**Pros:**
- Positive flag name (not a double-negative)
- Backward-compatible — default unchanged
- Scoped to commands where it makes sense (not `search` or `stats`)
- Env var support for scriptable CI/startup scenarios
- Clean separation: `run_tool()` path vs `run_direct()` path

**Cons:**
- Two code paths (but the service functions are already tested)
- Need to handle config loading outside the daemon context

**Effort:** Low-Medium

## Trade-off Comparison

| Criterion | A: `--no-daemon` | B: Auto-detect | C: Separate binary | D: `--direct` |
|---|---|---|---|---|
| Backward compat | ✅ Full | ⚠️ Changes auto-spawn | ✅ Full | ✅ Full |
| User clarity | ⚠️ Double negative | ❌ Implicit magic | ⚠️ Two entry points | ✅ Clear intent |
| Implementation | Low | Medium | Medium | Low-Medium |
| Scriptability | ✅ Flag + env var | ✅ Automatic | ⚠️ Different binary | ✅ Flag + env var |
| Concurrency risk | Same | Same | Same | Same |
| Testing burden | Low | Medium | Medium | Low |

## Decision

**Option D: `--direct` flag** — recommended and chosen.

### Rationale

1. **Positive naming** — `--direct` is clearer than `--no-daemon`
2. **Scoped application** — only on `sync` and `index`, not globally
3. **Env var** — `ENGRAM_DIRECT=1` enables `start.ps1` without flag passing
4. **Minimal risk** — services layer is already standalone; we're just adding a new entry point
5. **Preserves daemon auto-spawn** — existing MCP/Copilot workflow unchanged

### Mutual exclusion via `DaemonLock`

The operator confirmed that CLI-direct and the daemon should NOT run in parallel on the same workspace. The existing `DaemonLock` (`src/daemon/lockfile.rs`) already provides the exact mutual exclusion primitive needed:

- `DaemonLock::acquire(workspace)` takes an exclusive OS-level write lock on `.engram/run/engram.lock`
- The lock is released when the `DaemonLock` is dropped or the process exits
- Stale lock detection (dead PID) with automatic cleanup is already implemented

**CLI-direct implementation:**

1. Before opening the DB, call `DaemonLock::acquire(workspace)`
2. If `LockError::AlreadyHeld { pid }` → print `"error: daemon (PID {pid}) is running in this workspace. Stop it first or omit --direct."` and exit with code 2
3. Hold the `DaemonLock` for the duration of the indexing operation
4. On drop (normal exit or error), the lock is released automatically

This ensures only one process (daemon OR CLI-direct) can write to the DB at any time. No concurrent SQLITE_BUSY risk. No new locking code needed — reuse `DaemonLock` as-is.

### `start.ps1` recommended pattern

```powershell
# Pre-populate the code graph before launching Copilot.
# Uses --direct so no daemon is left running; the shim will spawn one later.
engram sync --direct --json

# Now launch Copilot — the shim spawns the daemon on first MCP call.
copilot
```

Or with env var:

```powershell
$env:ENGRAM_DIRECT = "1"
engram sync --json
Remove-Item Env:\ENGRAM_DIRECT
copilot
```

## Rejected Alternatives

- **Option B (Auto-detect)**: Too much implicit behavior. Users expect `engram sync` to behave consistently regardless of daemon state. Silent mode-switching would make debugging harder.
- **Option C (Separate binary)**: Unnecessary complexity. A flag achieves the same result without doubling the command surface.
- **Option A (`--no-daemon`)**: Functionally equivalent to Option D but with a confusing double-negative name.

## Unresolved Questions

1. ~~**Read-only direct mode**~~: Should `search`, `symbols`, `stats` also support `--direct`? Deferred — these depend on an indexed database. If the user ran `engram sync --direct` first, the DB is populated and reads would work. But this is a future extension.

2. ~~**Config resolution in direct mode**~~: The daemon loads config via `parse_config()` and caches it in `AppState`. Direct mode needs to call `parse_config()` independently. The function already accepts a path, so this is straightforward.

3. ~~**Daemon lockfile check**~~: Resolved — `DaemonLock::acquire()` provides mutual exclusion. CLI-direct refuses if daemon is running.

4. **Freshness handoff** (045.004-T): The daemon's `detect_offline_changes()` already skips re-indexing when `offline_count == 0`. But `sync_workspace()` does NOT record file hashes (only `index_workspace` does). This means a `--direct` sync leaves stale hashes and the daemon will detect phantom changes. Fix: add `record_file_hash` calls to `sync_workspace()` and write an index freshness marker after completion.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Daemon already running when `--direct` used | Medium | None (blocked) | `DaemonLock::acquire()` returns `AlreadyHeld`; CLI exits with clear error |
| Schema drift between daemon and direct | Low | Medium | Both call `run_schema_bootstrap` idempotently |
| Config not loaded in direct mode | Low | Low | Call `parse_config()` explicitly |
| User confusion about when to use `--direct` | Low | Low | Document: use `--direct` for one-shot CLI; omit for MCP/Copilot |
| Stale lock after CLI crash | Low | Low | `DaemonLock` stale-PID detection already handles this |
