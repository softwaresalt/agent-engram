---
id: TASK-017.02
title: Evaluation Engine and Configuration
status: To Do
assignee: []
created_date: '2026-03-30 01:56'
labels:
  - epic
  - daemon
dependencies: []
parent_task_id: TASK-017
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Evaluation data models, computation service, and workspace configuration. Covers Plan 2 Units 2, 3, and 5.

Includes:
- `src/models/evaluation.rs` with AgentEfficiency, EvaluationReport, AnomalyFlag
- `src/services/evaluation.rs` with evaluate() function
- EvaluationConfig in WorkspaceConfig (configurable thresholds and weights)
- Scoring algorithm: token efficiency, error rate, diversity, latency
- Anomaly detection: token ratio spike, error burst, tool hammering
- Template-based recommendations
<!-- SECTION:DESCRIPTION:END -->
