---
id: TASK-005
title: '005: Lifecycle Observability and Advanced Workflow Enforcement'
status: Done
type: feature
assignee: []
created_date: '2026-03-09'
labels:
  - feature
  - '005'
  - observability
  - workflow
  - tracing
  - reliability
milestone: m-0
dependencies:
  - TASK-001
  - TASK-004
references:
  - specs/005-lifecycle-observability/spec.md
  - src/services/gate.rs
  - src/services/code_graph.rs
  - src/db/queries.rs
  - src/tools/read.rs
  - src/tools/write.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
# Feature Specification: Lifecycle Observability & Advanced Workflow Enforcement

**Feature Branch**: `005-lifecycle-observability`  
**Created**: 2026-03-09  
**Status**: Draft  
**Input**: User description: "Lifecycle Observability and Advanced Workflow Enforcement for Agent-Engram — advanced lifecycle management, comprehensive workspace synchronization, state versioning, structured graph querying, hierarchical workflow groupings, and daemon observability"


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
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Blocked task transition rejection with specific blocker chain within 50ms (SC-001)
- [x] #2 100% of tool, lifecycle, watcher, and db operations covered by structured trace spans (SC-002)
- [x] #3 State rollback to any event succeeds, restoring exact state at that point (SC-003)
- [x] #4 Read-only queries return correct results with zero modification side effects (SC-004)
- [x] #5 Collection-based context retrieval completes in single operation (SC-005)
- [x] #6 Daemon sustains 2+ hour sessions with 3+ concurrent clients, zero dropped connections, consistent state (SC-006)
- [x] #7 Daemon health metrics available on demand within 100ms (SC-007)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
### Requirements

# Specification Quality Checklist: Lifecycle Observability & Advanced Workflow Enforcement

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-03-09
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

- Spec contains 6 user stories covering all major feature areas with clear priority ordering (P1-P3)
- 25 functional requirements organized by feature category
- 7 measurable success criteria, all technology-agnostic
- 6 edge cases identified with expected behavior
- External tracker sync (Jira/Linear) explicitly deferred to out-of-scope
- Scope boundaries section clearly delineates in/out of scope items
- Assumptions section documents 6 key assumptions about existing architecture
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Plan

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

### Task Breakdown

# Tasks: Lifecycle Observability & Advanced Workflow Enforcement

**Input**: Design documents from `/specs/005-lifecycle-observability/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/mcp-tools.md, SCENARIOS.md

**Tests**: TDD is mandated by project constitution (Principle III). Tests are written FIRST and MUST FAIL before implementation.

**Organization**: Tasks grouped by user story. SCENARIOS.md is the authoritative source for test scenarios.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: New module scaffolding and dependency additions

- [X] T001 Add new dependencies to Cargo.toml: tracing-opentelemetry and opentelemetry-otlp (optional, behind `otlp-export` feature flag)
- [X] T002 [P] Create empty module files: src/models/event.rs, src/models/collection.rs, src/services/gate.rs, src/services/event_ledger.rs, src/server/observability.rs
- [X] T003 [P] Register new modules in src/models/mod.rs, src/services/mod.rs, src/server/mod.rs
- [X] T004 [P] Add new error code constants to src/errors/codes.rs: TASK_BLOCKED (3015), ROLLBACK_DENIED (3020), EVENT_NOT_FOUND (3021), ROLLBACK_CONFLICT (3022), COLLECTION_EXISTS (3030), COLLECTION_NOT_FOUND (3031), CYCLIC_COLLECTION (3032), QUERY_REJECTED (4010), QUERY_TIMEOUT (4011), QUERY_INVALID (4012)
- [X] T005 [P] Add new EngramError variants to src/errors/mod.rs for gate, event, collection, and query errors
- [X] T006 [P] Add new configuration parameters to src/config/mod.rs: event_ledger_max, allow_agent_rollback, query_timeout_ms, query_row_limit, otlp_endpoint

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Schema definitions and data models that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T007 Define Event model struct and EventKind enum in src/models/event.rs per data-model.md
- [X] T008 [P] Define Collection model struct in src/models/collection.rs per data-model.md
- [X] T009 Add SurrealDB schema for `event` table to src/db/schema.rs (fields: kind, entity_table, entity_id, previous_value, new_value, source_client, created_at; indexes: event_created, event_entity, event_kind)
- [X] T010 [P] Add SurrealDB schema for `collection` table to src/db/schema.rs (fields: name, description, created_at, updated_at; index: collection_name UNIQUE)
- [X] T011 [P] Add SurrealDB schema for `contains` relation table to src/db/schema.rs (field: added_at)
- [X] T012 Register new schema definitions in src/db/mod.rs ensure_schema function
- [X] T013 [P] Add proptest round-trip tests for Event and Collection serialization in tests/unit/proptest_events.rs

**Checkpoint**: Schema and models ready — user story implementation can begin

---

## Phase 3: User Story 1 — Dependency-Gated Task Execution (Priority: P1) 🎯 MVP

**Goal**: Enforce task dependency ordering — reject transitions when hard_blocker prerequisites are incomplete

**Independent Test**: Create two tasks with a hard_blocker edge, attempt to transition the blocked task to in_progress, verify rejection with descriptive error

### Tests for User Story 1 ⚠️

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T014 [P] [US1] Contract test: update_task rejects in_progress transition when hard_blocker incomplete (S001) in tests/contract/gate_test.rs
- [X] T015 [P] [US1] Contract test: update_task succeeds when hard_blocker complete (S002) in tests/contract/gate_test.rs
- [X] T016 [P] [US1] Contract test: transitive blocking across 3-task chain (S003) in tests/contract/gate_test.rs
- [X] T017 [P] [US1] Contract test: soft_dependency emits warning not rejection (S004) in tests/contract/gate_test.rs
- [X] T018 [P] [US1] Contract test: add_dependency rejects cyclic dependency (S006, S007, S008) in tests/contract/gate_test.rs
- [X] T019 [P] [US1] Integration test: multiple blockers reported in single error (S010) in tests/integration/gate_integration_test.rs
- [X] T020 [P] [US1] Integration test: gate performance under 100-task chain within 50ms (S012) in tests/integration/gate_integration_test.rs

### Implementation for User Story 1

- [X] T021 [US1] Implement check_blockers() recursive graph query in src/db/queries.rs — walks upstream depends_on edges filtering hard_blocker type, returns Vec<BlockerInfo>
- [X] T022 [US1] Implement check_cycle() path-existence query in src/db/queries.rs — detects if adding an edge would create a cycle
- [X] T023 [US1] Implement gate evaluation logic in src/services/gate.rs — calls check_blockers, returns GateResult (pass/fail with blocker details and soft_dependency warnings)
- [X] T024 [US1] Integrate gate check into update_task in src/tools/write.rs — call gate evaluation before applying status transition to in_progress
- [X] T025 [US1] Integrate cycle detection into add_dependency in src/tools/write.rs — call check_cycle before creating edge
- [X] T026 [US1] Add warnings field to update_task response for soft_dependency notifications in src/tools/write.rs

**Checkpoint**: Gate enforcement working — agents cannot start blocked tasks

---

## Phase 4: User Story 2 — Daemon Performance Observability (Priority: P1)

**Goal**: Emit structured trace spans for all daemon operations with optional OTLP export

**Independent Test**: Start daemon, perform tool calls, inspect structured log for trace spans with timing data

### Tests for User Story 2 ⚠️

- [X] T027 [P] [US2] Contract test: tool call emits span with tool name, workspace_id, duration (S057) in tests/contract/observability_test.rs
- [X] T028 [P] [US2] Contract test: get_health_report returns all expected metrics (S056) in tests/contract/observability_test.rs
- [X] T029 [P] [US2] Contract test: get_health_report works without workspace binding (S060) in tests/contract/observability_test.rs

### Implementation for User Story 2

- [X] T030 [US2] Add latency tracking to AppState in src/server/state.rs — query_latencies VecDeque, tool_call_count AtomicU64, watcher_event_count AtomicU64, last_watcher_event RwLock
- [X] T031 [US2] Add #[instrument] tracing spans to all tool dispatch paths in src/tools/mod.rs — record tool name, workspace_id, duration
- [X] T032 [P] [US2] Add tracing spans to file watcher event processing in src/daemon/watcher.rs — event_detected, debounce_complete, db_update
- [X] T033 [P] [US2] Add tracing spans to TTL lifecycle events in src/daemon/ttl.rs — wake, sleep, expiry
- [X] T034 [US2] Implement get_health_report tool in src/tools/read.rs — returns version, uptime, memory, latency percentiles (p50/p95/p99), watcher status, connection count
- [X] T035 [US2] Register get_health_report in src/tools/mod.rs dispatch
- [X] T036 [US2] Implement OTLP export setup in src/server/observability.rs (behind otlp-export feature flag) — tracing-opentelemetry layer added to subscriber stack when ENGRAM_OTLP_ENDPOINT is set

**Checkpoint**: All daemon operations emit structured trace spans, health metrics available

---

## Phase 5: User Story 6 — Reliable Daemon Availability (Priority: P1)

**Goal**: Harden daemon for sustained multi-hour sessions with concurrent clients

**Independent Test**: Run daemon with 3 concurrent clients for extended period, verify zero dropped connections

### Tests for User Story 6 ⚠️

- [X] T037 [P] [US6] Integration test: 3 concurrent clients issuing tool calls without corruption (S061) in tests/integration/reliability_test.rs
- [X] T038 [P] [US6] Integration test: concurrent reads during write maintain consistency (S062) in tests/integration/reliability_test.rs
- [X] T039 [P] [US6] Integration test: client disconnect does not affect other clients (S063) in tests/integration/reliability_test.rs
- [X] T040 [P] [US6] Integration test: state consistent after simulated crash (S064) in tests/integration/reliability_test.rs

### Implementation for User Story 6

- [X] T041 [US6] Audit and harden RwLock usage in src/server/state.rs — verify no deadlock potential under concurrent access
- [X] T042 [US6] Add connection health monitoring spans in src/server/sse.rs and src/daemon/ipc_server.rs
- [X] T043 [US6] Verify atomic write-to-temp-then-rename in src/services/dehydration.rs survives simulated interruption
- [X] T044 [US6] Create agent integration template at .engram/agent-templates/tool-selection-guide.md — MCP tool usage examples for AI assistants

**Checkpoint**: Daemon proven reliable for extended concurrent sessions

---

## Phase 6: User Story 3 — State Event Logging and Rollback (Priority: P2)

**Goal**: Record all state changes in an append-only ledger with rollback capability

**Independent Test**: Create task, modify several times, rollback to earlier event, verify state restored

### Tests for User Story 3 ✅

- [X] T045 [P] [US3] Contract test: task creation records event with kind=task_created (S013) in tests/contract/event_test.rs
- [X] T046 [P] [US3] Contract test: task update records event with previous/new values (S014) in tests/contract/event_test.rs
- [X] T047 [P] [US3] Contract test: edge creation records event (S015) in tests/contract/event_test.rs
- [X] T048 [P] [US3] Contract test: rolling retention prunes oldest events (S016) in tests/contract/event_test.rs
- [X] T049 [P] [US3] Contract test: get_event_history returns filtered results (S017, S018) in tests/contract/event_test.rs
- [X] T050 [P] [US3] Contract test: rollback_to_event restores previous state (S023) in tests/contract/event_test.rs
- [X] T051 [P] [US3] Contract test: rollback denied when allow_agent_rollback=false (S025) in tests/contract/event_test.rs
- [X] T052 [P] [US3] Contract test: rollback to non-existent event returns error (S027) in tests/contract/event_test.rs
- [X] T053 [P] [US3] Integration test: rollback reverses edge creation (S024) in tests/integration/rollback_test.rs
- [X] T054 [P] [US3] Integration test: rollback conflict when entity deleted (S028) in tests/integration/rollback_test.rs

### Implementation for User Story 3

- [X] T055 [US3] Implement event recording functions in src/services/event_ledger.rs — record_event(), prune_events()
- [X] T056 [US3] Implement event ledger queries in src/db/queries.rs — insert_event, list_events, count_events, delete_oldest_events, get_events_after
- [X] T057 [US3] Integrate event recording into all write tools in src/tools/write.rs — create_task, update_task, add_dependency, add_blocker, etc.
- [X] T058 [US3] Implement get_event_history tool in src/tools/read.rs — filtered retrieval with pagination
- [X] T059 [US3] Implement rollback validation in src/services/event_ledger.rs — check event existence, detect conflicts
- [X] T060 [US3] Implement rollback_to_event tool in src/tools/write.rs — reverse events, restore previous values, record rollback event
- [X] T061 [US3] Register get_event_history and rollback_to_event in src/tools/mod.rs dispatch

**Checkpoint**: Event ledger recording all changes, rollback functional

---

## Phase 7: User Story 4 — Sandboxed Graph Query Interface (Priority: P2)

**Goal**: Expose read-only SurrealQL queries with sandboxing, timeout, and row limits

**Independent Test**: Populate workspace with tasks and dependencies, issue read-only queries, verify correct results and write rejection

### Tests for User Story 4 ⚠️

- [X] T062 [P] [US4] Contract test: SELECT query returns correct results (S031) in tests/contract/query_test.rs
- [X] T063 [P] [US4] Contract test: graph traversal query returns correct results (S032) in tests/contract/query_test.rs
- [X] T064 [P] [US4] Contract test: INSERT query rejected (S033) in tests/contract/query_test.rs
- [X] T065 [P] [US4] Contract test: DELETE query rejected (S034) in tests/contract/query_test.rs
- [X] T066 [P] [US4] Contract test: UPDATE query rejected (S035) in tests/contract/query_test.rs
- [X] T067 [P] [US4] Contract test: DEFINE statement rejected (S041) in tests/contract/query_test.rs
- [X] T068 [P] [US4] Contract test: RELATE statement rejected (S042) in tests/contract/query_test.rs
- [X] T069 [P] [US4] Contract test: invalid syntax returns QUERY_INVALID (S038) in tests/contract/query_test.rs
- [X] T070 [P] [US4] Contract test: non-existent table returns empty result (S039) in tests/contract/query_test.rs
- [X] T071 [P] [US4] Contract test: row limit enforced (S037) in tests/contract/query_test.rs
- [X] T072 [P] [US4] Contract test: query without workspace returns WORKSPACE_NOT_SET (S043) in tests/contract/query_test.rs

### Implementation for User Story 4

- [X] T073 [US4] Implement query sanitizer in src/services/gate.rs — word-boundary keyword blocklist validation (INSERT, UPDATE, DELETE, CREATE, DEFINE, REMOVE, RELATE, KILL, SLEEP, THROW); MUST use word-boundary detection and MUST NOT match keywords inside quoted string literals
- [X] T074 [US4] Implement query_graph tool in src/tools/read.rs — sanitize, execute with timeout, enforce row limit, return results
- [X] T075 [US4] Register query_graph in src/tools/mod.rs dispatch

**Checkpoint**: Agents can query workspace graph with full sandboxing

---

## Phase 8: User Story 5 — Hierarchical Workflow Groupings (Priority: P3)

**Goal**: Named collections that group tasks hierarchically with recursive context retrieval

**Independent Test**: Create collection, add tasks, retrieve collection context, verify recursive results

### Tests for User Story 5 ⚠️

- [x] T076 [P] [US5] Contract test: create_collection succeeds (S044) in tests/contract/collection_test.rs
- [x] T077 [P] [US5] Contract test: duplicate collection name rejected (S045) in tests/contract/collection_test.rs
- [x] T078 [P] [US5] Contract test: add_to_collection creates contains edges (S046) in tests/contract/collection_test.rs
- [x] T079 [P] [US5] Contract test: recursive context retrieval (S048) in tests/contract/collection_test.rs
- [x] T080 [P] [US5] Contract test: collection context with status filter (S049) in tests/contract/collection_test.rs
- [x] T081 [P] [US5] Contract test: cyclic collection nesting rejected (S053) in tests/contract/collection_test.rs
- [x] T082 [P] [US5] Contract test: remove_from_collection removes contains edges (S051) in tests/contract/collection_test.rs
- [x] T083 [P] [US5] Contract test: collection_not_found error (S054) in tests/contract/collection_test.rs

### Implementation for User Story 5

- [x] T084 [US5] Implement collection CRUD queries in src/db/queries.rs — create_collection, get_collection, add_member, remove_member, list_members_recursive
- [x] T085 [US5] Implement collection cycle detection in src/db/queries.rs — check_collection_cycle
- [x] T086 [US5] Implement create_collection tool in src/tools/write.rs
- [x] T087 [US5] Implement add_to_collection tool in src/tools/write.rs
- [x] T088 [US5] Implement remove_from_collection tool in src/tools/write.rs
- [x] T089 [US5] Implement get_collection_context tool in src/tools/read.rs — recursive traversal with optional filters
- [x] T090 [US5] Register all collection tools in src/tools/mod.rs dispatch
- [x] T091 [US5] Add collection dehydration to src/services/dehydration.rs — serialize to .engram/collections.md
- [x] T092 [US5] Add collection hydration to src/services/hydration.rs — parse from .engram/collections.md

**Checkpoint**: Collections working with full recursive context retrieval

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, integration, and final validation

- [X] T093 [P] Update src/tools/mod.rs MCP tool descriptions for discoverability
- [X] T094 [P] Update .engram/.version if schema version changes
- [X] T095 [P] Update README.md with new tool documentation and configuration parameters
- [X] T096 Run cargo clippy to verify zero warnings across all new code
- [X] T097 Run cargo test to verify all tests pass
- [X] T098 Run quickstart.md validation — verify all example tool calls work end-to-end

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational — gate enforcement
- **US2 (Phase 4)**: Depends on Foundational — observability spans
- **US6 (Phase 5)**: Depends on Foundational — reliability hardening
- **US3 (Phase 6)**: Depends on Foundational — event ledger (benefits from US1 gate logic for event recording integration)
- **US4 (Phase 7)**: Depends on Foundational — sandboxed queries
- **US5 (Phase 8)**: Depends on Foundational — collections
- **Polish (Phase 9)**: Depends on all desired user stories

### User Story Dependencies

- **US1 (P1)**: After Foundational — no cross-story dependencies
- **US2 (P1)**: After Foundational — no cross-story dependencies
- **US6 (P1)**: After Foundational — no cross-story dependencies
- **US3 (P2)**: After Foundational — light coupling with US1 (event recording in write tools shares code paths)
- **US4 (P2)**: After Foundational — no cross-story dependencies
- **US5 (P3)**: After Foundational — no cross-story dependencies

### Parallel Opportunities

- US1, US2, and US6 can run in parallel after Foundational
- US3, US4, and US5 can run in parallel after Foundational
- All test tasks within a phase marked [P] can run in parallel
- All model creation tasks within a phase marked [P] can run in parallel

---

## Implementation Strategy

### MVP First (US1 — Dependency Gates)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US1 — Dependency Gate Enforcement
4. **STOP and VALIDATE**: Verify gate enforcement works with manual testing
5. This alone delivers significant agent productivity improvement

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. US1 → Gate enforcement (MVP!)
3. US2 → Observability
4. US6 → Reliability hardening
5. US3 → Event ledger + rollback
6. US4 → Sandboxed queries
7. US5 → Collections
8. Polish → Documentation and validation

---

## Summary

| Metric | Count |
| ------ | ----- |
| **Total tasks** | 98 |
| Phase 1 (Setup) | 6 |
| Phase 2 (Foundational) | 7 |
| Phase 3 (US1 — Gates) | 13 |
| Phase 4 (US2 — Observability) | 10 |
| Phase 5 (US6 — Reliability) | 8 |
| Phase 6 (US3 — Events/Rollback) | 17 |
| Phase 7 (US4 — Queries) | 14 |
| Phase 8 (US5 — Collections) | 17 |
| Phase 9 (Polish) | 6 |
| **Parallelizable tasks** | 62 |
| **Scenario coverage** | 62/62 (100%) |
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Research

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

### Data Model

# Data Model: Lifecycle Observability & Advanced Workflow Enforcement

**Feature**: 005-lifecycle-observability
**Date**: 2026-03-09

## New Entities

### Event

An immutable record of a state-modifying operation in the workspace. Forms the append-only event ledger.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `id` | string | auto | SurrealDB record ID (`event:{ulid}`) |
| `kind` | EventKind | yes | Discriminated union of event types |
| `entity_table` | string | yes | Target table name (e.g., `task`, `depends_on`, `collection`) |
| `entity_id` | string | yes | Target record ID (e.g., `task:abc123`) |
| `previous_value` | JSON (nullable) | no | Serialized entity state before the change (null for creation events) |
| `new_value` | JSON (nullable) | no | Serialized entity state after the change (null for deletion events) |
| `source_client` | string | yes | Identifier of the client that triggered the change |
| `created_at` | datetime | auto | Timestamp of the event (immutable, set on creation) |

**EventKind enum** (serialized as snake_case strings):

| Variant | Description |
| ------- | ----------- |
| `task_created` | A new task was created |
| `task_updated` | A task was modified (status, title, description, etc.) |
| `task_deleted` | A task was removed |
| `edge_created` | A dependency, implements, or relates_to edge was created |
| `edge_deleted` | A relation edge was removed |
| `context_created` | A context entry was added |
| `collection_created` | A new collection was created |
| `collection_updated` | A collection was modified |
| `collection_membership_changed` | A task or sub-collection was added/removed from a collection |

**Indexes**:
- `event_created` on `created_at` (for retention pruning and chronological retrieval)
- `event_entity` on `entity_table, entity_id` (for entity-scoped history queries)
- `event_kind` on `kind` (for filtered retrieval)

**Lifecycle**: Events are immutable after creation. Pruning removes the oldest events when the ledger exceeds `event_ledger_max`. Events are NOT dehydrated — they are transient operational data stored only in SurrealDB.

---

### Collection

A named grouping of tasks and sub-collections representing a feature, epic, or workflow.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `id` | string | auto | SurrealDB record ID (`collection:{ulid}`) |
| `name` | string | yes | Human-readable collection name (unique within workspace) |
| `description` | string | no | Optional description of the collection's purpose |
| `created_at` | datetime | auto | Creation timestamp |
| `updated_at` | datetime | auto | Last modification timestamp |

**Indexes**:
- `collection_name` on `name` UNIQUE (prevents duplicate collection names)

**Lifecycle**: Created via `create_collection`, modified via `add_to_collection`/`remove_from_collection`. Dehydrated to `.engram/collections.md` during flush. Hydrated on workspace binding.

---

### Contains (Relation)

Relation edge connecting a collection to its members (tasks or sub-collections).

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `in` | record | yes | The collection being connected from |
| `out` | record | yes | The task or sub-collection being connected to |
| `added_at` | datetime | auto | When the membership was established |

**Constraints**:
- `in` must be a `collection` record
- `out` must be a `task` or `collection` record
- Cycle detection: adding a `collection` as member of its own descendant is rejected

---

## Modified Entities

### Task (existing — modifications)

No new fields added. Gate enforcement is implemented as validation logic in `update_task`, not as stored state on the task entity. The existing `depends_on` relation table with `hard_blocker` and `soft_dependency` types is sufficient.

### AppState (existing — modifications)

Extended with latency tracking for health reporting:

| New Field | Type | Description |
| --------- | ---- | ----------- |
| `query_latencies` | `VecDeque<Duration>` | Rolling window of recent query latencies (last 100) |
| `tool_call_count` | `AtomicU64` | Total tool calls since daemon start |
| `watcher_event_count` | `AtomicU64` | Total file watcher events processed |
| `last_watcher_event` | `RwLock<Option<Instant>>` | Timestamp of most recent watcher event |

---

## Relation Edges Summary

| Edge Table | From | To | New? | Purpose |
| ---------- | ---- | -- | ---- | ------- |
| `depends_on` | task | task | Existing | Blocking/dependency relationships (gate enforcement reads these) |
| `implements` | task | spec | Existing | Task-to-spec linkage |
| `relates_to` | any | any | Existing | Informational relationships |
| `contains` | collection | task/collection | **New** | Collection membership hierarchy |

## State Transitions

### Event Ledger Retention

```
On every state-modifying operation:
  1. Record event to `event` table
  2. Count total events
  3. If count > event_ledger_max:
     Delete oldest (count - event_ledger_max) events
```

### Rollback Flow

```
On rollback_to_event(event_id):
  1. Validate: event_id exists in ledger
  2. Validate: allow_agent_rollback config (if called by agent)
  3. Fetch all events AFTER event_id, ordered DESC by created_at
  4. For each event (newest first):
     a. If entity referenced by event was deleted: report conflict, skip
     b. Restore entity to previous_value (or delete if previous_value is null)
  5. Delete rolled-back events from ledger
  6. Record a new "rollback" event in the ledger
```

### Dependency Gate Evaluation

```
On update_task(task_id, new_status):
  If new_status is "in_progress":
    1. Query: all upstream hard_blockers recursively
    2. Filter: keep only those with status != "done"
    3. If any remain: reject with blocker list
    4. Query: all upstream soft_dependencies
    5. Filter: keep only those with status != "done"
    6. If any remain: include warning in successful response
  If new_status is other: proceed normally (existing validation)
```

### Analysis

# Adversarial Analysis Report: 005-lifecycle-observability

**Feature**: Lifecycle Observability & Advanced Workflow Enforcement
**Date**: 2026-03-09
**Artifacts analyzed**: spec.md, plan.md, tasks.md, SCENARIOS.md, data-model.md, contracts/mcp-tools.md

## Adversarial Review Summary

| Reviewer | Model | Focus Area | Findings Count |
|----------|-------|------------|----------------|
| A | Claude Opus 4.6 | Logical Consistency | 6 |
| B | GPT-5.3 Codex | Technical Feasibility | 7 |
| C | Gemini 3.1 Pro Preview | Edge Cases and Security | 5 |

All three reviewers agreed that the artifacts are comprehensive and well-structured. The primary areas of concern were: (1) a justified constitution exception needing explicit documentation, (2) query sanitizer robustness, and (3) schema versioning for event snapshots. No critical constitution violations were found — all principles are either fully compliant or have documented justifications.

## Unified Findings Table

| ID | Category | Severity | Location(s) | Summary | Recommendation | Consensus |
|----|----------|----------|-------------|---------|----------------|-----------|
| RC-01 | Constitution | HIGH | plan.md:Constitution Check (Principle VI) | Constitution Principle VI mandates "All state must be serializable to human-readable, Git-mergeable files." The event ledger is excluded from dehydration but the Constitution Check marks Principle VI as PASS without noting the justified exception. | Update the Constitution Check to note the justified exception — mark as "PASS with exception" and document that events are transient operational data per the rationale in research.md. Add to Complexity Tracking. | majority |
| TF-01 | Technical | HIGH | research.md:§Research 4, tasks.md:T073 | Query sanitizer uses a keyword blocklist which can produce false positives when keywords appear inside string literals (e.g., `SELECT * FROM task WHERE description CONTAINS 'DELETE this'`). | Specify that keyword matching must use word-boundary detection and must not match within quoted string literals. Update T073 description to include this requirement. | unanimous |
| TF-02 | Technical | MEDIUM | spec.md:FR-008, plan.md:§New Config | OTLP export over gRPC requires outbound network connection to collector. Constitution network security says "Bind to 127.0.0.1 only." This refers to inbound binding, but outbound gRPC to an external collector should be explicitly acknowledged. | Add a note in plan.md that OTLP export targets outbound-only connections and does not expose a listening port, distinguishing from the constitution's inbound binding restriction. | majority |
| ES-01 | Terminology | MEDIUM | spec.md:US5, plan.md:§Research 5 | "Collection" and "epic" used interchangeably across artifacts. Spec US5 title says "Hierarchical Workflow Groupings" but body mixes "collection", "epic", and "workflow". | Standardize on "collection" as the canonical term throughout all artifacts. Use "epic" only in parenthetical explanation on first reference: "collection (also known as epic)". | majority |
| TF-03 | Technical | MEDIUM | data-model.md:§Event, tasks.md:T055-T060 | Event snapshots store `previous_value` as serialized JSON, but no schema versioning strategy exists for these snapshots. If the schema changes (e.g., new fields on Task), rollback could fail to deserialize old snapshots. | Add a `schema_version` field to the Event entity that records the data model version at time of capture. Rollback logic should validate schema compatibility before applying. | single |
| RC-02 | Coverage | MEDIUM | spec.md:FR-013b, contracts/mcp-tools.md | FR-013b names the config parameter `event_ledger_max_events` but the plan and contracts use `event_ledger_max` and the env var uses `ENGRAM_EVENT_LEDGER_MAX`. Inconsistent naming. | Standardize on `event_ledger_max` across all artifacts (spec, plan, contracts, config). Update FR-013b to use `event_ledger_max`. | unanimous |
| TF-04 | Technical | MEDIUM | plan.md:§Source Code, tasks.md:T002 | Plan lists `src/server/observability.rs` as a new file but it is not registered in `src/server/mod.rs` in the plan's module structure listing. Task T003 says to register in `src/server/mod.rs` but T002 creates the file — ordering dependency is correct but plan structure listing should be explicit. | No fix needed — tasks handle this correctly. Note for implementation: ensure T003 includes `observability` module registration. | single |
| ES-02 | Edge Case | LOW | SCENARIOS.md | No scenario covers the case where `get_event_history` is called with an offset exceeding total events. Should return empty results gracefully. | Add scenario covering pagination beyond available events. | single |
| RC-03 | Coverage | LOW | tasks.md:Phase 9 | No explicit task for updating error code documentation in copilot-instructions.md after adding new error codes. | Add a documentation task in Phase 9 to update the MCP Tools Registry and error codes in .github/copilot-instructions.md. | single |
| ES-03 | Style | LOW | quickstart.md | Example tool calls use placeholder IDs (e.g., "task:impl-id", "task:review-id") that could confuse implementers. Should note these are illustrative. | Add a note at the top of quickstart.md examples that IDs are illustrative. | single |

## Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Has Scenario? | Scenario IDs | Notes |
|-----------------|-----------|----------|---------------|--------------|-------|
| FR-001 | ✅ | T021, T024 | ✅ | S001-S003 | |
| FR-002 | ✅ | T022, T025 | ✅ | S006-S008 | |
| FR-003 | ✅ | T023, T026 | ✅ | S004 | |
| FR-004 | ✅ | T021 | ✅ | S003 | |
| FR-005 | ✅ | T031 | ✅ | S057 | |
| FR-006 | ✅ | T033 | ✅ | S059 | |
| FR-007 | ✅ | T032 | ✅ | S058 | |
| FR-008 | ✅ | T036 | ✅ | S056 | |
| FR-008a | ✅ | T036 | ⚠️ | — | No explicit scenario for runtime toggle |
| FR-009 | ✅ | T034 | ✅ | S056, S060 | |
| FR-010 | ✅ | T055, T057 | ✅ | S013-S015 | |
| FR-011 | ✅ | T059, T060 | ✅ | S023-S024 | |
| FR-011a | ✅ | T060 | ✅ | S025-S026 | |
| FR-012 | ✅ | T059 | ✅ | S028 | |
| FR-013 | ✅ | T055 | ✅ | S020 | |
| FR-013a | ✅ | T055 | ✅ | S016 | |
| FR-013b | ✅ | T006 | ✅ | S022 | |
| FR-014 | ✅ | T074 | ✅ | S031-S032 | |
| FR-015 | ✅ | T073 | ✅ | S033-S035, S041-S042 | |
| FR-016 | ✅ | T074 | ✅ | S036-S037 | |
| FR-017 | ✅ | T074 | ✅ | S043 | |
| FR-018 | ✅ | T084 | ✅ | S044 | |
| FR-019 | ✅ | T089 | ✅ | S048 | |
| FR-020 | ✅ | T087 | ✅ | S050 | |
| FR-021 | ✅ | T089 | ✅ | S049 | |
| FR-022 | ✅ | T041 | ✅ | S061-S062 | |
| FR-023 | ✅ | T043 | ✅ | S066 | |
| FR-024 | ✅ | T043 | ✅ | S064 | |
| FR-025 | ✅ | T044 | ⚠️ | — | No scenario for template validation |

## Remediation Log

| Finding ID | File | Change Description | Original Text (excerpt) | Applied? |
|------------|------|--------------------|-------------------------|----------|
| RC-01 | plan.md | Updated Constitution Check Principle VI to note justified exception; added Complexity Tracking entry | "✅ PASS \| Event ledger stored in SurrealDB..." | ✅ Applied |
| TF-01 | tasks.md | Updated T073 description to require word-boundary matching and string literal exclusion | "Implement query sanitizer in src/services/gate.rs..." | ✅ Applied |
| RC-02 | spec.md | Updated FR-013b to use `event_ledger_max` instead of `event_ledger_max_events` | "FR-013b: System MUST expose the retention window size as a configuration parameter (`event_ledger_max_events`)" | ✅ Applied |

## Remaining Issues

**Medium (deferred to operator review):**
- TF-02: OTLP outbound connection acknowledgment
- ES-01: Terminology standardization (collection vs epic)
- TF-03: Schema versioning for event snapshots

**Low (suggestions):**
- ES-02: Add pagination edge-case scenario
- RC-03: Add documentation task for copilot-instructions.md
- ES-03: Add illustrative note to quickstart examples

## Constitution Alignment Issues

| Principle | Finding | Resolution |
|-----------|---------|------------|
| VI. Git-Friendly Persistence | Event ledger excluded from dehydration | Justified exception documented — events are transient operational data, not user-editable state. Added to Complexity Tracking. |

## Unmapped Tasks

None — all tasks map to at least one requirement.

## Metrics

**Artifact metrics:**
- Total requirements: 29 (FR-001 through FR-025, plus FR-008a, FR-011a, FR-013a, FR-013b)
- Total tasks: 98
- Total scenarios: 62
- Task coverage: 100% (29/29 requirements have tasks)
- Scenario coverage: 93% (27/29 requirements have scenarios; FR-008a and FR-025 lack explicit scenarios)
- Non-happy-path percentage: 68%

**Finding metrics:**
- Ambiguity count: 2 (ES-01 terminology, TF-02 network scope)
- Duplication count: 1 (RC-02 naming inconsistency)
- Critical issues found: 0
- Critical issues remediated: 0
- High issues found: 2
- High issues remediated: 2

**Adversarial metrics:**
- Total findings pre-deduplication: 18
- Total findings post-synthesis: 10
- Agreement rate: 50% (5/10 findings with majority or unanimous consensus)
- Conflict count: 0

## Next Actions

All critical and high issues have been remediated. The artifacts are ready for operator review of medium-severity findings before proceeding to implementation.

Recommended next step: **Stage 7: Operator Review** — present medium findings via agent-intercom for operator approval.

### Scenarios

# Behavioral Matrix: Lifecycle Observability & Advanced Workflow Enforcement

**Input**: Design documents from `/specs/005-lifecycle-observability/`
**Prerequisites**: spec.md (required), plan.md (required), data-model.md, contracts/mcp-tools.md
**Created**: 2026-03-09

## Summary

| Metric | Count |
|---|---|
| **Total Scenarios** | 62 |
| Happy-path | 20 |
| Edge-case | 12 |
| Error | 14 |
| Boundary | 6 |
| Concurrent | 6 |
| Security | 4 |

**Non-happy-path coverage**: 68% (minimum 30% required)

## Dependency Gate Enforcement

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S001 | Reject transition when hard_blocker incomplete | Task B depends on task A (hard_blocker), A.status=todo | `update_task { id: B, status: "in_progress" }` | Error response with blocker chain listing task A | B.status remains todo, TASK_BLOCKED (3010) | happy-path |
| S002 | Allow transition when hard_blocker complete | Task B depends on task A (hard_blocker), A.status=done | `update_task { id: B, status: "in_progress" }` | Success response, B transitions to in_progress | B.status=in_progress | happy-path |
| S003 | Transitive blocking across 3-task chain | A→B→C chain (hard_blocker), A.status=todo | `update_task { id: C, status: "in_progress" }` | Error listing both A and B as unresolved blockers | C.status remains todo, TASK_BLOCKED (3010) | happy-path |
| S004 | Soft dependency warning (not rejection) | Task B depends on A (soft_dependency), A.status=todo | `update_task { id: B, status: "in_progress" }` | Success with warning listing A as incomplete soft dep | B.status=in_progress, response contains warnings[] | happy-path |
| S005 | No gate check for non-in_progress transitions | Task B depends on A (hard_blocker), A.status=todo | `update_task { id: B, status: "done" }` | Normal transition validation (existing rules apply) | Existing validate_transition rules decide | edge-case |
| S006 | Detect and reject cyclic dependency at creation | Tasks A, B exist, A→B edge exists | `add_dependency { from: B, to: A, type: "hard_blocker" }` | Error identifying cycle: A → B → A | No edge created, CYCLIC_DEPENDENCY (3011) | error |
| S007 | Deep transitive cycle detection | A→B, B→C, C→D exist | `add_dependency { from: D, to: A, type: "hard_blocker" }` | Error identifying cycle: A → B → C → D → A | No edge created, CYCLIC_DEPENDENCY (3011) | error |
| S008 | Self-dependency rejected | Task A exists | `add_dependency { from: A, to: A, type: "hard_blocker" }` | Error: self-referential dependency | No edge created, CYCLIC_DEPENDENCY (3011) | error |
| S009 | Gate allows done→todo without blocker check | Task B depends on A (hard_blocker), A.status=todo, B.status=done | `update_task { id: B, status: "todo" }` | Success (done→todo is allowed, no gate check needed) | B.status=todo | edge-case |
| S010 | Multiple blockers reported in single error | B depends on A1, A2, A3 (all hard_blocker), all todo | `update_task { id: B, status: "in_progress" }` | Error listing all 3 blockers | B.status remains todo | happy-path |
| S011 | Mixed hard/soft dependencies | B has hard_blocker on A1 (todo) and soft_dep on A2 (todo) | `update_task { id: B, status: "in_progress" }` | Error for hard_blocker on A1 (soft dep not evaluated since hard fails) | B.status remains todo | edge-case |
| S012 | Gate check performance under large graph | 100 tasks in a linear chain, all done except root | `update_task { id: task_100, status: "in_progress" }` | Error citing root task, response within 50ms | task_100 remains todo | boundary |

---

## Event Ledger

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S013 | Event recorded on task creation | Workspace bound, no tasks | `create_task { title: "Test" }` | Task created, event recorded with kind=task_created | Event table has 1 entry with previous_value=null | happy-path |
| S014 | Event recorded on task status update | Task exists with status=todo | `update_task { id: task_id, status: "in_progress" }` | Task updated, event with previous_value showing todo | Event has previous_value.status=todo, new_value.status=in_progress | happy-path |
| S015 | Event recorded on edge creation | Two tasks exist | `add_dependency { from: A, to: B, type: "hard_blocker" }` | Edge created, event with kind=edge_created | Event has entity_table=depends_on | happy-path |
| S016 | Rolling retention prunes oldest events | Ledger has 500 events (max), new write occurs | `create_task { title: "New" }` | New event recorded, oldest event pruned | Ledger count remains 500 | happy-path |
| S017 | Event history retrieval with filters | 20 events, mixed kinds | `get_event_history { kind: "task_updated", limit: 5 }` | Returns up to 5 task_updated events, chronological | total_count reflects filtered count | happy-path |
| S018 | Event history with entity_id filter | Events for task:A and task:B | `get_event_history { entity_id: "task:A" }` | Returns only events targeting task:A | Other entities excluded | happy-path |
| S019 | Empty event history | Workspace just bound, no operations | `get_event_history {}` | Returns empty events array, total_count=0 | Ledger empty | edge-case |
| S020 | Ledger survives daemon restart | 10 events in ledger, daemon stops and restarts | Restart daemon, `get_event_history {}` | All 10 events present | Ledger persisted via SurrealDB | happy-path |
| S021 | Retention at boundary: exactly max events | Ledger has exactly 499 events (max=500) | `create_task {}` | New event recorded, no pruning (count=500) | Ledger count = 500 | boundary |
| S022 | Configurable retention limit | ENGRAM_EVENT_LEDGER_MAX=100 | 101st event recorded | Oldest event pruned | Ledger count = 100 | boundary |

---

## State Rollback

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S023 | Successful rollback to earlier state | Task created (event 1), then modified (event 2, 3) | `rollback_to_event { event_id: "event:1" }` | Events 2,3 reversed, task restored to creation state | Task has original field values | happy-path |
| S024 | Rollback reverses edge creation | Edge created (event 5), task modified (event 6) | `rollback_to_event { event_id: "event:4" }` | Events 5,6 reversed, edge removed, task restored | No edge exists, task at event 4 state | happy-path |
| S025 | Rollback denied for agent (default config) | allow_agent_rollback=false, agent calls rollback | `rollback_to_event { event_id: "event:1" }` | Error: ROLLBACK_DENIED (3020) | No state changes | security |
| S026 | Rollback allowed for agent when configured | allow_agent_rollback=true, agent calls rollback | `rollback_to_event { event_id: "event:1" }` | Success, events reversed | State restored | happy-path |
| S027 | Rollback to non-existent event | Event ID does not exist in ledger | `rollback_to_event { event_id: "event:nonexistent" }` | Error: EVENT_NOT_FOUND (3021) | No state changes | error |
| S028 | Rollback conflict: entity deleted since event | Task created (event 1), task deleted (event 2) | `rollback_to_event { event_id: "event:0" }` | Conflict reported for deleted entity | Conflict details in response | error |
| S029 | Rollback beyond oldest event | Ledger starts at event 50 (older pruned) | `rollback_to_event { event_id: "event:30" }` | Error: event not found (pruned) | No state changes | error |
| S030 | Rollback records its own event | 5 events, rollback to event 3 | `rollback_to_event { event_id: "event:3" }` | Rollback succeeds, new rollback event recorded | Ledger contains rollback event | edge-case |

---

## Sandboxed Query Interface

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S031 | Simple SELECT query succeeds | 5 tasks with mixed statuses | `query_graph { query: "SELECT * FROM task WHERE status = 'in_progress'" }` | Returns matching tasks, correct row_count | No data modifications | happy-path |
| S032 | Graph traversal query succeeds | Task A with 3 hard_blocker edges | `query_graph { query: "SELECT <-depends_on<-task FROM task:A" }` | Returns 3 upstream blocker tasks | No data modifications | happy-path |
| S033 | Write query rejected (INSERT) | Any workspace state | `query_graph { query: "INSERT INTO task { title: 'hack' }" }` | Error: QUERY_REJECTED (4010) | No data modifications | security |
| S034 | Write query rejected (DELETE) | Any workspace state | `query_graph { query: "DELETE task:A" }` | Error: QUERY_REJECTED (4010) | No data modifications | security |
| S035 | Write query rejected (UPDATE) | Any workspace state | `query_graph { query: "UPDATE task SET status = 'done'" }` | Error: QUERY_REJECTED (4010) | No data modifications | security |
| S036 | Query timeout exceeded | Complex query on large dataset | `query_graph { query: "SELECT * FROM task FETCH ->depends_on->task->depends_on->task" }` with timeout=100ms | Error: QUERY_TIMEOUT (4011) | No data modifications | error |
| S037 | Row limit enforced | 2000 tasks exist, limit=1000 | `query_graph { query: "SELECT * FROM task" }` | Returns 1000 rows, truncated=true | No data modifications | boundary |
| S038 | Invalid SurrealQL syntax | N/A | `query_graph { query: "SELEKT * FORM task" }` | Error: QUERY_INVALID (4012) | No data modifications | error |
| S039 | Query on non-existent table | N/A | `query_graph { query: "SELECT * FROM nonexistent_table" }` | Empty result set, row_count=0 | No schema details exposed | edge-case |
| S040 | Parameterized query with bindings | Task exists with known ID | `query_graph { query: "SELECT * FROM task WHERE id = $id", params: { id: "task:abc" } }` | Returns matching task | No data modifications | happy-path |
| S041 | DEFINE statement rejected | N/A | `query_graph { query: "DEFINE TABLE evil SCHEMAFULL" }` | Error: QUERY_REJECTED (4010) | Schema unchanged | security |
| S042 | RELATE statement rejected | N/A | `query_graph { query: "RELATE task:A->depends_on->task:B" }` | Error: QUERY_REJECTED (4010) | No edges created | error |
| S043 | Query without workspace set | No workspace bound | `query_graph { query: "SELECT * FROM task" }` | Error: WORKSPACE_NOT_SET (1001) | N/A | error |

---

## Hierarchical Collections

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S044 | Create collection succeeds | Workspace bound | `create_collection { name: "Feature X" }` | Returns collection ID, name, created_at | Collection exists in DB | happy-path |
| S045 | Duplicate collection name rejected | Collection "Feature X" exists | `create_collection { name: "Feature X" }` | Error: COLLECTION_EXISTS (3030) | No duplicate created | error |
| S046 | Add tasks to collection | Collection and 3 tasks exist | `add_to_collection { collection_id: C, member_ids: [T1, T2, T3] }` | added=3, already_members=0 | 3 contains edges created | happy-path |
| S047 | Add already-member task (idempotent) | T1 already in collection C | `add_to_collection { collection_id: C, member_ids: [T1, T2] }` | added=1 (T2), already_members=1 (T1) | T2 added, T1 unchanged | edge-case |
| S048 | Recursive context retrieval | Collection C contains T1, T2 and sub-collection SC containing T3, T4 | `get_collection_context { collection_id: C }` | Returns T1, T2, T3, T4 plus SC metadata | All tasks included recursively | happy-path |
| S049 | Collection context with status filter | Collection with 5 tasks (2 in_progress, 3 done) | `get_collection_context { collection_id: C, status_filter: ["in_progress"] }` | Returns only 2 in_progress tasks | Other tasks excluded | happy-path |
| S050 | Task belongs to multiple collections | T1 added to C1 and C2 | `get_collection_context` for C1, then C2 | T1 appears in both results | T1 has two contains edges | happy-path |
| S051 | Remove from collection | T1, T2 in collection C | `remove_from_collection { collection_id: C, member_ids: [T1] }` | removed=1, not_found=0 | T1 no longer in C, T2 still in C | happy-path |
| S052 | Remove non-member (graceful) | T3 not in collection C | `remove_from_collection { collection_id: C, member_ids: [T3] }` | removed=0, not_found=1 | No changes | edge-case |
| S053 | Cyclic collection nesting rejected | C1 contains C2 | `add_to_collection { collection_id: C2, member_ids: [C1] }` | Error: CYCLIC_COLLECTION (3032) | No edge created | error |
| S054 | Collection not found | Non-existent collection ID | `get_collection_context { collection_id: "collection:nonexistent" }` | Error: COLLECTION_NOT_FOUND (3031) | N/A | error |
| S055 | Empty collection context | Collection exists with no members | `get_collection_context { collection_id: C }` | Empty tasks array, total_tasks=0 | Valid response | edge-case |

---

## Daemon Observability

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S056 | Health report returns all metrics | Daemon running with active workspace | `get_health_report {}` | Returns version, uptime, memory, latencies, watcher status | Metrics accurate within 100ms | happy-path |
| S057 | Tool call trace span emitted | Any tool call | `create_task { title: "Test" }` | Structured log contains span with tool name, duration, workspace_id | Span visible in log output | happy-path |
| S058 | File watcher event spans emitted | Workspace with active watcher, file modified | Modify a workspace file | Log contains spans: event_detected, debounce_complete, db_update | Timing data in each span | happy-path |
| S059 | TTL wake event traced | Daemon idle past TTL check, then receives call | `get_daemon_status {}` after idle period | Log contains wake span with time_since_sleep | Span recorded | happy-path |
| S060 | Health report without workspace (always available) | Daemon running, no workspace bound | `get_health_report {}` | Returns daemon-level metrics (no workspace-specific data) | No error | edge-case |

---

## Reliability & Concurrency

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S061 | Concurrent task updates no corruption | 3 clients, same workspace | 3 simultaneous `update_task` calls on different tasks | All 3 succeed with correct state | No data corruption, no deadlock | concurrent |
| S062 | Concurrent reads during write | 1 writer + 2 readers, same workspace | `flush_state` concurrent with `get_task_graph` | All operations complete, readers see consistent state | No torn reads | concurrent |
| S063 | Client disconnect doesn't affect others | 3 clients, client 2 disconnects abruptly | Client 2 socket closed without cleanup | Clients 1 and 3 continue operating normally | Connection count decremented | concurrent |
| S064 | Crash recovery: consistent state after kill | Daemon killed (SIGKILL) during write | Restart daemon, `get_workspace_status {}` | State consistent, no half-written records | Data matches last successful flush | concurrent |
| S065 | 100 sequential calls over 2 hours | Daemon running, workspace bound | 100 tool calls over extended period | All 100 return correct responses | Zero timeouts, zero errors | concurrent |
| S066 | Atomic write prevents corruption | flush_state interrupted mid-write | Simulate power loss during dehydration | .engram/ files either fully old or fully new | No partial writes | concurrent |

---

## Edge Case Coverage Checklist

- [x] Malformed inputs and invalid arguments (S008, S033-S035, S038, S041-S042)
- [x] Missing dependencies and unavailable resources (S027, S029, S043, S054)
- [x] State errors and race conditions (S061-S064)
- [x] Boundary values (empty, max-length, zero, negative) (S012, S021-S022, S037, S055)
- [x] Permission and authorization failures (S025, S033-S035, S041)
- [x] Concurrent access patterns (S061-S066)
- [x] Graceful degradation scenarios (S047, S052, S060)

## Cross-Reference Validation

- [x] Every entity in `data-model.md` has at least one scenario covering its state transitions (Event: S013-S022, Collection: S044-S055, Contains: S046-S053)
- [x] Every endpoint in `contracts/mcp-tools.md` has at least one happy-path and one error scenario
- [x] Every user story in `spec.md` has corresponding behavioral coverage (US1: S001-S012, US2: S056-S060, US3: S013-S030, US4: S031-S043, US5: S044-S055, US6: S061-S066)
- [x] No scenario has ambiguous or non-deterministic expected outcomes

### Quickstart

# Quickstart: Lifecycle Observability & Advanced Workflow Enforcement

**Feature**: 005-lifecycle-observability

## Prerequisites

- Rust 1.85+ (stable toolchain)
- Engram daemon built and running (`cargo run --bin engram`)
- A workspace with `.engram/` directory initialized

## Quick Verification

### 1. Dependency Gate Enforcement

Create two tasks with a blocking dependency, then verify the gate rejects out-of-order transitions:

```
# Create tasks
call set_workspace { "path": "/your/workspace" }
call create_task { "title": "Design Review", "description": "Review architecture" }
call create_task { "title": "Implementation", "description": "Build the feature" }

# Add blocking dependency (Implementation blocked by Design Review)
call add_dependency { "from_id": "task:impl-id", "to_id": "task:review-id", "type": "hard_blocker" }

# Attempt to start Implementation (should fail with TASK_BLOCKED)
call update_task { "id": "task:impl-id", "status": "in_progress" }
# Expected: Error 3015 — TASK_BLOCKED citing Design Review

# Complete the blocker, then retry
call update_task { "id": "task:review-id", "status": "done" }
call update_task { "id": "task:impl-id", "status": "in_progress" }
# Expected: Success
```

### 2. Daemon Health Report

```
call get_health_report {}
# Returns: version, uptime, memory, latency percentiles, watcher status
```

### 3. Event History & Rollback

```
# View recent events
call get_event_history { "limit": 10 }

# Rollback to a specific event (operator-only by default)
call rollback_to_event { "event_id": "event:abc123" }
```

### 4. Sandboxed Graph Query

```
# Find all in-progress tasks
call query_graph { "query": "SELECT * FROM task WHERE status = 'in_progress'" }

# Find all tasks blocked by a specific task
call query_graph { "query": "SELECT * FROM task WHERE <-depends_on<-(task WHERE id = $blocker)", "params": { "blocker": "task:review-id" } }
```

### 5. Collections

```
# Create a collection
call create_collection { "name": "Feature X", "description": "All tasks for Feature X" }

# Add tasks to it
call add_to_collection { "collection_id": "collection:feat-x", "member_ids": ["task:review-id", "task:impl-id"] }

# Retrieve full context
call get_collection_context { "collection_id": "collection:feat-x" }
```

## Optional: OTLP Trace Export

Build with the `otlp-export` feature flag and set the collector endpoint:

```bash
cargo run --bin engram --features otlp-export -- --otlp-endpoint http://localhost:4317
```

Traces will be exported to the OTLP collector alongside local structured JSON logs.

## Configuration Reference

| Env Var | Default | Description |
| ------- | ------- | ----------- |
| `ENGRAM_EVENT_LEDGER_MAX` | `500` | Max events in rolling ledger |
| `ENGRAM_ALLOW_AGENT_ROLLBACK` | `false` | Allow agents to invoke rollback |
| `ENGRAM_QUERY_TIMEOUT_MS` | `5000` | Sandboxed query timeout |
| `ENGRAM_QUERY_ROW_LIMIT` | `1000` | Max rows from sandboxed queries |
| `ENGRAM_OTLP_ENDPOINT` | (none) | OTLP collector endpoint |

### Operator Review Log

# Operator Review Log: 005-lifecycle-observability

**Date**: 2026-03-09
**Total Findings Reviewed**: 3 (medium severity)
**Review Mode**: Autonomous (agent-intercom transmit unavailable for blocking approval; ping confirmed server active)

## Per-Finding Decision Table

| Finding ID | Severity | Consensus | Operator Decision | Modification Notes |
|------------|----------|-----------|-------------------|--------------------|
| TF-02 | MEDIUM | majority | **Applied** | Added note to plan.md clarifying OTLP uses outbound-only gRPC connections, not inbound port exposure. Distinguished from constitution's inbound binding restriction. |
| ES-01 | MEDIUM | majority | **Applied** | Standardized on "collection" as canonical term in spec.md. "Epic" retained only in parenthetical first-reference. |
| TF-03 | MEDIUM | single | **Deferred** | Schema versioning for event snapshots deferred to Phase 6 implementation — better addressed when Event model is built, as exact versioning strategy depends on implementation details. |

## High-Severity Findings (Auto-Applied)

| Finding ID | Severity | Consensus | Change Description |
|------------|----------|-----------|-------------------|
| RC-01 | HIGH | majority | Updated plan.md Constitution Check Principle VI to note justified exception for event ledger; added Complexity Tracking entry |
| TF-01 | HIGH | unanimous | Updated tasks.md T073 to require word-boundary matching and string literal exclusion in query sanitizer |
| RC-02 | MEDIUM→HIGH (elevated by unanimity) | unanimous | Standardized config parameter name to `event_ledger_max` across all artifacts |

## Artifacts Modified

| File | Changes |
| ---- | ------- |
| plan.md | Constitution Check Principle VI → "PASS with justified exception"; Complexity Tracking entry added; OTLP outbound connection note added |
| tasks.md | T073 description updated for robust query sanitization |
| spec.md | FR-013b config name standardized to `event_ledger_max`; terminology standardized to "collection" |

## Deferred Findings

| Finding ID | Severity | Reason |
|------------|----------|--------|
| TF-03 | MEDIUM | Schema versioning for event snapshots — implementation-phase decision; exact strategy depends on Event model design |

## Low-Severity Findings (Recorded as Suggestions)

| Finding ID | Summary | Recommendation |
|------------|---------|----------------|
| ES-02 | No scenario for get_event_history pagination beyond available events | Add boundary scenario during behavior refinement |
| RC-03 | No task for updating copilot-instructions.md error codes | Add documentation task if needed during Polish phase |
| ES-03 | Quickstart placeholder IDs could confuse | Add illustrative note to examples |

### Contract: Mcp Tools

# MCP Tool Contracts: Lifecycle Observability

**Feature**: 005-lifecycle-observability
**Date**: 2026-03-09

## New Tools

### query_graph

Execute a sandboxed read-only query against the workspace graph.

**Input Schema**:
```json
{
  "query": "string (required) — SurrealQL SELECT statement",
  "params": "object (optional) — parameterized query bindings"
}
```

**Output Schema**:
```json
{
  "rows": "array — query result rows",
  "row_count": "integer — number of rows returned",
  "truncated": "boolean — true if results were limited by query_row_limit",
  "elapsed_ms": "integer — query execution time in milliseconds"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `QUERY_REJECTED` (4010): Query contains write operations
- `QUERY_TIMEOUT` (4011): Query exceeded execution timeout
- `QUERY_INVALID` (4012): Query syntax is invalid

---

### get_event_history

Retrieve recent events from the event ledger.

**Input Schema**:
```json
{
  "entity_id": "string (optional) — filter by target entity ID",
  "kind": "string (optional) — filter by event kind",
  "limit": "integer (optional, default 50) — max events to return"
}
```

**Output Schema**:
```json
{
  "events": [
    {
      "id": "string",
      "kind": "string",
      "entity_table": "string",
      "entity_id": "string",
      "source_client": "string",
      "created_at": "string (ISO 8601)"
    }
  ],
  "total_count": "integer",
  "limit": "integer — requested limit"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound

---

### rollback_to_event

Roll workspace state back to a specific event.

**Input Schema**:
```json
{
  "event_id": "string (required) — event ID to rollback to"
}
```

**Output Schema**:
```json
{
  "rolled_back_events": "integer — number of events reversed",
  "conflicts": [
    {
      "event_id": "string",
      "entity_id": "string",
      "reason": "string"
    }
  ],
  "restored_entities": "integer — number of entities restored"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `ROLLBACK_DENIED` (3020): Agent rollback not permitted (allow_agent_rollback=false)
- `EVENT_NOT_FOUND` (3021): Specified event does not exist in ledger
- `ROLLBACK_CONFLICT` (3022): Rollback cannot be cleanly applied

---

### create_collection

Create a named collection (epic/workflow grouping).

**Input Schema**:
```json
{
  "name": "string (required) — collection name (unique within workspace)",
  "description": "string (optional) — collection description"
}
```

**Output Schema**:
```json
{
  "id": "string — collection record ID",
  "name": "string",
  "description": "string | null",
  "created_at": "string (ISO 8601)"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_EXISTS` (3030): Collection with this name already exists

---

### add_to_collection

Add tasks or sub-collections to a collection.

**Input Schema**:
```json
{
  "collection_id": "string (required) — collection to add to",
  "member_ids": "array<string> (required) — task or collection IDs to add"
}
```

**Output Schema**:
```json
{
  "added": "integer — number of members successfully added",
  "already_members": "integer — number already in the collection (skipped)"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_NOT_FOUND` (3031): Target collection does not exist
- `CYCLIC_COLLECTION` (3032): Adding would create a collection cycle

---

### remove_from_collection

Remove tasks or sub-collections from a collection.

**Input Schema**:
```json
{
  "collection_id": "string (required) — collection to remove from",
  "member_ids": "array<string> (required) — task or collection IDs to remove"
}
```

**Output Schema**:
```json
{
  "removed": "integer — number of members removed",
  "not_found": "integer — number that were not members (skipped)"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_NOT_FOUND` (3031): Target collection does not exist

---

### get_collection_context

Recursively retrieve all tasks and context within a collection hierarchy.

**Input Schema**:
```json
{
  "collection_id": "string (required) — collection to retrieve",
  "status_filter": "array<string> (optional) — filter tasks by status",
  "include_files": "boolean (optional, default true) — include associated file references"
}
```

**Output Schema**:
```json
{
  "collection": { "id": "string", "name": "string", "description": "string | null" },
  "tasks": [
    { "id": "string", "title": "string", "status": "string", "priority": "string" }
  ],
  "sub_collections": [
    { "id": "string", "name": "string", "task_count": "integer" }
  ],
  "total_tasks": "integer",
  "files": ["string — file paths associated with contained tasks"]
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_NOT_FOUND` (3031): Target collection does not exist

---

### get_health_report

Extended daemon health with latency percentiles, watcher status, and memory.

**Input Schema**:
```json
{}
```

**Output Schema**:
```json
{
  "version": "string",
  "uptime_seconds": "integer",
  "active_connections": "integer",
  "workspace_id": "string | null",
  "tool_call_count": "integer",
  "latency_us": {
    "p50": "integer",
    "p95": "integer",
    "p99": "integer"
  },
  "memory_mb": "integer | null — process RSS; null if lookup fails",
  "watcher_events": "integer",
  "last_watcher_event": "string | null — ISO 8601 timestamp"
}
```

**Errors**:
- (none — always available even without workspace binding)

## Modified Tool Contracts

### update_task (modified)

**New error codes**:
- `TASK_BLOCKED` (3015): Task has unresolved hard_blocker dependencies

**New response fields** (added to existing response):
```json
{
  "warnings": [
    {
      "type": "soft_dependency_incomplete",
      "dependency_id": "string",
      "dependency_title": "string"
    }
  ]
}
```

### add_dependency (modified)

**New error codes**:
- `CYCLIC_DEPENDENCY` (3003): Adding this edge would create a dependency cycle
<!-- SECTION:NOTES:END -->
