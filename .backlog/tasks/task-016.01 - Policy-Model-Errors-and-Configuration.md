---
id: TASK-016.01
title: 'Policy Model, Errors, and Configuration'
status: To Do
assignee: []
created_date: '2026-03-30 01:50'
labels:
  - epic
  - daemon
dependencies: []
parent_task_id: TASK-016
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Data model, error types, and workspace configuration for the MCP sandbox policy engine. Covers Plan Units 1 and 4.

Includes:
- `PolicyRule` struct (agent_role, allow list, deny list)
- `PolicyConfig` struct (enabled, unmatched policy, rules list)
- `UnmatchedPolicy` enum (Allow, Deny)
- `PolicyError` variants in `EngramError` (Denied code 14001, ConfigInvalid code 14002)
- Error codes 14001–14002 in `src/errors/codes.rs`
- `[policy]` section in `WorkspaceConfig` deserialization
- `policy_config()` accessor on `AppState`
<!-- SECTION:DESCRIPTION:END -->
