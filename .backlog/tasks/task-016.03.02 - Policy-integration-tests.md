---
id: TASK-016.03.02
title: Policy integration tests
status: Done
implementation_note: policy_test.rs 22 tests (commit 51da38f), contract_policy 5 tests (commit cc24aeb)
assignee: []
created_date: '2026-03-30 01:54'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
  - testing
dependencies:
  - TASK-016.02.02
  - TASK-016.04
parent_task_id: TASK-016.03
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create integration tests for end-to-end policy enforcement through the dispatch pipeline.

**File to create:** `tests/integration/policy_integration_test.rs`
**Must add:** `[[test]]` block in `Cargo.toml` (per memory: test registration required)

**Integration test cases:**
1. Workspace with policy config enforces tool restrictions end-to-end
2. Agent role flows through from mcp_handler to dispatch to policy check
3. Multiple agents with different roles get appropriate access
4. Policy-denied agent can still call allowed tools
5. Backward compatibility: dispatch with default ToolCallContext and no policy config allows all
6. Metrics recording includes agent_role when provided
7. Config loading with invalid policy section falls back to disabled

**Compile time note:** These tests do not require the `embeddings` feature.
<!-- SECTION:DESCRIPTION:END -->
