---
id: TASK-012
title: Harness Resilience and Knowledge Retention
status: Done
assignee: []
created_date: '2026-03-29 04:55'
updated_date: '2026-03-29 07:16'
labels:
  - epic
  - harness
dependencies: []
references:
  - .backlog/plans/2026-03-29-harness-resilience-plan.md
  - >-
    .backlog/brainstorm/2026-03-29-harness-resilience-improvements-requirements.md
  - .backlog/research/Agent-Harness-Evaluation-Report.md
  - .backlog/plans/2026-03-29-context-management-plan.md
  - .backlog/plans/2026-03-29-orchestration-routing-plan.md
  - .backlog/plans/2026-03-29-tool-guardrails-plan.md
  - .backlog/plans/2026-03-29-dynamic-injection-plan.md
  - .backlog/plans/2026-03-29-observability-evaluation-plan.md
  - .backlog/reviews/2026-03-29-primitives-1-3-4-5-6-plan-review.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Improve the agent harness across three areas identified by the external evaluation report: context lifecycle management, workflow-embedded instruction reinforcement, and compound knowledge activation. All changes are harness-level (agent/skill markdown files) with no daemon code modifications.

**Problem**: The harness workflows do not actively manage their own cognitive health. Agents accumulate tracking files without pruning, forget constitutional rules during long workflows, and solve problems without recording the solutions for future sessions.

**Approach**: Harness-centric changes only (no daemon code). Restructure when existing capabilities are invoked (compound moves from session-end to per-task) and add new workflow steps (compaction trigger, instruction re-reads) at specific points in existing step sequences.

**Key Decisions**:
- Build-feature's leaf executor constraint means compound invocation stays in the orchestrator
- Learnings-researcher requires no changes (already has full search strategy)
- Compact-context is the only net-new file; all other units modify existing files
- Automated grading deferred to a separate scope
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Retroactive plan-review cycle completed 2026-03-29. All 5 primitive plans reviewed and PASSED. Plans created for P1, P3, P4, P5, P6. P3 supervisor pattern rejected (build-orchestrator already fulfills role). P4 MCP policy engine deferred (needs daemon code). Implementation expanded to include: stop conditions docs, feature flag enforcement, protected file warnings, DoD pre-flight, metrics check, granularity compliance reporting.
<!-- SECTION:NOTES:END -->
