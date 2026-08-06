---
title: "Cold CLI request/frame correlation decided plan"
type: decided-plan
date: 2026-08-05
status: implemented-with-runtime-blocker
shipment_id: "108-S"
feature_id: "112-F"
source_plan: "docs/archive/plans/2026-08-05-cold-cli-request-frame-correlation-plan.md"
---

# Cold CLI Request/Frame Correlation — Decided Plan

## Decision

Run one bounded Windows characterization shipment rather than another spike.
Use a focused RED-first ignored test, add only the minimum debug-contained
launch/frame observability, perform one post-seam run, and publish the retained
result without changing production timeout semantics or IPC behavior.

## Implementation Units

1. **Focused RED harness:** create one integration test file with deterministic
   parser/cleanup coverage and one ignored Windows live scenario. Use fixed
   request ID `62046B37-cold-1`, correlation ID `62046B37`, a one-second user
   timeout, one owned workspace/daemon/pipe, and a five-minute aggregate bound.
2. **Contained correlation seam:** add a debug-only boolean capture switch with
   fixed files under the owned workspace `.engram`, plus a terminal frame event
   carrying connection ID, exact response ID, and write/flush outcome.
3. **Durable decision:** record both attempts, cleanup, exact retained evidence,
   and the gate for any later timeout-contract change.

Dependency order was `U1 → U2 → U3`. The RED run counted as attempt one, and
the post-seam run counted as attempt two.

## Protected Constraints

- No third live attempt belongs to shipment `108-S`.
- No persistence reopening, S072 work, audit work, retained-test refactor,
  timeout fix, protocol redesign, or repository-daemon mutation.
- Capture cannot select an arbitrary host path or alter release behavior.
- JSON-RPC wire bytes, ID echo, timeout semantics, startup ordering, and
  shutdown ordering remain unchanged.
- Cleanup uses graceful shutdown and an inherited idle fallback; force
  termination requires separate approval.

## Accepted Execution Deviation

Review remediation added `src/bin/engram.rs` so the existing tracing subscriber
selected JSON only under the same debug-only capture switch. This avoided
awaited capture I/O in the IPC handler and preserved shutdown ordering. It was
covered deterministically but not live-executed because the two-run cap was
already exhausted.

## Outcome

Both live CLI runs completed and cleaned up their exact PID and named pipe.
The client and dispatch records retained the fixed IDs, but attempt two used
pretty tracing, so no JSON-decodable terminal frame record was preserved. The
final JSON remediation therefore remains non-live verified only.

The static startup-outside-deadline finding is corroborated: both cold
commands completed after more than seven seconds despite `--timeout 1`.
Production behavior was not changed.

Final classification: **BLOCKED**. Fresh live validation is routed through
stash `9D943A6F` and requires a new Stage cycle.

## Rejected Alternatives

- **Another spike:** rejected because shipment `107-S` had already isolated the
  unknown and safety controls.
- **Editing the retained 107-S test:** rejected to preserve width isolation and
  avoid the unrelated retained-test refactor.
- **Timestamp or connection adjacency:** rejected because exact correlation
  requires the JSON-RPC response ID.
- **Arbitrary capture paths or release logging:** rejected for containment and
  contract safety.
- **Implementing an end-to-end timeout now:** rejected as a separate
  user-visible behavior change requiring fresh planning and evidence.
