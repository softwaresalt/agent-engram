# Implementation Plan: Lifecycle Observability & Advanced Workflow Enforcement

**Branch**: `005-lifecycle-observability` | **Date**: 2026-03-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-lifecycle-observability/spec.md`

## Summary

This feature introduces dependency-gated task execution, comprehensive daemon observability with optional OTLP export, an append-only event ledger with rollback capability, a sandboxed read-only graph query interface, hierarchical collection groupings, and daemon reliability hardening. The implementation extends the existing SurrealDB graph model, tracing infrastructure, and MCP tool dispatch to deliver these capabilities while preserving single-binary simplicity.

## Technical Context

**Language/Version**: Rust 2024 edition, `rust-version = "1.85"`, stable toolchain
**Primary Dependencies**: axum 0.7, tokio 1 (full), surrealdb 2 (embedded surrealkv), serde 1, tracing 0.1, tracing-subscriber 0.3, tracing-opentelemetry (new, behind `otlp-export` feature flag), opentelemetry-otlp (new, behind `otlp-export` feature flag)
**Storage**: SurrealDB 2 embedded (surrealkv backend), per-workspace namespace via SHA-256 path hash
**Testing**: cargo test, proptest 1, tokio-test 0.4 — TDD required with contract, integration, unit, and property tests
**Target Platform**: Windows, macOS, Linux (localhost daemon, single binary)
**Project Type**: Single Rust binary with library crate
**Performance Goals**: <50ms blocker chain evaluation, <10ms event logging, <100ms health report, <50ms sandboxed query (up to 1000 rows)
**Constraints**: <100MB idle RAM, <500MB under load, localhost-only binding, `#![forbid(unsafe_code)]`
**Scale/Scope**: <1000 tasks per workspace, <500 events in rolling ledger (configurable), <10 concurrent clients

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
| --------- | ------ | ----- |
| I. Rust Safety First | ✅ PASS | All code uses `Result<T, EngramError>`, no `unsafe`, clippy pedantic enabled |
| II. Async Concurrency | ✅ PASS | All new tools are async, shared state via `Arc<AppState>` with `RwLock`, channel-based watcher events |
| III. Test-First Development | ✅ PASS | Contract tests for all new MCP tools, integration tests for gate enforcement and rollback, property tests for event serialization |
| IV. MCP Protocol Compliance | ✅ PASS | All new capabilities exposed as MCP tools via JSON-RPC dispatch, workspace scoping enforced |
| V. Workspace Isolation | ✅ PASS | Event ledger, collections, and queries all scoped to active workspace database namespace |
| VI. Git-Friendly Persistence | ⚠️ PASS (with justified exception) | Event ledger stored in SurrealDB only (not dehydrated to `.engram/` files) since events are transient operational data, not user-editable state — see research.md §Research 2 for rationale. Collections dehydrate to `.engram/collections.md` |
| VII. Observability & Debugging | ✅ PASS | This feature directly enhances observability with structured trace spans and health reporting |
| VIII. Error Handling & Recovery | ✅ PASS | Rollback validation before execution, descriptive errors for gate violations, graceful degradation for watcher failures |
| IX. Simplicity & YAGNI | ✅ PASS | OTLP export behind feature flag, rolling retention avoids unbounded growth, sandboxed queries use SurrealQL natively (no custom parser) |

**Note on Network Security**: OTLP trace export (FR-008, behind `otlp-export` feature flag) initiates outbound-only gRPC connections to a configured collector endpoint. This does not expose any inbound listening port and is distinct from the constitution's "Bind to 127.0.0.1 only" restriction, which governs the daemon's inbound service port.

### Documentation (this feature)

```text
specs/005-lifecycle-observability/
├── spec.md              # Feature specification (complete)
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (MCP tool contracts)
├── SCENARIOS.md         # Behavioral matrix (speckit.behavior output)
└── tasks.md             # Task breakdown (speckit.tasks output)
```

### Source Code (repository root)

```text
src/
├── models/
│   ├── event.rs         # NEW: Event entity, EventKind enum
│   ├── collection.rs    # NEW: Collection entity
│   └── [existing]       # task.rs, graph.rs, etc. (modified for gate enforcement)
├── db/
│   ├── schema.rs        # MODIFIED: event table, collection table definitions
│   └── queries.rs       # MODIFIED: event ledger CRUD, collection queries, blocker chain evaluation
├── services/
│   ├── gate.rs          # NEW: Dependency gate evaluation logic
│   ├── event_ledger.rs  # NEW: Event recording, rollback, retention pruning
│   ├── dehydration.rs   # MODIFIED: collection dehydration
│   └── hydration.rs     # MODIFIED: collection hydration
├── tools/
│   ├── lifecycle.rs     # MODIFIED: enhanced daemon health, gate-aware status
│   ├── write.rs         # MODIFIED: gate enforcement on update_task, event recording on all writes
│   └── read.rs          # MODIFIED: sandboxed query tool, collection retrieval, event history
├── daemon/
│   └── [existing]       # ttl.rs, watcher.rs (modified for observability spans)
└── server/
    ├── state.rs         # MODIFIED: latency tracking, connection metrics
    └── observability.rs # NEW: OTLP export setup (behind feature flag)

tests/
├── contract/
│   ├── gate_test.rs     # NEW: blocker gate enforcement contract tests
│   ├── event_test.rs    # NEW: event ledger contract tests
│   ├── query_test.rs    # NEW: sandboxed query contract tests
│   └── collection_test.rs # NEW: collection CRUD contract tests
├── integration/
│   ├── gate_integration_test.rs    # NEW: transitive blocking, cycle detection
│   ├── rollback_test.rs            # NEW: event rollback round-trips
│   └── reliability_test.rs         # NEW: concurrent client stress tests
└── unit/
    └── proptest_events.rs          # NEW: event serialization round-trips
```

**Structure Decision**: Follows existing single-project layout. New functionality is added as new modules within the established `models/`, `services/`, `tools/`, and `tests/` directories. No new top-level directories needed.

## Complexity Tracking

> No constitution violations. All features align with existing principles.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Event ledger excluded from `.engram/` dehydration (Principle VI) | Events are transient operational data (rollback snapshots, audit trail) that grow rapidly and are not human-editable. Dehydrating would produce large, noisy files that pollute Git diffs without benefit. | Dehydrating to `.engram/events.md` was rejected because events are append-only operational data, not user-curated state. Their volume (up to 500 per workspace) would dominate `.engram/` diffs without providing human-reviewable value. |

## Phase 0: Research Summary

See [research.md](research.md) for full findings. Key decisions:

1. **Dependency gate evaluation**: Recursive CTE query in SurrealQL to walk the `depends_on` graph transitively. Evaluated in `update_task` before applying transition.
2. **Event ledger storage**: SurrealDB `event` table with rolling retention (default 500 events). NOT dehydrated to `.engram/` files — events are transient operational data.
3. **Trace export**: `tracing-opentelemetry` + `opentelemetry-otlp` behind `otlp-export` Cargo feature flag. Local JSON logs remain the always-on default.
4. **Sandboxed queries**: SurrealQL executed via `db.query()` with a read-only transaction mode. Statement parsing validates no write keywords before execution.
5. **Collection model**: New `collection` table + `contains` relation edge. Recursive traversal via SurrealQL `->contains->` graph path queries.
6. **Rollback implementation**: Reverse-apply events by restoring `previous_value` snapshots. Operator-only by default (`allow_agent_rollback = false`).

## Phase 1: Design Summary

See [data-model.md](data-model.md) for entity schemas and [contracts/](contracts/) for MCP tool contracts.

### New MCP Tools

| Tool | Type | Description |
| ---- | ---- | ----------- |
| `query_graph` | read | Execute a sandboxed read-only query against the workspace graph |
| `get_event_history` | read | Retrieve recent events from the ledger with optional filtering |
| `rollback_to_event` | write | Roll workspace state back to a specific event (operator-gated) |
| `create_collection` | write | Create a named collection (epic/workflow grouping) |
| `add_to_collection` | write | Add tasks or sub-collections to a collection |
| `remove_from_collection` | write | Remove tasks or sub-collections from a collection |
| `get_collection_context` | read | Recursively retrieve all tasks and context within a collection |
| `get_health_report` | read | Extended daemon health with latency percentiles, watcher status, memory |

### Modified MCP Tools

| Tool | Change |
| ---- | ------ |
| `update_task` | Gate enforcement: check blocker chain before applying status transition |
| `add_dependency` | Cycle detection: validate no circular dependencies before creating edge |
| `get_daemon_status` | Extended with latency metrics and watcher health |
| `get_workspace_status` | Extended with event count and collection count |

### New Configuration Parameters

| Env Var | CLI Flag | Default | Description |
| ------- | -------- | ------- | ----------- |
| `ENGRAM_EVENT_LEDGER_MAX` | `--event-ledger-max` | `500` | Maximum events retained in rolling ledger |
| `ENGRAM_ALLOW_AGENT_ROLLBACK` | `--allow-agent-rollback` | `false` | Whether agents can invoke rollback |
| `ENGRAM_QUERY_TIMEOUT_MS` | `--query-timeout-ms` | `5000` | Sandboxed query execution timeout |
| `ENGRAM_QUERY_ROW_LIMIT` | `--query-row-limit` | `1000` | Maximum rows returned by sandboxed queries |
| `ENGRAM_OTLP_ENDPOINT` | `--otlp-endpoint` | (none) | OTLP collector endpoint (requires `otlp-export` feature) |
