---
id: TASK-017.04.02
title: Evaluation integration tests
status: To Do
assignee: []
created_date: '2026-03-30 01:59'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
  - testing
dependencies:
  - TASK-017.03.01
  - TASK-017.01.02
parent_task_id: TASK-017.04
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create integration tests for end-to-end evaluation workflow.

**File to create:** `tests/integration/evaluation_integration_test.rs`
**Must add:** `[[test]]` block in `Cargo.toml` (per memory: test registration required)

**Integration test cases:**
1. Record events with different agent roles, then call get_evaluation_report — verify per-agent attribution
2. Record events with varying success/failure rates — verify scoring reflects error rate
3. Verify anomaly detection triggers on known bad patterns (high token ratio, error bursts)
4. Verify backward compatibility: evaluation works with existing metrics files that lack agent_role/outcome fields
5. Verify configurable thresholds: custom EvaluationConfig changes scoring and anomaly detection
6. Verify evaluation report includes recommendations matched to detected anomalies

**Compile time note:** These tests do not require the `embeddings` feature.
<!-- SECTION:DESCRIPTION:END -->
