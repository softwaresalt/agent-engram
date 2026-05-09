---
title: CLI Full Test Cycle — Session Memory
type: memory
date: 2026-05-08
session: cli-test-cycle
---

# CLI Full Test Cycle — Session Memory

## Context

Following user request to run all `engram` CLI commands through a full real-life
test cycle in debug mode and capture bugs. Task 043.003-T (archived) tracked this
but the actual execution was never performed. This session completed it.

## Test Environment

- Binary: `target/debug/engram.exe` (built 2026-05-08, v0.0.1)
- Test workspaces: fresh temp dirs in `%TEMP%` with git init + `engram install`
- Important: `ENGRAM_DATA_DIR` must be unset for isolated workspace tests

## Test Results — All Commands

| Command | Status | Notes |
|---------|--------|-------|
| `--help` | ✅ PASS | Clean; SurrealQL fix confirmed |
| `--version` | ✅ PASS | Returns `engram 0.0.1` |
| `manifest` | ✅ PASS | Lists all 18 MCP tools |
| `install` | ✅ PASS | Works from CWD; `--workspace` flag ignored (bug BC9A6B23) |
| `update` | ✅ PASS | Works from CWD; `--workspace` flag ignored (bug BC9A6B23) |
| `reinstall` | ✅ PASS | Works from CWD; `--workspace` flag ignored (bug BC9A6B23) |
| `uninstall` | ✅ PASS | Works from CWD; requires daemon stopped first; `--workspace` ignored (bug BC9A6B23) |
| `daemon-status` | ✅ PASS | Auto-spawns daemon; prints health; long startup delay on first run (bug E0CF06A6) |
| `workspace-status` | ✅ PASS | Returns workspace path, db_path, scan_status, code_graph stats |
| `sync` (IPC) | ✅ PASS | Returns incremental sync results |
| `sync --direct` | ✅ PASS | Correct when no daemon running and ENGRAM_DATA_DIR unset; fails with BUSY panic when shared dir locked (bug A98E9409) |
| `index` (IPC) | ✅ PASS | Returns full index results; shows `files_skipped` for .rs without Cargo.toml |
| `index --direct` | ✅ PASS | Same behavior as `sync --direct` |
| `flush` | ✅ PASS | Writes code-graph JSONL and metrics summary |
| `bind` | ✅ PASS | Re-binds workspace, returns workspace_id and path |
| `symbols` | ✅ PASS | Returns indexed functions/structs with file and line info |
| `search` | ✅ PASS | Semantic search returns scored results |
| `query-memory` | ✅ PASS | Returns empty array for empty workspace (correct) |
| `map-code` | ✅ PASS | Returns call graph with edges and neighbors |
| `impact` | ✅ PASS | Returns code_neighborhood for modified symbol |
| `query-graph` | ✅ PASS | Returns expected stub error "not yet implemented (Phase 2)" |
| `stats` | ✅ PASS | Returns embedding status, code graph counts, registry info |
| `health` | ✅ PASS | Returns latency percentiles, tool call counts, uptime |
| `branch-metrics` | ✅ PASS | Returns per-tool token metrics for current branch |
| `report token-savings` | ✅ PASS | Returns human-readable token delivery summary |
| `report retry-metrics` | ✅ PASS | Returns retry_count and last_retry_at |
| `report eval` | ✅ PASS | Returns efficiency_score, anomalies, recommendations |

**27/27 commands pass functionally.** 3 bugs found and stashed.

## Bugs Stashed

### BC9A6B23 — HIGH — Installer commands ignore `--workspace` flag

`install`, `update`, `reinstall`, `uninstall` all use `std::env::current_dir()` instead
of `flags.resolve_workspace()` (src/bin/engram.rs lines 257, 266, 270, 274). Running
`engram install --workspace /path/to/ws` silently installs into CWD. Fix: replace
`current_dir()` with the flags-resolved workspace in all four dispatch arms.

### A98E9409 — MEDIUM — `--direct` mode panics with CozoDB SQLITE_BUSY

`sync --direct` / `index --direct` panic when `ENGRAM_DATA_DIR` points to a shared
directory whose SQLite database is already open by a running daemon. The daemon lock
is acquired successfully but `DB open task panicked` with `Error { code: Some(5) }`.
Should be caught and surfaced as: "workspace database is locked by daemon (PID N)".
Reproduce: set `ENGRAM_DATA_DIR=<active-workspace>/.engram`, run `sync --direct`.

### E0CF06A6 — LOW — No progress indicator during daemon auto-spawn

First CLI command in a fresh workspace hangs silently for 30-60 seconds while the
daemon initializes (CozoDB init, embedding model load, workspace scan). No output
during this period. Fix: print a "Starting engram daemon..." status line before
blocking on `ensure_daemon_running`.

## Observations

- `query-graph --help` correctly says "Datalog (CozoScript)" — SurrealQL fix confirmed
- `daemon-status` health check correctly returns `overall: yellow` with remediation hints
- `report eval` `token_ratio_spike` anomaly fires when tokens-per-result > 10 threshold
- `ENGRAM_DATA_DIR` must be unset for test isolation; when set globally it routes all
  workspaces to the same DB, causing SQLITE_BUSY under concurrent access
- `install` creates `.engram/run/engram.lock` and `.engram/run/engram.pid` only after
  the lock is acquired on first daemon spawn, not during `install` itself

## Next Steps

- Stage should review BC9A6B23 (high priority) for inclusion in the next shipment
- A98E9409 and E0CF06A6 are medium/low — candidates for a polish shipment
- `029-S` (Indexing Resilience) is still queued and ready for next Ship session
