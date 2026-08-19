---
title: "Shipment 118-S Worktree MCP Startup Operational Closure"
date: 2026-08-19
artifact_type: operational-closure
shipment_id: 118-S
feature_id: 122-F
task_id: 122.010-T
pr: 344
merge_sha: 08676d341d94fd97b9d7ea3ea30562e63c5c9bba
readiness: READY WITH CONDITIONS
---

## Readiness

**READY WITH CONDITIONS** for shipment reconciliation and archival. The code
is merged, local and CI gates are green, exact-HEAD Copilot review completed,
and post-merge runtime verification returned **PASS WITH FOLLOW-UP**.

The remaining condition is a 24-hour observation window after the next binary
release. No schema migration, data migration, feature flag, or immediate
deployment step is required.

Runtime evidence:
`docs/closure/2026-08-19-118-s-worktree-mcp-startup-runtime-verification.md`.

## Change Summary

Shipment `118-S`, feature `122-F`, sourced from `B30EA752`, delivered:

* secure native Git worktree metadata and active-branch resolution, including
  relative worktree pointers and reftable repositories
* bounded shim startup failures and concurrent daemon-winner publication
* bounded fail-open Copilot launcher pre-warm and exact-process cleanup
* Ship guidance for bounded worktree diagnostics
* Windows CI coverage for the launcher contract

PR `#344` merged through two-parent merge commit
`08676d341d94fd97b9d7ea3ea30562e63c5c9bba`.

## Invariants to Preserve

1. Valid ordinary repositories and native linked worktrees are admitted.
2. Malformed, linked, or structurally unowned metadata fails closed.
3. Worktree branch identity never falls back to the primary checkout.
4. MCP stdout contains protocol frames only.
5. MCP initialization, tools listing, and read-only status remain usable.
6. One workspace has at most one reusable daemon.
7. EOF and shutdown leave no test-owned endpoint or PID.
8. Launcher pre-warm cannot indefinitely delay Copilot.
9. Launcher cleanup never uses name-wide or daemon-tree termination.

## Pre-Deploy Audit

| Check | Result |
|---|---|
| Feature flag required | No |
| Schema or data migration | None |
| Backward compatibility | Existing ordinary Git repositories remain covered |
| Cross-service dependency | None |
| Rollback documented | Yes |
| Monitoring plan complete | Yes |
| CI at exact feature HEAD | Ubuntu and Windows jobs green |
| Review at exact feature HEAD | Copilot complete; no unresolved threads |
| Merge strategy | Merge commit; squash and rebase disabled |

## Rollout Path

The repository change is merge-complete. Runtime distribution occurs through
the next normal Engram binary release. Do not introduce a special migration or
out-of-band daemon cleanup step. Existing daemons transition through normal
protocol and release lifecycle behavior.

## Post-Release Checks

1. Start Copilot from a native linked worktree with the normal launcher.
2. Confirm Copilot invocation begins within the launcher budget.
3. Complete MCP initialize and `tools/list`.
4. Run read-only `get_workspace_status`.
5. Confirm the reported path and branch are the linked worktree identity.
6. Repeat the read-only call and confirm daemon reuse.
7. Close stdin or request protocol shutdown.
8. Confirm endpoint and exact PID disappear within five seconds.

## Monitoring Plan

There is no external dashboard for this local-first binary. The operator uses
structured launcher warnings, Engram startup logs, MCP error responses, and
exact process/PID inspection.

| SLI | Baseline | Alert threshold | Observation |
|---|---|---|---|
| Valid-worktree `NotGitRoot` errors | 0 | Any occurrence | Startup logs and CLI stderr |
| MCP initialize latency | Contract under 20s | 20s or more | Client timing and MCP logs |
| Early invalid/child-exit failure | Contract under 15s | 15s or more | Shim stderr and elapsed time |
| Launcher pre-warm | Production budget 15s | More than 15s | Launcher warning and elapsed time |
| Copilot launch delay | Bounded fail-open | More than 30s | Operator stopwatch/session start |
| EOF/PID cleanup | Contract within 5s | Residual endpoint/PID after 5s | Exact endpoint and PID query |
| Duplicate workspace daemon | 0 | More than 1 | Exact workspace PID metadata |

Owner: repository maintainer/operator.

Observation window: merge-time runtime checks completed on 2026-08-19; monitor
for 24 hours after the next binary release.

## Failure Signals and Rollback Trigger

Rollback or halt release promotion when any of these occurs:

* a valid native linked worktree returns `NotGitRoot`
* MCP initialize reaches or exceeds 20 seconds
* Copilot launch is delayed more than 30 seconds
* an endpoint or exact daemon PID remains five seconds after shutdown
* more than one daemon owns the same workspace
* branch identity reports the primary checkout instead of the linked worktree

## Rollback Procedure

1. Stop release promotion or pin the previous Engram release.
2. Revert merge commit `08676d341d94fd97b9d7ea3ea30562e63c5c9bba`
   through a new reviewed PR using a merge commit.
3. Run the same MCP lifecycle, direct-worktree, and launcher contracts.
4. Do not force-push, rewrite history, or kill processes by name.

## Risky Action Record

| Proposed action | Action risk | Approval path | Action result |
|---|---|---|---|
| Trust native external Git administration metadata after validation | High | Explicit operator authorization | Applied; security and adversarial review completed |
| Observe exact spawned daemon during readiness | High | Explicit operator authorization | Applied; concurrency contracts pass |
| Terminate over-budget launcher foreground process | Moderate | Explicit operator authorization | Applied to exact process with bounded confirmation |
| Merge PR `#344` | High | Explicit full-cycle authorization | Applied through policy-compliant merge commit |

## Residual Work

The following non-blocking work is preserved through Stage-owned stash:

* `568B257C` — retained capability-rooted metadata handles
* `22DF3329` — serve static MCP initialization before daemon readiness
* `C2413934` — restore canonical development-test coverage
* `DE460A88` — independent agent-visible catalog contract

The direct diagnostic observation that IPC `--timeout` does not bound direct
indexing is recorded in runtime verification. The production launcher supplies
the required external shared deadline; this observation is not treated as an
unverified product regression.

## Closure Decision

Operational closure is complete enough to reconcile and archive shipment
`118-S`. The post-release observation window remains owned by the repository
maintainer/operator, with the rollback triggers above.
