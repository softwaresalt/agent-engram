---
session: ship-009-s-daemon-b2
date: 2026-04-23
branch: release/009-s-daemon-b2
pr: "https://github.com/softwaresalt/agent-engram/pull/21"
status: awaiting-merge-approval
shipment: 009-S
feature: 029-F
---

# Session Memory — Ship 009-S B2 Daemon Reliability

## Items Completed (19/19)

All 19 backlog items for feature 029-F shipped on branch `release/009-s-daemon-b2`.

### WS-2 Doctor (029.004)
- 029.004.001-T: `HealthStatus`, `HealthCheck`, `HealthReport`, `SmokeResult` models
- 029.004.002-T: `get_health_report_for_daemon` 8-check implementation in `doctor.rs`
- 029.004.003-T: `run_smoke_test` IPC round-trip handshake
- 029.004.004-T: Doctor contract tests (4) + smoke integration tests (2)

### WS-4 Registry Validation (029.005)
- 029.005.001-T: `KNOWN_RENAMES` constant and harness
- 029.005.002-T: `validate_sources_strict` with path-traversal guard, optional-source skip, rename hints
- 029.005.003-T: Registry strict validation contract tests (6)

### WS-6 Background Scan + SLA (029.006)
- 029.006.001-T: `set_workspace` < 500ms SLA harness
- 029.006.002-T: `background_db_hydration` async task, `ScanProgress` model
- 029.006.003-T: `begin_scan_generation()` CancellationToken per generation
- 029.006.004-T: Background scan contract tests (5)
- 029.006.005-T: `hydration_ready` AtomicBool gating `_health` ready status

### WS-7 Integration (029.007)
- 029.007.001-T: `registry_missing_source_test.rs` (3 tests)
- 029.007.002-T: `multi_session_resume_test.rs` (2 tests)

### WS-8 Reliability Counters (029.008)
- 029.008.001-T: `ReliabilityCounters` harness
- 029.008.002-T: `ReliabilityCounters` implementation (4 AtomicU64) + `ReliabilitySnapshot`
- 029.008.003-T: Wire into `DaemonStatus.telemetry`
- 029.008.004-T: Reliability counters contract tests (6)

### WS-9 Socket Permissions (029.009)
- 029.009.001-T: Unix socket permissions (`/tmp/engram-{key}/` at 0o700, TOCTOU-safe)

## Branch State

- HEAD: `7e8a4ea` — `fix: reset hydration_ready on re-bind`
- All commits pushed to `origin/release/009-s-daemon-b2`
- 5 commits total: `eab23c3`, `32df870`, `7e8a4ea` (plus earlier wave commits)

## PR Status

- PR #21: https://github.com/softwaresalt/agent-engram/pull/21
- CI: ✅ Both backends green (`cozo-backend` 55s, `surreal-backend` 7m54s)
- Copilot Review: ✅ 17/17 threads replied to and resolved (12 fixed, 5 deferred)
- Operational Closure: ✅ READY WITH CONDITIONS

## Copilot Review Summary

### Fixed (12 issues, commits 32df870 + 7e8a4ea)
- Double `metrics::initialize` removed from `background_db_hydration`
- `hydrate_code_graph` error no longer silently discarded (logged via `tracing::warn!`)
- `get_health_report_for_daemon` error no longer silently swallowed (`unwrap_or_default` → explicit match + log)
- `check_cancel!()` added before expensive `sync_code_graph` re-index
- `derive_overall`: `Unknown` escalates to `Yellow` (conservative health roll-up)
- Socket dir: mode verified post-create (mode & 0o777 == 0o700); insecure pre-existing dir returns error
- 4 test files: stale "Red phase" comments removed
- `hydration_ready` now resets to `false` on each `set_workspace` call via `clear_hydration_ready()`

### Deferred (5 issues → backlog)
- `validate_sources_strict`: lexical `..` pre-check (defense-in-depth; current behavior is safe)
- Scan generation race: stale `set_scan_progress` overwrite after cancel
- Scan generation ID: prevent stale progress overwrites via monotonic ID
- `check_registry_validity`: surface `registry_validation_failures` counter
- `check_registry_validity` (re-review): same as above

## Key Decisions and Rationale

1. **`hydration_ready` reset before spawn**: Added `clear_hydration_ready()` call in `set_workspace`
   BEFORE `tokio::spawn(background_db_hydration)` to ensure the new background task's completion
   signal is not confused with the prior workspace's signal.

2. **Socket dir mode check**: Used `fs::metadata(&dir)?.permissions().mode() & 0o777 == 0o700`
   instead of uid check (uid check requires `unsafe` via `libc::getuid()` which is forbidden).
   Mode check is sufficient — owner is whoever created the dir.

3. **`Unknown` → `Yellow` in `derive_overall`**: Conservative health roll-up: if any check
   status is unknown, treat as degraded rather than healthy. This prevents false "Green" reporting.

4. **Fix-CI cycles exceeded**: 7 cycles (limit 5). Each was a distinct root cause. Disclosed to
   operator before merge gate.

5. **SurrealKV WAL corruption recovery** (from prior session): Per-path `OPEN_LOCKS` mutex prevents
   concurrent opens; 500ms sleep ensures background threads tear down before retry.

## Deferred Backlog Items (stashed)

1. Scan generation race: monotonic generation ID for `set_scan_progress`
2. `validate_sources_strict` lexical `..` pre-check
3. `check_registry_validity`: surface `registry_validation_failures` counter

All stashed in `.backlogit/queue/.stash.md` for Stage to process.

## Files Modified This Session

- `src/tools/lifecycle.rs` — background hydration, `clear_hydration_ready`, error logging, cancel check
- `src/server/state.rs` — `clear_hydration_ready()` method
- `src/tools/doctor.rs` — `derive_overall` Unknown → Yellow
- `src/daemon/ipc_server.rs` — socket dir mode verification post-create
- `tests/unit/socket_permissions_test.rs` — `.git/HEAD` init in `long_workspace()`
- `tests/contract/background_scan_test.rs` — stale comment removed
- `tests/integration/doctor_smoke_test.rs` — stale comment removed
- `tests/contract/registry_strict_validation_test.rs` — stale comment removed
- `tests/contract/reliability_counters_test.rs` — stale comment removed
- `docs/closure/2026-04-23-009-s-daemon-b2-runtime-verification.md` (new)
- `docs/closure/2026-04-23-009-s-daemon-b2-closure.md` (new)

## Next Steps

1. **Awaiting user merge approval** for PR #21
2. After merge: run Ship Step 6 (post-merge closure)
   - Archive shipment 009-S in backlogit
   - Invoke `operational-closure` in `post-merge` mode
   - Invoke `compound-refresh` for any stale learnings
   - Invoke `compact-context`
3. Live daemon smoke test (run within 48h of merge in local environment)
