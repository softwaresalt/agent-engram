---
title: "034-S Daemon Startup Reliability — Session Memory"
type: session-memory
date: 2026-05-09
shipment: 034-S
feature: 049-F
pr: 127
merge_sha: 6ed2b36fd7efb3e80eea280d370e78f49c4b3209
---

## Tasks Completed

| Task ID | Title | Outcome |
|---|---|---|
| 049.001-T | Deadline-based poll_until_ready | ✅ merged |
| 049.002-T | Early set_hydration_ready | ✅ merged |
| 049.003-T | yield_now in hydration loops | ✅ merged |
| 049.004-T | ENGRAM_AUTO_REINDEX gate | ✅ merged |
| 049.005-T | Diagnostic logging for empty code count | ✅ merged |
| 049.006-T | Remove is_indexing guards from read tools | ✅ merged |

## Files Modified

- `src/shim/lifecycle.rs` — Deadline-based poll loop
- `src/tools/lifecycle.rs` — Early hydration-ready + auto-reindex gate
- `src/tools/read.rs` — Removed indexing guards from 7 handlers
- `src/services/hydration.rs` — yield_now every 50 upserts; diagnostic logging
- `tests/contract/read_test.rs` — 3 tests: allowed during indexing, dual assert_ne!
- `tests/integration/indexing_resilience_test.rs` — t_ixr_01..03 updated

## Key Decisions

1. **Early hydration-ready**: `set_hydration_ready()` moved from after JSONL load to immediately after `connect_db`. Rationale: the shim's health check fires within 500ms; if we wait for 22MB JSONL load we always time out.

2. **ENGRAM_AUTO_REINDEX gate**: Auto-reindex at startup scans all source files. With 1,382 files it consumed 14GB+ RAM. Defaulting to `false` and requiring opt-in via `ENGRAM_AUTO_REINDEX=true` prevents OOM crashes.

3. **yield_now counter inside upsert branch**: The counter for "every 50 attempts" must only increment when an actual upsert is attempted (inside `if let Ok(node)` block), not on corrupt lines. Copilot review caught this.

4. **Dual assert_ne! in tests**: Contract tests for "allowed while indexing" must assert neither INDEX_IN_PROGRESS nor WORKSPACE_NOT_SET — single assertion was too permissive.

## CI Issue Encountered

Integration tests in `tests/integration/indexing_resilience_test.rs` were NOT found by `cargo test contract` filter (they're in `integration` module). When read tool guards were removed, these tests still expected `INDEX_IN_PROGRESS`. Fix: updated t_ixr_01..03 to assert NOT IndexInProgress. Lesson: when removing error codes from handlers, search ALL test directories for assertions on that error code.

## Stash Entries

Stash IDs for this work: BDA80DA0, A12DCDE8, 3B541819, 232C7A71, 40C376F5, 6FC4BEFA

## Open Items at Session End

- `chore/034-s-post-merge-archive` branch with PR #128 — needs merge (trivial archive housekeeping)
- Stash entries not yet marked harvested in stash.jsonl
- Closure doc, memory, compound learning written but not committed (pending PR #128)

## Next Steps

1. Merge PR #128 (archive housekeeping)
2. Verify daemon works post-fix: `engram.exe daemon --workspace <path>` should reach Ready within 30s
3. Review queue for next shipment candidates
