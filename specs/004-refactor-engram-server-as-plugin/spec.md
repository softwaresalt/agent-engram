# Feature Specification: Refactor Engram Server as Workspace-Local Plugin

**Feature Branch**: `004-refactor-engram-server-as-plugin`  
**Created**: 2026-03-04  
**Status**: Draft  
**Input**: User description: "Refactor engram server from centralized HTTP/SSE model to a decentralized per-workspace daemon architecture with stdio MCP shim, local IPC, background file watching, embedded database, and TTL-based lifecycle management"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Zero-Configuration Workspace Memory (Priority: P1)

As an AI coding assistant invoked in a workspace, I need persistent memory that is automatically available without manual setup, so I can retrieve and store context across sessions without the developer configuring ports, starting servers, or managing processes.

When the assistant issues an MCP tool call (e.g., `query_memory`, `update_task`), the system automatically starts a workspace-scoped memory service if one is not already running. The assistant never needs to know about the underlying process lifecycle — it simply calls tools and gets answers. The memory service is scoped exclusively to the current workspace, preventing cross-project context leakage.

**Why this priority**: This is the fundamental value proposition. Without automatic, zero-configuration workspace memory, the refactoring has no purpose. Every subsequent feature depends on this working seamlessly.

**Independent Test**: Can be fully tested by installing the plugin in a fresh workspace, invoking a memory tool from an MCP client, and verifying the response returns valid data. Delivers immediate value: AI assistants gain persistent workspace memory without any developer intervention.

**Acceptance Scenarios**:

1. **Given** a workspace with the engram plugin installed but no running memory service, **When** an MCP client issues a `set_workspace` tool call, **Then** the memory service starts automatically within 2 seconds and returns a successful response.
2. **Given** a workspace with an active memory service, **When** an MCP client issues a tool call, **Then** the response arrives within 50ms for read operations and 10ms for write operations.
3. **Given** a workspace with the engram plugin installed, **When** the developer opens a new terminal and invokes an MCP client, **Then** the client connects to the same active memory service without starting a duplicate.

---

### User Story 2 - Workspace Isolation Without Port Conflicts (Priority: P1)

As a developer working on multiple projects simultaneously, I need each project's memory service to run independently without port collisions, shared state, or cross-contamination, so I can safely use AI assistants across all my workspaces at the same time.

Each workspace's memory service communicates through a workspace-local channel (not a network port), making conflicts impossible regardless of how many projects are open. No workspace can access another workspace's data, even if both services are running concurrently.

**Why this priority**: Port collisions and cross-workspace data leaks are the primary problems motivating this refactoring. Without isolation, the architecture change has no value over the current centralized model.

**Independent Test**: Can be fully tested by installing the plugin in two separate workspaces, starting both simultaneously, storing data in each, and verifying that queries in one workspace never return data from the other. Delivers value: developers can run 20+ workspaces without conflicts.

**Acceptance Scenarios**:

1. **Given** two workspaces (A and B) each with the engram plugin, **When** both memory services are started simultaneously, **Then** neither service interferes with the other and both respond normally.
2. **Given** workspace A with task "Fix login bug" stored, **When** an agent in workspace B queries for tasks, **Then** the query returns only workspace B's tasks with no leakage from workspace A.
3. **Given** 20 workspaces with active memory services, **When** a new workspace starts its service, **Then** it starts successfully without displacement or conflict with existing services.

---

### User Story 3 - Automatic Lifecycle Management (Priority: P2)

As a developer, I need the memory service to manage its own lifecycle — starting on demand, staying alive while active, and shutting down gracefully after a period of inactivity — so that idle workspaces do not consume system resources.

The memory service starts only when needed (triggered by a tool call) and automatically shuts down after a configurable idle timeout (default: 4 hours). Shutdown preserves all data integrity, flushing pending state to disk before exiting. On next invocation, the service restarts transparently.

**Why this priority**: Automatic lifecycle management prevents zombie processes from draining system resources (CPU, RAM, battery). This is critical for laptop developers who may have dozens of past workspaces.

**Independent Test**: Can be fully tested by starting a memory service, waiting for the idle timeout to expire, verifying clean shutdown occurred, then invoking a tool call and verifying seamless restart. Delivers value: zero resource waste from idle projects.

**Acceptance Scenarios**:

1. **Given** an active memory service with no tool calls or file changes for the configured idle timeout, **When** the timeout expires, **Then** the service flushes all pending state to disk, cleans up runtime artifacts, and exits cleanly.
2. **Given** a memory service that was previously shut down by idle timeout, **When** an MCP client issues a tool call, **Then** the service restarts within 2 seconds and responds with correct, un-corrupted data.
3. **Given** an active memory service receiving periodic tool calls, **When** each call is received, **Then** the idle timeout resets and the service remains running.
4. **Given** a system power loss or crash during memory service operation, **When** the service is next started, **Then** it recovers gracefully by rehydrating from the persisted `.engram/` files without data loss.

---

### User Story 4 - Real-Time File System Awareness (Priority: P2)

As an AI agent, I need the memory service to continuously monitor workspace file changes, so that when I query for context (file relationships, recent changes, code structure), the information reflects the current state of the workspace — not a stale snapshot from the last explicit sync.

While the memory service is running, it watches the workspace file system for creates, modifications, and deletions. Changes are debounced and processed in near-real-time, updating the internal knowledge graph. When the agent queries memory, results include the latest file states.

**Why this priority**: Real-time awareness is what distinguishes a persistent daemon from on-demand indexing. It enables agents to understand the workspace as it evolves, rather than asking for re-indexing at each prompt.

**Independent Test**: Can be fully tested by starting the memory service, modifying a file in the workspace, waiting 2 seconds, and querying memory for the change. Delivers value: agents always see current workspace state.

**Acceptance Scenarios**:

1. **Given** an active memory service, **When** a file in the workspace is created, modified, or deleted, **Then** the change is reflected in query results within 2 seconds.
2. **Given** rapid consecutive saves to a file (e.g., IDE auto-save), **When** the memory service processes these events, **Then** it debounces them into a single update rather than processing each save individually.
3. **Given** an active memory service, **When** changes occur in excluded directories (e.g., `.engram/`, `.git/`, `node_modules/`, `target/`), **Then** those changes are ignored and do not trigger processing.

---

### User Story 5 - Plugin Installation & Management (Priority: P3)

As a developer, I need simple commands to install, update, reinstall, and uninstall the engram plugin in any workspace, so that setup is painless and recovery from corruption is straightforward.

Installation creates the required directory structure, verifies the runtime is available, generates the MCP client configuration file, and confirms readiness. Update replaces the runtime while preserving stored data. Reinstall performs a clean installation in case of corruption. Uninstall removes all plugin artifacts cleanly.

**Why this priority**: Good installation UX is important but secondary to the core memory and isolation functionality. The system must work correctly before it needs easy installation.

**Independent Test**: Can be fully tested by running the install command in a clean workspace, verifying all artifacts are created, running an MCP tool call, then uninstalling and verifying complete cleanup. Delivers value: frictionless onboarding for new workspaces.

**Acceptance Scenarios**:

1. **Given** a workspace without engram installed, **When** the developer runs the install command, **Then** the `.engram/` directory structure is created, the MCP configuration file is generated, and a verification check confirms readiness.
2. **Given** a workspace with engram installed and data stored, **When** the developer runs the update command, **Then** the runtime is updated but all stored task data, context, and configuration are preserved.
3. **Given** a workspace with a corrupted engram installation, **When** the developer runs the reinstall command, **Then** the runtime artifacts are replaced cleanly while the database is rehydrated from `.engram/` files.
4. **Given** a workspace with engram installed, **When** the developer runs the uninstall command, **Then** all plugin artifacts (runtime files, sockets, PID files) are removed, with an option to preserve or delete the stored data in `.engram/`.

---

### User Story 6 - Configurable Behavior (Priority: P3)

As a developer, I need to customize the memory service behavior (idle timeout, watched directories, debounce timing, file extensions) through a configuration file, so that engram adapts to different project sizes and structures.

A configuration file in `.engram/` allows customization of operational parameters. Sensible defaults work for most projects, so configuration is entirely optional. Changes to configuration take effect on the next service restart.

**Why this priority**: Configurability is a polish feature. The system must work well with defaults before customization matters.

**Independent Test**: Can be fully tested by creating a configuration file with a custom idle timeout, starting the service, and verifying the custom timeout is respected. Delivers value: engram adapts to diverse project types.

**Acceptance Scenarios**:

1. **Given** no configuration file exists, **When** the memory service starts, **Then** it operates with sensible defaults (4-hour idle timeout, 500ms debounce, standard exclusion list).
2. **Given** a configuration file specifying a 30-minute idle timeout, **When** the memory service starts, **Then** it uses the 30-minute timeout instead of the default.
3. **Given** a configuration file specifying additional directories to watch or exclude, **When** the memory service starts and files change in those directories, **Then** it respects the custom inclusion/exclusion rules.

### Edge Cases

- What happens when the workspace path contains spaces, Unicode characters, or symlinks? The system must handle all valid OS path formats correctly.
- What happens when the developer moves or renames the workspace directory while the service is running? The service must detect the invalidation and shut down cleanly.
- What happens when two MCP clients try to start the service simultaneously (race condition)? Only one service instance must ever run per workspace, enforced by an exclusive lock.
- What happens when the persistent data files in `.engram/` are manually edited or corrupted? The service must detect corruption and attempt rehydration before failing with a clear error.
- What happens when disk space runs out during a flush operation? The service must fail atomically — no partial writes — and report the error clearly.
- What happens when the service is started in a read-only filesystem? The service must fail fast with a clear error rather than silently dropping writes.
- What happens when the service process is killed with SIGKILL (or equivalent)? On next startup, stale runtime artifacts (lock files, sockets) must be detected and cleaned up.
- What happens when a very large workspace (100,000+ files) is indexed for the first time? Initial indexing must not block tool call responses; it should proceed in the background with progressive availability.

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
