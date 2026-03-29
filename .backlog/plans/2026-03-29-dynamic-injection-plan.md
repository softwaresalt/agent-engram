---
title: "Dynamic Instruction Injection"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: reviewed
---

# Dynamic Instruction Injection

## Problem Frame

Research Primitive 5 identifies that static global instructions suffer from the
"lost in the middle" phenomenon. Agents executing multi-step workflows forget
constitutional rules by step 5. The report proposes tool-bound injections and
Definition of Done pre-flight checks.

TASK-012 already implemented explicit instruction re-reads at critical workflow
points (`[REINFORCE]` broadcasts). This plan validates that coverage and adds
the missing DoD pre-flight check.

## Requirements Trace

| # | Requirement | Origin | Status |
|---|---|---|---|
| R1 | Constitution re-reads at workflow decision points | Research P5: "Tool-Bound Injections" | Implemented (TASK-012.03, .05, .06) |
| R2 | Definition of Done pre-flight checks before commit | Research P5: "DoD Checks" | In scope (new) |

## Scope Boundaries

### In Scope

- Validate existing `[REINFORCE]` re-read coverage is sufficient
- Add DoD pre-flight check to build-orchestrator before commit

### Non-Goals

- Dynamic instruction loading via platform `applyTo` changes
- Removing static global instructions (they still serve as baseline)

## Implementation Units

### Unit 1: Add DoD pre-flight check to build-orchestrator

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a DoD pre-flight step before Step 5 (Commit and Record). The orchestrator
MUST read the current task's backlog entry (`backlog-task_view`) and verify
that all acceptance criteria and Definition of Done items are satisfied. If any
are unsatisfied, broadcast at `warning` level and do not commit until resolved.

**Verification:**
- Build-orchestrator contains DoD pre-flight step before commit
- Broadcast includes `[DOD]` prefix

## Key Decisions

- Existing `[REINFORCE]` coverage is sufficient for instruction re-reads
- DoD check reads the task dynamically (not a static checklist)
- DoD check is blocking (must pass before commit), unlike `[REINFORCE]` which is advisory

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I–VII | N/A or Compliant | Markdown-only changes |
