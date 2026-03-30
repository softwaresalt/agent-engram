---
id: TASK-016.03
title: Policy Engine Tests
status: To Do
assignee: []
created_date: '2026-03-30 01:52'
labels:
  - epic
  - daemon
  - testing
dependencies: []
parent_task_id: TASK-016
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Contract and integration tests for the MCP sandbox policy engine. Covers Plan Unit 6.

Includes:
- Contract tests (`tests/contract/policy_contract_test.rs`)
- Integration tests (`tests/integration/policy_integration_test.rs`)
- `[[test]]` registration in `Cargo.toml` for both test files
- Test coverage for all policy evaluation paths
<!-- SECTION:DESCRIPTION:END -->
