---
title: "Engram Server Reliability & Dog-Fooding"
description: "Deliberation on making the local release build (target/release/engram.exe) usable for dog-fooding — covering daemon startup hang fix and end-to-end connectivity verification"
topic: "Fix engram MCP server connectivity for local dog-fooding (stash 9B4996E5 + queue 025-F)"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - ".backlogit/queue/025-F.md"
tags:
  - "daemon"
  - "reliability"
  - "dog-fooding"
  - "mcp"
  - "ipc"
  - "windows"
---

## Problem Frame

The local release build (`target/release/engram.exe`) is not working correctly
when Copilot attempts to connect via the MCP stdio protocol. The `.vscode/mcp.json`
configuration points to `target/release/engram.exe` with the `shim` subcommand,
but tool calls fail because the daemon never reaches Ready state.

### Symptoms observed

1. Shim reports: "Daemon failed to reach Ready state within 2000ms"
2. When the daemon is launched directly, it hangs after "idle TTL configured"
   (step 4 of daemon::run) and never reaches "IPC listener bound" (step 8)
3. No error output — the daemon stays alive but silent, suggesting a blocking
   operation rather than a crash
4. The daemon's PID file shows a dead process (PID 23324), indicating previous
   daemon sessions also failed or were killed

### Who cares

- The developer (operator) cannot use engram's own MCP tools when working on the
  engram codebase — self-hosted dog-fooding is completely blocked
- This blocks the broader 025-F milestone ("Releasable engram server with
  installer, instructions, and docs")

### Success criteria

1. `target/release/engram.exe shim` successfully spawns a daemon and serves tool calls
2. Copilot can invoke engram MCP tools from the local repo workspace
3. Daemon startup completes within 5 seconds on the developer machine
4. End-to-end flow verified: shim → daemon spawn → IPC connect → tool call → response

### Scope boundaries

**In scope:**
- Diagnosing and fixing the daemon startup hang
- Verifying the shim↔daemon IPC connectivity chain
- Ensuring the MCP config works for local development dog-fooding

**Out of scope (deferred to future 025-F work):**
- Cross-platform installer packaging
- User-facing documentation and onboarding
- Release distribution pipeline
- Performance optimization of daemon startup beyond "doesn't hang"

## Research Findings

### Architecture summary

The engram binary operates in two modes:
- **Shim mode** (default / `engram shim`): MCP stdio server that forwards tool
  calls to the daemon via IPC (named pipe on Windows)
- **Daemon mode** (`engram daemon --workspace <path>`): Long-lived background
  process that manages workspace state, code graph, and file watching

### Daemon startup sequence (daemon/mod.rs)

1. Canonicalize workspace path → `\\?\D:\Source\GitHub\agent-engram`
2. Acquire daemon lockfile ✓ (logged successfully)
3. Load plugin config ✓ (logged successfully)
4. Configure idle TTL ✓ (logged successfully)
5. Spawn Ctrl-C signal handler (tokio::spawn — non-blocking)
6. Start file watcher (`notify_debouncer_full`, `RecursiveMode::Recursive`) ← **SUSPECT**
7. Spawn workspace-moved detector (tokio::spawn — non-blocking)
8. Call `ipc_server::run_with_shutdown` which binds the named pipe ← **NEVER REACHED**

### Named pipe endpoint

Windows uses `\\.\pipe\engram-{workspace_key}` where `workspace_key` is the
UUID from `.engram/.workspace-id` (currently `cb582681-ca5d-4c6d-ad2c-3bf1003d9956`).

### File watcher hypothesis

The daemon hangs between step 4 (idle TTL) and step 8 (IPC bind). The primary
suspect is `start_watcher` at step 6:
- Uses `notify_debouncer_full::new_debouncer` + `debouncer.watch(&root, RecursiveMode::Recursive)`
- On Windows, `notify` uses `ReadDirectoryChangesW` which registers directory
  change notifications
- The workspace path is UNC-prefixed (`\\?\D:\...`) from `std::fs::canonicalize`
- Even with RUST_LOG=debug, no watcher error or success log appears

### Alternative hypotheses

1. **Blocking I/O in release mode**: Different optimization levels could change
   timing behavior of async operations
2. **Stale .engram/run/ state**: Old `engram.pid`, `engram.lock`, and `engram.exe`
   files from March 23 may interfere with startup
3. **Canonicalized path with UNC prefix**: `\\?\` prefix from `canonicalize()` may
   cause compatibility issues with `notify` or `interprocess` crates on Windows
4. **tokio runtime initialization timing**: In release mode, the single-threaded
   vs multi-threaded runtime configuration may matter for synchronous blocking calls

### Prior art (compound library)

- `docs/compound/bugs/stale-engram-citation-2026-04-29.md` — documents
  file-watcher ingestion lag but not startup hangs
- No compound entry exists for daemon startup failures on Windows

### Stale state in .engram/run/

| File | Content | Concern |
|------|---------|---------|
| `engram.pid` | PID 23324 (dead) | Stale — no cleanup on abnormal exit |
| `engram.lock` | fd-lock file | May prevent re-acquisition if corrupted |
| `engram.exe` | 81MB, March 23 build | Old binary copy, not the current release |

## Options Evaluated

### Option A: Targeted Diagnostic & Fix

Instrument the daemon startup with additional logging between steps 4-8 to
pinpoint the exact blocking call. Fix the identified blocker (likely file
watcher initialization or path normalization issue).

- **Pros**: Minimal scope, directly addresses the bug
- **Cons**: Requires iterative debugging; root cause not yet confirmed
- **Effort**: Low-Medium (2-4 hours)
- **Fit**: High — directly targets the blocking issue

### Option B: Defensive Startup with Timeout Guard

Wrap the file watcher initialization (and any other potentially-blocking
pre-IPC steps) in a tokio timeout. If initialization exceeds a threshold
(e.g., 5 seconds), continue in degraded mode and bind the IPC listener
immediately. The daemon can still serve tools while the watcher initializes
asynchronously.

- **Pros**: Makes daemon startup resilient regardless of root cause;
  guarantees IPC availability within bounded time
- **Cons**: May mask underlying bugs; adds complexity to startup flow
- **Effort**: Medium (2-3 hours)
- **Fit**: High — guarantees the "daemon reaches Ready" contract

### Option C: Combined Diagnostic + Defensive Design

First: add diagnostic instrumentation to identify the exact blocker.
Then: refactor daemon startup to move all potentially-blocking pre-IPC
initialization into background tasks (file watcher, code graph hydration
already does this). Ensure the IPC listener binds as early as possible in
the startup sequence.

- **Pros**: Finds and fixes root cause AND makes startup architecturally
  resilient; prevents future regression from similar blocking operations
- **Cons**: Larger scope; may require moving code between daemon/mod.rs steps
- **Effort**: Medium (4-6 hours = 2-3 tasks)
- **Fit**: Highest — permanent fix with architectural improvement

## Trade-off Comparison

| Criterion | Option A: Targeted Fix | Option B: Timeout Guard | Option C: Combined |
|---|---|---|---|
| Root cause identified | Yes | No (masked) | Yes |
| Startup resilience | Only for this bug | Yes (general) | Yes (general) |
| Regression risk | Medium | Low | Low |
| Scope creep risk | Low | Low | Medium |
| Dog-fooding unblocked | Yes | Yes | Yes |
| Effort | Low-Medium | Medium | Medium |
| Architectural benefit | None | Some | High |

## Decision

**Chosen: Option C — Combined Diagnostic + Defensive Design**

Rationale:
1. Dog-fooding is the primary development workflow for this project — daemon
   reliability is a first-class concern, not a nice-to-have
2. The IPC server already uses "bind first, hydrate later" for workspace
   initialization (see run_with_shutdown lines 398-405) but the daemon::run
   function has blocking operations BEFORE calling run_with_shutdown
3. Moving file watcher startup into a background task (or after IPC bind)
   follows the same pattern already used for workspace hydration
4. A timeout guard on the watcher ensures future changes don't regress

The work decomposes naturally into 2-3 tasks within the 2-hour rule:
- T1: Diagnostic instrumentation + identify exact blocker (investigation)
- T2: Refactor daemon startup to eliminate blocking before IPC bind
- T3: End-to-end verification of dog-fooding connectivity

## Rejected Alternatives

**Option A alone**: Finding and fixing only the current blocker without
architectural improvement would leave the daemon vulnerable to the same
class of bug from any future blocking operation added before IPC bind.

**Option B alone**: Masking the root cause prevents future learning and
makes the system harder to debug. Timeout guards are appropriate as a
defensive layer but not as the primary fix strategy.

## Unresolved Questions

1. **Exact blocking operation**: Is it `start_watcher`, `canonicalize` with
   UNC paths, or something else? Task T1 will answer this.
2. **Stale .engram/run/ cleanup**: Should the daemon clean up stale PID/lock
   files from dead processes on startup? (Partially handled by existing
   lockfile acquisition logic, but `engram.exe` copy and dead PID file persist.)
3. **ENGRAM_READY_TIMEOUT_MS default**: The 30s default may be too long for
   developer experience; consider reducing to 10s once startup is reliable.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| File watcher is the wrong suspect | Wasted T1 effort | Instrumentation will conclusively identify the blocker |
| Refactoring startup order introduces new bugs | Daemon fails to start | Existing test suite covers daemon lifecycle |
| Moving watcher to background loses early events | Missed file changes during startup | Acceptable — workspace hydration already handles initial state |
| Stale state interference | Daemon can't start even after fix | Add stale-state cleanup as part of T2 |
