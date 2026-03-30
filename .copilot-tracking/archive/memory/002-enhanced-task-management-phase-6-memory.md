# Phase 6 Memory: US4 — Agent-Driven Compaction (T041-T048)

**Date**: 2026-02-14
**Spec**: 002-enhanced-task-management
**Phase**: 6
**Commit**: `8efcaec`
**Branch**: 002-enhanced-task-management

## Tasks Completed

- **T041**: Contract tests — 4 read compaction tests + 2 write compaction tests (all pass)
- **T042**: Compaction candidate query (`get_compaction_candidates`) and `apply_compaction` in `queries.rs`
- **T043**: `get_compaction_candidates` handler in `read.rs` — reads `CompactionConfig`, returns candidates with age_days
- **T044**: `apply_compaction` handler in `write.rs` — strips `task:` prefix, calls query, returns results
- **T045**: Rule-based truncation service in `services/compaction.rs` — `truncate_at_word_boundary()`
- **T046**: 6 unit tests for truncation service (all pass)
- **T047**: Integration test — 50 done tasks, candidates, apply, graph preservation, flush/rehydrate round-trip
- **T048**: Integration test — pinned exclusion, compaction_level increment to level 2

## Files Modified

- `src/db/schema.rs` — Changed all `VALUE time::now()` → `DEFAULT time::now()`, added `DEFINE FIELD OVERWRITE`, `DEFINE TABLE IF NOT EXISTS`, `DEFINE INDEX IF NOT EXISTS`
- `src/db/queries.rs` — Added `get_compaction_candidates()` and `apply_compaction()` methods
- `src/tools/read.rs` — Added `get_compaction_candidates` handler with `GetCompactionCandidatesParams`
- `src/tools/write.rs` — Added `apply_compaction` handler with `CompactionItem`/`ApplyCompactionParams`
- `src/tools/mod.rs` — Moved compaction tools from stub to handler dispatch
- `src/services/compaction.rs` — Implemented `truncate_at_word_boundary()` with 6 inline unit tests
- `src/services/dehydration.rs` — Added `compaction_level` and `compacted_at` to frontmatter output
- `src/services/hydration.rs` — Added parsing for `compaction_level` and `compacted_at` from frontmatter
- `tests/contract/read_test.rs` — 4 compaction contract tests
- `tests/contract/write_test.rs` — 2 compaction contract tests
- `tests/integration/enhanced_features_test.rs` — T047 and T048 integration tests

## Key Decisions

- **VALUE → DEFAULT schema change**: SurrealDB `VALUE` always recomputes on every write, preventing test control of timestamps. `DEFAULT` only applies when field is omitted, which is safe because all write paths explicitly set timestamps.
- **Schema evolution modifiers**: Added `OVERWRITE` on `DEFINE FIELD` and `IF NOT EXISTS` on `DEFINE TABLE`/`DEFINE INDEX` to support incremental schema changes.
- **RecordId import path**: Use `surrealdb::RecordId` (not `surrealdb::opt::RecordId`) for direct DB queries.
- **Duration format**: SurrealQL `type::duration($threshold)` expects format `"7d"` for day arithmetic.

## Test Results

- 145 tests pass across all binaries
- 1 pre-existing failure: `t098_hydration_1000_tasks_under_500ms` (debug build too slow, 5.4s vs 5s target)
- Clippy clean, fmt clean

## Remaining Stubs in `tools/mod.rs`

claim_task, release_task, defer_task, undefer_task, pin_task, unpin_task, get_workspace_statistics, batch_update_tasks, add_comment (9 stubs)

## Next Phase

Phase 7: US5 — Task Claiming and Assignment (T049-T053)
