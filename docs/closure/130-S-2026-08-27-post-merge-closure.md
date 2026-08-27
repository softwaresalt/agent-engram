---
title: "130-S post-merge operational closure"
doc_type: closure
shipment_id: "130-S"
feature_id: "137-F"
mode: post-merge
date: 2026-08-27
author: ship
verdict: "READY WITH CONDITIONS"
closure_status: "READY WITH CONDITIONS"
pr_number: 364
merge_commit: "2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0"
head_commit_merged: "db68add3514e1d85e9354fe2c93f63ec7e31c006"
runtime_verification_report: "docs/closure/130-S-2026-08-27-runtime-verification.md"
reconciliation_reports:
  - ".backlogit/reconcile/130-S-pre-20260827T104137-0700.md"
  - ".backlogit/reconcile/130-S-halt-20260827T104502-0700.md"
rca_reference: "docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md"
follow_up: "137.006-T"
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
| Requested reviewers at merge | none (Copilot absent from `requestRequests`) |
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
  instead of terminal — this is the known, tracked, and accepted residual
  gap (`137.006-T`); escalate if observed in the field before that follow-up
  lands.
* Any new panic, deadlock, or resource leak attributable to the single-flight
  probe/cooldown path.

## Rollback Procedure

Revert merge commit `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0` on `main` via a
standard `git revert -m 1` PR (merge-commit revert, preserving history), gated
through the same PR review/CI/approval pipeline as any other change. No data
migration or external state requires separate rollback — the change is
in-process shim/daemon logic only.

## Validation Window

Standard: next 2 normal development sessions / daemon lifecycles that
naturally exercise a slow or delayed daemon startup (this defect is
startup-timing-dependent and cannot be forced on a fixed calendar schedule;
observe opportunistically whenever a `readiness_timeout` is logged).

## Owner

Ship agent / repository maintainer (`softwaresalt`) for monitoring; Stage
owns triage and scheduling of the queued follow-up `137.006-T`.

## Outstanding Follow-Up (Not Part of This Closure)

`137.006-T` — *Distinguish terminal health-probe errors from transient
not-ready in late-readiness recovery path* — remains `status: queued`,
`parent_id: 137-F`, opened directly from the Copilot PR #364 review. It is
explicitly out of scope for `137-F`'s retro-staged verification-only mandate
(no production-logic edits) and is **not** implemented or claimed complete by
this closure. See disposition detail in
[`130-S-halt-20260827T104502-0700.md`](../../.backlogit/reconcile/130-S-halt-20260827T104502-0700.md).

## Backlog Closure Status

* `137.005-T`: `done` (archived).
* `137-F`: left `status: active` (commit field records merge SHA
  `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0` for traceability). `backlogit
  shipment ship 130-S` refused to close the shipment because it derives
  release scope from the covering-feature parent/child relationship and
  found `137.006-T` (a queued child of `137-F`) blocking closure, even though
  `137.006-T` is not a declared manifest member of `130-S`. Forcing past this
  gate would require an operator-authorized `--force-gates` override, which
  was not requested or granted (the operator's explicit approval covered only
  the PR #364 merge commit). `137-F` is therefore left accurately `active`
  rather than falsely marked `done`/shipped.
* `130-S`: remains `status: active` (not shipped/archived) pending resolution
  of the above. **The code change itself is fully merged, confirmed, and live
  on `origin/main` regardless of this backlog administrative state.**

## Verdict

**READY WITH CONDITIONS** — the shipped code is production-ready and
verified; the backlog shipment record cannot be formally closed
(archived) until `137.006-T` is completed, re-scoped away from `137-F` by
Stage, or an operator authorizes a force-gate override.
