# Feature Specification: Lifecycle Observability & Advanced Workflow Enforcement

**Feature Branch**: `005-lifecycle-observability`  
**Created**: 2026-03-09  
**Status**: Draft  
**Input**: User description: "Lifecycle Observability and Advanced Workflow Enforcement for Agent-Engram — advanced lifecycle management, comprehensive workspace synchronization, state versioning, structured graph querying, hierarchical workflow groupings, and daemon observability"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Dependency-Gated Task Execution (Priority: P1)

As an AI coding assistant working through a multi-step feature implementation, I need the memory service to enforce task dependency ordering automatically, so that I cannot start work on a task whose prerequisites are incomplete — preventing wasted effort, hallucinated out-of-order execution, and broken downstream workflows.

When the assistant attempts to modify a task that has unresolved blocking dependencies, the system intercepts the operation and returns a clear, actionable error explaining which upstream tasks must be completed first. This forces the assistant to redirect attention to the correct task, drastically reducing wasted tokens and preventing cascading failures.

**Why this priority**: Without dependency enforcement, agents routinely attempt tasks out of order, producing work that must be discarded when prerequisites are later completed differently. This is the single highest-impact improvement for agent productivity — every other feature assumes tasks are executed in the correct order.

**Independent Test**: Can be fully tested by creating two tasks with a blocking dependency, attempting to transition the blocked task to `in_progress`, and verifying the system rejects the transition with a descriptive error citing the blocker. Delivers immediate value: agents stop thrashing on blocked work.

**Acceptance Scenarios**:

1. **Given** task B depends on task A via a `hard_blocker` edge and task A has status `todo`, **When** an agent attempts to transition task B to `in_progress`, **Then** the system rejects the transition with an error message naming task A as the unresolved blocker.
2. **Given** task B depends on task A via a `hard_blocker` edge and task A has status `done`, **When** an agent transitions task B to `in_progress`, **Then** the transition succeeds normally.
3. **Given** a chain of three tasks A → B → C with `hard_blocker` edges, **When** an agent attempts to transition task C to `in_progress` while task A is still `todo`, **Then** the system rejects the transition citing the entire upstream chain (both A and B) as unresolved.
4. **Given** task B depends on task A via a `soft_dependency` edge and task A has status `todo`, **When** an agent transitions task B to `in_progress`, **Then** the transition succeeds but includes a warning that task A is an incomplete soft dependency.

---

### User Story 2 - Daemon Performance Observability (Priority: P1)

As a developer running the engram daemon across multiple workspaces, I need visibility into daemon performance metrics — wake/sleep cycles, query latency, file watcher throughput, and memory consumption — so I can diagnose bottlenecks and verify the daemon is operating correctly during extended unattended sessions.

The daemon emits structured trace spans for all significant operations: startup, shutdown, tool call processing, file event handling, database queries, and hydration/dehydration cycles. These traces are available both in local log files and optionally exported to external observability collectors for aggregation.

**Why this priority**: The daemon runs as an unattended background service for hours or days. Without performance observability, diagnosing slow queries, stalled file watchers, or memory leaks requires reproducing the exact scenario. Structured traces are the primary diagnostic tool for production issues.

**Independent Test**: Can be fully tested by starting the daemon, performing several tool calls and triggering file events, then inspecting the structured log output for the expected trace spans with timing data. Delivers immediate value: operators can verify daemon health and diagnose performance issues without restarting.

**Acceptance Scenarios**:

1. **Given** a running daemon with observability enabled, **When** a tool call is processed, **Then** the structured log contains a span with the tool name, execution duration, workspace ID, and success/failure status.
2. **Given** a running daemon with a file watcher active, **When** a file change is detected and processed, **Then** the structured log contains spans for event detection, debounce processing, and database update, each with timing data.
3. **Given** a daemon that has been idle and enters the TTL sleep cycle, **When** the daemon wakes on a new tool call, **Then** the structured log contains a span for the wake event including time-since-sleep and re-initialization duration.
4. **Given** a daemon with optional trace export configured, **When** traces are generated, **Then** spans are exported to the configured collector endpoint in addition to local log output.

---

### User Story 3 - State Event Logging and Rollback (Priority: P2)

As an AI coding assistant operating on a workspace, I need the ability to review a history of all state changes and roll back to a previous known-good state, so that corrupted or hallucinated state modifications can be undone without losing the entire workspace history.

Every discrete state change (task creation, status transition, edge addition, context storage) is recorded as an immutable event in a ledger. An authorized user or oversight agent can replay events in reverse to restore the workspace graph to a previous point in time, reverting only the affected nodes and edges.

**Why this priority**: Agents operating unattended can produce cascading state corruption through hallucinated updates. Without rollback capability, the only recovery path is manual file editing or full workspace reset, both of which lose valuable accumulated context.

**Independent Test**: Can be fully tested by creating a task, modifying it several times, then issuing a rollback to a specific event, and verifying the task returns to its state at that point. Delivers value: safe undo for any state corruption without manual intervention.

**Acceptance Scenarios**:

1. **Given** a workspace with an event ledger containing 10 recorded state changes, **When** the user requests a rollback to event 7, **Then** events 8–10 are reversed in order and the workspace state matches the state after event 7.
2. **Given** a workspace with a task that was created (event 5) and later modified (events 6, 8), **When** a rollback to event 5 occurs, **Then** the task reflects its original creation state and the modifications are undone.
3. **Given** a rollback that would remove a dependency edge between two tasks, **When** the rollback is applied, **Then** both the edge record and any derived blocking state are reverted.
4. **Given** a request to rollback beyond the oldest available event, **When** the rollback is attempted, **Then** the system rejects it with an error explaining the earliest available rollback point.

---

### User Story 4 - Sandboxed Graph Query Interface (Priority: P2)

As an AI coding assistant, I need the ability to perform complex, exact-match graph queries across my workspace memory — beyond what semantic search provides — so I can answer precise structural questions like "which tasks block this task?" or "what are all in-progress tasks assigned to this agent?"

The system exposes a read-only query interface that allows structured graph traversals and filtered lookups. All queries are sandboxed: they cannot modify data, access other workspaces, or execute arbitrary operations. The query interface provides the analytical power to navigate the full task-file-context graph.

**Why this priority**: Semantic search is excellent for fuzzy retrieval but cannot answer precise structural questions. Agents frequently need to understand dependency chains, filter by exact status, or traverse relationships — capabilities that require structured querying. This unlocks a new class of agent self-awareness about workspace state.

**Independent Test**: Can be fully tested by populating a workspace with tasks, dependencies, and labels, then issuing read-only queries and verifying correct results. Delivers value: agents can answer precise structural questions about workspace state without relying on file parsing.

**Acceptance Scenarios**:

1. **Given** a workspace with 5 tasks in various statuses, **When** an agent queries for all tasks with status `in_progress`, **Then** the query returns exactly the matching tasks with their full details.
2. **Given** a task with three outgoing `hard_blocker` edges, **When** an agent queries for all tasks blocked by this task, **Then** the query returns exactly the three downstream tasks.
3. **Given** a query that attempts a write operation (INSERT, UPDATE, DELETE), **When** the query is submitted, **Then** the system rejects it with a clear error explaining that only read operations are permitted.
4. **Given** a query with valid syntax but referencing a table that does not exist, **When** the query is submitted, **Then** the system returns an empty result set without exposing internal schema details.

---

### User Story 5 - Hierarchical Workflow Groupings (Priority: P3)

As a developer managing a large feature with dozens of related tasks spanning design, implementation, and testing, I need the ability to group tasks into named collections (also known as epics or workflows), so that an AI assistant can hydrate all relevant context for a feature in a single operation rather than hunting for individual tasks.

A collection node aggregates related tasks, files, and contexts under a named hierarchy. When the assistant requests context for a collection, the system recursively fetches all contained sub-tasks, their associated files, and relevant context entries — assembling a cohesive prompt payload that covers the entire feature scope.

**Why this priority**: Without grouping, agents must discover related tasks through search or explicit references, leading to fragmented context and missed dependencies. Collections solve the "context stuffing" problem by providing curated, feature-scoped views of workspace state. This becomes critical at scale but is not required for basic workflow enforcement.

**Independent Test**: Can be fully tested by creating a collection, adding tasks and sub-tasks to it, then requesting the collection's context and verifying it returns all contained items recursively. Delivers value: feature-scoped context retrieval in a single operation.

**Acceptance Scenarios**:

1. **Given** a collection "Feature X" containing 5 tasks, **When** an agent requests the collection's context, **Then** the system returns all 5 tasks with their descriptions, statuses, and associated file references.
2. **Given** a collection with nested sub-collections (e.g., "Design" and "Implementation" under "Feature X"), **When** an agent requests the parent collection, **Then** the system recursively includes tasks from all nested sub-collections.
3. **Given** a task that belongs to two different collections, **When** either collection is queried, **Then** the task appears in both result sets.
4. **Given** a collection with 50 tasks, **When** an agent requests the collection with a filter for only `in_progress` tasks, **Then** only matching tasks are returned, reducing payload size.

---

### User Story 6 - Reliable Daemon Availability (Priority: P1)

As a developer using AI coding assistants in active workspaces, I need the engram daemon to demonstrate reliable availability — starting on demand, maintaining stable connections during active use, and recovering gracefully from interruptions — so that the memory service can be trusted as a core part of the development workflow.

The daemon must prove it can sustain multi-hour sessions without dropped connections, handle concurrent tool calls without data corruption, survive workspace switches and IDE restarts, and provide clear diagnostics when problems occur. Tool selection guidance and integration templates ensure agents actively use engram as their primary context source rather than falling back to file search.

**Why this priority**: All other features are valueless if the daemon is unreliable. The current implementation has not demonstrated reliable availability and connection stability in active workspaces. This reliability gate must be cleared before advanced features can be trusted.

**Independent Test**: Can be fully tested by running the daemon in an active workspace for an extended session, performing concurrent tool calls, triggering file events, and verifying zero dropped connections and consistent state. Delivers value: the daemon becomes trustworthy enough to serve as the foundation for all other features.

**Acceptance Scenarios**:

1. **Given** a daemon started in a workspace, **When** 100 sequential tool calls are issued over a 2-hour period, **Then** all calls receive correct responses with zero timeouts or connection errors.
2. **Given** 3 concurrent AI assistants connected to the same workspace daemon, **When** all three issue tool calls simultaneously, **Then** all calls are processed correctly without data corruption or deadlocks.
3. **Given** a daemon that has been idle for 30 minutes and then receives a tool call, **When** the call arrives, **Then** the daemon responds within 2 seconds, including any re-initialization time.
4. **Given** an IDE that restarts while the daemon is running, **When** the IDE reconnects, **Then** the daemon accepts the new connection and serves all previously stored workspace state.
5. **Given** a daemon crash during a write operation, **When** the daemon restarts, **Then** the workspace state is consistent (no half-written records) and the event ledger accurately reflects the last successful operation.

---

### Edge Cases

- What happens when a dependency chain contains a cycle (A blocks B blocks A)? The system must detect and reject cycles at edge-creation time.
- How does the system handle rollback of an event that created an entity now referenced by later events? Cascading undo must be explicit and bounded.
- What happens when the file watcher cannot keep up with rapid file changes (e.g., `git checkout` switching hundreds of files)? The debounce window should absorb bursts; degraded mode is acceptable.
- How does the system handle a sandboxed query that would scan the entire database? Query execution must have a timeout or row limit to prevent resource exhaustion.
- What happens when an agent attempts to create a collection with the same name as an existing one? The system should return a descriptive conflict error.
- How does the system behave when the event ledger grows very large (thousands of events)? Compaction or pruning of old events should be supported.

## Requirements *(mandatory)*

### Functional Requirements

**Dependency Gate Enforcement**

- **FR-001**: System MUST evaluate the complete upstream dependency chain when a task status transition is requested and reject transitions that violate `hard_blocker` constraints.
- **FR-002**: System MUST detect and reject circular dependency chains at edge-creation time, returning an error that identifies the cycle.
- **FR-003**: System MUST include a warning (not rejection) when transitioning a task that has incomplete `soft_dependency` edges.
- **FR-004**: System MUST support transitive blocking — if A blocks B and B blocks C, then C is transitively blocked by A.

**Observability**

- **FR-005**: System MUST emit structured trace spans for every tool call, including tool name, workspace ID, execution duration, and outcome.
- **FR-006**: System MUST emit structured trace spans for daemon lifecycle events: startup, shutdown, TTL expiry, and wake-from-idle.
- **FR-007**: System MUST emit structured trace spans for file watcher events: detection, debounce completion, and database write.
- **FR-008**: System MUST support configurable trace export to an external collector endpoint using OpenTelemetry Protocol (OTLP) over gRPC, gated behind a Cargo feature flag (`otlp-export`). Local structured JSON logs remain the always-available default.
- **FR-008a**: System MUST allow trace export to be enabled/disabled at runtime via configuration without recompilation (when the feature flag is compiled in).
- **FR-009**: System MUST expose a daemon health summary tool that reports uptime, query latency percentiles, file watcher status, active connections, and memory consumption.

**State Event Logging and Rollback**

- **FR-010**: System MUST record every state-modifying operation as an immutable event in an append-only ledger, including the operation type, affected entity, previous value, new value, and timestamp.
- **FR-011**: System MUST support rollback to any recorded event, reversing subsequent events in order.
- **FR-011a**: Rollback MUST be guarded by a configuration flag (`allow_agent_rollback`, default `false`). When disabled, rollback requests from agents are rejected with a descriptive error directing them to request operator intervention. When enabled, any connected client may invoke rollback.
- **FR-012**: System MUST validate rollback feasibility before execution, detecting and reporting conflicts (e.g., entities that have since been deleted).
- **FR-013**: System MUST retain the event ledger across daemon restarts via the persistence layer.
- **FR-013a**: System MUST implement a rolling retention window for the event ledger, retaining the most recent N events (configurable, default 500) and automatically pruning older entries on each write.
- **FR-013b**: System MUST expose the retention window size as a configuration parameter (`event_ledger_max`).

**Sandboxed Query Interface**

- **FR-014**: System MUST expose a read-only query interface that supports structured graph traversals and filtered lookups across the workspace graph.
- **FR-015**: System MUST reject any query that would modify data (writes, deletes, schema changes).
- **FR-016**: System MUST enforce query execution limits (timeout and row count) to prevent resource exhaustion.
- **FR-017**: System MUST scope all queries to the active workspace — cross-workspace data access is forbidden.

**Hierarchical Workflow Groupings**

- **FR-018**: System MUST support a collection entity that groups tasks under a named hierarchy, with support for nesting (sub-collections).
- **FR-019**: System MUST support recursive context retrieval for a collection, returning all tasks, associated files, and context entries within the hierarchy.
- **FR-020**: System MUST allow a task to belong to multiple collections simultaneously.
- **FR-021**: System MUST support filtering when retrieving collection contents (by status, priority, assignee).

**Reliability**

- **FR-022**: System MUST handle concurrent tool calls from multiple connected clients without data corruption or deadlocks.
- **FR-023**: System MUST use atomic write operations for all persistence, ensuring no half-written state survives a crash.
- **FR-024**: System MUST recover gracefully from unexpected shutdown, restoring consistent state from the last successful persistence checkpoint.
- **FR-025**: System MUST provide integration templates and tool selection guidance that direct AI assistants to use engram as their primary context source.

### Key Entities

- **Event**: An immutable record of a state change — captures the operation type, target entity, before/after values, timestamp, and originating client. Forms the append-only ledger for state versioning and rollback.
- **Collection**: A named grouping of tasks and sub-collections that represents a feature, workflow, or epic. Supports hierarchical nesting and cross-referencing. Tasks may belong to multiple collections. "Collection" is the canonical term used throughout this specification.
- **Trace Span**: A structured observability record capturing operation name, duration, workspace context, and outcome. Emitted for tool calls, lifecycle events, file watcher activity, and database operations.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Agents attempting to transition a blocked task receive a rejection with the specific blocker chain within 50ms, reducing wasted agent tokens by preventing out-of-order execution.
- **SC-002**: 100% of tool calls, lifecycle events, and file watcher operations are covered by structured trace spans with timing data, enabling post-hoc diagnosis of any performance issue.
- **SC-003**: State rollback to any recorded event completes successfully, restoring the workspace to the exact state at that point — verified by round-trip tests (modify → rollback → compare).
- **SC-004**: Read-only queries return correct results for all supported graph traversal patterns, with zero data modification side effects — verified by before/after state comparison.
- **SC-005**: Collection-based context retrieval returns the complete recursive contents of a hierarchy in a single operation, reducing multi-query context assembly to one call.
- **SC-006**: The daemon sustains 2+ hour active sessions with concurrent clients (3+), zero dropped connections, and consistent state — verified by extended integration testing.
- **SC-007**: Daemon health metrics are available on demand, reporting accurate uptime, latency, memory, and watcher status within 100ms.

## Clarifications

### Session 2026-03-09

- Q: What is the event ledger retention policy? → A: Rolling window — retain the most recent N events (configurable, default 500), automatically prune older entries on each write. Unbounded growth would violate single-binary simplicity; a fixed-count window balances safety with resource efficiency.
- Q: What protocol/format should trace export use? → A: OpenTelemetry Protocol (OTLP) over gRPC as the export format, behind a Cargo feature flag (`otlp-export`). OTLP is the industry standard for trace export and supported by all major APM tools. Local structured JSON logs remain the default with no feature flag required.
- Q: Who may invoke state rollback — any connected agent or only the operator? → A: Operator-only by default. Rollback is exposed as an MCP tool but guarded by a configuration flag (`allow_agent_rollback`, default `false`). When disabled, rollback requests from agents are rejected with a descriptive error directing them to request operator intervention. This prevents agents from accidentally reverting each other's work.

## Assumptions

- The existing `depends_on` relation table with `hard_blocker` and `soft_dependency` types provides the foundation for dependency gate enforcement — no new edge types are required.
- The existing `tracing` integration provides the base instrumentation; this feature extends span coverage and adds optional export, not a replacement of the tracing framework.
- Event ledger storage uses the same embedded database as all other workspace state — no additional persistence layer is introduced.
- Sandboxed queries execute against the embedded database's native query language — no custom DSL or parser is needed.
- Collection entities are stored as graph nodes in the existing database schema, using relation edges to link to contained tasks and sub-collections.
- Reliability improvements are focused on the existing daemon architecture — no fundamental transport or protocol changes are expected.

## Scope Boundaries

**In scope:**
- Dependency gate enforcement on task status transitions
- Structured trace span coverage for all daemon operations
- Optional trace export to external collectors
- Append-only event ledger with rollback capability
- Read-only sandboxed query interface
- Collection hierarchical grouping model
- Daemon reliability hardening and health reporting
- Agent integration templates and tool selection guidance

**Out of scope:**
- External tracker synchronization (Jira, Linear) — deferred to a future feature
- Real-time webhook ingestion from external services
- Visual dashboards or UI for observability data
- Multi-workspace aggregation (each workspace remains isolated)
- Custom query language or DSL (uses the database's native query capability)
