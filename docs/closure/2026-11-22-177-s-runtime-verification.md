---
title: "177-S runtime verification and pre-merge operational readiness"
date: 2026-11-22
doc_type: closure
source: "177-S / 142.058-T"
shipment: 177-S
task: 142.058-T
feature: 142-F
mode: pre-merge
surface: contract-test
verification_status: PASS_WITH_FOLLOW_UP
closure_status: READY_WITH_CONDITIONS
compaction_status: pending
owner: softwaresalt
last_code_affecting_head: "0c4f9d454fae43333df0910a44ca8235c9f9ccb4"
follow_up_stash:
  - "FF1FC372"
conditions:
  - condition: "Required feature-PR CI is green on the final head"
    satisfied: false
    evidence: "The feature PR has not yet been created in this resumed phase"
    owner: softwaresalt
    due_utc: "2026-11-23T23:59:59Z"
    attestation_evidence: "Required build and start-launcher-windows checks pass on the final head"
    escalation_path: "Hold merge and invoke fix-ci within scope; halt at the circuit breaker"
    backlog_item_id: "N/A"
  - condition: "Copilot review is satisfied for the final head"
    satisfied: false
    evidence: "The feature PR has not yet been created in this resumed phase"
    owner: softwaresalt
    due_utc: "2026-11-23T23:59:59Z"
    attestation_evidence: "gsd gate copilot-review reports PASS with no open findings"
    escalation_path: "Hold merge until all threads and review-body findings are dispositioned"
    backlog_item_id: "N/A"
  - condition: "P-014 merge approval applies to the bounded 177-S scope"
    satisfied: true
    evidence: "The operator's 2026-11-22 activation explicitly pre-authorizes merge for scope [179-S, 177-S] after all P-014/P-018/CI/P-009/P-016 checks pass"
    owner: softwaresalt
    due_utc: "2026-11-23T23:59:59Z"
    attestation_evidence: "Bounded P-017 activation records merge_approval_pre_authorized: true for 177-S"
    escalation_path: "Wait for explicit operator approval if the bounded activation no longer applies"
    backlog_item_id: "N/A"
---

## Scope and immutable evidence

Shipment `177-S` completes the F54d read-provenance and CLI/MCP equivalence
contract for task `142.058-T`. The last code-affecting revision is
`0c4f9d454fae43333df0910a44ca8235c9f9ccb4`. The original implementation
commit remains `38b570acae5521088741b8f51724565abece8560`.

The implementation remains confined to
`tests/contract/read_server_cli_mcp_parity_test.rs`. It adds
`ExpectedOutcome::DaemonScopedRead`, distinguishes serving-generation reads
from daemon-scoped reads, asserts zero side effects for daemon-scoped reads,
and verifies CLI and stdio MCP equivalence.

| Evidence | Outcome |
|---|---|
| RED harness | PASS: commit `098df64d` compiled and failed at the intended missing `DaemonScopedRead` behavior |
| Implementation | PASS: commit `38b570ac` completed the F54d contract |
| M12 run 1 | PASS: 10/10, 96.312s, all settle-barrier lines `Active` |
| M12 run 2 | PASS: 10/10, 51.491s, all settle-barrier lines `Active` |
| M12 run 3 | PASS: 10/10, 53.540s, all settle-barrier lines `Active` |
| Committed-final M12 run 1 | PASS: 10/10, 39.15s, all settle-barrier lines `Active` |
| Committed-final M12 run 2 | PASS: 10/10, 36.79s, all settle-barrier lines `Active` |
| Committed-final M12 run 3 | PASS: 10/10, 35.34s, all settle-barrier lines `Active` |
| Default feature matrix | PASS |
| CI feature matrix | PASS |
| Consumer suites | PASS |
| Format | PASS |
| Pedantic Clippy | PASS |
| All-target build | PASS |
| Full local test command | PASS_WITH_FOLLOW_UP: focused and in-scope gates pass; unrelated external-`TEMP` migration rollback failure is tracked as `FF1FC372` |

R-A6 records `125` loaded lines from `6d216d19` and at least `44` authored or
adapted lines at IC `38b570ac`. The post-review binding-state fix is included
in the final candidate's line count. The final authored/adapted total remains
within the `400`-line limit.

## Drift and history preservation

The R9.9 ledger over `f6f3171f..0c4f9d45` contains
`a13c4245` and `b8721d67` on `src/tools/capabilities.rs`. The parked T0 history,
the `7bd9e504` and `6d216d19` records, and B0 at
`0f2cca20:docs/memory/2026-10-04-ship-142-f-a0-b0.md` remain historical
evidence rather than replacement evidence.

Backlogit upgraded from `1.12.0` to `v1.13.0` at `76f7b144` and began enforcing
`blocking_children`. The completed task therefore closed its two verified child
subtasks before moving the parent to `done`:

* `142.058.002-ST` is fully delivered by `174-S` / `142.087-T` plus `177-S`
  commit `38b570ac`
* `142.058.003-ST` is fully delivered by `175-S` / `142.088-T`

This reconciliation honors the operator's 2026-09-29
subtasks-close-with-parent decision and the `d59c455c` precedent. Protected
feature, successor shipment, and downstream task records remain active or
queued as appropriate.

## Validator evidence

| Surface | Expected signal | Observed signal | Outcome |
|---|---|---|---|
| Direct IPC read | Generation-backed read reports the serving generation | `provenance.generation_id == EXPECTED_GENERATION` | PASS |
| Daemon-scoped read | Read succeeds with no false generation provenance or side effect | `generation == None` and `side_effect_count == 0` | PASS |
| CLI read | CLI accepts the read and preserves its expected provenance | Equivalent normalized CLI/MCP result | PASS |
| Stdio MCP read | MCP accepts the same declared read and returns the equivalent normalized payload | Equivalent normalized CLI/MCP result | PASS |
| Refusal behavior | Earlier F54b coverage remains intact | No refusal regression across the 10-case matrix | PASS |

## Monitoring and rollback handoff

| Requirement | Evidence and threshold | Owner |
|---|---|---|
| Healthy signal | Final PR CI and the next `main` CI are green; M12 parity remains 10/10 | softwaresalt |
| Failure signal | False generation provenance, nonzero daemon-scoped side effects, or CLI/MCP divergence | softwaresalt |
| Monitoring plan | Observe PR CI, Copilot review, and the next `main` CI | softwaresalt |
| Rollback trigger | A confirmed parity, provenance, or side-effect regression after merge | softwaresalt |
| Rollback procedure | Revert the merged PR with a reviewed two-parent merge-commit revert | softwaresalt |
| Validation window | Feature PR CI and the next `main` CI | softwaresalt |

## Risky action record

| Proposed action | ActionRisk | Approval and containment | ActionResult |
|---|---|---|---|
| Close verified child subtasks with their completed parent | moderate | Bounded 177-S scope; protected-feature and successor records hash-checked | applied |
| Merge the feature PR | high | Requires P-014/P-018, green CI, P-009, and P-016 verification | pending |
| Deploy or publish a binary | high | Separate operator approval required | not started |

## Safe-close boundary

The intended 177-S close scope is only `142.058-T` and the `177-S` shipment record.
The already-archived `142.058.002-ST` and `142.058.003-ST` records,
`.backlogit/queue/142-F.md`, `.backlogit/queue/178-S.md`, and
`.backlogit/queue/142.059-T.md` are protected baseline observations for
P-015 INV-1/INV-7/INV-10 against the post-feature-merge baseline. They must
remain byte-identical through pre-mode classification, single-artifact
SAFE_CLOSE, and post-mode reconciliation.
No ancestry walk or cascade operation is permitted.

## Releasability

The contract is `PASS_WITH_FOLLOW_UP`. Pre-merge releasability is
`READY_WITH_CONDITIONS` until final-head CI and Copilot review conditions are
satisfied. The external-temp migration rollback failure remains deferred in
`FF1FC372`; no out-of-scope test or fixture change was made. The
`last_code_affecting_head` is the historical reviewed code revision; mutable
current-HEAD readiness is recorded only in the feature PR body.

## Documentation consistency audit

Verdict: `AUDIT_FOLLOWUPS`. The in-scope binding-state and evidence
contradictions are fixed in the final candidate. Out-of-scope findings remain
captured in `FF1FC372`. T0/B0 location qualification and any broader
historical propagation remain resolved for this shipment by the explicit
T0-qualified locator. Final mutable-HEAD PR readiness remains in the PR body.
