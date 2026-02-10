<!-- markdownlint-disable-file -->
# Memory: Phase 5 Stale Strategy

**Created:** 2026-02-10 | **Last Updated:** 2026-02-10

## Task Overview

Implement Phase 5 (US3 continuation) of `001-core-mcp-daemon`: stale file detection with configurable strategy (warn/rehydrate/fail) per FR-012a and FR-012b. Tasks T113-T117 and T123 cover file fingerprint tracking, strategy dispatch in `flush_state`, live staleness in `get_workspace_status`, and integration tests for all three modes.

**Success criteria:** All 79 tests pass (`cargo test`), `cargo clippy` and `cargo fmt` clean, tasks marked complete in `specs/001-core-mcp-daemon/tasks.md`.

## Current State

All Phase 5 tasks complete. Full test suite green (47 unit + 15 contract + 8 integration + 4 proptest + 5 doc = 79 total, 0 failures).

### Files Modified (vs default branch)

| File | Change |
|------|--------|
| [src/server/state.rs](src/server/state.rs) | Added `file_mtimes: HashMap<String, FileFingerprint>` to `WorkspaceSnapshot`; `stale_strategy` field and `with_stale_strategy()` constructor on `AppState`; `update_workspace()` helper; `stale_strategy()` accessor |
| [src/services/hydration.rs](src/services/hydration.rs) | Added `FileFingerprint { modified, len }` struct; `collect_file_mtimes()` and `detect_stale_since()` functions; updated `HydrationSummary` to carry `file_mtimes`; refactored `last_flush_state()` to use fingerprints |
| [src/services/dehydration.rs](src/services/dehydration.rs) | Added safety-net fallback: when DB returns no tasks, parse `tasks.md` from disk to avoid dropping user edits during rehydrate+flush |
| [src/tools/lifecycle.rs](src/tools/lifecycle.rs) | `set_workspace` stores `file_mtimes` in snapshot; `get_workspace_status` recomputes stale via `detect_stale_since` and updates snapshot |
| [src/tools/write.rs](src/tools/write.rs) | `flush_state` checks staleness and applies strategy (Warn pushes warning, Rehydrate rehydrates DB, Fail returns `StaleWorkspace` error); always rehydrates in Rehydrate mode even when not stale; updates snapshot metadata (`last_flush`, `stale_files=false`, `file_mtimes`, `task_count`) |
| [src/bin/t-mem.rs](src/bin/t-mem.rs) | `AppState` initialized with configured stale strategy from `Config` |
| [tests/integration/hydration_test.rs](tests/integration/hydration_test.rs) | Added three integration tests: `flush_state_warns_on_stale_files_in_warn_mode`, `flush_state_rehydrates_before_writing_when_configured`, `flush_state_fails_on_stale_files_in_fail_mode` |
| [tests/contract/read_test.rs](tests/contract/read_test.rs) | Updated `WorkspaceSnapshot` construction to include `file_mtimes` field |
| [specs/001-core-mcp-daemon/tasks.md](specs/001-core-mcp-daemon/tasks.md) | Marked T113-T117, T123 complete |

## Important Discoveries

* **Decisions:**
  - Used `FileFingerprint { modified: SystemTime, len: u64 }` instead of mtime-only to detect in-place content changes that keep the same timestamp (edge case on fast writes)
  - `flush_state` in Rehydrate mode always rehydrates (even when not stale) to guarantee disk-to-DB consistency before writing back
  - Added dehydration safety net: if DB `all_tasks()` returns empty after a rehydrate into a fresh SurrealKV namespace, fall back to parsing `tasks.md` from disk so external edits are not silently dropped
* **Failed Approaches:**
  - Initial attempt relied solely on `detect_stale_since` for the rehydrate branch — on Windows with SurrealKV per-workspace-hash namespacing, the DB connection opened a fresh namespace with no tasks, so `all_tasks()` returned empty after rehydrate. The dehydration then wrote an empty `tasks.md`, losing external edits. Fixed with the disk-fallback safety net in `dehydrate_workspace`
  - File fingerprint-based stale detection alone did not trigger reliably in tests because `fs::write` can complete within the same mtime granularity on some Windows filesystems, requiring the `len` check as a secondary signal

## Next Steps

1. Address remaining tasks in later phases (T125 query_memory limit, US5 items, etc.)
2. Consider Phase 6 and Phase 7 work per [specs/001-core-mcp-daemon/tasks.md](specs/001-core-mcp-daemon/tasks.md)

## Context to Preserve

* **Sources:** `specs/001-core-mcp-daemon/tasks.md` — Phase 5 section (lines 188-199) lists T113-T117, T123
* **Sources:** `specs/001-core-mcp-daemon/plan.md` — Phase 5 scope and FR-012a/b requirements
* **Agents:** `build-feature` SKILL at `.github/skills/build-feature/SKILL.md` — used to drive the phase build
* **Questions:** None unresolved
