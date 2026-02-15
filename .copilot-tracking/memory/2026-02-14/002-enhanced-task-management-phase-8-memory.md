# Phase 8 Memory: US6 — Issue Types and Task Classification (T054-T058)

**Date**: 2026-02-14
**Spec**: 002-enhanced-task-management
**Phase**: 8
**Commit**: `b9e08ec`
**Branch**: 002-enhanced-task-management

## Tasks Completed

- T054: 4 contract tests for issue_type on update_task and create_task
- T055: Extended update_task and create_task handlers with issue_type param and allowed_types validation
- T056: Verified hydration already parses issue_type from YAML frontmatter (pre-existing from Phase 4)
- T057: Verified dehydration already writes issue_type to YAML frontmatter (pre-existing from Phase 4)
- T058: Integration test — create 3 types, filter, custom type validation, type change context note, flush/rehydrate round-trip

## Files Modified

- `src/tools/write.rs` — added `issue_type` field to `UpdateTaskParams` and `CreateTaskParams`, validation against `WorkspaceConfig.allowed_types`
- `src/db/queries.rs` — added `issue_type: Option<&str>` parameter to `create_task()`
- `tests/contract/write_test.rs` — 4 new contract tests, added `INVALID_ISSUE_TYPE` to imports
- `tests/integration/enhanced_features_test.rs` — T058 integration test
- `specs/002-enhanced-task-management/tasks.md` — marked T054-T058 [x]

## Decisions

1. **Extended queries.create_task()** — Added `issue_type: Option<&str>` parameter rather than creating task then updating, for clean single-write semantics.
2. **Validation pattern matches allowed_labels** — Used same pattern as `add_label` validation: check if `allowed_types` is non-empty, then verify the requested type is in the list.
3. **create_task response includes issue_type** — Added `"issue_type"` field to create_task JSON response for client visibility.
4. **get_ready_work uses "id" not "task_id"** — The `serialize_task` function uses `"id"` as the field name, not `"task_id"`.

## Test Results

- Full suite: 157 passed, 1 failed (pre-existing t098 benchmark)
- New tests: 4 contract + 1 integration = 5 new tests
- Clippy: clean
- Fmt: clean

## Next Steps

- Phase 9 (T059-T066): US7 — Defer/Snooze and Pinned Tasks
