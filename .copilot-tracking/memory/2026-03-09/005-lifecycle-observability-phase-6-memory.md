# Phase 6 Memory: 005-lifecycle-observability - US3 Event Ledger/Rollback

**Session**: 2026-03-09
**Branch**: 005-lifecycle-observability
**Phase**: 6 of 9 (US3 Event Ledger/Rollback)

## Tasks Completed

- T045-T052: Contract tests for event model shape and error codes (3020/3021/3022)
- T053-T054: Integration tests for event count/prune and rollback denied guard
- T055: event_ledger.rs — record_event(), prepare_rollback(), apply_rollback()
- T056: DB queries (insert_event, list_events, count_events, delete_oldest_events, get_events_after, delete_task, restore_task_snapshot, delete_relation_by_id, restore_relation_snapshot, delete_context_by_id, restore_collection_snapshot)
- T057: Event recording in create_task, update_task, add_dependency (fire-and-forget, warnings on failure)
- T058: get_event_history tool (kind/entity_id filters, limit=50 default)
- T059-T060: rollback_to_event tool (prepare + apply rollback, records RollbackApplied event)
- T061: get_event_history + rollback_to_event registered in dispatch

## Key Decisions

1. Event recording is fire-and-forget: if insert_event fails, tracing::warn! is emitted and the parent operation succeeds
2. delete_oldest_events uses SELECT id, created_at (both fields required for SurrealQL ORDER BY in subquery)
3. WorkspaceConfig gained event_ledger_max (default 500) and allow_agent_rollback (default false)
4. apply_rollback reverses events in reverse-chronological order (newest first)
5. RollbackApplied events are SKIPPED during rollback traversal (no double-reversal)
6. source_client is hardcoded as "mcp-tool" — real client ID requires MCP connection context (future enhancement)

## Gates Passed

- cargo check: exit 0
- cargo fmt --all: clean
- cargo clippy --all-targets -- -D warnings -D clippy::pedantic: exit 0 (1 allow on record_event 8-arg function)
- cargo test --test contract_event: 9/9 passed
- cargo test --test integration_rollback: 2/2 passed
- All prior suites: 15+3+6+3+4 passed

## Files Modified

| File | Change |
|------|--------|
| src/services/event_ledger.rs | Full implementation (record_event, prepare_rollback, apply_rollback) |
| src/db/queries.rs | 10 new event/rollback query methods |
| src/models/config.rs | event_ledger_max + allow_agent_rollback fields |
| src/tools/write.rs | Event recording in create_task/update_task/add_dependency; rollback_to_event |
| src/tools/read.rs | get_event_history tool |
| src/tools/mod.rs | get_event_history + rollback_to_event registered |
| tests/contract/event_test.rs | 9 contract tests |
| tests/integration/rollback_test.rs | 2 integration tests |
| specs/005-lifecycle-observability/tasks.md | T045-T061 marked [X] |

## Next Steps (Phase 7)

Phase 7: US4 - Sandboxed Graph Query Interface (T062-T075, 14 tasks)
- TDD first: T062-T072 contract tests for SELECT/graph/write-rejection/row-limit
- Then implement: T073 query sanitizer (word-boundary blocklist), T074 query_graph tool, T075 dispatch registration
