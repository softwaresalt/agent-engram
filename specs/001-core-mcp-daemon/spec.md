# Feature Specification: T-Mem Core MCP Daemon

**Feature Branch**: `001-core-mcp-daemon`  
**Created**: 2026-02-05  
**Status**: Draft  
**Input**: Implement T-Mem v0 core MCP daemon: a high-performance local-first state engine serving as the shared brain for software development environments with SurrealDB backend, SSE transport, workspace isolation, and git-backed persistence

## User Scenarios & Testing *(mandatory)*

<!--
  User stories derived from T-Mem v0 specification.
  Each story represents an independently deliverable slice of the daemon.
-->

### User Story 1 - Daemon Connection & Workspace Binding (Priority: P1)

As an MCP client (CLI, IDE, or agent), I connect to the T-Mem daemon and bind to a specific Git repository workspace so that all subsequent operations are scoped to that project's state.

**Why this priority**: This is the foundational capability. Without connection and workspace binding, no other features can function. Every client interaction begins here.

**Independent Test**: Start the daemon, connect via SSE, call `set_workspace` with a valid Git repo path, and verify the connection enters ACTIVE state with workspace metadata returned.

**Acceptance Scenarios**:

1. **Given** the daemon is running, **When** a client connects to the SSE endpoint, **Then** the daemon assigns a unique connection ID and the connection enters CONNECTED state
2. **Given** a CONNECTED client, **When** `set_workspace("/path/to/git/repo")` is called, **Then** the daemon validates the path has a `.git/` directory and returns workspace metadata
3. **Given** a client with ACTIVE workspace, **When** `get_workspace_status()` is called, **Then** the daemon returns task count, context count, and last flush timestamp
4. **Given** a client calls `set_workspace` with an invalid path, **When** the path does not exist, **Then** the daemon returns error code 1001 (WorkspaceNotFound)

---

### User Story 2 - Task State Management (Priority: P2)

As an orchestrator or agent, I create, update, and query tasks within my workspace so that work progress is tracked and persisted across sessions.

**Why this priority**: Task management is the core value proposition. Once connected, clients need to read and write task state to coordinate work.

**Independent Test**: Connect to workspace, call `update_task` to modify a task status, call `get_task_graph` to verify the change, then call `flush_state` and verify the `.tmem/tasks.md` file reflects the update.

**Acceptance Scenarios**:

1. **Given** an ACTIVE workspace with existing tasks, **When** `update_task(id, "in_progress", "Starting work")` is called, **Then** the task status changes and a context note is appended
2. **Given** a task in progress, **When** `add_blocker(task_id, "Waiting for API response")` is called, **Then** the task status becomes "blocked" and a blocker context node is created
3. **Given** an ACTIVE workspace, **When** `get_task_graph(root_id)` is called, **Then** a tree view of subtasks and dependencies is returned with current status
4. **Given** an ACTIVE workspace, **When** `register_decision("auth", "Use OAuth2")` is called, **Then** an architectural decision record is stored in the graph

---

### User Story 3 - Git-Backed Persistence (Priority: P3)

As a developer, I flush workspace state to `.tmem/` files in my Git repository so that task state, context, and decisions travel with the codebase and can be committed, merged, and shared with teammates.

**Why this priority**: Persistence to Git-friendly files enables collaboration and state recovery. Without this, T-Mem is ephemeral.

**Independent Test**: Modify task state via MCP tools, call `flush_state`, verify `.tmem/tasks.md` contains human-readable task entries with preserved comments, and verify round-trip hydration reproduces the same state.

**Acceptance Scenarios**:

1. **Given** modified workspace state, **When** `flush_state()` is called, **Then** the daemon writes `.tmem/tasks.md`, `.tmem/graph.surql`, and updates `.tmem/.lastflush`
2. **Given** a `.tmem/tasks.md` with user comments, **When** `flush_state()` is called after task updates, **Then** user comments are preserved using diff-match-patch
3. **Given** a new workspace with no `.tmem/` directory, **When** `set_workspace` is called, **Then** the daemon initializes an empty workspace structure
4. **Given** corrupted SurrealDB database files, **When** `set_workspace` is called, **Then** the daemon recovers by re-hydrating from `.tmem/` files

---

### User Story 4 - Semantic Memory Query (Priority: P4)

As an AI agent, I query the workspace memory using natural language so that I receive relevant context from specs, tasks, and prior decisions to ground my responses.

**Why this priority**: Semantic search adds intelligence to context retrieval. Functional without it (can use task graph), but significantly enhanced with vector search.

**Independent Test**: Populate a workspace with specs and context, call `query_memory("authentication flow")`, and verify results include semantically related content ranked by relevance.

**Acceptance Scenarios**:

1. **Given** a workspace with specs and context, **When** `query_memory("user login")` is called, **Then** the daemon returns ranked snippets combining vector similarity and keyword matching
2. **Given** the embedding model is not yet downloaded, **When** `query_memory` is called for the first time, **Then** the model is lazily downloaded to `~/.local/share/t-mem/models/`
3. **Given** no network access and model in cache, **When** `query_memory` is called, **Then** the search completes using cached model (offline-capable)
4. **Given** a query exceeding 500 tokens, **When** `query_memory` is called, **Then** error code 4001 (QueryTooLong) is returned

---

### User Story 5 - Multi-Client Concurrent Access (Priority: P5)

As a development team, multiple clients (CLI orchestrator, IDE, dashboard) connect to the same daemon simultaneously so that all tools share a consistent view of workspace state without conflicts.

**Why this priority**: Concurrent access is essential for production use but requires all prior features to be stable first.

**Independent Test**: Connect 10 clients to the same workspace, have each perform interleaved read/write operations, verify no data corruption and all clients see consistent state.

**Acceptance Scenarios**:

1. **Given** 10 connected clients, **When** all call `get_workspace_status()` concurrently, **Then** all receive consistent responses within 50ms
2. **Given** two clients updating the same task, **When** updates arrive with different timestamps, **Then** last-write-wins based on `updated_at` with no data loss for append-only context
3. **Given** two clients calling `flush_state()` concurrently, **When** both flush the same workspace, **Then** operations are serialized (FIFO), both succeed, and file state is consistent
4. **Given** a client disconnects without flushing, **When** another client connects to the same workspace, **Then** the in-memory state is preserved and accessible

---

### Edge Cases

* What happens when workspace path contains symlinks? Canonicalize and validate the resolved path.
* How does system handle concurrent external edits to `.tmem/` files? Default: warn-and-proceed (emit StaleWorkspace warning 2004, continue with in-memory state). Configurable via daemon config to `rehydrate` (reload from disk) or `fail` (reject operation until explicit resolve).
* What happens if SurrealDB database grows very large? Performance degrades gracefully; recommend periodic archival of old context.
* How does system handle workspaces on network drives? Not officially supported; may have latency issues.
* What happens during ungraceful daemon termination (SIGKILL)? State in SurrealDB preserved; `.tmem/` may be stale until next flush.

## Clarifications

### Session 2026-02-09

- Q: What is the maximum number of concurrent workspaces per daemon? → A: Configurable upper bound with default of 10 (matches FR-002 client limit)
- Q: What is the default conflict strategy for concurrent external edits to `.tmem/` files? → A: Default warn (emit stale-workspace warning, proceed with in-memory state); configurable to rehydrate or fail

## Requirements *(mandatory)*

### Functional Requirements

**Connection & Lifecycle:**

* **FR-001**: System MUST start as a daemon binding to `127.0.0.1` on a configurable port
* **FR-002**: System MUST accept multiple simultaneous SSE connections (minimum 10 concurrent)
* **FR-003**: System MUST assign unique connection IDs (UUID v4) to each client
* **FR-004**: System MUST implement 15-second keepalive pings on SSE connections
* **FR-005**: System MUST timeout inactive connections after 60 seconds (configurable)
* **FR-006**: System MUST flush all active workspaces on graceful shutdown (SIGTERM/SIGINT)

**Workspace Management:**

* **FR-007**: System MUST validate workspace paths as existing directories with `.git/` subdirectory
* **FR-008**: System MUST reject paths containing `..` after canonicalization (path traversal prevention)
* **FR-009**: System MUST map each workspace to an isolated SurrealDB database via deterministic path hash
* **FR-009a**: System MUST enforce a configurable maximum number of concurrent active workspaces (default: 10); exceeding the limit returns an error prompting the client to release an existing workspace
* **FR-010**: System MUST hydrate workspace state from `.tmem/` files on first access
* **FR-011**: System MUST dehydrate workspace state to `.tmem/` files on `flush_state` call
* **FR-012**: System MUST preserve user comments in `tasks.md` during dehydration using diff-match-patch
* **FR-012a**: System MUST detect external modifications to `.tmem/` files (via mtime or content hash) before flush or hydrate operations
* **FR-012b**: System MUST default to warn-and-proceed when stale files are detected (emit error 2004 StaleWorkspace as warning, continue with in-memory state); behavior MUST be configurable to `rehydrate` or `fail`

**Task Operations:**

* **FR-013**: System MUST support task status values: `todo`, `in_progress`, `done`, `blocked`
* **FR-014**: System MUST automatically update `updated_at` timestamp on task modifications
* **FR-015**: System MUST append context notes on task updates (never overwrite existing context)
* **FR-016**: System MUST detect cyclic dependencies when adding task relationships
* **FR-017**: System MUST support linking tasks to external work item IDs (reference storage only)

**Memory & Search:**

* **FR-018**: System MUST generate embeddings using `all-MiniLM-L6-v2` model (384 dimensions)
* **FR-019**: System MUST perform hybrid search combining vector similarity (0.7 weight) and keyword matching (0.3 weight)
* **FR-020**: System MUST lazily download embedding model on first query if not cached
* **FR-021**: System MUST operate offline if model exists in local cache

**Observability:**

* **FR-022**: System MUST expose daemon status via `get_daemon_status()` tool (version, uptime, memory usage)
* **FR-023**: System MUST log all operations with structured tracing and correlation IDs
* **FR-024**: System MUST return structured error responses with numeric codes per error taxonomy

### Key Entities

* **Spec**: High-level requirement captured from specification files. Attributes: title, content, embedding, file_path, timestamps.
* **Task**: Unit of work derived from specs. Attributes: title, status, work_item_id (optional), description, context_summary, timestamps.
* **Context**: Ephemeral knowledge captured during execution. Attributes: content, embedding, source_client, created_at.
* **depends_on**: Graph edge representing task dependencies. Attributes: type (hard_blocker, soft_dependency).
* **implements**: Graph edge linking Task to Spec for traceability.
* **relates_to**: Graph edge linking Task to Context for memory association.

## Success Criteria *(mandatory)*

### Measurable Outcomes

* **SC-001**: Daemon cold start completes in under 200ms to accepting connections
* **SC-002**: Workspace hydration completes in under 500ms for projects with fewer than 1000 tasks
* **SC-003**: `query_memory` hybrid search returns results in under 100ms
* **SC-004**: `update_task` write operations complete in under 10ms
* **SC-005**: `flush_state` completes in under 1 second for full workspace dehydration
* **SC-006**: Daemon consumes less than 100MB RAM when idle with no active workspaces
* **SC-007**: Daemon handles 10 simultaneous client connections without request failures
* **SC-008**: Round-trip serialization (hydrate → modify → dehydrate → hydrate) preserves 100% of user comments in markdown files
* **SC-009**: All MCP tool errors return structured responses with appropriate error codes (no internal errors exposed)
* **SC-010**: 95% of `query_memory` results are relevant to the query (manual evaluation on test corpus)

## Assumptions

* Target platform is local developer workstations (Windows, macOS, Linux)
* Users have Git installed and workspaces are Git repositories
* Network access available for initial model download; subsequent operation can be offline
* Workspaces are on local filesystems (not network shares)
* Single user per daemon instance (no multi-user authentication required for localhost)

## Out of Scope (v0)

* Bidirectional sync with external work item trackers (ADO, GitHub Issues)
* Multi-user authentication/authorization
* Remote daemon access (always localhost)
* Real-time file watching for `.tmem/` changes
* Web UI or dashboard
* Workspace archival/cleanup utilities
