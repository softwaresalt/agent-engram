---
id: TASK-016.03.01
title: Policy contract tests
status: To Do
assignee: []
created_date: '2026-03-30 01:54'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
  - testing
dependencies:
  - TASK-016.02.02
parent_task_id: TASK-016.03
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create contract tests verifying MCP-level policy enforcement behavior.

**File to create:** `tests/contract/policy_contract_test.rs`
**Must add:** `[[test]]` block in `Cargo.toml` (per memory: test registration required)

**Contract test cases:**
1. Policy denial returns JSON-RPC error response with code 14001
2. Policy denial error data includes `agent_role` and `tool` fields
3. Allowed call with policy enabled returns normal result
4. No-policy workspace (disabled) allows all calls for any agent role
5. Unknown agent role with `unmatched = "deny"` returns 14001
6. Unknown agent role with `unmatched = "allow"` succeeds

**Compile time note:** These tests do not require the `embeddings` feature.
<!-- SECTION:DESCRIPTION:END -->
