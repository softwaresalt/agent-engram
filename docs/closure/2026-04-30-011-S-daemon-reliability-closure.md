# Operational Closure — Shipment 011-S (Pre-Merge)

**Date**: 2026-04-30
**Shipment**: 011-S — Daemon Reliability Program
**Features**: 001-F (concurrent agent sessions), 003-F (code-graph co-location — closed as resolved)
**PR**: https://github.com/softwaresalt/agent-engram/pull/51
**Branch**: `feat/011-S-daemon-reliability`
**Base**: `main`
**Status**: ✅ READY — awaiting user merge approval

## Release Readiness

| Check | Status |
|-------|--------|
| CI/build (surreal-backend) | ✅ pass (7m45s) |
| CI/build (cozo-backend) | ✅ pass (55s) |
| Copilot review comments | ✅ 11/11 replied to and resolved |
| cargo fmt --all | ✅ clean |
| cargo clippy -D warnings -D clippy::pedantic | ✅ clean |
| cargo test | ✅ 6 tests pass (concurrent_sessions) |
| Rubber-duck review gate | ✅ all P1/P2/P3 findings addressed |
| Runtime surfaces modified | ✅ none — tests + docs + backlog only |

## Scope

Characterization-and-documentation shipment — no production Rust source code was modified.

The deliverables are:
- 4 new integration tests (`tests/integration/concurrent_sessions_test.rs`) characterizing concurrent IPC behavior
- A new **Concurrency Model** section in `docs/architecture.md` documenting the per-connection model and lock hierarchy
- 3 schema version fixes (`3.0.0` → `4.0.0`) in `docs/architecture.md`
- `Cargo.toml` test target registration for the new test file
- Backlog archival prep: 001.009-T, 001.010-T, 003.001-T, 003-F, 001-F all set to `done`
- 011-S manifest updated to reflect shipped state

## Delivered Changes

### `tests/integration/concurrent_sessions_test.rs` (new)

Four concurrent IPC session tests:

| Test | Scenario | Assertion |
|------|----------|-----------|
| s_cs1 | 3 concurrent `_health` calls | No response corruption; each response id matches request id |
| s_cs2 | 3 concurrent `get_daemon_status` calls | Consistent daemon state; each response id matches |
| s_cs3 | `set_workspace` + 3 concurrent status reads via Barrier | Lifecycle serialization holds during concurrent reads |
| s_cs4 | 2 concurrent `index_workspace` calls | Either serializes (error 7003 in `error.data["engram_code"]`) or both succeed on a fast workspace |

Uses `tokio::sync::Barrier` for deterministic simultaneous dispatch. No `sleep`-based timing.

### `docs/architecture.md` — Concurrency Model section (appended)

Documents:
- Per-connection `tokio::spawn` accept model
- `AppState` lock hierarchy: `RwLock<WorkspaceState>` (read-mostly), `AtomicBool indexing_in_progress` (write-exclusive)
- `hydration_ready` lifecycle: cleared on `set_workspace` start, re-set after hydration completes
- Internal command bypass: `_health` and `_shutdown` handled directly in `ipc_server.rs`, all other methods through `tools::dispatch()`
- Schema version `4.0.0` (was `3.0.0` in 3 places — corrected)

### `Cargo.toml`

Added `[[test]]` entry required for cargo to discover `integration_concurrent_sessions` target.

### Backlog

All 5 manifest items set to `done`; 001-F feature set to `done`; 003-F closed as resolved with rationale.

## Invariants to Preserve

1. `indexing_in_progress` AtomicBool must guard `index_workspace` calls — only one indexing pass runs at a time
2. `_health` and `_shutdown` must remain direct-dispatch (bypass `tools::dispatch()`) for daemon liveness
3. `hydration_ready` must be cleared at the start of `set_workspace` and re-set after hydration completes on each call
4. Per-connection tokio task model must not be changed to a shared connection pool without updating the concurrency documentation

## Pre-Deploy Audit

N/A — no deployment, no migration, no runtime surface change. This is a test + documentation shipment only.

## Post-Deploy Checks

Not applicable for this shipment. The integration tests themselves ARE the post-deploy smoke checks for the daemon's concurrent session handling:

```bash
cargo test --test integration_concurrent_sessions
```

All 4 scenarios pass in CI and locally.

## Risky Action Record

| Action | Risk | Approval | Result |
|--------|------|----------|--------|
| None | — | — | — |

No destructive operations were performed. All changes are additive (new test file, appended docs section, updated backlog state).

## Monitoring Plan

No runtime monitoring required — no production code changed.

If a future PR modifies the IPC accept loop or `AppState` locking:
- Run `cargo test --test integration_concurrent_sessions` as a regression gate
- Watch for flaky test results in s_cs4 (known timing sensitivity — see Known Limitations below)

## Healthy Signals

- All 4 concurrent session tests pass (`cargo test --test integration_concurrent_sessions`)
- Daemon health check responds under concurrent load
- `get_daemon_status` returns consistent state across concurrent calls

## Failure Signals

- s_cs1/s_cs2: response id mismatch → response cross-contamination in IPC layer
- s_cs3: `workspace_status` returns wrong state after `set_workspace` → lifecycle race
- s_cs4: both calls fail (neither 7003 nor success) → indexing serialization broken

## Rollback Trigger

No production signal. If the merged tests cause persistent CI flakiness, revert with:

```bash
git revert {merge_sha}
```

## Rollback Procedure

`git revert {merge_sha}` removes the test file and documentation section without affecting production code.

## Validation Window

N/A — static test + documentation change. Watch CI on subsequent PRs to confirm concurrent session tests remain stable.

## Owner

softwaresalt

## Known Limitations

**s_cs4 timing sensitivity**: On a near-empty `TempDir` workspace, `index_workspace` may complete before the second concurrent call arrives, causing both calls to succeed (neither returns 7003). The test correctly accepts this outcome via an early-return guard. To deterministically exercise the 7003 serialization path, the test workspace would need enough indexable content to force indexing to overlap with the second call. This is recorded as a follow-up improvement item.

## Source Artifact Cleanup

### 001-F

- `custom_fields.source_stash_id`: absent (originated from `backlog_md_source_path: .backlog/drafts/draft-001 - Can-the-shim-handle-multiple-concurrent-agent-sessions.md`)
- `custom_fields.source_deliberation_id`: absent
- Deliberation reference: `docs/decisions/2026-04-30-011-S-daemon-reliability-deliberation.md`
- Plan reference: `docs/exec-plans/2026-04-30-011-S-concurrent-sessions-plan.md`

### 003-F

- `custom_fields.source_stash_id`: absent (originated from `backlog_md_source_path: .backlog/drafts/draft-003 - Bring-the-code-graph-into-the-db-branch-version.md`)
- `custom_fields.source_deliberation_id`: absent
- Closed as resolved — no implementation work; architectural rationale documented in `003-F.md` Resolution section

Note: `backlogit_stash_remove` and `backlogit_archive_item` are not in the installed registry. Traceability recorded here for manual follow-up.

## Follow-Up Items

| # | Summary | Priority |
|---|---------|----------|
| 1 | Improve s_cs4 to deterministically trigger IndexInProgress (7003) by adding enough indexable content to the temp workspace | low |

## Artifacts

| Artifact | Path |
|----------|------|
| Closure record (this file) | `docs/closure/2026-04-30-011-S-daemon-reliability-closure.md` |
| Deliberation | `docs/decisions/2026-04-30-011-S-daemon-reliability-deliberation.md` |
| Implementation plan | `docs/exec-plans/2026-04-30-011-S-concurrent-sessions-plan.md` |
| Session memory | `docs/memory/2026-04-30/` |
| PR | https://github.com/softwaresalt/agent-engram/pull/51 |
