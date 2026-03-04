# Backlog

## Feature Requests (unassigned)
- Enable engram database to be capable of supporting agent loop memory and search.
- Consider that engram will be useful for the outer loop memory needed for more advanced agent frameworks.
- Need the ability to also track git commit numbers to changes in the repo through a graph representation with actual code and text snippets to enable faster change detection and search during agentic adversarial code reviews.

### Feature: 004-refactor-engram-server-as-plugin

Architecture Specification: agent-engram (Workspace-Local Daemon Model)

1. Overview

This document outlines the architectural refactoring of the agent-engram MCP server. The objective is to transition from a centralized HTTP Web Socket model to a Decentralized, Per-Workspace Daemon Architecture.

Historically, centralized agent memory systems suffer from configuration drift, port collisions, and the risk of cross-pollinating context between unrelated projects. This new approach guarantees that the local GitHub Copilot agent has uninterrupted, real-time context of the workspace's file system (via unified vector and graph databases in SurrealDB) while maintaining zero configuration overhead, strict context isolation, and a near-zero idle resource footprint. By embedding the entire intelligence engine directly into the project folder, the memory becomes as portable and context-bound as the source code itself.

2. Core Architecture

The system is split into two primary Rust binaries that reside directly within the local workspace in a dedicated .engram/ directory. This bifurcation is necessary to bridge the gap between GitHub Copilot's expectation of a short-lived, synchronous tool execution and the reality of needing a persistent, asynchronous file watcher.

2.1. The Components

agent-engram-shim (The MCP Interface)

Role: A highly ephemeral, incredibly lightweight executable invoked directly by the GitHub Copilot CLI via its plugin configuration (.mcp.json).

Protocol: Communicates with Copilot via standard input/output (stdio) using standard MCP JSON-RPC.

Lifecycle: Boots instantly on a Copilot prompt, acts as a transparent proxy to forward the request to the daemon, and dies immediately after writing the final response to stdout. Its fast startup time is critical to prevent perceived latency in the Copilot chat UI.

agent-engram-daemon (The Brain & File Watcher)

Role: A persistent, detached background process (ideally utilizing an async runtime like Tokio) responsible for continuous filesystem observation, data ingestion, chunking, indexing, and serving queries.

Protocol: Listens for queries from the shim via a local Unix Domain Socket (UDS) or Named Pipe.

Lifecycle: Dynamically spawned by the shim (if not already running). Runs silently in the background to capture IDE auto-saves and background git operations, and gracefully self-terminates after a predefined period of inactivity (TTL) to conserve system battery and memory.

The Embedded Database (SurrealDB)

Role: The local storage engine, chosen for its ability to unify vector embeddings (for semantic search of code logic) and graph relationships (for mapping ASTs, imports, and cross-file dependencies) in a single embedded instance.

Location: Strictly confined to <workspace_root>/.engram/db/.

3. Directory Layout

The entire system is self-contained within the root of the active workspace. The .engram directory should be added to the project's .gitignore to prevent committing the compiled binaries, large database files, and ephemeral runtime sockets. However, behavioral files (like .agent.md and *.skill.md) may be committed if team-wide agent behavior standardization is desired.

```text
<workspace_root>/
├── .engram/
│   ├── bin/
│   │   ├── agent-engram-shim        # The lightweight executable referenced in .mcp.json
│   │   └── agent-engram-daemon      # The heavy-lifting background worker
│   ├── db/
│   │   ├── vectors.db               # Embedded vector index for semantic queries
│   │   └── graph.db                 # Embedded graph index for dependency mapping
│   ├── run/
│   │   ├── engram.sock              # Unix Domain Socket for fast shim-to-daemon IPC
│   │   └── daemon.pid               # Process ID lockfile for concurrency control
│   ├── logs/
│   │   └── daemon.log               # Background execution logs (crucial for debugging the detached process)
│   ├── .mcp.json                    # Copilot Plugin config (points to the shim)
│   ├── engram.agent.md              # Copilot behavioral instructions
│   └── skills/
│       └── engram.skill.md          # Specific instructions for using MCP tools
├── src/                             # Normal workspace code
└── package.json                     # Normal workspace files
```

4. Lifecycle & Process Management

4.1. The "Cold Start" Proxy Flow

When a user queries Copilot in a workspace that has been dormant:

Copilot CLI invokes agent-engram-shim via stdio.

The shim checks for the existence and health of <workspace>/.engram/run/engram.sock.

If the socket is missing or dead (Connection Refused):

The shim must attempt to acquire an exclusive OS-level file lock (e.g., using flock on Unix) on daemon.pid before spawning the process. This prevents severe race conditions where multiple rapid Copilot prompts cause multiple shims to spawn redundant, conflicting daemons simultaneously.

If the lock is successfully acquired, the shim executes agent-engram-daemon as a detached child process (using std::process::Command with platform-specific detachment flags like setsid on Unix or CREATE_NO_WINDOW on Windows).

The shim waits, polling the filesystem with an exponential backoff strategy (e.g., 50ms, 100ms, 200ms) for a maximum of 2 seconds for the .sock file to be created and bound by the new daemon.

The shim opens a connection to the socket, forwards the raw MCP JSON-RPC payload, awaits the response, and writes the output to stdout. If the socket isn't available after the 2-second timeout, the shim must explicitly return a standard JSON-RPC error format to stdout. This ensures the LLM knows the tool execution failed rather than hanging the UI indefinitely.

The shim gracefully exits, releasing any held resources.

4.2. Daemon Execution & File Watching

Once the daemon is successfully running in the background:

It binds to the local IPC socket to serve incoming MCP tool calls (e.g., store_memory, retrieve_context).

It instantiates a filesystem watcher (using the Rust notify crate) scoped strictly to <workspace_root>, deliberately ignoring high-noise directories like .engram/, .git/, and node_modules/.

Debouncing Logic: Modern IDEs frequently trigger dozens of file save events per second (e.g., "save on type"). On receiving Create, Modify, or Delete events, the daemon must implement a debounce buffer (typically 500ms to 1s). Once the file settles, the daemon parses the updated file, generates new vector embeddings, updates graph edges representing dependencies, and commits the transaction to the local SurrealDB instance.

4.3. The Idle Timeout (TTL)

To prevent "zombie" processes accumulating across inactive projects and draining laptop battery life, the daemon implements a strict Time-To-Live (TTL) mechanism.

The daemon maintains an internal last_active_timestamp in memory.

This timestamp is updated whenever:

A filesystem event is processed from the watcher.

A query is received over the UDS from the shim.

A background async task routinely checks the current system time against the last_active_timestamp every 5 minutes.

If the difference exceeds the configured TTL (Default: 4 hours), the daemon begins its shutdown sequence:

Data Integrity: It forces a flush of all pending database Write-Ahead Logs (WALs) and active transactions to disk, preventing corruption.

Cleanup: It safely deletes <workspace_root>/.engram/run/engram.sock and daemon.pid to signal to future shims that a cold start is required.

Termination: It executes a clean, graceful exit (std::process::exit(0)).

5. Communication Protocol

5.1. Copilot ↔ Shim (Standard MCP)

Standard JSON-RPC 2.0 over stdio. The shim acts strictly as a transparent, high-throughput proxy. It does not waste CPU cycles parsing or validating the complex MCP schema; it merely forwards the raw byte stream directly to the socket, ensuring minimal overhead.

5.2. Shim ↔ Daemon (Local IPC)

Communication occurs over a Unix Domain Socket (Linux/macOS) or a Named Pipe (Windows).

Payload: The raw JSON-RPC bytes received from the Copilot CLI.

Why UDS/Named Pipes? This transport layer bypasses the system's network stack entirely. This prevents port collisions (allowing developers to run 20 different workspaces simultaneously without conflict) and eliminates irritating OS firewall prompts.

Security: The daemon must explicitly set strict file permissions (e.g., chmod 0600 on Unix) when creating the .sock file to actively enforce OS-level security. This ensures only the local, authenticated user account executing the Copilot CLI can read or write to the memory database, protecting proprietary source code on shared machines.

6. Plugin Configuration Integration

To hook this architecture into the GitHub Copilot CLI, the .mcp.json file inside the .engram directory will be configured to execute the local shim. The CLI handles resolving the ${workspaceFolder} variable at runtime:

```json
{
  "mcpServers": {
    "agent-engram": {
      "command": "./bin/agent-engram-shim",
      "args": [],
      "cwd": "${workspaceFolder}/.engram"
    }
  }
}
```

The accompanying .agent.md file will dictate strict behavioral rules. Because LLMs are non-deterministic, these instructions provide the necessary triggers, ensuring the LLM leverages the context effectively rather than guessing when to use the tools:

```markdown
# Agent Engram Directives

@description: You are an expert development agent equipped with an autonomous file-system memory and dependency graph.

1. **Information Retrieval:** Always query the `agent-engram` server using the `retrieve_context` tool *before* answering questions about workspace architecture, project history, or cross-file dependencies. Do not rely on your base training data for project specifics.
2. **Knowledge Persistence:** If the user asks you to "remember" a specific design decision, architectural pattern, or refactoring plan, you must use the `store_memory` tool to explicitly save this insight to the local graph database so it is available in future sessions.
```

- The .agent.md file will be the primary place to encode the "personality" and behavioral triggers for the agent, while the .mcp.json strictly defines how the shim is executed. This separation of concerns allows for maximum flexibility in tuning agent behavior without risking the stability of the underlying execution architecture.
- Incorporate necessary hooks to ensure that the agent can leverage the full power of the embedded SurrealDB instance, including both vector and graph queries, to provide rich, context-aware responses that are grounded in the actual state of the codebase.
- By embedding the entire system within the workspace, we ensure that the agent's memory is as portable and context-bound as the source code itself, enabling seamless collaboration and knowledge sharing across teams without risking cross-contamination of context between unrelated projects.
- This architecture not only addresses the technical challenges of maintaining a persistent, real-time memory system for GitHub Copilot agents but also sets the stage for future enhancements, such as multi-agent collaboration, advanced reasoning capabilities, and integration with external knowledge sources, all while maintaining strict context isolation and minimal resource overhead.
- The next steps involve implementing the shim and daemon binaries, setting up the SurrealDB instance, and rigorously testing the entire flow to ensure reliability, performance, and security in real-world development environments.
- Must include comprehensive logging and error handling, especially in the daemon, to facilitate debugging of the detached process and ensure that any issues with file watching, database operations, or IPC communication are properly surfaced to the user through the shim's responses.
- Must include a configuration file (e.g., .engram/settings.json) to allow users to customize parameters such as the TTL duration, debounce timing, folders to be monitored by the daemon including folder depth and file extension wildcarding.
- Must include plugin installer and uninstaller similar to how Spec-Kit and Beads are installed to automate the setup and teardown of the .engram directory structure, database initialization, and plugin configuration in .mcp.json, ensuring a smooth onboarding experience for users. Must include install, update, and reinstall for clean install in case of corruption or misconfiguration or breaking changes to the plugin architecture.
- Need ability to also track git commit numbers to changes in the repo through a graph representation with actual code and text snippets to enable faster change detection and search during agentic adversarial code reviews.
- Need to consider tradeoff of tracking changes by branch, tag, or commit hash and how to represent this in the graph database for efficient querying. This will allow agents to quickly understand the evolution of the codebase and identify when specific changes were introduced, which is crucial for effective code reviews and debugging.  Or, is this not necessary since the daemon will be watching the file system and updating the graph in real time, thus providing an up-to-date representation of the codebase without needing to explicitly track git commits? This is a design decision that needs to be carefully considered based on the specific use cases and requirements of the agents using the memory system.
- Need to consider how to handle large repositories with thousands of files and complex dependency graphs, and whether additional optimizations or sharding strategies are needed for the SurrealDB instance to maintain performance and responsiveness for the agents.
- Need to consider how to handle edge cases such as file renames, moves, and deletions in the file watcher and how to reflect these changes accurately in the graph database to maintain the integrity of the memory system.
- Need to consider how to handle concurrent access to the memory system from multiple agents or processes, and whether additional locking or synchronization mechanisms are needed to prevent race conditions and ensure data integrity in the SurrealDB instance.
- Need to consider how to handle backup and recovery of the memory database in case of corruption or data loss, and whether additional tools or processes are needed to facilitate this for users.  Or is it fine to just rehydrate the database from the file system on startup since the daemon will be watching the file system and updating the graph in real time, thus providing an up-to-date representation of the codebase without needing to explicitly backup and recover the database? This is a design decision that needs to be carefully considered based on the specific use cases and requirements of the agents using the memory system.


### Feature: 005-lifecycle-observability

Lifecycle Observability & Advanced Workflow Enforcement for Agent-Engram

1. Overview

This specification defines feature enhancements for agent-engram that introduce advanced lifecycle management, comprehensive workspace synchronization, state versioning, structured graph querying, hierarchical workflow groupings, and daemon observability. These capabilities extend the existing SurrealDB-backed daemon architecture, the hook-enforced shim, and the real-time file-syncing engine to provide robust state lifecycle control, deep observability, and strict workflow enforcement.

2. Feature Matrix

| Feature Category | Current State | Target Enhancement |
|---|---|---|
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

