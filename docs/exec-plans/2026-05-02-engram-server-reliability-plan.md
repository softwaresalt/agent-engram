---
title: "Engram Server Reliability & Dog-Fooding — Implementation Plan"
description: "Technical plan for fixing daemon startup hang and enabling local dog-fooding of the engram MCP server"
source: "docs/decisions/2026-05-02-engram-server-reliability-dogfooding-deliberation.md"
feature_id: "025-F"
stash_id: "9B4996E5"
status: "draft"
created_at: "2026-05-02"
---

## Problem Frame

The engram daemon (`target/release/engram.exe daemon --workspace <path>`) hangs
during startup and never reaches the Ready state that the shim needs to forward
MCP tool calls. The hang occurs between step 4 (idle TTL configuration, logged at
`src/daemon/mod.rs:136`) and step 8 (IPC server start via `run_with_shutdown`).

The daemon startup sequence in `src/daemon/mod.rs::run()` has synchronous,
potentially-blocking operations (file watcher initialization, workspace-moved
detection setup) that execute BEFORE the IPC listener is bound. Since the shim
polls the IPC endpoint for readiness, any blocking operation before the bind
prevents the daemon from ever reaching Ready.

The code already implements "bind first, hydrate later" for workspace initialization
inside `run_with_shutdown` (lines 398-405 of `src/daemon/ipc_server.rs`), but the
caller (`daemon::run`) has its own blocking sequence before calling `run_with_shutdown`.

### Affected code paths

| File | Role | Issue |
|------|------|-------|
| `src/daemon/mod.rs` | Daemon entry point | Steps 5-7 run before IPC bind |
| `src/daemon/watcher.rs` | File watcher init | `start_watcher` with `RecursiveMode::Recursive` may block |
| `src/shim/lifecycle.rs` | Shim daemon discovery | Reports NotReady on timeout |
| `src/daemon/ipc_server.rs` | IPC listener | Already uses bind-first pattern internally |

## Requirements Trace

| Requirement (from deliberation) | Implementation Action |
|---|---|
| Daemon must not hang during startup | Move blocking ops after IPC bind (Unit 2) |
| Identify exact blocking operation | Add diagnostic instrumentation (Unit 1) |
| Shim can connect and serve tool calls | End-to-end verification (Unit 3) |
| Startup completes within 5 seconds | Timeout guard on watcher init (Unit 2) |
| MCP config works for local dev | Verify `.vscode/mcp.json` path resolution (Unit 3) |

## Implementation Units

### Unit 1: Diagnostic Instrumentation — Identify Startup Blocker

**Goal**: Pinpoint the exact operation that blocks daemon startup between
"idle TTL configured" and "IPC listener bound".

**Changes needed**:

1. Add `info!` tracing spans at each step boundary in `src/daemon/mod.rs::run()`
   between lines 136-203:
   - Before/after signal handler spawn (step 6)
   - Before/after `start_watcher` call (step 7)
   - Before/after workspace-moved detector spawn
   - Before `run_with_shutdown` call (step 8)

2. Run the daemon and confirm which operation blocks.

3. Document the root cause in the commit message.

**Files affected**: `src/daemon/mod.rs`

**Tests**: Existing daemon lifecycle tests must still pass. No new tests needed
for instrumentation (it's diagnostic logging).

**Execution posture**: Investigation-first — add traces, run, observe, document.

**Acceptance criteria**:
- Root cause identified and documented
- Diagnostic traces added at each startup phase boundary
- Daemon output clearly shows which step blocks

---

### Unit 2A: Refactor Daemon Startup — Move Watcher Init After IPC Bind

**Goal**: Ensure the daemon's IPC listener is bound as early as possible by
moving `start_watcher` (a synchronous, potentially-blocking call) to after
the IPC endpoint is bound.

**Critical design note**: `start_watcher` is a **synchronous** function
(`src/daemon/watcher.rs:107`). It calls `new_debouncer` and `debouncer.watch()`
which are both blocking I/O operations (Windows `ReadDirectoryChangesW`
registration). An async `tokio::time::timeout` CANNOT preempt a blocking
synchronous call. The correct fix is architectural: bind IPC first, then run
the watcher init.

**Changes needed**:

1. **Restructure `daemon::run()` startup order**: Remove `start_watcher` from
   its current position (before `run_with_shutdown`) and move it into
   `run_with_shutdown` AFTER the IPC listener is bound (after line 407
   "IPC listener bound").

   Specifically: pass `WatcherConfig` into `run_with_shutdown` and call
   `start_watcher` between `bind_listener` (line 407) and the background
   workspace hydration task (line 447). This positions the watcher init after
   the pipe exists but before the accept loop starts.

2. **Use `tokio::task::spawn_blocking` for watcher init**: Since `start_watcher`
   is synchronous and may block the tokio runtime thread, wrap it in
   `spawn_blocking` with a 5-second `tokio::time::timeout`. If it times out or
   errors, continue in degraded mode (consistent with existing graceful
   degradation at `src/daemon/watcher.rs:120-124`).

3. **Update `run_with_shutdown` signature**: Accept `WatcherConfig` as a new
   parameter. Create the `mpsc::unbounded_channel` for watcher events inside
   `run_with_shutdown` rather than in the caller.

**Files affected**:
- `src/daemon/mod.rs` (remove watcher init, pass config to run_with_shutdown)
- `src/daemon/ipc_server.rs` (accept WatcherConfig param, init watcher after bind)

**Tests**:
- Extend `tests/integration/daemon_lifecycle_test.rs` with a test that
  asserts daemon reaches Ready within 5 seconds
- Verify existing contract tests still pass

**Execution posture**: Test-first — write a test asserting Ready within 5s,
confirm it fails on current code, then implement.

**Acceptance criteria**:
- Daemon reaches "IPC listener bound" within 1 second of startup
- File watcher initialization does not block IPC availability
- Watcher timeout/failure produces graceful degraded mode
- All existing tests pass

---

### Unit 2B: Stale Runtime State Cleanup

**Goal**: Clean up stale PID files from dead daemon processes to prevent
confusing diagnostics and stale-state interference.

**Changes needed**:

1. In `daemon::run()`, before acquiring the daemon lock, check if
   `engram.pid` references a dead process (PID not alive). If so, remove
   the stale PID file and log an info message.

2. The lockfile acquisition (`DaemonLock::acquire`) already handles stale
   locks via fd-lock, so only the PID file needs explicit cleanup.

**Files affected**: `src/daemon/mod.rs`

**Tests**:
- Leverage existing `tests/integration/stale_pid_recovery_test.rs` — verify
  it still passes and covers this scenario

**Execution posture**: Characterization-first — verify existing test covers
stale PID scenario, then add cleanup logic.

**Acceptance criteria**:
- Stale PID file from dead process is cleaned up on startup
- Log message documents the cleanup action
- Existing stale PID tests pass

---

### Unit 3: End-to-End Dog-Fooding Verification

**Goal**: Verify the full MCP connectivity chain works for the local
development workflow after Units 1, 2A, and 2B.

**Changes needed**:

1. **Verify shim → daemon → tool call → response**: Run
   `target/release/engram.exe shim` with a JSON-RPC request on stdin and
   confirm a valid MCP response on stdout.

2. **Verify `.vscode/mcp.json` configuration**: Ensure the path to the release
   binary and the `ENGRAM_WORKSPACE` env var produce a working connection
   from Copilot.

**Files affected**: No production code changes.

**Tests**: Manual verification of the MCP protocol flow using the existing
integration test harness pattern from `tests/integration/`.

**Execution posture**: Verification-first — run the built binary, exercise the
MCP protocol, confirm tool responses.

**Acceptance criteria**:
- `engram shim` successfully spawns daemon and returns tool list
- At least one tool call (e.g., `get_daemon_status`) returns valid JSON
- Copilot can discover and invoke engram tools from VS Code

## Dependency Graph

```text
Unit 1 ─── (identifies root cause) ───→ Unit 2A
                                              │
                                         Unit 2B (parallel, no dependency on 2A)
                                              │
                                              ↓
                                         Unit 3
```

- Unit 1 must complete first (identifies what to fix)
- Unit 2A depends on Unit 1's findings (implements startup reorder)
- Unit 2B is independent of 2A (stale PID cleanup is orthogonal)
- Unit 3 depends on both 2A and 2B (verifies the full fix end-to-end)

## Decisions and Rationale

| Decision | Rationale |
|----------|-----------|
| Move watcher after IPC bind (not into background task) | The existing code in `run_with_shutdown` already demonstrates "bind first" for workspace hydration. Extending this pattern to the watcher keeps the architecture consistent. |
| Timeout guard (5s) on watcher init | Defensive — even if the current bug is fixed, future watcher changes shouldn't be able to regress startup time. 5s is generous for a `ReadDirectoryChangesW` registration. |
| Clean up stale PID on startup | Stale PID files from abnormally terminated daemons cause confusing diagnostics. The lockfile handles stale locks, but PID cleanup is a separate concern. |
| Keep Unit 3 as verification (not automation) | The primary goal is unblocking dog-fooding. Automated end-to-end MCP tests are valuable but out of scope for this release unit. |

## Risks and Caveats

| Risk | Impact | Mitigation |
|------|--------|------------|
| Root cause is NOT the file watcher | Unit 1 findings invalidate Unit 2A approach | Unit 1 is explicitly diagnostic — it will identify the actual blocker before Unit 2A proceeds |
| `start_watcher` blocks tokio runtime thread | Entire async runtime stalls during watcher init | Use `spawn_blocking` to move synchronous watcher init off the tokio thread pool |
| Reordering startup breaks existing behavior | Daemon fails to start or loses file events | Existing integration tests cover daemon lifecycle; run full test suite after refactor |
| Watcher timeout masks a real initialization failure | Degraded mode silently loses file-change tracking | Log warning at INFO level; daemon already handles graceful degradation |
| `.engram/run/` stale state causes confusion | Developer wastes time debugging old binary | Unit 2B cleanup addresses stale PID; runtime file cleanup is manual |

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | No unsafe code; all errors via `Result<T, EngramError>` |
| II. Test-First Development | Unit 2A requires failing test before implementation |
| III. Workspace Isolation | No path traversal; all ops within workspace root |
| IV. CLI Containment | No files created outside cwd |
| V. Structured Observability | Diagnostic traces added in Unit 1 |
| VII. Destructive Approval | No destructive operations |
| Quality Gates | fmt → clippy → test required before merge |

## Plan Hardening Signals

- [ ] Public API, schema, or contract change — **NO**: Internal daemon startup
  refactoring; MCP tool interface unchanged
- [ ] Security, auth, permission, or compliance-sensitive behavior — **NO**: No
  auth or permission changes
- [ ] Migration, backfill, destructive data/config action — **NO**: No data
  migration; stale PID cleanup is safe (dead process only)
- [ ] External integration, operator checkpoint — **NO**: Purely internal
  daemon lifecycle change
- [ ] High runtime, rollout, or rollback risk — **NO**: Changes are local to
  daemon startup; if they regress, the daemon simply doesn't start (same as
  current broken state)

**Requires plan hardening: no**

## Runtime Verification and Closure

### Changed runtime surface

The daemon startup sequence is the affected runtime surface. After Unit 2:

- **Runtime verification**: Confirm the daemon reaches Ready state within 5s
  by running the shim and observing successful tool call responses.
- **Rollback trigger**: If the daemon cannot reach Ready after the refactor,
  revert the startup reordering commit and investigate further.
- **Monitoring**: Daemon logs "IPC listener bound" and "workspace hydration
  complete" with timestamps — compare startup time before/after.
- **Validation window**: 1 development session of active dog-fooding.
- **Owner**: Developer (operator) verifies during normal usage.

### Closure artifact

After Unit 3, produce a brief closure note in `docs/closure/` confirming:
- Daemon startup time
- Tool call latency
- Any remaining degraded-mode conditions
