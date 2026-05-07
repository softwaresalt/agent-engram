---
title: "CLI Parity for MCP Tool Operations"
description: "Full CLI subcommand surface mirroring all 18 MCP tools with JSON-RPC 2.0 output"
topic: "Add CLI parity for all MCP tool operations"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-05-06-cli-parity-plan.md"
tags:
  - "cli"
  - "clap"
  - "json-rpc"
  - "preloading"
  - "agent-fallback"
stash_id: "D391F5AF"
---

## Problem Frame

**Problem**: When the MCP server receives its first tool call from Copilot, it may
need to index the workspace, which takes time and causes MCP server timeout. Agents
have no fallback when MCP transport fails mid-session.

**Who cares**: All agents using engram; operators running `start.ps1` to bootstrap
the workspace before launching Copilot.

**Constraints**:
- JSON-RPC 2.0 output format is non-negotiable — agents consume CLI output with
  zero code changes relative to MCP consumption
- Must route through the same dispatch logic — no duplicated business logic
- Must work for startup preloading (before Copilot launches) and agent fallback
  (when MCP transport fails)

**Success criteria**:
- Every MCP tool callable via CLI with identical response shape
- `start.ps1` can call `engram sync` / `engram index` and block until DB is populated
- Agents can exec `engram <cmd> --json` as a subprocess and parse the output
- `engram manifest` exposes the tools/list catalog for discovery without daemon connection

**Scope boundaries**:
- IN: All 18 default-feature MCP tools + manifest command
- IN: Global flags (--workspace, --id, --json, --format, --quiet)
- IN: Exit codes (0 = success, 1 = tool error, 2 = CLI error)
- OUT: Feature-gated tools (git-graph `query_changes`) — added when the feature lands
- OUT: Human-friendly TUI or interactive modes
- OUT: Batch/pipeline mode (single command per invocation)

## Research Findings

### Current Architecture

- **Binary**: `src/bin/engram.rs` uses clap `Parser` + `Subcommand` derive
- **Existing subcommands**: Shim (default), Daemon, Install, Update, Reinstall, Uninstall
- **IPC client**: `src/shim/ipc_client.rs` — connects to daemon via named pipe,
  sends `IpcRequest`, receives `IpcResponse` with configurable timeout
- **Lifecycle**: `src/shim/lifecycle.rs` — health-checks daemon, spawns it if not
  running, waits for ready with exponential backoff
- **Tools catalog**: `src/shim/tools_catalog.rs` — `all_tools()` returns 18 `Tool`
  definitions with name, description, and inputSchema
- **Dispatch**: `src/tools/mod.rs` — `dispatch()` routes tool name to handler function

### Key Integration Points

1. The shim already connects to daemon, ensures it's running, and sends tool calls.
   CLI subcommands can reuse the same flow (lifecycle ensure + IPC send).
2. `tools_catalog::all_tools()` is the single source of truth for the manifest.
3. The daemon's `IpcResponse` already contains JSON-RPC-shaped content that can be
   reformatted for CLI output.
4. The `IpcRequest` struct matches the shape CLI subcommands need to construct.

### Relevant Patterns

- Shim module already demonstrates the full lifecycle: spawn daemon → ensure ready →
  send request → parse response. CLI commands follow the same pattern with different
  output formatting.

## Options Evaluated

### Option A: CLI-to-Daemon via IPC (Reuse Shim Transport)

CLI subcommands construct an `IpcRequest`, use the shim's lifecycle module to ensure
the daemon is running, send the request via `ipc_client::send_request()`, and format
the `IpcResponse` as JSON-RPC 2.0 on stdout.

- **Pros**: Reuses all existing infrastructure; no code duplication; daemon manages
  state consistently; leverages existing spawn/ready logic; minimal new code surface
- **Cons**: Requires daemon to be running (but auto-spawn handles this); adds latency
  for daemon startup on first call
- **Effort**: Medium — new clap subcommands + thin IPC wrapper + output formatter
- **Fit**: Excellent — directly serves both stated purposes (preloading starts daemon
  and triggers indexing; fallback uses already-running daemon)

### Option B: CLI with In-Process Execution (No Daemon)

CLI initializes its own `SharedState`, opens a CozoDB connection, and calls
`dispatch()` directly without going through IPC.

- **Pros**: No daemon dependency; single-process; lower latency for individual calls
- **Cons**: CozoDB exclusive lock conflicts with running daemon; duplicates state
  initialization; doesn't share workspace state with daemon; must replicate daemon
  bootstrap logic; concurrent access hazard
- **Effort**: High — must factor out state initialization; handle lock contention
- **Fit**: Poor — CozoDB uses exclusive file locks, so running both daemon and CLI
  against the same database simultaneously would fail

### Option C: Hybrid (Try IPC, Fallback to In-Process)

Try daemon connection first; if unreachable and auto-spawn fails, create ephemeral
in-process context.

- **Pros**: Most resilient; works even if daemon can't start
- **Cons**: Two code paths; Option B's lock problem still applies for concurrent use;
  significantly more complex; harder to test; fallback path rarely exercised
- **Effort**: Very high — all of Option A + all of Option B + switching logic
- **Fit**: Over-engineered — auto-spawn covers the "daemon not running" case

## Trade-off Comparison

| Criterion | Option A (IPC) | Option B (In-Process) | Option C (Hybrid) |
|---|---|---|---|
| Complexity | Low | Medium–High | Very High |
| Code reuse | Excellent | Poor | Mixed |
| CozoDB conflicts | None | Critical | Partial |
| Daemon required | Yes (auto-spawned) | No | Preferred |
| Preloading support | ✓ (spawns + indexes) | ✓ (opens DB directly) | ✓ |
| Agent fallback | ✓ (daemon presumably running) | ✓ | ✓ |
| Maintenance burden | Low | High | Very High |

## Decision

**Chosen**: Option A — CLI-to-Daemon via IPC

**Rationale**:
1. The shim module already proves the pattern works end-to-end
2. CozoDB's exclusive file locking makes Option B fundamentally incompatible with
   concurrent daemon operation
3. Auto-spawn eliminates the "daemon not running" concern
4. For the preloading use case, `start.ps1` will: start daemon → call `engram sync` →
   wait for response. The auto-spawn in the CLI handles daemon lifecycle.
5. Minimal new code: new clap subcommands → translate args → IpcRequest → send → format output

**Rejected alternatives**:
- Option B: CozoDB lock conflicts make this unworkable when daemon is also running
- Option C: Over-engineered given auto-spawn handles the missing-daemon case

**Unresolved questions**: None — architecture is clear.

**Risks and mitigations**:
- Risk: Daemon startup takes too long for `start.ps1` → Mitigation: Existing ready-wait
  backoff with configurable timeout; can increase `ENGRAM_READY_TIMEOUT_MS`
- Risk: Manifest command requires daemon → Mitigation: `engram manifest` can be served
  from `tools_catalog::all_tools()` directly (compile-time catalog, no daemon needed)

## Implementation Architecture

```
src/bin/engram.rs
  └── Command enum (top-level flat variants)
        ├── Shim, Daemon, Install, ...    (existing internal commands)
        ├── Bind, Sync, Index, Search, …  (new CLI subcommands)
        └── Manifest                      (local catalog, no daemon needed)

Dispatch flow:
  new subcommand variant matched
    ├── manifest → tools_catalog::all_tools() → format as JSON-RPC
    └── all others → lifecycle::ensure_daemon() → ipc_client::send_request() → format response

New modules:
  src/cli/
    mod.rs        — CLI orchestration: dispatch subcommand to handler
    output.rs     — JSON-RPC 2.0 formatter + human-readable formatter
    runner.rs     — IPC call wrapper: ensure_daemon → build IpcRequest → send → return
```
