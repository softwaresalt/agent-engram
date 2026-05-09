---
title: "032-S CLI Resilience & Error Handling — Closure"
type: closure
date: 2026-05-09
feature: 047-F
shipment: 032-S
pr: 114
merge_sha: 10134ad
branch: feat/032-s-cli-resilience-error-handling
---

## Summary

Shipped 3 CLI resilience improvements for 047-F across `src/cli/direct.rs`,
`src/cli/output.rs`, and `src/cli/runner.rs`.

## Tasks Completed

| Task | Title | Status |
|---|---|---|
| 047.004-T | Guard `--direct` mode against daemon-held database | archived |
| 047.005-T | Harden IndexInProgress detection fallback | archived |
| 047.006-T | Daemon startup progress indicator | archived |

## Changes Shipped

### 047.004-T — db-lock guard in `--direct` mode (`src/cli/direct.rs`)

- After `DaemonLock::acquire` succeeds and `data_dir`+`branch` are resolved,
  probe `data_dir/cozo/{branch_safe}/engram.db.lock` with `fd_lock::try_write()`
- `Err` from `open()` → exit 2: "cannot open workspace database lock file: {e}"
  (Copilot review fix: handles permissions/sharing violations, avoids 30 s fallthrough)
- `try_write()` failure → exit 2: "workspace database is locked by another process;
  stop the daemon first or use IPC mode (omit --direct)"
- Branch sanitization matches `connect_db`: `branch.replace(['/', '\\', ':'], "_")`
- Integration test `direct_sync_detects_locked_database` (Windows-only, uses
  background thread holding the fd_lock to simulate a running daemon)
- Unit test `fd_lock_try_write_conflicts_with_held_write_lock` documents
  Windows per-handle exclusivity guarantee

### 047.005-T — IndexInProgress fallback (`src/cli/runner.rs`)

- Extracted `friendly_error_message(err: &IpcError) -> String` helper
- Primary path: `data.engram_code == 7003` → "Indexing is in progress…"
- Fallback path: code `-32603` + message contains "index" and "progress" → same
  (covers daemon versions without explicit `engram_code` field)
- 5 unit tests: primary, fallback, primary-wins precedence, two no-false-positives

### 047.006-T — Progress hint (`src/cli/output.rs`, `src/cli/runner.rs`)

- Added `progress_hint(message: &str)` to `OutputFormatter`: emits to stderr in
  text mode only; suppressed in JSON mode and `--quiet`
- In `run_tool_timed`: compute IPC endpoint before `ensure_daemon_running`, run
  500 ms `check_health` probe, emit "Starting engram daemon…" when not ready
- 3 unit tests for suppression rules (JSON mode, quiet mode, combined)

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | ✅ |
| `cargo clippy -- -D warnings -D clippy::pedantic` | ✅ |
| `cargo test` | ✅ (1 pre-existing flaky test unrelated to this PR) |
| CI | ✅ (2 m 43 s) |

## Copilot Review

- 1 finding: silent `if let Ok(f)` on lock file open → fixed in fb5f5d1 with
  explicit `match` returning exit 2 on `Err`
- Thread resolved programmatically

## Pre-existing Flaky Test

`t047_data_persists_across_crash_and_restart` fails under concurrent test load
but passes in isolation. Not caused by this PR (timing/resource contention).
Tracked in compound library at `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`.

## Rollback

```bash
git revert --no-edit -m 1 10134ad
```

## Monitoring

These are defensive UX improvements (fast error paths, progress messaging).
No new runtime surfaces introduced. No monitoring plan required.
