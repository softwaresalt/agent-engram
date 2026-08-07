---
title: "Final JSON cold CLI validation decided plan"
type: decided-plan
date: 2026-08-06
status: implemented
shipment_id: "109-S"
feature_id: "113-F"
source_plan: "docs/archive/plans/2026-08-06-final-json-cold-cli-validation-plan.md"
---

# Final JSON Cold CLI Validation — Decided Plan

## Decision

Use a distinct validation-only release unit with one authorized Windows live
attempt. Reuse the merged `108-S` harness without source or test edits, require
the exact client/usage/frame identity chain and complete owned cleanup, then
publish one durable classification.

## Implementation Units

1. **Deterministic preflight:** run the focused parser, JSON-line, typed-frame,
   exact-cardinality, adjacency-rejection, and cleanup coverage while leaving
   the ignored live scenario unexecuted.
2. **Sole live validation:** run the existing ignored Windows scenario exactly
   once with request ID `62046B37-cold-1`, correlation ID `62046B37`, corpus
   hash `58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25`,
   a 300-second aggregate bound, a 60-second cleanup reserve, and a 20-second
   idle fallback.
3. **Durable decision:** close the prior blocker only if one exact
   client/usage/frame chain and PID/pipe/temp cleanup are all proven.

Dependency order was `U1 → U2 → U3`.

## Protected Constraints

- Shipment `108-S` remains archived and exhausted at `2/2`.
- Shipment `109-S` owns exactly one attempt and is exhausted at `1/1`.
- No production timeout fix, daemon redesign, IPC change, source or retained
  test edit, schema/configuration change, S072 work, audit work, or force
  termination.
- Repository daemon state is observation-only.
- Missing or ambiguous evidence fails closed as `BLOCKED`; no retry is
  authorized.

## Outcome

The deterministic preflight passed. The sole live attempt completed in
8,354 ms aggregate time:

- client response ID `62046B37-cold-1`, disposition `completion`, exit `0`;
- `index_workspace` usage correlation `62046B37`, outcome `success`;
- terminal frame response ID `62046B37-cold-1`, outcome `flushed`; and
- exact PID dead, exact named pipe unreachable, exact temp path absent, with no
  force kill.

Final classification: **CORRELATED-COMPLETION**. The final-JSON runtime blocker
retained by `108-S` is closed without changing production behavior.

## Rejected Alternatives

- **A third `108-S` attempt:** rejected because its `2/2` cap is immutable.
- **Editing the retained harness:** rejected because the merged contract was
  already sufficient and edits would widen the release unit.
- **Temporal or adjacency correlation:** rejected because only exact response
  ID equality closes the evidence gap.
- **A second live attempt:** rejected by the `1/1` cap.
- **Production deadline or lifecycle changes:** rejected as separate product
  work requiring fresh reviewed intake.
