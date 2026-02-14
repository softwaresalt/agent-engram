# **T-Mem: Task Memory System Specification**

## **1\. Executive Summary**

**T-Mem (Task Memory)** is a high-performance, local-first state engine designed to serve as the "shared brain" for software development environments. It bridges the gap between ephemeral AI agent sessions and persistent project history.

Functioning as a **Consumer-Agnostic Model Context Protocol (MCP) Server**, T-Mem runs as a background daemon. While typically orchestrated by a host process (like a CLI tool or IDE), it is designed to run independently, utilizing an embedded **SurrealDB** instance to manage a semantic graph of tasks, specifications, and architectural decisions. It ensures all state is serializable to Git-friendly Markdown files for team collaboration.

## **2\. System Architecture**

### **2.1 Multi-Client Connectivity Model**

T-Mem is architected as a **Universal State Hub**. It does not distinguish between "orchestrators" or "observers" at the architectural level. Instead, it exposes a uniform interface via **Server-Sent Events (SSE)**, allowing any MCP-compatible client to connect, query, and modify state concurrently.

**Unified Client Capabilities:**

* **Concurrent Access**: T-Mem supports multiple simultaneous connections (e.g., an Orchestrator CLI running background agents, an IDE querying context, and a terminal dashboard) without locking contention.  
* **Role Flexibility**: Any connected client can perform any action (Read or Write), provided they have the correct tool definitions.  
  * *Example*: An **Automated Orchestrator** might write high-volume updates during execution.  
  * *Example*: An **IDE** might strictly query context for chat grounding.  
  * *Example*: A **CI Pipeline** might connect solely to "Dehydrate" the state for a build artifact.  
* **Real-Time Synchronization**: State changes made by one consumer are immediately consistent and available to all other connected consumers.

### **2.2 Workspace Isolation & Multi-Tenancy**

To support developers working on multiple codebases simultaneously, T-Mem implements a **Multi-Tenant Architecture** within a single running daemon.

* **The Session Scope**: T-Mem acts as a user-level singleton. A single process manages state for all of the user's active projects.  
* **Workspace Mapping**:  
  * When a client connects (or invokes the workspace tool), it provides the **Project Root Path**.  
  * T-Mem maps this path to a unique, isolated **SurrealDB Database** (e.g., NS: t-mem, DB: project\_hash\_xyz).  
* **Strict Isolation**: Queries and Vector Searches are strictly scoped to the active database. Data from "Repo A" is physically segmented from "Repo B," ensuring no context leakage occurs between projects.

### **2.3 Technology Stack**

* **Language**: Rust (2024 edition).  
* **Server Framework**: axum (High-performance async HTTP).  
* **Protocol**: mcp-sdk-rs (Implementing SSE Transport for multi-client support).  
* **Database**: surrealdb (Embedded surrealkv mode).  
  * *Storage Note*: The runtime binary databases are stored in a central user directory (e.g., \~/.local/share/t-mem/), mapped by project hash. They are NOT stored in the repo to avoid binary merge conflicts.  
* **Serialization**: serde \+ pulldown-cmark (Markdown parsing/generation).

### **2.4 Concurrency Model**

T-Mem supports multiple simultaneous clients modifying state. The following semantics ensure predictable behavior:

**Write Conflict Resolution:**

* **Simple Fields (status, title, description)**: Last-write-wins using `updated_at` timestamps. The most recent write overwrites previous values.
* **Append-Only Fields (context, notes)**: New entries are appended, never overwritten. Each entry includes `created_at` and `source_client` for auditability.
* **Graph Edges (depends_on, implements, relates_to)**: Idempotent operations. Adding an existing edge is a no-op; removing a non-existent edge succeeds silently.

**Flush Coordination:**

* `flush_state` acquires a per-workspace write lock.
* Concurrent `flush_state` calls for the same workspace are queued (FIFO).
* Read operations are never blocked by flush operations.

**Atomicity Guarantees:**

* Individual tool calls are atomic within SurrealDB transactions.
* Cross-tool atomicity is NOT guaranteed (clients must implement their own sagas if needed).

### **2.5 Connection Lifecycle**

Each SSE connection follows a defined lifecycle:

**Connection Establishment:**

1. Client connects to SSE endpoint.
2. T-Mem assigns a unique `connection_id` (UUID v4).
3. Connection enters `CONNECTED` state (no workspace context yet).

**Workspace Binding:**

1. Client calls `set_workspace(path)`.
2. T-Mem validates path, hydrates workspace if needed.
3. Connection enters `ACTIVE` state with workspace context.

**Keepalive & Timeout:**

* SSE keepalive ping every 15 seconds.
* Connection timeout after 60 seconds of no activity (configurable).
* Clients should send periodic `check_status` or implement ping handling.

**Disconnection Behavior:**

* **Clean disconnect**: Client closes SSE stream gracefully.
* **Timeout disconnect**: Connection reaped after timeout period.
* **No auto-flush**: Workspace state persists in SurrealDB; manual `flush_state` required before commit.
* **Resource cleanup**: Connection-specific resources released; workspace remains active if other connections exist.

**Graceful Shutdown:**

* On SIGTERM/SIGINT, T-Mem flushes all active workspaces.
* Maximum 10-second grace period for in-flight operations.
* Clients receive SSE close event before connection termination.

## **3\. Data Architecture (SurrealDB Schema)**

T-Mem uses a Graph-Relational model to track not just *what* needs to be done, but *why* and *how*. The schema below applies to **each isolated project database**.

### **3.1 Core Tables**

```surql
-- The Intent: High-level requirements from spec files
DEFINE TABLE spec SCHEMAFULL;
DEFINE FIELD title ON TABLE spec TYPE string;
DEFINE FIELD content ON TABLE spec TYPE string;
DEFINE FIELD embedding ON TABLE spec TYPE array<float>;
DEFINE FIELD file_path ON TABLE spec TYPE string;
DEFINE FIELD created_at ON TABLE spec TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE spec TYPE datetime DEFAULT time::now();

-- Indexes for spec table
DEFINE INDEX spec_file_path ON TABLE spec COLUMNS file_path UNIQUE;
DEFINE INDEX spec_embedding ON TABLE spec COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;

-- The Unit of Work: Actionable items from tasks.md
DEFINE TABLE task SCHEMAFULL;
DEFINE FIELD title ON TABLE task TYPE string;
DEFINE FIELD status ON TABLE task TYPE string;
  -- Valid values: 'todo', 'in_progress', 'done', 'blocked'
DEFINE FIELD work_item_id ON TABLE task TYPE option<string>;
  -- Reference-only link to external trackers (ADO: "AB#12345", GitHub: "org/repo#123")
  -- No bidirectional sync in v0; future versions may add tracker integration
DEFINE FIELD description ON TABLE task TYPE string;
DEFINE FIELD context_summary ON TABLE task TYPE option<string>;
  -- AI-generated summary of "what happened so far"
DEFINE FIELD created_at ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE task TYPE datetime DEFAULT time::now();

-- Indexes for task table
DEFINE INDEX task_status ON TABLE task COLUMNS status;
DEFINE INDEX task_work_item ON TABLE task COLUMNS work_item_id;
DEFINE INDEX task_updated ON TABLE task COLUMNS updated_at;

-- The Memory: Ephemeral knowledge captured during execution
DEFINE TABLE context SCHEMAFULL;
DEFINE FIELD content ON TABLE context TYPE string;
DEFINE FIELD embedding ON TABLE context TYPE array<float>;
DEFINE FIELD source_client ON TABLE context TYPE string;
  -- Tracks which client added this (e.g., 'orchestrator_cli', 'ide', 'ci')
DEFINE FIELD created_at ON TABLE context TYPE datetime DEFAULT time::now();

-- Indexes for context table
DEFINE INDEX context_source ON TABLE context COLUMNS source_client;
DEFINE INDEX context_created ON TABLE context COLUMNS created_at;
DEFINE INDEX context_embedding ON TABLE context COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
```

### **3.2 Relations (The Graph)**

```surql
-- Dependency Tracking (Task -> depends_on -> Task)
DEFINE TABLE depends_on SCHEMAFULL;
DEFINE FIELD in ON TABLE depends_on TYPE record<task>;
DEFINE FIELD out ON TABLE depends_on TYPE record<task>;
DEFINE FIELD type ON TABLE depends_on TYPE string;
  -- Valid values: 'hard_blocker', 'soft_dependency'
DEFINE FIELD created_at ON TABLE depends_on TYPE datetime DEFAULT time::now();

-- Traceability: Task implements Spec
DEFINE TABLE implements SCHEMAFULL;
DEFINE FIELD in ON TABLE implements TYPE record<task>;
DEFINE FIELD out ON TABLE implements TYPE record<spec>;
DEFINE FIELD created_at ON TABLE implements TYPE datetime DEFAULT time::now();

-- Traceability: Task relates to Context
DEFINE TABLE relates_to SCHEMAFULL;
DEFINE FIELD in ON TABLE relates_to TYPE record<task>;
DEFINE FIELD out ON TABLE relates_to TYPE record<context>;
DEFINE FIELD created_at ON TABLE relates_to TYPE datetime DEFAULT time::now();
```

### **3.3 Vector Search Configuration**

T-Mem uses local embedding generation for semantic search capabilities:

**Embedding Model:**

* **Default Model**: `all-MiniLM-L6-v2` (sentence-transformers)
* **Dimensions**: 384 floats
* **Distance Metric**: Cosine similarity
* **Provider**: `fastembed-rs` (Rust-native, no Python dependency)

**Model Storage:**

* **Cache Location**: `~/.local/share/t-mem/models/`
* **Download Behavior**: Lazy download on first `query_memory` call
* **Offline Mode**: If model exists in cache, no network required
* **Model Size**: ~90MB for default model

**Indexing Strategy:**

* **Index Type**: MTREE (M-Tree for metric space indexing)
* **Indexed Tables**: `spec.embedding`, `context.embedding`
* **Re-indexing**: Triggered on hydration if embeddings are stale or missing

**Hybrid Search:**

* `query_memory` combines vector similarity (semantic) with keyword matching (BM25-style)
* Results ranked by weighted combination: 0.7 * vector_score + 0.3 * keyword_score
* Top-K results returned (default K=10, configurable per query)

## **4\. Git-Backed Persistence Strategy**

To ensure T-Mem state travels with the repo, the system implements a **Hydration/Dehydration Cycle**.

### **4.1 Storage Layout (.tmem/)**

The repository will contain a hidden `.tmem` directory **at the root of each Git repo**:

```
/path/to/my-repo/.tmem/
├── .version              # Schema version for migration compatibility
├── tasks.md              # Canonical source of task state (human-readable)
├── graph.surql           # Relationship dumps not expressible in Markdown
├── specs/                # Spec file references and metadata
│   └── index.md          # Registry of tracked spec files with paths
├── context/              # Memory fragments captured during execution
│   ├── decisions/        # Architectural Decision Records (ADRs)
│   │   └── 001-auth-strategy.md
│   └── snapshots/        # Point-in-time context captures
│       └── api-schema-v1.md
└── .lastflush            # Timestamp of last dehydration (for conflict detection)
```

**File Format Details:**

* **`.version`**: Single line containing schema version (e.g., `1.0.0`)
* **`tasks.md`**: Markdown with YAML frontmatter per task; human-editable
* **`graph.surql`**: SurrealQL statements for relationships; machine-generated
* **`specs/index.md`**: Table mapping spec IDs to file paths in repo
* **`.lastflush`**: ISO 8601 timestamp; used for stale detection

### **4.2 The Lifecycle**

1. **Hydration (Startup/Context Switch)**:  
   * When a client activates a workspace, T-Mem locates the `.tmem/` folder in that specific path.  
   * T-Mem loads the data into the corresponding **SurrealDB Database** for that project.  
   * If the DB is empty (first run), it builds the graph from the files.
   * Embeddings are generated for any content missing vector representations.
2. **Dehydration (Commit/Sync)**:  
   * Triggered by `flush_state` or graceful daemon shutdown.  
   * T-Mem serializes the active SurrealDB database into the specific `.tmem/` directory of that workspace.  
   * **Crucial**: It preserves user comments in `tasks.md` by using a diff-match-patch strategy, ensuring the file remains human-editable.
   * Updates `.lastflush` timestamp after successful write.

### **4.3 Edge Cases & Recovery**

**External File Modification:**

* On `set_workspace`, T-Mem compares `.lastflush` against file modification times.
* If `.tmem/` files are newer than `.lastflush`, a **stale warning** is emitted.
* Behavior options (configurable):
  * `warn`: Log warning, proceed with DB state (default)
  * `rehydrate`: Discard DB state, rebuild from files
  * `fail`: Return error, require explicit resolution

**Partial/Corrupted State:**

| Condition | Recovery Action |
|-----------|----------------|
| `.tmem/` missing entirely | Initialize empty workspace |
| `tasks.md` missing | Initialize empty task list |
| `graph.surql` missing | Rebuild from task relationships |
| `.version` missing | Assume v1.0.0, migrate if needed |
| Parse error in any file | Return `HydrationError` with file path and line |

**Schema Migration:**

* `.version` file tracks schema version.
* On hydration, T-Mem checks version compatibility.
* Forward migrations applied automatically (v1 → v2).
* Backward-incompatible versions return `SchemaMismatchError` with upgrade instructions.

**Database Corruption:**

* If SurrealDB database fails integrity checks, T-Mem:
  1. Logs corruption details to stderr.
  2. Deletes corrupted database files.
  3. Triggers full re-hydration from `.tmem/` files.
  4. Returns `RecoveryCompleted` status to client.

**Concurrent External Edits:**

* T-Mem does NOT watch `.tmem/` for live changes.
* External edits between `set_workspace` and `flush_state` may be overwritten.
* Recommendation: Use `flush_state` before `git add` to ensure consistency.

## **5\. API Specification (MCP Tools)**

T-Mem exposes a standard set of tools. These tools are available to *any* authenticated MCP client.

### **5.1 Lifecycle & Context Tools**

* **set_workspace**(path: string) → WorkspaceResult  
  * **Mandatory**: Must be called upon connection to define which repo the client is working in.  
  * T-Mem validates the path and switches the SurrealDB `USE NS tmem DB {hash}` context for this connection.
  * Returns workspace metadata on success.

* **get_daemon_status**() → DaemonStatus  
  * Returns daemon health and operational metrics (no workspace context required).
  * Response:
    ```json
    {
      "version": "0.1.0",
      "uptime_seconds": 3600,
      "active_workspaces": 3,
      "active_connections": 5,
      "memory_bytes": 52428800,
      "model_loaded": true,
      "model_name": "all-MiniLM-L6-v2"
    }
    ```

* **get_workspace_status**() → WorkspaceStatus  
  * Returns status of the currently active workspace for this connection.
  * Response:
    ```json
    {
      "path": "/path/to/repo",
      "task_count": 42,
      "context_count": 128,
      "last_flush": "2026-02-05T10:30:00Z",
      "stale_files": false,
      "connection_count": 2
    }
    ```

### **5.2 Read Tools**

* **query_memory**(query: string) → QueryResult  
  * Performs a hybrid search (Vector + Keyword) across Specs and Contexts *within the active workspace*.  
  * Returns: Ranked snippets with relevance scores to ground the agent.
  * Parameters:
    * `query`: Natural language search query (max 500 tokens)
    * `limit` (optional): Maximum results to return (default: 10)

* **get_task_graph**(root_task_id: string) → TaskGraph  
  * Returns a tree view of subtasks and dependencies.
  * Includes: task status, blockers, and implementation links.

* **check_status**(work_item_ids: string[]) → StatusMap  
  * Returns current status for specified work item IDs from the DB.
  * Useful for batch status checks from external trackers.

### **5.3 Write Tools**

* **update_task**(id: string, status: string, notes: string) → TaskResult  
  * Updates the task record and appends a "progress note" to the context.
  * Automatically updates `updated_at` timestamp.
  * Valid status values: `todo`, `in_progress`, `done`, `blocked`

* **add_blocker**(task_id: string, reason: string) → BlockerResult  
  * Sets task status to `blocked` and creates a linked context node explaining the blocker.
  * Returns the created blocker context ID.

* **register_decision**(topic: string, decision: string) → DecisionResult  
  * Stores a permanent architectural decision (ADR) into the graph.
  * Creates a context node in `context/decisions/` on next flush.

* **flush_state**() → FlushResult  
  * Forces a dehydration of the *active workspace's memory* to its local `.tmem/` directory.
  * Returns file paths written and any warnings.

### **5.4 Error Taxonomy**

All MCP tool errors use structured error responses with the following codes:

**Workspace Errors (1xxx):**

| Code | Name | Description |
|------|------|-------------|
| 1001 | `WorkspaceNotFound` | Specified path does not exist |
| 1002 | `NotAGitRoot` | Path exists but lacks `.git/` directory |
| 1003 | `WorkspaceNotSet` | Tool called before `set_workspace` |
| 1004 | `WorkspaceAlreadyActive` | `set_workspace` called with same path (no-op warning) |

**Hydration Errors (2xxx):**

| Code | Name | Description |
|------|------|-------------|
| 2001 | `HydrationFailed` | Failed to parse `.tmem/` files |
| 2002 | `SchemaMismatch` | `.tmem/` version incompatible with daemon |
| 2003 | `CorruptedState` | Database or file integrity check failed |
| 2004 | `StaleWorkspace` | External modifications detected (warning) |

**Task Errors (3xxx):**

| Code | Name | Description |
|------|------|-------------|
| 3001 | `TaskNotFound` | Task ID does not exist |
| 3002 | `InvalidStatus` | Status value not in allowed set |
| 3003 | `CyclicDependency` | Adding dependency would create cycle |
| 3004 | `BlockerExists` | Task already has active blocker |

**Query Errors (4xxx):**

| Code | Name | Description |
|------|------|-------------|
| 4001 | `QueryTooLong` | Query exceeds maximum token limit |
| 4002 | `ModelNotLoaded` | Embedding model failed to initialize |
| 4003 | `SearchFailed` | Vector/keyword search internal error |

**System Errors (5xxx):**

| Code | Name | Description |
|------|------|-------------|
| 5001 | `DatabaseError` | SurrealDB operation failed |
| 5002 | `FlushFailed` | Could not write to `.tmem/` directory |
| 5003 | `RateLimited` | Too many requests from connection |
| 5004 | `ShuttingDown` | Daemon is in graceful shutdown |

**Error Response Format:**

```json
{
  "error": {
    "code": 1001,
    "name": "WorkspaceNotFound",
    "message": "Path '/invalid/path' does not exist",
    "details": {
      "path": "/invalid/path",
      "suggestion": "Verify the path exists and is accessible"
    }
  }
}
```

## **6\. Implementation Plan**

The implementation is divided into phases with realistic timelines. Each phase produces a testable deliverable.

### **Phase 1: Core Daemon (Weeks 1-3)**

* **Deliverable**: A functional Rust binary `t-mem` that starts an HTTP server with basic MCP transport.
* **Tasks**:
  * Project scaffolding: Cargo workspace, CI/CD, linting configuration
  * Initialize axum server with SSE transport for MCP
  * Embed SurrealDB (surrealkv backing)
  * Implement `set_workspace` with namespace isolation
  * Implement `get_daemon_status` and `get_workspace_status`
  * Connection lifecycle management (keepalive, timeout, cleanup)
  * Unit tests for all core components
* **Exit Criteria**: Daemon starts, accepts connections, switches workspaces

### **Phase 2: Persistence & Serialization (Weeks 4-5)**

* **Deliverable**: Two-way sync between `.tmem/tasks.md` and SurrealDB.
* **Tasks**:
  * Implement Markdown parser (using pulldown-cmark) to extract tasks
  * Implement Dehydrator (Struct → Markdown with diff-match-patch)
  * Define SurrealDB schemas with indexes
  * Implement `flush_state` with atomic writes
  * Edge case handling: stale detection, corruption recovery
  * Schema versioning and migration framework
  * Integration tests for hydration/dehydration cycles
* **Exit Criteria**: Round-trip serialization preserves user comments

### **Phase 3: Task Operations (Week 6)**

* **Deliverable**: Full task CRUD and graph operations.
* **Tasks**:
  * Implement `update_task`, `add_blocker`, `register_decision`
  * Implement `get_task_graph`, `check_status`
  * Graph edge operations with cycle detection
  * Concurrency testing with multiple clients
* **Exit Criteria**: All write tools functional, concurrent stress tests pass

### **Phase 4: Vector Search Integration (Weeks 7-8)**

* **Deliverable**: Semantic search capability via `query_memory`.
* **Tasks**:
  * Integrate `fastembed-rs` for local embedding generation
  * Implement lazy model download and caching
  * Hybrid search (vector + keyword) implementation
  * MTREE index configuration and tuning
  * Performance benchmarks (< 100ms query latency target)
* **Exit Criteria**: `query_memory` returns relevant results, offline-capable

### **Phase 5: Integration & Hardening (Weeks 9-10)**

* **Deliverable**: Production-ready daemon with reference client validation.
* **Tasks**:
  * Reference CLI integration (daemon spawning, worktree injection)
  * Concurrent access stress testing (10+ simultaneous clients)
  * Error taxonomy validation (all codes exercised in tests)
  * Graceful shutdown testing
  * Documentation: API reference, troubleshooting guide
  * Performance profiling and optimization
* **Exit Criteria**: All NFRs met, documentation complete

### **Timeline Summary**

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 1: Core Daemon | 3 weeks | Week 3 |
| Phase 2: Persistence | 2 weeks | Week 5 |
| Phase 3: Task Operations | 1 week | Week 6 |
| Phase 4: Vector Search | 2 weeks | Week 8 |
| Phase 5: Hardening | 2 weeks | Week 10 |

**Total Estimated Duration: 10 weeks**

## **7\. Non-Functional Requirements**

* **Startup Time**: \< 200ms (Must feel instant to any client).  
* **Memory Footprint**: \< 100MB RAM when idle.  
* **Concurrency**: Must handle at least 10 simultaneous client connections (via Tokio).  
* **Security**: Bind only to 127.0.0.1.  
* **Isolation**: Strict data boundaries between workspaces; no query leakage.