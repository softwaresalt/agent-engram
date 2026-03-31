---
title: "Task Granularity and Horizon Scoping"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: draft
---

# Task Granularity and Horizon Scoping

## Problem Frame

The AI Agent Harness Evaluation Report (Primitive 2) identifies that agent
reliability drops below 50% for tasks requiring more than 2 hours of
human-equivalent effort and approaches 0% for tasks exceeding 4 hours.
Sequential error compounding makes failure mathematically guaranteed on
multi-day feature specifications dispatched to a single agent loop.

The current harness decomposes work at the epic and sub-task level, but does
not enforce granularity constraints on individual task size or enforce domain
isolation between skill specialties within the same task. The backlog-harvester,
impl-plan, and harness-architect agents all produce tasks without validating
that each task stays within a safe execution horizon.

## Requirements Trace

| # | Requirement | Origin |
|---|---|---|
| R1 | Enforce a 2-hour maximum human-equivalent effort per task | Research Primitive 2: "The 2-Hour Rule" |
| R2 | Isolate tasks by skill domain (width isolation) | Research Primitive 2: "Width vs. Depth Isolation" |
| R3 | Require every task to produce a verifiable state | Research Primitive 2: "Atomic Milestone Validation" |

## Scope Boundaries

### In Scope

- Granularity enforcement rules in impl-plan, backlog-harvester, and harness-architect
- Width isolation guidelines in backlog-harvester task decomposition
- Atomic milestone validation in build-feature and build-orchestrator verification gates
- Documentation of the 2-hour rule in AGENTS.md

### Non-Goals

- Automated time estimation (no reliable mechanism exists; rely on heuristics)
- Changing the build-feature circuit breaker limit (already at 5 attempts)
- Adding new daemon code or MCP tools
- Modifying Rust source code

### Deferred to Implementation

- Calibrating heuristics for what constitutes ">2 hours" in practice

## Validation Approach

The granularity rules are enforced by agent instruction, not by automated tooling.
To verify adoption, the build-orchestrator's session completion report (Step 7e)
MUST include a granularity compliance note: how many tasks were within the 2-hour
heuristic and how many were flagged as oversized. This provides a lightweight
post-implementation feedback loop without adding new tooling.

The backlog-harvester performs the authoritative granularity check at decomposition
time (Phase 3). The harness-architect performs a secondary advisory check at harness
generation time. If the architect flags a task, it broadcasts a warning but does not
block; it recommends re-running the harvester to split the task.

## Implementation Units

### Unit 1: Add granularity constraints to impl-plan skill

**Files:** `.github/skills/impl-plan/SKILL.md`
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a "Task Granularity Rules" section to impl-plan's Core Principles and Plan
Quality Bar. When structuring implementation units in Phase 2, the skill must
validate that each unit is scoped to roughly 2 hours of human effort. If a unit
appears larger, it must be split. The skill must also enforce width isolation:
a single unit should not mix Rust code changes with documentation changes or
database migrations with API changes.

**Verification:**
- impl-plan SKILL.md contains a Task Granularity Rules section
- Plan Quality Bar includes granularity and isolation checks
- Implementation unit template includes an effort-size signal

### Unit 2: Add granularity validation to backlog-harvester

**Files:** `.github/agents/backlog-harvester.agent.md`
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**
Add a granularity validation step in Phase 3 (Harvest) after Step 3.2 (Build
the Decomposition). Before creating tasks, validate each Level 3 task against
the 2-hour rule and width isolation constraints. Tasks that appear too large
must be split. Tasks that mix skill domains must be separated.

**Verification:**
- Harvester contains a granularity validation step
- Oversized tasks are rejected and split
- Mixed-domain tasks are separated

### Unit 3: Add granularity check to harness-architect

**Files:** `.github/agents/harness-architect.agent.md`
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a granularity check in Step 4 (Backlog Analysis). When analyzing subtasks
from the backlog, the harness-architect should flag any task that appears to
require harness tests spanning more than 3-4 test functions as potentially
oversized. Broadcast a warning and recommend splitting via the backlog-harvester.

**Verification:**
- Harness-architect contains a granularity check in Step 4
- Warning is broadcast for oversized tasks

### Unit 4: Add atomic milestone validation to build-orchestrator and build-feature

**Files:** `.github/agents/build-orchestrator.agent.md`, `.github/skills/build-feature/SKILL.md`
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a verification rule that every completed task must produce a verifiable
state change: a passing test, a successful build, or a measurable output.
In build-feature, this is already enforced by the harness loop (test must pass).
In build-orchestrator, add a validation step after Step 4 (Verify Completion
Gates) that confirms the task produced a concrete, observable artifact (commit
with test results, not just a code change without verification).

**Verification:**
- Build-orchestrator Step 4 includes atomic milestone validation
- Build-feature already satisfies this through the harness loop (document this)

### Unit 5: Document the 2-hour rule in AGENTS.md

**Files:** `AGENTS.md`
**Execution note:** test-first
**Dependencies:** Units 1-4

**Approach:**
Add a "Task Granularity" subsection under Development Workflow in AGENTS.md
that codifies the 2-hour rule, width isolation, and atomic milestone validation
as project-level conventions.

**Verification:**
- AGENTS.md contains the Task Granularity section

## Dependency Graph

```text
Unit 1 (impl-plan granularity rules)
  └─► Unit 2 (harvester validation)
        └─► Unit 5 (AGENTS.md documentation)

Unit 3 (harness-architect check) — independent
Unit 4 (build-orchestrator/build-feature validation) — independent
Unit 5 depends on Units 1-4 (documents the full set of rules)
```

## Key Decisions

- **Heuristic, not automated**: The 2-hour rule is enforced by instructing agents
  to evaluate task size using heuristics (file count, function count, test count)
  rather than attempting automated time estimation
- **Width isolation by convention**: Tasks are separated by skill domain through
  decomposition rules, not through tooling restrictions
- **Build-feature already compliant**: The harness loop already mandates a passing
  test as the exit criterion, satisfying atomic milestone validation

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I. Safety-First Rust | N/A | No Rust code changes |
| II. MCP Protocol Fidelity | N/A | No tool surface changes |
| III. Test-First Development | N/A | Markdown-only changes |
| IV. Workspace Isolation | Compliant | All paths within cwd |
| V. Structured Observability | Compliant | Granularity warnings use existing broadcast |
| VI. Single-Binary Simplicity | Compliant | No binary changes |
| VII. CLI Workspace Containment | Compliant | All paths within cwd |
