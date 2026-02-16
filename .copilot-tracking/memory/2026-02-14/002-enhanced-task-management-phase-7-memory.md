# Phase 7 Memory: US5 — Task Claiming and Assignment (T049-T053)

**Date**: 2026-02-14
**Spec**: 002-enhanced-task-management
**Phase**: 7
**Commit**: `ed64aff`
**Branch**: 002-enhanced-task-management

## Tasks Completed

- T049: 6 contract tests (3 claim + 3 release) in `tests/contract/write_test.rs`
- T050: `claim_task()` and `release_task()` DB queries in `src/db/queries.rs`
- T051: `claim_task` MCP handler in `src/tools/write.rs`
- T052: `release_task` MCP handler in `src/tools/write.rs`
- T053: Integration test `t053_claim_release_conflict_audit_trail_and_assignee_filter` in `tests/integration/enhanced_features_test.rs`

## Files Modified

- `src/db/queries.rs` — added `claim_task()` and `release_task()` methods
- `src/tools/write.rs` — added `ClaimTaskParams`, `ReleaseTaskParams`, claim/release handlers with context note creation
- `src/tools/mod.rs` — moved `claim_task` and `release_task` from stub to handler dispatch
- `tests/contract/write_test.rs` — 6 new contract tests
- `tests/integration/enhanced_features_test.rs` — T053 integration test
- `specs/002-enhanced-task-management/tasks.md` — marked T049-T053 [x]

## Decisions and Rationale

1. **claim_task checks `assignee.is_none()`** — If assignee is already set, returns `TaskError::AlreadyClaimed { id, assignee }` with the current claimant name, enabling clients to know who holds the task.

2. **release_task checks `assignee.is_some()`** — If task has no assignee, returns `TaskError::NotClaimable { id, status }` using the task's status as context.

3. **Context note audit trail** — Both claim and release create context notes linked to the task via `link_task_context()`. Claim notes say "Claimed by {claimant}", release notes say "Released, previously claimed by {previous_claimant}".

4. **Contract tests capture task_id from create_task response** — Cannot use `check_status` (requires `work_item_ids`) for simple task ID retrieval in tests. Instead, parse `task_id` from the `create_task` JSON response.

## Failed Approaches

- Initial contract tests used `check_status` to retrieve task IDs — this tool requires `work_item_ids` parameter and does not work for simple task listing. Fixed by extracting `task_id` from `create_task` response JSON.

## Test Results

- Full suite: 152 passed, 1 failed (pre-existing t098 benchmark)
- Clippy: clean (0 warnings)
- Fmt: applied

## Open Issues

- Pre-existing: t098 benchmark fails in debug builds (~5.7s vs 5s target)
- Remaining stubs in `tools/mod.rs`: defer_task, undefer_task, pin_task, unpin_task, get_workspace_statistics, batch_update_tasks, add_comment (7 stubs)

## Next Steps

- Phase 8 (T054-T058): US6 — Issue Types and Task Classification
  - May be partially pre-implemented (issue_type field exists in model, hydration/dehydration handle it since Phase 4)
  - Needs: contract tests for update_task with issue_type, validation against allowed_types in WorkspaceConfig
