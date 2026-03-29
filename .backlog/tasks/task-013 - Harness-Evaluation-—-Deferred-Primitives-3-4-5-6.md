---
id: TASK-013
title: 'Harness Evaluation — Deferred Primitives (3, 4, 5, 6)'
status: To Do
assignee: []
created_date: '2026-03-29 04:57'
updated_date: '2026-03-29 06:55'
labels:
  - epic
  - harness
  - deferred
dependencies: []
references:
  - .backlog/research/Agent-Harness-Evaluation-Report.md
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remaining harness improvement items from the AI Agent Harness Evaluation Report that were explicitly deferred from TASK-012 (Harness Resilience). These address Research Primitives 3, 4, and 6 that require either daemon code changes or represent significant new capabilities. Primitive 2 (Task Granularity) is now covered by TASK-014.

**Deferred items:**
- **Primitive 3 (Orchestration)**: Supervisor pattern agent — evaluated and deemed unnecessary given existing build-orchestrator session limits and circuit breakers
- **Primitive 4 (Tool Execution)**: Per-agent write policies via MCP policy engine — requires daemon code changes to restrict tool access per agent identity
- **Primitive 4 (Tool Execution)**: Feature flag enforcement for agent-generated code — rule addition to `rust.instructions.md`
- **Primitive 5 (Dynamic Reminders)**: Definition of Done pre-flight checks — forces agents to self-verify DoD before final commit
- **Primitive 6 (Observability)**: Adversarial evaluator agent as CI blocker — requires separate brainstorm on quality gates
- **Primitive 6 (Observability)**: Metrics-driven adaptation — auto-flag inefficient skills based on token ratio spikes from `get_branch_metrics`

**Why deferred:** These items either require daemon code (violating the harness-only constraint), introduce new agent roles with their own risk profiles (false positives, additional token cost), or depend on infrastructure not yet built (metrics-driven automation).
<!-- SECTION:DESCRIPTION:END -->
