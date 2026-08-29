---
title: "131-S / 138-F post-merge operational closure"
doc_type: closure
date: 2026-08-29
shipment_id: "131-S"
feature_id: "138-F"
pr: 366
merge_commit: dd0ba6116a39c54f8c25ff033c72211041b2a65f
status: ready-with-conditions
---

## Closure Verdict

**READY WITH CONDITIONS.** PR #366 was merged by a two-parent merge commit,
the approved PR-head tree is preserved exactly, shipment `131-S` and every
member are archived, and post-merge compilation is healthy. The code enters
distribution through the repository's normal GitHub Release path. The release
operator must perform the manual observation window below when a binary
containing this merge is deployed.

This record closes the post-merge workflow. It supplements, rather than
duplicates, the detailed pre-merge runtime evidence in
[`2026-08-27-131-s-terminal-vs-transient-health-classification-runtime-verification.md`](2026-08-27-131-s-terminal-vs-transient-health-classification-runtime-verification.md).

## Merge and Review Evidence

| Gate | Result |
|---|---|
| PR | `#366`, final state `MERGED` at `2026-08-29T21:05:38Z` |
| Approved head | `19ac3bbf290652ae9300482db3813e222e8e3faa` |
| Merge commit | `dd0ba6116a39c54f8c25ff033c72211041b2a65f` |
| Merge method | Merge commit; parents `06c1baa8...` and `19ac3bbf...` |
| Tree integrity | Merge tree equals approved-head tree `a31ecc31fd32e0d914b1b813a7741ac4cda4447d` |
| Review | Exact-head Copilot review present; 25 threads, 0 unresolved |
| CI | Build and Windows launcher checks succeeded on the approved head |
| Post-merge check | `cargo check --locked` passed using the shared target directory |

No admin bypass, squash, rebase, force push, or direct protected-main push was
used.

## Invariants to Preserve

1. Transport absence, reset, timeout, EOF, and response-cap exhaustion remain
   transient and retryable; only a received response that proves protocol or
   content incompatibility can become terminal.
2. Once a shim session publishes `Degraded`, no late monitor may overwrite it
   with `Ready`.
3. Terminal protocol incompatibility is fail-closed: wire code `15005`, exit
   code `14`, `recoverable: false`, and no `retry_after_ms` key.
4. Daemon-controlled error text and workspace paths do not flow into terminal
   client diagnostics or durable failure records.
5. The late-readiness monitor remains the sole best-effort writer for the
   late-terminal startup-failure record.
6. Existing transient readiness-timeout behavior remains compatible.

## Pre-Deploy Audit

| Check | Outcome |
|---|---|
| Feature flag / rollout gate | No bypass exists by design; a bypass would reopen the fail-closed path |
| Rollback readiness | Actionable merge-revert procedure recorded below |
| Data/schema compatibility | No migration or persistent schema change; one additive failure-class value |
| Cross-service coordination | No external service dependency; shim and daemon protocol behavior ship together |
| Monitoring readiness | Manual log/contract observation plan is defined below |
| Review and CI | Exact-head review, zero unresolved threads, and green required checks confirmed before merge |

## Deployment and Rollout Path

The merge is on `origin/main` and follows the repository's existing
GitHub Release pipeline into the next published `engram` binary. There is no
database migration, daemon-side staged rollout, maintenance window, or runtime
configuration change. Do not deploy a closure-branch build; only a normal
release containing merge `dd0ba611...` is in scope.

## Post-Deploy Checks

1. Start the released shim against a matching healthy daemon and confirm normal
   `initialize` and `tools/list` behavior.
2. Exercise a transient unavailable/not-ready daemon and confirm the response
   stays recoverable with `readiness_timeout` and a present retry delay.
3. Exercise a controlled protocol mismatch and confirm terminal
   `protocol_incompatible`, wire code `15005`, exit code `14`, and absent
   `retry_after_ms`.
4. Inspect `<workspace>/.engram/diagnostics/shim-startup-failures.jsonl` when
   the late monitor remains alive long enough to record the mismatch. Treat
   record absence as inconclusive because the write is best-effort.
5. Confirm a terminal `Degraded` state never returns to `Ready` in the same
   shim session.

## Monitoring Plan

The project has no centralized production dashboard for local-first shim
instances, so observation is a structured manual release checklist.

| SLI / signal | Baseline | Failure threshold | Query / location | Owner |
|---|---|---|---|---|
| Healthy matching-daemon startup | Initialize and tool listing succeed | Any matching healthy daemon classified terminal | Shim client response and process exit | Release operator |
| Transient classification | Unavailable/not-ready remains retryable | Any transport-only failure emits `protocol_incompatible` or exit `14` | Structured MCP error and shim exit code | Release operator |
| Terminal classification | Controlled mismatch is non-retryable | Mismatch remains indefinitely retryable, or includes `retry_after_ms` | Structured MCP error fields | Release operator |
| Monotonic state | `Degraded` is absorbing | Any `Degraded -> Ready` transition in one session | Shim logs and contract reproduction | Release operator |
| Durable diagnostic | Best-effort record for observed late terminal state | Malformed record, leaked daemon text/path, or duplicate record per monitor | `.engram/diagnostics/shim-startup-failures.jsonl` | Release operator |

## Healthy and Failure Signals

Healthy release signals are unchanged successful startup for matching peers,
retryable transient failure for unavailable peers, deterministic non-retryable
classification for proven incompatibility, and no regression in launcher
behavior.

Intervene if a healthy peer is terminalized, a proven mismatch loops as
recoverable, a terminal state reverts to `Ready`, daemon-controlled text leaks
to the client or record, or the Windows launcher check regresses.

## Rollback Trigger and Procedure

**Trigger:** any reproducible false terminal classification of a matching
healthy daemon, any `Degraded -> Ready` reversal, or a protocol-mismatch result
that violates the fail-closed wire contract.

**Procedure:** open a protected-main rollback PR that reverts the merge commit
with `git revert -m 1 dd0ba6116a39c54f8c25ff033c72211041b2a65f`.
Run the shim contract tests and required CI on the revert branch, obtain normal
review and operator approval, merge through repository policy, and republish
through the normal release process. No data rollback is needed.

## Validation Window and Owner

The **release operator** owns observation from the first release containing
the merge through **24 hours or the first 10 representative shim startups,
whichever is later**. Record the outcome as healthy, degraded, or rolled back
in the release handoff. Until that window completes, the operational result is
`READY WITH CONDITIONS`, not a claim that a deployed binary has been observed.

## Shipment and Backlog Closure

- `131-S`: archived, merge SHA `dd0ba611...`
- `138-F`: archived, merge SHA `dd0ba611...`
- `138.001-T` through `138.014-T`: archived
- active shipments after closure: none
- returned items: none
- post-reconciliation:
  [`.backlogit/reconcile/131-S-post-20260829T144828-0700.md`](../../.backlogit/reconcile/131-S-post-20260829T144828-0700.md)

Ignored completion-gate evidence for `138.008-T` through `138.014-T` was
unavailable in retained worktrees. It was regenerated without force through
the supported `done -> active -> done` path before shipment archival. The
existing compound guidance was corroborated and refined:
[`post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`](../compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md).

## Risky Action Record

| ProposedAction | ActionRisk | Approval / containment | ActionResult |
|---|---|---|---|
| Remove two large live worktrees after preserving unique state | Destructive: uncommitted work and 69.65 GiB of directories could be lost | Explicit operator approval at `2026-08-28T15:34:46-07:00`; rescue commits pushed first; non-force removal only | Applied; both approved worktrees removed, preserved refs retained |
| Merge PR #366 using a merge commit | High impact: changes protected default branch | Explicit operator approval at `2026-08-29T14:03:35-07:00`; all four load-bearing gates revalidated | Applied; PR merged as `dd0ba611...`, approved head/tree verified |
| Rehydrate missing ignored gate evidence | Medium: rewrites lifecycle timestamps on completed task artifacts | Normal backlogit transitions only; no `--force-gates`; merge SHA already confirmed | Applied; all seven gates passed and shipment archived |
| Preserve review worktree, rescue refs, and caches | Safety containment against accidental loss | Operator prohibited cleanup without separate approval | Applied; no additional destructive cleanup performed |

## Tooling and Residual Conditions

Backlogit CLI `v1.10.1` reads and index operations are healthy. Repository-wide
doctor output retains legacy advisories outside this release unit; there are
zero gate or shipped-event findings for `131-S`, `138-F`, or `138.*`.

Engram semantic discovery remained degraded after bounded daemon/workspace
diagnostics timed out. No daemon PID, runtime state, or workspace binding was
altered. This did not block merge, backlog reconciliation, or the direct
post-merge build check.
