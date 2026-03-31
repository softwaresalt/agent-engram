# Phase 5 Memory — US3: Enhanced Dependency Graph

**Spec**: 002-enhanced-task-management
**Phase**: 5 (T034–T040)
**Date**: 2026-02-14
**Commit**: `b3cc70b`
**Branch**: `002-enhanced-task-management`

## Tasks Completed

| Task | Description | Status |
|------|-------------|--------|
| T034 | Contract tests for `add_dependency` (4 tests) | Done |
| T035 | `create_dependency` query — pre-existing, no changes needed | Done |
| T036 | `add_dependency` tool handler in `write.rs` | Done |
| T037 | `get_task_graph` `EdgeNode` type annotations in `read.rs` | Done |
| T038 | Dehydration for all 8 edge types — pre-existing | Done |
| T039 | Hydration for all 8 edge types — pre-existing | Done |
| T040 | Integration test: parent-child, duplicate, blocked_by interactions | Done |

## Files Modified

- `src/tools/write.rs` — `AddDependencyParams`, `add_dependency()` handler
- `src/tools/read.rs` — `EdgeNode` struct, `build_node()` creates typed edges
- `src/tools/mod.rs` — moved `add_dependency` from stub to dispatch
- `tests/contract/write_test.rs` — 4 add_dependency contract tests
- `tests/integration/enhanced_features_test.rs` — t040 integration test
- `specs/002-enhanced-task-management/tasks.md` — T034-T040 marked [X]

## Decisions and Rationale

1. **T035/T038/T039 pre-existing**: The `create_dependency` query, dehydration `format_dependency`, hydration `apply_relation`, and `parse_dependency_type` already supported all 8 `DependencyType` variants from Phase 2/3 work. No changes needed for these tasks.

2. **EdgeNode wrapper**: Introduced `EdgeNode { dependency_type: String, #[serde(flatten)] node: TaskNode }` to annotate graph edges with their type in the JSON output. This avoids changing the recursive `TaskNode` structure while adding type metadata to each child edge.

3. **add_dependency task: prefix stripping**: The handler strips `task:` prefix from `from_task_id`/`to_task_id` before calling `create_dependency`, matching the pattern used in other handlers that accept task IDs.

4. **DependencyType serialization in EdgeNode**: Used `serde_json::to_value(&edge.kind)` to get the snake_case string representation, then stripped quotes. This leverages the existing `#[serde(rename_all = "snake_case")]` on the enum.

## Test Results

- 93 tests pass (47 unit, 7 error codes, 5 lifecycle, 9 read, 19 write, 2 connection, 3 enhanced features, 5 proptest, 5 benchmark non-t098)
- 1 pre-existing failure: t098 benchmark (5.4s debug, 5s threshold)
- Clippy clean, fmt clean

## Known Issues

- Pre-existing: t098 benchmark fails in debug builds (5.4s vs 5s target)
- Pre-existing: `fastembed` TLS feature flag blocks embedding features

## Next Steps

- Phase 6: T041-T048 — US4: Agent-Driven Compaction
  - `get_compaction_candidates` and `apply_compaction` two-phase flow
  - Rule-based truncation fallback in `services/compaction.rs`
  - Pinned task exclusion
  - Graph preservation after compaction
