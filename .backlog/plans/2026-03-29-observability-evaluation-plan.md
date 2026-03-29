---
title: "Observability and Evaluation"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: reviewed
---

# Observability and Evaluation

## Problem Frame

Research Primitive 6 identifies that the evaluation loop is primarily human-driven.
There is no automated model-based grader rejecting poor agent outputs synchronously.
The report proposes an adversarial evaluator agent and metrics-driven adaptation.

The existing `plan-review` skill (now embedded in backlog-harvester) and the
`review` skill (used by build-orchestrator's post-build review gate) already
provide multi-persona automated review. The gap is that these reviews happen
at plan and code stages but not as a CI-blocking gate.

Metrics-driven adaptation requires the `get_branch_metrics` MCP tool (TASK-010)
which is already implemented but not yet wired into the harness for automated
flagging.

## Requirements Trace

| # | Requirement | Origin | Status |
|---|---|---|---|
| R1 | Automated review gate before merge | Research P6: "Adversarial Evaluator" | Partially implemented (review skill exists) |
| R2 | Metrics-driven skill flagging | Research P6: "Metrics-Driven Adaptation" | In scope |
| R3 | Granularity compliance reporting | Plan-review P1 finding from P2 plan | In scope |

## Scope Boundaries

### In Scope

- Wire the existing `review` skill as a mandatory gate in build-orchestrator session completion
- Add metrics check at session end using `get_branch_metrics`
- Add granularity compliance note to session completion report

### Non-Goals

- New adversarial evaluator agent (existing review skill + plan-review suffice)
- CI pipeline integration (out of scope for harness-only changes)
- Automated prompt optimization (requires separate infrastructure)

## Implementation Units

### Unit 1: Strengthen review gate as session-end mandatory check

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
The build-orchestrator already has Step 7a (Standalone Review) at session end.
Strengthen it: if P0/P1 findings remain unresolved after the review-fix cycle
limit, the orchestrator MUST NOT push the branch. It must broadcast at `error`
level and halt, requiring human intervention.

**Verification:**
- Step 7a explicitly blocks push on unresolved P0/P1

### Unit 2: Add metrics check at session end

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a Step 7a.5 between review and compound capture. Call `get_branch_metrics`
(engram MCP tool) to retrieve token usage for the session. If the input-to-output
token ratio exceeds a threshold (10:1), broadcast at `warning` level:
`[METRICS] High token ratio ({ratio}:1) for this session — review skill efficiency`.
This is advisory, not blocking.

**Verification:**
- Build-orchestrator calls `get_branch_metrics` at session end
- Token ratio warning broadcasts when threshold exceeded

### Unit 3: Add granularity compliance to session report

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
In Step 7e (Report and Hand Off), add a granularity compliance note: how many
tasks were within the 2-hour heuristic and how many were flagged. This provides
the post-implementation feedback loop identified by the plan-review P1 finding.

**Verification:**
- Session completion report includes granularity compliance

## Key Decisions

- Existing review skill is sufficient (no new adversarial agent needed)
- Metrics check is advisory, not blocking (avoid false-positive stalls)
- Granularity compliance reporting closes the feedback loop from TASK-014

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I–VII | N/A or Compliant | Markdown-only changes |
| IX | Compliant | Uses `get_branch_metrics` engram tool |
