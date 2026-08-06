---
title: "Final JSON cold CLI response-frame validation"
type: impl-plan
date: 2026-08-06
status: "reviewed / ready for harvest — plan-review PASS"
stash_id: "9D943A6F"
prior_feature: "112-F"
prior_shipment: "108-S"
pinned_autoharness_sha: "6a791dbe6d47d044595000fe894c94f051df6ba6"
scope: "one new bounded Windows validation release unit; no production change"
---

# Implementation Plan — Final JSON Cold CLI Response-Frame Validation

## Problem Frame

Shipment `108-S` exhausted its authorized `2/2` live attempts before its final debug-only JSON trace-format remediation could be exercised by a cold Windows CLI run. Its retained evidence proves exact client request ID `62046B37-cold-1`, usage correlation ID `62046B37`, owned PID and named-pipe cleanup, and a concrete pretty-trace/parser mismatch, but it does not prove a JSON-decodable terminal response-frame record at runtime.

The merged harness now requires one client disposition, one correlated `index_workspace` usage record, and one terminal `response_frame_result` carrying the same request ID and an explicit outcome. It already fixes the corpus hash, Windows named-pipe path, one-second request timeout, five-minute aggregate limit, sixty-second cleanup reserve, and twenty-second idle fallback.

This plan authorizes one distinct fresh validation attempt owned by a new feature and shipment. It is never attempt three of `108-S`. No production timeout fix, daemon redesign, protocol change, S072 work, audit work, or broad retained-test refactor is authorized.

## Routing Decision

Use direct planning, not a spike. The archived `108-S` investigation, item history, runtime report, decision, pre-merge closure, and post-merge closure already identify the exact unknown, the merged validation seam, the fixed IDs, and the safety controls. The remaining work is execution of the existing reviewed harness plus durable classification, not open-ended investigation.

## Requirements Trace

| Requirement | Planned action | Exit evidence |
|---|---|---|
| Distinct release unit | Create a new feature and queued shipment and label its sole live run `1/1` | New feature/task/shipment IDs; no mutation or reinterpretation of `108-S` |
| Exact IDs | Reuse request ID `62046B37-cold-1` and correlation ID `62046B37` without substitution | Exact equality across client envelope, usage record, and terminal frame record |
| TDD posture | U1 runs the existing deterministic parser, JSON-line, typed-frame, and cleanup contract before any live probe | Focused non-live tests pass at the shipment revision; failure blocks U2 |
| Strict Windows bounds | U2 uses the existing internal five-minute supervisor, sixty-second cleanup reserve, and twenty-second idle fallback | Start/finish and aggregate elapsed evidence; exactly one live invocation |
| Owned cleanup | Snapshot the new temp identity, prove exact daemon PID dead and exact pipe unreachable, then verify the exact temp workspace no longer exists after harness return | PID, pipe, force-kill flag, temp path, and post-return removal result |
| Decisive classification | U3 closes the old runtime blocker only on a complete exact-ID chain and complete cleanup; otherwise retains `BLOCKED` with one concrete cause | New runtime verification and decision artifacts with one final disposition |
| Scope freeze | Use the merged harness as-is and stop if code or timeout behavior would need modification | No source, schema, config, S072, audit, or retained-test refactor in the shipment |

## Scope Boundaries

In scope: focused non-live contract preflight, exactly one ignored Windows cold real-CLI invocation, one owned temporary workspace/daemon/named pipe, exact-ID evidence capture, post-return temp removal verification, and documentation of the close-or-retain decision.

Out of scope: any source or test edit, production timeout semantics, daemon startup or shutdown redesign, IPC wire changes, schema or persistence work, S072, audit, `12418607`, `017-D`, broad retained-test refactors, repository-daemon mutation, force termination without separate approval, or a second live attempt.

## Implementation Units

### U1 — Prove the existing JSON correlation contract before live execution

- Posture: test-first preflight using the already merged TDD harness; no source edits.
- Surface: `tests/integration/cold_cli_request_frame_correlation_test.rs` deterministic scenarios and the production typed-frame serializer tests introduced by `112.001-T` and `112.002-T`.
- Run only non-live focused coverage first. The ignored Windows live scenario must remain ignored during U1.
- Confirm the shipment revision still contains request ID `62046B37-cold-1`, correlation ID `62046B37`, corpus SHA-256 `58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25`, JSON capture switch `ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE`, and typed frame outcomes `flushed|serialize_error|write_error|flush_error`.
- Require the deterministic contract to reject timestamp-only or adjacency-only frame correlation and to require exactly one client, usage, and matching frame record.
- If any focused contract fails or the constants differ, stop before the live run, classify the new unit `BLOCKED`, and route any correction through a fresh Stage cycle. Do not edit code or tests in this shipment.
- TDD gate: existing tests are the RED-then-GREEN contract produced by `108-S`; U1 must prove GREEN before runtime validation. No implementation follows in this validation-only unit.
- Atomic exit: the exact merged contract is green and recorded, or U2 is blocked without consuming the live `1/1` allowance.
- Budget: <= 1 hour. Size: `S`. Complexity: `low`.

### U2 — Execute one newly owned bounded Windows cold CLI validation

- Posture: runtime characterization only; no file edits.
- Execute the existing ignored `windows_live::windows_cold_cli_request_frame_correlation` scenario exactly once after U1. This is attempt `1/1` of the new release unit and is not associated with `108-S` attempt accounting.
- Before invocation, record revision, binary provenance, Windows platform, an inventory of repository-contained `tmp/cold-cli-correlation-*` paths, and repository daemon identity as observation-only.
- Require a fresh workspace with no PID state and an unreachable derived named pipe before the CLI launches.
- Preserve the existing one-second user timeout, five-minute internal aggregate supervisor, sixty-second cleanup reserve, and twenty-second inherited idle fallback. Do not widen any bound and do not retry on environmental or evidence failure.
- Capture the exact request ID `62046B37-cold-1`, correlation ID `62046B37`, frozen corpus hash, client disposition, `index_workspace` usage outcome, terminal frame response ID and terminal outcome, CLI/aggregate elapsed times, exact owned daemon PID, and exact named pipe.
- Require exact PID death and pipe unreachability without force termination. After harness return, identify the exact newly owned temp path from the evidence and before/after inventory and prove that path no longer exists.
- If the harness preserves a workspace, the exact PID or pipe remains live, the temp path cannot be uniquely attributed or removed by normal harness cleanup, the aggregate bound expires, or a complete exact-ID chain is missing, stop with `BLOCKED`. Preserve the concrete PID, pipe, and path; do not kill a process or delete preserved evidence without explicit approval.
- A complete client/usage/frame ID chain with frame outcome `flushed` and complete PID/pipe/temp cleanup may classify as `CORRELATED-COMPLETION` or `CORRELATED-TIMEOUT` according to client disposition. No other result closes the old runtime blocker.
- Atomic exit: one and only one new live evidence packet exists, cleanup is proven or one concrete blocker is retained, and no second run is attempted.
- Budget: <= 2 hours. Size: `M`. Complexity: `medium`.

### U3 — Publish the final JSON validation decision and closure

- Posture: documentation-only width.
- Create a new runtime verification record and a new decision/follow-up record for the new feature and shipment; do not rewrite the historical `108-S` attempt table.
- Record revision, binary, Windows/named-pipe provenance, exact request/correlation IDs, corpus hash, U1 preflight, the new `1/1` attempt, all time bounds, client/usage/frame evidence, exact PID/pipe cleanup, temp cleanup, and force-kill status.
- End with exactly one classification: `CORRELATED-COMPLETION`, `CORRELATED-TIMEOUT`, or `BLOCKED`.
- State `108-S remains exhausted at 2/2` and identify this result as the sole attempt of the new release unit.
- Close the prior runtime `BLOCKED` classification only when U2 proves exact ID equality, a terminal frame outcome, and complete owned cleanup. Otherwise explicitly retain `BLOCKED` and name the single concrete blocker.
- If evidence suggests a production deadline change, reference the existing candidate boundary only as future intake; do not implement or plan that fix here.
- Atomic exit: durable evidence lets closure decide the runtime blocker without reopening raw logs or extending this shipment.
- Budget: <= 1 hour. Size: `S`. Complexity: `low`.

## Dependency Graph

```text
U1 -> U2 -> U3
```

The graph is acyclic. U1 is a fail-closed TDD preflight, U2 owns the sole live attempt, and U3 can publish only after U2 records cleanup or a concrete blocker.

## Decisions and Rationale

1. Direct plan over spike: prior shipped evidence resolved the investigative unknowns; only one controlled execution remains.
2. Reuse the merged harness over editing tests: exact IDs and containment are already encoded, while edits would widen scope and weaken traceability.
3. New `1/1` cap over extending `108-S`: the archived shipment cap is immutable and the intake asks for one fresh validation.
4. Exact response ID over temporal adjacency: only the terminal frame record can close the retained correlation gap.
5. Fail closed on temp provenance: PID and pipe cleanup alone do not satisfy this intake; successful closure also proves normal removal of the exact owned temp workspace.
6. Characterize, do not fix: a successful or blocked validation cannot authorize production timeout or daemon lifecycle changes.

## Risks and Caveats

- Cold startup can outlive the one-second request timeout because startup remains outside the request deadline. That known behavior is evidence, not authority to fix it.
- Running an ignored integration test may compile before execution. U1 must finish compilation and focused checks first so U2 is only the one bounded live invocation.
- Detached Windows processes and named-pipe release can lag. Cleanup remains inside the existing reserve and may not trigger a retry.
- The harness preserves its workspace on some blockers. Such preservation is a concrete blocker; deletion or force termination requires separate operator approval.
- Repository daemon identity must remain observation-only and must never be mistaken for the new temp daemon.

## Plan Hardening Signals

- Public API, schema, or contract change: ABSENT. This unit validates merged behavior and authorizes no edits.
- Security, auth, permission, or compliance-sensitive behavior: PRESENT at low scope because child-process ownership, workspace containment, and filesystem cleanup cross a local trust boundary.
- Migration, backfill, destructive data/config action, or irreversible step: PRESENT only on blocked paths; force termination or deleting preserved evidence is destructive and excluded without approval.
- External integration, operator checkpoint, or external dependency: PRESENT for explicit approval if a preserved process or workspace would otherwise be destroyed.
- High runtime, rollout, or rollback risk: PRESENT because a real CLI auto-spawns a detached Windows daemon and only one live attempt is authorized.

Requires plan hardening: yes

## Runtime Verification and Closure

U1 verifies the test contract without a live probe. U2 is the sole runtime window and must produce an exact client/usage/frame ID chain plus PID, pipe, and temp cleanup. U3 owns the new runtime verification and decision artifacts. Healthy signals are one temp identity, one exact ID chain, `flushed`, bounded completion, exact PID dead, exact pipe unreachable, temp path absent, and no force kill. Any missing signal retains `BLOCKED`.

No deployment, migration, or production rollout exists. Rollback is not applicable to runtime behavior because this release unit changes no code. Documentation correction is a normal path-scoped follow-up. Ship owns the single execution-session validation window.

## Plan Hardening

Hardening required: YES. The live path auto-spawns a detached Windows daemon, owns filesystem state, and has a non-repeatable `1/1` evidence allowance.

### Reinforcing context consulted

- Archived reviewed plan and decided plan for `108-S`.
- `108-S` runtime verification, decision, pre-merge closure, post-merge closure, PR #323 evidence, PR #324 closure evidence, and complete `112-*` backlog history.
- Engram CLI search, symbol inventory, code mapping, and impact analysis for the focused harness, contained capture, and typed frame event.
- Pinned autoharness `impl-plan`, `plan-harden`, `plan-review`, and `harvest` templates at exact SHA `6a791dbe6d47d044595000fe894c94f051df6ba6`.
- Compound-learning search returned no more specific retained guidance than the `108-S` durable decision.

### Protected invariants

1. `108-S` remains archived, shipped, and exhausted at `2/2`.
2. The new shipment owns exactly one live attempt, labeled `1/1`.
3. Request ID `62046B37-cold-1`, correlation ID `62046B37`, and corpus hash remain byte-for-byte exact.
4. Release behavior, wire bytes, timeout semantics, startup/shutdown ordering, and persistence do not change.
5. Only the new temp workspace, daemon PID, and named pipe are owned; repository daemon state is observation-only.
6. The five-minute aggregate includes daemon cleanup; successful closure also proves post-return temp removal.
7. No S072, audit, timeout fix, daemon redesign, or retained-test refactor enters the unit.

### Strict-safety action record

**ProposedAction A1**

- summary: Run focused non-live deterministic contract tests before the live probe.
- targets: existing cold CLI correlation and typed frame tests.
- change_kind: read/execute only; no edits.
- approval_required: no.
- ActionRisk: low.
- ActionResult: planned for Ship; Stage does not build or test.

**ProposedAction A2**

- summary: Execute exactly one cold Windows real-CLI validation in one repository-contained temp workspace.
- targets: one new temp workspace, one auto-spawned daemon PID, and one named pipe.
- change_kind: bounded local runtime characterization.
- approval_required: no.
- ActionRisk: moderate.
- ActionResult: planned as attempt `1/1`; no retry.

**ProposedAction A3**

- summary: Force-terminate any CLI or daemon that survives graceful and idle cleanup.
- targets: exact owned PID only.
- change_kind: destructive process termination.
- approval_required: yes.
- ActionRisk: destructive.
- ActionResult: excluded; classify `BLOCKED` and report identity unless separately approved.

**ProposedAction A4**

- summary: Delete a workspace intentionally preserved by the harness on a blocked path.
- targets: exact uniquely attributed temp path only.
- change_kind: destructive evidence removal.
- approval_required: yes.
- ActionRisk: destructive.
- ActionResult: excluded; preserve and report the path unless separately approved.

**ProposedAction A5**

- summary: Change production timeout, daemon lifecycle, IPC framing, or capture behavior.
- targets: production source and protocol surfaces.
- change_kind: contract or architecture change.
- approval_required: yes through a future reviewed Stage cycle.
- ActionRisk: high.
- ActionResult: blocked and outside this plan.

### Monitoring, rollback, and validation window

Ship owns one execution-session window. Monitor one temp identity, client process completion, exact daemon PID, exact named pipe, JSON-line parser output, usage correlation, terminal frame outcome, aggregate elapsed time, and post-return path existence. Stop signals are pre-existing endpoint state, multiple daemon identities, malformed or missing JSON, ID mismatch, aggregate expiry, failed PID/pipe cleanup, unproven temp removal, or any request for a second run.

No code rollback exists because no production or test edit is allowed. The safe fallback is `BLOCKED` with preserved evidence. Destructive process or filesystem cleanup requires explicit approval.

### Review-gate capability record

`TOOL_DEGRADED: reviewer-subagent-dispatch — declared fallback: single-agent persona pass`

`TOOL_DEGRADED: model-specific-review-routing — declared fallback: caller-model rubric pass`

All selected personas must be covered inline, and the review must emit literal `dispatch_mode:` and `decision:` markers before harvest.

## Plan Review

dispatch_mode: single-agent-declared-degradation
decision: PASS

**Gate:** PASS

**Review-fix cycles:** 1 of 3. An initial hardening finding required explicit post-return temp cleanup provenance and a fail-closed rule for preserved workspaces. The final plan now includes both.

**Hardening required:** yes — satisfied.

### Persona coverage

| Persona | Mode | Result |
|---|---|---|
| Constitution Reviewer | inline declared degradation | PASS — Stage only plans; Ship owns tests and runtime work; all three units fit the two-hour rule |
| Rust Reviewer | inline declared degradation | PASS — no source edit is authorized; the existing test-first exact-ID contract gates the live run |
| Scope Boundary Auditor | inline declared degradation | PASS — one validation attempt and two documentation outputs; timeout fixes, redesign, S072, audit, and refactors are excluded |
| Learnings Researcher | inline declared degradation | PASS — `108-S` evidence is authoritative and no compound learning supersedes it |
| Architecture Strategist | inline declared degradation | PASS — the plan reuses the existing harness and production seam without coupling or ownership changes |
| Agent-Native Parity Reviewer | inline declared degradation | PASS — evidence comes from the real JSON CLI and exact request/correlation identifiers |
| Security Lens Reviewer | inline declared degradation | PASS — fixed workspace containment, exact ownership, non-destructive cleanup, and approval boundaries are explicit |

### Findings

- P0: 0
- P1: 0 open
- P2: 0
- P3: 0

### Gate rationale

Every intake requirement maps to one dependency-ordered unit. The sole live run is a new `1/1` release-unit attempt, exact IDs and all Windows bounds are immutable, TDD preflight fails closed, and PID/pipe/temp cleanup is required for closure. Destructive fallback actions and production changes are explicitly excluded. The plan is ready for harvest.
