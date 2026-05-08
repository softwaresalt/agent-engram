---
title: "028-S CLI Hardening & Startup Validation — Operational Closure"
type: closure
date: 2026-05-08
feature: 043-F
shipment: 028-S
pr: 90
merge_sha: fcdec8d69f2b5a5d3ea9cebc006ebe1a88b6cb13
branch: feat/cli-hardening-startup-validation
---

# 028-S — CLI Hardening & Startup Validation — Closure

## Summary

Shipment 028-S (feature 043-F) hardened the `engram` CLI + daemon startup path so release
builds no longer hang or misreport readiness, and improved test isolation for daemon-spawning
integration tests. The work resolved 10 root-cause bugs discovered during a full CLI test
cycle in debug mode.

## Merge Details

| Field | Value |
|---|---|
| PR | [#90](https://github.com/softwaresalt/agent-engram/pull/90) |
| Merge SHA | `fcdec8d69f2b5a5d3ea9cebc006ebe1a88b6cb13` |
| Merged at | 2026-05-08T06:28:43Z |
| Branch | `feat/cli-hardening-startup-validation` |
| CI | ✅ green |

## Tasks Completed

| Task | Title | Status |
|---|---|---|
| 043.001-T | Fix SurrealQL references in help text and tool schemas | done |
| 043.002-T | Install updated binary and verify | done (manual) |
| 043.003-T | Full CLI test cycle in debug mode | done |
| 043.004-T | Cold-start test with start.ps1 | done (manual) |
| 043.005-T | Add --quiet flag e2e test coverage | archived (deferred) |

## Root Causes Fixed

| ID | Root Cause | Fix Location |
|---|---|---|
| BUG-A | Null IPC request ID when `--id` not supplied | `src/cli/runner.rs` |
| BUG-D | `--depth` help said "default 2" but handler used 1 | `src/bin/engram.rs` |
| BUG-E | `set_hydration_ready()` called after `sync_code_graph` (minutes delay) | `src/tools/lifecycle.rs` |
| — | SurrealQL terminology in help text and 6 tool schemas | `src/bin/engram.rs`, `src/shim/tools_catalog.rs` |
| — | `.copilot/logs/` (1 GB+) hashed on every scan → RAM spike | `src/daemon/watcher.rs`, `src/services/file_tracker.rs`, `src/models/config.rs` |
| — | Auto-sync task raced with `background_db_hydration` for indexing lock | `src/tools/lifecycle.rs` |
| — | `set_workspace` spawn ordered AFTER watcher init (5s block) | `src/daemon/ipc_server.rs` |
| — | `db_path` in `get_workspace_status` used old SurrealDB path | `src/tools/lifecycle.rs` |
| — | `code_graph` stats unconditionally queried DB (broke s072 test) | `src/tools/lifecycle.rs` |
| — | `ENGRAM_DATA_DIR` inherited by all daemon subprocess spawns | `tests/helpers/mod.rs`, `src/shim/lifecycle.rs` |
| — | Test harness 30-attempt backoff only gave ~12.6s not 30s | `tests/helpers/mod.rs` |
| — | `let _ = state.try_start_indexing()` discarded lock result (P1 concurrency) | `src/tools/lifecycle.rs` |

## Files Modified

- `src/bin/engram.rs` — SurrealQL→Datalog/CozoScript; depth default fix
- `src/cli/runner.rs` — auto-generate request ID
- `src/daemon/ipc_server.rs` — workspace spawn before watcher init
- `src/daemon/watcher.rs` — .copilot/ .copilot-tracking/ in DEFAULT_EXCLUDE_PREFIXES
- `src/models/config.rs` — .copilot/ .copilot-tracking/ in default_exclude_patterns
- `src/services/file_tracker.rs` — MAX_HASH_FILE_BYTES = 10 MiB guard
- `src/services/gate.rs` — doc comment terminology fix
- `src/shim/lifecycle.rs` — .env_remove("ENGRAM_DATA_DIR") in spawn_daemon
- `src/shim/tools_catalog.rs` — 6 tool schemas corrected; unified_search region narrowed; max_nodes defaults added
- `src/tools/lifecycle.rs` — acquired_lock guard; set_hydration_ready order; code_graph feature guard; CozoDB db_path
- `tests/helpers/mod.rs` — deadline polling; .env_remove("ENGRAM_DATA_DIR") in all 3 spawns

## Healthy Signals Post-Merge

- `cargo test` — all tests pass ✅
- `cargo clippy -- -D warnings -D clippy::pedantic` — clean ✅
- `cargo fmt --all -- --check` — clean ✅
- CI green on merge commit `fcdec8d` ✅
- `engram --help` shows Datalog/CozoScript terminology ✅
- Daemon readiness no longer blocked by `sync_code_graph` before `set_hydration_ready` ✅
- Test suite runs in isolation (no production CozoDB opened by daemon subprocesses) ✅

## Failure Signals to Watch

- `SQLITE_BUSY` errors from stats/search/query-memory MCP tools during active indexing
  → Tracked as BUG-B (`B6F12A12`) in stash for future shipment
- `sync` command 30s IPC timeout when indexing in progress
  → Tracked as BUG-C (`D3C7173A`) in stash for future shipment

## Rollback Procedure

```bash
git revert --no-edit -m 1 fcdec8d69f2b5a5d3ea9cebc006ebe1a88b6cb13
```

## Deferred Work (Stash)

| Stash ID | Description |
|---|---|
| `B6F12A12` | SQLITE_BUSY panic for stats/search/query-memory during indexing |
| `D3C7173A` | sync command 30s IPC timeout when indexing in progress |

## Compound Learnings

- `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`
  — ENGRAM_DATA_DIR must be stripped from all daemon subprocess spawns (test and production)
