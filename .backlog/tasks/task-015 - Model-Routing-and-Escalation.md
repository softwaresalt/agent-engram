---
id: TASK-015
title: Model Routing and Escalation
status: Done
assignee: []
created_date: '2026-03-29 16:48'
labels:
  - epic
  - harness
  - model-routing
dependencies: []
references:
  - .backlog/research/Agent-Harness-Evaluation-Report.md
  - .backlog/plans/2026-03-29-model-routing-plan.md
  - .backlog/reviews/2026-03-29-model-routing-plan-review.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement model routing and escalation across the agent harness. Assign model tiers to all agents, add escalation on consecutive failures, extend review skill with cross-model personas, and track model usage in session reports.

Based on Research Primitive 3 (Model Routing & Escalation). Three-tier taxonomy: Fast/Cheap (Haiku), Standard (Sonnet), Frontier (Opus/GPT-5.4). Escalation retries failed tasks on higher-tier models before halting.
<!-- SECTION:DESCRIPTION:END -->
