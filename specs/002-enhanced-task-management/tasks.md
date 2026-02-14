# Tasks: Enhanced Task Management

**Input**: Design documents from `/specs/002-enhanced-task-management/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: TDD enforced per Constitution III — each user story phase has a Red (tests first, expect failure) and Green (implementation, make tests pass) sub-phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new dependency and prepare project structure for enhanced task management

- [ ] T001 Add `toml = "0.8"` dependency to Cargo.toml for workspace config parsing (FR-064)
- [ ] T002 [P] Create placeholder module files: src/models/label.rs, src/models/comment.rs, src/models/config.rs, src/services/compaction.rs, src/services/config.rs, src/services/output.rs
- [ ] T003 [P] Create test file stubs: tests/integration/enhanced_features_test.rs, tests/integration/performance_test.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core models, error codes, DB schema, and property tests that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 [P] Extend Task struct with 9 new fields (priority, priority_order, issue_type, assignee, defer_until, pinned, compaction_level, compacted_at) and 2 reserved fields (workflow_state, workflow_id) in src/models/task.rs (FR-026, FR-043, FR-047, FR-050, FR-052, FR-040, FR-067)
- [ ] T005 [P] Create Label struct in src/models/label.rs: id, task_id, name, created_at with serde derives and validation (FR-031)
- [ ] T006 [P] Create Comment struct in src/models/comment.rs: id, task_id, content, author, created_at with serde derives (FR-061)
- [ ] T007 [P] Create WorkspaceConfig, CompactionConfig, BatchConfig structs with Default impls and serde defaults in src/models/config.rs (FR-064, FR-065)
- [ ] T008 [P] Extend DependencyType enum from 2 to 8 variants (add child_of, blocked_by, duplicate_of, related_to, predecessor, successor) in src/models/graph.rs (FR-035)
- [ ] T009 [P] Implement compute_priority_order() utility function with unit tests in src/models/task.rs: parse numeric suffix from priority string, return u32 (FR-026)
- [ ] T010 Update src/models/mod.rs to declare and re-export Label, Comment, WorkspaceConfig, CompactionConfig, BatchConfig
- [ ] T011 [P] Add error code constants 3005–3012 (TASK_ALREADY_CLAIMED through TASK_NOT_CLAIMABLE) and 6001–6003 (CONFIG_PARSE_ERROR through UNKNOWN_CONFIG_KEY) in src/errors/codes.rs (FR-069, FR-070)
- [ ] T012 [P] Add TaskError variants (AlreadyClaimed, LabelValidation, BatchPartialFailure, CompactionFailed, InvalidPriority, InvalidIssueType, DuplicateLabel, NotClaimable) and ConfigError enum (ParseError, InvalidValue, UnknownKey) to src/errors/mod.rs (FR-071)
- [ ] T013 Extend SurrealDB schema in src/db/schema.rs: DEFINE FIELD for all new task fields with defaults, DEFINE TABLE label SCHEMAFULL and comment SCHEMAFULL, DEFINE INDEX for task_priority, task_assignee, task_defer_until, task_issue_type, task_pinned, task_compaction, label_task_name (UNIQUE), label_name, comment_task; implement `.tmem/.version` bump from 1.0.0 to 2.0.0 on schema bootstrap
- [ ] T014 [P] Add property tests for extended Task, Label, Comment, WorkspaceConfig, and 8-variant DependencyType serde JSON round-trips in tests/unit/proptest_models.rs (FR-068)
- [ ] T015 [P] Add YAML frontmatter serialization round-trip property tests for enhanced Task (with labels array, all new fields) in tests/unit/proptest_serialization.rs (SC-019)
- [ ] T016 Extend AppState to store Option\<WorkspaceConfig\> alongside workspace snapshot in src/server/state.rs
- [ ] T017 Register 15 new tool names in dispatch() match skeleton in src/tools/mod.rs: get_ready_work, add_label, remove_label, add_dependency, get_compaction_candidates, apply_compaction, claim_task, release_task, defer_task, undefer_task, pin_task, unpin_task, get_workspace_statistics, batch_update_tasks, add_comment — all stubs returning WorkspaceNotSet

**Checkpoint**: Foundation ready — all models, errors, schema, and dispatch stubs in place. User story implementation can begin.

---

## Phase 3: User Story 1 — Priority-Based Ready-Work Queue (Priority: P1) MVP

**Goal**: `get_ready_work` returns unblocked, undeferred, incomplete tasks sorted by pinned → priority → creation date with limit and 4 filter dimensions.

**Independent Test**: Create 20 tasks across priority levels, block 5, defer 3, call `get_ready_work` and verify filtering + sort order.

### Red Phase (Tests First — Expect Failure)

- [ ] T018 [US1] Write contract tests for get_ready_work in tests/contract/read_test.rs: workspace-not-set error (1003), basic call returns tasks, limit parameter caps results, empty workspace returns empty list (FR-027, FR-028)

### Green Phase (Implementation)

- [ ] T019 [US1] Implement ready-work SurrealQL query in src/db/queries.rs: WHERE status NOT IN [done, blocked], defer_until IS NULL OR \<= now(), NOT IN blocking subquery (hard_blocker, blocked_by where out.status != done), NOT IN duplicate_of subquery; ORDER BY pinned DESC, priority_order ASC, created_at ASC; LIMIT $limit (FR-027, FR-028, FR-030, FR-037, FR-054)
- [ ] T020 [US1] Implement get_ready_work tool handler in src/tools/read.rs: parse params (limit, label, priority, issue_type, assignee, brief, fields), call query, serialize to TaskSummary array, return total_eligible count (FR-027, FR-028)
- [ ] T021 [P] [US1] Add label filter dimension to ready-work query via parameterized WHERE clause in src/db/queries.rs: AND-filter using label table join (FR-029, FR-033)
- [ ] T022 [P] [US1] Add priority threshold filter to ready-work query in src/db/queries.rs: WHERE priority_order \<= compute_priority_order($threshold) (FR-029)
- [ ] T023 [P] [US1] Add issue_type filter to ready-work query in src/db/queries.rs: WHERE issue_type = $type (FR-029)
- [ ] T024 [P] [US1] Add assignee filter to ready-work query in src/db/queries.rs: WHERE assignee = $assignee (FR-029)
- [ ] T025 [US1] Integration test in tests/integration/enhanced_features_test.rs: 20 tasks at p0–p4, block 5 with hard_blocker, defer 3 to future, pin 1 low-priority; verify get_ready_work returns 12 tasks, pinned first, sorted by priority then created_at; verify limit=5 caps results (SC-011)

**Checkpoint**: `get_ready_work` fully functional with all 4 filter dimensions. US1 independently testable.

---

## Phase 4: User Story 2 — Task Priorities and Labels (Priority: P2)

**Goal**: Assign priorities, add/remove labels with validation, AND-filter by labels, serialize labels in YAML frontmatter.

**Independent Test**: Create tasks with different priorities and labels, filter for multi-label AND match, verify correct results.

### Red Phase (Tests First — Expect Failure)

- [ ] T026 [P] [US2] Write contract tests for add_label and remove_label in tests/contract/write_test.rs: workspace-not-set (1003), valid add returns label_count, duplicate label returns error 3011, label not in allowed_labels returns error 3006 (FR-032, FR-034)

### Green Phase (Implementation)

- [ ] T027 [US2] Implement label CRUD queries in src/db/queries.rs: insert_label with UNIQUE check (error 3011 on conflict), delete_label, get_labels_for_task, filter_tasks_by_labels using GROUP BY + HAVING count() for AND logic (FR-031, FR-032, FR-033)
- [ ] T028 [US2] Implement add_label tool handler in src/tools/write.rs: parse task_id and label, validate against WorkspaceConfig.allowed_labels if set (error 3006), call insert_label, return task_id + label + label_count (FR-032, FR-034)
- [ ] T029 [US2] Implement remove_label tool handler in src/tools/write.rs: parse task_id and label, call delete_label, return task_id + label + remaining label_count (FR-032)
- [ ] T030 [US2] Extend update_task handler in src/tools/write.rs to accept priority param: compute priority_order via compute_priority_order(), validate if needed, create context note recording priority change (FR-026)
- [ ] T031 [US2] Extend hydration to parse labels array from task YAML frontmatter and populate label table via insert_label in src/services/hydration.rs (FR-031b)
- [ ] T032 [US2] Extend dehydration to query labels per task and write labels array into YAML frontmatter in src/services/dehydration.rs (FR-031b)
- [ ] T033 [US2] Integration test in tests/integration/enhanced_features_test.rs: create 5 tasks with varying labels, add_label, remove_label; filter by \["frontend", "bug"\] AND logic verifies intersection; flush → rehydrate → verify labels preserved (SC-019)

**Checkpoint**: Priorities and labels fully functional including AND-filtering and round-trip serialization. US2 independently testable.

---

## Phase 5: User Story 3 — Enhanced Dependency Graph (Priority: P3)

**Goal**: 8 dependency types via `add_dependency` tool, cycle detection across all types, `duplicate_of` exclusion from ready-work, parent-child surfacing.

**Independent Test**: Parent with children, duplicate, blocked_by — verify all render correctly in `get_task_graph`.

### Red Phase (Tests First — Expect Failure)

- [ ] T034 [P] [US3] Write contract tests for add_dependency in tests/contract/write_test.rs: workspace-not-set (1003), valid add of each type, self-reference rejection, cycle rejection (3003) (FR-035b, FR-036)

### Green Phase (Implementation)

- [ ] T035 [US3] Implement add_dependency query in src/db/queries.rs: validate dependency_type against 8-variant enum, reject self-reference (in == out), cycle detection via recursive graph traversal across all edge types, insert RELATE edge (FR-035b, FR-036)
- [ ] T036 [US3] Implement add_dependency tool handler in src/tools/write.rs: parse from_task_id, to_task_id, dependency_type; call query; return edge details with created_at (FR-035b)
- [ ] T037 [US3] Extend get_task_graph in src/tools/read.rs to include all 8 dependency types in graph output with type annotations (FR-035)
- [ ] T038 [US3] Extend dehydration to serialize all 8 edge types in .tmem/graph.surql in src/services/dehydration.rs (FR-035)
- [ ] T039 [US3] Extend hydration to parse all 8 edge types from .tmem/graph.surql RELATE statements in src/services/hydration.rs (FR-035)
- [ ] T040 [US3] Integration test in tests/integration/enhanced_features_test.rs: parent task with 3 children (child_of), mark duplicate (duplicate_of → excluded from ready-work), add blocked_by (blocked in ready-work), mark all children done → parent surfaced in ready-work as completable (US3 scenario 5) (FR-037)

**Checkpoint**: All 8 dependency types functional with cycle detection and ready-work interaction. US3 independently testable.

---

## Phase 6: User Story 4 — Agent-Driven Compaction (Priority: P4)

**Goal**: `get_compaction_candidates` and `apply_compaction` two-phase flow, rule-based truncation fallback, graph preservation after compaction.

**Independent Test**: 50 done tasks >7 days old, get candidates, apply summaries, verify compaction_level and graph edges.

### Red Phase (Tests First — Expect Failure)

- [ ] T041 [P] [US4] Write contract tests for get_compaction_candidates in tests/contract/read_test.rs and apply_compaction in tests/contract/write_test.rs: workspace-not-set (1003), valid candidates returned, empty list when none eligible, compaction of nonexistent task (3008), pinned task excluded (FR-038, FR-039)

### Green Phase (Implementation)

- [ ] T042 [US4] Implement compaction candidate query in src/db/queries.rs: WHERE status = 'done' AND updated_at \< (now - threshold_days) AND pinned = false, ORDER BY updated_at ASC, LIMIT $limit (FR-038)
- [ ] T043 [US4] Implement get_compaction_candidates tool handler in src/tools/read.rs: read threshold_days and max_candidates from WorkspaceConfig, call query, return candidates with task_id, title, description, compaction_level, age_days (FR-038)
- [ ] T044 [US4] Implement apply_compaction tool handler in src/tools/write.rs: for each {task_id, summary}, replace description with summary, increment compaction_level, set compacted_at to now(); return per-item results with new_compaction_level (FR-039, FR-040, FR-041)
- [ ] T045 [US4] Implement rule-based truncation service in src/services/compaction.rs: truncate_at_word_boundary(text, max_len) that truncates to configurable length (default 500) at word boundary, preserves metadata prefix "\[Compacted\]" (FR-042)
- [ ] T046 [US4] Unit tests for truncation service in src/services/compaction.rs: typical 2000-char description → \<500 chars (>70% reduction, SC-014), word boundary preservation, short text unchanged, empty input
- [ ] T047 [US4] Integration test in tests/integration/enhanced_features_test.rs: create 50 done tasks with old timestamps, call get_compaction_candidates, apply_compaction with summaries, verify compaction_level=1, verify graph edges preserved (SC-020)
- [ ] T048 [US4] Verify pinned done task excluded from candidates; verify second apply_compaction increments to compaction_level=2 in integration test

**Checkpoint**: Agent-driven compaction fully functional with rule-based fallback. US4 independently testable.

---

## Phase 7: User Story 5 — Task Claiming and Assignment (Priority: P5)

**Goal**: `claim_task` and `release_task` with conflict rejection, context note audit trail, ready-work assignee filter.

**Independent Test**: Two clients, Client A claims, Client B rejected, third-party release, audit trail verified.

### Red Phase (Tests First — Expect Failure)

- [ ] T049 [P] [US5] Write contract tests for claim_task and release_task in tests/contract/write_test.rs: workspace-not-set (1003), valid claim sets assignee, already-claimed returns error 3005 with current claimant, release unclaimed returns error 3012, release records previous claimant in context note (FR-044, FR-045, FR-046)

### Green Phase (Implementation)

- [ ] T050 [US5] Implement claim/release queries in src/db/queries.rs: claim_task with atomic assignee IS NULL check (return current claimant on conflict), release_task clears assignee and returns previous claimant (FR-044, FR-045)
- [ ] T051 [US5] Implement claim_task tool handler in src/tools/write.rs: parse task_id + claimant, call claim query, create context note "Claimed by {claimant}", return task_id + claimant + context_id + claimed_at (FR-044, FR-046)
- [ ] T052 [US5] Implement release_task tool handler in src/tools/write.rs: parse task_id, call release query, create context note "Released by {releaser}, previously claimed by {previous}", return task_id + previous_claimant + context_id (FR-044, FR-046)
- [ ] T053 [US5] Integration test in tests/integration/enhanced_features_test.rs: Client A claims task, Client B claim rejected (3005), Client B releases Client A's claim, verify context notes record both events with identities, verify get_ready_work(assignee: "agent-1") returns only agent-1's tasks

**Checkpoint**: Task claiming functional with audit trail and ready-work integration. US5 independently testable.

---

## Phase 8: User Story 6 — Issue Types and Task Classification (Priority: P6)

**Goal**: `issue_type` field with defaults, update support, type filtering on ready-work, custom types from config.

**Independent Test**: Create tasks of different types, filter by type, verify custom type from config.

### Red Phase (Tests First — Expect Failure)

- [ ] T054 [P] [US6] Write contract tests for update_task with issue_type param in tests/contract/write_test.rs: valid type change creates context note, invalid type returns error 3010 when allowed_types configured (FR-047, FR-048)

### Green Phase (Implementation)

- [ ] T055 [US6] Extend update_task handler in src/tools/write.rs to accept issue_type param: validate against WorkspaceConfig.allowed_types if set (error 3010), update field, create context note recording type change (FR-047, FR-048)
- [ ] T056 [US6] Extend hydration to parse issue_type from YAML frontmatter (default "task" when missing) in src/services/hydration.rs (FR-047)
- [ ] T057 [US6] Extend dehydration to write issue_type to YAML frontmatter in src/services/dehydration.rs (FR-047)
- [ ] T058 [US6] Integration test in tests/integration/enhanced_features_test.rs: create tasks as "task", "bug", "spike"; filter get_ready_work(issue_type: "bug") returns only bugs; custom type from config accepted; type change creates context note

**Checkpoint**: Issue types functional with filtering and config validation. US6 independently testable.

---

## Phase 9: User Story 7 — Defer/Snooze and Pinned Tasks (Priority: P7)

**Goal**: `defer_task`, `undefer_task`, `pin_task`, `unpin_task` tools with ready-work interaction.

**Independent Test**: Defer to tomorrow (excluded from ready-work), pin low-priority (appears first).

### Red Phase (Tests First — Expect Failure)

- [ ] T059 [P] [US7] Write contract tests for defer_task, undefer_task, pin_task, unpin_task in tests/contract/write_test.rs: workspace-not-set (1003), valid defer sets field, valid pin sets flag, each creates context note (FR-050, FR-051, FR-052, FR-053)

### Green Phase (Implementation)

- [ ] T060 [US7] Implement defer_task tool handler in src/tools/write.rs: parse task_id + until (ISO 8601), set defer_until, create context note "Deferred until {date}" (FR-050, FR-051)
- [ ] T061 [US7] Implement undefer_task tool handler in src/tools/write.rs: parse task_id, clear defer_until, create context note with previous defer date (FR-051)
- [ ] T062 [US7] Implement pin_task and unpin_task tool handlers in src/tools/write.rs: set/clear pinned flag, create context notes (FR-052, FR-053)
- [ ] T063 [US7] Extend hydration to parse defer_until (ISO 8601 datetime) and pinned (boolean) from YAML frontmatter in src/services/hydration.rs (FR-050, FR-052)
- [ ] T064 [US7] Extend dehydration to write defer_until and pinned to YAML frontmatter in src/services/dehydration.rs (FR-050, FR-052)
- [ ] T065 [US7] Integration test in tests/integration/enhanced_features_test.rs: defer task to tomorrow → excluded from ready-work; undefer → reappears; pin low-priority p4 task → appears above p0 unpinned; unpin → returns to p4 position; pinned tasks sorted by priority among themselves (FR-054)
- [ ] T066 [US7] Edge case test: defer_until in the past at hydration time → task immediately eligible for ready-work queue

**Checkpoint**: Defer and pin fully functional with ready-work interaction. US7 independently testable.

---

## Phase 10: User Story 8 — MCP Output Controls and Workspace Statistics (Priority: P8)

**Goal**: `brief` and `fields` params on all read tools, `get_workspace_statistics` with grouped counts.

**Independent Test**: `brief: true` returns only essential fields; statistics returns correct grouped counts.

### Red Phase (Tests First — Expect Failure)

- [ ] T067 [P] [US8] Write contract tests for get_workspace_statistics in tests/contract/read_test.rs and brief/fields params on get_ready_work: workspace-not-set (1003), statistics returns by_status/by_priority/by_type/by_label, brief mode strips descriptions (FR-055, FR-056, FR-057)

### Green Phase (Implementation)

- [ ] T068 [US8] Implement filter_fields(value, brief, fields) utility in src/services/output.rs: when brief=true keep only \[id, title, status, priority, assignee\]; when fields provided keep only listed fields (FR-055, FR-056)
- [ ] T069 [US8] Apply output filter to get_ready_work, get_task_graph, and check_status response paths in src/tools/read.rs (FR-055, FR-056)
- [ ] T070 [US8] Implement workspace statistics query in src/db/queries.rs: GROUP BY status, GROUP BY priority, GROUP BY issue_type; label counts via label table; compacted_count, eligible_count, avg_compaction_level; deferred_count, pinned_count, claimed_count (FR-057)
- [ ] T071 [US8] Implement get_workspace_statistics tool handler in src/tools/read.rs: call statistics query, return structured response (FR-057)
- [ ] T072 [US8] Integration test in tests/integration/enhanced_features_test.rs: workspace with 20 tasks (mixed status, priority, type, labels, some deferred/pinned/claimed), call statistics and verify all group counts correct; call get_ready_work(brief: true) and verify only essential fields returned (SC-015)

**Checkpoint**: Output controls and statistics functional. US8 independently testable.

---

## Phase 11: User Story 9 — Batch Operations and Comments (Priority: P9)

**Goal**: `batch_update_tasks` with per-item results, `add_comment` with chronological retrieval, `.tmem/comments.md` serialization.

**Independent Test**: Batch 10 tasks in one call, verify all updated; add comments, verify chronological order.

### Red Phase (Tests First — Expect Failure)

- [ ] T073 [P] [US9] Write contract tests for batch_update_tasks and add_comment in tests/contract/write_test.rs: workspace-not-set (1003), valid batch returns per-item results, batch with one invalid ID returns partial failure (3007), valid comment returns comment_id (FR-058, FR-059, FR-062)

### Green Phase (Implementation)

- [ ] T074 [US9] Implement batch_update_tasks tool handler in src/tools/write.rs: validate batch.max_size from config (FR-060), iterate updates calling existing update_task logic per item, collect per-item success/failure results, return succeeded + failed counts (FR-058, FR-059)
- [ ] T075 [US9] Implement comment queries in src/db/queries.rs: insert_comment(task_id, content, author), get_comments_for_task(task_id) ordered by created_at ASC (FR-061, FR-062, FR-063)
- [ ] T076 [US9] Implement add_comment tool handler in src/tools/write.rs: parse task_id + content + author, validate task exists, call insert_comment, return comment_id + task_id + author + created_at (FR-062)
- [ ] T077 [US9] Implement comments.md hydration in src/services/hydration.rs: parse ## task:\* section headers, ### timestamp — author comment headers, body content until next header; populate comment table (FR-063b)
- [ ] T078 [US9] Implement comments.md dehydration in src/services/dehydration.rs: query comments per task grouped chronologically, write .tmem/comments.md with ## task:\* and ### timestamp — author format (FR-063b)
- [ ] T079 [US9] Integration test in tests/integration/enhanced_features_test.rs: batch_update_tasks on 10 tasks (one invalid → partial failure), verify per-item results; add 3 comments to one task, verify chronological order; flush → rehydrate → verify comments preserved (SC-019)
- [ ] T080 [US9] Edge case test: batch with duplicate task IDs → last update wins, each generates its own context note

**Checkpoint**: Batch operations and comments functional including `.tmem/comments.md` serialization. US9 independently testable.

---

## Phase 12: User Story 10 — Project Configuration (Priority: P10)

**Goal**: Read `.tmem/config.toml` on hydration, validate values, apply defaults on missing/invalid, wire into dependent tools.

**Independent Test**: Create config with custom values, verify daemon reads on hydration and enforces them.

### Red Phase (Tests First — Expect Failure)

- [ ] T081 [P] [US10] Write contract tests for config loading in tests/contract/lifecycle_test.rs: no config.toml → built-in defaults, valid config populates WorkspaceConfig, TOML parse error → defaults with warning (6001), invalid value (compaction.threshold_days=0) → error 6002 (FR-064, FR-065, FR-066)

### Green Phase (Implementation)

- [ ] T082 [US10] Implement parse_config() in src/services/config.rs: read .tmem/config.toml via tokio::fs::read_to_string, deserialize with toml::from_str::\<WorkspaceConfig\>, on missing file return Ok(default), on parse error emit tracing::warn and return Ok(default) (FR-064, FR-066)
- [ ] T083 [US10] Implement validate_config() in src/services/config.rs: check threshold_days >= 1, max_candidates >= 1, truncation_length >= 50, batch.max_size in 1..=1000, default_priority parsable; return Err(ConfigError::InvalidValue) on violation (FR-065)
- [ ] T084 [US10] Integrate config loading into hydration flow in src/services/hydration.rs: after workspace bind, call parse_config() + validate_config(), store result in AppState via state.rs (FR-064, FR-066, SC-016)
- [ ] T085 [US10] Wire WorkspaceConfig values into all dependent tool handlers: add_label checks allowed_labels (FR-034), update_task checks allowed_types (FR-048), get_compaction_candidates uses threshold_days + max_candidates (FR-065), apply_compaction truncation uses truncation_length (FR-042), batch_update_tasks uses max_size (FR-060)
- [ ] T086 [US10] Integration test in tests/integration/enhanced_features_test.rs: config.toml with threshold_days=14, allowed_labels=\["a","b"\], batch.max_size=5; verify compaction uses 14-day threshold, add_label("c") rejected (3006), batch of 6 rejected; verify \<50ms config overhead (SC-016)
- [ ] T087 [US10] Integration test: rehydrate workspace after config.toml change, verify updated values take effect; missing config.toml → defaults applied without error

**Checkpoint**: Configuration fully functional including validation and fallback. US10 independently testable.

---

## Phase 13: Polish & Cross-Cutting Concerns

**Purpose**: End-to-end validation, performance benchmarks, round-trip guarantees, cleanup

- [ ] T088 [P] End-to-end integration test in tests/integration/enhanced_features_test.rs: full workflow — set_workspace with config.toml, create tasks with priorities/labels/types, claim, defer, pin, add dependencies, add comments, batch update, get_ready_work with filters, get_compaction_candidates, apply_compaction, get_workspace_statistics, flush_state, rehydrate, verify all state preserved
- [ ] T089 [P] Performance benchmark tests in tests/integration/performance_test.rs: SC-011 get_ready_work \<50ms (1000 tasks), SC-012 batch 100 \<500ms, SC-013 compaction candidates \<100ms (5000 tasks), SC-015 statistics \<100ms (5000 tasks), SC-018 each filter dimension \<20ms overhead
- [ ] T090 [P] Round-trip serialization test in tests/unit/proptest_serialization.rs: hydrate tasks.md + comments.md + graph.surql + config.toml → modify all new fields → dehydrate → rehydrate → assert 100% data preservation including labels, comments, all edge types, workflow_state/workflow_id (SC-019, FR-068)
- [ ] T091 [P] Reserved workflow field test: create task with workflow_state and workflow_id values, verify all tools ignore them, verify hydrate/dehydrate preserves them, verify get_ready_work does not filter on them (FR-067, FR-068)
- [ ] T092 Run quickstart.md validation: exercise all curl examples from specs/002-enhanced-task-management/quickstart.md against running daemon, verify expected responses
- [ ] T093 Code cleanup: verify all new tool handlers have tracing::instrument spans, error paths log at warn/error, cargo clippy clean with pedantic, cargo fmt --check passes
- [ ] T094 [P] SC-017 error format validation: verify all 15 new tools and 11 new error codes produce `ErrorResponse` JSON with `code`, `name`, `message`, and `details` fields consistent with v0 error taxonomy (SC-017)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — **BLOCKS all user stories**
- **User Stories (Phases 3–12)**: All depend on Foundational phase completion
  - Can proceed in parallel (if staffed) or sequentially in priority order (P1 → P10)
  - Each story is independently testable after completion
- **Polish (Phase 13)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependencies on other stories. Provides `get_ready_work` used by later stories for integration testing.
- **US2 (P2)**: Can start after Foundational — label filter wired into US1's ready-work query but independently testable
- **US3 (P3)**: Can start after Foundational — `duplicate_of` and `blocked_by` exclusions in US1's ready-work query but independently testable
- **US4 (P4)**: Can start after Foundational — compaction threshold uses config (US10) but defaults work without it
- **US5 (P5)**: Can start after Foundational — assignee filter wired into US1's ready-work query but independently testable
- **US6 (P6)**: Can start after Foundational — issue_type filter uses config (US10) but defaults work without it
- **US7 (P7)**: Can start after Foundational — defer/pin interact with US1's ready-work query but independently testable
- **US8 (P8)**: Can start after Foundational — output filter applies to US1's get_ready_work but independently testable
- **US9 (P9)**: Can start after Foundational — batch uses existing update_task; comments are independent
- **US10 (P10)**: Can start after Foundational — config wires into US2 (labels), US4 (compaction), US6 (types), US9 (batch). Best done last so all consumers exist.

### Within Each User Story

1. Red phase: Write tests first — **ensure they FAIL** before implementation
2. Green phase: Models/queries → service logic → tool handlers → serialization → integration tests
3. Story complete before moving to next priority (or parallelize across developers)

### Parallel Opportunities

- All Foundational tasks marked \[P\] can run in parallel (8 of 14 are parallel)
- Once Foundational completes, all user stories can start in parallel (if team capacity allows)
- Within each story, tasks marked \[P\] within the same phase can run in parallel
- All Polish tasks marked \[P\] can run in parallel

---

## Parallel Example: Foundational Phase

```text
# Launch all model tasks together (different files):
T004: Extend Task struct in src/models/task.rs
T005: Create Label in src/models/label.rs
T006: Create Comment in src/models/comment.rs
T007: Create WorkspaceConfig in src/models/config.rs
T008: Extend DependencyType in src/models/graph.rs
T009: compute_priority_order() in src/models/task.rs (same file as T004 but different function)

# Launch error tasks together (different files):
T011: Error codes in src/errors/codes.rs
T012: Error variants in src/errors/mod.rs

# Launch test tasks together (different files):
T014: Property tests in tests/unit/proptest_models.rs
T015: Serialization tests in tests/unit/proptest_serialization.rs
```

## Parallel Example: User Story 1

```text
# After T019 (core query) and T020 (handler):
# Launch all 4 filter dimensions in parallel (same file but separate functions):
T021: Label filter
T022: Priority threshold filter
T023: Issue type filter
T024: Assignee filter
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (3 tasks)
2. Complete Phase 2: Foundational (14 tasks) — **CRITICAL, blocks all stories**
3. Complete Phase 3: User Story 1 — Ready-Work Queue (8 tasks)
4. **STOP and VALIDATE**: Test US1 independently with 20-task scenario
5. Deploy if ready — this single story transforms t-mem from passive storage to active work coordinator

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. US1 → Test → Deploy/Demo (**MVP — highest value**)
3. US2 → Test → Deploy (priorities + labels enable triage)
4. US3 → Test → Deploy (rich dependency graph)
5. US4–US10 → Each independently testable and deployable
6. Polish → Performance validation and end-to-end guarantee

### Parallel Team Strategy

With multiple developers after Foundational is complete:

- Developer A: US1 (ready-work queue) → US4 (compaction)
- Developer B: US2 (priorities/labels) → US5 (claiming)
- Developer C: US3 (dependencies) → US6 (types) → US7 (defer/pin)
- Developer D: US8 (statistics) → US9 (batch/comments) → US10 (config)
- All: Polish phase after stories complete

---

## Notes

- \[P\] tasks = different files, no dependency conflicts — safe to run in parallel
- \[US\*\] label maps each task to its user story for traceability
- TDD enforced: Red phase (tests fail) before Green phase (implementation)
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate the story independently
- FR-to-task coverage: all 49 functional requirements (FR-026 through FR-071) are mapped to specific tasks
- SC-to-task coverage: all 10 success criteria (SC-011 through SC-020) are validated by specific test tasks
