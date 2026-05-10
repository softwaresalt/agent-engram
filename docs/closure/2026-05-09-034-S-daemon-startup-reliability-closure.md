---
title: "034-S Daemon Startup and Indexing Reliability — Closure"
type: closure
date: 2026-05-09
feature: 049-F
shipment: 034-S
pr: 127
merge_sha: 6ed2b36fd7efb3e80eea280d370e78f49c4b3209
status: closed
---

## Summary

Fixed 6 daemon startup and indexing reliability bugs that were preventing `engram` from
reaching Ready state and making read-only tools unavailable during indexing. All 6 backlog
items (049-F + 049.001–006-T) delivered via PR #127, merged 2026-05-09.

## Delivered Items

| ID | Title | Status |
|---|---|---|
| 049-F | Daemon Startup and Indexing Reliability | archived |
| 049.001-T | Deadline-based poll_until_ready loop | archived |
| 049.002-T | Early set_hydration_ready after connect_db | archived |
| 049.003-T | Tokio yield_now in hydration loops | archived |
| 049.004-T | ENGRAM_AUTO_REINDEX gate for startup re-index | archived |
| 049.005-T | Diagnostic logging for count_code_files=0 | archived |
| 049.006-T | Remove is_indexing guards from read-only tools | archived |

## Files Changed

- `src/shim/lifecycle.rs` — Deadline-based `poll_until_ready` loop driven by `ENGRAM_READY_TIMEOUT_MS`
- `src/tools/lifecycle.rs` — Early `set_hydration_ready()` after `connect_db`; `ENGRAM_AUTO_REINDEX` gate
- `src/tools/read.rs` — Removed `if state.is_indexing()` guards from 7 read handlers
- `src/services/hydration.rs` — `yield_now()` every 50 actual upserts; diagnostic logging for empty code file count
- `tests/contract/read_test.rs` — 3 tests updated to assert tools allowed during indexing (dual `assert_ne!`)
- `tests/integration/indexing_resilience_test.rs` — t_ixr_01..03 updated to assert NOT IndexInProgress

## Root Causes Fixed

| Bug | Root Cause | Fix |
|---|---|---|
| BDA80DA0 | `poll_until_ready` ignored `ENGRAM_READY_TIMEOUT_MS` (fixed 30-attempt cap ~15s) | Deadline-based loop |
| A12DCDE8 | `set_hydration_ready()` called only after 22MB JSONL load completed | Moved to immediately after `connect_db` |
| 3B541819 | Synchronous `run_script()` inside async fn blocked tokio executor during JSONL load | `yield_now()` every 50 actual upserts |
| 232C7A71 | Auto-reindex scanned all 1,382 files at startup causing 14GB+ RAM use | `ENGRAM_AUTO_REINDEX=true` env gate; default false |
| 40C376F5 | `count_code_files()` returned 0 with no diagnostic info | Added `info!` logging with `data_dir`/`branch` fields |
| 6FC4BEFA | 7 read-only tools rejected all queries while indexing in progress | Removed `is_indexing()` guards; tools return partial data |

## CI Notes

A CI failure occurred after the initial push: 3 integration tests in
`tests/integration/indexing_resilience_test.rs` (t_ixr_01..03) still expected
`INDEX_IN_PROGRESS`. Fixed in commit `eb2ee30`, CI went green.

Copilot review found 3 issues:
1. `read_test.rs` contract tests were too permissive (only one `assert_ne!`) — added dual assertions
2. `hydration.rs` yield_now counter incremented on corrupt lines — moved inside upsert branch
3. Review confirmed integration tests were already fixed

## Monitoring Plan

- **Signal**: Daemon reaches Ready state within 30s at default settings; within `ENGRAM_READY_TIMEOUT_MS` when set
- **Baseline**: Daemon RAM consumption < 500MB at startup without `ENGRAM_AUTO_REINDEX=true`
- **Alert**: Daemon fails to reach Ready within configured timeout → check `ENGRAM_READY_TIMEOUT_MS` env var
- **Owner**: Agent operator

## Rollback

```bash
git revert --no-edit -m 1 6ed2b36fd7efb3e80eea280d370e78f49c4b3209
```

## Pre-Deploy Checklist

- [x] All quality gates passed (fmt → clippy → test)
- [x] PR #127 reviewed (Copilot review, 3 comments addressed)
- [x] CI green on all platforms
- [x] No breaking changes to MCP tool contracts
- [x] `ENGRAM_AUTO_REINDEX` defaults to `false` (safe opt-in)

## Post-Deploy Observation

- Observation window: immediate (fixes are defensive; daemon starts correctly or exits)
- Owner: agent operator
- Outcome: healthy — daemon reaches Ready state, read tools available during indexing

## Stash Entries

Source stash IDs: BDA80DA0, A12DCDE8, 3B541819, 232C7A71, 40C376F5, 6FC4BEFA
