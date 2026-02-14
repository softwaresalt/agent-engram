# Phase 4 Memory: 001-core-mcp-daemon

## Task Overview

Phase 4 implements User Story 2 — Task State Management. The 9 remaining tasks (T127, T129–T136) add the `create_task` MCP tool and supporting infrastructure: error code 3005 (`TaskTitleEmpty`), DB query method, tool implementation, dispatch wiring, and comprehensive tests.

## Current State

### Tasks Completed
- [X] T127 — Contract test for work_item_id assignment and retrieval via update_task and get_task_graph (FR-017)
- [X] T129 — Contract test for create_task returning WorkspaceNotSet (1003)
- [X] T130 — Contract test for create_task with empty/oversized title returning TaskTitleEmpty (3005)
- [X] T131 — Integration test for create_task with parent_task_id creating depends_on edge
- [X] T132 — Added TaskTitleEmpty (TitleEmpty) variant to TaskError enum
- [X] T133 — Added error code constant TASK_TITLE_EMPTY: u16 = 3005
- [X] T134 — Added create_task query method to Queries struct
- [X] T135 — Implemented create_task tool in src/tools/write.rs
- [X] T136 — Added create_task dispatch route in src/tools/mod.rs

### Files Modified
- `src/errors/codes.rs` — Added `TASK_TITLE_EMPTY = 3005`
- `src/errors/mod.rs` — Added `TaskError::TitleEmpty` variant and error response mapping
- `src/db/queries.rs` — Added `Queries::create_task()` method with UUID generation and optional parent dependency
- `src/tools/write.rs` — Added `CreateTaskParams` struct, `MAX_TITLE_LEN`, and `create_task` tool function; fixed pre-existing clippy semicolon issue
- `src/tools/mod.rs` — Added `"create_task"` dispatch arm
- `tests/contract/write_test.rs` — Added 4 tests: work_item_id roundtrip (T127), workspace-not-set (T129), empty title (T130), oversized title (T130)
- `tests/integration/hydration_test.rs` — Added parent_task_id edge creation test (T131)

### Test Results
- 80 tests total: 47 unit + 4 contract/lifecycle + 5 contract/read + 10 contract/write + 2 integration/connection + 8 integration/hydration + 1 proptest + 3 proptest/serialization
- All 80 pass
- Library clippy clean; pre-existing float_cmp in search.rs tests remains (unrelated)
- cargo fmt clean

## Important Discoveries

1. **TaskNode serialization**: The `get_task_graph` response uses `children` field (from `TaskNode.children`), not `dependencies` as specified in mcp-tools.json. This is a pre-existing inconsistency from Phase 4's earlier tasks (T054). Tests were written to match the actual implementation.

2. **Semicolon lint fix**: Fixed a pre-existing clippy pedantic issue in write.rs where `warnings.push(...)` was missing a trailing semicolon in the `flush_state` function's stale warning branch.

3. **create_task tool pattern**: Follows the standard tool flow (validate workspace → parse params → connect DB → Queries method → return JSON). Title validation is trimmed before length check, matching the principle of being lenient on input.

## Next Steps

- Phase 4 is now 27/27 complete (all tasks done)
- Phase 5 has 1 remaining task: T108 (graceful shutdown flush)
- Phase 7 has 12 tasks (concurrency)
- Phase 8 has 14 tasks (polish)
- Consider addressing the `children` vs `dependencies` field name inconsistency in a future phase (CHK028 in full-spec.md checklist)

## Context to Preserve

- `TaskNode` struct in `src/tools/read.rs` uses `children` field for dependencies
- `Queries::create_task()` generates UUID v4, creates task with `todo` status, and optionally creates `depends_on` edge to parent
- Pre-existing clippy issues: `float_cmp` in `src/services/search.rs:161,168` (test assertions with f64 zero)
- Error code 3005 maps to `TaskError::TitleEmpty` → name "TaskTitleEmpty"
