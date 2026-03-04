# Implementation Plan: 004-refactor-engram-server-as-plugin

**Branch**: `004-refactor-engram-server-as-plugin` | **Date**: 2026-03-04 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/004-refactor-engram-server-as-plugin/spec.md`

## Summary

Refactor the engram MCP server from a centralized HTTP/SSE architecture to a decentralized per-workspace daemon model. The single `engram` binary gains subcommands: a lightweight **shim** (stdio MCP proxy) that MCP clients invoke, and a persistent **daemon** (background IPC server) that manages workspace state, file watching, and tool execution. Communication between shim and daemon uses cross-platform local IPC (`interprocess` crate) rather than network ports, eliminating port collisions and enabling 20+ concurrent isolated workspaces. The daemon auto-starts on first tool call and self-terminates after a configurable idle timeout (4h default).

## Technical Context

**Language/Version**: Rust 2024 edition, `rust-version = "1.85"`, stable toolchain  
**Primary Dependencies**: rmcp 1.1 (MCP stdio transport), interprocess 2.4 (cross-platform IPC), notify 9 + notify-debouncer-full 0.7 (file watching), fd-lock 4 (PID locking), SurrealDB 2 embedded (persistence), clap 4 (CLI), tokio 1 (async runtime)  
**Storage**: SurrealDB 2 embedded (surrealkv), per-workspace namespace via SHA-256 hash (unchanged from current)  
**Testing**: `cargo test` — contract, integration, unit, property-based (proptest). TDD required per constitution.  
**Target Platform**: Windows, macOS, Linux (cross-platform from day one)  
**Project Type**: Single Rust binary with subcommands  
**Performance Goals**: <2s cold start, <50ms read latency, <10ms write latency, <2s file change reflection  
**Constraints**: <50MB idle memory per workspace, `#![forbid(unsafe_code)]`, single binary  
**Scale/Scope**: 20+ concurrent workspaces, 100k+ file workspaces, 4h idle TTL default

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Safety First | PASS | `#![forbid(unsafe_code)]` maintained. `fd-lock` and `interprocess` are safe Rust APIs. No `unwrap()`/`expect()` in library code. |
| II. Async Concurrency Model | PASS | Tokio runtime, `interprocess` async IPC, `notify` Tokio integration. No blocking on async threads. |
| III. Test-First Development | PASS | TDD enforced. Contract tests for shim/daemon IPC protocol, integration tests for lifecycle, unit tests for lock logic. |
| IV. MCP Protocol Compliance | AMENDMENT NEEDED | Spec references `mcp-sdk 0.0.3` but it's unused in source. Replacing with `rmcp 1.1` (official SDK). Constitution amendment required. |
| V. Workspace Isolation | PASS | Strengthened — IPC per workspace eliminates shared network port entirely. |
| VI. Single-Binary Simplicity | PASS | Single `engram` binary with clap subcommands (shim, daemon, install, etc.). |
| VII. Git-Friendly Persistence | PASS | Unchanged — dehydration/hydration lifecycle preserved. |
| VIII. Observability | PASS | Daemon emits structured tracing to `.engram/logs/`. |
| IX. Error Handling | PASS | Typed errors, crash recovery via rehydration. |
| X. Simplicity & YAGNI | PASS | File watcher is thin event source (triggers existing pipelines, per clarification Q1). |

## Project Structure

### Documentation (this feature)

```text
specs/004-refactor-engram-server-as-plugin/
├── plan.md              # This file
├── research.md          # Phase 0 output — dependency research
├── data-model.md        # Phase 1 output — entity model
├── quickstart.md        # Phase 1 output — developer quickstart
├── contracts/           # Phase 1 output — IPC protocol contract
│   ├── ipc-protocol.md  # JSON-RPC IPC message format
│   └── mcp-tools.md     # MCP tool registry (unchanged)
├── SCENARIOS.md         # Phase 2 output (/speckit.behavior)
└── tasks.md             # Phase 3 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── lib.rs                   # Crate root (unchanged attributes)
├── bin/
│   └── engram.rs            # Binary entrypoint — clap subcommands: shim, daemon, install, etc.
├── config/
│   └── mod.rs               # Config struct — extended with daemon-specific settings
├── shim/
│   ├── mod.rs               # Shim module re-exports
│   ├── transport.rs         # rmcp StdioTransport setup, MCP ServerHandler impl
│   ├── ipc_client.rs        # Connect to daemon via interprocess LocalSocketStream
│   └── lifecycle.rs         # Daemon health check, spawn, wait-for-ready
├── daemon/
│   ├── mod.rs               # Daemon module re-exports
│   ├── ipc_server.rs        # interprocess LocalSocketListener, accept + dispatch
│   ├── watcher.rs           # notify file watcher setup, exclusion patterns
│   ├── debounce.rs          # notify-debouncer-full integration, event routing
│   ├── ttl.rs               # Idle timeout timer, activity tracking, shutdown trigger
│   └── lockfile.rs          # fd-lock PID file management, stale detection
├── installer/
│   ├── mod.rs               # Install/update/reinstall/uninstall logic
│   └── templates.rs         # .vscode/mcp.json, .gitignore templates
├── db/                      # Unchanged
├── errors/                  # Extended with new error variants
├── models/                  # Unchanged
├── server/                  # Retained for legacy HTTP/SSE (optional, may be removed)
├── services/                # Unchanged — reused by daemon
└── tools/                   # Unchanged — reused by daemon

tests/
├── contract/
│   ├── ipc_protocol_test.rs # IPC JSON-RPC message format validation
│   ├── shim_lifecycle_test.rs # Cold start, warm start, concurrent spawn
│   └── (existing tests)     # Unchanged
├── integration/
│   ├── daemon_lifecycle_test.rs # Start, idle timeout, crash recovery
│   ├── file_watcher_test.rs # Change detection, debounce, exclusion
│   ├── multi_workspace_test.rs # Isolation, concurrent workspaces
│   └── (existing tests)     # Unchanged
└── unit/
    ├── lockfile_test.rs     # PID lock acquire/release/stale detection
    ├── ttl_test.rs          # Timer reset, expiry, shutdown
    └── (existing tests)     # Unchanged
```

**Structure Decision**: Single project layout (Option 1) extended with three new top-level modules (`shim/`, `daemon/`, `installer/`). Existing modules (`db/`, `errors/`, `models/`, `services/`, `tools/`) are unchanged and reused by the daemon. The `server/` module (HTTP/SSE) is retained but may be removed in a future cleanup if the IPC transport fully replaces it.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Constitution II (MCP Protocol Fidelity) mandates `mcp-sdk 0.0.3` and SSE transport; we use `rmcp 1.1` and stdio/IPC | `mcp-sdk` was never used in source; `rmcp` is the official SDK with stdio transport support. SSE is replaced by IPC (more suitable for per-workspace daemon) | Keeping `mcp-sdk` would mean hand-rolling stdio transport and missing protocol compliance features. **Requires constitution MINOR amendment.** |
| Constitution IV (Workspace Isolation) mandates `127.0.0.1` TCP binding; we use local IPC instead | IPC is strictly more restrictive than localhost TCP — no network exposure at all. Eliminates port collisions (core motivator for this feature) | Keeping TCP binding would require port allocation, conflict resolution, and defeat purpose of refactoring. **Requires constitution MINOR amendment.** |
| Constitution V (Structured Observability) mandates stderr logging; daemon is a background process with no terminal | Background daemons cannot log to stderr (no terminal attached). File-based logging via `tracing-appender` is the standard approach | Stderr-only would lose all diagnostic output from the daemon. **Requires constitution MINOR amendment.** |
| Constitution Performance Standards specify <200ms cold start; spec uses <2s | New architecture requires subprocess spawning + IPC handshake, inherently higher latency than in-process startup | Sub-200ms is physically impossible with process spawn. **Requires constitution amendment to add "daemon cold start" as separate metric.** |
| `notify` v9 is RC (9.0.0-rc.2) | v9 matches our `rust-version = 1.85`; v8 is on `1.72`. RC is well-tested by community | Using v8 would require older API patterns and lacks Tokio feature integration |

**⚠️ PREREQUISITE**: Constitution amendments for Principles II, IV, V, and Performance Standards MUST be drafted and ratified before implementation begins. This is tracked as the first task in Phase 1.

## Architecture Overview

```text
┌──────────────────────────────────────────────────────────────────┐
│  MCP Client (Copilot CLI, VS Code, Cursor, Claude Code)         │
│  Invokes: engram [shim] via stdio                                │
└──────────────────┬───────────────────────────────────────────────┘
                   │ JSON-RPC over stdin/stdout
                   ▼
┌──────────────────────────────────────────────────────────────────┐
│  Shim (engram shim)                                              │
│  • rmcp StdioTransport — handles MCP protocol                   │
│  • Checks daemon health via IPC socket/pipe                     │
│  • If daemon not running: acquire PID lock, spawn daemon         │
│  • Forwards tool calls to daemon via IPC                         │
│  • Returns response to MCP client, exits                         │
└──────────────────┬───────────────────────────────────────────────┘
                   │ JSON-RPC over local IPC
                   │ (UDS on Unix, Named Pipe on Windows)
                   ▼
┌──────────────────────────────────────────────────────────────────┐
│  Daemon (engram daemon --workspace <path>)                       │
│  • interprocess LocalSocketListener — accepts IPC connections    │
│  • Dispatches tool calls via existing tools::dispatch()          │
│  • File watcher (notify) → debounce → trigger existing pipelines│
│  • TTL timer — resets on activity, shutdown on expiry            │
│  • fd-lock PID file — single instance enforcement                │
│  • SurrealDB embedded — per-workspace namespace (unchanged)      │
│  • Hydration on start, dehydration on flush/shutdown             │
└──────────────────────────────────────────────────────────────────┘
```

## Phased Implementation

### Phase 1: Foundation — CLI Subcommands & IPC Transport

**Scope**: Restructure the binary entrypoint to support subcommands. Implement the IPC transport layer. No behavioral changes to existing functionality.

**Work items**:
- Extend `bin/engram.rs` with clap subcommands: `shim`, `daemon`, `install` (stub), `update` (stub), `uninstall` (stub)
- Create `shim/` module: IPC client connecting via `interprocess` `LocalSocketStream`
- Create `daemon/ipc_server.rs`: `LocalSocketListener` accepting connections, dispatching to `tools::dispatch()`
- Create `daemon/lockfile.rs`: `fd-lock` PID file management
- Create `shim/lifecycle.rs`: daemon health check, spawn, exponential backoff wait
- Add dependencies: `interprocess`, `fd-lock`
- Contract tests: IPC JSON-RPC message format, shim→daemon round-trip

### Phase 2: MCP stdio Transport

**Scope**: Replace HTTP/SSE with stdio transport via `rmcp`. Shim becomes a conformant MCP server.

**Work items**:
- Add `rmcp` dependency, remove `mcp-sdk`
- Create `shim/transport.rs`: `rmcp` `StdioTransport` + `ServerHandler` implementation
- Implement tool forwarding: `call_tool` → IPC → daemon → response
- Implement `tools/list` with compiled-in tool registry
- Integration tests: MCP protocol compliance, tool call round-trip via stdio

### Phase 3: File System Watcher

**Scope**: Add real-time file watching to the daemon.

**Work items**:
- Add `notify`, `notify-debouncer-full` dependencies
- Create `daemon/watcher.rs`: file watcher setup, exclusion patterns (`.engram/`, `.git/`, `node_modules/`, `target/`)
- Create `daemon/debounce.rs`: debouncer integration, event routing to existing pipelines
- Configurable watch depth, exclusion patterns, debounce timing
- Integration tests: change detection, debounce behavior, exclusion filtering

### Phase 4: TTL Lifecycle Management

**Scope**: Implement idle timeout and graceful shutdown.

**Work items**:
- Create `daemon/ttl.rs`: activity timestamp tracking, periodic check, shutdown trigger
- Wire TTL reset into: IPC request handler, file watcher event handler
- Graceful shutdown sequence: flush state → close IPC → clean runtime artifacts → exit
- Crash recovery: stale lock detection, stale socket cleanup
- Integration tests: timeout expiry, activity reset, clean/unclean shutdown recovery

### Phase 5: Plugin Installer

**Scope**: Implement install/update/reinstall/uninstall commands.

**Work items**:
- Create `installer/` module: directory creation, config generation, .gitignore updates
- `engram install`: create `.engram/` structure, generate `.vscode/mcp.json`, health check
- `engram update`: replace runtime, preserve data
- `engram reinstall`: clean runtime, rehydrate from `.engram/` files
- `engram uninstall`: remove plugin artifacts, optional data preservation
- Create `installer/templates.rs`: `.vscode/mcp.json` template, `.gitignore` entries
- Integration tests: install in clean workspace, update preserving data, uninstall cleanup

### Phase 6: Configuration & Polish

**Scope**: Configuration file support, cross-platform hardening, performance validation.

**Work items**:
- Configuration file parsing (`.engram/config.toml`)
- Configurable: idle timeout, debounce timing, watch patterns, exclusion patterns
- Cross-platform path handling (spaces, Unicode, symlinks)
- Permission setting on IPC artifacts (Unix `0o600`, Windows security descriptors)
- Performance validation: cold start <2s, read <50ms, write <10ms
- Large workspace testing: 100k+ files background indexing

## Out of Scope

The following are explicitly excluded from this feature and deferred to future specifications:

- **Event sourcing / state versioning** (backlog feature 005) — time-travel rollback of graph state
- **External tracker sync** (backlog feature 005) — Jira/Linear background synchronization
- **Epic/Collection groupings** (backlog feature 005) — hierarchical task groupings
- **OTLP/OpenTelemetry exports** (backlog feature 005) — daemon exports to APM tools
- **Hook-based workflow enforcement** (backlog feature 005) — blocking gates via shim intercept hooks
- **Sandboxed SurrealQL query interface** (backlog feature 005) — agent-facing query API
- **Git commit tracking in graph** (backlog) — change-to-commit graph representation
- **Removal of HTTP/SSE transport** — the `server/` module is retained for now; removal is a separate cleanup task
