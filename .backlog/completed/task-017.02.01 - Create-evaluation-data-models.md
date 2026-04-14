---
id: TASK-017.02.01
title: Create evaluation data models
status: Done
assignee: []
created_date: '2026-03-30 01:57'
labels:
  - daemon
dependencies: []
parent_task_id: TASK-017.02
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create evaluation data models.

**Files to create/modify:**
- Create `src/models/evaluation.rs`
- Modify `src/models/mod.rs` to add `pub mod evaluation;`

**Structs to implement:**

`AgentEfficiency`:
- agent_role: String
- total_calls: u64
- total_tokens: u64
- avg_tokens_per_call: f64
- tokens_per_result: f64
- error_rate: f64
- tool_diversity: u32
- anomalies: Vec<String>

`EvaluationReport`:
- branch: String
- efficiency_score: u32 (0–100)
- agents: Vec<AgentEfficiency>
- anomalies: Vec<AnomalyFlag>
- recommendations: Vec<String>
- evaluated_at: String (RFC 3339)

`AnomalyFlag`:
- anomaly_type: String
- severity: String ("info", "warning", "critical")
- description: String
- agent_role: Option<String>

`EvaluationConfig`:
- max_token_ratio: f64 (default 10.0)
- max_error_rate: f64 (default 0.3)
- min_tool_diversity: u32 (default 2)
- slow_query_threshold_ms: u64 (default 200)
- weights: ScoringWeights (token_efficiency, error_rate, diversity, latency — all f64 with defaults)

All structs derive `Debug, Clone, Serialize, Deserialize, PartialEq`.

**Test scenarios:**
- All evaluation models serialize/deserialize round-trip
- EvaluationConfig defaults are correct
- AnomalyFlag with and without agent_role
<!-- SECTION:DESCRIPTION:END -->
