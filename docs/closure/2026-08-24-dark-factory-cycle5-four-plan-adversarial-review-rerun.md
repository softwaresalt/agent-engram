---
title: "Dark factory cycle 5 four-plan adversarial review rerun — invalidated"
type: adversarial-review
doc_type: closure
source: "configuration-only rerun; no authoritative execution-model binding"
date: 2026-08-24
status: failed-closed-unverified
commit: 72600a33284148c6a13ef807812fd0e7e06d883a
reviewers_requested: 3
reviewers_consensus_eligible: 0
---

# Dark factory cycle 5 four-plan adversarial review rerun — invalidated

## Gate decision

**FAILED CLOSED / UNVERIFIED. No adversarial consensus was assembled.**

This report previously treated checked-in reviewer frontmatter and named dispatch
slots as sufficient model-routing evidence. That contradicted the unchanged
fail-closed requirement in the initial report. The rerun is invalid for gate
purposes, and its prior pass and confidence calculations are withdrawn.

## One evidence standard

A reviewer response is consensus-eligible only when authoritative execution
evidence binds that specific response to the model that executed it. Acceptable
evidence is execution-system task or dispatch-result metadata, or runtime
metadata, containing at minimum:

* a stable task, response, or dispatch-result identifier
* the observed provider and model identity emitted by the execution system
* a binding between that identifier/model identity and the returned response
* the exact reviewed commit and reviewer slot bound to that response
* complete direct coverage by that reviewer of every required review instruction

Checked-in routing configuration, requested model labels, named slots, and a
reviewer self-assertion are not execution binding. They may describe intent but
do not prove which model produced a response.

## Evidence retrieval and decision

The remediation examined all available artifacts for the initial attempt, this
rerun, and the bounded final rerun, including their embedded raw reviewer
outputs and the checked-in reviewer frontmatter used for dispatch. No separate
task receipt, dispatch-result record, response ID, provider result, or runtime
model field is preserved in the repository for any rerun response. The embedded
outputs explicitly state that runtime self-introspection was not exposed.

**Authoritative binding evidence does not exist in the available record.** No
operator requirement change was recorded. The initial standard remains in
force, so configuration-only routing evidence cannot cure the missing runtime
binding.

## Configuration record, not execution proof

| Slot | Configured reviewer | Requested provider/model | Response or task ID | Execution-system model field | Eligible |
|---|---|---|---|---|---|
| A | Concurrency Reviewer | `openai/gpt-5.4-mini` | none preserved | unavailable | No |
| B | Rust Engineer | `anthropic/claude-sonnet-4.6` | none preserved | unavailable | No |
| C | Security Sentinel | `anthropic/claude-opus-4.6` | none preserved | unavailable | No |

These are the same three configured reviewers referenced by the prior rerun.
Because none is authoritatively bound to execution, they cannot be cited in a
consensus denominator.

## Consensus correction

* Eligible reviewers: 0
* HIGH findings: not calculated
* MEDIUM findings: not calculated
* LOW findings: not calculated
* Prior `M-01` and `M-02`: retained only as useful unverified reviewer
  observations; they are not consensus findings
* Prior raw `block` normalization: withdrawn

Plan edits made in response to `M-01` and `M-02` may remain as conservative
planning improvements, but they do not clear an adversarial gate.

## Fail-closed dispositions

| Plan/source stash | Gate state | Backlog disposition |
|---|---|---|
| `7B15B447` | failed/unverified | feature `132-F`, tasks, and shipment `126-S` blocked; stash reactivated as `172AE8CE` |
| `1CB366DB` | failed/unverified and dependent | feature `133-F`, tasks, and shipment `127-S` blocked; stash reactivated as `8C7733CE`; `126-S` remains the terminal predecessor |
| `1C2A3CB3` | failed/unverified | feature `134-F`, tasks, and shipment `128-S` blocked; stash reactivated as `721A42F0` |
| `5DF94427` | failed/unverified | feature `135-F`, tasks, and shipment `129-S` blocked; stash reactivated as `BD5DD62A` |
| `49000348` | environment-blocked | remains non-executable and outside this review |

Shipment `125-S` and feature `131-F` are unrelated optional-OTLP maintenance and
remain queued with their durable integration claim guard.

## Requirement-resolution decision

The requirement was not weakened, superseded, or reinterpreted. A future valid
run must preserve minimal non-secret receipts containing response/task IDs,
execution-system model fields, and exact reviewed-commit and reviewer-slot
evidence for each independent reviewer, and every counted reviewer must
directly cover every required review instruction. Until then, the four
security/durability plans remain non-executable.
