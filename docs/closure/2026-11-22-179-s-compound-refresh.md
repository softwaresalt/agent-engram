---
title: "179-S compound refresh and recent learning scan"
date: 2026-11-22
doc_type: closure
source: "179-S / PR #490"
shipment_id: "179-S"
mode: propose
scope: recent
evidence:
  - "PR #490"
  - "merge commit a2677cf9d4d682e7703f14aa9f98c6545a7deebf"
  - "docs/closure/2026-11-21-179-s-runtime-verification.md"
  - "docs/closure/179-S-142-F-post-merge-closure.md"
  - "docs/closure/evidence/179-S-142-F-close-evidence.json"
---

## Scope

This refresh examined recent compound and continuous-learning evidence for
three 179-S concerns:

* doctor lifecycle ownership
* daemon smoke non-destructive semantics
* closure and safe-close evidence

PR #490 merged at
`a2677cf9d4d682e7703f14aa9f98c6545a7deebf`. The final feature HEAD was
`1708c75f414b0bc678b3c4a931b53de358e1136c`.

## Entries reviewed

| Entry | Classification | Evidence and rationale |
|---|---|---|
| `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md` | update | Its risk and exact-child principles remain valid, but any direct restoration procedure must be marked historical-only until it explicitly follows current `shipment-reconcile` capture, halt, fresh-approval, and revalidation safeguards |
| `docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md` | keep | Doctor smoke still performs a real readiness connection, so probes must not be misdescribed as having no interaction at all. The closure evidence is narrower: no workspace delta, bind, replacement, shutdown, or control RPC. The existing warning about probes affecting idle state remains accurate and important |
| `docs/compound/workflow-issues/backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md` | update | The risk statement remains valid, but any direct cascade procedure is historical-only; current closure requires the machine-classified close path, verified engine semantics, and explicit safe-close or approved cascade routing |
| `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md` | update | The root-cause observation remains useful, but the response text needs current `shipment-reconcile` safeguards. 179-S selected `SAFE_CLOSE` and verified the protected set rather than relying on a direct cascade |
| `docs/compound/workflow-issues/shipment-done-status-post-merge-closure-repair-2026-08-15.md` | keep | This entry addresses a shipment incorrectly moved to generic `done`. 179-S used a different governed closure path, so it is neither duplicated nor superseded |
| `docs/compound/destructive-reconciliation-needs-a-materialized-snapshot-2026-08-08.md` | keep | The 179-S close evidence materialized a 121-path protected observation set and verified it remained byte-identical. That supports the generic fail-closed snapshot principle without requiring a closure-specific rewrite |

## Shipped evidence

PR #490 introduced `DaemonOwnership` and an explicit preservation policy for
doctor lifecycle admission:

* an already-live compatible daemon is reused and left running
* protocol mismatch is returned without replacement
* a live PID with an unreachable endpoint is preserved and reported
* a reachable daemon may be awaited without ownership transfer
* only an invocation-started child is cleaned up when readiness fails
* a successfully started daemon is detached into its normal lifecycle

The smoke workflow reads the live daemon's reported mode, then issues only
read-only readiness requests. Managed mode receives `get_daemon_status`.
ReadServer mode additionally receives `get_workspace_status`. It issues no
workspace bind and no shutdown request.

The integration suite passed 14/14. It covers reused Managed and ReadServer
daemons, protocol mismatch, unreachable pre-existing endpoints, startup delay,
failed-start exact-child cleanup, hidden CLI behavior, structured errors, and a
recursive workspace snapshot with no ReadServer write-side state.

The post-merge closure selected `SAFE_CLOSE` with reason
`STANDARD_CLOSURE_SUFFICIENT`. It archived only the manifest task and shipment,
and all 121 protected observation-set paths remained byte-identical. The
active `142-F` feature and required successor records remained unchanged.

## Recent learning scan

The recent observation set contains no 179-S record and no direct duplicate
covering doctor smoke ownership. It includes:

* a 2026-10-31 ownership/isolation observation for live daemons and validation
  build trees
* a 2026-11-01 Cargo resource-cap observation

The ownership observation is adjacent but not a close semantic duplicate:
it concerns test/build isolation, while 179-S concerns lifecycle ownership and
process preservation. The resource-cap observation is unrelated to this
shipment's focus.

Three provisional patterns are recorded in
`.gsd/continuous-learning/instincts/2026-11-22-179-s-doctor-lifecycle-instincts.md`.
None meets the threshold for promotion to a new instruction or skill.

## Maintenance decision

Three entries retain valid risk evidence but contain procedures that should be
marked historical-only under current `shipment-reconcile` safeguards:

* the linked-worktree recovery and overship restore sequence needs the
  capture, halt, fresh-approval, and revalidation contract
* direct cascade guidance needs the classifier and engine-semantics gate
* the proposed updates remain outside this post-merge closure's mutation
  scope, so this report records them without rewriting the compound entries

The following guidance remains accurate:

* the exact-child and materialized-snapshot risk principles remain valid
* the new doctor-specific behavior is already captured in durable code,
  tests, runtime verification, and post-merge closure evidence
* the safe-close response is already institutionalized in workflow learnings

No compound file was changed. The report is intentionally in propose mode
and records three required historical-only updates for follow-up.

## Follow-up

Keep observing the following questions:

* whether diagnostic smoke probes repeatedly need a distinct preservation
  policy rather than the normal replacement-capable lifecycle path
* whether live-mode discovery should become a broader lifecycle convention
* whether the structured protected-observation-set safe-close format becomes
  the standard evidence shape across additional closures

If later shipped work supplies independent recurrences and invalidates or
duplicates an existing learning, perform a narrowly scoped update with the new
citations.
