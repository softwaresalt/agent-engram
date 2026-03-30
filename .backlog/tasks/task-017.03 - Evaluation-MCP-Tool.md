---
id: TASK-017.03
title: Evaluation MCP Tool
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
New MCP tool exposing evaluation data to connected agents. Covers Plan 2 Unit 4.

Includes:
- `get_evaluation_report` handler in `src/tools/read.rs`
- Dispatch routing in `src/tools/mod.rs`
- Registration in `should_record_metrics`
- Parameters: branch (optional), include_recommendations (optional, default true)
<!-- SECTION:DESCRIPTION:END -->
