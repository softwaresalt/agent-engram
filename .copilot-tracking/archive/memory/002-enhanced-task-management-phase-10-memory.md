# Phase 10 Memory — US8: MCP Output Controls and Workspace Statistics

**Date**: 2026-02-14
**Spec**: 002-enhanced-task-management
**Phase**: 10 (T067–T072)
**Commit**: `b8ae715`
**Branch**: `002-enhanced-task-management`

## Tasks Completed

- **T067**: Contract tests for workspace statistics (3 tests)
- **T068**: `filter_fields` / `serialize_task` in `services/output.rs`
- **T069**: Brief/fields params on `TaskGraphParams` and `CheckStatusParams`
- **T070**: Refactored `get_ready_work` to use `output::serialize_task`
- **T071**: Added `get_workspace_statistics` handler to `read.rs` and wired dispatch
- **T072**: Integration test — 20 tasks with mixed attributes, brief mode validation

## Files Modified

| File | Changes |
|------|---------|
| `src/db/queries.rs` | Added `WorkspaceStatistics` struct, `get_workspace_statistics()` with 8 SurrealQL queries, per-field GROUP BY structs |
| `src/services/output.rs` | Implemented `serialize_task`, `filter_fields`, `filter_value`, `BRIEF_FIELDS` |
| `src/tools/read.rs` | Added `get_workspace_statistics` handler, refactored to use `output` module |
| `src/tools/mod.rs` | Moved `get_workspace_statistics` from stub to real dispatch |
| `tests/contract/read_test.rs` | Added 3 contract tests (16 total) |
| `tests/integration/enhanced_features_test.rs` | Added T072 test (10 total) |
| `specs/002-enhanced-task-management/tasks.md` | Marked T067–T072 as complete |

## Key Decisions

- **SurrealDB GROUP BY AS alias bug**: `SELECT field AS alias ... GROUP BY field` breaks grouping in SurrealDB v2 — all rows collapse to same group value. Fixed by removing aliases and using per-field deserialization structs (`StatusGroup`, `PriorityGroup`, `TypeGroup`, `LabelGroupRow`).
- **`build_node` kept at 3 args**: `TaskNode` (id + status + children) is already compact; `brief`/`fields` params added to `TaskGraphParams` for API consistency but marked `#[allow(dead_code)]`.
- **Variable renaming for clippy `similar_names`**: `stats` → `statistics` (vs `state`), `workspace_id` → `ws_id` (vs function name).

## Test Results

- 172+ tests pass across all binaries
- 1 pre-existing failure: t098 benchmark in debug builds (~7s vs 5s target)
- Clippy clean with `-D warnings -D clippy::pedantic`
- `cargo fmt` clean

## Discoveries

- `CreateTaskParams` does NOT accept `priority` — default is `"p2"`
- `DeferTaskParams.until` (not `defer_until`), no `reason` field
- `ClaimTaskParams.claimant` (not `assignee`)
- `UpdateTaskParams.id` (not `task_id`), `status` is required
- Status transition `todo` → `todo` is INVALID
- `tempfile::tempdir()` + `set_workspace` dispatch gives clean DB for contract tests

## Next Steps

- Phase 11: US9 — Batch Operations and Comments (T073–T080)
- Two remaining stubs in `mod.rs`: `batch_update_tasks`, `add_comment`
