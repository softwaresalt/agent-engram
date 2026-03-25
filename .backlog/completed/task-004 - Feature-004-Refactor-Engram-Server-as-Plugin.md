---
id: TASK-004
title: '004: Refactor Engram Server as Workspace-Local Plugin'
status: Done
type: feature
assignee: []
created_date: '2026-03-04'
labels:
  - feature
  - '004'
  - architecture
  - lifecycle
  - ipc
  - plugin
milestone: m-0
dependencies:
  - TASK-001
  - TASK-003
references:
  - specs/004-refactor-engram-server-as-plugin/spec.md
  - src/bin/engram.rs
  - src/config/mod.rs
  - src/server/router.rs
  - src/server/state.rs
  - src/services/hydration.rs
  - src/services/dehydration.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
# Feature Specification: Refactor Engram Server as Workspace-Local Plugin

**Feature Branch**: `004-refactor-engram-server-as-plugin`  
**Created**: 2026-03-04  
**Status**: Draft  
**Input**: User description: "Refactor engram server from centralized HTTP/SSE model to a decentralized per-workspace daemon architecture with stdio MCP shim, local IPC, background file watching, embedded database, and TTL-based lifecycle management"


## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST operate as a workspace-local service that starts automatically on the first MCP tool invocation and requires no manual process management by the developer.
- **FR-002**: The system MUST communicate with MCP clients through a standard protocol interface (stdio) so that any MCP-compatible client (GitHub Copilot CLI, Claude Code, Cursor, VS Code) can connect without client-specific adapters.
- **FR-003**: The system MUST use a local inter-process communication channel (not a network port) for internal communication between the client-facing interface and the persistent service, eliminating port collisions entirely.
- **FR-004**: Each workspace's memory service MUST operate in complete isolation — separate data storage, separate communication channels, separate processes — with no possibility of cross-workspace data access.
- **FR-005**: The system MUST persist all workspace state to human-readable, Git-mergeable files in the `.engram/` directory, consistent with the existing dehydration/hydration lifecycle.
- **FR-006**: The memory service MUST watch the workspace file system for changes (creates, modifications, deletions) and trigger re-processing through existing indexing pipelines (code graph, embedding service) within 2 seconds of the filesystem event, with configurable debounce timing. The file watcher itself is a thin event source — it does not implement its own parsing or indexing logic.
- **FR-007**: The memory service MUST automatically shut down after a configurable period of inactivity (default: 4 hours), flushing all pending state before exit and cleaning up runtime artifacts.
- **FR-008**: The system MUST enforce single-instance-per-workspace semantics through an exclusive process lock, preventing concurrent duplicate services from running in the same workspace.
- **FR-009**: The system MUST perform cold start (from no running service to responding to a tool call) within 2 seconds, including service startup, data rehydration, and response delivery. *Note: This relaxes the constitution's <200ms server cold start target because the new architecture requires subprocess spawning + IPC handshake, which has inherently higher latency than in-process startup. A constitution amendment is required to formalize this as a separate "daemon cold start" metric.*
- **FR-010**: The memory service MUST expose all existing MCP tools (`set_workspace`, `update_task`, `get_task_graph`, `query_memory`, `flush_state`, etc.) without behavioral changes — the refactoring must be transparent to agents.
- **FR-011**: The system MUST provide install, update, reinstall, and uninstall commands that manage the `.engram/` directory structure, runtime artifacts, and MCP client configuration.
- **FR-012**: The system MUST support a configuration file for customizable parameters including idle timeout duration, debounce timing, watched/excluded directory patterns, and file extension filters.
- **FR-013**: The memory service MUST recover gracefully from unclean shutdowns (crashes, SIGKILL) by detecting and cleaning stale runtime artifacts on next startup and rehydrating from persisted `.engram/` files.
- **FR-014**: The system MUST emit structured diagnostic logs to a file in `.engram/logs/` to enable debugging of the background service when issues occur during unattended operation.
- **FR-015**: The client-facing interface (shim) MUST be a lightweight, stateless proxy — holding no workspace state of its own and delegating all persistence and business logic to the daemon. The shim process persists for the duration of the MCP session (required by the stdio transport protocol) but maintains no state between tool calls. This ensures minimal resource usage and prevents the shim from becoming a point of state divergence.
- **FR-016**: The system MUST set restrictive permissions on runtime artifacts (communication channels, lock files) so that only the local user can access the workspace memory, protecting proprietary source code on shared machines.
- **FR-017**: The `.engram/` directory layout MUST be self-contained and partitioned into committed state and gitignored runtime artifacts. **Committed** (Git-safe, human-readable): `tasks.md`, `graph.surql`, `config.toml`, `.version`, `.lastflush`. **Runtime** (gitignored, binary/ephemeral): `run/` (PID lock, socket), `logs/` (structured log files), `db/` (SurrealDB data). The installer MUST generate `.engram/.gitignore` to exclude `run/`, `logs/`, and `db/` directories.
- **FR-018**: The system MUST handle workspace paths containing spaces, Unicode characters, and symlinks correctly across all supported operating systems (Windows, macOS, Linux).
- **FR-019**: Initial indexing of a large workspace (100,000+ files) MUST NOT block tool call responses; indexing proceeds in the background with progressive result availability.

### Key Entities

- **Workspace Plugin**: The self-contained unit installed in a workspace's `.engram/` directory, encompassing runtime executable(s), configuration, persistent data, and logs. Tied to exactly one workspace.
- **Memory Service**: The long-running background process responsible for file watching, data indexing, query serving, and state persistence. One instance per workspace, lifecycle managed automatically.
- **Client Interface**: The lightweight, ephemeral process that bridges MCP clients (via stdio) to the memory service (via local IPC). Starts and stops with each tool invocation.
- **Communication Channel**: The local IPC mechanism (not a network port) used for client-to-service communication. Scoped to the workspace directory, eliminating cross-workspace interference.
- **Configuration**: User-editable settings file in `.engram/` controlling service behavior (timeouts, watched directories, debounce timing). Optional — sensible defaults apply when absent.
- **Runtime Artifacts**: Ephemeral files (lock files, communication channel endpoints, PID files) created while the service is running and cleaned up on shutdown.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Developers can add engram memory to any workspace with a single install command and have it operational within 30 seconds, with no configuration required.
- **SC-002**: AI assistants receive responses from the memory service within 50ms for read operations and 10ms for write operations after the service is running.
- **SC-003**: Cold start (first tool call in a dormant workspace) completes in under 2 seconds, including service startup, data loading, and response delivery.
- **SC-004**: 20 or more workspaces can run concurrent memory services on the same machine with zero port conflicts, zero cross-workspace data leakage, and less than 50MB idle memory per service.
- **SC-005**: The idle timeout mechanism reduces resource consumption to zero for inactive workspaces — no processes, no open file handles, no memory usage — within 60 seconds of the timeout expiring.
- **SC-006**: File system changes in the workspace are reflected in query results within 2 seconds of the change event, with debouncing preventing redundant processing of rapid consecutive saves.
- **SC-007**: Unclean shutdown recovery (e.g., after a system crash) completes with zero data loss by rehydrating from persisted `.engram/` files, with recovery taking less than 5 seconds.
- **SC-008**: All existing MCP tools continue to function identically after the refactoring — verified by the existing contract test suite passing without modification. **Known behavioral delta**: Workspace binding changes from per-SSE-connection to per-daemon. Two MCP clients that previously had independent `set_workspace` bindings now share daemon state; a daemon bound to workspace A will reject `set_workspace` with a different path B. This is a stricter isolation guarantee but a semantic change from the current model.

## Assumptions

- The single-binary principle from the constitution will be maintained by using subcommands or execution modes within the same `engram` binary, rather than producing separate binaries for the client interface and memory service.
- The existing MCP tool APIs and behaviors remain unchanged; this refactoring modifies only the transport and process architecture, not the tool semantics.
- The `.engram/` directory will be added to `.gitignore` by convention for runtime artifacts (binaries, database files, logs, sockets), while human-authored files (configuration, behavioral instructions) may optionally be committed.
- The existing hydration/dehydration lifecycle (`services/hydration.rs`, `services/dehydration.rs`) will be reused as-is for data persistence. No changes to the data format are required.
- Cross-platform support (Windows, macOS, Linux) is required from the start, as developers use all three operating systems. Platform-specific IPC mechanisms will be abstracted behind a common interface.
- The background file watching capability is additive — existing explicit sync operations (`flush_state`) remain available and functional alongside real-time watching.

## Clarifications

### Session 2026-03-04

- Q: When the file watcher detects a change, what level of processing should it trigger? → A: Trigger existing pipelines (Option B). The watcher acts as a thin event source feeding into existing code graph and embedding infrastructure from spec 003, not a new independent processing engine.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Single install command, 30 seconds to operational, no configuration required (SC-001)
- [x] #2 Read operations under 50ms, write operations under 10ms after service starts (SC-002)
- [x] #3 Cold start including startup, loading, and response under 2 seconds (SC-003)
- [x] #4 20+ concurrent workspaces with zero port conflicts, zero cross-workspace leakage, under 50MB idle memory per service (SC-004)
- [x] #5 Idle timeout reduces consumption to zero within 60s (SC-005)
- [x] #6 File changes reflected in queries within 2 seconds with debouncing (SC-006)
- [x] #7 Unclean shutdown recovery with zero data loss via rehydration, under 5 second recovery time (SC-007)
- [x] #8 All existing MCP tools function identically; contract test suite passes unchanged (SC-008)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
### Requirements

# Specification Quality Checklist: Refactor Engram Server as Workspace-Local Plugin

**Purpose**: Validate specification completeness and quality before proceeding to planning  
**Created**: 2026-03-04  
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- The spec references IPC and stdio as transport concepts, which are architecture-level abstractions rather than implementation details. These are necessary to convey the nature of the isolation guarantee.
- The single-binary assumption (using subcommands rather than separate binaries) is documented explicitly to reconcile with the constitution's Single-Binary Simplicity principle.
- FR-003 describes "local inter-process communication channel (not a network port)" which is intentionally abstract — the specific mechanism (UDS, Named Pipe, etc.) is deferred to the plan.
- All 19 functional requirements map to at least one acceptance scenario across the 6 user stories.
- 8 edge cases identified covering path handling, race conditions, corruption, disk space, read-only FS, unclean shutdown, and large workspace scaling.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Plan

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

### Task Breakdown

# Tasks: Refactor Engram Server as Workspace-Local Plugin

**Input**: Design documents from `/specs/004-refactor-engram-server-as-plugin/`
**Prerequisites**: plan.md (required), spec.md (required), SCENARIOS.md, data-model.md, contracts/

**Tests**: Included per constitution Principle III (Test-First Development, NON-NEGOTIABLE). Test tasks reference SCENARIOS.md as the authoritative source.

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

### Phase Mapping

| Tasks Phase | Plan Phase | Description |
|-------------|------------|-------------|
| Phase 1 | Phase 1 | Setup (dependencies, stubs) |
| Phase 1.5 | — | Prerequisites (test migration, test harness) |
| Phase 2 | Phase 1 | Foundational (IPC transport, lockfile) |
| Phase 3 | Phase 2 | US1+US2: Zero-Config + Isolation (MVP) |
| Phase 4 | Phase 3 | US4: File Watching |
| Phase 5 | Phase 4 | US3: Lifecycle Management |
| Phase 6 | Phase 5 | US5: Plugin Installer |
| Phase 7 | Phase 6 | US6: Configuration |
| Phase 8 | Phase 6 | Polish & Cross-Cutting |

### Terminology Mapping

| Spec Term | Implementation Term |
|-----------|--------------------|
| Memory Service | Daemon (`src/daemon/`) |
| Client Interface | Shim (`src/shim/`) |
| Communication Channel | IPC (Unix Domain Socket / Windows Named Pipe) |
| Workspace Plugin | `.engram/` directory + daemon + shim |
| Runtime Artifacts | `run/` dir (PID lock, socket), `logs/` dir |
| Configuration | `.engram/config.toml` → `PluginConfig` struct |

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single project layout: `src/`, `tests/` at repository root
- New modules: `src/shim/`, `src/daemon/`, `src/installer/`
- Tests: `tests/contract/`, `tests/integration/`, `tests/unit/`

---

## Phase 1: Setup

**Purpose**: Add dependencies, restructure binary entrypoint, create module stubs

- [X] T001 Add new dependencies to Cargo.toml: `interprocess = { version = "2", features = ["tokio"] }`, `fd-lock = "4"`, `rmcp = { version = "1.1", features = ["server", "transport-io"] }`, `notify = "9"`, `notify-debouncer-full = "0.7"`
- [X] T002 Remove unused `mcp-sdk = "0.0.3"` dependency from Cargo.toml (replaced by rmcp)
- [X] T003 Restructure src/bin/engram.rs with clap subcommands: `shim` (default), `daemon`, `install`, `update`, `reinstall`, `uninstall` per plan.md architecture
- [X] T004 [P] Create module stubs: src/shim/mod.rs (re-exports), src/shim/transport.rs, src/shim/ipc_client.rs, src/shim/lifecycle.rs
- [X] T005 [P] Create module stubs: src/daemon/mod.rs (re-exports), src/daemon/ipc_server.rs, src/daemon/watcher.rs, src/daemon/debounce.rs, src/daemon/ttl.rs, src/daemon/lockfile.rs
- [X] T006 [P] Create module stubs: src/installer/mod.rs, src/installer/templates.rs
- [X] T007 Add new error variants to src/errors/mod.rs: `IpcConnection`, `DaemonSpawn`, `LockAcquisition`, `WatcherInit`, `ConfigParse`, `InstallError` per data-model.md
- [X] T008 [P] Add error code constants to src/errors/codes.rs: 8xxx range (IPC/daemon) and 9xxx range (installer) per data-model.md
- [X] T009 Verify `cargo check` passes with new dependencies and module stubs

**Checkpoint**: Project compiles with all new dependencies and module structure.

---

## Phase 1.5: Prerequisites

**Purpose**: Constitution amendments and test infrastructure required before feature implementation.

- [X] T088 [US1] Update existing contract and integration test files in tests/ to remove mcp-sdk imports — test setup may reference mcp-sdk types removed in T002; adapt to direct JSON-RPC construction or rmcp types
- [X] T089 [US1] Build process-based test harness for spawning daemon processes in tests/ — helper that starts `engram daemon`, waits for IPC ready, provides cleanup on drop; required by T020-T025 (shim lifecycle tests)

---

## Phase 2: Foundational (IPC Transport & Lockfile)

**Purpose**: Core IPC transport and process locking that MUST be complete before any user story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests (write first, verify they fail)

- [X] T010 [P] Contract test for IPC JSON-RPC request/response format in tests/contract/ipc_protocol_test.rs — validate request serialization (S014-S016), missing fields (S017-S020), parse errors (S025) per SCENARIOS.md
- [X] T011 [P] Unit test for lockfile acquire/release/stale detection in tests/unit/lockfile_test.rs — validate lock acquisition (S027), stale lock detection (S029), cleanup on release (S032), read-only directory error (S030) per SCENARIOS.md
- [X] T012 [P] Unit test for IpcRequest/IpcResponse/IpcError serialization round-trips in tests/unit/proptest_models.rs — extend existing proptest coverage to new IPC models

### Implementation

- [X] T013 [P] Implement IpcRequest, IpcResponse, IpcError structs with serde in src/daemon/protocol.rs per data-model.md — include JSON-RPC 2.0 validation (jsonrpc field, id echoing). Note: IPC types are transport-layer, not domain models.
- [X] T014 [P] Implement DaemonState and DaemonStatus enum in src/daemon/mod.rs per data-model.md — Starting, Ready, ShuttingDown variants with serde(rename_all = "snake_case")
- [X] T015 Implement daemon lockfile management in src/daemon/lockfile.rs — fd-lock PID file acquire, release, stale detection (process liveness check), cleanup; covers S027-S033
- [X] T016 Implement daemon IPC server (listener + accept loop) in src/daemon/ipc_server.rs — interprocess LocalSocketListener, newline-delimited JSON-RPC framing, dispatch to tools::dispatch(); covers S014-S026
- [X] T017 Implement IPC endpoint naming in src/daemon/ipc_server.rs — Unix: `.engram/run/engram.sock`, Windows: `\\.\pipe\engram-{hash_prefix_16}` per contracts/ipc-protocol.md
- [X] T018 Wire IPC server into daemon startup sequence in src/daemon/mod.rs — hydrate → bind IPC → transition to Ready; covers S034-S036 state transitions
- [X] T019 Verify `cargo test` passes for all Phase 2 tests

**Checkpoint**: Foundation ready — IPC transport functional, lockfile enforced. User story implementation can begin.

---

## Phase 3: User Story 1 + User Story 2 — Zero-Config Workspace Memory & Workspace Isolation (Priority: P1) 🎯 MVP

**Goal**: MCP clients invoke tools via stdio, shim auto-starts daemon, workspace isolation via per-workspace IPC channels. This is the core value proposition.

**Independent Test**: Install plugin in fresh workspace, invoke `set_workspace` from an MCP client via stdio, verify daemon auto-starts and returns valid response. Open two workspaces simultaneously and verify zero cross-contamination.

### Tests for US1 + US2 (write first, verify they fail)

- [X] T020 [P] [US1] Contract test for shim cold start in tests/contract/shim_lifecycle_test.rs — no daemon running, shim spawns daemon, forwards request, returns response (S001); covers S005 cold start <2s
- [X] T021 [P] [US1] Contract test for shim warm start in tests/contract/shim_lifecycle_test.rs — daemon already running, shim connects and forwards (S002)
- [X] T022 [P] [US1] Contract test for shim error forwarding in tests/contract/shim_lifecycle_test.rs — daemon returns error, shim forwards faithfully (S004, S008)
- [X] T023 [P] [US1] Integration test for shim malformed input in tests/integration/shim_error_test.rs — invalid JSON (S006), empty stdin (S007), daemon timeout (S009), daemon crash (S010)
- [X] T024 [P] [US2] Integration test for multi-workspace isolation in tests/integration/multi_workspace_test.rs — two workspaces with separate data, verify no leakage (S088-S089); covers S091 symlink resolution
- [X] T025 [P] [US2] Integration test for concurrent workspace scaling in tests/integration/multi_workspace_test.rs — 20 workspaces running concurrently (S090 boundary test)

### Implementation for US1 + US2

- [X] T026 [US1] Implement shim IPC client in src/shim/ipc_client.rs — connect to daemon via interprocess LocalSocketStream, send JSON-RPC request, read response with timeout; covers S003, S009-S010
- [X] T027 [US1] Implement shim lifecycle in src/shim/lifecycle.rs — daemon health check (_health IPC message), spawn via std::process::Command, exponential backoff wait for ready; covers S001, S012, S013
- [X] T028 [US1] Implement daemon spawn guard in src/shim/lifecycle.rs — acquire lock before spawn, detect existing daemon, connect to existing if running; covers S028-S029
- [X] T029 [US1] Implement rmcp StdioTransport + ServerHandler in src/shim/transport.rs — rmcp ServerHandler trait impl, call_tool forwards to IPC client, tools/list returns compiled-in registry; covers S003, S015-S016
- [X] T030 [US1] Wire shim subcommand in src/bin/engram.rs — default subcommand invokes shim transport, connects stdio to daemon via IPC
- [X] T031 [US1] Wire daemon subcommand in src/bin/engram.rs — starts daemon process with --workspace arg, binds IPC, enters ready state; covers S034-S036
- [X] T032 [US2] Implement workspace-scoped IPC addressing in src/daemon/ipc_server.rs — each workspace gets unique IPC endpoint via SHA-256 hash prefix; covers S089, S091
- [X] T033 [US1] Implement _health IPC handler in src/daemon/ipc_server.rs — returns status, uptime, workspace, active connections per contracts/ipc-protocol.md (S021)
- [X] T034 [US1] Verify `cargo test` passes for all Phase 3 tests

**Checkpoint**: Core MVP functional — MCP clients can invoke tools via stdio, daemon auto-starts, workspaces fully isolated. User Stories 1 & 2 independently testable.

---

## Phase 4: User Story 4 — Real-Time File System Awareness (Priority: P2)

**Goal**: Daemon continuously monitors workspace for file changes and triggers existing indexing pipelines with configurable debounce. File watcher is a thin event source per spec clarification.

**Independent Test**: Start daemon, create/modify/delete a file, verify change reflected in queries within 2 seconds.

### Tests for US4 (write first, verify they fail)

- [X] T035 [P] [US4] Integration test for file change detection in tests/integration/file_watcher_test.rs — create, modify, delete events (S052-S054); verify WatcherEvent emission
- [X] T036 [P] [US4] Integration test for debounce behavior in tests/integration/file_watcher_test.rs — rapid saves collapse to single event (S055); verify timing with 500ms default
- [X] T037 [P] [US4] Integration test for exclusion patterns in tests/integration/file_watcher_test.rs — .engram/, .git/, node_modules/, target/ ignored (S056-S059); custom exclusions (S060)
- [X] T038 [P] [US4] Integration test for edge cases in tests/integration/file_watcher_test.rs — file rename (S062), large batch creates (S063), symlinks (S065), binary files (S066)

### Implementation for US4

- [X] T039 [P] [US4] Implement WatcherEvent and WatchEventKind models in src/models/ per data-model.md — Created, Modified, Deleted, Renamed variants
- [X] T040 [US4] Implement file watcher setup in src/daemon/watcher.rs — notify v9 RecommendedWatcher with exclusion pattern filtering; covers S052-S059, S064
- [X] T041 [US4] Implement debouncer integration in src/daemon/debounce.rs — notify-debouncer-full with configurable duration (default 500ms); covers S055, S063
- [X] T042 [US4] Wire debounced events to existing pipelines in src/daemon/debounce.rs — emit WatcherEvent, trigger code_graph and embedding services (thin event source per clarification); covers S052-S054, S062
- [X] T043 [US4] Handle watcher initialization failure gracefully in src/daemon/watcher.rs — log WatcherInit error, daemon continues without file watching (S064 degraded mode)
- [X] T044 [US4] Verify `cargo test` passes for all Phase 4 tests

**Checkpoint**: File watching operational — changes reflected in queries within 2 seconds. User Story 4 independently testable.

---

## Phase 5: User Story 3 — Automatic Lifecycle Management (Priority: P2)

**Goal**: Daemon self-manages its lifecycle with idle timeout, graceful shutdown, and crash recovery. Zero resource waste from idle workspaces.

**Independent Test**: Start daemon, wait for idle timeout, verify clean shutdown. Restart and verify data intact. Kill daemon, restart and verify recovery.

### Tests for US3 (write first, verify they fail)

- [X] T045 [P] [US3] Unit test for TTL timer in tests/unit/ttl_test.rs — expiry triggers shutdown (S045), activity resets timer (S046-S047), zero timeout = run forever (S049), rapid activity (S051)
- [X] T046 [P] [US3] Integration test for daemon lifecycle in tests/integration/daemon_lifecycle_test.rs — graceful shutdown flushes state (S037), shutdown during request (S038), restart after timeout (S050)
- [X] T047 [P] [US3] Integration test for crash recovery in tests/integration/daemon_lifecycle_test.rs — SIGKILL recovery (S039-S040), stale lock detection, data rehydration; covers S095-S096

### Implementation for US3

- [X] T048 [US3] Implement idle TTL timer in src/daemon/ttl.rs — activity timestamp tracking, periodic expiry check (S045), configurable duration; covers S048-S049
- [X] T049 [US3] Wire TTL reset into IPC request handler in src/daemon/ipc_server.rs — every tool call resets idle timer (S046)
- [X] T050 [US3] Wire TTL reset into file watcher event handler in src/daemon/watcher.rs — every file event resets idle timer (S047)
- [X] T051 [US3] Implement graceful shutdown sequence in src/daemon/mod.rs — transition to ShuttingDown, flush state, close IPC listener, remove lock file, remove socket, exit; covers S037
- [X] T052 [US3] Implement _shutdown IPC handler in src/daemon/ipc_server.rs — trigger graceful shutdown from shim command per contracts/ipc-protocol.md (S022)
- [X] T053 [US3] Implement crash recovery in src/daemon/lockfile.rs — detect stale lock (fd-lock not held), clean stale socket/pipe, allow fresh daemon start; covers S039-S040, S042
- [X] T054 [US3] Handle SIGTERM/SIGINT via tokio signal handler in src/daemon/mod.rs — trigger graceful shutdown on signal; covers S038
- [X] T055 [US3] Verify `cargo test` passes for all Phase 5 tests

**Checkpoint**: Lifecycle management complete — daemon auto-shuts down, recovers from crashes, zero resource waste. User Story 3 independently testable.

---

## Phase 6: User Story 5 — Plugin Installation & Management (Priority: P3)

**Goal**: Simple commands to install, update, reinstall, and uninstall the engram plugin in any workspace. Painless setup and corruption recovery.

**Independent Test**: Run `engram install` in clean workspace, verify `.engram/` created and MCP config generated. Run tool call. Uninstall and verify cleanup.

### Tests for US5 (write first, verify they fail)

- [X] T056 [P] [US5] Integration test for install command in tests/integration/installer_test.rs — clean workspace (S067), existing installation (S068), path with spaces (S076), Unicode path (S077), read-only FS (S078)
- [X] T057 [P] [US5] Integration test for update/reinstall/uninstall in tests/integration/installer_test.rs — update preserves data (S069), reinstall after corruption (S070), uninstall with keep-data (S071), full removal (S072)
- [X] T058 [P] [US5] Integration test for installer with running daemon in tests/integration/installer_test.rs — install while running (S073), uninstall stops daemon first (S074)

### Implementation for US5

- [X] T059 [US5] Implement install command in src/installer/mod.rs — create `.engram/` structure (tasks.md, .version, config stub, run/, logs/), generate MCP config, health check verification; covers S067, S075
- [X] T060 [P] [US5] Implement MCP config templates in src/installer/templates.rs — `.vscode/mcp.json` template with correct command path, `.gitignore` entries for runtime artifacts
- [X] T061 [US5] Implement update command in src/installer/mod.rs — replace runtime artifacts, preserve data files (tasks.md, graph.surql, config.toml); covers S069
- [X] T062 [US5] Implement reinstall command in src/installer/mod.rs — clean runtime, re-create structure, rehydrate from `.engram/` files; covers S070
- [X] T063 [US5] Implement uninstall command in src/installer/mod.rs — stop running daemon (_shutdown), remove artifacts, `--keep-data` flag for data preservation; covers S071-S074
- [X] T064 [US5] Detect existing installation in src/installer/mod.rs — check for `.engram/` directory, running daemon; covers S068, S073
- [X] T065 [US5] Wire installer subcommands in src/bin/engram.rs — install, update, reinstall, uninstall subcommands invoke installer module
- [X] T066 [US5] Verify `cargo test` passes for all Phase 6 tests

**Checkpoint**: Plugin installer complete — single-command setup and management. User Story 5 independently testable.

---

## Phase 7: User Story 6 — Configurable Behavior (Priority: P3)

**Goal**: Configuration file in `.engram/config.toml` customizes daemon behavior. Sensible defaults when absent.

**Independent Test**: Create config with custom idle timeout, start daemon, verify custom setting applied.

### Tests for US6 (write first, verify they fail)

- [X] T067 [P] [US6] Unit test for PluginConfig parsing in tests/unit/plugin_config_test.rs — no config file defaults (S079), valid config (S080-S081), unknown fields ignored (S082), malformed TOML fallback (S083), negative values (S084), boundary values (S085, S087)
- [X] T068 [P] [US6] Integration test for config-driven behavior in tests/integration/config_test.rs — custom exclusion patterns (S060-S061), custom timeout (S048), runtime config change no-op (S086)

### Implementation for US6

- [X] T069 [US6] Implement PluginConfig struct with TOML parsing in src/models/config.rs — all fields per data-model.md with Default impl for sensible fallbacks; covers S079-S081
- [X] T070 [US6] Implement config validation in src/models/config.rs — reject negative values, warn on unknown fields, clamp extreme values; covers S082-S085
- [X] T071 [US6] Implement config file loading in src/daemon/mod.rs — read `.engram/config.toml`, fall back to defaults on missing or invalid; covers S083
- [X] T072 [US6] Wire config into daemon subsystems — pass idle_timeout to TTL timer, debounce_ms to watcher, exclusion/watch patterns to watcher; covers S048, S060-S061, S087
- [X] T073 [US6] Verify `cargo test` passes for all Phase 7 tests

**Checkpoint**: Configuration complete — daemon adapts to project-specific settings. User Story 6 independently testable.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Security hardening, performance validation, cross-platform resilience, documentation

### Tests

- [X] T074 [P] Integration test for security scenarios in tests/integration/security_test.rs — Unix socket permissions (S097), path traversal rejection (S099), IPC message injection (S101), no secrets in `.engram/` (S102)
- [X] T075 [P] Integration test for error recovery in tests/integration/recovery_test.rs — disk full during flush (S093-S094), corrupted tasks.md recovery (S095)

### Implementation

- [X] T076 [P] Implement cross-platform path handling in src/daemon/mod.rs and src/installer/mod.rs — spaces, Unicode, symlink resolution; covers S031, S076-S077, S091, FR-018
- [X] T077 [P] Implement IPC artifact permissions in src/daemon/ipc_server.rs — Unix `0o600` socket permissions (S097), Windows default ACL (S098); covers FR-016
- [X] T078 [P] Implement structured logging to `.engram/logs/` in src/daemon/mod.rs — tracing-subscriber file appender, structured spans for all significant operations; covers S044, S103, FR-014
- [X] T079 Implement atomic flush failure handling in src/services/dehydration.rs — detect disk-full during temp file write, preserve existing `.engram/` files; covers S093-S094
- [X] T080 Implement workspace-moved detection in src/daemon/mod.rs — periodic check that workspace path still valid, shutdown if moved; covers S092
- [X] T081 [P] Implement IPC request size validation in src/daemon/ipc_server.rs — reject oversized requests to prevent memory exhaustion; covers S024, S101
- [X] T082 Performance validation: cold start under 2s benchmark in tests/integration/ — covers SC-003, S005
- [X] T083 Performance validation: read latency <50ms, write latency <10ms benchmark in tests/integration/ — covers SC-002
- [X] T084 [P] Large workspace test: 100k+ files background indexing does not block tool calls — covers FR-019, S063
- [X] T085 [P] Documentation updates: update README.md with new architecture, update docs/ with ADR for rmcp migration
- [X] T086 Run specs/004-refactor-engram-server-as-plugin/quickstart.md validation end-to-end
- [X] T087 Final `cargo clippy -- -D warnings` and `cargo test` full pass
- [X] T090 Verify all module stubs from Phase 1 (T004-T006) are replaced with real implementations — no placeholder code remaining per constitution "No dead code" rule
- [X] T091 Decide on server/ module (HTTP/SSE): either remove entirely or feature-gate behind `legacy-sse` flag — document decision in ADR
- [X] T092 [US4] Implement WatcherEvent→service adapter in src/daemon/debounce.rs — bridge WatcherEvent to existing code_graph and embedding service interfaces (existing services don't accept WatcherEvent directly)
- [X] T093 [US2] Implement Unix socket path overflow fallback in src/daemon/ipc_server.rs — detect path >108 bytes, fall back to /tmp/engram-{hash}.sock with 0o600 permissions; covers S119

**Checkpoint**: All user stories complete, hardened, and validated. Feature ready for review.

---

## Dependencies & Execution Order

### Phase Dependencies

```text
Phase 1: Setup ──────────────────────> Phase 2: Foundational (IPC + Lock)
                                              │
                                              ├──> Phase 3: US1+US2 (Zero-Config + Isolation) 🎯 MVP
                                              │          │
                                              │          ├──> Phase 4: US4 (File Watching)
                                              │          │          │
                                              │          │          └──> Phase 5: US3 (Lifecycle)
                                              │          │
                                              │          └──> Phase 6: US5 (Installer)
                                              │                     │
                                              │                     └──> Phase 7: US6 (Config)
                                              │
                                              └──> Phase 8: Polish (after all desired stories)
```

### User Story Dependencies

- **US1 + US2 (P1)**: Depends on Phase 2 (Foundational). No dependencies on other stories. **This is the MVP.**
- **US4 (P2)**: Depends on Phase 3 (US1+US2) for daemon infrastructure. Independent of US3, US5, US6.
- **US3 (P2)**: Depends on Phase 3 (US1+US2) for daemon infrastructure. Benefits from US4 (watcher events reset TTL) but can be tested independently.
- **US5 (P3)**: Depends on Phase 3 (US1+US2) for daemon lifecycle to test `install` health check. Independent of US3, US4, US6.
- **US6 (P3)**: Depends on Phase 5 (US3) for TTL config wiring and Phase 4 (US4) for watcher config wiring. Can implement config parsing independently.

### Within Each Phase

- Tests MUST be written and FAIL before implementation (constitution Principle III)
- Models before services
- Services before handlers
- Core implementation before integration wiring
- Phase complete before moving to next

### Parallel Opportunities

- Phase 1: T004, T005, T006 (module stubs) can run in parallel; T007, T008 (error variants) can run in parallel
- Phase 2: T010, T011, T012 (tests) can run in parallel; T013, T014 (models) can run in parallel
- Phase 3: T020-T025 (tests) can run in parallel
- Phase 4: T035-T038 (tests) can run in parallel; T039 (model) parallel with test writing
- Phase 5: T045-T047 (tests) can run in parallel
- Phase 6: T056-T058 (tests) can run in parallel; T060 (templates) parallel with T059 (install)
- Phase 7: T067-T068 (tests) can run in parallel
- Phase 8: T074-T075 (tests) parallel; T076, T077, T078, T081, T085 all target different files

---

## Parallel Example: Phase 3 (MVP)

```bash
# Launch all tests for US1+US2 together:
Task T020: "Contract test for shim cold start in tests/contract/shim_lifecycle_test.rs"
Task T021: "Contract test for shim warm start in tests/contract/shim_lifecycle_test.rs"
Task T022: "Contract test for shim error forwarding in tests/contract/shim_lifecycle_test.rs"
Task T023: "Integration test for shim malformed input in tests/integration/shim_error_test.rs"
Task T024: "Integration test for multi-workspace isolation in tests/integration/multi_workspace_test.rs"
Task T025: "Integration test for concurrent workspace scaling in tests/integration/multi_workspace_test.rs"

# Then implement sequentially:
Task T026: IPC client (core transport)
Task T027: Shim lifecycle (spawn + health)
Task T028: Daemon spawn guard (lock-based)
Task T029: rmcp StdioTransport (MCP protocol)
Task T030-T031: Wire subcommands
Task T032-T033: Workspace addressing + health handler
```

---

## Implementation Strategy

### MVP First (Phase 1-3: US1 + US2 Only)

1. Complete Phase 1: Setup (dependencies, module stubs)
2. Complete Phase 2: Foundational (IPC transport, lockfile)
3. Complete Phase 3: US1 + US2 (shim, daemon, MCP stdio)
4. **STOP and VALIDATE**: MCP client can invoke tools via stdio, workspaces isolated
5. This is the minimum viable product — AI assistants gain workspace memory

### Incremental Delivery

1. Phase 1-3 → MVP: Zero-config workspace memory with isolation
2. + Phase 4 → Add: Real-time file watching
3. + Phase 5 → Add: Idle timeout, lifecycle management, crash recovery
4. + Phase 6 → Add: One-command install/update/uninstall
5. + Phase 7 → Add: Configurable behavior via TOML
6. + Phase 8 → Polish: Security, performance, documentation
7. Each phase adds value without breaking previous phases

### Task Summary

| Phase | Story | Tasks | Test Tasks | Impl Tasks |
|-------|-------|-------|------------|------------|
| 1 | Setup | 9 | 0 | 9 |
| 1.5 | Prerequisites | 2 | 0 | 2 |
| 2 | Foundational | 10 | 3 | 7 |
| 3 | US1+US2 (P1) | 15 | 6 | 9 |
| 4 | US4 (P2) | 10 | 4 | 6 |
| 5 | US3 (P2) | 11 | 3 | 8 |
| 6 | US5 (P3) | 11 | 3 | 8 |
| 7 | US6 (P3) | 7 | 2 | 5 |
| 8 | Polish | 18 | 2 | 16 |
| **Total** | | **93** | **23** | **70** |

---

## SCENARIOS.md Coverage Map

Every scenario in SCENARIOS.md is covered by at least one task:

| SCENARIOS.md Section | Scenario IDs | Primary Tasks |
|---|---|---|
| Shim Lifecycle | S001-S013 | T020-T034 |
| IPC Protocol | S014-S026 | T010, T013, T016-T017 |
| Daemon Lockfile | S027-S033 | T011, T015, T028, T053 |
| Daemon Lifecycle | S034-S044 | T018, T031, T046-T047, T051, T054, T078 |
| TTL Management | S045-S051 | T045, T048-T050 |
| File Watcher | S052-S066 | T035-T043 |
| Plugin Installer | S067-S078 | T056-T066 |
| Configuration | S079-S087 | T067-T072 |
| Workspace Isolation | S088-S092 | T024-T025, T032, T076, T080 |
| Error Recovery | S093-S096 | T047, T075, T079 |
| Security | S097-S103 | T074, T077-T078, T081 |
| MCP Tool Compatibility | S104-S108 | T020-T022, T029, T033 |

---

## Notes

- [P] tasks target different files with no dependencies — safe to parallelize
- [Story] labels map tasks to user stories for traceability
- Constitution Principle III (TDD): test tasks precede implementation in every phase
- Constitution Principle I (Safety): no `unsafe`, no `unwrap()`, Result/EngramError throughout
- Constitution Principle VI (Single Binary): all subcommands in single `engram` binary
- Total: 93 tasks, 23 test tasks, 70 implementation tasks
- Commit after each task or logical group per constitution commit discipline
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Research

# Research: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04  
**Branch**: `004-refactor-engram-server-as-plugin`

## R1: Cross-Platform IPC in Rust

**Decision**: Use `interprocess` crate v2.4 with `tokio` feature flag.

**Rationale**: `interprocess` provides a unified API for Unix Domain Sockets (Linux/macOS) and Named Pipes (Windows) behind common `LocalSocketStream`/`LocalSocketListener` types. It has native Tokio integration, active maintenance, and minimal transitive dependencies. The `LocalSocketStream` implements `tokio::io::AsyncRead + AsyncWrite`, enabling direct use with Tokio codecs for JSON-RPC message framing.

**Alternatives considered**:

| Crate | Why rejected |
|-------|-------------|
| `parity-tokio-ipc` 0.9.0 | Less actively maintained, narrower API, designed for Parity/Substrate ecosystem |
| `tipsy` 0.6.5 | Smaller user base, less battle-tested, thinner documentation |
| Tokio built-in `UnixStream` + `NamedPipeServer` | Requires manual platform-specific abstractions; Tokio's Named Pipe API is lower-level |
| Raw `std::os::unix::net::UnixStream` | Synchronous only; defeats Tokio async runtime |

**Key details**:
- Socket path: `.engram/run/engram.sock` (Unix), `\\.\pipe\engram-{sha256_hash_prefix}` (Windows)
- Security: Unix socket permissions `0o600`; Windows Named Pipes inherit user security descriptor
- Dependency: `interprocess = { version = "2.4", features = ["tokio"] }`

## R2: File System Watching in Rust

**Decision**: Use `notify` v9 with `tokio` feature and `notify-debouncer-full` v0.7 for debouncing.

**Rationale**: `notify` is the de facto standard for cross-platform file watching in Rust. v9 requires `rust-version = 1.85` (matches our toolchain). The companion `notify-debouncer-full` provides content-aware debouncing that tracks file IDs (inode on Unix, file index on Windows), correctly handling rename chains and rapid IDE auto-saves.

**Alternatives considered**:

| Alternative | Why rejected |
|------------|-------------|
| `inotify` crate | Linux-only |
| Facebook `watchman` | External C++ daemon; violates single-binary principle |
| `hotwatch` | Thin wrapper over `notify` with less debounce control |
| Custom polling | Wastes CPU; `notify` is strictly better |

**Key details**:
- Native backends: FSEvents (macOS), inotify (Linux), ReadDirectoryChangesW (Windows)
- Linux gotcha: inotify watch limit (`max_user_watches`, default 8192) may need graceful fallback for 100k+ file workspaces
- Dependencies: `notify = { version = "9", features = ["tokio"] }`, `notify-debouncer-full = "0.7"`
- Exclusion patterns: Use existing `ignore` crate for `.gitignore`-aware filtering

## R3: Process Management Patterns in Rust

**Decision**: Use `std::process::Command` with platform-specific flags for daemon spawning, and `fd-lock` v4 for cross-platform PID file locking.

**Rationale**: Composition of focused primitives without requiring `unsafe` code. `Command` with `Stdio::null()` + Windows `creation_flags(CREATE_NO_WINDOW)` achieves detached spawning. `fd-lock` provides advisory file locks using OS file descriptors — OS automatically releases on process death, enabling stale lock detection.

**Alternatives considered**:

| Crate | Why rejected |
|-------|-------------|
| `daemonize` 0.5.0 | Unix-only, uses `fork()` requiring `unsafe` |
| `fs2` 0.4.3 | Unmaintained since 2019 |
| `nix` for `setsid` | Requires `unsafe`; violates `forbid(unsafe_code)` |
| `command-group` | Adds dependency for narrow use case |

**Key details**:
- PID file protocol: shim acquires `fd-lock` write lock → stale = spawn daemon; held = connect to IPC
- Daemon holds write lock for lifetime; OS releases on death
- SIGHUP handling: daemon ignores via `tokio::signal::unix::signal(SignalKind::hangup())`
- Dependency: `fd-lock = "4"`

## R4: MCP stdio Transport

**Decision**: Replace unused `mcp-sdk = "0.0.3"` with `rmcp = { version = "1.1", features = ["server", "transport-io"] }` for stdio transport. Daemon keeps hand-rolled JSON-RPC dispatch over IPC.

**Rationale**: `mcp-sdk 0.0.3` is listed as a dependency but never imported in any source file. The entire MCP implementation is hand-rolled JSON-RPC over axum HTTP/SSE. `rmcp` is the official Rust MCP SDK (maintained by MCP specification authors) providing native stdio transport, ServerHandler trait, tool router, and protocol compliance (capabilities negotiation, error codes).

**Architecture**: Shim uses `rmcp` `StdioTransport` to handle MCP protocol, forwards parsed tool calls to daemon over IPC as JSON-RPC, daemon processes and returns results.

**Alternatives considered**:

| Approach | Why rejected |
|----------|-------------|
| rmcp on both sides | Over-engineered for v1; daemon's existing dispatch works |
| Raw JSON-RPC byte forwarding | Fragile; misses protocol-level features (capabilities, initialize, tools/list) |

**Key details**:
- Constitution impact: requires amendment to reference `rmcp 1.1` instead of `mcp-sdk 0.0.3`
- Tool listing: compile-in static tool list for v1 (fast, no IPC round-trip for `tools/list`)
- `rmcp` `transport-io` feature provides `StdioTransport` reading JSON-RPC from stdin/writing to stdout

## R5: Single Binary Architecture

**Decision**: Use clap subcommands: `engram shim` (default), `engram daemon`, `engram install`, `engram update`, `engram reinstall`, `engram uninstall`.

**Rationale**: Constitution mandates single binary. Clap subcommands are idiomatic Rust and already the project's CLI framework. No subcommand defaults to shim mode for `.mcp.json` compatibility.

**Alternatives considered**:

| Approach | Why rejected |
|----------|-------------|
| Two separate binaries | Violates constitution principle VI (single-binary simplicity) |
| Auto-detection via isatty() | Fragile; unreliable in piped/CI environments |
| Symlink-based dispatch (argv[0]) | Confusing, poor Windows support |
| Compile-time feature flags | Requires two compilations; not a runtime solution |

**Key details**:
- MCP config: `{ "command": "engram", "args": ["shim"], "cwd": "${workspaceFolder}" }`
- Daemon spawning: shim uses `std::env::current_exe()` to spawn itself with `daemon --workspace <path>`
- Install creates `.engram/` structure, generates `.vscode/mcp.json`, updates `.gitignore`

## Dependency Summary

| Purpose | Crate | Version | Features | Replaces |
|---------|-------|---------|----------|----------|
| Cross-platform IPC | `interprocess` | 2.4 | `tokio` | N/A (new) |
| File watching | `notify` | 9 | `tokio` | N/A (new) |
| File watch debouncing | `notify-debouncer-full` | 0.7 | (default) | N/A (new) |
| PID file locking | `fd-lock` | 4 | (default) | N/A (new) |
| MCP stdio transport | `rmcp` | 1.1 | `server`, `transport-io` | `mcp-sdk` 0.0.3 (unused) |

**Removed**: `mcp-sdk` 0.0.3 (never used in source).  
**Potentially removable post-refactor**: `axum`, `tower`, `tower-http` (if HTTP/SSE transport fully removed; deferred pending health endpoint decision).

### Data Model

# Data Model: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04  
**Branch**: `004-refactor-engram-server-as-plugin`

## Overview

This feature introduces new architectural entities for the shim/daemon model. Existing data models (Task, Spec, Context, DependencyType) are **unchanged** — the refactoring affects only process architecture and communication, not the domain model.

## New Entities

### DaemonState

Represents the current state of the workspace daemon process.

| Field | Type | Description |
|-------|------|-------------|
| `workspace_path` | `PathBuf` | Canonical absolute path to the workspace root |
| `workspace_hash` | `String` | SHA-256 hash of canonical workspace path (reuses `db::workspace::hash_workspace_path`) |
| `pid` | `u32` | Process ID of the running daemon |
| `ipc_address` | `String` | IPC endpoint address (socket path or pipe name) |
| `started_at` | `DateTime<Utc>` | Timestamp when daemon started |
| `last_activity` | `DateTime<Utc>` | Timestamp of most recent activity (tool call or file event) |
| `idle_timeout` | `Duration` | Configured idle timeout duration |
| `status` | `DaemonStatus` | Current lifecycle state |

### DaemonStatus (enum)

| Variant | Description |
|---------|-------------|
| `Starting` | Daemon is initializing (hydrating data, binding IPC) |
| `Ready` | Daemon is accepting connections and processing events |
| `ShuttingDown` | Daemon is flushing state and cleaning up before exit |

### IpcRequest

JSON-RPC request message sent from shim to daemon over IPC.

| Field | Type | Description |
|-------|------|-------------|
| `jsonrpc` | `String` | Always `"2.0"` |
| `id` | `Value` | Request ID (number or string, echoed in response) |
| `method` | `String` | MCP tool name (e.g., `"set_workspace"`, `"update_task"`) |
| `params` | `Option<Value>` | Tool parameters (JSON object or null) |

### IpcResponse

JSON-RPC response message sent from daemon to shim over IPC.

| Field | Type | Description |
|-------|------|-------------|
| `jsonrpc` | `String` | Always `"2.0"` |
| `id` | `Value` | Request ID matching the request |
| `result` | `Option<Value>` | Success response payload (mutually exclusive with `error`) |
| `error` | `Option<IpcError>` | Error response (mutually exclusive with `result`) |

### IpcError

| Field | Type | Description |
|-------|------|-------------|
| `code` | `i32` | JSON-RPC error code |
| `message` | `String` | Human-readable error description |
| `data` | `Option<Value>` | Additional error data |

### WatcherEvent

Represents a debounced file system change event.

| Field | Type | Description |
|-------|------|-------------|
| `path` | `PathBuf` | Relative path from workspace root (primary path for Created/Modified/Deleted) |
| `old_path` | `Option<PathBuf>` | Previous path for Renamed events (None for all other event kinds) |
| `kind` | `WatchEventKind` | Type of change |
| `timestamp` | `DateTime<Utc>` | When the debounced event was emitted |

### WatchEventKind (enum)

| Variant | Description |
|---------|-------------|
| `Created` | New file or directory created |
| `Modified` | File content changed |
| `Deleted` | File or directory removed |
| `Renamed` | File moved or renamed — `old_path` contains the previous location, `path` contains the new location |

### PluginConfig

User-configurable settings loaded from `.engram/config.toml`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `idle_timeout_minutes` | `u64` | `240` (4 hours) | Minutes of inactivity before daemon shuts down |
| `debounce_ms` | `u64` | `500` | Milliseconds to debounce file events |
| `watch_patterns` | `Vec<String>` | `["**/*"]` | Glob patterns for files to watch |
| `exclude_patterns` | `Vec<String>` | `[".engram/", ".git/", "node_modules/", "target/", ".env*"]` | Glob patterns for files to exclude from watching |
| `log_level` | `String` | `"info"` | Daemon log verbosity |
| `log_format` | `String` | `"pretty"` | Log output format (`pretty` or `json`) |

## Relationships

```text
MCP Client ──stdio──> Shim ──IPC──> Daemon ──> SurrealDB (existing)
                                       │
                                       ├──> File Watcher ──> Existing Pipelines
                                       │      (code graph, embeddings)
                                       │
                                       └──> TTL Timer
```

## Unchanged Entities

The following existing entities are **not modified** by this feature:

- `Task` (models/task.rs) — all fields, status transitions, and serialization preserved
- `Spec` (models/spec.rs) — unchanged
- `Context` (models/context.rs) — unchanged
- `DependencyType` (models/graph.rs) — unchanged
- `CodeFile`, `Function`, `Class`, `Interface`, `Comment`, `CodeEdge` (models/) — unchanged
- `EngramError` (errors/mod.rs) — extended with new variants (see below)

## New Error Variants

| Variant | Code Range | Description |
|---------|-----------|-------------|
| `IpcConnection` | 8xxx | IPC socket/pipe errors (connect, send, receive) |
| `DaemonSpawn` | 8xxx | Daemon process spawning failures |
| `LockAcquisition` | 8xxx | PID file lock failures |
| `WatcherInit` | 8xxx | File watcher initialization failures |
| `ConfigParse` | 8xxx | Plugin configuration parsing errors |
| `InstallError` | 9xxx | Plugin install/update/uninstall failures |

*Note: 6xxx (config) and 7xxx (code graph) are already occupied in `src/errors/codes.rs`. These new variants use 8xxx and 9xxx to avoid collision.*

## State Transitions

### Daemon Lifecycle

```text
                     spawn
  [Not Running] ────────────> [Starting]
       ▲                          │
       │                          │ hydrate + bind IPC
       │                          ▼
       │  TTL expired        [Ready] <──── tool call / file event
       │  or SIGTERM             │          (resets TTL)
       │                         │
       └──── [ShuttingDown] <────┘
                  │
                  │ flush + cleanup
                  ▼
             [Not Running]
```

### Shim Request Flow

```text
  [Receive stdio] ──> [Check IPC] ──> [Connected?]
                                          │
                           Yes ◄──────────┤
                            │             No
                            │              │
                            │         [Acquire lock]
                            │              │
                            │         [Spawn daemon]
                            │              │
                            │         [Wait for ready]
                            │              │
                            ▼              ▼
                       [Forward via IPC]
                            │
                            ▼
                       [Return response via stdout]
                            │
                            ▼
                         [Exit]
```

### Analysis

# Adversarial Analysis Report: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04
**Artifacts Analyzed**: spec.md, plan.md, tasks.md, SCENARIOS.md, data-model.md, contracts/

## Adversarial Review Summary

| Reviewer | Model | Focus Area | Findings Count |
|----------|-------|------------|----------------|
| A | Claude Opus 4.6 | Logical Consistency | 30 |
| B | GPT-5.3 Codex | Technical Feasibility | 30 |
| C | Gemini 3.1 Pro Preview | Edge Cases and Security | 30 |

**Total pre-dedup**: 90 findings across 3 reviewers.

**Agreement patterns**: All three reviewers independently identified the constitution amendment requirement (mcp-sdk → rmcp, SSE → stdio/IPC) and the 127.0.0.1 binding conflict as CRITICAL. The WatcherEvent.Renamed data model defect was caught by 2/3 reviewers. Windows named pipe security concerns appeared in 2/3 reviewers. The stderr-vs-file logging conflict was unanimous.

**Conflicts resolved**: RC-09 flagged read latency (50ms) > write latency (10ms) as "inverted." Resolution: this matches the constitution's performance standards (query_memory latency < 50ms; update_task latency < 10ms) because semantic search is computationally heavier than simple DB writes. Finding dismissed as false positive.

## Unified Findings Table

| ID | Category | Severity | Location(s) | Summary | Recommendation | Consensus |
|----|----------|----------|-------------|---------|----------------|-----------|
| UF-01 | Constitution | **CRITICAL** | plan.md:Constitution Check, constitution.instructions.md:§II | Constitution mandates `mcp-sdk 0.0.3` and SSE transport. Spec replaces with `rmcp 1.1` and stdio/IPC. Plan acknowledges amendment needed but none exists. | Draft constitution amendment (MINOR bump minimum) before implementation. Add amendment task to Phase 1. | Unanimous |
| UF-02 | Constitution | **CRITICAL** | spec.md:FR-003, constitution.instructions.md:§IV | Constitution: "daemon MUST bind exclusively to 127.0.0.1." Spec: local IPC replaces TCP binding entirely. IPC is more restrictive but violates the literal text. | Amend Principle IV to generalize to "local-only communication." Add to Complexity Tracking. | Unanimous |
| UF-03 | Constitution | **CRITICAL** | spec.md:FR-017, constitution.instructions.md:§VIII | Constitution: ".engram/ files may be committed to Git" and "no binary files in .engram/." Spec places SurrealDB data, sockets, PID files, and logs in .engram/. These are binary/runtime, violating VIII. | Partition .engram/ into committed state (tasks.md, graph.surql, config.toml) and gitignored runtime (run/, db/, logs/). Amend VIII or relocate runtime artifacts. | Majority |
| UF-04 | Constitution | **CRITICAL** | spec.md:FR-014, constitution.instructions.md:§V | Constitution: "emit structured tracing spans to stderr." Spec: "structured diagnostic logs to .engram/logs/." Background daemon has no terminal for stderr. | Amend to permit file-based logging for background daemons. Add tracing-appender to dependencies. | Unanimous |
| UF-05 | Constitution | **CRITICAL** | spec.md:FR-009/SC-003, constitution:Performance Standards | Constitution: cold start < 200ms. Spec: cold start < 2s (10× slower). Process spawning overhead justifies the relaxation but no amendment exists. | Document justification (subprocess spawn + IPC handshake inherently slower than in-process startup). Amend performance targets or add "daemon cold start" as a separate metric from "server cold start." | Majority |
| UF-06 | Requirement Conflict | **HIGH** | spec.md:FR-015, contracts/ipc-protocol.md | FR-015: shim is "lightweight and ephemeral — starting instantly, forwarding, exiting." But MCP stdio transport (rmcp) requires a persistent session (initialize → tool calls → shutdown). The shim cannot be both ephemeral and an MCP server. | Rewrite FR-015: shim is a "stateless proxy" — persistent for the session but holds no state (all state in daemon). Remove "ephemeral" phrasing. | Majority |
| UF-07 | Data Model | **HIGH** | data-model.md:Error Variants, codes.rs | Data model assigns 6xxx (IPC/daemon) and 7xxx (installer) error ranges. But codes.rs already uses 6xxx (config errors: 6001-6003) and 7xxx (code graph: 7001-7007). Collision would corrupt error reporting. | Reassign: IPC/daemon → 8xxx, installer → 9xxx. Update data-model.md, tasks.md T007/T008. | Single (verified) |
| UF-08 | Data Model | **HIGH** | data-model.md:WatcherEvent, SCENARIOS.md:S062 | `WatcherEvent` has a single `path: PathBuf` but `Renamed` variant says "with old and new paths." Structurally impossible. S062 references both paths. | Add `old_path: Option<PathBuf>` to `WatcherEvent` or use enum-associated data: `Renamed { from: PathBuf, to: PathBuf }`. | Majority |
| UF-09 | Requirement Conflict | **HIGH** | contracts/mcp-tools.md:Behavioral Guarantees, spec.md:FR-010/SC-008 | Workspace binding changed from per-SSE-connection to per-daemon. Two MCP clients previously had independent bindings; now they share daemon state. SC-008 "100% backward compatibility" is overstated. | Document as known behavioral delta. Add scenario: daemon bound to workspace A, shim sends set_workspace with path B → error. Qualify SC-008. | Unanimous |
| UF-10 | Security | **HIGH** | contracts/ipc-protocol.md:Security, SCENARIOS.md:S098 | Windows named pipe "default ACL" claim is incorrect. Default ACL grants Everyone read access. Without explicit SECURITY_ATTRIBUTES, pipe is accessible to other users. | Specify explicit DACL restricting to current user SID. Update ipc-protocol.md. Add security test for non-owner rejection. | Majority |
| UF-11 | Coverage Gap | **HIGH** | SCENARIOS.md, spec.md:FR-019 | FR-019 requires 100k+ file background indexing. SCENARIOS.md S063 covers only 500 files and no scenario tests querying during indexing (progressive availability). | Add dedicated scenario: 100k+ file indexing in progress, tool call arrives, returns available results without blocking. | Majority |
| UF-12 | Coverage Gap | **HIGH** | SCENARIOS.md:S104-S108, tasks.md:Coverage Map | S104-S108 backward compatibility scenarios cover only 5/10 MCP tools. Missing: add_blocker, register_decision, check_status, query_memory, get_workspace_status. | Add backward compatibility scenarios for all 10 tools. | Majority |
| UF-13 | Implementation Gap | **HIGH** | tasks.md:T002, tests/contract/ | T002 removes mcp-sdk from Cargo.toml but no task updates existing contract/integration tests that import from mcp-sdk. Removing the dependency breaks compilation. | Add task: "Update existing test files to remove mcp-sdk imports and adapt to direct JSON-RPC construction." | Majority |
| UF-14 | Implementation Gap | **HIGH** | tasks.md:T020-T025 | Shim lifecycle tests require spawning real daemon processes. No test harness for process-based testing exists. Existing test infrastructure is in-memory only. | Add task before T020: "Build process-based test harness for spawning daemon, waiting for IPC ready, cleanup on drop." | Single (validated) |
| UF-15 | Naming Inconsistency | **MEDIUM** | plan.md:Phase 6 | Plan references ".engram/settings.toml or .engram/config.toml." All other artifacts use config.toml exclusively. | Standardize on config.toml. Fix plan.md. | Unanimous |
| UF-16 | Phase Mismatch | **MEDIUM** | plan.md (6 phases) vs tasks.md (8 phases) | Plan and tasks use different phase counts and boundaries, complicating cross-reference. | Add mapping table to tasks.md header. | Majority |
| UF-17 | Dependency Justification | **MEDIUM** | plan.md, research.md | 5 new crates lack individual requirement mapping per constitution Principle VI. | Add dependency-to-requirement mapping table in research.md. | Majority |
| UF-18 | Edge Case Gap | **MEDIUM** | SCENARIOS.md | No scenario for shim connecting to daemon in ShuttingDown state or concurrent flush. | Add scenarios for shutdown race conditions. | Single |
| UF-19 | Edge Case Gap | **MEDIUM** | SCENARIOS.md:S028 | Two-shim spawn race within 10ms window not explicitly tested. S028 covers detection but not timing. | Add explicit concurrent spawn scenario. | Single |
| UF-20 | Architecture | **MEDIUM** | tasks.md:T013 | IPC message types (IpcRequest/Response/Error) placed in src/models/ alongside domain entities. These are transport types, not domain models. | Move to src/shim/messages.rs or src/daemon/protocol.rs. Update tasks. | Majority |
| UF-21 | Acceptance Criteria | **MEDIUM** | spec.md:US5 | US5 acceptance scenarios summarized in one sentence; all other stories use full Given/When/Then. | Expand to structured format with measurable outcomes. | Single |
| UF-22 | Coverage Gap | **MEDIUM** | SCENARIOS.md, spec.md:SC-005 | SC-005 requires zero resources within 60s of timeout. No scenario validates the timing constraint. | Add boundary scenario for 60s cleanup window. | Majority |
| UF-23 | Dead Code | **MEDIUM** | plan.md:Project Structure, tasks.md | server/ module "may be removed" with no task or timeline. No task verifies stubs replaced. Constitution §6: "No dead code." | Add Phase 8 tasks: verify stubs replaced; decide on server/ module (remove or feature-gate). | Majority |
| UF-24 | Edge Case Gap | **MEDIUM** | SCENARIOS.md, contracts/ipc-protocol.md | Unix socket path max ~108 bytes. Deep workspace paths could overflow. No fallback specified. | Add scenario for UDS path overflow. Fallback: use /tmp/engram-{hash}.sock or shortened hash. | Single |
| UF-25 | Edge Case Gap | **MEDIUM** | SCENARIOS.md | No scenario for watcher events on paths outside workspace boundary (via symlinks resolving external). | Add scenario: symlinked dir points outside workspace, event filtered. | Majority |
| UF-26 | Terminology Drift | **MEDIUM** | spec.md vs plan.md vs tasks.md | Spec uses "Memory Service", "Client Interface", "Communication Channel." Plan/tasks use "daemon", "shim", "IPC." No formal mapping. | Add terminology mapping table to plan.md or spec.md. | Single |
| UF-27 | Missing Spec | **MEDIUM** | SCENARIOS.md, contracts/ipc-protocol.md | No daemon-side IPC read timeout. Client not sending \n causes daemon hang. | Add daemon-side read timeout (60s) to protocol contract. | Single |
| UF-28 | Security | **MEDIUM** | SCENARIOS.md, spec.md:FR-006 | PluginConfig watch_patterns default `**/*` may match .env files containing secrets. | Add `.env*` to default exclude_patterns. | Single |
| UF-29 | Ambiguity | **MEDIUM** | spec.md:FR-006 | "near-real-time" undefined. AC says "within 2 seconds." | Replace "near-real-time" with "within 2 seconds of the filesystem event." | Single |
| UF-30 | Implementation Gap | **MEDIUM** | tasks.md:T042, services/ | T042 "trigger code_graph and embedding services" but existing services don't accept WatcherEvent. No adapter task. | Add task for adapter layer: WatcherEvent → incremental service update call. | Single |
| UF-31 | Dependency Risk | **LOW** | plan.md:Complexity Tracking | notify v9 is RC. No fallback plan if it breaks before stable. | Pin exact RC version. Document fallback to v8 in research.md. | Unanimous |
| UF-32 | Log Management | **LOW** | spec.md:FR-014 | No log rotation or size limit. Daemon running days could produce unbounded logs. | Add max log size + rotation count to PluginConfig defaults (10MB, 3 rotations). | Single |
| UF-33 | Data Model | **LOW** | data-model.md:DaemonState.ipc_address | String type doesn't constrain or document platform-dependent format. | Either use enum or add format documentation. | Single |
| UF-34 | Coverage | **LOW** | SCENARIOS.md | Only 9% concurrent scenarios despite concurrency being primary motivator. | Add 5-8 additional concurrent scenarios. | Single |
| UF-35 | Specification | **LOW** | SCENARIOS.md:S026 vs ipc-protocol.md | S026 tolerates missing \n but protocol says newline-delimited. Inconsistent. | Decide: mandatory \n or tolerant. Update both. | Single |

## Remediation Log

| Finding ID | File | Change Description | Applied? |
|------------|------|--------------------|----------|
| UF-01 | plan.md | Added constitution amendment prerequisite task to Phase 1 | Yes |
| UF-02 | plan.md | Added Principle IV IPC deviation to Complexity Tracking | Yes |
| UF-03 | spec.md | Clarified .engram/ layout: committed state vs gitignored runtime | Yes |
| UF-04 | plan.md | Added Principle V logging deviation to Complexity Tracking | Yes |
| UF-05 | spec.md | Reworded cold start target with justification for 2s vs 200ms | Yes |
| UF-06 | spec.md | Rewrote FR-015: "stateless proxy" instead of "ephemeral" | Yes |
| UF-07 | data-model.md | Changed error code ranges: IPC→8xxx, installer→9xxx | Yes |
| UF-07 | tasks.md | Updated T007/T008 error code range references | Yes |
| UF-08 | data-model.md | Added old_path field to WatcherEvent for Renamed support | Yes |
| UF-09 | spec.md | Qualified SC-008 backward compat claim with known behavioral delta | Yes |
| UF-09 | contracts/mcp-tools.md | Added behavioral delta section for workspace binding change | Yes |
| UF-10 | contracts/ipc-protocol.md | Fixed Windows pipe security: explicit DACL, not default ACL | Yes |
| UF-11 | SCENARIOS.md | Added S109: 100k+ file indexing with concurrent tool call | Yes |
| UF-12 | SCENARIOS.md | Added S110-S114: backward compat for remaining 5 MCP tools | Yes |
| UF-13 | tasks.md | Added T088: migrate existing tests from mcp-sdk | Yes |
| UF-14 | tasks.md | Added T089: build process-based test harness | Yes |
| UF-15 | plan.md | Fixed: settings.toml → config.toml | Yes |

## Remaining Issues (Medium — Require Operator Approval)

| ID | Summary | Recommendation |
|----|---------|----------------|
| UF-16 | Phase numbering mismatch (plan 6 phases vs tasks 8 phases) | Add mapping table to tasks.md |
| UF-17 | Dependency justification table missing | Add dep→requirement mapping to research.md |
| UF-18 | No scenario for shim→daemon during ShuttingDown state | Add shutdown race scenario |
| UF-19 | Two-shim spawn race not explicitly tested | Add concurrent spawn scenario |
| UF-20 | IPC types in models/ should be in transport layer | Move to src/daemon/protocol.rs |
| UF-21 | US5 acceptance scenarios not in Given/When/Then | Expand to structured format |
| UF-22 | SC-005 60s cleanup window not tested | Add boundary scenario |
| UF-23 | server/ module dead code, no stub verification task | Add Phase 8 cleanup tasks |
| UF-24 | Unix socket path length overflow | Add UDS overflow scenario |
| UF-25 | Watcher events outside workspace via symlinks | Add external symlink scenario |
| UF-26 | Terminology drift (Memory Service vs daemon) | Add mapping table |
| UF-27 | No daemon-side IPC read timeout | Add timeout to protocol |
| UF-28 | PluginConfig watch patterns may match .env secrets | Add .env* to default excludes |
| UF-29 | "near-real-time" undefined in FR-006 | Replace with "within 2 seconds" |
| UF-30 | No WatcherEvent→service adapter task | Add adapter task |

## Remaining Issues (Low — Suggestions Only)

| ID | Summary |
|----|---------|
| UF-31 | notify v9 RC risk — pin version, document fallback |
| UF-32 | No log rotation/size limits specified |
| UF-33 | DaemonState.ipc_address String type is platform-ambiguous |
| UF-34 | Concurrent scenarios underrepresented (9%) |
| UF-35 | S026 newline handling inconsistent with protocol |

## Constitution Alignment Issues

| Principle | Violation | Resolution |
|-----------|-----------|------------|
| II. MCP Protocol Fidelity | mcp-sdk 0.0.3 replaced by rmcp 1.1; SSE replaced by stdio/IPC | **Needs formal amendment** — documented as Phase 0 prerequisite |
| IV. Workspace Isolation | "bind to 127.0.0.1" replaced by IPC (more restrictive) | **Needs formal amendment** — added to Complexity Tracking |
| V. Structured Observability | stderr logging impractical for background daemon | **Needs formal amendment** — added to Complexity Tracking |
| VIII. Git-Friendly Persistence | .engram/ now contains runtime artifacts (binary) | **Needs clarification** — partitioned into committed vs runtime directories |
| Performance: cold start <200ms | Spec uses 2s (subprocess spawn overhead) | **Needs formal amendment** — justification documented |

## Metrics

**Artifact metrics:**
- Total requirements: 19 (FR-001 through FR-019)
- Total tasks: 89 (87 original + 2 added)
- Total scenarios: 114 (98 original + 16 added)
- Task coverage: 100% (all FRs have tasks)
- Scenario coverage: 100% (all FRs have scenarios)
- Non-happy-path: 68% (exceeds 30% minimum)

**Finding metrics:**
- Ambiguity count: 2
- Cross-artifact inconsistency count: 8
- Critical issues found: 5
- Critical issues remediated: 5 (documented, amendments flagged)
- High issues found: 9
- High issues remediated: 9

**Adversarial metrics:**
- Total findings pre-dedup: 90 (30 per reviewer)
- Total findings post-synthesis: 35
- Findings per reviewer: A=30, B=30, C=30
- Agreement rate: 49% (17/35 with majority or unanimous)
- Conflict count: 1 (RC-09 latency inversion — dismissed as false positive)

## Next Actions

1. **Block implementation until constitution amendments are drafted** for Principles II (transport), IV (binding), V (logging), and Performance (cold start). These 5 CRITICAL findings are addressed in artifacts but require formal constitutional ratification.
2. **Review and approve/reject 15 MEDIUM findings** via operator review before proceeding to build.
3. **All critical and high findings have been remediated** in the spec artifacts. The specification is ready for implementation pending constitution amendments and medium finding disposition.

### Scenarios

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

### Quickstart

# Quickstart: 004-refactor-engram-server-as-plugin

## Prerequisites

- Rust stable toolchain, edition 2024, `rust-version = "1.85"`
- `cargo` available on PATH
- A workspace directory to install engram into

## Build

```bash
cargo build --release
```

The single `engram` binary is output to `target/release/engram`.

## Install in a Workspace

```bash
cd /path/to/your/workspace
engram install
```

This creates:
- `.engram/` directory structure (run/, logs/, db/)
- `.vscode/mcp.json` with the engram stdio server entry
- Updates `.gitignore` with `.engram/run/`, `.engram/logs/`, `.engram/db/`

## Verify Installation

```bash
engram shim <<< '{"jsonrpc":"2.0","id":1,"method":"get_daemon_status","params":null}'
```

Expected: the daemon starts automatically, responds with status JSON, and remains running for subsequent calls.

## How It Works

1. **MCP client** (VS Code, Copilot CLI, Cursor) invokes `engram` via stdio
2. **Shim** checks for a running daemon via the IPC socket
3. If no daemon running: acquires PID lock, spawns daemon, waits for readiness
4. **Shim** forwards the tool call to the daemon via IPC
5. **Daemon** processes the tool call, returns the result
6. **Shim** writes the response to stdout and exits

The daemon continues running in the background, watching files and serving subsequent tool calls. After the configured idle timeout (default: 4 hours), it shuts down gracefully.

## Configuration (Optional)

Create `.engram/config.toml` to customize behavior:

```toml
# Idle timeout in minutes (default: 240 = 4 hours)
idle_timeout_minutes = 60

# File event debounce in milliseconds (default: 500)
debounce_ms = 1000

# Additional patterns to exclude from file watching
exclude_patterns = [".engram/", ".git/", "node_modules/", "target/", "dist/"]

# Log level: trace, debug, info, warn, error (default: info)
log_level = "debug"
```

## Management Commands

```bash
engram install     # Install plugin in current workspace
engram update      # Update runtime, preserve data
engram reinstall   # Clean install, rehydrate from .engram/ files
engram uninstall   # Remove plugin (--keep-data to preserve stored state)
```

## MCP Client Configuration

### VS Code (`.vscode/mcp.json`)

```json
{
  "servers": {
    "engram": {
      "type": "stdio",
      "command": "engram",
      "args": ["shim"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

### GitHub Copilot CLI

The `.mcp.json` at workspace root:

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["shim"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

## Troubleshooting

### Daemon won't start

Check `.engram/logs/daemon.log` for errors. Common causes:
- Missing write permissions on `.engram/`
- Stale PID file (delete `.engram/run/daemon.pid` and retry)
- Port/socket conflict with another process

### Stale lock file after crash

If the daemon was killed ungracefully, the shim detects stale locks automatically on next invocation. If issues persist:

```bash
rm .engram/run/daemon.pid .engram/run/engram.sock
```

### Data corruption

```bash
engram reinstall
```

This preserves `.engram/tasks.md` and `.engram/graph.surql` while rebuilding the database from those files.

### Operator Review Log

# Operator Review Log: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04  
**Review Session**: Adversarial analysis findings from Stage 6  
**Total Findings Reviewed**: 35

## Summary

| Decision | Count |
|----------|-------|
| Auto-applied (critical/high) | 14 |
| Approved (medium) | 15 |
| Deferred | 0 |
| Rejected | 0 |
| Recorded (low suggestions) | 6 |

## Critical/High Findings (Auto-Applied)

| Finding ID | Severity | Consensus | Decision | Notes |
|------------|----------|-----------|----------|-------|
| UF-01 | CRITICAL | Unanimous | Applied | Constitution amendment prerequisite added to plan.md |
| UF-02 | CRITICAL | Unanimous | Applied | Principle IV IPC deviation documented in Complexity Tracking |
| UF-03 | CRITICAL | Majority | Applied | .engram/ layout partitioned (committed vs runtime) in FR-017 |
| UF-04 | CRITICAL | Unanimous | Applied | Principle V logging deviation documented in Complexity Tracking |
| UF-05 | CRITICAL | Majority | Applied | Cold start target justified in FR-009 |
| UF-06 | HIGH | Majority | Applied | FR-015 rewritten (ephemeral → stateless proxy) |
| UF-07 | HIGH | Single (verified) | Applied | Error codes reassigned: IPC→8xxx, installer→9xxx |
| UF-08 | HIGH | Majority | Applied | WatcherEvent.old_path added for Renamed |
| UF-09 | HIGH | Unanimous | Applied | SC-008 qualified; behavioral delta section in mcp-tools.md |
| UF-10 | HIGH | Majority | Applied | Windows pipe security upgraded to explicit DACL |
| UF-11 | HIGH | Majority | Applied | S109 added (100k+ file indexing with concurrent tool call) |
| UF-12 | HIGH | Majority | Applied | S110-S114 added (backward compat for 5 remaining tools) |
| UF-13 | HIGH | Majority | Applied | T088 added (migrate existing tests from mcp-sdk) |
| UF-14 | HIGH | Single (validated) | Applied | T089 added (process-based test harness) |

## Medium Findings (Operator Approved)

| Finding ID | Severity | Consensus | Operator Decision | Notes |
|------------|----------|-----------|-------------------|-------|
| UF-15 | MEDIUM | Unanimous | Approved | settings.toml → config.toml standardized |
| UF-16 | MEDIUM | Majority | Approved | Phase mapping table added to tasks.md |
| UF-17 | MEDIUM | Majority | Approved | Deferred to research.md update (dependency justification) |
| UF-18 | MEDIUM | Single | Approved | S115 added (shim→daemon during ShuttingDown) |
| UF-19 | MEDIUM | Single | Approved | S116 added (two-shim spawn race) |
| UF-20 | MEDIUM | Majority | Approved | T013 moved IPC types to src/daemon/protocol.rs |
| UF-21 | MEDIUM | Single | Approved | Deferred to build phase (US5 acceptance expansion) |
| UF-22 | MEDIUM | Majority | Approved | S117 added (60s cleanup boundary test) |
| UF-23 | MEDIUM | Majority | Approved | T090 + T091 added (dead code verification) |
| UF-24 | MEDIUM | Single | Approved | S119 added (UDS path overflow fallback) + T093 |
| UF-25 | MEDIUM | Majority | Approved | S118 added (external symlink filtering) |
| UF-26 | MEDIUM | Single | Approved | Terminology mapping table added to tasks.md |
| UF-27 | MEDIUM | Single | Approved | 60s daemon-side IPC read timeout added to contract |
| UF-28 | MEDIUM | Single | Approved | .env* added to default exclude_patterns |
| UF-29 | MEDIUM | Single | Approved | FR-006 "near-real-time" → "within 2 seconds" |

## Low Findings (Recorded as Suggestions)

| Finding ID | Summary | Notes |
|------------|---------|-------|
| UF-31 | notify v9 RC risk — pin version, document fallback | Tracked in Complexity Tracking |
| UF-32 | No log rotation/size limits specified | Consider adding to PluginConfig during build |
| UF-33 | DaemonState.ipc_address String type is platform-ambiguous | Consider enum during implementation |
| UF-34 | Concurrent scenarios underrepresented (9%) | Additional concurrent scenarios added (S116) |
| UF-35 | S026 newline handling inconsistent with protocol | Clarify during IPC implementation |

## Artifacts Modified

| File | Changes Applied |
|------|----------------|
| spec.md | FR-006 wording, FR-009 justification, FR-015 rewrite, FR-017 layout, SC-008 qualification |
| plan.md | Complexity Tracking expanded (4 entries), settings.toml→config.toml |
| data-model.md | Error code ranges (8xxx/9xxx), WatcherEvent.old_path, .env* exclude |
| tasks.md | Phase mapping, terminology mapping, T088-T093 added, T008/T013 updated |
| SCENARIOS.md | S109-S119 added (11 new scenarios), summary metrics updated |
| contracts/ipc-protocol.md | Windows pipe DACL security, daemon-side read timeout |
| contracts/mcp-tools.md | Behavioral delta section for workspace binding change |
| ANALYSIS.md | Full adversarial analysis report |

## Dismissed Finding

| Finding ID | Original Reviewer | Reason for Dismissal |
|------------|------------------|----------------------|
| RC-09 | Reviewer A | Read latency (50ms) > write latency (10ms) is correct per constitution: semantic search (query_memory) is computationally heavier than simple DB writes (update_task). Not inverted. |

### Contract: Ipc Protocol

# IPC Protocol Contract: Shim ↔ Daemon

**Version**: 1.0.0  
**Transport**: Local IPC (Unix Domain Socket / Windows Named Pipe)  
**Framing**: Newline-delimited JSON (each message is a single line terminated by `\n`)  
**Encoding**: UTF-8

## Connection Lifecycle

1. **Shim connects** to the daemon's IPC endpoint.
2. **Shim sends** one JSON-RPC 2.0 request (single line, terminated by `\n`).
3. **Daemon responds** with one JSON-RPC 2.0 response (single line, terminated by `\n`).
4. **Connection closes** — each tool call is a single request/response cycle.

The protocol is **stateless per connection**. The daemon maintains state internally (workspace binding, active sessions), but each IPC connection is independent.

## IPC Endpoint Naming

| Platform | Format | Example |
|----------|--------|---------|
| Unix (Linux/macOS) | File path | `.engram/run/engram.sock` |
| Windows | Named Pipe | `\\.\pipe\engram-{sha256_hash_prefix_16}` |

The workspace SHA-256 hash prefix (first 16 hex characters) ensures unique pipe names on Windows. On Unix, the socket file is relative to the workspace root.

## Request Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tool_name",
  "params": {
    "key": "value"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `jsonrpc` | string | yes | Must be `"2.0"` |
| `id` | number or string | yes | Request identifier, echoed in response |
| `method` | string | yes | MCP tool name (e.g., `"set_workspace"`, `"update_task"`) |
| `params` | object or null | no | Tool parameters |

## Response Format — Success

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "ok",
    "data": { ... }
  }
}
```

## Response Format — Error

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Workspace not set",
    "data": {
      "engram_code": 1001,
      "details": "Call set_workspace before using workspace-scoped tools"
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `error.code` | integer | JSON-RPC error code (standard or custom) |
| `error.message` | string | Human-readable error summary |
| `error.data.engram_code` | integer | Engram-specific error code (from `errors/codes.rs`) |
| `error.data.details` | string | Detailed error context |

## Standard JSON-RPC Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error (malformed JSON) |
| -32600 | Invalid request (missing required fields) |
| -32601 | Method not found (unknown tool name) |
| -32602 | Invalid params (parameter validation failure) |
| -32603 | Internal error (unexpected daemon failure) |

## Internal Protocol Messages

In addition to tool calls, the shim may send internal protocol messages:

### Health Check

```json
{
  "jsonrpc": "2.0",
  "id": "health",
  "method": "_health",
  "params": null
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "health",
  "result": {
    "status": "ready",
    "uptime_seconds": 3600,
    "workspace": "/path/to/workspace",
    "active_connections": 2
  }
}
```

### Graceful Shutdown

```json
{
  "jsonrpc": "2.0",
  "id": "shutdown",
  "method": "_shutdown",
  "params": null
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "shutdown",
  "result": {
    "status": "shutting_down",
    "flush_started": true
  }
}
```

## Timeout Behavior

- **Shim connection timeout**: 2 seconds (covers cold start)
- **Shim request timeout**: 60 seconds (matches existing `ENGRAM_REQUEST_TIMEOUT_MS` default)
- **Daemon idle timeout**: configurable (default 4 hours)
- **Daemon IPC read timeout**: 60 seconds — if the daemon does not receive a complete request (terminated by `\n`) within 60 seconds of accepting an IPC connection, it closes the connection and logs a warning. This prevents hung connections from clients that connect but never send data.

If the daemon does not respond within the request timeout, the shim returns a JSON-RPC timeout error to the MCP client.

## Security

- **Unix**: Socket file created with permissions `0o600` (owner read/write only)
- **Windows**: Named Pipe created with explicit `SECURITY_ATTRIBUTES` containing a DACL that grants access only to the creating user's SID. The default ACL is insufficient — it grants `Everyone` read access. The implementation MUST use `CreateNamedPipe` with a `SECURITY_DESCRIPTOR` that restricts to `GENERIC_READ | GENERIC_WRITE` for the current user SID and denies all other principals.
- **No authentication**: Trust model is OS-level user isolation (same as current localhost binding)

### Contract: Mcp Tools

# MCP Tools Contract: 004-refactor-engram-server-as-plugin

**Version**: 1.0.0 (unchanged from current)

## Overview

The MCP tool registry is **unchanged** by this refactoring. All tools, their parameters, return types, and error behaviors remain identical. The only change is the transport: tools are now invoked via stdio (shim) → IPC (daemon) instead of HTTP POST `/mcp`.

## Tool Registry

| Tool | Module | Parameters | Description |
|------|--------|------------|-------------|
| `set_workspace` | lifecycle | `{ workspace_path: string }` | Bind connection to a workspace, trigger hydration |
| `get_daemon_status` | lifecycle | none | Report uptime, connections, workspaces |
| `get_workspace_status` | lifecycle | none | Report task/context counts, flush state, staleness |
| `update_task` | write | `{ task_id: string, status?: string, description?: string }` | Change task status, creates context note |
| `add_blocker` | write | `{ task_id: string, blocker_id: string, reason?: string }` | Block a task with reason |
| `register_decision` | write | `{ title: string, content: string }` | Record architectural decision as context |
| `flush_state` | write | none | Serialize DB state to `.engram/` files |
| `get_task_graph` | read | `{ root_id?: string, depth?: number }` | Recursive dependency graph traversal |
| `check_status` | read | `{ work_item_ids: string[] }` | Batch work item status lookup |
| `query_memory` | read | `{ query: string, limit?: number }` | Semantic search (embedding-based) |

## Behavioral Guarantees

All existing behavioral contracts are preserved:

1. **Workspace binding**: `set_workspace` MUST be called before any workspace-scoped tool. Tools called without a bound workspace return error code `1001` (WORKSPACE_NOT_SET).
2. **Status transitions**: `update_task` validates transitions per the state machine (todo → in_progress → done, etc.). Invalid transitions return error code `3001`.
3. **Context notes**: Every `update_task` call creates a context note recording the transition (FR-015 from spec 001).
4. **Idempotency**: Write operations are idempotent where documented.
5. **Error codes**: All error codes from `errors/codes.rs` are unchanged.

### Known Behavioral Delta: Workspace Binding Semantics

The workspace binding model changes from **per-SSE-connection** to **per-daemon**:

| Aspect | Before (SSE) | After (Daemon) |
|--------|-------------|----------------|
| Binding scope | Each SSE connection has independent workspace binding | Daemon is bound to one workspace for its entire lifetime |
| Multiple clients | Two clients could bind to different workspaces on the same server | All clients connecting to a daemon share its workspace binding |
| `set_workspace` with different path | Allowed (each connection independent) | Returns error — daemon is already bound to a different workspace |
| Isolation guarantee | Logical (per-connection) | Physical (per-process + per-IPC-channel) |

**Impact**: Agents that previously relied on rebinding `set_workspace` to a different path within the same server session will now receive an error. This is a **stricter isolation guarantee** — each workspace has its own daemon process — but constitutes a semantic change that may affect multi-workspace tooling. The recommended migration is to let each workspace's shim auto-start its own daemon.

## Transport Change Only

| Aspect | Before (current) | After (refactored) |
|--------|-------------------|---------------------|
| Client → Server | HTTP POST `/mcp` | stdio → shim → IPC → daemon |
| Tool discovery | HTTP GET `/sse` (SSE event with tool list) | `tools/list` via MCP protocol (rmcp handles) |
| Connection model | Persistent SSE + POST requests | Per-invocation shim process |
| Workspace binding | Per-SSE-connection state | Per-daemon state (bound on first `set_workspace`) |
| Error format | JSON-RPC 2.0 over HTTP | JSON-RPC 2.0 over stdio (same schema) |

## Contract Test Compatibility

Existing contract tests in `tests/contract/` MUST continue to pass. The tests validate tool input/output schemas and error codes — these are transport-independent. Test setup may need adaptation to use the IPC transport instead of HTTP, but assertion logic remains identical.
<!-- SECTION:NOTES:END -->
