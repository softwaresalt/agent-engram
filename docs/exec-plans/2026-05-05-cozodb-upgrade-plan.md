---
title: "CozoDB Upgrade & SQLITE_BUSY Root-Cause Resolution"
source: "docs/decisions/2026-05-05-cozodb-upgrade-deferred-deliberation.md"
status: "blocked"
blocked_on: "CozoDB >= 0.8 release on crates.io"
date: 2026-05-05
---

## Problem Frame

CozoDB 0.7.6 panics via `unwrap()` at `sqlite.rs:49` when encountering
`SQLITE_BUSY`. The workspace has accumulated multiple mitigation layers
(fd-lock, per-script retry, metrics, platform-specific test ignores). All
mitigations can be removed once CozoDB ships a release that handles
`SQLITE_BUSY` gracefully by returning an error instead of panicking.

**Affected code paths**:
- `src/db/cozo_backend/mod.rs` — fd-lock advisory lock in `connect_db`
- `src/db/cozo_queries.rs` — `run_script_busy_retry_mutable` loop + atomics
- `src/tools/read.rs` — `get_retry_metrics` MCP tool
- `tests/integration/smoke_test.rs` — `cfg_attr` ignore annotations
- `tests/integration/graph_vector_rehydration_test.rs` — `cfg_attr` ignore annotations

## Requirements Trace

| Requirement | Implementation Action |
|---|---|
| Upgrade CozoDB dependency | Bump `cozo` in `Cargo.toml` to >= 0.8 |
| Verify graceful SQLITE_BUSY handling | Add integration test that forces concurrent opens |
| Remove fd-lock workaround | Delete lock acquisition in `connect_db` |
| Remove retry loop | Delete `run_script_busy_retry_mutable` and atomics |
| Remove metrics tool | Delete `get_retry_metrics` tool, update TOOL_COUNT |
| Remove test ignore gates | Delete `cfg_attr` annotations from subprocess tests |
| Fix rehydration test design | Verify rehydrated-only state before auto-index |

## Implementation Units

### Unit 1: Dependency Upgrade & Verification

- **Files**: `Cargo.toml`, `Cargo.lock`
- **Action**: Bump `cozo` to >= 0.8. Run `cargo build` to verify API compatibility.
- **Test**: `cargo test` — all tests should pass without fd-lock or retry
- **Posture**: Migration-first (upgrade, then verify)
- **Blocked**: Yes — awaiting upstream release

### Unit 2: Remove SQLITE_BUSY Mitigations

- **Files**: `src/db/cozo_backend/mod.rs`, `src/db/cozo_queries.rs`, `src/services/metrics.rs`
- **Action**: Remove fd-lock in `connect_db`, remove `run_script_busy_retry_mutable`,
  remove `MUTABLE_RETRY_COUNT` / `MUTABLE_RETRY_EPOCH` atomics
- **Test**: Existing tests pass without mitigations
- **Posture**: Test-first (verify tests pass after removal)
- **Depends on**: Unit 1

### Unit 3: Remove Metrics Tool & Update Catalog

- **Files**: `src/tools/read.rs`, `src/shim/tools_catalog.rs`
- **Action**: Remove `get_retry_metrics` tool handler. Decrement `TOOL_COUNT`.
  Update architecture doc.
- **Test**: Contract tests for tool catalog pass with updated count
- **Posture**: Test-first
- **Depends on**: Unit 2

### Unit 4: Remove Test Ignore Gates & Fix Rehydration Test

- **Files**: `tests/integration/smoke_test.rs`,
  `tests/integration/graph_vector_rehydration_test.rs`
- **Action**: Remove `cfg_attr(any(target_os = "windows", target_os = "linux"), ignore)`
  annotations. Redesign rehydration test to verify rehydrated-only state before
  auto-index completes (use a flag or timing gate).
- **Test**: Both tests pass on all platforms
- **Posture**: Characterization-first (run existing tests to baseline)
- **Depends on**: Unit 1

## Dependency Graph

```text
Unit 1 (upgrade) ──┬──→ Unit 2 (remove mitigations) ──→ Unit 3 (remove tool)
                   └──→ Unit 4 (remove ignores + fix test)
```

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Remove retry metrics tool entirely | The tool exists only to observe SQLITE_BUSY retries; once the root cause is fixed, the tool has no purpose |
| Remove fd-lock entirely (not just shrink scope) | CozoDB 0.8 should handle concurrent opens gracefully; fd-lock was only needed because of the upstream `unwrap()` |
| Fix rehydration test design in same shipment | The test's current design (wait-for-auto-index) masks whether rehydration actually works; this is the right time to fix it |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| CozoDB 0.8 API breaks | Review changelog; cozo has a stable public API. Pin exact version. |
| CozoDB 0.8 introduces new bugs | Run full test suite; maintain ability to revert to 0.7.6 + mitigations |
| Some mitigations still needed for edge cases | Keep retry logic behind a feature flag initially if unsure |

## Plan Hardening Signals

- [ ] Public API, schema, or contract change — **YES** (MCP tool removal changes the tool surface)
- [ ] Security, auth, permission — No
- [ ] Migration, destructive data/config — No
- [ ] External integration, external dependency — **YES** (upstream CozoDB crate)
- [ ] High runtime, rollout, or rollback risk — Low (can revert dep bump)

**Requires plan hardening: no** — The signals present (tool removal, dep upgrade) are
mechanical and well-understood. The dependency upgrade is the primary gate; once CozoDB 0.8
works, the removal steps are straightforward. No hardening needed beyond standard review.

## Runtime Verification and Closure

- **Runtime surface changed**: MCP tool surface (removal of `get_retry_metrics`)
- **Verification**: Run `cargo test` on all platforms without ignore gates. Verify daemon
  subprocess tests pass on Windows and Linux. Confirm tool catalog count matches.
- **Closure**: Update architecture doc tool count. Archive compound entries that reference
  the workaround as superseded.
