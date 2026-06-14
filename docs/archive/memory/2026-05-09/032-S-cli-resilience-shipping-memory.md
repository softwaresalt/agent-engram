---
type: session-memory
date: 2026-05-09
shipment: 032-S
feature: 047-F
merge_sha: 10134ad
pr: 114
---

## Tasks Completed

- 047.004-T: Guard `--direct` mode against daemon-held database ✅
- 047.005-T: Harden IndexInProgress detection fallback ✅
- 047.006-T: Daemon startup progress indicator ✅
- 047-F: Feature archived ✅
- 032-S: Shipment shipped and archived ✅

## Files Modified

| File | Change |
|---|---|
| `src/cli/direct.rs` | db-lock probe + FdRwLock guard + unit test |
| `src/cli/output.rs` | `progress_hint()` method + 3 unit tests |
| `src/cli/runner.rs` | `friendly_error_message()` + check_health probe; 9 new unit tests |
| `tests/integration/cli_direct_test.rs` | `direct_sync_detects_locked_database` integration test |
| `docs/closure/2026-05-09-032-S-cli-resilience-error-handling-closure.md` | Closure doc |

## Key Decisions

1. **db-lock probe path**: Used `branch.replace(['/', '\\', ':'], "_")` to match
   `connect_db`'s exact `branch_safe` sanitization — NOT the double-underscore form
   from `resolve_git_branch`. Critical for correct path computation.

2. **Copilot review fix**: Changed `if let Ok(f)` to explicit `match` so `open()` errors
   return exit 2 immediately instead of silently falling through to the 30-second
   connect_db timeout.

3. **Windows-only integration test**: `fd_lock::LockFileEx` on Windows enforces
   per-handle exclusivity within the same process; Linux/macOS advisory flock does not.
   Test is `#[cfg(target_os = "windows")]`.

4. **Progress hint ordering**: IPC endpoint computation moved BEFORE `ensure_daemon_running`
   so we can do the pre-spawn `check_health` probe.

5. **IndexInProgress fallback**: Code `-32603` + message heuristic is a secondary path
   only — the primary `engram_code == 7003` path takes priority.

## Pre-existing Issues (not caused by this shipment)

- `t047_data_persists_across_crash_and_restart`: flaky under concurrent test load,
  passes in isolation. Pre-existing timing issue.

## Next Steps

- Compound refresh if these patterns recur
- Next shipment to be determined by Stage agent
