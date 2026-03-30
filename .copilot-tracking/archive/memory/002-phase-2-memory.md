# Phase 2 Memory: Foundational (002-enhanced-task-management)

**Date**: 2025-07-17
**Commit**: `d26d46d`
**Branch**: `002-enhanced-task-management`
**Phase**: 2 of 13 — Foundational (Blocking Prerequisites)
**Tasks**: T004–T017 (14 tasks, all complete)

## Files Modified (20 files)

### Source Files

- `src/models/task.rs` — Extended Task with 9 new fields + 2 reserved; added `compute_priority_order()`
- `src/models/graph.rs` — Extended DependencyType from 2→8 variants; added `ALL` constant
- `src/models/mod.rs` — Added `compute_priority_order` re-export
- `src/errors/codes.rs` — Added 3005-3012, 6001-6003; renumbered TASK_TITLE_EMPTY to 3013
- `src/errors/mod.rs` — Added 8 TaskError variants, ConfigError enum (3 variants), Config variant in TMemError
- `src/db/schema.rs` — Extended DEFINE_TASK, added DEFINE_LABEL/DEFINE_COMMENT, SCHEMA_VERSION = "2.0.0"
- `src/db/mod.rs` — ensure_schema executes DEFINE_LABEL and DEFINE_COMMENT
- `src/db/queries.rs` — Extended TaskRow, into_task, upsert_task, create_task; updated format/parse dependency
- `src/server/state.rs` — Added `workspace_config: RwLock<Option<WorkspaceConfig>>` to AppState
- `src/tools/mod.rs` — Registered 15 new tool stubs in dispatch; added workspace_not_set() helper
- `src/tools/write.rs` — update_task preserves new fields from existing task
- `src/services/hydration.rs` — parse_tasks_md includes new field defaults; apply_relation handles 8 types
- `src/services/dehydration.rs` — format_dependency handles 8 types; manual_let_else clippy fix

### Test Files

- `tests/unit/proptest_models.rs` — Rewritten: 5 roundtrip tests (task, dep_type, label, comment, config)
- `tests/unit/proptest_serialization.rs` — Extended arb_task with priority and all new fields
- `tests/contract/error_codes_test.rs` — Added config error codes test, extended task error codes
- `tests/contract/write_test.rs` — Updated TASK_TITLE_EMPTY code reference
- `tests/integration/hydration_test.rs` — Updated Task constructors with new fields
- `tests/integration/benchmark_test.rs` — Updated make_task with new fields

### Spec Files

- `specs/002-enhanced-task-management/tasks.md` — Phase 2 tasks marked [X]

## Decisions Made

1. **Error code renumbering**: TASK_TITLE_EMPTY moved from 3005→3013 to free 3005–3012 for spec-defined codes
2. **Schema version**: Explicit `SCHEMA_VERSION = "2.0.0"` constant added (was implicit 1.0.0)
3. **Default priority**: "p2" with priority_order=2; compute_priority_order parses trailing digits
4. **Hydration defaults**: New Task fields get hardcoded defaults at hydration time; Phase 4 (T031/T032) will add frontmatter parsing
5. **Dispatch stubs**: All 15 new tools registered with combined match arm returning workspace_not_set()

## Known Issues

- Pre-existing benchmark test failures (t098 hydration 6122ms > 5000ms, t100 update_task 17ms > 10ms) in debug builds — not Phase 2 regressions
- fastembed TLS feature flag issue on ort-sys still pending

## Test Results

- 82 tests passing: 47 lib + 7 error_codes + 5 lifecycle + 5 read + 10 write + 5 proptest + 3 serialization
- 2 benchmark tests failing (pre-existing timing issues in debug builds)
- Clippy pedantic: clean
- Compilation: clean

## Next Steps

- Phase 3: User Story 1 — Priority-Based Ready-Work Queue (T018–T025, 8 tasks)
  - T018: Contract tests for get_ready_work (RED phase — tests expect failure)
  - T019-T024: Implement ready-work query with 4 filter dimensions (GREEN phase)
  - T025: Integration test with 20-task scenario
- Phase 3 ready-work queries should rely on DB field defaults rather than hydrated frontmatter values
