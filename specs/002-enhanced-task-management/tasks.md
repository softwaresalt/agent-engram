# Tasks: Enhanced Task Management

**Input**: Design documents from `/specs/002-enhanced-task-management/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), data-model.md, contracts/

**Tests**: Contract tests are included per user story as required by the project constitution (Test-First TDD). Property tests for new models are in the Foundational phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Existing project structure from v0 is extended; no new top-level directories

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add dependencies and create file stubs for the enhanced task management feature

- [ ] T001 Add `toml` crate dependency to Cargo.toml for workspace configuration parsing
- [ ] T002 [P] Create empty module files for new source modules: src/models/label.rs, src/models/comment.rs, src/models/config.rs, src/services/compaction.rs, src/services/config.rs, src/services/output.rs
- [ ] T003 [P] Update src/services/mod.rs to declare new submodules (compaction, config, output)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend core models, DB schema, error taxonomy, and serialization to support all new task fields and entities

**CRITICAL**: No user story work can begin until this phase is complete

### Models (parallel — different files)

- [ ] T004 [P] Extend Task struct with new fields (priority: String default "p2", issue_type: String default "task", assignee: Option\<String\>, defer_until: Option\<DateTime\<Utc\>\>, pinned: bool default false, compaction_level: u32 default 0, compacted_at: Option\<DateTime\<Utc\>\>, workflow_state: Option\<String\>, workflow_id: Option\<String\>) in src/models/task.rs
- [ ] T005 [P] Create Label struct (id: String, task_id: String, name: String, created_at: DateTime\<Utc\>) in src/models/label.rs
- [ ] T006 [P] Create Comment struct (id: String, task_id: String, content: String, author: String, created_at: DateTime\<Utc\>) in src/models/comment.rs
- [ ] T007 [P] Create WorkspaceConfig struct (default_priority: String, allowed_labels: Option\<Vec\<String\>\>, allowed_types: Vec\<String\>, compaction_threshold_days: u64, compaction_max_candidates: usize, compaction_truncation_length: usize, batch_max_size: usize) with Default impl in src/models/config.rs
- [ ] T008 [P] Extend DependencyType enum to 8 variants (HardBlocker, SoftDependency, ChildOf, BlockedBy, DuplicateOf, RelatedTo, Predecessor, Successor) in src/models/graph.rs

### Re-exports and Schema

- [ ] T009 Update re-exports in src/models/mod.rs to include Label, Comment, WorkspaceConfig, and new DependencyType variants
- [ ] T010 [P] Extend DB schema in src/db/schema.rs: add new DEFINE FIELD statements for task table (priority, issue_type, assignee, defer_until, pinned, compaction_level, compacted_at, workflow_state, workflow_id), add DEFINE TABLE for label and comment, add indexes (task_priority, task_assignee, task_defer_until, label_task_id, label_name, comment_task_id). Note: depends_on ASSERT constraint unchanged here — updated in US3 Phase 5
- [ ] T011 [P] Add new error code constants in src/errors/codes.rs: CLAIM_CONFLICT (3005), LABEL_VALIDATION_FAILED (3006), BATCH_PARTIAL_FAILURE (3007), COMPACTION_ERROR (3008), INVALID_PRIORITY (3009), INVALID_ISSUE_TYPE (3010), DUPLICATE_LABEL (3011), TASK_NOT_CLAIMABLE (3012), CONFIG_PARSE_ERROR (6001), INVALID_CONFIG_VALUE (6002), CONFIG_KEY_UNKNOWN (6003)
- [ ] T012 Add error enum variants for new codes (ClaimConflict, LabelValidationFailed, BatchPartialFailure, CompactionError, InvalidPriority, InvalidIssueType, DuplicateLabel, TaskNotClaimable, ConfigParseError, InvalidConfigValue, ConfigKeyUnknown) with Display and code() impls in src/errors/mod.rs

### Serialization

- [ ] T013 Extend hydration to parse new task frontmatter fields (priority, issue_type, assignee, defer_until, pinned, compaction_level, compacted_at, labels array) from `.tmem/tasks.md` frontmatter, and parse `.tmem/comments.md` into comment table in DB in src/services/hydration.rs
- [ ] T014 Extend dehydration to serialize new task frontmatter fields (including labels array) to .tmem/tasks.md, write new edge types to .tmem/graph.surql, and serialize comments to .tmem/comments.md with per-task sections in src/services/dehydration.rs

### Testing

- [ ] T015 Add property tests for Label, Comment, WorkspaceConfig structs and extended Task/DependencyType serialization round-trips in tests/unit/proptest_models.rs

**Checkpoint**: Foundation ready — user story implementation can now begin in priority order

---

## Phase 3: User Story 1 — Priority-Based Ready Work Queue (Priority: P1) MVP

**Goal**: Implement `get_ready_work` tool that returns the next actionable tasks sorted by pinned status, priority, then creation date — excluding blocked, deferred, and completed tasks.

**Independent Test**: Create 20 tasks across priorities, block 5, defer 3 to a future date, call `get_ready_work` and verify only eligible tasks are returned in correct sort order.

### Tests for User Story 1 (Red phase — must fail before implementation)

- [ ] T019 [US1] Add contract tests for get_ready_work in tests/contract/read_test.rs: workspace-not-set returns 1003, basic call returns sorted results, limit parameter caps results, filter parameters narrow results

### Implementation for User Story 1 (Green phase — make tests pass)

- [ ] T016 [US1] Implement ready-work query in src/db/queries.rs: SELECT from task WHERE status NOT IN \['done','blocked'\] AND (defer_until IS NULL OR defer_until <= now()) AND id NOT IN (SELECT in FROM depends_on WHERE type = 'hard_blocker' AND out.status != 'done'), ORDER BY pinned DESC, priority ASC, created_at ASC, with LIMIT parameter
- [ ] T017 [US1] Implement get_ready_work tool handler in src/tools/read.rs: accept params (limit: u64 default 10, priority: Option\<String\> threshold, labels: Option\<Vec\<String\>\> AND filter, issue_type: Option\<String\>, assignee: Option\<String\>), call ready-work query, return sorted task list
- [ ] T018 [US1] Register get_ready_work in dispatch match table in src/tools/mod.rs

**Checkpoint**: User Story 1 (MVP) is fully functional — agents can query for next actionable task

---

## Phase 4: User Story 2 — Task Priorities and Labels (Priority: P2)

**Goal**: Enable assigning priority levels and descriptive labels to tasks for triage, filtering, and categorization.

**Independent Test**: Create tasks with different priorities and labels, filter by label and priority threshold, verify correct results.

### Tests for User Story 2 (Red phase — must fail before implementation)

- [ ] T025 [US2] Add contract tests for label operations in tests/contract/write_test.rs: workspace-not-set returns 1003, add_label creates association, remove_label deletes association, duplicate label returns 3011, invalid priority returns 3009, label not in allowed_labels returns 3006

### Implementation for User Story 2 (Green phase — make tests pass)

- [ ] T020 [US2] Implement label DB queries in src/db/queries.rs: add_label (INSERT into label), remove_label (DELETE from label WHERE task_id AND name), get_labels_for_task (SELECT from label WHERE task_id), filter_tasks_by_labels (WHERE id IN SELECT task_id FROM label WHERE name IN $labels GROUP BY task_id HAVING count() = $label_count)
- [ ] T021 [US2] Implement add_label tool handler in src/tools/write.rs: accept params (task_id, label), validate task exists, check for duplicate label (error 3011), optionally validate against allowed_labels config, call add_label query, return confirmation with label and timestamp
- [ ] T022 [US2] Implement remove_label tool handler in src/tools/write.rs: accept params (task_id, label), validate task exists, call remove_label query, return confirmation
- [ ] T023 [US2] Extend update_task tool handler to validate priority values (parse numeric suffix, error 3009 for invalid) and record priority changes as context notes in src/tools/write.rs
- [ ] T024 [US2] Register add_label, remove_label in dispatch match table in src/tools/mod.rs

**Checkpoint**: Tasks can be prioritized and labeled; filtering by label works on read operations

---

## Phase 5: User Story 3 — Enhanced Dependency Graph (Priority: P3)

**Goal**: Support richer relationship types between tasks (parent/child, blocks/blocked-by, duplicates, related, predecessor/successor).

**Independent Test**: Create parent with child subtasks, mark duplicates, add blocks/blocked-by, call `get_task_graph` and verify all relationship types render correctly.

### Tests for User Story 3 (Red phase — must fail before implementation)

- [ ] T030 [US3] Add contract tests for enhanced dependency operations in tests/contract/read_test.rs: cyclic detection across new types, duplicate_of exclusion from ready-work, get_task_graph renders all 8 types, add_dependency creates typed edges

### Implementation for User Story 3 (Green phase — make tests pass)

- [ ] T073 [US3] Implement add_dependency DB query in src/db/queries.rs: RELATE $from->depends_on->$to SET type = $dep_type, with input validation for 8 allowed edge types
- [ ] T074 [US3] Implement add_dependency tool handler in src/tools/write.rs: accept params (from_task_id, to_task_id, dependency_type), validate both tasks exist, validate type is one of 8 allowed values, call cyclic detection, create edge, return confirmation with context note
- [ ] T075 [US3] Register add_dependency in dispatch match table in src/tools/mod.rs
- [ ] T026 [US3] Update depends_on schema ASSERT constraint to accept all 8 edge types in src/db/schema.rs (update DEFINE_RELATIONSHIPS constant)
- [ ] T027 [US3] Extend cyclic dependency detection in src/db/queries.rs to traverse all blocking edge types (hard_blocker, blocked_by, child_of, predecessor, successor) during cycle check
- [ ] T028 [US3] Add duplicate_of exclusion logic to ready-work query in src/db/queries.rs: exclude tasks that are the source of a duplicate_of edge
- [ ] T029 [US3] Update get_task_graph tool handler in src/tools/read.rs to include all relationship types in output with type labels on each edge

**Checkpoint**: Dependency graph supports all 8 relationship types with correct cycle detection and duplicate exclusion

---

## Phase 6: User Story 4 — Agent-Driven Compaction (Priority: P4)

**Goal**: Enable agents to compact old completed tasks into concise summaries to keep workspace memory within token limits.

**Independent Test**: Create 50 completed tasks older than 7 days, call `get_compaction_candidates`, verify eligible list returned. Apply compaction with summaries, verify originals replaced and graph relationships preserved.

### Tests for User Story 4 (Red phase — must fail before implementation)

- [ ] T036 [US4] Add contract tests for compaction tools in tests/contract/write_test.rs: workspace-not-set returns 1003, candidates returns eligible tasks, apply replaces content and increments level, pinned tasks excluded from candidates, compacted tasks retain all graph edges (SC-020), compacted content is at least 70% smaller (SC-014)

### Implementation for User Story 4 (Green phase — make tests pass)

- [ ] T031 [US4] Implement compaction candidate query in src/db/queries.rs: SELECT from task WHERE status = 'done' AND updated_at < (now() - threshold_days) AND pinned = false, ORDER BY updated_at ASC, LIMIT max_candidates
- [ ] T032 [US4] Implement get_compaction_candidates tool handler in src/tools/read.rs: accept optional params (threshold_days, max_candidates), use workspace config defaults, return task list with full content and metadata
- [ ] T033 [US4] Implement apply_compaction tool handler in src/tools/write.rs: accept params (compactions: Vec\<{task_id, summary}\>), validate each task exists and is done, replace description with summary, increment compaction_level, set compacted_at, create context note per compaction
- [ ] T034 [US4] Implement rule-based truncation fallback service in src/services/compaction.rs: truncate description to configurable length (default 500 chars) at word boundary, preserve metadata, return truncated text as summary
- [ ] T035 [US4] Register get_compaction_candidates, apply_compaction in dispatch match table in src/tools/mod.rs

**Checkpoint**: Agents can discover stale tasks and apply external summaries; graph relationships survive compaction

---

## Phase 7: User Story 5 — Task Claiming and Assignment (Priority: P5)

**Goal**: Enable lightweight task claiming so parallel workers do not duplicate effort on the same item.

**Independent Test**: Connect two clients, have both call `get_ready_work`, Client A claims a task, verify Client B's ready-work excludes it when filtering by assignee.

### Tests for User Story 5 (Red phase — must fail before implementation)

- [ ] T041 [US5] Add contract tests for claim/release operations in tests/contract/write_test.rs: workspace-not-set returns 1003, claim sets assignee, double-claim returns 3005, release clears assignee, release records audit trail

### Implementation for User Story 5 (Green phase — make tests pass)

- [ ] T037 [US5] Implement claim and release DB queries in src/db/queries.rs: claim_task (UPDATE task SET assignee = $claimant WHERE id = $task_id AND assignee IS NULL), release_task (UPDATE task SET assignee = NULL WHERE id = $task_id), check_claimant (SELECT assignee FROM task WHERE id = $task_id)
- [ ] T038 [US5] Implement claim_task tool handler in src/tools/write.rs: accept params (task_id, claimant), check if already claimed (error 3005 with current claimant), set assignee, create context note recording claim
- [ ] T039 [US5] Implement release_task tool handler in src/tools/write.rs: accept params (task_id), record previous claimant and releaser in context note, clear assignee
- [ ] T040 [US5] Register claim_task, release_task in dispatch match table in src/tools/mod.rs

**Checkpoint**: Tasks can be claimed and released with full audit trail; parallel agents coordinate via claims

---

## Phase 8: User Story 6 — Issue Types and Task Classification (Priority: P6)

**Goal**: Classify tasks by type (task, bug, spike, decision, milestone) for differentiated filtering and tracking.

**Independent Test**: Create tasks of different types, filter by type, verify correct results.

### Tests for User Story 6 (Red phase — must fail before implementation)

- [ ] T045 [US6] Add contract tests for issue type operations in tests/contract/write_test.rs: invalid type returns 3010, type change creates context note, type filter on get_ready_work returns correct subset

### Implementation for User Story 6 (Green phase — make tests pass)

- [ ] T042 [US6] Implement issue type filtering query in src/db/queries.rs: extend task queries to support WHERE issue_type = $type filter
- [ ] T043 [US6] Extend update_task tool handler to validate issue_type values against default set or workspace config (error 3010 for invalid) and record type changes as context notes in src/tools/write.rs
- [ ] T044 [US6] Add issue_type filter parameter to get_ready_work handler in src/tools/read.rs

**Checkpoint**: Tasks can be classified by type and filtered in ready-work queries

---

## Phase 9: User Story 7 — Defer/Snooze and Pinned Tasks (Priority: P7)

**Goal**: Enable deferring tasks to future dates and pinning critical tasks to the top of ready-work results.

**Independent Test**: Defer a task to tomorrow, verify excluded from today's ready-work. Pin a low-priority task, verify it appears first.

### Tests for User Story 7 (Red phase — must fail before implementation)

- [ ] T051 [US7] Add contract tests for defer/pin tools in tests/contract/write_test.rs: workspace-not-set returns 1003, defer excludes from ready-work, expired defer re-includes, pin promotes to top of results, unpin restores normal position

### Implementation for User Story 7 (Green phase — make tests pass)

- [ ] T046 [US7] Implement defer_task tool handler in src/tools/write.rs: accept params (task_id, until: DateTime), validate future date, set defer_until, create context note
- [ ] T047 [US7] Implement undefer_task tool handler in src/tools/write.rs: accept params (task_id), clear defer_until, create context note
- [ ] T048 [US7] Implement pin_task tool handler in src/tools/write.rs: accept params (task_id), set pinned = true, create context note
- [ ] T049 [US7] Implement unpin_task tool handler in src/tools/write.rs: accept params (task_id), set pinned = false, create context note
- [ ] T050 [US7] Register defer_task, undefer_task, pin_task, unpin_task in dispatch match table in src/tools/mod.rs

**Checkpoint**: Deferred tasks auto-resurface when due; pinned tasks always appear at top of ready-work

---

## Phase 10: User Story 8 — MCP Output Controls and Workspace Statistics (Priority: P8)

**Goal**: Provide abbreviated responses and aggregate workspace statistics to help agents manage token budgets.

**Independent Test**: Call `get_ready_work(brief: true)` and verify minimal fields. Call `get_workspace_statistics()` and verify counts by status, type, priority, and label.

### Tests for User Story 8 (Red phase — must fail before implementation)

- [ ] T056 [US8] Add contract tests for output controls and statistics in tests/contract/read_test.rs: brief mode returns minimal fields, fields param selects specific fields, statistics returns correct aggregate structure

### Implementation for User Story 8 (Green phase — make tests pass)

- [ ] T052 [US8] Implement output field filtering utility in src/services/output.rs: filter_fields(value: Value, brief: bool, fields: Option\<Vec\<String\>\>) that strips a JSON Value to only requested fields; brief mode keeps only id, title, status, priority, assignee
- [ ] T053 [US8] Add brief and fields parameters to all read tool handlers (get_ready_work, get_task_graph, check_status) in src/tools/read.rs: parse optional brief/fields params, apply filter_fields to output before returning
- [ ] T054 [US8] Implement get_workspace_statistics tool handler in src/tools/read.rs: query aggregate counts grouped by status, priority, issue_type, and label; include total_tasks, compacted_count, deferred_count, claimed_count, pinned_count
- [ ] T055 [US8] Register get_workspace_statistics in dispatch match table in src/tools/mod.rs

**Checkpoint**: Agents can request abbreviated responses and get workspace health overview

---

## Phase 11: User Story 9 — Batch Operations and Comments (Priority: P9)

**Goal**: Enable bulk task updates in a single call and attach discussion comments to tasks.

**Independent Test**: Create 10 tasks, batch-update all to "in_progress", verify all updated with individual context notes. Add multiple comments, verify chronological retrieval.

### Tests for User Story 9 (Red phase — must fail before implementation)

- [ ] T061 [US9] Add contract tests for batch and comment operations in tests/contract/write_test.rs: workspace-not-set returns 1003, batch updates all valid tasks, batch with invalid ID returns per-item errors, add_comment stores and retrieves chronologically

### Implementation for User Story 9 (Green phase — make tests pass)

- [ ] T057 [US9] Implement batch_update_tasks tool handler in src/tools/write.rs: accept params (updates: Vec\<{id, status, notes}\>, max batch_max_size from config default 100), iterate updates calling existing update_task logic per item, collect per-item results (success/failure), return aggregate response with error 3007 if any fail
- [ ] T058 [US9] Implement comment DB queries in src/db/queries.rs: add_comment (INSERT into comment), get_comments_for_task (SELECT from comment WHERE task_id ORDER BY created_at ASC)
- [ ] T059 [US9] Implement add_comment tool handler in src/tools/write.rs: accept params (task_id, content, author), validate task exists, call add_comment query, return comment ID and timestamp
- [ ] T060 [US9] Register batch_update_tasks, add_comment in dispatch match table in src/tools/mod.rs

**Checkpoint**: Bulk workflows are efficient; task discussions are preserved separate from context notes

---

## Phase 12: User Story 10 — Project Configuration (Priority: P10)

**Goal**: Enable workspace-level configuration via `.tmem/config.toml` for default priority, allowed types/labels, compaction thresholds, and batch limits.

**Independent Test**: Create `.tmem/config.toml` with custom values, hydrate workspace, verify daemon applies custom defaults.

### Tests for User Story 10 (Red phase — must fail before implementation)

- [ ] T066 [US10] Add contract tests for config loading and validation in tests/contract/lifecycle_test.rs: missing config uses defaults, valid config overrides defaults, malformed config falls back with warning, invalid values return 6002

### Implementation for User Story 10 (Green phase — make tests pass)

- [ ] T062 [US10] Implement config.toml parser with defaults fallback in src/services/config.rs: parse_config(path) reads TOML file, maps to WorkspaceConfig struct, falls back to WorkspaceConfig::default() on missing or malformed file with tracing::warn
- [ ] T063 [US10] Implement config validation in src/services/config.rs: validate allowed_labels list (non-empty strings), allowed_types list (non-empty strings), priority format (matches p\d+ pattern), threshold and size ranges (positive integers), return error 6002 for invalid values
- [ ] T064 [US10] Integrate config loading into workspace hydration in src/services/hydration.rs: after workspace path validation, attempt to read .tmem/config.toml, store parsed WorkspaceConfig in AppState alongside workspace snapshot
- [ ] T065 [US10] Wire WorkspaceConfig into tool handlers in src/tools/mod.rs: pass config to dispatch calls so handlers can read default_priority, allowed_labels, allowed_types, compaction settings, and batch limits instead of hardcoded defaults

**Checkpoint**: Workspace behavior is customizable via config file; sensible defaults apply when config is absent

---

## Phase 13: Polish and Cross-Cutting Concerns

**Purpose**: Integration testing, performance validation, documentation, and cleanup

- [ ] T067 [P] Add integration test for full enhanced workflow (set_workspace → create tasks → set priorities/labels → claim → defer → get_ready_work → compaction → statistics) in tests/integration/enhanced_features_test.rs
- [ ] T068 [P] Update serialization property tests for all new models (Label, Comment, WorkspaceConfig) and extended Task round-trips in tests/unit/proptest_serialization.rs
- [ ] T069 [P] Add performance benchmark tests targeting SC-011 (get_ready_work < 50ms), SC-012 (batch 100 < 500ms), SC-013 (compaction candidates < 100ms), SC-014 (compaction 70% size reduction), SC-015 (statistics < 100ms), SC-016 (config hydration < 50ms overhead), SC-018 (filter < 20ms per dimension), SC-019 (new field round-trip 100%), SC-020 (zero edge loss after compaction) in tests/integration/performance_test.rs
- [ ] T070 [P] Update MCP tool contract documentation in specs/002-enhanced-task-management/contracts/
- [ ] T071 Code cleanup: resolve clippy warnings, remove dead_code allows for new modules, verify fmt compliance
- [ ] T072 Run quickstart.md validation to confirm all new tools work end-to-end

---

## Dependencies and Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Stories (Phase 3–12)**: All depend on Foundational phase completion
  - Stories can proceed in priority order (P1 → P2 → … → P10)
  - Many stories can proceed in parallel if staffed (see below)
- **Polish (Phase 13)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependency on other stories (MVP)
- **US2 (P2)**: Can start after Foundational — US1 benefits from label filtering but US2 is independently testable
- **US3 (P3)**: Can start after Foundational — enhances US1 ready-work exclusion (duplicate_of) but independently testable
- **US4 (P4)**: Can start after Foundational — operates on done tasks only, independent of ready-work
- **US5 (P5)**: Can start after Foundational — adds assignee field used by US1 filtering but independently testable
- **US6 (P6)**: Can start after Foundational — adds issue_type filtering to US1 but independently testable
- **US7 (P7)**: Can start after Foundational — adds defer/pin logic used by US1 but independently testable
- **US8 (P8)**: Should start after US1 — output controls wrap existing read tools; statistics needs populated data
- **US9 (P9)**: Can start after Foundational — batch wraps existing update_task; comments are independent
- **US10 (P10)**: Can start after Foundational — config wires defaults into other story handlers; best done last so all handlers exist

### Within Each User Story

- Contract tests written first (Red phase — tests must fail)
- DB queries before tool handlers (Green phase — make tests pass)
- Tool handlers before dispatch registration
- Core implementation before integration points

### Parallel Opportunities

**Foundational phase** (within Phase 2):

- T004, T005, T006, T007, T008 — all different model files, run in parallel
- T010, T011 — schema and error codes are independent files, parallel with each other

**Cross-story parallelism** (after Phase 2 completes):

- US1, US2, US3 can start in parallel (different query/handler code)
- US4, US5 can start in parallel with each other and with US1–US3
- US6, US7 can start in parallel once US1 handler exists (they add filter params)
- US9 can start in parallel with any story (batch wraps existing logic)

---

## Parallel Example: Foundational Phase

```text
# Parallel batch 1 — Model files (all different files):
T004: Extend Task struct in src/models/task.rs
T005: Create Label model in src/models/label.rs
T006: Create Comment model in src/models/comment.rs
T007: Create WorkspaceConfig model in src/models/config.rs
T008: Extend DependencyType in src/models/graph.rs

# Parallel batch 2 — Schema and errors (different files):
T010: Extend DB schema in src/db/schema.rs
T011: Add error codes in src/errors/codes.rs

# Sequential after models:
T009: Update mod.rs re-exports
T012: Add error variants in src/errors/mod.rs
T013: Extend hydration
T014: Extend dehydration
T015: Property tests
```

## Parallel Example: User Story 1 (MVP)

```text
# Red phase (tests first):
T019: Contract tests in tests/contract/read_test.rs (must fail initially)

# Green phase (sequential within story):
T016: Ready-work query in src/db/queries.rs
T017: get_ready_work handler in src/tools/read.rs
T018: Register in dispatch in src/tools/mod.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1 (get_ready_work)
4. **STOP and VALIDATE**: Test `get_ready_work` independently with varied task data
5. Deploy/demo if ready — agents can now query for next actionable task

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add US1 → Test independently → **MVP complete**
3. Add US2 → Labels and priorities → Enhanced filtering
4. Add US3 → Rich dependency graph → Accurate project structure
5. Add US4 → Compaction → Token budget management
6. Add US5 → Claiming → Multi-agent coordination
7. Add US6 → Issue types → Work classification
8. Add US7 → Defer/Pin → Temporal task management
9. Add US8 → Output controls → Agent efficiency
10. Add US9 → Batch/Comments → Bulk operations
11. Add US10 → Configuration → Workspace customization
12. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers after Foundational completes:

- Developer A: US1 (MVP) → US8 (output controls, wraps US1)
- Developer B: US2 (labels) → US6 (types) → US10 (config)
- Developer C: US3 (deps) → US4 (compaction) → US7 (defer/pin)
- Developer D: US5 (claiming) → US9 (batch/comments)

---

## Notes

- [P] tasks = different files, no dependencies between them
- [Story] label maps task to specific user story for traceability
- **TDD enforced**: Each user story starts with contract tests (Red phase) before implementation (Green phase) per Constitution III
- Each user story is independently completable and testable after Foundational phase
- All new MCP tools follow the existing dispatch pattern in src/tools/mod.rs
- All tool handlers require workspace-bound validation (return 1003 if not set)
- Every task status/metadata change MUST create a context note (FR-015 from v0)
- Priority sorting uses ordinal numeric extraction from string suffix (p0 < p1 < p10)
- Status remains the v0 set (todo, in_progress, done, blocked); defer/claim/pin are orthogonal
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
