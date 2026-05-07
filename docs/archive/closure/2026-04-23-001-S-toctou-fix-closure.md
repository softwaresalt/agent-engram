# Operational Closure — 001-S: Atomic Policy TOCTOU Fix

**Mode**: post-merge  
**Date**: 2026-04-23  
**PR**: [#22 — feat: atomic dispatch snapshot eliminates policy TOCTOU window](https://github.com/softwaresalt/agent-engram/pull/22)  
**Merge commit**: `c0068d0`  
**Branch**: `018-atomic-workspace-config-snapshot-toctou`  
**Shipment**: 001-S (archived)  
**Tasks**: 024.001-T, 024.002-T (archived)  

---

## Change Summary

Eliminates the policy TOCTOU window in `tools::dispatch`. Previously the function acquired the `active_workspace` read lock, released it, then acquired `workspace_config` separately — a concurrent `set_workspace` or `set_workspace_config` call between those two reads could produce a mismatched workspace/config pair (workspace from T1, policy from T2). The fix holds both read guards simultaneously in `snapshot_dispatch_context()` before cloning, producing a point-in-time `DispatchSnapshot`.

Also adds metrics recording for policy-denied calls (`outcome="denied"`) so denied calls appear in `get_evaluation_report` output.

---

## Invariants to Preserve

1. `snapshot_dispatch_context()` must always return a snapshot where `workspace` and `config` are from the same logical point in time (no cross-snapshot mixing).
2. Policy-denied calls must record a `UsageEvent` with `outcome="denied"` before returning the error.
3. When no workspace is bound, `snapshot_dispatch_context()` returns `None` and policy evaluation is bypassed.
4. When a workspace is bound but no config is loaded, `WorkspaceConfig::default()` is used (policy disabled).
5. All 7 C018 contract tests (`contract_atomic_policy_snapshot`) must pass in both surreal-backend and cozo-backend CI environments.

---

## Pre-Deploy Audit

| Check | Status | Notes |
|-------|--------|-------|
| No unsafe code | ✅ | `#![forbid(unsafe_code)]` enforced |
| No `unwrap()`/`expect()` in changed paths | ✅ | All use `?` or `.unwrap_or_default()` |
| Clippy pedantic clean | ✅ | Both backends, CI run 24846996785 |
| Full test suite | ✅ | 92+ unit/contract tests pass |
| Lock ordering consistent | ✅ | `active_workspace` always acquired before `workspace_config` in snapshot path |
| No behavior change for non-policy paths | ✅ | Snapshot is read-only; dispatch flow unchanged when policy disabled |
| Backlog IDs deconflicted | ✅ | 007-S restored; 029-F archived; new items use 010-S/011-S/032-F |

---

## Deployment / Rollout Path

**Merge-only** — this is a library/daemon change with no migration, schema change, or external API change. No deployment step beyond merging to main and rebuilding the daemon binary.

- No database schema changes
- No IPC protocol changes  
- No CLI interface changes
- No configuration format changes

---

## Affected Runtime Surfaces

| Surface | Change | Risk |
|---------|--------|------|
| `tools::dispatch` hot path | Now acquires two read locks instead of one per call | Negligible — read locks do not block each other; latency impact unmeasurable |
| `get_evaluation_report` | Now includes denied calls | Additive — existing consumers see more data, not less |
| Policy enforcement correctness | TOCTOU window closed | Improves correctness; no behavioral regression for correctly-behaving callers |

---

## Healthy Signals

- `get_evaluation_report` returns denied events for tools called without a matching policy rule
- Daemon starts and binds workspace successfully in post-merge smoke test
- `list_symbols`, `unified_search`, and other tools respond correctly when policy is disabled
- `snapshot_dispatch_context` returns `None` before workspace is bound (validated by C018-02)

---

## Failure Signals

- Any tool call that previously succeeded now returns a policy-denied error (would indicate policy config was unintentionally set to deny-all)
- `get_evaluation_report` returns no events at all (would indicate metrics recording broke)
- Daemon hangs on dispatch (would indicate unexpected lock contention — extremely unlikely with read-only lock pattern)

---

## Monitoring Plan

| What to watch | Where | Threshold |
|---------------|-------|-----------|
| Policy denied count | `get_evaluation_report` → `outcome="denied"` events | Any denied events on a workspace with policy disabled = bug |
| Dispatch latency | Structured trace logs (if tracing enabled) | >10ms average on cached workspace = investigate |
| Test suite regression | CI on `main` | Any failure in `contract_atomic_policy_snapshot` = rollback |

This is an internal correctness fix with no user-visible surface change. Monitoring is lightweight.

---

## Rollback Trigger

**Trigger**: Any new policy enforcement regression (tool calls that were working are now denied without a policy change) OR test suite failure in `contract_atomic_policy_snapshot` on `main` after merge.

---

## Rollback Procedure

1. `git revert` the merge commit on `main`
2. Rebuild daemon binary
3. No migration or data rollback required

---

## Validation Window

**24 hours** after merge. If no regression in CI on `main` and no operator-reported denied-call anomaly, the change is considered absorbed.

**Owner**: @softwaresalt (repository owner)

---

## Risky Action Record

| Action | Risk | Approval | Result |
|--------|------|----------|--------|
| Change lock acquisition order in `snapshot_dispatch_context` | moderate — hot dispatch path | Validated by 7-test contract suite + Copilot review acceptance | applied (commit 1c51848) |
| Add `UsageEvent` recording on denied path | low — additive metrics | — | applied (commit 0849503) |

---

## Readiness Status

**COMPLETE** — Merged to `main` at `c0068d0` (2026-04-23). CI green on both backends, Copilot review comments addressed and threads resolved, all contract tests pass. Shipment 001-S archived; 024-F, 024.001-T, 024.002-T archived.

**Post-merge**: Validation window active (24 hours from merge). No regressions observed at merge time. Monitoring per plan below.

---

## Follow-Up Items

None identified from this change. Post-merge, the next shipments in queue are:

- **010-S** — Backlogit Ship-Shipment Integrity (032-F, P1)
- **011-S** — Daemon Reliability Program (028-F, 001-F, 003-F)
- **007-S** — Code Graph Tier-2 Completion (030-F)

These are deferred to Stage → Ship pipeline ordering and are not blockers for this merge.
