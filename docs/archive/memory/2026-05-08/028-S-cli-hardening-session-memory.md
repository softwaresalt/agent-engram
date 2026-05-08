---
title: "028-S CLI Hardening Session Memory"
type: session-memory
date: 2026-05-08
feature: 043-F
shipment: 028-S
branch: feat/cli-hardening-startup-validation
pr: "https://github.com/softwaresalt/agent-engram/pull/90"
status: pr-open-awaiting-merge
---

## Tasks Completed This Session

| Task | Title | Status | Commits |
|---|---|---|---|
| 043.001-T | Fix stale SurrealQL references in CLI help text | done (archived) | 4bbf245, 062af8f |
| 043.003-T | Full CLI command test cycle in debug mode | done | 6b5f384, f43fe0a, 60a254e |

## Tasks Remaining in Shipment

| Task | Title | Status | Notes |
|---|---|---|---|
| 043.002-T | Install updated engram binary for start.ps1 | active | Manual: `cargo install --path .` after merge |
| 043.004-T | Cold-start test of start.ps1 with updated binary | active | Manual test — depends on 043.002-T |
| 043.005-T | Add --quiet flag e2e test coverage | active | Code task; not started |

## Files Modified

- `src/bin/engram.rs` — SurrealQL → Datalog/CozoScript help text; depth default
- `src/services/gate.rs` — doc comment SurrealQL → Datalog
- `src/shim/tools_catalog.rs` — 6 tool schemas corrected (param names + defaults)
- `src/cli/runner.rs` — auto-generate request ID `"1"` when `--id` not supplied
- `src/tools/lifecycle.rs` — acquired_lock guard; try_start_indexing at entry; set_hydration_ready before sync_code_graph; code_graph gated on git-graph feature; CozoDB db_path fix
- `src/daemon/ipc_server.rs` — set_workspace spawn before watcher init
- `src/daemon/watcher.rs` — .copilot/ and .copilot-tracking/ in DEFAULT_EXCLUDE_PREFIXES
- `src/services/file_tracker.rs` — MAX_HASH_FILE_BYTES = 10 MiB guard
- `src/models/config.rs` — .copilot/ and .copilot-tracking/ in default_exclude_patterns()
- `src/shim/lifecycle.rs` — .env_remove("ENGRAM_DATA_DIR") in spawn_daemon
- `tests/helpers/mod.rs` — deadline-only polling; .env_remove("ENGRAM_DATA_DIR") in all 3 spawns

## Root Cause Chain (Full Discovery Order)

1. SurrealQL in help text (schema drift)
2. Null IPC request IDs (runner.rs never generated default)
3. `--depth` help says "default 2" but handler uses 1
4. Daemon never reached Ready — set_hydration_ready() called AFTER sync_code_graph (minutes-long on large workspace)
5. .copilot/logs/ 1 GB+ files hashed on every scan → RAM spike
6. SQLITE_BUSY cascade — auto-sync task raced with background_db_hydration for DB connection
7. Watcher init (5s) blocked set_workspace spawn → fixed ordering in ipc_server.rs
8. db_path in get_workspace_status used old SurrealDB path
9. code_graph stats unconditionally queried DB — broke s072 test when git-graph feature disabled
10. ENGRAM_DATA_DIR inherited by test subprocesses → all shim lifecycle tests timed out (production CozoDB opened)
11. Test harness MAX_ATTEMPTS=30 gave ~12.6s not 30s timeout
12. (P1 review finding) `let _ = try_start_indexing()` discarded return value → premature lock release if another task held it

## Decisions Made

- `set_hydration_ready()` is intentionally called BEFORE `sync_code_graph` — clients get "ready" promptly; all code-graph tool handlers already guard via `is_indexing()`
- `.env_remove("ENGRAM_DATA_DIR")` added to both DaemonHarness spawn functions AND `spawn_daemon` in shim/lifecycle.rs so ALL daemon spawns (tests and production shim) create isolated daemons
- `MAX_HASH_FILE_BYTES = 10 MiB` is a trade-off: files >10 MiB are not hashed for change detection (returns "no change"). This is acceptable since large binary files rarely need re-indexing.

## Stashed Bugs for Future Shipment

- **BUG-B** (`B6F12A12`): SQLITE_BUSY panic for stats/search/query-memory during indexing
- **BUG-C** (`D3C7173A`): sync command 30s IPC timeout when indexing in progress

## Quality Gates at Session End

- `cargo fmt --all -- --check`: ✅
- `cargo clippy -- -D warnings -D clippy::pedantic`: ✅
- `cargo test --test contract_shim_lifecycle --test integration_daemon_lifecycle --test integration_smoke`: ✅
- Full `cargo test` (previous run before acquired_lock fix): ✅ (only s072 was failing, fixed in same session)

## Next Steps

1. Wait for CI on PR #90 to go green
2. Wait for Copilot review on PR #90
3. User approves merge
4. Post-merge: `cargo install --path .` (043.002-T), cold-start test (043.004-T)
5. Future shipment: implement 043.005-T (--quiet e2e tests) and stashed BUG-B/BUG-C
