---
title: "029-S Indexing Resilience — Ship Session Memory"
type: session-memory
date: 2026-05-09
feature: 044-F
shipment: 029-S
pr: 101
merge_sha: 8f23c3c844eab79971f40082691b26c1391a8547
status: shipped
---

## What Was Done

Shipped feature 044-F (Indexing Resilience — SQLITE_BUSY Guards, Queued Sync, Configurable CLI Timeout) as shipment 029-S, merged in PR #101.

### Tasks Completed
- **044.001-T**: Added `is_indexing()` guards to `get_workspace_statistics`, `query_memory`, `unified_search`, `query_changes` in `src/tools/read.rs`. Returns `IndexInProgress` (7003) instead of hitting locked DB.
- **044.002-T**: CLI user-friendly IndexInProgress message in `src/cli/runner.rs` — detects `err.data["engram_code"] == 7003` and shows retry instruction.
- **044.003-T**: 5 integration tests in `tests/integration/indexing_resilience_test.rs` (T-IXR-01 through T-IXR-05).
- **044.004-T**: `pending_sync: AtomicBool` in `AppState` — `sync_workspace` returns `{"status":"queued"}` when indexing active; `drain_pending_sync()` runs coalesced sync after indexing completes.
- **044.005-T**: `--timeout`/`ENGRAM_CLI_TIMEOUT` global flag — index/sync --full default 300s, all others 30s.

### Review Fixes (pre-merge, multi-persona review)
Pre-merge review found 4 P1 bugs all fixed in commit `d56701d`:
1. **Drain race**: `take_pending_sync()` was clearing the flag before `try_start_indexing()` confirmed the lock. Fixed: re-set the flag if lock acquisition fails.
2. **Missing drain in `index_workspace`**: `finish_indexing()` at `write.rs:138` had no drain. Fixed.
3. **Missing drain in startup hydration** (`ipc_server.rs:613`). Fixed.
4. **Missing drain in file-watcher auto-sync** (`ipc_server.rs:694`). Fixed.
P2-01 (sync_workspace missing drain) also fixed in same commit.

## Files Modified
- `src/tools/read.rs` — is_indexing() guards
- `src/tools/write.rs` — queued sync + drain at finish_indexing
- `src/tools/lifecycle.rs` — drain_pending_sync() helper (extracted + race-fixed)
- `src/server/state.rs` — pending_sync AtomicBool + set/take methods
- `src/daemon/ipc_server.rs` — drain at both finish_indexing sites
- `src/cli/flags.rs` — --timeout global flag
- `src/cli/runner.rs` — run_tool_timed(), IndexInProgress friendly message
- `src/cli/commands/indexing.rs` — 300s timeout for index/sync --full
- `tests/contract/read_test.rs` — 3 new tests
- `tests/contract/write_test.rs` — queues_while_in_progress test
- `tests/integration/indexing_resilience_test.rs` — NEW (5 tests)
- `tests/unit/cli_parser_test.rs` — --timeout in global flags

## Key Technical Decisions

### Wire protocol for error codes
`IpcResponse::error()` wraps all `EngramError` as JSON-RPC code `-32603` with `data: { "engram_code": <u16> }`. The actual Engram error code (7003 for IndexInProgress) lives in `err.data["engram_code"]`, NOT in `err.code`.

### drain_pending_sync design
- Only `take_pending_sync()` as fast-path guard (no flag = immediate return)
- Acquire lock BEFORE consuming the flag is not possible with `compare_exchange`, so re-set on lock failure
- Called from every `finish_indexing()` site (4 total call sites)
- Coalesces all concurrent sync requests into single run

### SeqCst ordering (P3 advisory — not fixed)
All AtomicBool ops use SeqCst. AcqRel/Acquire/Release would suffice but correctness takes priority over micro-optimization. Stashed as advisory.

## Open Items
- Stash 3AA1E6DD: Harden IndexInProgress detection in CLI runner (P2-02 — low priority)
- Pre-existing flaky test: `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` (SQLITE_BUSY on Windows during schema bootstrap — unrelated to this shipment)

## Commits
- `76fdef8` — feat(tools): add is_indexing() guards to unguarded read-only tool handlers
- `aa62123` — feat(tools): queued sync, configurable CLI timeout, IndexInProgress message (044.002-005)
- `85fe4f1` — chore(backlog): mark 044-F tasks done and 029-S active
- `d56701d` — fix(tools): drain pending_sync at all finish_indexing sites (PR review P1 fixes)
- `8f23c3c` — merge commit for PR #101

## Compound Learnings Written
- `docs/compound/concurrency-issues/pending-sync-drain-must-cover-all-finish-indexing-sites-2026-05-09.md`
- `docs/compound/concurrency-issues/atomicbool-drain-race-take-before-lock-2026-05-09.md`
