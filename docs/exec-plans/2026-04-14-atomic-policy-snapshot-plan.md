---
title: Atomic Policy Snapshot — Eliminate TOCTOU in Dispatch
source: .backlogit/queue/024-F.md
feature_id: 024-F
created: 2026-04-14
status: draft
---

# Atomic Policy Snapshot — Eliminate TOCTOU in Dispatch

## Problem Frame

The `dispatch()` function in `src/tools/mod.rs` reads `policy_config()` under
a read lock (line 127), releases the lock, then executes the tool. A
concurrent `set_workspace_config()` call between the policy check and tool
execution can change policy while an already-approved call proceeds. This is a
classic time-of-check-to-time-of-use (TOCTOU) race documented at
`src/tools/mod.rs:116-120` and referencing TASK-018 (024-F).

A secondary defect: policy-denied calls return early (line 129) before the
metrics recording block (line 192), so denied calls are invisible to
`get_evaluation_report` and `get_branch_metrics`.

## Requirements Trace

| Source AC | Implementation Action |
|---|---|
| AC #1: Workspace binding and config are snapshotted atomically at dispatch entry | Add `DispatchSnapshot` struct and `snapshot_dispatch_context()` method to `AppState` |
| AC #2: A concurrent `set_workspace_config` call cannot change policy mid-dispatch | Use the cloned snapshot throughout `dispatch()` instead of re-reading state |
| AC #3: Policy-denied calls are recorded in metrics with `outcome=denied` | Move metrics recording before the early return on policy denial |

## Implementation Units

### Unit 1: DispatchSnapshot struct and snapshot method

**Scope**: Add the snapshot API to `AppState`.

**Files**: `src/server/state.rs`

**Changes**:

1. Add `DispatchSnapshot` struct:

   ```rust
   #[derive(Clone, Debug)]
   pub struct DispatchSnapshot {
       pub workspace: WorkspaceSnapshot,
       pub config: WorkspaceConfig,
   }
   ```

2. Add `snapshot_dispatch_context()` method to `AppState`:

   ```rust
   pub async fn snapshot_dispatch_context(&self) -> Option<DispatchSnapshot> {
       let workspace_guard = self.active_workspace.read().await;
       let config_guard = self.workspace_config.read().await;
       workspace_guard.as_ref().map(|ws| DispatchSnapshot {
           workspace: ws.clone(),
           config: config_guard.clone().unwrap_or_default(),
       })
   }
   ```

   Both read locks are held simultaneously while cloning. No write can
   interleave between the workspace and config reads while both guards are
   alive. The guards drop together after the `map` closure completes.

**Tests verified by**: C018-01 through C018-05 in
`tests/contract/atomic_policy_snapshot_test.rs`

**Execution posture**: test-first (harness already exists in red phase)

**Granularity check**: 1 file, 2 items (struct + method), 5 test scenarios ✓

### Unit 2: Wire snapshot into dispatch and record denied metrics

**Scope**: Replace the separate `policy_config()` + `snapshot_workspace()`
calls in `dispatch()` with the atomic snapshot, and add metrics recording for
policy-denied calls.

**Files**: `src/tools/mod.rs`, `Cargo.toml`

**Changes**:

1. At dispatch entry (after `agent_role` extraction), call
   `state.snapshot_dispatch_context().await` and store the result.

2. Replace the `policy_config()` check (lines 127-130) with policy evaluation
   against the snapshot's config:

   ```rust
   let dispatch_ctx = state.snapshot_dispatch_context().await;
   if let Some(ref ctx) = dispatch_ctx {
       if ctx.config.policy.enabled {
           if let Err(e) = policy::evaluate(
               &ctx.config.policy,
               agent_role.as_deref(),
               method,
           ) {
               // Record denied metric before returning
               if should_record_metrics(method) {
                   metrics::record(UsageEvent {
                       tool_name: method.to_owned(),
                       timestamp: chrono::Utc::now().to_rfc3339(),
                       response_bytes: 0,
                       estimated_tokens: 0,
                       symbols_returned: 0,
                       results_returned: 0,
                       branch: ctx.workspace.branch.clone(),
                       connection_id: None,
                       agent_role: agent_role.clone(),
                       outcome: "denied".to_string(),
                   });
               }
               return Err(EngramError::from(e));
           }
       }
   }
   ```

3. Replace the `snapshot_workspace()` call in the metrics block (line 193)
   with the already-captured snapshot:

   ```rust
   if should_record_metrics(method) {
       if let Some(ref ctx) = dispatch_ctx {
           // use ctx.workspace instead of state.snapshot_workspace().await
       }
   }
   ```

4. Remove the TOCTOU design-constraint comment block (lines 113-126) and
   replace with a reference to the atomic snapshot.

5. Add `[[test]]` registration in `Cargo.toml`:

   ```toml
   [[test]]
   name = "atomic_policy_snapshot_test"
   path = "tests/contract/atomic_policy_snapshot_test.rs"
   ```

**Tests verified by**: C018-06 and C018-07 in
`tests/contract/atomic_policy_snapshot_test.rs`

**Execution posture**: test-first (harness already exists in red phase)

**Granularity check**: 2 files, 3 functions modified, 2 test scenarios ✓

## Dependency Graph

```text
Unit 1 (DispatchSnapshot + snapshot_dispatch_context)
  └── Unit 2 (Wire into dispatch + denied metrics + Cargo.toml)
```

Unit 2 depends on Unit 1. Sequential execution required.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Keep both `RwLock` fields in `AppState` rather than combining | Combining workspace + config into one `RwLock` would require changing ~10 accessor methods and all callers. The dual-read-lock snapshot holds both guards simultaneously, preventing interleaved writes. The atomicity gap (between the two `.await` read acquisitions) is a single executor yield point — acceptable for v1. |
| Use `WorkspaceConfig::default()` when config is `None` | Matches existing behavior: `policy_config()` returns `None` when config is unset, which means policy is disabled. `DispatchSnapshot` always carries a config (defaulting to disabled policy). |
| Record denied metrics inline before early return | Avoids restructuring the entire dispatch flow. The denied event is recorded at the denial site, not in the shared metrics block at the end. |

## Risks and Caveats

| Risk | Likelihood | Mitigation |
|---|---|---|
| Dual-read-lock still has a theoretical interleave window | Low — single yield point between two read acquisitions | Acceptable for v1. Document as known limitation. Full fix would require single-`RwLock` refactor (future chore). |
| Existing callers of `policy_config()` may diverge from dispatch snapshot | Low — only `dispatch()` uses policy for gating | Keep `policy_config()` API for non-dispatch callers. |
| Test harness uses `expect()` in setup helpers | Acceptable — test code, not library code | No change needed; `expect()` is permitted in test helpers per project conventions. |

## Plan Hardening Signals

| Signal | Present | Justification |
|---|---|---|
| Public API, schema, or contract change | No | `DispatchSnapshot` and `snapshot_dispatch_context()` are new `pub(crate)` additions. No MCP protocol change. |
| Security, auth, permission, or compliance-sensitive behavior | Yes | Policy enforcement is security-adjacent. The change tightens the enforcement window — strictly an improvement. |
| Migration, backfill, destructive data/config action | No | No data migration. Pure code change. |
| External integration, operator checkpoint | No | No external dependencies. |
| High runtime, rollout, or rollback risk | No | Internal refactor. Rollback is a single revert. |

**Requires plan hardening: no**

The security signal is present but the change strictly narrows the TOCTOU
window. The risk direction is toward safety, not away from it. No plan
hardening needed.

## Runtime Verification and Closure

| Unit | Runtime Surface Changed | Verification | Closure |
|---|---|---|---|
| Unit 1 | None (internal API) | Contract tests C018-01 through C018-05 | N/A |
| Unit 2 | Dispatch behavior (policy enforcement timing, denied metrics) | Contract tests C018-06, C018-07; existing policy integration tests | Verify `get_branch_metrics` includes denied calls after deployment |

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | ✓ No unsafe code. All errors propagated via `Result<T, EngramError>`. |
| II. Test-First Development | ✓ Harness exists in red phase. Implementation follows. |
| III. Workspace Isolation | ✓ No filesystem changes. |
| IV. CLI Containment | ✓ No external file operations. |
| V. Structured Observability | ✓ Denied calls now produce metrics events. |
| VII. Destructive Approval | N/A — no destructive operations. |

## Plan Review

**Gate decision: PASS**

**Reviewers:** Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor,
Learnings Researcher

**Plan hardening required:** No — correctly assessed. Security signal present but
change direction is toward safety (narrowing TOCTOU window).

### Findings

#### CONST-01 (P3 — Advisory)

**Description:** The plan correctly adds denied-call metrics recording, which
addresses the observability gap (CR-004 from the prior 022-F policy engine
review). Positive constitution alignment with Principle V (Structured
Observability).

**Recommendation:** No action needed. This is an affirmation.

#### RUST-01 (P3 — Advisory)

**Description:** `DispatchSnapshot` fields are declared `pub` in the plan. The
project convention is `pub(crate)` unless public API. However, `WorkspaceSnapshot`
and `AppState` are both `pub struct` (state.rs lines 31, 82), and the contract
tests access `DispatchSnapshot` fields directly from the external test crate.
`pub` is correct here.

**Recommendation:** Keep `pub` on `DispatchSnapshot` and its fields. Matches
existing state.rs visibility pattern.

#### RUST-02 (P3 — Advisory)

**Description:** The dual-read-lock `snapshot_dispatch_context()` has a
theoretical interleave window between the two `.await` points for acquiring
`active_workspace` and `workspace_config` read locks. A concurrent writer could
modify config between the two acquisitions.

**Recommendation:** Acceptable for v1. The plan documents this as a known
limitation with a future single-`RwLock` refactor path. The window is a single
executor yield point — negligible in practice.

#### SCOPE-01 (P3 — Advisory)

**Description:** All changes map precisely to the 3 acceptance criteria. No scope
creep, no YAGNI additions. The dual-lock approach is appropriately conservative
vs. a single-`RwLock` refactor that would touch ~10 methods and all callers.
Granularity (2 units) satisfies the 2-hour rule.

**Recommendation:** No action needed.

#### LEARN-01 (P3 — Advisory)

**Description:** The existing test harness already follows the TempDir lifetime
pattern documented in `docs/compound/test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md`
(returns `(Arc<AppState>, TempDir)` tuple). The `clippy::derivable_impls`
learning is not directly applicable. No relevant learnings are being ignored.

**Recommendation:** No action needed.

### Summary

| Severity | Count |
|---|---|
| P0 | 0 |
| P1 | 0 |
| P2 | 0 |
| P3 | 5 |

All findings are advisory (P3). The plan is well-scoped, constitutionally
compliant, and leverages prior learnings. Proceed to harvest.

<!-- plan-review-attempt: 1 -->