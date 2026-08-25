---
title: PR 363 mandatory adversarial escalation review
type: adversarial-review
doc_type: closure
source: PR 363 reviews 5015926424 and 5016087555
source_commit: e00c650eb06073a67a9f228e1fd056c3c359ecb7
date: 2026-08-25
status: review-pending
feature_id: 131-F
review_id: 131.001-R
shipment_id: 125-S
reviewers_requested: 3
reviewers_consensus_eligible: 0
---

# PR 363 Mandatory Adversarial Escalation Review

## Gate state

**BLOCKED / REVIEW PENDING. No adversarial consensus is claimed.** Seven accumulated P1 findings crossed the repository escalation threshold. Feature `131-F`, review `131.001-R`, and shipment `125-S` remain unclaimable until the receipt and finding gates below pass.

## Exact source-head blockers

| Thread / comment | Review | Path | Required disposition |
|---|---|---|---|
| `PRRT_kwDORJEduc6b-DlL` / `3850407649` | `5015926424` | `docs/closure/2026-08-25-pr-363-review-5015710467-remediation.md` | Correct stale 76/76 evidence to 78/78 at source head `e00c650...` and label it source-head-only. |
| `PRRT_kwDORJEduc6b-Dlp` / `3850407688` | `5015926424` | `.backlogit/queue/131.011-T.md` | Add deterministic compile-first spawn-failure RED plus non-panicking `Builder::spawn` and `EngramError` GREEN ownership. |
| `PRRT_kwDORJEduc6b-ZzP` / `3850544771` | `5016087555` | `docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md` | Invoke mandatory independent adversarial escalation or remain blocked. |

Copilot review `5016087555` covered 78/78 files at exact source head `e00c650eb06073a67a9f228e1fd056c3c359ecb7`. That count is not inherited by later commits.

## Spawn seam and task graph under review

The proposed graph is sixteen tasks with fifteen linear dependency edges and seventeen shipment items including `131-F`:

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T -> 131.014-T -> 131.015-T
-> 131.016-T
```

`131.011-T` is behavior-neutral spawner scaffolding. `131.012-T` compiles then deterministically fails through a fake `io::Error`. `131.013-T` owns safe `std::thread::Builder::spawn` result propagation, typed `EngramError`, zero synchronous fallback, and caller-retained provider residual. `131.014-T` separately owns bounded wait, stall, panic or channel loss, and combined-error precedence. `131.015-T` and `131.016-T` own runtime and closure verification. Estimates are 45-105 minutes.

## Frozen instruction manifest for dispatch

Every reviewer must directly cover the same complete PR diff and these exact instructions at the pinned review commit:

1. `AGENTS.md`
2. `.github/copilot-instructions.md`
3. `.github/agents/subagents/adversarial-review.agent.md`
4. `.github/instructions/adversarial-review.instructions.md`
5. `.github/instructions/constitution.instructions.md`
6. `.github/instructions/strict-safety.instructions.md`
7. `.github/instructions/concurrency.instructions.md`
8. `.github/instructions/release-observability.instructions.md`
9. `.github/instructions/backlogit.instructions.md`
10. `.github/policies/workflow-policies.md`
11. `docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md`
12. `docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md`
13. `.backlogit/queue/131-F.md`, `131.001-R`, `131.001-T` through `131.016-T`, and `125-S.md`
14. All files in `git diff --name-only origin/main...<review-commit>` and the full planning diff
15. Exact blocker bodies for comments `3850407649`, `3850407688`, and `3850544771`

Required lenses are architecture; security and TOCTOU; concurrency and lifecycle; Rust safety and error handling; scope, width, graph, roster, and dependencies; constitution and workflow policy; test-first RED/GREEN ownership; operations and rollback; and all accumulated P1 remediations.

## Authoritative receipt gate

A response is eligible only when execution-system metadata binds all of the following:

* stable task, session, response, or dispatch-result identifier
* explicit invocation and reviewer slot
* requested model override and execution-observed provider/model
* exact reviewed commit
* exact frozen instruction manifest
* returned response identifier or hash
* independent completion before consensus assembly

Checked-in routing configuration, requested model labels, named slots, reviewer self-assertion, and an unbound transcript do not prove execution identity. Preserve only minimal non-secret receipt fields; do not check in prompts, transcripts, credentials, tokens, or environment dumps. If fewer than three eligible responses return, fail closed with zero consensus-eligible reviewers.

## Finding policy

Assemble a queue ordered by confidence times severity. HIGH requires all eligible reviewers; MEDIUM requires a majority; LOW is a single reviewer. HIGH P0/P1 blocks. Every MEDIUM finding must be fixed or explicitly deferred with rationale. LOW remains advisory. A bounded remediation may trigger one rerun; review-fix cycles remain capped at three.

## Strict-safety action record

| ProposedAction | Targets | ActionRisk | Rollback | Approval | ActionResult |
|---|---|---|---|---|---|
| Split cleanup launch, spawn-failure RED, bounded wait, and verification | Plan, decision, feature, tasks, review, shipment | high | Revert planning commit | Operator explicitly requested | applied |
| Dispatch read-only independent adversarial reviewers | Complete pinned PR planning scope | high | Discard unbound responses and remain blocked | Operator explicitly requested | planned |
| Reply and resolve bot threads | PR 363 threads only | moderate | Reopen if evidence regresses | Operator explicitly requested | blocked pending evidence |

## Current disposition

No reviewer response is yet counted. No HIGH, MEDIUM, or LOW confidence label is assigned. No thread is resolved by this pending scaffold. PR 362 is out of scope and untouched.
