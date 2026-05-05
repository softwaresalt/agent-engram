---
title: "CozoDB Upgrade & Daemon Subprocess Reliability — Deferred Feature"
description: "Consolidation of 3 stash entries into a single blocked covering feature awaiting upstream CozoDB >= 0.8 release"
topic: "Upgrade CozoDB dependency and remove SQLITE_BUSY workarounds"
depth: "lightweight"
decision_status: "decided"
promoted_to: "both"
source_stash_ids:
  - "1092D3D6"
  - "100EACD8"
  - "D13A3452"
linked_artifacts:
  - "docs/decisions/2026-05-01-cozodb-concurrency-hardening-deliberation.md"
  - "docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md"
  - "docs/compound/test-failures/cfg-attr-platform-ignore-vs-unconditional-2026-05-04.md"
  - "docs/compound/data-plane/sqlite-busy-retry-granularity-2026-05-03.md"
tags:
  - "cozo"
  - "sqlite"
  - "upgrade"
  - "blocked-upstream"
  - "U015-FLK1"
  - "deferred"
---

## Problem Frame

Three stash entries describe overlapping work that all depend on the same
upstream precondition: a CozoDB release >= 0.8 that handles `SQLITE_BUSY`
gracefully instead of panicking via `unwrap()`.

| Stash ID | Priority | Summary |
|---|---|---|
| `1092D3D6` | low | Upgrade cozo 0.7→0.8+, remove fd-lock workaround in `connect_db` |
| `100EACD8` | medium | Daemon subprocess spawn timeout on Windows — CozoDB panics at `sqlite.rs:49` |
| `D13A3452` | medium | Upgrade CozoDB ≥0.8, remove `cfg_attr` ignore gates, fix rehydration test design |

These three entries are near-duplicates of the same upstream dependency gap.
`1092D3D6` and `D13A3452` describe the same action (upgrade + remove workarounds).
`100EACD8` describes the symptom that the upgrade would resolve.

## Prior Art

The workspace has already shipped three phases of daemon reliability work:

- **037-F** (015-S): fd-lock scope extension covering `DbInstance::new` + `run_schema_bootstrap`
- **038-F** (021-S): Per-`run_script` SQLITE_BUSY retry with mutable back-off
- **039-F** (022-S): Platform-specific `cfg_attr` ignore annotations, `continue-on-error` removal
- **040-F** (023-S): SQLITE_BUSY retry metrics MCP tool for observability

All of these are mitigations within the current CozoDB 0.7.6 constraint. The
stash entries represent the *final resolution* — upgrading the dependency to
eliminate the root cause entirely.

## Decision

**Consolidate all three stash entries into a single covering feature** titled:

> **CozoDB Upgrade & SQLITE_BUSY Root-Cause Resolution**

This feature is **blocked** on the upstream release of CozoDB >= 0.8 on crates.io.
As of 2026-05-05, the latest stable release is `cozo 0.7.6`.

### Scope (when unblocked)

1. Bump `cozo` dependency to >= 0.8 in `Cargo.toml`
2. Verify CozoDB 0.8 handles `SQLITE_BUSY` gracefully (returns `Err` instead of panicking)
3. Remove fd-lock advisory lock in `src/db/cozo_backend/mod.rs` (`connect_db`)
4. Remove per-`run_script` SQLITE_BUSY retry loop in `src/db/cozo_queries.rs`
5. Remove `MUTABLE_RETRY_COUNT` / `MUTABLE_RETRY_EPOCH` atomics and `get_mutable_script_retry_metrics` tool
6. Remove `cfg_attr(any(target_os = "windows", target_os = "linux"), ignore)` from subprocess tests
7. Fix rehydration test design — verify rehydrated-only state before auto-index completes
8. Run full test suite on all platforms without ignore gates

### Watch Trigger

**Condition**: `cozo >= 0.8` published on crates.io with confirmed `SQLITE_BUSY`
error propagation (no `unwrap()` in SQLite storage layer).

**Monitor**: Check crates.io quarterly or subscribe to upstream release notifications.

**Action when triggered**: Move feature from `blocked` → `queued`, unblock all
child tasks, and notify Stage for shipment assembly.

### Risk Assessment

- **Risk level**: Low (mechanical dependency upgrade + removal of workaround code)
- **Blast radius**: Medium (touches `db/cozo_backend`, `db/cozo_queries`, test annotations, metrics tool)
- **Rollback**: Revert the dependency bump if CozoDB 0.8 introduces regressions

## Rejected Alternatives

- **Ship now without CozoDB 0.8**: Impossible — the workarounds exist because CozoDB 0.7.x panics.
  Removing them without the upstream fix would cause test failures and daemon crashes.
- **Fork CozoDB**: Over-engineered — the fix is a single `unwrap()` → `?` change in upstream.
  Wait for the official release rather than maintaining a fork.
