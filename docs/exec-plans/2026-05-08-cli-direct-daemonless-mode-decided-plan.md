---
title: "CLI-Direct Daemonless Mode — Decided Plan"
type: decided-plan
date: 2026-05-08
feature: 045-F
shipment: 030-S
source: docs/archive/plans/2026-05-08-cli-direct-daemonless-mode-plan.md
status: shipped
merge_sha: d09bba011bce49cccf3dd9377aa4e0126cdee262
---

## Decision Summary

Add `--direct` flag to `engram sync` and `engram index` to bypass the daemon entirely. Use `DaemonLock` for mutual exclusion. Record per-file hashes in `sync_workspace` so daemon freshness detection returns 0 changes after a `--direct` run and skips re-indexing.

**Chosen option**: Option D from deliberation (`docs/decisions/2026-05-08-cli-direct-daemonless-mode-deliberation.md`).

## Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| `DaemonLock` in `run_direct_sync` — exit 2 if held | Prevents concurrent DB writes; reuses existing lock mechanism |
| `Box::leak` for lock guard | One lock per process lifetime; OS reclaims on exit |
| `BoolishValueParser` for `ENGRAM_DIRECT` | clap 4 default bool parser only accepts "true"/"false"; `BoolishValueParser` accepts "1" |
| `record_file_hash_precomputed` | Avoids double disk I/O — caller already has hash in memory |
| `count_code_files` fast-path in hydration | Skips JSONL reload when DB already populated after `--direct` run |
| `set_hydration_ready` before `sync_code_graph` | Clients get "ready" promptly; tool handlers already gate on `is_indexing()` |
| `canonicalize_workspace` rejects non-git dirs | Returns Err (does not fall back); falls back to `"default"` branch name |

## Units Shipped

| Unit | Task | Files | Status |
|------|------|-------|--------|
| 1 | 045.001-T | `src/cli/direct.rs` (new), `src/cli/mod.rs` | ✅ |
| 2 | 045.002-T | `src/bin/engram.rs`, `src/cli/commands/indexing.rs` | ✅ |
| 3 | 045.003-T | `tests/integration/cli_direct_test.rs` (5 tests; cases 3+5 deferred) | ✅ |
| 4 | 045.004-T | `src/services/code_graph.rs`, `src/services/file_tracker.rs`, `src/services/hydration.rs` | ✅ |

## Deferred

- Test case 3 (daemon mutex) and test case 5 (no orphaned processes) — require `DaemonHarness` subprocess infrastructure not yet available. To be addressed in a future shipment.

## Rejected Alternatives

- **Option A** (daemon always required): Rejected — creates circular dependency for `start.ps1` preloading
- **Option B** (separate binary): Rejected — doubles distribution surface
- **Option C** (embedded daemon timeout): Rejected — adds complexity without resolving root cause

## Constraints

- `--direct` flag is per-subcommand (not global) — only `sync` and `index` support it
- `DaemonLock` is workspace-scoped — CLI-direct and daemon cannot coexist on same workspace
- Files >10 MiB are not hashed for change detection (MAX_HASH_FILE_BYTES guard)
