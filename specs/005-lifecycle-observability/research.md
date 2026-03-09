# Research: Lifecycle Observability & Advanced Workflow Enforcement

**Feature**: 005-lifecycle-observability
**Date**: 2026-03-09

## Research 1: Dependency Gate Evaluation Strategy

**Decision**: Use SurrealDB recursive graph traversal to evaluate the full upstream blocker chain during `update_task`.

**Rationale**: SurrealDB natively supports graph traversal queries (e.g., `SELECT <-depends_on<-task WHERE type = 'hard_blocker'`). A recursive query walks the entire upstream chain in a single database round-trip, avoiding the N+1 problem of iterative queries. The existing `depends_on` relation table already stores `hard_blocker` and `soft_dependency` edge types.

**Alternatives considered**:
- **Application-level BFS/DFS**: Would require loading all edges into memory and traversing in Rust. Rejected because SurrealDB's native graph engine is faster and avoids memory overhead for large graphs.
- **Materialized blocker view**: Pre-compute transitive closure on edge creation. Rejected because it adds write-path complexity and the closure must be recomputed on every edge change. Read-time evaluation is fast enough (<50ms) for graphs under 1000 nodes.

**Implementation approach**: Add a `check_blockers(task_id) -> Vec<BlockerInfo>` function to `Queries` that executes a recursive traversal. Call it in `update_task` before applying any transition to `in_progress`. Return `BlockerInfo { task_id, title, status }` for each unresolved upstream blocker.

**Cycle detection**: On `add_dependency`, execute a path-existence query (`SELECT * FROM task:A<-depends_on<-* WHERE id = task:B`) to detect if creating the new edge would form a cycle. Reject with `EngramError::Task(CyclicDependency { ... })`.

## Research 2: Event Ledger Architecture

**Decision**: Append-only `event` table in SurrealDB with rolling retention (default 500 events, configurable).

**Rationale**: The event ledger records state changes for rollback and audit. Storing in SurrealDB keeps the persistence layer unified (Principle VI: single persistence layer). Rolling retention prevents unbounded growth while providing a sufficient rollback window for typical agent sessions. Events are NOT dehydrated to `.engram/` files because they are transient operational data — not human-editable state that benefits from Git tracking.

**Alternatives considered**:
- **Separate file-based WAL**: Write events to a `.engram/events.log` file. Rejected because it would require a custom parser and violate the unified database principle.
- **Unbounded retention**: Keep all events forever. Rejected because a workspace with heavy agent activity could accumulate thousands of events per session, consuming significant database storage with diminishing rollback value.
- **Per-entity versioning**: Store version history on each entity. Rejected because it complicates every model and makes cross-entity rollback difficult.

**Rollback mechanism**: Each event stores `previous_value` (serialized JSON of the entity before the change) and `new_value` (after). Rollback reverses events in descending order by timestamp, restoring `previous_value` for each affected entity. Before executing, the rollback validator checks that no target entity has been deleted (if so, reports conflict).

**Pruning**: After each event insert, count total events. If count exceeds `event_ledger_max`, delete the oldest events to bring count back to the limit. This is a simple `DELETE FROM event ORDER BY created_at ASC LIMIT (count - max)`.

## Research 3: Trace Export via OpenTelemetry

**Decision**: Use `tracing-opentelemetry` bridge + `opentelemetry-otlp` exporter, gated behind `otlp-export` Cargo feature flag.

**Rationale**: The daemon already uses `tracing` for structured logging. The `tracing-opentelemetry` crate provides a zero-friction bridge that exports existing tracing spans to an OpenTelemetry collector. OTLP over gRPC is the industry standard supported by Jaeger, Grafana Tempo, Datadog, and most APM tools. Gating behind a feature flag keeps the default binary lean (no gRPC dependency).

**Alternatives considered**:
- **Custom JSON exporter**: Write spans to `.engram/logs/` as JSON files. Rejected because it requires a custom file rotation system and doesn't integrate with existing APM tools.
- **Prometheus metrics endpoint**: Expose `/metrics` in Prometheus format. Rejected because structured traces provide richer information than metrics alone, and the daemon already binds to localhost making metrics scraping complex.
- **Always-on OTLP**: Include gRPC dependency in the default binary. Rejected because it significantly increases binary size and compile time for a capability most users won't need.

**Implementation approach**:
1. Add `tracing-opentelemetry` and `opentelemetry-otlp` as optional dependencies under `[features] otlp-export`.
2. In `lib.rs::init_tracing`, check if the feature is compiled in and `ENGRAM_OTLP_ENDPOINT` is set.
3. If so, create an OTLP exporter layer and add it to the tracing subscriber stack alongside the existing fmt layer.
4. Spans are automatically exported — no changes needed to existing `tracing::instrument` annotations.

## Research 4: Sandboxed Query Interface

**Decision**: Accept SurrealQL strings, parse to validate read-only intent, execute via `db.query()` with timeout and row limit.

**Rationale**: SurrealQL is already the native query language of the embedded database. Exposing it directly (with sandboxing) gives agents maximum analytical power without building a custom DSL. The sandboxing layer validates that queries contain no write keywords (INSERT, UPDATE, DELETE, CREATE, DEFINE, REMOVE, RELATE) before execution.

**Alternatives considered**:
- **Custom DSL with parser**: Define a simplified query language. Rejected because it requires maintaining a parser, limits analytical capability, and duplicates SurrealQL functionality.
- **Pre-built query templates**: Offer a fixed set of parameterized queries. Rejected because it's too restrictive — agents need ad-hoc analytical capability to answer novel structural questions.
- **Raw db.query() without validation**: Trust agent input. Rejected because a hallucinating agent could issue destructive queries.

**Sandboxing strategy**:
1. **Keyword blocklist**: Reject queries containing `INSERT`, `UPDATE`, `DELETE`, `CREATE`, `DEFINE`, `REMOVE`, `RELATE`, `KILL`, `SLEEP`, `THROW` (case-insensitive, word-boundary matched).
2. **Timeout**: Execute with a configurable timeout (default 5000ms). Cancel and return error on timeout.
3. **Row limit**: Append `LIMIT {query_row_limit}` to SELECT queries that don't already have a LIMIT clause.
4. **Namespace scoping**: Query executes against the already-connected workspace database — no namespace escape possible.

## Research 5: Collection Model Design

**Decision**: New `collection` table + `contains` relation edge, using SurrealDB's native graph traversal for recursive retrieval.

**Rationale**: The existing graph schema already supports relation tables (`depends_on`, `implements`, `relates_to`). Collections fit naturally as nodes with `contains` edges to tasks and sub-collections. SurrealDB's `->contains->*` recursive traversal handles arbitrary nesting depth in a single query.

**Alternatives considered**:
- **Flat tags**: Add a `collection` field to tasks. Rejected because it doesn't support hierarchy, and retrieving "all tasks in a collection" requires scanning all tasks.
- **Materialized path**: Store the full collection path (e.g., "epic:A/sub:B/task:C") as a string field. Rejected because it's fragile when collections are reorganized and doesn't leverage SurrealDB's graph capabilities.
- **Separate table per collection**: Create dynamic tables. Rejected because it complicates schema management and doesn't work with SurrealDB's static schema model.

**Dehydration**: Collections are dehydrated to `.engram/collections.md` as YAML frontmatter + markdown. Each collection gets a `## collection:{id}` section listing its contained task IDs and sub-collection IDs. This format is human-readable, Git-mergeable, and follows the established `tasks.md` pattern.

## Research 6: Reliability Hardening

**Decision**: Focus on connection resilience, concurrent access safety, and crash recovery within the existing IPC architecture.

**Rationale**: The backlog's "Reliability Gate" section identifies that the daemon must prove reliable function in active workspaces before advanced features are trusted. This is not a fundamental architecture change — it's hardening the existing IPC server, state management, and persistence paths.

**Key areas**:
1. **Connection resilience**: Ensure IPC server handles client disconnect/reconnect without leaking state. Add connection health monitoring spans.
2. **Concurrent access**: Verify `RwLock` usage prevents deadlocks under concurrent tool calls. Add stress tests with 10 simultaneous clients.
3. **Crash recovery**: Existing atomic write-to-temp-then-rename protects `.engram/` files. Event ledger in SurrealDB survives crashes via surrealkv's WAL. Validate with kill-during-write tests.
4. **Integration templates**: Create `.engram/agent-templates/` with MCP tool usage examples for Claude Code, GitHub Copilot, and Cursor agents.
