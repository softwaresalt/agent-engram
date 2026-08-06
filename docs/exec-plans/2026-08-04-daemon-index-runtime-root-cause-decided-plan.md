---
title: "Decided plan — daemon index persistence and IPC boundary characterization"
type: decided-plan
date: 2026-08-05
shipment_id: 107-S
feature_id: 111-F
decision: PARTIAL
source_plan: "docs/archive/plans/2026-08-04-daemon-index-runtime-root-cause-plan.md"
---

# Decided Plan — Daemon Index Persistence and IPC Boundaries

## Final Scope

The shipped unit is a bounded investigation, not a production fix. It freezes a
validated corpus, uses one owned daemon and endpoint, characterizes persistence
and IPC independently, and publishes exactly one FIX-READY, PARTIAL, or
NON-REPRODUCING decision.

## Decisions That Survived Review

1. Use separate baseline and daemon workspaces/databases with byte-identical
   source content.
2. Pre-warm an empty owned daemon workspace, suppress watcher ingestion, and
   issue exactly one measured index through one endpoint.
3. Observe singleton visibility before flush, after flush/finalize, and after
   graceful shutdown.
4. Treat startup and timed request/response phases as separate evidence
   boundaries while carrying one end-to-end deadline.
5. Cap every live probe at five minutes and two equivalent attempts.
6. Keep all diagnostics test-only or fully reverted; production protocol,
   schema, persistence, and timeout behavior remain out of scope.
7. Keep persistence and IPC findings separable and route any future fix through
   a fresh Stage review and width-isolated shipment.

## Result

- Persistence: no current defect on the validated bare-call corpus; one
  singleton remained visible before flush, after flush, and after shutdown.
- IPC: static `startup-outside-deadline` finding.
- Runtime blocker: missing cold CLI end-to-end request-ID/frame correlation.
- Final decision: **PARTIAL**.

## Rejected Alternatives

- Do not fold this work into archived 105-F; its duplicate-callee
  reconciliation mechanism is distinct.
- Do not infer a streaming or protocol redesign without the missing measured
  frame boundary.
- Do not repeat unbounded waits, use the repository daemon as a repro target,
  share baseline state, or manufacture a defect when the controlled corpus does
  not reproduce it.

## Follow-Up Gate

Entry `62046B37` owns one fresh bounded cold-CLI correlation run. Any later
production change must establish one deadline before health/startup and pass
only remaining budget into the request phase, with separate reviewed
acceptance contracts for persistence and IPC.
