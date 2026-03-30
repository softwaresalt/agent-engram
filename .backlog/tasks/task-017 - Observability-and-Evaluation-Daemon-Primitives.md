---
id: TASK-017
title: Observability and Evaluation Daemon Primitives
status: To Do
assignee: []
created_date: '2026-03-30 01:55'
updated_date: '2026-03-30 01:59'
labels:
  - epic
  - daemon
  - observability
dependencies:
  - TASK-016
references:
  - .backlog/research/Agent-Harness-Evaluation-Report.md
  - .backlog/plans/2026-03-30-observability-evaluation-daemon-plan.md
  - .backlog/reviews/2026-03-30-sandbox-observability-plan-review.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Daemon-level observability infrastructure for agent-attributed metrics, session efficiency scoring, anomaly detection, and evaluation reporting via MCP tool.

**Problem**: The evaluation loop is primarily human-driven. There is no automated mechanism to flag inefficient tool usage patterns, detect anomalous sessions, or provide actionable efficiency data to the harness. The existing metrics system tracks tool usage but lacks agent attribution and evaluation capabilities.

**Approach**: Extend the existing metrics subsystem in three layers:
1. Attribution layer: Enrich `UsageEvent` with agent identity and call outcome
2. Evaluation layer: New service computing session-level and agent-level efficiency scores
3. Exposure layer: New `get_evaluation_report` MCP tool returning evaluation data

**Key Decisions**:
- Evaluation is batch-computed on demand (not streaming)
- Agent identity reuses `_meta.agent_role` extraction from Policy Engine
- Efficiency score is a composite weighted metric (configurable weights)
- Anomaly thresholds are configurable per workspace
- No external LLM calls from daemon (recommendations are template-based)
- Evaluation data is ephemeral (computed from JSONL, not persisted separately)

**Review findings (P2 advisory)**:
- F4: Make scoring weights configurable in EvaluationConfig
- F5: Use skip_serializing_if for by_agent field in MetricsSummary
- F7: Consider caching evaluation results for large metrics files
<!-- SECTION:DESCRIPTION:END -->
