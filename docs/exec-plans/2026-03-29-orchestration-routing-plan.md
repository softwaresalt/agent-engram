---
title: "Orchestration and Routing"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: reviewed
---

# Orchestration and Routing

## Problem Frame

Research Primitive 3 identifies that the orchestration is "flat" and relies on
human-in-the-loop prompting to switch contexts. The report proposes a supervisor
agent and hard stop conditions.

However, codebase analysis reveals that the build-orchestrator ALREADY implements
comprehensive stop conditions: 5 session loop limits (20 tasks, 3 consecutive
failures, 3 review-fix cycles, 5 fix-ci cycles, 3 stalls), stall detection with
watchdog timeouts, and a 2-hop subagent depth constraint.

The supervisor pattern is unnecessary because the build-orchestrator already
performs supervisor functions (claims tasks, delegates to build-feature, manages
state). Adding a separate supervisor would create redundant orchestration.

## Requirements Trace

| # | Requirement | Origin | Status |
|---|---|---|---|
| R1 | Hard stop conditions in agent configurations | Research P3: "Stop Conditions" | Already implemented |
| R2 | Explicit supervisor pattern | Research P3: "Supervisor Pattern" | Rejected (redundant) |

## Scope Boundaries

### In Scope

- Document existing stop conditions in AGENTS.md for visibility
- Verify no gaps in the existing doom-loop prevention

### Non-Goals

- New supervisor agent (build-orchestrator already fulfills this role)
- Changes to existing loop limits (already calibrated)

## Implementation Units

### Unit 1: Document existing stop conditions in AGENTS.md

**Files:** `AGENTS.md`
**Effort size:** small
**Skill domain:** docs
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a "Stop Conditions and Circuit Breakers" subsection under Development
Workflow in AGENTS.md that references the existing build-orchestrator limits.
This makes the existing protections visible to all agents without adding new
mechanisms.

**Verification:**
- AGENTS.md contains stop conditions documentation

## Key Decisions

- Supervisor pattern rejected — build-orchestrator already acts as supervisor
- Existing limits are well-calibrated (validated through TASK-012 codebase analysis)
- Documentation-only change to improve visibility of existing protections

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I–VII | N/A or Compliant | Documentation-only change |
