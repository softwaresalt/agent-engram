# Behavioral Matrix: Refactor Engram Server as Workspace-Local Plugin

**Input**: Design documents from `/specs/004-refactor-engram-server-as-plugin/`
**Prerequisites**: spec.md (required), plan.md (required), data-model.md, contracts/
**Created**: 2026-03-04

## Summary

| Metric | Count |
|---|---|
| **Total Scenarios** | 119 |
| Happy-path | 38 |
| Edge-case | 21 |
| Error | 22 |
| Boundary | 12 |
| Concurrent | 10 |
| Security | 8 |

**Non-happy-path coverage**: 67% (minimum 30% required)

---

## Shim Lifecycle

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S001 | Shim cold start — daemon not running | No daemon process, no PID lock file, plugin installed | MCP client sends `set_workspace` via stdio | Shim acquires PID lock, spawns daemon, waits for ready, forwards request, returns success response via stdout | Daemon process running, PID lock held, shim exits with code 0 | happy-path |
| S002 | Shim warm start — daemon already running | Daemon process running and ready, IPC endpoint accepting | MCP client sends `update_task` via stdio | Shim connects to IPC, forwards request, returns response via stdout | Daemon remains running, shim exits with code 0 | happy-path |
| S003 | Shim forwards tool response faithfully | Daemon running, tool call returns JSON with nested objects | MCP client sends `get_task_graph` via stdio | Shim returns daemon response byte-for-byte (no transformation) via stdout | Shim exits with code 0, response JSON matches daemon output exactly | happy-path |
| S004 | Shim forwards error response faithfully | Daemon running, tool returns EngramError (workspace not set) | MCP client sends `update_task` without prior `set_workspace` | Shim returns JSON-RPC error with code 1001 and message "Workspace not set" | Shim exits with code 0 (protocol-level success, application-level error) | happy-path |
| S005 | Shim cold start completes within 2 seconds | No daemon running, workspace with 1000 tasks in `.engram/` | MCP client sends `set_workspace` via stdio | Response received in under 2 seconds from shim invocation | Daemon started and hydrated, shim exited | boundary |
| S006 | Shim receives malformed JSON on stdin | Daemon running | MCP client sends `{invalid json` on stdin | Shim returns JSON-RPC parse error (code -32700) | Shim exits with code 0 | error |
| S007 | Shim receives empty stdin (EOF immediately) | Any state | MCP client closes stdin without sending data | Shim exits cleanly without crashing | Shim exits with code 0, no daemon spawned if not already running | edge-case |
| S008 | Shim receives request with unknown method | Daemon running | MCP client sends `{"jsonrpc":"2.0","id":1,"method":"nonexistent_tool"}` | Shim forwards to daemon, daemon returns method-not-found error (code -32601) | Shim exits with code 0 | error |
| S009 | Daemon unresponsive — request timeout | Daemon running but hung (not processing) | MCP client sends tool call, daemon does not respond within 60s | Shim returns JSON-RPC timeout error after 60 seconds | Shim exits with code 0 | error |
| S010 | Daemon crashes during shim request | Daemon running, crashes mid-processing | MCP client sends tool call, IPC connection drops | Shim returns JSON-RPC internal error (code -32603) with "daemon connection lost" | Shim exits with code 0 | error |
| S011 | Shim spawn fails — binary not found | Plugin installed but engram binary not on PATH or at expected location | MCP client sends tool call | Shim returns JSON-RPC error with DaemonSpawn error code (8xxx) | Shim exits with code 0, no daemon spawned | error |
| S012 | Shim waits for daemon ready with exponential backoff | No daemon running | MCP client sends tool call, daemon takes 1.5s to become ready | Shim retries IPC connection with exponential backoff, connects when daemon is ready | Daemon running, response returned, shim exits with code 0 | happy-path |
| S013 | Shim daemon spawn timeout — daemon never becomes ready | No daemon running, daemon fails during hydration | MCP client sends tool call | Shim retries for 2 seconds, then returns timeout error | Shim exits with code 0, partial daemon process may exist | error |

---

## IPC Protocol

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S014 | Valid JSON-RPC request round-trip | Daemon running, workspace bound | IPC client sends `{"jsonrpc":"2.0","id":1,"method":"get_workspace_status","params":null}\n` | Daemon returns `{"jsonrpc":"2.0","id":1,"result":{...}}\n` | Connection closed after response | happy-path |
| S015 | Request ID echoed in response — numeric | Daemon running | IPC request with `"id": 42` | Response contains `"id": 42` (exact match, same type) | Connection closed | happy-path |
| S016 | Request ID echoed in response — string | Daemon running | IPC request with `"id": "req-abc-123"` | Response contains `"id": "req-abc-123"` | Connection closed | happy-path |
| S017 | Missing jsonrpc field | Daemon running | IPC request `{"id":1,"method":"get_daemon_status"}` (no jsonrpc) | Daemon returns JSON-RPC invalid request error (code -32600) | Connection closed | error |
| S018 | Wrong jsonrpc version | Daemon running | IPC request with `"jsonrpc": "1.0"` | Daemon returns JSON-RPC invalid request error (code -32600) | Connection closed | error |
| S019 | Missing method field | Daemon running | IPC request `{"jsonrpc":"2.0","id":1}` (no method) | Daemon returns JSON-RPC invalid request error (code -32600) | Connection closed | error |
| S020 | Missing id field | Daemon running | IPC request `{"jsonrpc":"2.0","method":"get_daemon_status"}` (no id) | Daemon returns JSON-RPC invalid request error (code -32600) with `"id": null` | Connection closed | error |
| S021 | Health check internal message | Daemon running | IPC request `{"jsonrpc":"2.0","id":"health","method":"_health"}` | Response includes `status: "ready"`, `uptime_seconds`, `workspace`, `active_connections` | Connection closed, daemon continues | happy-path |
| S022 | Shutdown internal message | Daemon running | IPC request `{"jsonrpc":"2.0","id":"shutdown","method":"_shutdown"}` | Response includes `status: "shutting_down"`, `flush_started: true` | Daemon begins graceful shutdown sequence | happy-path |
| S023 | Multiple messages on same connection (invalid) | Daemon running | Client sends two JSON-RPC requests on same IPC connection | Daemon processes first request only, ignores or rejects second | First response returned, connection closed | edge-case |
| S024 | Oversized request (>1MB JSON payload) | Daemon running | IPC request with params containing 2MB of JSON | Daemon rejects with internal error or processes within memory limits | Connection closed | boundary |
| S025 | Binary data in IPC stream | Daemon running | Client sends raw binary (non-UTF-8) bytes | Daemon returns parse error (code -32700) | Connection closed | error |
| S026 | Newline-delimited framing — no trailing newline | Daemon running | Client sends valid JSON without trailing `\n` | Daemon reads until connection close, processes request | Response returned, connection closed | edge-case |

---

## Daemon Lockfile / PID Management

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S027 | Lock acquisition on fresh workspace | No existing PID lock file | Shim spawns daemon | Daemon creates PID lock file via fd-lock, writes its PID | Lock file exists at `.engram/run/engram.pid`, lock held | happy-path |
| S028 | Lock held by running daemon — second spawn rejected | Daemon running with PID lock held | Second shim tries to spawn a daemon | Second shim detects lock is held, connects to existing daemon instead | Only one daemon process running | happy-path |
| S029 | Stale lock from crashed daemon | PID lock file exists but process is dead (PID no longer valid) | Shim attempts to connect, IPC fails, checks lock | Shim detects stale lock (fd-lock released by OS on process death), cleans up, spawns new daemon | Old lock file replaced, new daemon running | happy-path |
| S030 | Lock file in read-only directory | `.engram/run/` directory exists but is read-only | Daemon attempts to create PID lock | Daemon returns LockAcquisition error (8xxx) | Daemon does not start, shim returns error | error |
| S031 | Lock file path contains spaces | Workspace path is `C:\My Projects\My App` | Daemon creates lock file | Lock file created successfully at path with spaces | Lock held, daemon running | edge-case |
| S032 | Lock cleanup on graceful shutdown | Daemon running with lock held | Daemon graceful shutdown triggered | Lock file removed, fd-lock released | No lock file exists after shutdown | happy-path |
| S033 | Lock survives daemon across multiple shim invocations | Daemon running with lock held | 100 sequential shim invocations | All shims connect to same daemon, lock remains held | Single daemon, lock intact | boundary |

---

## Daemon Lifecycle

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S034 | Daemon starts and transitions to Ready | No daemon running, workspace with valid `.engram/` | `engram daemon --workspace /path/to/ws` | Daemon hydrates from `.engram/` files, binds IPC endpoint, transitions Starting → Ready | DaemonStatus::Ready, IPC accepting connections | happy-path |
| S035 | Daemon hydrates existing data on start | `.engram/tasks.md` contains 5 tasks, `graph.surql` has 3 edges | Daemon starts | All 5 tasks and 3 edges loaded into SurrealDB | Data queryable via tool calls | happy-path |
| S036 | Daemon starts with empty workspace (no `.engram/`) | Workspace exists but no `.engram/` directory | Daemon starts | Daemon creates `.engram/` directory structure, starts with empty state | DaemonStatus::Ready, empty database | happy-path |
| S037 | Daemon graceful shutdown flushes state | Daemon running with pending changes | `_shutdown` IPC message or SIGTERM | Daemon transitions to ShuttingDown, flushes to `.engram/`, closes IPC, removes lock | Process exited, `.engram/` files updated | happy-path |
| S038 | Daemon shutdown while IPC request in progress | Daemon processing a tool call | SIGTERM received | Daemon completes current request, then proceeds with graceful shutdown | Response sent, then clean shutdown | edge-case |
| S039 | Daemon SIGKILL — unclean termination | Daemon running | SIGKILL or system crash | Process terminated immediately, no flush | Lock file exists but fd-lock released by OS, `.engram/` may have stale data | edge-case |
| S040 | Daemon recovery after SIGKILL | Stale lock from previous crash | New daemon starts | Detects stale lock (fd-lock not held), cleans up socket, rehydrates from `.engram/` | New daemon running, data recovered from last flush | happy-path |
| S041 | Daemon start with corrupted `.engram/tasks.md` | `tasks.md` contains malformed YAML frontmatter | Daemon starts | Daemon logs hydration warning, starts with empty/partial state, does not crash | DaemonStatus::Ready with degraded data | error |
| S042 | Daemon start with missing `.engram/.version` | `.engram/` exists but no `.version` file | Daemon starts | Daemon assumes default version, logs warning, proceeds | DaemonStatus::Ready | edge-case |
| S043 | Daemon start on read-only filesystem | Workspace directory is read-only | `engram daemon --workspace /readonly/path` | Daemon fails with clear error: cannot create `.engram/` or write lock file | Daemon exits with non-zero code | error |
| S044 | Daemon logs structured diagnostics | Daemon running | Any tool call or file event | Structured tracing spans emitted to `.engram/logs/engram.log` | Log file contains JSON or pretty tracing output | happy-path |

---

## TTL / Idle Timeout Management

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S045 | Idle timeout expires — clean shutdown | Daemon running, default 4h timeout, no activity | 4 hours of no tool calls and no file events | Daemon flushes state, removes runtime artifacts, exits | Process terminated, lock released, `.engram/` files updated | happy-path |
| S046 | Activity resets idle timer — tool call | Daemon running, 3h59m into idle timeout | Tool call received via IPC | Idle timer reset to full duration (4h from now) | Daemon continues running | happy-path |
| S047 | Activity resets idle timer — file event | Daemon running, 3h59m into idle timeout | File watcher detects a file change | Idle timer reset to full duration | Daemon continues running | happy-path |
| S048 | Custom idle timeout from config | `config.toml` sets `idle_timeout_minutes = 30` | Daemon starts, 30 minutes pass with no activity | Daemon shuts down after 30 minutes | Process terminated | happy-path |
| S049 | Idle timeout zero — daemon runs indefinitely | `config.toml` sets `idle_timeout_minutes = 0` | Daemon starts | Daemon never auto-shuts down regardless of inactivity | Daemon runs until explicit shutdown | boundary |
| S050 | Restart after idle timeout | Daemon shut down by idle timeout | New MCP client sends tool call | Shim detects no daemon, spawns new one, cold start completes | New daemon running, data rehydrated | happy-path |
| S051 | Rapid activity during idle timeout check | Daemon running, periodic TTL check fires | 1000 tool calls in 1 second | Each call resets timer; timer never reaches expiry | Daemon continues running | boundary |

---

## File System Watcher

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S052 | File created in workspace | Daemon running with watcher active | New file `src/main.rs` created | WatcherEvent(Created) emitted after debounce, triggers code graph pipeline | File indexed, available in queries within 2 seconds | happy-path |
| S053 | File modified in workspace | Daemon running, `src/lib.rs` already indexed | `src/lib.rs` content changed | WatcherEvent(Modified) emitted after debounce, triggers re-index | Updated content reflected in queries | happy-path |
| S054 | File deleted in workspace | Daemon running, `src/old.rs` indexed | `src/old.rs` deleted | WatcherEvent(Deleted) emitted, triggers removal from index | File no longer appears in queries | happy-path |
| S055 | Rapid saves debounced to single event | Daemon running, debounce = 500ms | `src/lib.rs` modified 10 times in 200ms | Single WatcherEvent(Modified) emitted after 500ms debounce | One pipeline trigger, not ten | happy-path |
| S056 | `.engram/` directory changes ignored | Daemon running | File modified in `.engram/tasks.md` (by flush_state) | No WatcherEvent emitted, no re-indexing triggered | Watcher exclusion list filters it out | happy-path |
| S057 | `.git/` directory changes ignored | Daemon running | Git objects modified in `.git/objects/` | No WatcherEvent emitted | Watcher exclusion filters it out | happy-path |
| S058 | `node_modules/` changes ignored | Daemon running | Package installed, files created in `node_modules/` | No WatcherEvent emitted | Watcher exclusion filters it out | happy-path |
| S059 | `target/` directory changes ignored | Daemon running | Cargo build output in `target/debug/` | No WatcherEvent emitted | Watcher exclusion filters it out | happy-path |
| S060 | Custom exclusion pattern from config | `config.toml` adds `exclude_patterns = ["build/", "dist/"]` | File created in `build/output.js` | No WatcherEvent emitted | Custom exclusion respected | happy-path |
| S061 | Custom watch pattern from config | `config.toml` sets `watch_patterns = ["src/**/*.rs"]` | File created in `docs/readme.md` (outside pattern) | No WatcherEvent emitted for docs file | Only matched patterns trigger events | edge-case |
| S062 | File renamed in workspace | Daemon running | `src/old.rs` renamed to `src/new.rs` | WatcherEvent(Renamed) with old and new paths | Old path removed from index, new path added | happy-path |
| S063 | Large batch of file creates (git checkout) | Daemon running | `git checkout` creates 500 files simultaneously | Events debounced, processed in batches, no individual per-file overhead | All files indexed progressively, no blocking | edge-case |
| S064 | Watcher initialization failure — inotify limit | Daemon running on Linux with inotify watch limit reached | Daemon attempts to set up watcher | WatcherInit error logged, daemon continues without file watching | Daemon running but degraded (no file events) | error |
| S065 | Symlink in workspace | Daemon running | Symlinked file `src/link.rs` modified | Event triggered for the symlink path | File indexed at its apparent path | edge-case |
| S066 | Binary file modified | Daemon running | `image.png` modified in workspace | WatcherEvent emitted but code graph pipeline skips non-text files | No crash, binary file not indexed as code | edge-case |

---

## Plugin Installer

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S067 | Install in clean workspace | Workspace exists, no `.engram/` directory | `engram install` | Creates `.engram/` structure (tasks.md, .version, config stub), generates `.vscode/mcp.json`, verification passes | `.engram/` directory exists with expected structure, exit code 0 | happy-path |
| S068 | Install in workspace with existing `.engram/` | `.engram/` already exists with data | `engram install` | Detects existing installation, returns error or warning suggesting `update` instead | No files overwritten, exit code non-zero | error |
| S069 | Update preserves stored data | `.engram/` exists with 10 tasks | `engram update` | Runtime artifacts updated, `tasks.md` and `graph.surql` preserved unchanged | Updated runtime, all 10 tasks intact | happy-path |
| S070 | Reinstall after corruption | `.engram/` exists with corrupt database files | `engram reinstall` | Removes runtime artifacts, re-creates structure, rehydrates from `tasks.md`/`graph.surql` | Fresh runtime, data recovered from Markdown/SurQL files | happy-path |
| S071 | Uninstall with data preservation | `.engram/` exists with data, `--keep-data` flag | `engram uninstall --keep-data` | Runtime artifacts (lock, socket, PID, logs) removed; `tasks.md`, `graph.surql`, `config.toml` preserved | `.engram/` exists with data files only | happy-path |
| S072 | Uninstall with full removal | `.engram/` exists with data | `engram uninstall` (no keep flag) | Entire `.engram/` directory removed | No `.engram/` directory | happy-path |
| S073 | Install while daemon is running | Daemon currently running for this workspace | `engram install` | Detects running daemon, returns error instructing user to stop daemon first | No changes, exit code non-zero | error |
| S074 | Uninstall while daemon is running | Daemon currently running | `engram uninstall` | Sends `_shutdown` to daemon, waits for exit, then removes artifacts | Daemon stopped, artifacts removed | happy-path |
| S075 | Install generates correct MCP config | Workspace at `/home/user/project` | `engram install` | `.vscode/mcp.json` contains correct command path and workspace argument | Config file is valid JSON, references engram binary | happy-path |
| S076 | Install in path with spaces | Workspace at `C:\My Projects\My App` | `engram install` | All paths correctly escaped, `.engram/` created, MCP config valid | Fully functional installation | edge-case |
| S077 | Install in path with Unicode | Workspace at `/home/用户/项目` | `engram install` | All paths handled correctly, `.engram/` created | Fully functional installation | edge-case |
| S078 | Install on read-only filesystem | Workspace directory is read-only | `engram install` | Clear error: "Cannot create .engram/ directory: permission denied" | No partial artifacts left, exit code non-zero | error |

---

## Configuration

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S079 | No config file — use defaults | No `.engram/config.toml` exists | Daemon starts | Daemon uses defaults: 4h timeout, 500ms debounce, standard exclusions | Daemon running with default configuration | happy-path |
| S080 | Valid config file parsed | `.engram/config.toml` with `idle_timeout_minutes = 30` | Daemon starts | 30-minute timeout applied | Daemon running with custom configuration | happy-path |
| S081 | Config file with all fields set | `.engram/config.toml` with all fields populated | Daemon starts | All custom values applied | Daemon running with fully custom configuration | happy-path |
| S082 | Config file with unknown field | `.engram/config.toml` contains `unknown_field = true` | Daemon starts | Unknown field ignored with warning log, daemon starts normally | Daemon running, warning logged | edge-case |
| S083 | Config file with invalid TOML syntax | `.engram/config.toml` contains malformed TOML | Daemon starts | ConfigParse error logged, daemon falls back to all defaults | Daemon running with defaults, error logged | error |
| S084 | Config with negative timeout value | `.engram/config.toml` sets `idle_timeout_minutes = -1` | Daemon starts | ConfigParse validation error, falls back to default timeout | Daemon running with default 4h timeout, warning logged | error |
| S085 | Config with extremely large debounce | `.engram/config.toml` sets `debounce_ms = 999999999` | Daemon starts | Value accepted or clamped to maximum; daemon starts | Daemon running, debounce may be clamped | boundary |
| S086 | Config file changed at runtime | Daemon running, `.engram/config.toml` modified | Config file saved | No effect until daemon restart (per spec: "changes take effect on next service restart") | Daemon continues with original config | edge-case |
| S087 | Config debounce_ms set to 0 | `.engram/config.toml` sets `debounce_ms = 0` | Daemon starts | Every file event processed immediately without debouncing | Daemon running with zero debounce | boundary |

---

## Workspace Isolation

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S088 | Two workspaces running concurrently — no data leakage | Workspace A has tasks, Workspace B has different tasks | Query tasks in Workspace A | Only Workspace A tasks returned, zero Workspace B data | Each daemon isolated to its own SurrealDB namespace | happy-path |
| S089 | Two workspaces with separate IPC channels | Workspace A at `/ws/a`, Workspace B at `/ws/b` | Both daemons start simultaneously | Each daemon binds its own IPC endpoint (different socket/pipe) | Two independent IPC channels, no collision | happy-path |
| S090 | 20 concurrent workspaces | 20 different workspace paths | All 20 daemons started | All 20 running with independent state, IPC, and locks | <50MB idle memory per daemon, no conflicts | boundary |
| S091 | Workspace path with symlink | Workspace at `/ws/real`, symlink `/ws/link` → `/ws/real` | Daemon started via symlink path | Canonical path resolved, same SHA-256 hash as real path | Single daemon regardless of access path | edge-case |
| S092 | Workspace directory moved while daemon running | Daemon running for `/ws/project` | User renames `/ws/project` to `/ws/project-old` | Daemon detects invalidation (IPC socket/lock path invalid), shuts down | Daemon exits cleanly | edge-case |

---

## Error Recovery & Resilience

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S093 | Disk full during flush | Daemon running, disk at 100% capacity | `flush_state` tool call or idle shutdown flush | Atomic write fails (temp file creation fails), no partial `.engram/` corruption | Error returned to caller, previous `.engram/` files intact | error |
| S094 | Recovery after disk-full flush failure | Previous flush failed due to disk full, disk now has space | Next `flush_state` call or idle shutdown | Flush succeeds, `.engram/` files updated | Data consistent | happy-path |
| S095 | Corrupted `.engram/tasks.md` — rehydration recovery | `tasks.md` has invalid Markdown structure | Daemon starts | Hydration error logged, daemon starts with partial/empty state | DaemonStatus::Ready, degraded data, error context available | error |
| S096 | Power loss during database write | SurrealDB write in progress | System crash | On restart, SurrealDB recovers via WAL (write-ahead log) | Data consistent to last committed transaction | error |

---

## Security

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S097 | Unix socket permissions | Daemon running on Linux/macOS | Socket file created at `.engram/run/engram.sock` | Socket file permissions set to `0o600` (owner read/write only) | Only workspace owner can connect | security |
| S098 | Windows named pipe ACL | Daemon running on Windows | Named pipe `\\.\pipe\engram-{hash}` created | Pipe ACL restricts access to creating user only | Only workspace owner can connect | security |
| S099 | Path traversal in workspace_path | Daemon running | `set_workspace` called with `../../etc/sensitive` | Path rejected — workspace path must be canonical, no `..` components after resolution | Error returned, no file access outside workspace | security |
| S100 | Lock file prevents unauthorized daemon replacement | Daemon running with PID lock | Another process attempts to write daemon PID file | fd-lock prevents concurrent write, second process fails to acquire lock | Original daemon continues, intruder rejected | security |
| S101 | IPC message injection — oversized method name | Daemon running | IPC request with 10MB `method` string | Daemon rejects with parse/validation error within memory limits | No memory exhaustion | security |
| S102 | No secrets in `.engram/` files | Daemon running with environment variables containing secrets | `flush_state` call | `.engram/` files contain only task/context/graph data, no env vars or credentials | Files safe to commit to Git | security |
| S103 | Log files exclude sensitive data | Daemon running, tool calls include workspace paths | Daemon logs diagnostic information | Log files at `.engram/logs/` contain operational data only, no secret material | Logs safe for sharing in bug reports | security |

---

## MCP Tool Compatibility

These scenarios verify backward compatibility (SC-008) — existing tools produce identical results through the new shim/IPC transport.

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S104 | set_workspace via new transport | Daemon running, workspace not yet bound | `set_workspace` with valid path via shim→IPC | Returns hydration result identical to previous HTTP/SSE transport | Workspace bound, data hydrated | happy-path |
| S105 | get_daemon_status via new transport | Daemon running | `get_daemon_status` via shim→IPC | Returns uptime, connection count, workspace list — same schema as before | No state change | happy-path |
| S106 | update_task creates context note | Daemon running, workspace bound, task exists | `update_task` with status change via shim→IPC | Task updated AND context note created (FR-015 preserved) | Task status changed, context note recorded | happy-path |
| S107 | get_task_graph returns dependency tree | Daemon running, workspace bound, tasks with edges | `get_task_graph` via shim→IPC | Recursive graph traversal result identical to current | No state change | happy-path |
| S108 | flush_state dehydrates to .engram/ | Daemon running, workspace bound, data modified | `flush_state` via shim→IPC | `.engram/tasks.md` and `.engram/graph.surql` written atomically | Files updated with current DB state | happy-path |
| S109 | 100k+ file indexing with concurrent tool call | Daemon running, 100k+ file workspace, initial indexing in progress | `get_task_graph` via shim→IPC during background indexing | Tool call returns available results immediately without blocking on indexing completion | Response <50ms, indexing continues in background | boundary |
| S110 | add_blocker via new transport | Daemon running, workspace bound, two tasks exist | `add_blocker` with valid task_id and blocker_id via shim→IPC | Blocker edge created, response identical to HTTP/SSE transport | Edge stored in SurrealDB | happy-path |
| S111 | register_decision via new transport | Daemon running, workspace bound | `register_decision` with title and content via shim→IPC | Decision recorded as context, response identical to HTTP/SSE transport | Context entry created | happy-path |
| S112 | check_status via new transport | Daemon running, workspace bound, tasks exist | `check_status` with work_item_ids via shim→IPC | Batch status lookup returns same schema as HTTP/SSE transport | No state change | happy-path |
| S113 | query_memory via new transport | Daemon running, workspace bound, embeddings available | `query_memory` with search string via shim→IPC | Semantic search results identical to HTTP/SSE transport | No state change | happy-path |
| S114 | get_workspace_status via new transport | Daemon running, workspace bound | `get_workspace_status` via shim→IPC | Returns task/context counts, flush state, staleness — same schema | No state change | happy-path |

---

## Additional Edge Cases (from adversarial review)

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S115 | Shim connects during daemon ShuttingDown state | Daemon in ShuttingDown state (flushing) | New MCP client sends tool call via stdio | Shim connects to IPC, daemon rejects with "shutting down" error | Shim returns JSON-RPC error, daemon continues shutdown | edge-case |
| S116 | Two shims spawn daemon within 10ms (race) | No daemon running | Two MCP clients send tool calls simultaneously (within 10ms) | Only one daemon spawned — second shim detects lock or running daemon and connects | Single daemon running, both shims receive responses | concurrent |
| S117 | Idle timeout cleanup completes within 60s | Daemon running, idle timeout fires | Timeout expires, daemon begins shutdown | All resources (process, lock, socket, file handles) released within 60 seconds of timeout | Zero resources consumed per SC-005 | boundary |
| S118 | Watcher event from symlinked directory outside workspace | Daemon running, workspace contains symlink to /external/dir | File modified in /external/dir | Event filtered — resolved absolute path is outside workspace boundary | No WatcherEvent emitted, no indexing triggered | security |
| S119 | Unix socket path exceeds 108-byte limit | Workspace at deeply nested path (>108 bytes from root) | Daemon attempts to create UDS socket | Daemon detects path overflow, falls back to /tmp/engram-{hash}.sock with 0o600 permissions | Daemon starts with fallback socket path | edge-case |

---

## Edge Case Coverage Checklist

- [x] Malformed inputs and invalid arguments (S006, S017-S020, S025)
- [x] Missing dependencies and unavailable resources (S011, S043, S064, S078)
- [x] State errors and race conditions (S028, S029, S038-S040, S092)
- [x] Boundary values (empty, max-length, zero, negative) (S005, S024, S033, S049, S051, S085, S087, S090)
- [x] Permission and authorization failures (S030, S043, S078, S097-S100)
- [x] Concurrent access patterns (S028, S033, S063, S088-S090, S100)
- [x] Graceful degradation scenarios (S041, S064, S083, S093, S095)

## Cross-Reference Validation

- [x] Every entity in `data-model.md` has at least one scenario covering its state transitions
  - DaemonState: S034-S044
  - DaemonStatus (Starting→Ready→ShuttingDown): S034, S037, S045
  - IpcRequest/IpcResponse: S014-S026
  - IpcError: S017-S020, S025
  - WatcherEvent/WatchEventKind: S052-S066
  - PluginConfig: S079-S087
- [x] Every endpoint in `contracts/` has at least one happy-path and one error scenario
  - IPC tool calls: S014 (happy), S017-S020 (error)
  - Health check: S021 (happy)
  - Shutdown: S022 (happy)
  - MCP tools: S104-S114 (happy), S004/S008 (error)
- [x] Every user story in `spec.md` has corresponding behavioral coverage
  - US1 (Zero-Config): S001, S002, S012, S034-S036
  - US2 (Workspace Isolation): S088-S092
  - US3 (Lifecycle Management): S045-S051
  - US4 (File Watching): S052-S066
  - US5 (Plugin Install): S067-S078
  - US6 (Configuration): S079-S087
- [x] Every edge case in `spec.md` has corresponding scenario coverage
  - Spaces/Unicode paths: S031, S076, S077
  - Workspace moved/renamed: S092
  - Simultaneous start race: S028
  - Corrupted `.engram/` files: S041, S095
  - Disk full during flush: S093
  - Read-only filesystem: S043, S078
  - SIGKILL recovery: S039, S040
  - Large workspace indexing: S063 (plus S019 from FR-019 via background indexing in plan)
- [x] No scenario has ambiguous or non-deterministic expected outcomes

## Notes

- Scenario IDs are globally sequential (S001–S119) across all components
- Categories: `happy-path`, `edge-case`, `error`, `boundary`, `concurrent`, `security`
- Each row is deterministic — exactly one expected outcome per input state
- Tables are grouped by component/subsystem under level-2 headings
- Scenarios map directly to parameterized test cases (Rust `#[rstest]` blocks)
- S090 (20 concurrent workspaces) validates SC-004 memory constraint (<50MB per idle daemon)
- S005 validates SC-003 cold start (<2 seconds)
- S055 validates SC-006 debounce behavior
- S104-S114 validate SC-008 backward compatibility
