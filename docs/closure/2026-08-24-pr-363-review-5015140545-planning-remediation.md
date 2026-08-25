---
title: PR 363 review 5015140545 planning remediation
type: review-remediation
doc_type: closure
source: Copilot review 5015140545 on PR 363
source_commit: 2a80fd27342fbc614efc58c830b81bfa59301b7f
date: 2026-08-24
status: superseded-by-exact-head-remediation
---

# PR 363 review 5015140545 planning remediation


> [!IMPORTANT]
> **HISTORICAL / SUPERSEDED.** Any queued-shipment, executable-handoff, old-roster, old-edge, or old reviewed-file statement below is source-head history only. It cannot authorize claim or implementation. Current authority: [PR #363 fail-closed planning authority](../decisions/2026-08-25-pr-363-fail-closed-planning-authority.md).

> **Current-authority notice:** This is historical source-head evidence only. Every eight- or thirteen-task, twelve-edge, fourteen-item, queued-shipment, or local-PASS statement below is superseded and must not guide execution. Current authority is `131-F`, `131.001-R`, `125-S`, and `docs/closure/2026-08-25-pr-363-mandatory-escalation-review.md`: seventeen tasks, sixteen edges, eighteen roster items, and failed-closed review.

## Exact-head supersession

Reviews `5015373740` and `5015447062` supersede this record's cleanup-bound claims. The current design treats the SDK setting as a per-export-future limit only and uses a detached native worker plus a five-second bound on daemon wait, with completion unknown after timeout. See `docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md` and the exact-head remediation closure.

## Scope

This is planning and backlog closure only. No application source, test, manifest, lockfile, configuration, build, linter, shipment claim, shipment close, merge, amend, force push, or PR 362 mutation is part of this remediation.

## Findings and dispositions

| Thread | Finding | Planning disposition |
|---|---|---|
| `PRRT_kwDORJEduc6b8O89` | Original sequence could not reach a compiling RED before implementation. | Replaced by compile-neutral U1, graph U2, compile-baseline U3, explicit seam U4, and compiling provider RED U5. |
| `PRRT_kwDORJEduc6b8UJJ` | 131.001-T could only fail during production compilation. | 131.001-T now compiles with the feature disabled and fails runtime assertions over isolated `cargo tree` and `cargo check --lib` subprocesses. |
| `PRRT_kwDORJEduc6b8UIv` | Flush and shutdown inherited an operator-controlled SDK timeout. | Superseded: U6 owns only a per-export-future constant. U10/U11 isolate synchronous cleanup in a detached native worker and bound only daemon wait; timeout reports completion unknown. |
| `PRRT_kwDORJEduc6b8O9M` | 021-D lacked semantic archive provenance. | Added original queue path and archived status fields plus exact original stash, blocked feature/review/shipment, active replacement, and failed-harvest explanation. |
| `PRRT_kwDORJEduc6b8O9f` | 022-D lacked semantic archive provenance. | Added the parallel provenance record for original stash 7B15B447 and active replacement 172AE8CE. |

## Reviewed execution shape

Feature `131-F` now has thirteen linear tasks. All estimates are 45 to 115 minutes. `131.001-T` remains one test file and 75 minutes. No task exceeds two files or evidence surfaces, four functions, three scenarios, one skill domain, or one atomic milestone.

Shipment `125-S` contains the parent plus all thirteen tasks and remains queued and unclaimed. Blocked shipments `126-S` through `129-S` and their dependencies remain unchanged.

## Timeout and operational closure

`OTLP_EXPORT_TIMEOUT = 5s` is application-owned but bounds each exporter future only. `OTLP_CLEANUP_WAIT_TIMEOUT = 5s` separately bounds the daemon's wait on one detached native cleanup worker. If the synchronous SDK call does not return, the daemon reports completion unknown and does not join or claim cancellation.

Ship owns the post-deploy observation window: 30 minutes or three controlled daemon exits. Healthy state is zero export/worker/cleanup/timeout records and controlled child exit within the cleanup wait plus a two-second harness allowance. Any failure, timeout, hidden residual, missing span, or feature-gate regression disables `otlp-export` and reverts the owning GREEN commits.

## Provenance closure

Deliberations `021-D` and `022-D` are semantic archives of accepted deliberation, not successful executable harvests. Their attempted planning targets `133-F` / `127-S` and `132-F` / `126-S` are blocked and their review artifacts are invalidated. The original stash archive records now use reason `archived`, disposition `blocked_unverified_planning`, attempted target IDs, and sole replacement IDs. Active replacements are exactly `8C7733CE` for original `1CB366DB` and `172AE8CE` for original `7B15B447`; neither original is active.

## Review gate

Plan hardening and standard review were rerun after the redesign. Gate: **PASS** with no unresolved P0 or P1 planning finding. Cross-model and intercom dispatch were unavailable and are disclosed in the plan. Substantive commit `7f36eed8de3f4fb7c0335497da3652636151f0b3` was pushed; all five exact external review threads received evidence replies and are resolved, with zero unresolved PR threads at verification.

## PR metadata proposal for Ship

Proposed title:

```text
chore(stage): queue 13-task OTLP repair; keep four identity plans blocked
```

Current body facts: feature `131-F`; tasks `131.001-T` through `131.013-T`; fourteen-item parent-first roster; exactly twelve task dependency edges; estimates 45-115 minutes; target index 1,126 artifacts; layer-held tracer/provider clone retention plus a separate explicit application lifecycle/flush handle; child-process-only endpoint environment tests; five-second per-export-future policy; five-second daemon cleanup-wait deadline with unknown completion after timeout; `125-S` sole queued/unclaimed; `126-S` through `129-S` blocked; sole replacement pairs include `721A42F0` and `BD5DD62A`. The exact copy-ready body is superseded by `docs/closure/2026-08-25-pr-363-review-5015710467-remediation.md`; Stage does not edit live PR metadata.
