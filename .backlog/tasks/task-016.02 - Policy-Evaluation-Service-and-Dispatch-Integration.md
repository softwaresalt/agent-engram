---
id: TASK-016.02
title: Policy Evaluation Service and Dispatch Integration
status: To Do
assignee: []
created_date: '2026-03-30 01:51'
labels:
  - epic
  - daemon
dependencies: []
parent_task_id: TASK-016
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Core policy enforcement logic and wiring into the MCP dispatch pipeline. Covers Plan Units 2 and 3.

Includes:
- `src/services/policy.rs` with `evaluate()` pure function
- Agent identity extraction from `_meta.agent_role` in `src/server/mcp.rs`
- `ToolCallContext` struct to carry agent_role through dispatch (per review F1)
- Policy check wired into `tools::dispatch` before routing
- Backward compatibility: no policy config = allow all
- Tracing spans for policy evaluation and denial events
<!-- SECTION:DESCRIPTION:END -->
