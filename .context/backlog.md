# Backlog

## Feature Requests (unassigned)

- Enable engram database to be capable of supporting agent loop memory and search.
- Consider that engram will be useful for the outer loop memory needed for more advanced agent frameworks.
- Need the ability to also track git commit numbers to changes in the repo through a graph representation with actual code and text snippets to enable faster change detection and search during agentic adversarial code reviews.
- Incorporate use of cargo-release for versioning and publishing, ensuring that each release is properly tagged and documented in the changelog, and that the release process is streamlined for contributors.

### Feature: 005-lifecycle-observability

Lifecycle Observability & Advanced Workflow Enforcement for Agent-Engram

1. Overview

This specification defines feature enhancements for agent-engram that introduce advanced lifecycle management, comprehensive workspace synchronization, state versioning, structured graph querying, hierarchical workflow groupings, and daemon observability. These capabilities extend the existing SurrealDB-backed daemon architecture, the hook-enforced shim, and the real-time file-syncing engine to provide robust state lifecycle control, deep observability, and strict workflow enforcement.

2. Feature Matrix

| Feature Category | Current State | Target Enhancement |
| --- | --- | --- |
| State Primitives | Tasks, Specs, Comments | Expand graph model to support hierarchical groupings (Epics/Collections) |
| Context Management | Hydration/Dehydration, Embeddings | Integrate selective routing via recursive graph traversals |
| Integrations | File-watcher sync | Add bi-directional tracker sync directly into the background daemon |
| Workflow Logic | Basic Lifecycle | Implement `->blocks->` edge gates enforced strictly by shim hooks |
| Search/Retrieval | Embedding / Semantic | Expose sandboxed read-only SurrealQL queries via the shim |
| Telemetry | Standard Logs | Implement tracing/OTLP in the daemon to monitor file-sync and wake latency |
| Concurrency | Static flush locks (ADR-0002) | Use daemon as the single source of truth alongside SurrealDB graph locks |

The overarching theme is pushing heavy validation and synchronization logic into the background daemon while keeping the shim fast and restrictive. Agent-engram leverages the inherent strengths of Rust (speed, safety) and SurrealDB (native relations, event streaming) to minimize polling and custom parsing overhead.

3. Detailed Feature Specifications

3.1 Advanced Lifecycle Management (Gates & Blocking)

Problem: Agent-engram currently handles basic state transitions but lacks strict dependency blocking. Without gates, agents can thrash, hallucinate on tasks that are not ready, or execute operations out of order.

Specification: Implement Hook-Enforced Graph Blocking Semantics.

- Introduce a standard directional edge type within the SurrealDB schema: `RELATE task:A->blocks->task:B`.
- When an autonomous agent attempts to modify, execute, or deeply hydrate a blocked task via the shim, registered agent hooks must immediately intercept the action.
- Hooks rapidly query the local daemon to evaluate the entire upstream dependency chain.
- If blocking constraints are detected, the hook forcefully rejects the operation with a highly specific contextual error.
- Example: If an agent tries to initiate implementation on a feature branch while the prerequisite "Design Review" node remains marked as incomplete, the hook intercepts the call, halts execution, and feeds a corrective prompt back to the agent. This forces the agent to redirect attention to the blocking task.
- Benefits: Drastically reduces wasted LLM tokens, prevents hallucinated out-of-order execution, and eliminates broken downstream workflows.

3.2 Comprehensive Workspace Synchronization

Problem: Agent-engram needs a unified real-time view of both local file system state and external project tracker state.

Specification: Unify File Sync and External Tracker Sync in the Daemon.

- The long-running daemon must be positioned as the authoritative central nervous system for the workspace.
- While it actively watches file system events to update internal graph nodes, it should simultaneously orchestrate background tasks or listen to incoming webhooks to continuously sync with external trackers (e.g., Jira, Linear).
- Leverage SurrealDB Live Queries to instantly stream external state changes directly to the active shim, eliminating the need for inefficient polling loops.
- When the shim queries the daemon, it accesses a heavily-indexed, multi-dimensional graph that is guaranteed to be a precise, real-time, and unified reflection of both the local codebase state and the remote project board.

3.3 State Versioning and Time Travel

Problem: SurrealDB does not have Git-like branching out of the box, but agent-engram needs the ability to recover from corrupted or hallucinated state changes.

Specification: Implement Graph Event Sourcing and Snapshots.

- Instead of destructive updates that overwrite historical fields, the daemon should maintain an immutable, append-only ledger of intent within SurrealDB.
- Log every discrete state change to a dedicated event table (e.g., `FileModified`, `TaskCreated`, `GraphEdgeAdded`, `StatusTransitioned`), permanently preserving the entire narrative and context of the agent's actions over time.
- Expose a dedicated shim command (e.g., `engram rollback`) that empowers the user or an automated oversight agent to systematically replay events in reverse, un-applying edges and restoring previous property values.
- This cleanly reverts a corrupted or hallucinated subgraph back to a stable point in time without the overhead of running a full version-control database server.

3.4 Sandboxed SurrealQL Query Interface

Problem: Agent-engram has semantic search via vector embeddings but lacks a way for an agent to perform complex, exact-match graph querying across its memory.

Specification: Expose Sandboxed SurrealQL via the Shim.

- Agent-engram does not need a custom DSL parser. SurrealQL is inherently built for traversing complex graph networks.
- Create a strict shim interface (e.g., `engram.query()`) that allows the agent to execute heavily sanitized, read-only SurrealQL queries.
- Example: `SELECT * FROM task WHERE status = 'InProgress' FETCH ->blocks->task`.
- The daemon parses and executes queries safely against the `.engram` database, giving the agent analytical power over its own memory without risking injection or unauthorized data manipulation.

3.5 Workflow Groupings (Epics / Collections)

Problem: Agent-engram currently treats nodes mostly individually, which can lead to context fragmentation when dealing with large features that span planning, implementation, and testing.

Specification: Implement Sub-graphs / Epic Nodes.

- Introduce an `Epic` or `Collection` node type in `src/models/`.
- Allow the shim's hydration request to target this macro-node.
- The daemon uses a SurrealDB recursive traversal (e.g., `SELECT ->contains->task FROM epic:feature_x`) to intelligently fetch all active sub-tasks.
- Cross-reference these tasks with the file-watcher's latest file states, assembling a surgical, highly cohesive prompt payload.
- Solves the "context stuffing" problem by ensuring the LLM only sees the exact cluster of information it needs for the current objective.

3.6 Daemon Lifecycle Observability

Problem: Agent-engram introduces a TTL for its daemon (wake-on-demand, sleep when idle), but lacks visibility into daemon performance, file-watcher latency, and query bottlenecks.

Specification: Add Tracing with TTL-Aware OTLP Exports.

- Integrate the `tracing` and `tracing-opentelemetry` Rust crates inside the daemon.
- Log precisely when the daemon wakes up, how long the initial file-sync sweep takes to reconcile diffs, and the exact latency of shim queries hitting SurrealDB.
- Dump trace spans to a local log in `.engram/logs` or export them to an APM tool.
- This observability is critical for identifying whether performance bottlenecks are caused by the LLM provider, the file-watcher, or the local daemon's startup penalty.

4. Agent-Engram Architectural Advantages

These enhancements preserve and amplify agent-engram's distinct architectural strengths:

- **Continuous File-System Graphing**: The background daemon constantly watches file diffs and ties them directly into the SurrealDB graph. A file is not just text — it is a living node automatically connected to the tasks that modified it, providing real-time semantic understanding of the codebase.
- **Native Graph Traversal**: SurrealDB natively supports deep, recursive dependency queries between files, tasks, and documentation — vastly faster and requiring significantly less code than building relational joins on top of a traditional SQL database.
- **Hook-Based Enforcement**: The agent-specific plugin with dedicated intercept hooks proactively and securely sandboxes the agent. It analyzes intent and blocks destructive or out-of-bounds actions before the request is fully processed. This shifts the agent's error-correction loop from a slow, token-heavy reactive process to an instantaneous, preventative safety barrier.

5. Phased Implementation Roadmap

**Phase 1: Daemon Foundation & Semantic Parity**

- Establish the `.engram` directory structure, ensuring the local graph travels with the repository.
- Finalize the background file-watching daemon logic and the TTL lifecycle management.
- Add `blocks` / `blocked_by` relational edges to the SurrealDB schema to support dependency tracking.
- Create the secure shim interface to execute read-only SurrealQL for exact-match filtering by the agent.

**Phase 2: Hooks & Strict Enforcement**

- Implement the agent hooks within the shim to intercept state transitions and memory modifications.
- Enforce blocking logic via these hooks, actively querying the daemon to validate if an action is permitted before letting the agent proceed.
- Implement event-sourcing logging in SurrealDB to allow primitive state rollback and historical reconstruction.

**Phase 3: Macro-Workflows & Live Tracker Sync**

- Expand the daemon to poll or subscribe to external trackers (Jira/Linear) in the background, working alongside the active file watcher.
- Create the `Collection`/`Epic` model to group tasks, updating the hydration service to recursively fetch related files and tasks as a single optimized context block.
- Integrate OpenTelemetry tracing into the daemon to monitor file-watcher latency, query performance, and TTL wake times.

6. Reliability Gate

- Current implementation of server is not demonstrating reliable availability and connection.
- Must prove reliable function within an active code workspace.
- Must include tool selection to ensure engram is used as part of code development and task management by the agent.
- Must include skill template to include or link/reference within agents and/or skills for maximum effectiveness.
- Observability must demonstrate actual usage of engram by agent in place of file search and markdown ingestion into agent context window.
