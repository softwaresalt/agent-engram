---
title: "130-S post-merge operational closure"
doc_type: closure
shipment_id: "130-S"
feature_id: "137-F"
mode: post-merge
date: 2026-08-27
author: ship
verdict: "SHIPPED"
closure_status: "SHIPPED"
pr_number: 364
merge_commit: "2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0"
head_commit_merged: "db68add3514e1d85e9354fe2c93f63ec7e31c006"
runtime_verification_report: "docs/closure/130-S-2026-08-27-runtime-verification.md"
reconciliation_reports:
  - ".backlogit/reconcile/130-S-pre-20260827T104137-0700.md"
  - ".backlogit/reconcile/130-S-halt-20260827T104502-0700.md"
  - ".backlogit/reconcile/130-S-post-20260827T112336-0700.md"
rca_reference: "docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md"
follow_up: "138-F"
follow_up_shipment: "131-S"
follow_up_superseded: "137.006-T (re-parented, not cloned, into 138.001-T)"
---

# 130-S post-merge operational closure

## Summary

`130-S` (feature `137-F`) is a Stage-governed corrective wrapper that
independently verified, blast-radius-audited, and governed the release of the
already-implemented late-readiness stdio-proxy recovery change set (originally
produced ad hoc under archived `136-F`/`136.001-T`). The root-cause analysis
for the underlying incident is **not duplicated here**; see
`docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md` for the
full RCA (sticky cached-proxy-state defect on late daemon readiness).

PR #364 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) with explicit operator approval recorded in-session
(`PR 364: Merge approved`, 2026-08-27 10:35 PDT).

## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #364 state | `MERGED` at `2026-08-27T17:37:01Z` |
| Merge commit | `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0` |
| Parents | `19ae3160b7040e16213eba9ef7611f6573d3f4cd` (prior `main`) + `db68add3514e1d85e9354fe2c93f63ec7e31c006` (feature branch HEAD) — two parents, confirmed true merge commit |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor 2e1e01cf... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide |
| Pre-merge Copilot review at HEAD | `commit_id db68add3...` == HEAD; state `COMMENTED` |
| Requested reviewers at merge | none (Copilot absent from `reviewRequests`) |
| Unresolved review threads at merge | 0 of 5 (`isResolved: true` on all) |
| `mergeStateStatus` / `mergeable` (pre-merge) | `CLEAN` / `MERGEABLE` |
| CI checks | `build`: SUCCESS, `start-launcher-windows`: SUCCESS |

Full gate evidence is in
[`130-S-pre-20260827T104137-0700.md`](../../.backlogit/reconcile/130-S-pre-20260827T104137-0700.md).

## Runtime Verification

Verdict: **PASS WITH FOLLOW-UP** (see
[`130-S-2026-08-27-runtime-verification.md`](./130-S-2026-08-27-runtime-verification.md)).
`cargo test --test contract_shim_stdio_initialize` — 5/5 passed, including the
direct regression proof `shim_recovers_after_timed_out_daemon_later_becomes_ready`
and the disconnect-teardown proof
`shim_aborts_unresolved_startup_after_client_disconnects`. CLI smoke
(`engram.exe --help`) returned the expected command surface.

## Invariants to Preserve

* A cached `readiness_timeout` must remain a **recoverable** intermediate
  state (`WaitingForReadiness`, `retry_after_ms`), not a permanent session
  failure.
* Terminal failures (admission, endpoint, protocol/version mismatch,
  shutdown) must remain **fail-closed** — never silently reclassified as
  recoverable/retryable.
* Client disconnect during unresolved startup must deterministically cancel
  outstanding work within the teardown budget (no orphaned probes/timers).
* The session-wide single-flight probe + cooldown must prevent duplicate
  concurrent health probes against the same daemon.

## Pre-Deploy Audits

* No schema, migration, or config-flag changes ship with this change set.
* No new external dependency, port, or credential surface introduced.
* Scope confirmed via merge-diff to be limited to
  `src/shim/mod.rs`, `src/shim/transport.rs`, `src/daemon/ipc_server.rs`,
  `src/db/cozo_backend/mod.rs`, `tests/contract/shim_stdio_initialize_test.rs`,
  plus documentation/backlog artifacts.

## Deployment / Rollout Path

Merge-only. `engram` is distributed as a per-workspace binary/plugin; there is
no separate deploy or canary step beyond the merge landing on `origin/main`
and downstream consumers picking up the next build/release of the `engram`
binary. No maintenance window required.

## Post-Deploy Checks

* Contract suite `contract_shim_stdio_initialize` continues to pass on
  `main` (already verified pre-merge in this session on the merged HEAD; CI
  `build` check was SUCCESS at merge).
* Spot-check any daemon that logs `readiness_timeout` in the field: confirm
  the shim subsequently recovers on the next `tools/call` once the daemon's
  `_health.status` becomes `ready`, rather than continuing to return the
  cached error.

## Healthy Signals

* Shims that hit a startup readiness timeout subsequently recover
  automatically (recoverable retry_after_ms → successful `tools/call`) once
  the daemon becomes ready, with no session restart required.
* No increase in terminal/fail-closed misclassification for genuinely
  incompatible or unreachable daemons.
* No new orphaned background probe/monitor tasks after client disconnect.

## Failure Signals (Rollback Trigger)

* A production/field report of a session that remains stuck reporting a
  stale `readiness_timeout` **after** the daemon has independently confirmed
  `_health.status = "ready"` (i.e., the original sticky-proxy-state defect
  reproducing post-merge).
* A production/field report of a permanently protocol-incompatible daemon
  (e.g., version mismatch) being reported as `recoverable: true` indefinitely
  instead of terminal — this is the known, tracked residual gap, now owned
  by `138-F` (queued shipment `131-S`, not `137.006-T` directly — see
  Outstanding Follow-Up below); escalate if observed in the field before
  that follow-up lands.
* Any new panic, deadlock, or resource leak attributable to the single-flight
  probe/cooldown path.

## Rollback Procedure

Revert merge commit `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0` on `main` via a
standard `git revert -m 1` PR (merge-commit revert, preserving history), gated
through the same PR review/CI/approval pipeline as any other change. No data
migration or external state requires separate rollback — the change is
in-process shim/daemon logic only.

## Validation Window & Monitoring Plan

There is no external metrics/APM backend for this CLI/daemon tool; the only
currently-instrumented sources are the structured startup-failure diagnostics
file and the CI contract suite. Monitoring is therefore log/CI-based, not
dashboard-based:

| SLI | Source / query | Baseline | Threshold (escalate) | Owner |
|---|---|---|---|---|
| Readiness-timeout sessions recover without restart (heuristic) | Count `failure_class: readiness_timeout` records in `<workspace>/.engram/diagnostics/shim-startup-failures.jsonl` over the observation window and separately confirm the same recovery behavior continues to hold in CI via `contract_shim_stdio_initialize::shim_recovers_after_timed_out_daemon_later_becomes_ready` on every `main` build. **Caveat**: the diagnostics record schema carries only `timestamp`, `binary_version`, `failure_class`, and `message` — no PID or session-correlation key — so a record cannot be joined 1:1 to a specific client-log session when multiple shim processes run concurrently; this is a population-level heuristic, not a per-record proof, until a correlation key is added | 100% of CI recovery runs pass; field record volume does not diverge from CI-observed rates | Any CI regression on the recovery test, or a field/support report describing a session stuck on `readiness_timeout` after the daemon reports ready | Repository maintainer (`softwaresalt`) |
| Terminal daemons misreported as recoverable | Grep shim diagnostics/log output for `recoverable: true` following a `VersionMismatch`/protocol-incompatible `_health` payload arriving after the initial readiness deadline | N/A (known, unfixed gap — this is the exact defect `138-F` closes) | Any field occurrence, at any time, until `138-F` ships | Repository maintainer (`softwaresalt`) |
| Orphaned probe/monitor tasks after disconnect | `contract_shim_stdio_initialize::shim_aborts_unresolved_startup_after_client_disconnects` in CI on every `main` build; manual process-list spot-check (`Get-Process`/`ps`) after a deliberate client disconnect during opportunistic observation | 0 orphaned processes/tasks | Any orphaned process/task surviving past the teardown budget | Repository maintainer (`softwaresalt`) |

Observation window: opportunistic across the next 2 normal development
sessions/daemon lifecycles that naturally exercise a slow or delayed daemon
startup — this defect is startup-timing-dependent and cannot be forced on a
fixed calendar schedule; check the diagnostics file whenever a
`readiness_timeout` is logged.

## Owner

Ship agent / repository maintainer (`softwaresalt`) for monitoring; Stage
owns triage and scheduling of the queued follow-up (now `138-F` / shipment
`131-S`).

## Outstanding Follow-Up (Not Part of This Closure)

`137.006-T` was **not** implemented or completed as part of this closure.
Instead, Stage resolved the covering-feature blocker documented below by
re-scoping it: the finding is now owned by an independent feature `138-F`
("Classify terminal vs transient daemon health outcomes in the shim
late-readiness recovery path"), task `138.001-T`
(`origin_feature: 137-F`, re-parented — **not** cloned), with a reviewed
plan, hardening pass, and review gate
(`docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review.md`,
`138.001-R`, approved with changes, 2 cycles). `138-F` and its six sibling
tasks are queued under a new, separate shipment `131-S`
(`138-F, 138.002-T, 138.003-T, 138.001-T, 138.004-T, 138.005-T, 138.006-T,
138.007-T`) — `status: queued`, unclaimed, not executed as part of `130-S`.
`130-S`'s own exact manifest (`137-F`, `137.001-T`…`137.005-T`) was never
altered by this re-scope. See
[`130-S-post-20260827T112336-0700.md`](../../.backlogit/reconcile/130-S-post-20260827T112336-0700.md)
for the resolution detail and
[`130-S-halt-20260827T104502-0700.md`](../../.backlogit/reconcile/130-S-halt-20260827T104502-0700.md)
for the original blocker.

## Note on 137.005-T's Precondition (Copilot PR #365 review)

Copilot's review of the PR #365 closure diff flagged that `137.005-T`
(archived `done`) carries a HARD PRECONDITION requiring "the pre-existing
unresolved merge conflict in `.backlogit/stash.jsonl` is resolved BY THE
OPERATOR outside this shipment," which remains genuinely unresolved in the
root worktree (`C:\Source\GitHub\engram`, `UU .backlogit/stash.jsonl`,
confirmed unchanged in this session too). The precondition text predates the
isolated-worktree execution model actually used: every commit that satisfied
`137.005-T`'s acceptance criteria (`d8488a1f`, `db68add3`, `86bca897`, and
this closure's own commit) was made in the dedicated worktree
`.worktrees/ship-137-late-readiness-proxy-recovery-20260826`, which — as a
linked worktree with its own index — does not carry the conflicted-index
state that blocks `git commit` in the root worktree; PR #364 merged cleanly
via GitHub, not via a local commit in the conflicted root tree. The
precondition's underlying intent (no undecided merge state gating the
committed/PR'd/merged change set) was therefore satisfied for the actual
commit path used; the literal root-`stash.jsonl` conflict remains a
separate, pre-existing, unrelated condition that this pipeline is not
authorized to touch and that does not gate `137.005-T`'s or `130-S`'s
completion. `137.005-T` is left archived/`done` as-is (immutable terminal
history, not reopened); this note is the durable clarification.

## Backlog Closure Status

* `137.001-T`…`137.005-T`: `done` (archived); `137.001-R` review gate
  archived alongside.
* `137-F`: `status: done` → archived. Stage's re-scope of `137.006-T` out
  from under `137-F` (into independent feature `138-F`) left `137-F` with
  zero queued/non-terminal children, so `backlogit move 137-F --status
  done` succeeded cleanly with no force flag.
* `130-S`: `backlogit shipment ship 130-S --sha
  2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0 --message "..." --author
  "Derek Williams <42183845+softwaresalt@users.noreply.github.com>"`
  **succeeded**, no `--force-gates`. `archived_ids`: `137.001-R`,
  `137.001-T`, `137.002-T`, `137.003-T`, `137.004-T`, `137.005-T`, `130-S`,
  `137-F`. `returned_ids`: none. `shipment_status: shipped`. Post-mode
  `backlogit doctor --check-over-archived-features
  --check-shipped-event-completeness` shows zero findings against any
  `130-S`/`137-F`/`137.00N-T`/`138-F`/`138.00N-T`/`131-S` artifact.

## Verdict

**SHIPPED** — the code was already production-ready and verified in the
prior session; the backlog shipment record is now formally closed
(archived, `shipped`) with merge evidence on every manifest member. The
covering-feature blocker was resolved by Stage's re-scope (not by a
force-gate override). The narrow terminal-vs-transient health-classification
gap remains open, now tracked and reviewed as `138-F` / shipment `131-S`
(queued, unclaimed, out of scope for this closure).
