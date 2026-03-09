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

- [ ] T093 [P] Update src/tools/mod.rs MCP tool descriptions for discoverability
- [ ] T094 [P] Update .engram/.version if schema version changes
- [ ] T095 [P] Update README.md with new tool documentation and configuration parameters
- [ ] T096 Run cargo clippy to verify zero warnings across all new code
- [ ] T097 Run cargo test to verify all tests pass
- [ ] T098 Run quickstart.md validation — verify all example tool calls work end-to-end

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
