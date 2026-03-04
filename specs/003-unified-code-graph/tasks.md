# Tasks: Unified Code Knowledge Graph

**Input**: Design documents from `/specs/003-unified-code-graph/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/mcp-tools.json, contracts/error-codes.md, quickstart.md

**Tests**: Included per Constitution III (Test-First Development). TDD enforced — write tests first, verify they fail, then implement.

**Organization**: Tasks grouped by prerequisite phase (PRQ-001) then by user story (7 stories) to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Exact file paths included in descriptions

## Phase 0: Prerequisites (PRQ-001 — Codebase Rename)

**Purpose**: Rename all codebase references from engram / `engram` / `engram` / `engram` / `.engram` / `engram` to Agent Engram / `engram` / `.engram` / `ENGRAM`. No behavioral changes — mechanical find-and-replace only.

**GATE**: All verification gates (T011) must pass before Phase 1 begins.

- [X] T001 Rename binary source file from src/bin/engram.rs to src/bin/engram.rs and update internal content references (doc comments, module-level attributes)
- [X] T002 Update Cargo.toml — package name to `engram`, description to "Agent Engram MCP daemon", authors to "Engram Contributors", `[[bin]]` name to `engram` and path to `src/bin/engram.rs`
  > **Atomicity**: T001 and T002 MUST be applied in the same commit — renaming the binary without updating `Cargo.toml` (or vice versa) leaves the build broken.
- [X] T003 [P] Update src/config/mod.rs — all `ENGRAM_` env annotations to `ENGRAM_`, clap command name to `engram`, about text to "Agent Engram MCP daemon", default data directory path from `engram` to `engram`
- [X] T004 [P] Update src/lib.rs — `APP_NAME` constant from `"engram"` to `"engram"`, crate-level doc comments, tracing filter from `engram=debug` to `engram=debug`
- [X] T005 [P] Update src/errors/mod.rs — rename `EngramError` enum to `EngramError`, update all doc comments and inline references
- [X] T006 [P] Update src/db/ — DB storage path segment from `engram/db/` to `engram/db/` in mod.rs, any `engram` or `.engram` references in workspace.rs and queries.rs
- [X] T007 [P] Update src/services/ — embedding model cache path from `engram/models/` to `engram/models/` in embedding.rs, all `.engram` path references to `.engram` in hydration.rs, dehydration.rs, and config.rs
- [X] T008 [P] Update remaining src/ files — all `engram`, `engram`, `engram`, `engram`, `.engram` references in server/mod.rs, server/state.rs, server/mcp.rs, server/sse.rs, tools/mod.rs, tools/lifecycle.rs, tools/read.rs, tools/write.rs, and models/*.rs
- [X] T009 Update all tests/ files — all `use engram::` imports to `use engram::`, `.engram` path literals to `.engram`, `ENGRAM_` string literals to `ENGRAM_`, `engram` variable and function names to `engram` equivalents, and `"engram"` display strings to `"Engram"` across contract/, integration/, and unit/ directories
- [X] T010 [P] Update specs and documentation — all spec files (specs/001-core-mcp-daemon/, specs/002-enhanced-task-management/), design docs (data-model.md, quickstart.md, contracts/mcp-tools.json, contracts/error-codes.md), README.md, and copilot-instructions.md with new naming
- [X] T011 Run verification gates — `cargo check` (zero errors), `cargo test --all-targets` (all pass), `cargo clippy -- -D warnings` (zero warnings), case-insensitive grep for `t.mem|tmem|T.MEM|TMEM` across src/, tests/, and Cargo.toml returns zero matches

**Checkpoint**: Codebase fully renamed to "engram." All subsequent phases use the canonical name from the start.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add dependencies, error codes, configuration structures, and module declarations

- [X] T012 Add tree-sitter 0.24, tree-sitter-rust 0.23, and ignore 0.4 dependencies to Cargo.toml
- [X] T013 Switch embedding model from `AllMiniLML6V2` to `BGESmallENV15` and update `InitOptions` to `TextInitOptions` in src/services/embedding.rs
- [X] T014 [P] Add 7xxx error code constants (`PARSE_ERROR` through `SYNC_CONFLICT`) to src/errors/codes.rs
- [X] T015 [P] Add `CodeGraphError` enum with 7 variants and `#[from]` conversion to `EngramError` in src/errors/mod.rs
- [X] T016 [P] Add `CodeGraphConfig` and `EmbeddingConfig` structs with serde defaults to src/config/mod.rs
- [X] T017 [P] Add `pub mod parsing;` and `pub mod code_graph;` declarations to src/services/mod.rs
- [X] T018 [P] Add code graph model re-exports (`CodeFile`, `Function`, `Class`, `Interface`, `CodeEdge` types) to src/models/mod.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core models, schema, parsing service, and base queries that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T019 [P] Create `CodeFile` model struct with path, language, size_bytes, content_hash, last_indexed_at fields in src/models/code_file.rs
- [X] T020 [P] Create `Function` model struct with name, file_path, line range, signature, docstring, body, body_hash, token_count, embed_type, embedding, summary fields in src/models/function.rs
- [X] T021 [P] Create `Class` model struct (same schema as Function minus signature, maps to Rust `struct_item`) in src/models/class.rs
- [X] T022 [P] Create `Interface` model struct (same schema as Function minus signature, maps to Rust `trait_item`) in src/models/interface.rs
- [X] T023 [P] Create `CodeEdge` enums and structs for calls, imports, inherits_from, defines, concerns edge types in src/models/code_edge.rs
- [X] T024 Add `DEFINE TABLE`/`FIELD`/`INDEX` statements for code_file, function, class, interface nodes and calls, imports, inherits_from, defines, concerns edges to src/db/schema.rs
- [X] T025 Add code graph CRUD queries (insert/update/delete/lookup for nodes and edges) to src/db/queries.rs
- [X] T026 Create tree-sitter AST parsing service with node extraction for `function_item`, `struct_item`, `trait_item`, `impl_item` and edge discovery for `call_expression`, `use_declaration` in src/services/parsing.rs
- [X] T027 Add indexing-in-progress `AtomicBool` flag and `last_indexed_at` timestamp to `AppState` in src/server/state.rs
- [X] T028 [P] Add proptest `Arbitrary` implementations for `CodeFile`, `Function`, `Class`, `Interface`, `CodeEdge` types in tests/unit/proptest_models.rs
- [X] T029 [P] Add serde serialization round-trip property tests for code graph models in tests/unit/proptest_serialization.rs
- [X] T030 Add unit tests for tree-sitter node extraction (function, struct, trait, impl, call sites, use declarations) in tests/unit/parsing_test.rs

**Checkpoint**: Foundation ready — models, schema, parsing, and queries are in place. User story implementation can begin.

---

## Phase 3: User Story 1 — Code Structure Indexing (Priority: P1) MVP

**Goal**: Parse all workspace source files via tree-sitter, create code graph nodes (code_file, function, class, interface) with structural edges (calls, imports, inherits_from, defines), and generate per-symbol embeddings using the tiered strategy.

**Independent Test**: Call `index_workspace` on a workspace with Rust source files. Verify nodes exist in the graph with correct edges and embeddings. Verify Tier 1 vs Tier 2 classification is correct. Verify unsupported/oversized files are skipped.

### Tests for User Story 1

- [X] T031 [P] [US1] Add contract test for `index_workspace` (workspace-not-set returns error 1003, index-in-progress returns error 7003) in tests/contract/write_test.rs

### Implementation for User Story 1

- [X] T032 [US1] Create code_graph indexing orchestration service with file discovery (`ignore` crate for .gitignore + `code_graph.exclude_patterns` from config), parallel file parsing (`spawn_blocking`, concurrency bounded by `code_graph.parse_concurrency`), character-based token counting (4 chars/token), tiered embedding via batched `embed_texts()`, SSE progress event emission (FR-120), and batch edge creation in src/services/code_graph.rs
- [X] T033 [US1] Implement `index_workspace` tool handler that validates workspace, checks in-progress flag, delegates to code_graph service, and returns structured summary in src/tools/write.rs
- [X] T034 [US1] Add `index_workspace` match arm to `dispatch()` in src/tools/mod.rs
- [X] T035 [US1] Add integration test for full index round-trip (index workspace → verify nodes and edges in DB → verify embeddings generated → verify tier classification) in tests/integration/code_graph_test.rs

**Checkpoint**: `index_workspace` is functional. Workspace code is parsed into a navigable graph with embeddings. MVP is testable.

---

## Phase 4: User Story 2 — Graph-Backed Dependency Walking (Priority: P2)

**Goal**: Enable retrieval of a symbol's definition plus its graph neighborhood via BFS traversal. Falls back to vector search when the exact symbol name is not found. Provide `list_symbols` for agent discoverability (FR-150).

**Independent Test**: Index a workspace, call `map_code("dispatch", depth: 2)`. Verify the result contains the function definition plus all nodes reachable within 2 hops. Verify truncation at max_nodes. Verify vector search fallback for unknown symbol names. Call `list_symbols` and verify paginated results.

### Tests for User Story 2

- [x] T036 [P] [US2] Add contract test for `map_code` (workspace-not-set returns 1003, empty graph returns 7004 with suggestion) in tests/contract/read_test.rs
- [x] T037 [P] [US2] Add contract test for `list_symbols` (workspace-not-set returns 1003, empty graph returns 7004) in tests/contract/read_test.rs

### Implementation for User Story 2

- [x] T038 [US2] Add application-level BFS traversal queries (1-hop and multi-hop with max_nodes truncation) and symbol listing/filtering queries (by file_path, node_type, name_prefix with pagination) to src/db/queries.rs
- [x] T039 [US2] Implement `map_code` tool handler with exact-name lookup, BFS neighborhood expansion, vector-search fallback (FR-130), depth/max_nodes clamping (FR-149), and full source body loading (FR-148) in src/tools/read.rs
- [x] T040 [US2] Implement `list_symbols` tool handler with file_path, node_type, name_prefix filters and limit/offset pagination (FR-150) in src/tools/read.rs
- [x] T041 [US2] Add `map_code` and `list_symbols` match arms to `dispatch()` in src/tools/mod.rs

**Checkpoint**: `map_code` returns precise structural neighborhoods. `list_symbols` enables agents to discover valid symbol names. Agents can request exactly the code context they need.

---

## Phase 5: User Story 3 — Incremental Code Sync (Priority: P3)

**Goal**: Detect changed, added, and deleted files since last index and update only affected nodes. Use two-level hashing (file + symbol) to minimize re-embedding. Preserve concerns edges across file moves via hash-resilient identity.

**Independent Test**: Index a workspace, modify 3 files and delete 1, call `sync_workspace`. Verify only 4 files are re-processed, unchanged symbols keep their embeddings, deleted file nodes are removed, and concerns edges survive file moves.

### Tests for User Story 3

- [x] T042 [P] [US3] Add contract test for `sync_workspace` (workspace-not-set returns 1003, sync while indexing returns 7003) in tests/contract/write_test.rs

### Implementation for User Story 3

- [x] T043 [US3] Add two-level hash comparison logic (file-level content_hash then per-symbol body_hash) and selective re-embedding to src/services/code_graph.rs
- [x] T044 [US3] Add hash-resilient concerns edge relinking logic using `(name, body_hash)` tuple identity matching with orphan cleanup and context notes (FR-124) to src/services/code_graph.rs
- [x] T045 [US3] Implement `sync_workspace` tool handler that detects file changes, delegates to incremental sync, records sync context note (FR-125), and returns structured summary in src/tools/write.rs
- [x] T046 [US3] Add `sync_workspace` match arm to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Code graph stays current with minimal cost. Only changed symbols are re-embedded.

---

## Phase 6: User Story 4 — Cross-Region Task-to-Code Linking (Priority: P4)

**Goal**: Create and manage `concerns` edges between tasks (Region B) and code symbols (Region A). Implement `get_active_context` to return linked code neighborhoods for the highest-priority in-progress task.

**Independent Test**: Create a task, link it to 2 functions via `link_task_to_code`, call `get_active_context`. Verify the response includes the task plus full code neighborhoods of both linked functions. Unlink one function, verify it disappears from context.

### Tests for User Story 4

- [x] T047 [P] [US4] Add contract tests for `link_task_to_code` (workspace-not-set 1003, invalid task 3001, symbol-not-found 7004) and `unlink_task_from_code` in tests/contract/write_test.rs
- [x] T048 [P] [US4] Add integration test for cross-region concerns edge lifecycle (create link → `get_active_context` → unlink → verify removed) in tests/integration/cross_region_test.rs

### Implementation for User Story 4

- [x] T049 [US4] Add concerns edge CRUD queries (create by task+symbol name with idempotency per FR-152, delete by task+symbol name, orphan cleanup, list by task) to src/db/queries.rs
- [x] T050 [US4] Implement `link_task_to_code` tool handler that resolves symbol names to node IDs and creates idempotent concerns edges (FR-152) in src/tools/write.rs
- [x] T051 [US4] Implement `unlink_task_from_code` tool handler that removes matching concerns edges in src/tools/write.rs
- [x] T052 [US4] Implement `get_active_context` tool handler that returns all in-progress tasks, expands full code neighborhoods (with source bodies) for highest-priority task only, and returns symbol names only for remaining tasks (FR-127) in src/tools/read.rs
- [x] T053 [US4] Add `link_task_to_code`, `unlink_task_from_code`, and `get_active_context` match arms to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Tasks and code are unified via concerns edges. `get_active_context` returns grounded code context.

---

## Phase 7: User Story 5 — Unified Semantic Search (Priority: P5)

**Goal**: Perform a single natural language query that searches across both code symbols and task/context data, returning merged ranked results.

**Independent Test**: Populate workspace with billing-related tasks and payment-related code. Call `unified_search("billing logic")`. Verify results include both tasks and code symbols ranked by relevance. Verify region filter works.

### Tests for User Story 5

- [x] T054 [P] [US5] Add contract test for `unified_search` (workspace-not-set 1003, empty query 4001 per FR-157) in tests/contract/read_test.rs

### Implementation for User Story 5

- [x] T055 [US5] Add hybrid vector search queries across code tables (function, class, interface) and task tables (task, context, spec) with cosine similarity scoring to src/db/queries.rs
- [x] T056 [US5] Extend search service with cross-region result merging, ranking by descending cosine score, and region filtering in src/services/search.rs
- [x] T057 [US5] Implement `unified_search` tool handler with query embedding, empty query validation (FR-157), region dispatch, and merged response assembly (summary text only, not full bodies per FR-148 exemption) in src/tools/read.rs
- [x] T058 [US5] Add `unified_search` match arm to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Single query spans both code and task domains. Agents get holistic workspace results.

---

## Phase 8: User Story 6 — Impact Analysis Queries (Priority: P6)

**Goal**: Traverse code dependencies and cross-region concerns edges to find all tasks affected by changes to a specific code symbol.

**Independent Test**: Create 5 tasks, link 3 to functions that depend on `EngramError`. Call `impact_analysis("EngramError", depth: 2)`. Verify all 3 tasks appear with dependency paths. Verify status_filter narrows results.

### Tests for User Story 6

- [x] T059 [P] [US6] Add contract test for `impact_analysis` (workspace-not-set 1003, symbol-not-found 7004) in tests/contract/read_test.rs

### Implementation for User Story 6

- [x] T060 [US6] Add cross-region traversal queries (code BFS → collect node IDs → concerns edge lookup → task filtering by status) to src/db/queries.rs
- [x] T061 [US6] Implement `impact_analysis` tool handler with code neighborhood BFS, cross-region edge resolution, dependency path tracking, depth/max_nodes clamping (FR-149), task status filtering, and full source body loading (FR-148) in src/tools/read.rs
- [x] T062 [US6] Add `impact_analysis` match arm to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Agents can assess the blast radius of a code change across both code and task domains.

---

## Phase 9: User Story 7 — Code Graph Persistence (Priority: P7)

**Goal**: Serialize code graph metadata to `.engram/code-graph/` JSONL files during `flush_state` and hydrate the graph from JSONL + source files during `set_workspace`. Source bodies are NOT persisted — they are re-derived from source files.

**Independent Test**: Index a workspace, call `flush_state`, verify `.engram/code-graph/nodes.jsonl` and `edges.jsonl` exist with correct content. Delete SurrealDB state, call `set_workspace`, and verify the code graph is hydrated with persisted embeddings reused for unchanged symbols.

### Tests for User Story 7

- [x] T063 [P] [US7] Add integration test for code graph persistence round-trip (index → flush → clear DB → hydrate → verify embeddings and edges preserved within 1e-6 epsilon per SC-107) in tests/integration/hydration_test.rs
- [x] T064 [P] [US7] Add end-to-end integration test for full lifecycle (index → sync → query → persist → hydrate → query again) in tests/integration/code_graph_test.rs

### Implementation for User Story 7

- [x] T065 [US7] Extend dehydration service to serialize code graph nodes to `.engram/code-graph/nodes.jsonl` (metadata only, no source bodies, sorted by ID, atomic temp+rename) in src/services/dehydration.rs
- [x] T066 [US7] Extend dehydration service to serialize code graph edges to `.engram/code-graph/edges.jsonl` (all edge types including concerns, sorted by type+from+to, atomic temp+rename) in src/services/dehydration.rs
- [x] T067 [US7] Extend hydration service to load code graph from JSONL metadata, parse source files for bodies, compare body_hash for diff-rehydration, re-embed only changed symbols, and discard metadata for deleted files in src/services/hydration.rs. On JSONL parse failure (corrupt/truncated lines), log a warning, skip the bad line, and fall back to full re-index for affected symbols (FR-135)
- [x] T068 [US7] Extend `flush_state` tool to include code graph serialization alongside existing task/context persistence, and return error 7003 if indexing is in progress (FR-153) in src/tools/write.rs
- [x] T069 [US7] Extend `set_workspace` tool to trigger code graph hydration after existing workspace setup in src/tools/lifecycle.rs
- [x] T070 [US7] Extend `get_workspace_status` to include code_graph stats (file_count, function_count, class_count, interface_count, edge_count, concerns_count, last_indexed_at) in src/tools/lifecycle.rs

**Checkpoint**: Code graph metadata survives daemon restarts. Embeddings are reused for unchanged symbols. Full persistence lifecycle is complete.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Validation, cleanup, and documentation

- [x] T071 [P] Validate all quickstart.md scenarios against implemented tools
- [x] T072 Run `cargo clippy --all-targets -- -D warnings` and fix all pedantic warnings across new files
- [x] T073 Run full test suite (`cargo ci`) and verify all tests pass
- [x] T074 [P] Validate performance against success criteria (SC-101 through SC-116) on representative workspace
- [x] T075 [P] Add startup failure smoke test: verify daemon returns error 5001 with `suggestion: "try restarting"` when embedding model fails to load at startup (FR-154)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Prerequisites (Phase 0)**: No dependencies — start immediately. GATES all subsequent phases
- **Setup (Phase 1)**: Depends on Phase 0 completion
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 Indexing (Phase 3)**: Depends on Foundational — provides the populated graph that US2–US7 operate on
- **US2 Map Code (Phase 4)**: Depends on US1 (needs indexed graph to traverse)
- **US3 Sync (Phase 5)**: Depends on US1 (sync modifies an existing index)
- **US4 Cross-Region (Phase 6)**: Depends on US1 (needs code nodes to link to) + existing task infrastructure
- **US5 Unified Search (Phase 7)**: Depends on US1 (needs code embeddings) + existing search infrastructure
- **US6 Impact Analysis (Phase 8)**: Depends on US2 (BFS traversal) + US4 (concerns edges)
- **US7 Persistence (Phase 9)**: Depends on US1 (needs graph to persist)
- **Polish (Phase 10)**: Depends on all user stories being complete

### User Story Dependencies

```text
                    ┌──────────┐
                    │  PRQ-001 │
                    │ Phase 0  │  ◄── GATE: Rename must complete first
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │  Setup   │
                    │ Phase 1  │
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │Foundation│
                    │ Phase 2  │
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │  US1     │  ◄── MVP: Code Structure Indexing
                    │ Phase 3  │
                    └────┬─────┘
                         │
           ┌─────────────┼─────────────┬────────────┐
           │             │             │            │
      ┌────▼───┐   ┌────▼───┐   ┌────▼───┐  ┌────▼───┐
      │  US2   │   │  US3   │   │  US4   │  │  US5   │
      │Map Code│   │  Sync  │   │Linking │  │Search  │
      │Phase 4 │   │Phase 5 │   │Phase 6 │  │Phase 7 │
      └────┬───┘   └────────┘   └────┬───┘  └────────┘
           │                         │
           └───────────┬─────────────┘
                       │
                  ┌────▼───┐
                  │  US6   │  ◄── Requires BFS (US2) + Concerns (US4)
                  │Impact  │
                  │Phase 8 │
                  └────────┘
                       │
                  ┌────▼───┐
                  │  US7   │  ◄── Can start after US1, best after US4
                  │Persist │
                  │Phase 9 │
                  └────────┘
                       │
                  ┌────▼───┐
                  │ Polish  │
                  │Phase 10 │
                  └────────┘
```

### Within Each User Story

- Contract tests MUST be written and FAIL before implementation
- Models/queries before service logic
- Service logic before tool handlers
- Tool handlers before dispatch registration
- Integration tests validate the full story

### Parallel Opportunities

**Phase 0** (rename tasks that touch independent files run in parallel):

```text
T003: config  ║  T004: lib.rs  ║  T005: errors  ║  T006: db/  ║  T007: services/  ║  T008: remaining src/  ║  T010: specs/docs
    then: T009 (tests — depends on import path changes), T011 (verification — must be last)
```

**Phase 1** (all [P] tasks can run in parallel):

```text
T014: error codes  ║  T015: error enum  ║  T016: config  ║  T017: mod decls  ║  T018: re-exports
```

**Phase 2** (all model tasks can run in parallel):

```text
T019: CodeFile  ║  T020: Function  ║  T021: Class  ║  T022: Interface  ║  T023: CodeEdge
    then: T024 (schema), T025 (queries), T026 (parsing) — sequential
T028: proptest models  ║  T029: proptest serialization  (parallel with each other)
```

**After US1 completes**, US2/US3/US4/US5 can start in parallel:

```text
US2: Map Code (Phase 4)  ║  US3: Sync (Phase 5)  ║  US4: Linking (Phase 6)  ║  US5: Search (Phase 7)
    then: US6: Impact Analysis (Phase 8) — requires US2 + US4
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 0: Prerequisites (PRQ-001 rename)
2. Complete Phase 1: Setup
3. Complete Phase 2: Foundational
4. Complete Phase 3: User Story 1 — Code Structure Indexing
5. **STOP and VALIDATE**: Index the engram workspace itself, verify nodes/edges/embeddings
6. This provides a populated code graph for all downstream stories

### Incremental Delivery

1. PRQ-001 → Codebase uses canonical "engram" name everywhere
2. Setup + Foundational → Infrastructure ready
3. US1 → Index works → **MVP** (code graph exists and is queryable via raw DB)
4. US2 → `map_code` + `list_symbols` work → Agents can navigate code structure
5. US3 → `sync_workspace` works → Graph stays current
6. US4 → `link_task_to_code` + `get_active_context` → Code and tasks are unified
7. US5 → `unified_search` → Single-query across all regions
8. US6 → `impact_analysis` → Strategic refactoring support
9. US7 → Persistence → Graph survives restarts

### Task Count Summary

| Phase | Tasks | Parallel |
|-------|-------|----------|
| Phase 0: PRQ-001 Rename | 11 | 7 |
| Phase 1: Setup | 7 | 5 |
| Phase 2: Foundational | 12 | 7 |
| Phase 3: US1 Indexing | 5 | 1 |
| Phase 4: US2 Map Code | 6 | 2 |
| Phase 5: US3 Sync | 5 | 1 |
| Phase 6: US4 Linking | 7 | 2 |
| Phase 7: US5 Search | 5 | 1 |
| Phase 8: US6 Impact | 4 | 1 |
| Phase 9: US7 Persist | 8 | 2 |
| Phase 10: Polish | 5 | 3 |
| Phase 11: Adversarial Remediation | 36 | 12 |
| **Total** | **111** | **44** |

---

## Phase 11: Adversarial Remediation

*Added 2026-02-28. 36 tasks across 5 sub-phases. CRITICAL+HIGH = mandatory; MEDIUM+LOW = should-fix.*

### Phase 11a: Correctness Fixes (CRITICAL) — 4 tasks

- [x] **T076** — Implement embedding write-back in `index_workspace` (FR-160)
  - Update `code_graph.rs:340-355` to persist `embed_texts()` results back to function/class/interface records
  - Remove dead `embed_indices` vector (FR-171 / U-012)
  - Write test: index a small workspace → verify `embedding` field is non-zero for all symbols
  - Write test: index with `embeddings` feature disabled → verify `embedding` field is empty vec
  - **Files**: `src/services/code_graph.rs`, `src/db/queries.rs`

- [x] **T077** — Implement embedding write-back in `sync_workspace` (FR-160)
  - Mirror T076 logic for newly-added and modified files during sync
  - Write test: sync after adding a new file → new symbol has non-zero embedding
  - **Files**: `src/services/code_graph.rs`

- [x] **T078** — Write ADR for deferred SSE progress events (FR-161 / U-002)
  - Create `docs/adrs/0011-deferred-sse-progress-events.md`
  - Document: current behavior, spec commitment (FR-120), deferral rationale, resolution path
  - **Files**: `docs/adrs/0011-deferred-sse-progress-events.md`

- [x] **T079** — Write ADR for deferred parallel parsing (FR-162 / U-003)
  - Create `docs/adrs/0012-deferred-parallel-parsing.md`
  - Document: current behavior, spec commitment (FR-119), deferral rationale, resolution path
  - Ensure `parse_concurrency` config field remains in schema
  - **Files**: `docs/adrs/0012-deferred-parallel-parsing.md`, `src/models/config.rs`

### Phase 11b: Data Integrity (HIGH) — 13 tasks

- [x] **T080** — Implement body re-derivation on hydration (FR-163 / U-004)
  - After JSONL hydration, re-parse each code_file via tree-sitter to populate `body` fields
  - Match `content_hash` to detect changed files; mark changed files for re-index
  - Discard symbols from deleted files
  - Write test: hydrate → verify `body` is populated; modify source → verify re-index flag set
  - **Files**: `src/services/hydration.rs`, `src/services/parsing.rs`

- [x] **T081** — Fix `discover_files` exclude pattern accumulation (FR-164 / U-005)
  - Replace break-after-first-pattern with full `OverrideBuilder` accumulation
  - Write test: 3 exclude patterns → all 3 applied
  - **Files**: `src/services/code_graph.rs`

- [x] **T082** — Fix `map_code` multi-match behavior (FR-165 / U-006)
  - Return disambiguation array instead of BFS on matches[0]
  - Write test: workspace with 2 same-named functions → response has `matches` array, `root: null`
  - **Files**: `src/tools/read.rs`

- [x] **T083** — Deduplicate `connect_db` calls in 4 handlers (FR-166 / U-007)
  - Single `connect_db()` per handler, clone for query structs
  - Affected: `get_active_context`, `link_task_to_code`, `unlink_task_from_code`, `impact_analysis`
  - **Files**: `src/tools/read.rs`, `src/tools/write.rs`

- [x] **T084** — Fix `discover_files` symlink following (FR-169 / U-010)
  - Add `.follow_links(false)` to `WalkBuilder`
  - Write test: create symlink outside workspace → not traversed
  - **Files**: `src/services/code_graph.rs`

- [x] **T085** — Add `is_indexing()` guard to `map_code` and `impact_analysis` (FR-170 / U-011)
  - Return error 7003 when indexing is active
  - Write test: set indexing flag → call map_code → get 7003 error
  - **Files**: `src/tools/read.rs`

- [x] **T086** — Fix `list_symbols` error code for empty graph (FR-172 / U-013)
  - Return empty array on no-filter empty graph; use 7004 only for name_prefix miss
  - Write test: empty graph + no filter → Ok with empty symbols; empty graph + filter → 7004
  - **Files**: `src/tools/read.rs`

- [x] **T087** — Implement sync context note (FR-173 / U-014)
  - Insert Context record with sync summary after successful sync_workspace
  - Write test: sync → verify context record exists with correct summary fields
  - **Files**: `src/tools/write.rs`, `src/db/queries.rs`

- [x] **T088** — Implement hydration fallback tracking (FR-174 / U-015)
  - Track code_file paths with parse-failed lines; add `hydration_warnings` to result
  - Write test: JSONL with corrupt line → hydration completes with warning; next sync re-indexes
  - **Files**: `src/services/hydration.rs`

- [x] **T089** — Clamp `map_code` depth lower-bound (FR-175 / U-016)
  - Change `.min(max)` to `.clamp(1, max)`
  - Write test: depth=0 → response has depth=1
  - **Files**: `src/tools/read.rs`

- [x] **T090** — Fix `strip_prefix` fallback path leak (FR-176 / U-017)
  - Replace `unwrap_or(file_path)` with match+skip+warn
  - Write test: file outside workspace root → skipped, warning logged
  - **Files**: `src/services/code_graph.rs`

- [x] **T091** — Fix `IndexResult` serialization error code (FR-199 / U-040)
  - Map to `SystemError::DatabaseError` not `InvalidParams`
  - Write test: force serialization failure → verify error code is 5001 not 5005
  - **Files**: `src/tools/write.rs`, `src/errors/codes.rs`

- [x] **T092** — Remove dead `embed_indices` code (FR-171 / U-012)
  - Remove vector construction and all references after T076 integrates write-back
  - This task is performed as cleanup after T076; no separate test needed beyond T076's
  - **Files**: `src/services/code_graph.rs`

### Phase 11c: Performance & Queries (MEDIUM) — 7 tasks

- [x] **T093** — Implement server-side vector search for `unified_search` (FR-167 / U-008)
  - Add `vector_search_contexts()` and `vector_search_specs()` to Queries
  - Replace full-table-scan + in-memory sort with DB-side K-NN or LIMIT query
  - Write test: 100 contexts → only top-K returned; no full heap load
  - **Files**: `src/db/queries.rs`, `src/tools/read.rs`

- [x] **T094** — Implement COUNT queries for `get_workspace_status` (FR-168 / U-009)
  - Add `count_code_files()`, `count_functions()`, `count_classes()`, `count_interfaces()`, `count_code_edges()`
  - Replace 5 full-table loads with COUNT GROUP ALL
  - Write test: workspace with known counts → status matches
  - **Files**: `src/db/queries.rs`, `src/tools/lifecycle.rs`

- [x] **T095** — Write ADR for deferred startup smoke test (FR-185 / U-026)
  - Create `docs/adrs/0014-deferred-startup-smoke-test.md`
  - **Files**: `docs/adrs/0014-deferred-startup-smoke-test.md`

- [x] **T096** — Batch concerns query for `get_active_context` (FR-182 / U-023)
  - Add `list_concerns_for_tasks(ids: &[&str])` to CodeGraphQueries
  - Replace N per-task queries with single batch
  - Write test: 3 tasks with concerns → single query returns all
  - **Files**: `src/db/queries.rs`, `src/tools/read.rs`

- [x] **T097** — Atomic upsert for `link_task_to_code` (FR-183 / U-024)
  - Replace EXISTS-then-CREATE with RELATE … ON DUPLICATE KEY IGNORE
  - Write test: concurrent link calls → no duplicate edges, no errors
  - **Files**: `src/tools/write.rs`, `src/db/queries.rs`

- [x] **T098** — Add `ensure_workspace` guard to `get_active_context` (FR-184 / U-025)
  - Add missing guard at handler entry
  - Write test: call without workspace → 1001 error
  - **Files**: `src/tools/read.rs`

- [x] **T099** — Add keyword scoring for tasks in `unified_search` (FR-189 / U-030)
  - Add term-match scoring on task `title` + `description` alongside cosine
  - Write test: search for task keyword → task appears in results even with zero embedding
  - **Files**: `src/tools/read.rs`

### Phase 11d: Edge & Linking Fixes (MEDIUM) — 6 tasks

- [x] **T100** — Handle import edges during indexing (FR-178 / U-019)
  - Either persist via `create_imports_edge()` or log dropped count with ADR
  - Write test: file with `use` statements → import edges persisted (or dropped with count)
  - **Files**: `src/services/code_graph.rs`, `src/db/queries.rs`

- [x] **T101** — Track cross-file dropped edges count (FR-179 / U-020)
  - Add `cross_file_edges_dropped: usize` to IndexResult and SyncResult
  - Write ADR: `docs/adrs/0013-cross-file-call-edges.md`
  - **Files**: `src/services/code_graph.rs`, `docs/adrs/0013-cross-file-call-edges.md`

- [x] **T102** — Fix method call false positives in `extract_calls_from_body` (FR-180 / U-021)
  - Skip `field_expression` call targets; only emit edges for `identifier` and `scoped_identifier`
  - Add blocklist for `new`, `default`, `into`, `clone`, `from`
  - Write test: code with method calls → no false `Calls` edges
  - **Files**: `src/services/parsing.rs`

- [x] **T103** — Normalize `task_id` prefix in `link_task_to_code` (FR-186 / U-027)
  - Strip `"task:"` prefix if present before DB lookup
  - Write test: `link_task_to_code("task:abc")` and `link_task_to_code("abc")` both succeed
  - **Files**: `src/tools/write.rs`

- [x] **T104** — Explicit concerns edge deletion in `sync_workspace` (FR-190 / U-031)
  - DELETE old concerns edges before CREATE new ones during relinking
  - Write test: relink concerns → old edges gone, new edges present
  - **Files**: `src/services/code_graph.rs`, `src/db/queries.rs`

- [x] **T105** — Qualify `find_function_id` key for impl methods (FR-191 / U-032)
  - Use `"{impl_type}::{func_name}"` for methods in impl blocks
  - Write test: two methods named `new` in different impls → distinct IDs
  - **Files**: `src/services/parsing.rs`

### Phase 11e: Cleanup & Documentation (LOW) — 6 tasks

- [x] **T106** — Remove dead `brief` and `fields` from `MapCodeParams` (FR-192 / U-033)
  - Remove fields; return `InvalidParams` if caller supplies them
  - Update MCP tool contract docs
  - **Files**: `src/tools/read.rs`, `specs/003-unified-code-graph/contracts/`

- [x] **T107** — Fix JSONL zero-vector serialization (FR-177 / U-018)
  - Serialize `embedding` as `null` when all-zeros or embeddings feature disabled
  - Hydration: treat `null`/missing embedding as "needs re-embedding"
  - Write test: dehydrate with zero vec → JSONL has `null`; hydrate `null` → marked for re-embed
  - **Files**: `src/services/dehydration.rs`, `src/services/hydration.rs`

- [x] **T108** — Fix `truncate_summary` char boundary safety (FR-181 / U-022)
  - Replace `&text[..N]` with char-boundary-safe indexing
  - Write test: UTF-8 multi-byte string → no panic; truncated at valid boundary
  - **Files**: `src/tools/read.rs`, `src/services/hydration.rs`

- [x] **T109** — Add `max_nodes` parameter to `impact_analysis` (FR-187 / U-028)
  - Add `max_nodes` to `ImpactAnalysisParams` (clamped 1..=100)
  - Write test: impact_analysis with max_nodes=5 → at most 5 nodes returned
  - **Files**: `src/tools/read.rs`

- [x] **T110** — Tier-2 summary: add body preview when no docstring (FR-188 / U-029)
  - Include first 5 lines / 256 chars as preview fallback
  - Write test: function with no docstring → summary includes body preview
  - **Files**: `src/services/code_graph.rs`

- [x] **T111** — Documentation & error-code cleanups (FR-195, FR-196, FR-197, FR-198)
  - Add error code 7005 gap comment in `errors/codes.rs`
  - Fix `unified_search` doc comment FR citations
  - Add doc comment to `max_file_size_bytes` clarifying byte semantics
  - Correct `unified_search` doc comment to cite FR-131/FR-157
  - **Files**: `src/errors/codes.rs`, `src/tools/read.rs`, `src/models/config.rs`

### Phase 11 Task Dependencies

```
T076 (embed write-back index) ──→ T077 (embed write-back sync)
T076 ──→ T092 (remove dead embed_indices)
T076 ──→ T093 (vector search needs real embeddings)
T076 ──→ T099 (task keyword scoring meaningful only with real embeddings)
T076 ──→ T107 (JSONL null-vector depends on knowing what "real" vs "zero" means)
T080 (body re-derivation) ──→ T110 (body preview needs body populated)
T081 (exclude patterns) ──∥── T084 (symlinks) — parallel, both in discover_files
T101 (cross-file ADR) ──∥── T102 (method call fix) — parallel
T078, T079, T095 — ADR tasks, independent of all code tasks
```

### Phase 11 Parallelism

| Parallel Group | Tasks | Max Parallel |
|----------------|-------|-------------|
| 11a: ADRs | T078, T079 | 2 |
| 11a: Embed write-back | T076 → T077 | 1 (sequential) |
| 11b: discover_files fixes | T081, T084 | 2 |
| 11b: Tool guards & codes | T085, T086, T089, T091 | 4 |
| 11b: Data integrity | T080, T088, T090 | 3 |
| 11b: Handler fixes | T082, T083, T087 | 3 |
| 11c: Query perf | T093, T094, T096 | 3 |
| 11c: Other | T095, T097, T098, T099 | 4 |
| 11d: Edges | T100, T101, T102 | 3 |
| 11d: Linking | T103, T104, T105 | 3 |
| 11e: All | T106–T111 | 6 |
