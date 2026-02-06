# Tasks: T-Mem Core MCP Daemon

**Input**: Design documents from `/specs/001-core-mcp-daemon/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: TDD is REQUIRED per constitution. Tests are included for all user stories.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single Rust workspace**: `src/`, `tests/` at repository root
- Binary: `src/bin/t-mem.rs`
- Library modules: `src/{module}/mod.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and Cargo workspace structure

- [ ] T001 Create Cargo.toml workspace manifest with dependencies per research.md
- [ ] T002 [P] Create src/lib.rs with crate-level attributes (`#![forbid(unsafe_code)]`, `#![warn(clippy::pedantic)]`)
- [ ] T003 [P] Create src/bin/t-mem.rs binary entrypoint skeleton
- [ ] T004 [P] Configure .cargo/config.toml for clippy and rustfmt settings
- [ ] T005 [P] Create rust-toolchain.toml specifying Rust 2024 edition (1.82+)
- [ ] T006 [P] Create .github/workflows/ci.yml for cargo fmt, clippy, test, audit

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Error Infrastructure

- [ ] T007 Create src/errors/mod.rs with TMemError enum wrapping all error types
- [ ] T008 [P] Create src/errors/codes.rs with error code constants (1xxx-5xxx per error taxonomy)
- [ ] T009 [P] Implement MCP-compatible JSON error response serialization in src/errors/mod.rs

### Domain Models

- [ ] T010 Create src/models/mod.rs re-exporting all model types
- [ ] T011 [P] Create src/models/task.rs with Task struct and TaskStatus enum per data-model.md
- [ ] T012 [P] Create src/models/spec.rs with Spec struct per data-model.md
- [ ] T013 [P] Create src/models/context.rs with Context struct per data-model.md
- [ ] T014 [P] Create src/models/graph.rs with DependencyType enum and relationship types

### Database Layer

- [ ] T015 Create src/db/mod.rs with database connection management
- [ ] T016 Create src/db/schema.rs with SurrealDB schema definitions (DEFINE TABLE statements)
- [ ] T017 [P] Create src/db/queries.rs with query builder functions

### Configuration

- [ ] T018 Create src/config/mod.rs with Config struct (port, timeout, paths)
- [ ] T019 [P] Implement environment variable and CLI arg parsing in src/config/mod.rs

### Observability

- [ ] T020 Create tracing subscriber initialization in src/lib.rs with JSON/pretty format toggle
- [ ] T021 [P] Add correlation ID middleware structure in src/server/mod.rs placeholder

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Daemon Connection & Workspace Binding (Priority: P1) 🎯 MVP

**Goal**: MCP clients can connect via SSE and bind to a Git repository workspace

**Independent Test**: Start daemon, connect via SSE curl, call set_workspace, verify ACTIVE state returned

### Tests for User Story 1

> **TDD: Write tests FIRST, ensure they FAIL, then implement**

- [ ] T022 [P] [US1] Contract test for set_workspace in tests/contract/lifecycle_test.rs
- [ ] T023 [P] [US1] Contract test for get_daemon_status in tests/contract/lifecycle_test.rs
- [ ] T024 [P] [US1] Contract test for get_workspace_status in tests/contract/lifecycle_test.rs
- [ ] T025 [P] [US1] Integration test for SSE connection lifecycle in tests/integration/connection_test.rs
- [ ] T026 [P] [US1] Unit test for workspace path validation in src/services/connection.rs

### Implementation for User Story 1

- [ ] T027 Create src/server/mod.rs with module structure
- [ ] T028 Create src/server/router.rs with axum Router setup and routes
- [ ] T029 Create src/server/sse.rs with SSE connection handling and connection ID assignment
- [ ] T030 Create src/server/mcp.rs with MCP JSON-RPC request/response handling
- [ ] T031 Create src/db/workspace.rs with workspace path hashing and namespace isolation
- [ ] T032 Create src/services/mod.rs with module structure
- [ ] T033 Create src/services/connection.rs with ConnectionState enum and lifecycle management
- [ ] T034 [US1] Implement set_workspace tool in src/tools/lifecycle.rs (path validation, hydration trigger)
- [ ] T035 [US1] Implement get_daemon_status tool in src/tools/lifecycle.rs
- [ ] T036 [US1] Implement get_workspace_status tool in src/tools/lifecycle.rs
- [ ] T037 [US1] Create src/tools/mod.rs with MCP tool registry and dispatch
- [ ] T038 [US1] Add SSE keepalive ping (15s interval) in src/server/sse.rs
- [ ] T039 [US1] Add connection timeout handling (60s configurable) in src/server/sse.rs
- [ ] T040 [US1] Wire up daemon main() in src/bin/t-mem.rs with graceful shutdown (SIGTERM/SIGINT)

**Checkpoint**: Daemon starts, accepts SSE connections, binds workspaces

---

## Phase 4: User Story 2 - Task State Management (Priority: P2)

**Goal**: Clients can create, update, and query tasks with graph relationships

**Independent Test**: Connect, update_task to change status, get_task_graph to verify, add_blocker to block

### Tests for User Story 2

- [ ] T041 [P] [US2] Contract test for update_task in tests/contract/write_test.rs
- [ ] T042 [P] [US2] Contract test for add_blocker in tests/contract/write_test.rs
- [ ] T043 [P] [US2] Contract test for register_decision in tests/contract/write_test.rs
- [ ] T044 [P] [US2] Contract test for get_task_graph in tests/contract/read_test.rs
- [ ] T045 [P] [US2] Contract test for check_status in tests/contract/read_test.rs
- [ ] T046 [P] [US2] Unit test for cyclic dependency detection in src/db/queries.rs
- [ ] T047 [P] [US2] Property test for Task serialization round-trip in tests/unit/proptest_models.rs

### Implementation for User Story 2

- [ ] T048 [US2] Implement task CRUD operations in src/db/queries.rs
- [ ] T049 [US2] Implement graph edge operations (depends_on, implements, relates_to) in src/db/queries.rs
- [ ] T050 [US2] Implement cyclic dependency detection before edge insert in src/db/queries.rs
- [ ] T051 [US2] Implement update_task tool in src/tools/write.rs
- [ ] T052 [US2] Implement add_blocker tool in src/tools/write.rs
- [ ] T053 [US2] Implement register_decision tool in src/tools/write.rs
- [ ] T054 [US2] Implement get_task_graph tool in src/tools/read.rs (recursive graph traversal)
- [ ] T055 [US2] Implement check_status tool in src/tools/read.rs
- [ ] T056 [US2] Add context note creation on task update in src/services/connection.rs

**Checkpoint**: Task CRUD and graph operations functional

---

## Phase 5: User Story 3 - Git-Backed Persistence (Priority: P3)

**Goal**: Workspace state serializes to .tmem/ files preserving user comments

**Independent Test**: Modify state, flush_state, verify tasks.md human-readable with comments preserved, hydrate verifies round-trip

### Tests for User Story 3

- [ ] T057 [P] [US3] Contract test for flush_state in tests/contract/write_test.rs
- [ ] T058 [P] [US3] Integration test for hydration from .tmem/ files in tests/integration/hydration_test.rs
- [ ] T059 [P] [US3] Integration test for dehydration preserving comments in tests/integration/hydration_test.rs
- [ ] T060 [P] [US3] Property test for markdown round-trip in tests/unit/proptest_serialization.rs
- [ ] T061 [P] [US3] Unit test for stale file detection in src/services/hydration.rs

### Implementation for User Story 3

- [ ] T062 [US3] Create src/services/hydration.rs with .tmem/ file parsing (pulldown-cmark)
- [ ] T063 [US3] Implement tasks.md parser extracting YAML frontmatter and descriptions
- [ ] T064 [US3] Implement graph.surql parser for RELATE statements
- [ ] T065 [US3] Implement .version and .lastflush file handling
- [ ] T066 [US3] Create src/services/dehydration.rs with DB → file serialization
- [ ] T067 [US3] Implement diff-match-patch comment preservation using `similar` crate
- [ ] T068 [US3] Implement atomic file writes (temp file + rename pattern)
- [ ] T069 [US3] Implement flush_state tool in src/tools/write.rs
- [ ] T070 [US3] Implement stale file detection comparing .lastflush to file mtime
- [ ] T071 [US3] Implement corruption recovery (delete DB, re-hydrate from files)
- [ ] T072 [US3] Wire hydration into set_workspace flow in src/tools/lifecycle.rs

**Checkpoint**: Git-backed persistence with comment preservation functional

---

## Phase 6: User Story 4 - Semantic Memory Query (Priority: P4)

**Goal**: Hybrid vector + keyword search returns relevant context for natural language queries

**Independent Test**: Populate specs/context, query_memory with natural language, verify ranked relevant results

### Tests for User Story 4

- [ ] T073 [P] [US4] Contract test for query_memory in tests/contract/read_test.rs
- [ ] T074 [P] [US4] Unit test for embedding generation in src/services/embedding.rs
- [ ] T075 [P] [US4] Unit test for hybrid scoring (0.7 vector + 0.3 keyword) in src/services/search.rs
- [ ] T076 [P] [US4] Integration test for lazy model download in tests/integration/embedding_test.rs

### Implementation for User Story 4

- [ ] T077 [US4] Create src/services/embedding.rs with fastembed-rs integration
- [ ] T078 [US4] Implement lazy model download to ~/.local/share/t-mem/models/
- [ ] T079 [US4] Implement embedding generation for spec and context content
- [ ] T080 [US4] Create src/services/search.rs with hybrid search logic
- [ ] T081 [US4] Implement vector similarity search using SurrealDB MTREE index
- [ ] T082 [US4] Implement keyword matching (BM25-style) for text content
- [ ] T083 [US4] Implement weighted score combination (0.7 * vector + 0.3 * keyword)
- [ ] T084 [US4] Implement query_memory tool in src/tools/read.rs
- [ ] T085 [US4] Add query token limit validation (500 tokens max, error 4001)
- [ ] T086 [US4] Wire embedding generation into hydration for missing embeddings

**Checkpoint**: Semantic search returns relevant ranked results

---

## Phase 7: User Story 5 - Multi-Client Concurrent Access (Priority: P5)

**Goal**: 10+ clients access same workspace concurrently without conflicts

**Independent Test**: Connect 10 clients, interleaved read/write, verify consistent state, no corruption

### Tests for User Story 5

- [ ] T087 [P] [US5] Stress test with 10 concurrent clients in tests/integration/concurrency_test.rs
- [ ] T088 [P] [US5] Test last-write-wins for simple fields in tests/integration/concurrency_test.rs
- [ ] T089 [P] [US5] Test append-only semantics for context in tests/integration/concurrency_test.rs
- [ ] T090 [P] [US5] Test FIFO serialization of concurrent flush_state calls in tests/integration/concurrency_test.rs

### Implementation for User Story 5

- [ ] T091 [US5] Implement connection registry with Arc<RwLock<HashMap>> in src/services/connection.rs
- [ ] T092 [US5] Implement per-workspace write lock for flush_state in src/services/dehydration.rs
- [ ] T093 [US5] Implement last-write-wins with updated_at timestamps in src/db/queries.rs
- [ ] T094 [US5] Verify append-only context insertion (no overwrite) in src/db/queries.rs
- [ ] T095 [US5] Add connection cleanup on disconnect in src/server/sse.rs
- [ ] T096 [US5] Implement workspace state preservation across client disconnects

**Checkpoint**: Multi-client concurrent access stable

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Performance optimization, documentation, final hardening

### Performance Validation

- [ ] T097 Benchmark cold start time (target: < 200ms) and document results
- [ ] T098 Benchmark hydration time with 1000 tasks (target: < 500ms)
- [ ] T099 Benchmark query_memory latency (target: < 100ms)
- [ ] T100 Benchmark update_task latency (target: < 10ms)
- [ ] T101 Profile memory usage idle and under load (targets: < 100MB / < 500MB)

### Documentation

- [ ] T102 Create README.md with installation and usage instructions
- [ ] T103 Add rustdoc comments to all public APIs in src/lib.rs
- [ ] T104 Update specs/001-core-mcp-daemon/quickstart.md with final implementation details

### Final Hardening

- [ ] T105 Run cargo audit and resolve any vulnerabilities
- [ ] T106 Run full test suite with --release optimizations
- [ ] T107 Verify all error codes match contracts/error-codes.md
- [ ] T108 Add graceful shutdown flush of all workspaces on SIGTERM

---

## Dependency Graph

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundational) ─────────────────────────────────────┐
    ↓                                                        │
Phase 3 (US1: Connection) ← MVP                              │
    ↓                                                        │
Phase 4 (US2: Tasks) ← Depends on US1                       │
    ↓                                                        │
Phase 5 (US3: Persistence) ← Depends on US1, US2            │
    ↓                                                        │
Phase 6 (US4: Search) ← Depends on US1, integrates US3      │
    ↓                                                        │
Phase 7 (US5: Concurrency) ← Depends on ALL previous        │
    ↓                                                        │
Phase 8 (Polish) ← Final validation after all stories ──────┘
```

## Parallel Execution Examples

**Within Phase 2 (Foundational)**:
- T007, T008, T009 can run in parallel (error infrastructure)
- T010, T011, T012, T013, T014 can run in parallel (models)
- T015, T016, T017 can run sequentially (DB layer)

**Within Phase 3 (US1)**:
- T022, T023, T024, T025, T026 can run in parallel (all tests)
- T027 → T028 → T029 → T030 (server layer sequential)
- T034, T035, T036 can run in parallel after T037 (tools)

**Across Phases (after Phase 2 complete)**:
- US1 implementation blocks US2, US3, US4, US5
- Within each US phase, tests can run in parallel before implementation

## Implementation Strategy

1. **MVP First**: Complete Phase 1-3 for minimal working daemon
2. **Incremental Delivery**: Each user story phase delivers testable functionality
3. **Test Coverage**: TDD required; 80% minimum per constitution
4. **Performance Last**: Optimize only after correctness validated (Phase 8)

**Suggested MVP Scope**: Phases 1-3 (Setup + Foundational + US1) deliver a daemon that accepts connections and binds workspaces.
