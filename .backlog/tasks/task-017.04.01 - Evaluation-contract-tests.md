---
id: TASK-017.04.01
title: Evaluation contract tests
status: To Do
assignee: []
created_date: '2026-03-30 01:58'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
  - testing
dependencies:
  - TASK-017.03.01
parent_task_id: TASK-017.04
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create contract tests for the evaluation MCP tool.

**File to create:** `tests/contract/evaluation_contract_test.rs`
**Must add:** `[[test]]` block in `Cargo.toml` (per memory: test registration required)

**Contract test cases:**
1. `get_evaluation_report` returns valid JSON with expected schema fields
2. `efficiency_score` is within 0–100 range
3. `anomalies` array items have type, severity, description fields
4. `recommendations` array is present when anomalies exist and include_recommendations is true
5. `recommendations` array is empty when include_recommendations is false
6. `agents` array contains per-agent breakdowns when agent_role data exists
7. Empty metrics produces baseline report (score 100, no anomalies)

**Compile time note:** These tests do not require the `embeddings` feature.
<!-- SECTION:DESCRIPTION:END -->
