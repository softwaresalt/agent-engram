---
title: "Index coordinator and observability reliability"
type: implementation-plan
date: 2026-08-07
source: docs/decisions/2026-08-07-dark-factory-active-stash-triage-decision.md
status: reviewed
source_stash_ids: [86EDE287, 3FA0320D, 12418607]
---

# Index coordinator and observability reliability

## Problem Frame

Three reliability gaps remain after the single-authority coordinator release. Direct sync branch refresh completes the retiring owner before it claims reissued work, allowing a waiter notification between those operations. Metrics branch control is lossy under a full channel. The no-prior canonical-snapshot failure branch lacks its own deterministic regression. Workspace status may report zero graph counts because most immutable count queries bypass the existing SQLITE_BUSY retry and callers collapse errors to zero.

## Requirements Trace

- 86EDE287 maps to U1 and U2.
- 3FA0320D maps to U3.
- 12418607 maps to U4.

## Implementation Units

### U1 — Atomic direct-sync branch refresh

Files: src/tools/write.rs only, including its private tests. Preserve the publication admission returned by publish_workspace_generation_with_reissue and call CoordinatorCell::acknowledge_and_claim_reissued before any waiter notification. RED proof saturates the waiter interleaving and asserts the original direct Sync owns the reissued routine mask. Preserve queued response semantics. Cap: three scenarios, one file, 110 minutes.

### U2 — Guaranteed acknowledged metrics branch control

Files: src/services/metrics.rs and src/tools/write.rs. Replace lossy SwitchBranch try_send with an awaited control message carrying acknowledgment; ordinary usage events remain non-blocking and droppable. Every branch-refresh call waits for acknowledgment before branch-scoped work continues. RED proof fills the event channel, sends branch control, drains, and verifies the next empty-branch event lands in the new branch. Cap: three scenarios, two files, 110 minutes.

### U3 — No-prior canonical-snapshot retry regression

Files: tests/integration/code_graph_test.rs only. Add a deterministic ordinary-index scenario with no prior canonical snapshot, a topology-forced invalid-UTF-8 descendant, and a clean restoration. Assert failure leaves the relation absent, the clean pass reparses before publishing, and the following pass hash-skips. No production seam or fourth scenario. Cap: three phases, one file, 90 minutes.

### U4 — Busy-tolerant workspace graph counts

Files: src/db/cozo_queries.rs and tests/integration/smoke_test.rs. RED first against the existing immutable busy-retry driver, then route function, class, interface, and edge count scripts through run_script_busy_retry_immutable like count_code_files. Keep get_workspace_status and get_workspace_statistics contracts unchanged and prove S072 reports the preindexed fixture rather than transient zeros. Cap: three scenarios, two files, 110 minutes.

## Dependency Graph

U1 blocks U2 because metrics acknowledgment must be placed after the atomic successor claim. U3 and U4 are independent. Within the shipment, execute U1, U2, U3, U4 to minimize shared reliability uncertainty.

## Decisions and Rationale

Reuse acknowledge_and_claim_reissued; do not add a coordinator queue or notification flag. Make only branch-control messages reliable; usage telemetry remains best-effort. Cover no-prior behavior in integration tests without a public failpoint. Reuse the existing immutable busy-retry helper instead of polling in S072.

## Risks and Caveats

Awaiting branch control can deadlock if the writer is absent or closed, so the API must return an explicit error or no-op only when metrics is disabled and must never hold a coordinator lock across await. Count retries remain bounded and cannot hide persistent DB errors.

## Plan Hardening Signals

- Public API, schema, or wire change: absent.
- Security or permission behavior: absent.
- Migration or destructive action: absent.
- External integration: absent.
- High runtime or rollback risk: present; coordinator ownership and background writer ordering change.

Requires plan hardening: yes

## Runtime Verification and Closure

Ship runs saturated-channel, direct-sync waiter, canonical retry, and S072 focused tests in disposable workspaces, then ordered gates. Monitor coordinator stale/missing outcomes, metrics control failures, SQLITE_BUSY retries, and zero-count status responses. Rollback trigger: duplicate owners, lost routine work, metrics writer stall, or persistent status zeros. Closure records Windows and Unix disposition and a seven-day observation owner.

## Plan Hardening

Hardening is required for concurrent runtime ownership. No mutex may be held across metrics acknowledgment; control acknowledgment is bounded by channel closure and writer task health; permit ownership remains RAII-protected.

ProposedAction: atomically acknowledge the old direct Sync and claim its new-binding routine work.  ActionRisk: high.  Approval required: yes; operator dark-factory approval was retained for shipment 111-S.  ActionResult: applied on the reviewed feature branch; runtime verification and merge evidence remain required.

ProposedAction: make metrics branch switch an acknowledged control path while retaining droppable usage events.  ActionRisk: moderate.  Approval required: no additional approval.  ActionResult: applied with bounded acknowledgment and explicit disabled/unavailable writer behavior; runtime verification remains required.

Protected invariants: one owner, no waiter before successor installation, one routine bit, no coordinator lock across await, bounded busy retry, no public test seam. Rollback is a code revert.

## Plan Review

Gate: PASS. Hardening requirement satisfied. Constitution, Rust, scope-boundary, learnings, architecture, and agent-parity personas reviewed all units; the security persona was not triggered. Findings: P0 0, P1 0, P2 0, P3 0. The plan explicitly separates lossy event telemetry from reliable control and keeps all units under the two-hour cap. Ready for harvest.
