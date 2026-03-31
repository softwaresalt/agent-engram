# Phase 3 Session Memory: Priority-Based Ready-Work Queue

**Date**: 2025-07-17
**Spec**: 002-enhanced-task-management
**Phase**: 3 (US1 — Priority-Based Ready-Work Queue)
**Commit**: `2431e84`
**Branch**: 002-enhanced-task-management

## Tasks Completed

- T018: Contract tests for `get_ready_work` (4 tests: workspace-not-set, empty workspace, returns tasks, limit caps)
- T019: Ready-work SurrealQL query with multi-step filtering (defer, blocked, duplicate, labels, priority, issue_type, assignee)
- T020: `get_ready_work` tool handler with brief mode and field filtering
- T021: Label AND-filter via `task_has_all_labels()` COUNT query
- T022: Priority threshold filter via `compute_priority_order()`
- T023: Issue type exact match filter
- T024: Assignee exact match filter
- T025: Integration test — 20 tasks with blocking, defer, pin scenarios

## Files Modified

| File | Changes |
|------|---------|
| `src/db/queries.rs` | Added `ReadyWorkParams`, `ReadyWorkResult`, `CountRow` structs; `get_ready_work()`, `find_blocked_task_ids()`, `find_duplicate_task_ids()`, `task_has_all_labels()` methods; fixed SurrealQL backtick escaping and datetime storage |
| `src/tools/read.rs` | Added `GetReadyWorkParams`, `get_ready_work()` handler, `serialize_task()` helper |
| `src/tools/mod.rs` | Moved `get_ready_work` from stub to actual handler dispatch |
| `tests/contract/read_test.rs` | Added 4 contract tests, extracted `test_snapshot()` helper |
| `tests/integration/enhanced_features_test.rs` | Added T025 integration test with `make_task()` helper |
| `specs/002-enhanced-task-management/tasks.md` | Marked T018-T025 as complete |

## Decisions and Rationale

1. **Multi-step Rust-side filtering over SurrealQL subqueries**: Chose to fetch all non-done/non-blocked tasks from DB, then filter in Rust for defer, blocked dependencies, duplicates, and optional params. Simpler to debug and maintain than complex SurrealQL subqueries. Trade-off: slightly more data transfer for small-to-medium task sets.

2. **SurrealQL `type` keyword backtick escaping**: Discovered that `type` is a reserved keyword in SurrealQL v2. Without backtick escaping in WHERE clauses, the query runs without error but returns 0 rows. Applied `` `type` `` escaping to `find_blocked_task_ids()` and `find_duplicate_task_ids()`. SELECT clause doesn't need it.

3. **Datetime storage with conditional casting**: `defer_until` and `compacted_at` were stored as RFC3339 strings. Deserialization to `DateTime<Utc>` via `#[serde(default)]` silently returned `None`. Fixed with `IF $field != NONE THEN <datetime>$field END` in UPSERT, matching the pattern used for `created_at`/`updated_at`.

## Issues Discovered

- **SurrealQL reserved keyword `type`**: Any WHERE clause filtering on the `type` field of `depends_on` table must use backtick escaping. This is a cross-cutting concern for any future code that queries dependency types.
- **Optional datetime fields**: Any `Option<DateTime<Utc>>` field stored via bind must use conditional `<datetime>` casting in UPSERT to ensure proper round-trip serialization. Fields affected: `defer_until`, `compacted_at`.

## Failed Approaches

1. Initially thought 15 (instead of 12) eligible tasks was due to stale DB data — switched to UUID-based workspace IDs. The actual issue was SurrealQL reserved keyword escaping.
2. First backtick fix only addressed `find_blocked_task_ids()` — still saw 15 results because `defer_until` was also broken (stored as string, deserialized as None).

## Test Results

- 88 non-benchmark tests pass (47 lib + 7 error codes + 5 lifecycle + 9 read + 10 write + 5 concurrency + 2 connection + 10 embedding + 1 enhanced features + 10 hydration + 1 relevance + 5 proptest + 3 serialization)
- Clippy clean with pedantic lints
- Pre-existing: t098 benchmark fails in debug builds (6.1s vs 5s target)

## Next Steps

- Phase 4: US2 — Task Priorities and Labels (T026-T033)
- Phase 5: US3 — Enhanced Dependencies (T034-T041)
- Watch for: more SurrealQL reserved keyword issues in future queries
